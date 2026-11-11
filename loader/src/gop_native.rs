//! Native-GOP display recovery (issue #107 CSM class).
//!
//! On CSM-enabled boards the recursive `ConnectController` reconnect hangs
//! re-binding the legacy BiosVideo driver to the target GPU. This path
//! never calls ConnectController on the PCI handle: it starts the card's
//! OWN NVIDIA GOP driver explicitly (or a compatibility module shipped on
//! the boot volume), binds it with Driver Binding Supported/Start, hands
//! the console to the fresh GOP child, and restores VGA forwarding.
//!
//! Behavior reference: the #107 reporter's working implementation and the
//! community GOP-Updater tooling (both read for behavior only - the GOP
//! sources carry no license header, so this is an independent Rust
//! implementation per constitution §1.10).
//!
//! Decompression is delegated to the firmware's EFI_DECOMPRESS_PROTOCOL:
//! NVIDIA ships the GOP inside the option ROM as a CompressionType=1 EFI
//! image, and PciBus itself uses this exact protocol for compressed
//! OPROMs - the reference implementation, present on every UEFI system,
//! keeps this module parse-only (romcore walk/efirom/pe).
//!
//! Failure policy: ANY error aborts the recovery loudly and leaves the
//! console to bootmgfw (the proven fallback). Falling back to the
//! hanging ConnectController would defeat the purpose; never done.

use alloc::vec;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::ffi::c_void;
use core::ptr::null_mut;
use log::info;
use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams, SearchType};
use uefi::proto::console::gop::GraphicsOutput;
use uefi::{guid, Handle, Identify};

use crate::pci::PciIo;
use crate::tx_detach::{device_path_bytes, path_belongs_to};
use romcore::{efirom, pe};

/// VGA forwarding bits (EDK2 PciIo.h): VGA_PALETTE_IO | VGA_MEMORY |
/// VGA_IO. The 16-bit-decode variants (0x60000) are excluded - no board
/// in the compatibility matrix reports them on the GPU function.
pub const VGA_ATTR_MASK: u64 = 0x001C;
/// IO | MEMORY decode bits (never BUS_MASTER - the handoff keeps BME=0).
pub const DECODE_MASK: u64 = 0x0300;

const DECOMPRESS_GUID: uefi::Guid = guid!("d8117cfe-94a6-11d4-9a3a-0090273fc14d");
const DRIVER_BINDING_GUID: uefi::Guid = guid!("18a031ab-b443-4d1a-a5c0-0c09261e9f71");

/// EFI_DECOMPRESS_PROTOCOL (MdePkg/Include/Protocol/Decompress.h): two
/// function pointers and NOTHING else - there is no Revision field. A
/// phantom `_revision: u32` here shifted our `get_info` slot onto the
/// firmware's Decompress pointer, so the "GetInfo" call actually ran
/// Decompress with GetInfo's argument list - a full 164608-byte
/// decompression into the 4-byte `dst_size` stack slot, i.e. a stack
/// smash and the silent freeze #128 logged at its `GetInfo next` line
/// (caught by seti9585's #107 source review; the firmware service was
/// never at fault). Layout pinned by the asserts below.
#[repr(C)]
struct DecompressProto {
    get_info: unsafe extern "efiapi" fn(
        this: *mut DecompressProto,
        source: *const u8,
        source_size: u32,
        destination_size: *mut u32,
        scratch_size: *mut u32,
    ) -> uefi::Status,
    decompress: unsafe extern "efiapi" fn(
        this: *mut DecompressProto,
        source: *const u8,
        source_size: u32,
        destination: *mut u8,
        destination_size: u32,
        scratch: *mut u8,
        scratch_size: u32,
    ) -> uefi::Status,
}
const _: () = assert!(core::mem::offset_of!(DecompressProto, get_info) == 0);
const _: () = assert!(core::mem::offset_of!(DecompressProto, decompress) == 8);
const _: () = assert!(core::mem::size_of::<DecompressProto>() == 16);

