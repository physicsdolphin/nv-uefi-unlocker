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

/// EFI_DECOMPRESS_PROTOCOL (MdePkg/Include/Protocol/Decompress.h).
#[repr(C)]
struct DecompressProto {
    _revision: u32,
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
/// transaction's claim cleared them (see detach_and_claim).
pub fn recover(pci_handle: Handle, gpu: &PciIo, initial_attrs: Option<u64>) -> Result<(), &'static str> {
    info!("NATIVE-GOP: recovery begin (\\tx-native-gop.txt)");

    // 1. Locate the GOP EFI image.
    let (rom, source_note) = locate_source(gpu)?;
    info!("NATIVE-GOP: source = {source_note}");

    // 2. Parse the expansion-ROM chain and take the code-type-0x03 image.
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
    let image_handle = boot::load_image(
        boot::image_handle(),
        boot::LoadImageSource::FromBuffer { buffer: &pe_image, file_path: None },
    )
    .map_err(|e| {
        info!("NATIVE-GOP: LoadImage failed: {:?}", e.status());
        "load_image failed"
    })?;
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
    // BiosVideo cannot be re-selected even under CSM.
    match boot::connect_controller(child, &[], None, false) {
        Ok(()) => info!("NATIVE-GOP: console rebound to the native GOP child"),
        Err(e) => info!("NATIVE-GOP: console rebind failed {:?} (bootmgfw will self-establish)", e.status()),
    }

    // 9. VGA forwarding + decode, never BME.
    restore_vga(gpu, initial_attrs);

    info!("NATIVE-GOP: complete");
    Ok(())
}

/// `\tx-native-gop.txt` first line names a volume file; empty/absent
/// content selects the card's own shadowed ROM.
fn locate_source(gpu: &PciIo) -> Result<(Vec<u8>, alloc::string::String), &'static str> {
    if let Some(content) = crate::tx::read_boot_file("\\tx-native-gop.txt") {
        let name = String::from_utf8_lossy(&content)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if !name.is_empty() {
            let path = if name.starts_with('\\') { name.clone() } else { format!("\\{name}") };
            if let Some(bytes) = crate::tx::read_boot_file(&path) {
                let n = bytes.len();
                return Ok((bytes, alloc::format!("volume file {path} ({n} bytes)")));
            }
            info!("NATIVE-GOP: \\tx-native-gop.txt names {path} but it is not on the boot volume");
            return Err("gop module file missing");
        }
    }
    let Some(rom) = gpu.rom_image() else {
        info!("NATIVE-GOP: PciIo RomImage is empty (no shadowed option ROM)");
        return Err("no rom image");
    };
    Ok((rom.to_vec(), alloc::format!("PciIo RomImage ({} bytes)", rom.len())))
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
        Ok(dst)
    }
}

/// Find the Driver Binding installed by `image_handle` (exactly one) and
/// run Supported/Start against the target PCI handle.
fn bind_explicitly(image_handle: Handle, pci_handle: Handle) -> Result<(), &'static str> {
    let handles = boot::locate_handle_buffer(SearchType::ByProtocol(&DRIVER_BINDING_GUID))
        .map_err(|_| "no driver bindings")?;
    let mut matches = 0usize;
    let mut bound = false;
    for i in 0..handles.len() {
        let Some(binding) = handle_protocol_raw(handles[i], &DRIVER_BINDING_GUID) else {
            continue;
        };
        // Safety: protocol pointers live for the boot services lifetime.
        unsafe {
            if (*binding).image_handle != image_handle {
                continue;
            }
            matches += 1;
            let supported = ((*binding).supported)(binding, pci_handle, null_mut());
            info!(
                "NATIVE-GOP: binding {:?} version={:#x} Supported={supported:?}",
                handles[i],
                (*binding).version
            );
            if supported != uefi::Status::SUCCESS {
                continue;
            }
            let start = ((*binding).start)(binding, pci_handle, null_mut());
            info!("NATIVE-GOP: binding Start={start:?}");
            if start == uefi::Status::SUCCESS {
                bound = true;
            }
        }
    }
    if matches != 1 {
        info!("NATIVE-GOP: expected exactly one Driver Binding from the loaded image, found {matches}");
        return Err("binding identity mismatch");
    }
    if !bound {
        return Err("driver binding start failed");
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
fn verify_gop_child(pci_handle: Handle) -> Result<Handle, &'static str> {
    let handles = boot::locate_handle_buffer(SearchType::ByProtocol(&GraphicsOutput::GUID))
        .map_err(|_| "no gop handles after start")?;
    for i in 0..handles.len() {
        let gop = handles[i];
        if path_belongs_to(
            &device_path_bytes(pci_handle).unwrap_or_default(),
            &device_path_bytes(gop).unwrap_or_default(),
        ) {
            // Safety: GetProtocol, read-only mode info.
            if let Ok(g) = unsafe {
                boot::open_protocol::<GraphicsOutput>(
                    OpenProtocolParams {
                        handle: gop,
                        agent: boot::image_handle(),
                        controller: None,
                    },
                    OpenProtocolAttributes::GetProtocol,
                )
            } {
                let (w, h) = g.current_mode_info().resolution();
                info!("NATIVE-GOP: verified GOP child {:?} ({}x{})", gop, w, h);
            }
            return Ok(gop);
        }
    }
    info!("NATIVE-GOP: no GOP child on the target after Start");
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
        info!("NATIVE-GOP: no VGA attributes captured pre-claim - nothing to restore");
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