/// EFI_DRIVER_BINDING_PROTOCOL (MdePkg/Include/Protocol/DriverBinding.h).
#[repr(C)]
struct DriverBindingProto {
    supported:
        unsafe extern "efiapi" fn(this: *mut DriverBindingProto, controller: Handle, remaining: *mut c_void) -> uefi::Status,
    start: unsafe extern "efiapi" fn(this: *mut DriverBindingProto, controller: Handle, remaining: *mut c_void) -> uefi::Status,
    stop: unsafe extern "efiapi" fn(
        this: *mut DriverBindingProto,
        controller: Handle,
        number_of_children: usize,
        child_handles: *mut Handle,
    ) -> uefi::Status,
    version: u32,
    image_handle: Handle,
    driver_binding_handle: Handle,
}

/// Entry point. `initial_attrs` = the PciIo attributes captured BEFORE the
/// transaction's claim cleared them (see detach_and_claim). Default since
/// the native-GOP merge - the marker `\tx-native-gop.txt` is no longer an
/// enable switch but an OPTIONAL pin naming a specific GOP module file on
/// the boot volume (empty/absent = the card's own shadowed ROM).
pub fn recover(pci_handle: Handle, gpu: &PciIo, initial_attrs: Option<u64>) -> Result<(), &'static str> {
    info!("NATIVE-GOP: recovery begin (default console rebuild)");

    // 1. Locate the GOP EFI image.
    let (rom, source_note) = locate_source(gpu)?;
    info!("NATIVE-GOP: source = {source_note}");

    // 2. Parse the expansion-ROM chain and take the code-type-0x03 image.
    info!("NATIVE-GOP: ROM chain walk begin ({} bytes)", rom.len());
    let efi_images = romcore::walk::efi_images(&rom).map_err(|e| {
        info!("NATIVE-GOP: ROM chain walk failed: {e}");
        "rom walk failed"
    })?;
    if efi_images.len() != 1 {
        info!("NATIVE-GOP: expected exactly one EFI image, found {}", efi_images.len());
        return Err("unexpected EFI image count");
    }
    let img = &rom[efi_images[0].start..efi_images[0].start + efi_images[0].len];
    let header = efirom::EfiRomHeader::parse(img).map_err(|e| {
        info!("NATIVE-GOP: EFI image header parse failed: {e}");
        "bad efi rom header"
    })?;
    if header.efi_subsystem != efirom::SUBSYSTEM_BOOT_SERVICE_DRIVER {
        info!("NATIVE-GOP: EfiSubsystem={:#x} is not a boot-services driver", header.efi_subsystem);
        return Err("unexpected efi subsystem");
    }
    if header.efi_machine_type != efirom::MACHINE_X64 {
        info!("NATIVE-GOP: EfiMachineType={:#x} is not x64", header.efi_machine_type);
        return Err("unexpected machine type");
    }
    let payload = efirom::payload(img, &header).map_err(|e| {
        info!("NATIVE-GOP: payload header parse failed: {e}");
        "bad payload header"
    })?;
    info!(
        "NATIVE-GOP: EFI image at {:#x} len {:#x}, compression={} payload {} -> {} bytes",
        efi_images[0].start, efi_images[0].len, header.compression_type,
        payload.compressed_size, payload.uncompressed_size
    );

    // 3. Decompress through the firmware's own protocol.
    let pe_image = match header.compression_type {
        efirom::COMPRESSION_EFI_STANDARD => firmware_decompress(img, &payload)?,
        0 => img[payload.stream_offset..].to_vec(),
        other => {
            info!("NATIVE-GOP: unsupported CompressionType={other}");
            return Err("unsupported compression type");
        }
    };

    // 4. Sanity: the buffer must be the x64 boot-services driver we are
    // about to load. Readback-before-load (constitution §1.1: nothing is
    // "proven" by the decompress returning success).
    let pe_info = pe::parse(&pe_image).map_err(|e| {
        info!("NATIVE-GOP: decompressed image is not a valid PE32+: {e}");
        "pe validation failed"
    })?;
    info!(
        "NATIVE-GOP: PE ok: machine={:#06x} subsystem={} SizeOfImage={:#x} entry={:#x}",
        pe_info.machine, pe_info.subsystem, pe_info.size_of_image, pe_info.entry_point
    );

    // 5. Load + start.
    info!("NATIVE-GOP: LoadImage ({} bytes PE32+)", pe_image.len());
    let image_handle = boot::load_image(
        boot::image_handle(),
        boot::LoadImageSource::FromBuffer { buffer: &pe_image, file_path: None },
    )
    .map_err(|e| {
        info!("NATIVE-GOP: LoadImage failed: {:?}", e.status());
        "load_image failed"
    })?;
    info!("NATIVE-GOP: LoadImage ok ({image_handle:?}) - StartImage next");
    if let Err(e) = boot::start_image(image_handle) {
        info!("NATIVE-GOP: StartImage failed: {:?}", e.status());
        return Err("start_image failed");
    }
    info!("NATIVE-GOP: GOP driver started");

    // 6. Explicit bind: exactly the new image's Driver Binding, Supported
    // then Start, on the target GPU only ("ConnectController priority is
    // not an exclusion filter" - and it is the call that hangs under CSM,
    // so it is never made here).
    bind_explicitly(image_handle, pci_handle)?;

    // 7. Readback: a GOP child owned by the target must exist now.
    let child = verify_gop_child(pci_handle)?;

    // 8. Console: bind the console stack onto the fresh GOP child only.
    // A child-level connect never re-runs the PCI-level drivers, so
    // BiosVideo cannot be re-selected even under CSM. The two outcomes
    // stay distinguishable in the log (issue #107 review): a rebound
    // console is a verified display path; a failed rebind is a
    // structural success that leaves display recovery unproven.
    let console_rebound = match boot::connect_controller(child, &[], None, false) {
        Ok(()) => {
            info!("NATIVE-GOP: console rebound to the native GOP child");
            true
        }
        Err(e) => {
            info!("NATIVE-GOP: console rebind failed {:?} (bootmgfw will self-establish)", e.status());
            false
        }
    };

    // 9. VGA forwarding + decode, never BME.
    restore_vga(gpu, initial_attrs);

    if console_rebound {
        info!("NATIVE-GOP: complete (display path verified)");
        // Issue #228: the verified console child is the YOUNGEST GOP handle
        // (the rebuild replaced the POST-time child), so every first-pick
        // consumer (grub gfxterm, Linux efistub, bootmgr BGA) lands on an
        // older headless competitor. Dump is unconditional read-only; the
        // reorder/hide surgeries are marker-gated inside.
        crate::gop_order::after_console_rebound(child);
    } else {
        info!("NATIVE-GOP: complete (STRUCTURAL: console left to bootmgfw - display recovery NOT verified)");
    }
    Ok(())
}

/// Optional GOP module pin: `\tx-native-gop.txt`'s first line names a
/// volume file to use INSTEAD of the card's own shadowed ROM (compatibility
/// package form). Empty/absent content — or a named file that does not
/// resolve on the boot volume — falls through to the shadowed ROM (issue
/// #128 run 10: a zero-byte marker was refused by the content reader and a
/// mistyped name aborted the whole recovery; both now degrade to the ROM
/// path). A named file that EXISTS but fails parsing later is still a hard
/// error — the operator asked for that specific image.
fn locate_source(gpu: &PciIo) -> Result<(Vec<u8>, alloc::string::String), &'static str> {
    // Every step below is bracketed by its own flushed line (NATIVE-GOP
    // is a flush marker, issue #128 run 11): a hang must land BETWEEN two
    // lines instead of after the last one.
    if let Some(content) = crate::tx::read_boot_file("\\tx-native-gop.txt") {
        let name = String::from_utf8_lossy(&content)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if !name.is_empty() {
            let path = if name.starts_with('\\') { name.clone() } else { format!("\\{name}") };
            info!("NATIVE-GOP: marker names '{path}' - probing the boot volume");
            if let Some(bytes) = crate::tx::read_boot_file(&path) {
                let n = bytes.len();
                info!("NATIVE-GOP: volume file {path} loaded ({n} bytes)");
                return Ok((bytes, alloc::format!("volume file {path} ({n} bytes)")));
            }
            info!(
                "NATIVE-GOP: \\tx-native-gop.txt names {path} but it is not on the boot volume - falling back to the card's own shadowed ROM"
            );
        }
    } else {
        // Present (recover() only runs when the presence probe fired) but
        // the content reader refused it - on this path that means the
        // size-0 form, which is the documented marker shape, not a
        // failure. The reader's own TX-ASSET-DIAG refusal line reads like
        // one (issue #128 run 12 cosmetic note), so say so explicitly.
        info!("NATIVE-GOP: marker present but empty/unreadable (size-0 is the documented form) - using the card's own shadowed ROM");
    }
    info!("NATIVE-GOP: acquiring the card's own shadowed ROM (PciIo RomImage/RomSize)");
    let Some(rom) = gpu.rom_image() else {
        info!("NATIVE-GOP: PciIo RomImage is empty (no shadowed option ROM)");
        return Err("no rom image");
    };
    info!("NATIVE-GOP: shadowed ROM acquired ({} bytes) - copying", rom.len());
    let n = rom.len();
    let rom = rom.to_vec();
    info!("NATIVE-GOP: shadowed ROM copied ({} bytes)", rom.len());
    Ok((rom, alloc::format!("PciIo RomImage ({n} bytes)")))
}

/// Run the firmware's EFI_DECOMPRESS_PROTOCOL over the payload (the
/// {compressed, uncompressed} header pair included, as the protocol
/// expects).
fn firmware_decompress(img: &[u8], payload: &efirom::EfiPayload) -> Result<Vec<u8>, &'static str> {
    let Some(st) = uefi::table::system_table_raw() else {
        return Err("no system table");
    };
    // Safety: boot services are live; LocateProtocol is read-only for us.
    unsafe {
        let bt = st.as_ref().boot_services;
        if bt.is_null() {
            return Err("no boot services");
        }
        let mut proto: *mut DecompressProto = null_mut();
        let status = ((*bt).locate_protocol)(
            &DECOMPRESS_GUID,
            null_mut(),
            (&mut proto as *mut *mut DecompressProto).cast(),
        );
        if status != uefi::Status::SUCCESS || proto.is_null() {
            info!("NATIVE-GOP: EFI_DECOMPRESS_PROTOCOL unavailable ({status:?})");
            return Err("no decompress protocol");
        }
        info!("NATIVE-GOP: EFI_DECOMPRESS_PROTOCOL located - GetInfo next");
        let src = &img[payload.stream_offset - 8..];
        let src_len = (payload.compressed_size + 8) as u32;
        if src.len() < src_len as usize {
            info!("NATIVE-GOP: payload shorter than its declared compressed size");
            return Err("payload truncated");
        }
        let mut dst_size: u32 = 0;
        let mut scratch_size: u32 = 0;
        let status = ((*proto).get_info)(proto, src.as_ptr(), src_len, &mut dst_size, &mut scratch_size);
        if status != uefi::Status::SUCCESS {
            info!("NATIVE-GOP: Decompress.GetInfo failed ({status:?})");
            return Err("get_info failed");
        }
        if dst_size as u64 != payload.uncompressed_size as u64 {
            info!(
                "NATIVE-GOP: GetInfo size {} disagrees with the payload header {}",
                dst_size, payload.uncompressed_size
            );
            return Err("size disagreement");
        }
        // Both allocations are firmware-supplied sizes: gate them before
        // the vec![0u8; n] calls - a garbage size is an unbounded
        // allocation and a silent freeze (the run-11 shape), not an error
        // we could report afterwards. Real GOP drivers decompress to well
        // under 1 MiB; these ceilings only catch garbage.
        if dst_size as usize > 16 * 1024 * 1024 || scratch_size as usize > 64 * 1024 * 1024 {
            info!(
                "NATIVE-GOP: decompression sizes implausible (dst={dst_size} scratch={scratch_size}) - refusing",
            );
            return Err("implausible decompression sizes");
        }
        info!(
            "NATIVE-GOP: GetInfo ok (dst={dst_size} scratch={scratch_size}) - decompressing"
        );
        let mut dst = vec![0u8; dst_size as usize];
        let mut scratch = vec![0u8; scratch_size as usize];
        let status = ((*proto).decompress)(
            proto,
            src.as_ptr(),
            src_len,
            dst.as_mut_ptr(),
            dst_size,
            scratch.as_mut_ptr(),
            scratch_size,
        );
        if status != uefi::Status::SUCCESS {
            info!("NATIVE-GOP: Decompress failed ({status:?})");
            return Err("decompress failed");
        }
        info!("NATIVE-GOP: firmware decompress ok ({} bytes)", dst.len());
        Ok(dst)
    }
}

/// Find the Driver Binding installed by `image_handle` (exactly one) and
/// run Supported/Start against the target PCI handle. Uniqueness is
/// established BEFORE any callback runs (issue #107 review: invoking
/// Supported/Start while still counting means a second match would have
/// already executed its callbacks by the time the count check fires).
fn bind_explicitly(image_handle: Handle, pci_handle: Handle) -> Result<(), &'static str> {
    let handles = boot::locate_handle_buffer(SearchType::ByProtocol(&DRIVER_BINDING_GUID))
        .map_err(|_| "no driver bindings")?;
    let mut matches = 0usize;
    let mut unique: Option<(*mut DriverBindingProto, Handle)> = None;
    for i in 0..handles.len() {
        let Some(binding) = handle_protocol_raw(handles[i], &DRIVER_BINDING_GUID) else {
            continue;
        };
        // Safety: read-only field access inside the unsafe block below.
        unsafe {
            if (*binding).image_handle != image_handle {
                continue;
            }
            matches += 1;
            unique = Some((binding, handles[i]));
        }
    }
    if matches != 1 {
        info!("NATIVE-GOP: expected exactly one Driver Binding from the loaded image, found {matches}");
        return Err("binding identity mismatch");
    }
    // Safety: protocol pointers live for the boot services lifetime;
    // exactly one matching binding, validated above.
    let (binding, binding_handle) = unique.unwrap();
    unsafe {
        info!(
            "NATIVE-GOP: unique binding {binding_handle:?} version={:#x} - Supported next",
            (*binding).version
        );
        let supported = ((*binding).supported)(binding, pci_handle, null_mut());
        info!("NATIVE-GOP: binding Supported={supported:?}");
        if supported != uefi::Status::SUCCESS {
            return Err("driver binding supported refused");
        }
        let start = ((*binding).start)(binding, pci_handle, null_mut());
        info!("NATIVE-GOP: binding Start={start:?}");
        if start != uefi::Status::SUCCESS {
            return Err("driver binding start failed");
        }
    }
    Ok(())
}

/// HandleProtocol via raw boot services (the Driver Binding struct is not
/// modeled in uefi-rs).
fn handle_protocol_raw(handle: Handle, guid: &uefi::Guid) -> Option<*mut DriverBindingProto> {
    let st = uefi::table::system_table_raw()?;
    unsafe {
        let bt = st.as_ref().boot_services;
        if bt.is_null() {
            return None;
        }
        let mut out: *mut c_void = null_mut();
        let status = ((*bt).handle_protocol)(handle.as_ptr(), guid, &mut out);
        if status == uefi::Status::SUCCESS && !out.is_null() {
            Some(out as *mut DriverBindingProto)
        } else {
            None
        }
    }
}

/// Readback (constitution §1.1): the target GPU must now own a GOP handle
/// whose device path extends the GPU's device path.
///
/// The child may be installed a beat AFTER Start returns (notification-
/// event paths inside the GOP driver), so this probes up to three times
/// with a settle delay - Z170 run 2 saw Start=SUCCESS and then "no GOP
/// child" 26 ms later with zero visibility into what the enumeration DID
/// contain. Every probe now logs each GOP handle it saw (path length +
/// belongs verdict); on the first probe non-belonging handles also dump a
/// capped path hex, which separates "child exists but the path check
/// rejects it" (a path-belongs_to bug, PR13-shaped) from "the driver
/// really did not create a child" (timing/driver behavior) in one boot.
fn verify_gop_child(pci_handle: Handle) -> Result<Handle, &'static str> {
    let parent = device_path_bytes(pci_handle).unwrap_or_default();
    for attempt in 0..3u32 {
        if attempt > 0 {
            boot::stall(core::time::Duration::from_millis(250));
        }
        let handles = boot::locate_handle_buffer(SearchType::ByProtocol(&GraphicsOutput::GUID))
            .map_err(|_| "no gop handles after start")?;
        for i in 0..handles.len() {
            let gop = handles[i];
            let child_path = device_path_bytes(gop).unwrap_or_default();
            let belongs = path_belongs_to(&parent, &child_path);
            if belongs || attempt == 0 {
                // Issue #228: LocateHandleBuffer order is creation order and
                // every boot-graphics consumer picks the first handle, so the
                // position + EDID presence (sink proof) of every child are
                // first-class evidence. First card's probe logs the stock
                // order; the console card's post-rebuild probe logs the
                // flipped order - before/after in one unlock run.
                let edid = crate::gop_order::edid_size(gop);
                let edid_note = match edid {
                    Some(n) => alloc::format!(" yes({n}B)"),
                    None => " no".to_string(),
                };
                info!(
                    "NATIVE-GOP: probe {attempt}: GOP handle {:?} idx {i}/{} path {} bytes belongs_to_target={belongs} edid={}",
                    gop,
                    handles.len(),
                    child_path.len(),
                    edid_note
                );
                if !belongs && child_path.len() >= 4 {
                    let n = child_path.len().min(40);
                    let hex: alloc::string::String = child_path[..n]
                        .iter()
                        .map(|b| alloc::format!("{b:02x}"))
                        .collect();
                    info!("NATIVE-GOP: probe {attempt}: non-matching GOP path hex: {hex}");
                }
            }
            if !belongs {
                continue;
            }
            // Opening the GOP is part of the check, not a bonus (issue
            // #107 review): a child whose GOP cannot be opened is not a
            // verified display path, so it must not count as success.
            // Safety: GetProtocol, read-only mode info.
            match unsafe {
                boot::open_protocol::<GraphicsOutput>(
                    OpenProtocolParams {
                        handle: gop,
                        agent: boot::image_handle(),
                        controller: None,
                    },
                    OpenProtocolAttributes::GetProtocol,
                )
            } {
                Ok(g) => {
                    let (w, h) = g.current_mode_info().resolution();
                    info!(
                        "NATIVE-GOP: verified GOP child {:?} ({}x{}) after {attempt} probe(s)",
                        gop, w, h
                    );
                }
                Err(e) => {
                    info!(
                        "NATIVE-GOP: child {gop:?} extends the target path but GOP open failed {:?} - not verified",
                        e.status()
                    );
                    return Err("gop open on child failed");
                }
            }
            return Ok(gop);
        }
        info!(
            "NATIVE-GOP: probe {attempt}: no GOP child on the target ({} GOP handle(s) on the system)",
            handles.len()
        );
    }
    info!("NATIVE-GOP: no GOP child on the target after Start (3 probes, ~750 ms settle)");
    Err("no gop child on target")
}

/// Restore the VGA forwarding subset + IO|MEM decode captured before the
/// claim, then verify COMMAND ends MEM-on / BME-off (the handoff shape).
fn restore_vga(gpu: &PciIo, initial_attrs: Option<u64>) {
    let vga = initial_attrs.unwrap_or(0) & VGA_ATTR_MASK;
    if vga != 0 {
        let st = gpu.attributes_enable_op(vga);
        info!("NATIVE-GOP: VGA attributes {vga:#x} Enable={st:?}");
    } else {
        // Same distinction as the tx_detach twin: empty capture = the
        // firmware granted no VGA bit, chain-2 structurally dead here.
        info!(
            "NATIVE-GOP: pre-claim VGA subset empty (raw={:#x}) - chain-2 cannot engage on this board; IO|MEMORY-only (0x300) is the usual shape, not proof of no legacy-VGA conflict",
            initial_attrs.unwrap_or(0)
        );
    }
    let st = gpu.attributes_enable_op(DECODE_MASK);
    info!("NATIVE-GOP: IO|MEM decode Enable={st:?}");
    let cmd = gpu.config_read_u16(0x04);
    if cmd & 0x2 == 0 {
        info!("NATIVE-GOP: WARNING command={cmd:#06x} - MEMORY decode off");
    }
    if cmd & 0x4 != 0 {
        gpu.cfg_write_u16(0x04, cmd & !0x4);
        let after = gpu.config_read_u16(0x04);
        info!("NATIVE-GOP: BME cleared for handoff ({cmd:#06x} -> {after:#06x})");
    }
}
