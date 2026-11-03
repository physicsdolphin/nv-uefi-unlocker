//! Read-only chain-load of the OS bootloader (constitution §1.3:
//! LoadImage/StartImage only, never overwrite the ESP).
//!
//! Strategy: probe every SimpleFileSystem volume for the candidate loaders
//! in priority order — Windows bootmgfw first, then the common Linux
//! loader paths, and finally the generic removable-media BOOTX64.EFI as a
//! last resort. Because that generic path is also where THIS application
//! usually lives, the last resort is guarded by a 5-second key window: the
//! user may abort to the firmware shell (the transaction is already
//! complete and logged) and the self-volume is excluded by fingerprint
//! (our own image device path), because the handle-level match is not
//! reliable across firmware implementations.

use alloc::vec::Vec;
use log::info;
use uefi::boot::{self, LoadImageSource, OpenProtocolAttributes, OpenProtocolParams, SearchType};
use uefi::proto::BootPolicy;
use uefi::proto::device_path::DevicePath;
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::{File, FileAttribute, FileMode};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::{cstr16, CStr16, Identify, Result};

/// Candidate OS loaders, tried in order on every SimpleFileSystem volume.
/// bootmgfw first (Windows is the overwhelmingly common target on this
/// tool's audience), Linux distro loaders next, the generic removable
/// default last.
const CANDIDATES: &[&CStr16] = &[
    cstr16!(r"\EFI\Microsoft\Boot\bootmgfw.efi"),
    cstr16!(r"\EFI\ubuntu\shimx64.efi"),
    cstr16!(r"\EFI\ubuntu\grubx64.efi"),
    cstr16!(r"\EFI\fedora\shimx64.efi"),
    cstr16!(r"\EFI\debian\shimx64.efi"),
    cstr16!(r"\EFI\arch\grubx64.efi"),
    cstr16!(r"\EFI\Manjaro\grubx64.efi"),
    cstr16!(r"\EFI\opensuse\shimx64.efi"),
    cstr16!(r"\EFI\grub\grubx64.efi"),
    cstr16!(r"\EFI\systemd\systemd-bootx64.efi"),
    cstr16!(r"\EFI\limine\limine_x64.efi"),
    cstr16!(r"\EFI\shim\shimx64.efi"),
];

/// Generic removable-media default, tried LAST with a 5 s abort window
/// (it may be this very application).
const GENERIC_LAST: &CStr16 = cstr16!(r"\EFI\BOOT\BOOTX64.EFI");

/// Device path node header: type, subtype, u16 length (includes header).
const NODE_TYPE_MEDIA: u8 = 0x04;
const NODE_SUBTYPE_FILE: u8 = 0x04;
const NODE_TYPE_END: u8 = 0x7f;
/// MEDIA node subtype 0x01: Hard Drive (partition number + signature).
const NODE_SUBTYPE_HD: u8 = 0x01;
/// END_ENTIRE node: type 0x7f, subtype 0xff, length 0x0004 (little endian).
const END_ENTIRE: [u8; 4] = [0x7f, 0xff, 0x04, 0x00];

/// NVIDIA (0x10de) function with display base class 0x03.
fn is_nvidia_display(handle: uefi::Handle) -> bool {
    // Safety: GetProtocol, config reads only.
    let Ok(pci) = (unsafe {
        boot::open_protocol::<crate::pci::PciIo>(
            OpenProtocolParams { handle, agent: boot::image_handle(), controller: None },
            OpenProtocolAttributes::GetProtocol,
        )
    }) else {
        return false;
    };
    pci.config_read_u16(0x00) == crate::pci::NVIDIA_VENDOR_ID && pci.config_read_u32(0x08) >> 24 == 0x03
}

/// Chainload with an optional pre-transaction loader image (P2-7, the
/// TOTORONG craft nv_unlock_win 0.2.12 field-verified on AD103): a verified
/// preloaded image starts immediately, skipping the post-FLR volume scan
/// entirely - the allocation-failure window that motivates the preload
/// never gets a chance to strand the chain. Any preload problem (failed
/// load, broken PE header at start time) falls back to the proven full
/// path below, unchanged.
pub fn chain_load_bootmgfw(pre: Option<PreloadedOsLoader>) -> Result<()> {
    info!("PHASE: CHAIN");
    log_handoff_memmap();
    if let Some(pre) = pre.as_ref() {
        if start_preloaded(pre).is_ok() {
            return Ok(());
        }
        info!("CHAIN: preloaded start failed - falling back to the post-transaction volume scan");
    }
    log_firmware_boot_order();
    chain_load_bootmgfw_inner()
}

/// Memory map at handoff time (issue #146): what the OS loader - and
/// winload after it - actually inherits from the chained path. The
/// transaction-time MEMMAP capture in pci.rs predates the final frees and
/// reconnect; this one runs immediately before StartImage, so the residency
/// question (preload + asset cache + pool fragmentation tail below 4GiB)
/// has its read-back on the record in the same log. Read-only; every build.
fn log_handoff_memmap() {
    use uefi::mem::memory_map::{MemoryMap as _, MemoryType};
    let Ok(mmap) = boot::memory_map(MemoryType::BOOT_SERVICES_DATA) else {
        info!("CHAIN-MEMMAP: capture failed");
        return;
    };
    let mut lines = 0u32;
    let mut conv_below4g_mib: u64 = 0;
    let mut resident_below4g_mib: u64 = 0;
    for d in mmap.entries() {
        let end = d.phys_start.saturating_add((d.page_count as u64) * 4096);
        let mib = ((d.page_count as u64) * 4096) >> 20;
        if end <= 0x1_0000_0000 {
            if d.ty == MemoryType::CONVENTIONAL {
                conv_below4g_mib += mib;
            }
            // LOADER_DATA / BOOT_SERVICES_DATA: what our own path keeps
            // resident (image, caches, pool buckets) - winload cannot
            // allocate from these before ExitBootServices.
            if matches!(d.ty, MemoryType::LOADER_DATA | MemoryType::BOOT_SERVICES_DATA) {
                resident_below4g_mib += mib;
            }
        }
        if end >= 0x0800_0000 && lines < 64 {
            info!("CHAIN-MEMMAP: {:#x}-{:#x} ({} MiB) type={:?}", d.phys_start, end, mib, d.ty);
            lines += 1;
        }
    }
    info!(
        "CHAIN-MEMMAP: {conv_below4g_mib} MiB conventional / {resident_below4g_mib} MiB resident (LOADER+BS) below 4GiB ({lines} descriptors shown)"
    );
}

fn chain_load_bootmgfw_inner() -> Result<()> {
    // No PHASE line here: the fallback path already logged "PHASE: CHAIN".
    // The shell's `map -r` equivalent: before enumerating filesystems,
    // recursively reconnect every handle so block devices carry freshly
    // bound FAT/SFS instances (ours may have been torn down or left in an
    // intermediate state by the detach/reconnect storm during the
    // transaction; a fresh UEFI shell re-binds and sees everything).
    // NVIDIA display functions are skipped: tx_detach::reconnect_all already
    // re-bound the vendor GOP (non-recursively), and a recursive connect on
    // the GPU re-binds the console stack over the AMI pathless GOP splitter
    // child that survives the detach - the firmware resets mid-call
    // (ASUS Z790 + 2803). Block devices are what this pass is for.
    if let Ok(handles) = boot::locate_handle_buffer(SearchType::ByProtocol(&crate::pci::PciIo::GUID)) {
        info!("CHAIN: reconnect pass over {} PciIo handles (NVIDIA display skipped)", handles.len());
        for h in handles.iter() {
            if is_nvidia_display(*h) {
                info!("CHAIN: reconnect skipped on NVIDIA display {:?}", h);
                continue;
            }
            let _ = boot::connect_controller(*h, &[], None, true);
        }
        info!("CHAIN: reconnect pass done");
        // Let binding callbacks settle before the SFS enumeration.
        boot::stall(core::time::Duration::from_millis(300));
    }
    let fs_handles = boot::locate_handle_buffer(SearchType::ByProtocol(&SimpleFileSystem::GUID))?;
    info!("CHAIN: {} SimpleFileSystem handles", fs_handles.len());

    // Our own volume is fingerprinted per volume inside open_volume_logged
    // (device-path prefix + loaded-image device match; the plain handle
    // comparison does not hold on every firmware).

    // Pass 0: the stick-root pin (issue #8). `\chain-target.txt`'s first
    // usable line is an EFI path tried on EVERY volume before the built-in
    // candidate order - dual-boot machines put their Linux loader first
    // without waiting for a new loader entry in this file. Not found =
    // fall through to the default order with a loud log line: the chain
    // must never strand the user at the shell because a pin went stale
    // (contrast tx-select-bdf.txt's refuse-to-guess - GPU writes are
    // high-stakes, a boot target is not).
    if let Some(target) = pinned_chain_target() {
        info!("CHAIN: chain-target.txt pins '{target}' - trying it on every volume first");
        match uefi::CString16::try_from(target.as_str()) {
            Ok(target_c) => {
                let target_ref: &CStr16 = &target_c;
                for handle in fs_handles.iter() {
                    let Some((mut dir, _)) = open_volume_logged(*handle) else {
                        continue;
                    };
                    if dir.open(target_ref, FileMode::Read, FileAttribute::empty()).is_ok() {
                        info!("CHAIN: pinned loader found on {:?} ({}): {}", handle, volume_tag(*handle), target);
                        drop(dir);
                        return start_loader(*handle, target_ref);
                    }
                }
                info!("CHAIN: pinned target not found on any volume - falling back to the default candidates");
            }
            Err(_) => {
                info!("CHAIN: pinned path not UCS-2 convertible - falling back to the default candidates");
            }
        }
    }

    // Pass 0.5 (issue #128/#142): the firmware's own boot choice beats
    // enumeration order - a dual-ESP machine chains the leftover drive's
    // stale BCD otherwise. Same precedence as the preload scan: pin >
    // firmware option > built-in candidates.
    if let Some((handle, target)) = firmware_resolved_target(&fs_handles) {
        return start_loader(handle, target.cstr());
    }

    // Pass 1: specific candidates in GLOBAL priority order - bootmgfw on
    // ANY volume first (the normal Windows chain), then the common Linux
    // loaders. Our own volume is not excluded here: on a Windows install
    // stick bootmgfw lives on our own volume and is the correct target.
    let mut missed = alloc::vec![0u8; fs_handles.len()];
    for path in CANDIDATES {
        for (i, handle) in fs_handles.iter().enumerate() {
            let Some((mut dir, _)) = open_volume_logged(*handle) else {
                continue;
            };
            if dir.open(path, FileMode::Read, FileAttribute::empty()).is_ok() {
                info!("CHAIN: loader found on {:?} ({}): {}", handle, volume_tag(*handle), cstr16_lossy(path));
                drop(dir);
                return start_loader(*handle, path);
            }
            missed[i] = missed[i].saturating_add(1);
        }
    }
    for (i, handle) in fs_handles.iter().enumerate() {
        if missed[i] == 0 {
            continue; // already logged as unopenable during the scan
        }
        if let Some((_, is_own)) = open_volume_logged(*handle) {
            info!(
                "CHAIN: no specific candidate on {:?} (own={}, probes={})",
                handle,
                is_own,
                missed[i]
            );
        }
    }

    // Pass 2: the generic removable default, with a 5 s abort window. The
    // self-volume is excluded by fingerprint, so a stick that only contains
    // us does not relaunch itself; the user gets the shell instead. Doing
    // nothing runs the generic loader (the desired default for "whatever
    // the stick was set up to boot").
    for handle in fs_handles.iter() {
        let Some((mut dir, is_own)) = open_volume_logged(*handle) else {
            continue;
        };
        if is_own {
            info!("CHAIN: generic fallback skipped on own volume {:?}", handle);
            continue;
        }
        if dir.open(GENERIC_LAST, FileMode::Read, FileAttribute::empty()).is_err() {
            info!("CHAIN: no generic loader on {:?} (own={})", handle, is_own);
            continue;
        }
        info!("CHAIN: generic loader present on {:?}: {}", handle, cstr16_lossy(GENERIC_LAST));
        drop(dir);
        info!("CHAIN: fallback in 5 s - press ANY KEY to abort to the firmware shell");
        info!("CHAIN: unlock is COMPLETE and logged; the GPU is in a normal cold state -");
        info!("CHAIN: after returning to the shell you may start any OS normally.");
        match crate::keywait::wait_any_key_or_timeout(5) {
            crate::keywait::KeyOrTimeout::Key(k) => {
                info!("CHAIN: key {:?} - aborting to firmware (user choice)", k);
                return Err(uefi::Status::ABORTED.into());
            }
            crate::keywait::KeyOrTimeout::Timeout => info!("CHAIN: 5 s timeout - launching the generic loader"),
        }
        return start_loader(*handle, GENERIC_LAST);
    }

    info!("CHAIN: no OS loader found on any filesystem.");
    info!("CHAIN: unlock is COMPLETE and logged - the GPU is in a normal cold state.");
    info!("CHAIN: boot the machine into its UEFI boot menu / built-in shell and");
    info!("CHAIN: start the desired OS manually; no further tool action is needed.");
    Err(uefi::Status::NOT_FOUND.into())
}

/// Read cap for marker/config files: they are tiny; a larger file
/// truncated to this is still fine for presence checks and first-line
/// path parses.
const MARKER_READ_CAP: usize = 4096;

/// Read a small marker/config file from our own boot volume. Any
/// filesystem error counts as absent: the default semantics must never be
/// upgraded by a probe failure. Absent-file is the normal case and stays
/// silent; volume-level failures are logged (they explain a marker that
/// "should have" engaged but did not).
pub fn boot_volume_read(path: &CStr16) -> Option<alloc::vec::Vec<u8>> {
    let mut fs = match boot::get_image_file_system(boot::image_handle()) {
        Ok(fs) => fs,
        Err(e) => {
            info!("MARKER: image fs open failed: {:?}", e.status());
            return None;
        }
    };
    let mut dir = match fs.open_volume() {
        Ok(d) => d,
        Err(e) => {
            info!("MARKER: open_volume failed: {:?}", e.status());
            return None;
        }
    };
    let file = match dir.open(path, FileMode::Read, FileAttribute::empty()) {
        Ok(f) => f,
        Err(_) => return None,
    };
    let mut file = match file.into_regular_file() {
        Some(f) => f,
        None => return None,
    };
    let mut out = alloc::vec::Vec::new();
    let mut chunk = [0u8; 512];
    loop {
        match file.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                out.extend_from_slice(&chunk[..n]);
                if out.len() >= MARKER_READ_CAP {
                    out.truncate(MARKER_READ_CAP);
                    break;
                }
            }
            Err(e) => {
                info!("MARKER: read failed: {:?}", e.status());
                return None;
            }
        }
    }
    Some(out)
}

/// Does `path` exist on our own boot volume? Marker-file probe for the
/// CONFIRM phase (`\i_know_what_i_am_doing.txt` armed mode) - presence is
/// the signal, content is ignored.
pub fn boot_volume_has(path: &CStr16) -> bool {
    boot_volume_read(path).is_some()
}

/// First numeric token in a marker's content, clamped to `[min, max]`;
/// empty content or an unparsable first line yields `default`. Parsing
/// takes the leading ASCII digits of the first non-empty line ("2\r\n",
/// " 2s", "02" all read as 2); saturation above u64 then clamps to `max`.
/// Textually mirrored in sim/src/gate_parse.rs for shape tests.
fn parse_marker_number(content: &[u8], default: u64, min: u64, max: u64) -> u64 {
    let line = content
        .split(|&b| b == b'\r' || b == b'\n')
        .find(|l| !l.is_empty());
    let Some(line) = line else { return default };
    let digits: &[u8] = {
        let trimmed: &[u8] = {
            let start = line.iter().position(|b| !b.is_ascii_whitespace()).unwrap_or(line.len());
            let end = line.iter().rposition(|b| !b.is_ascii_whitespace()).map_or(0, |i| i + 1);
            &line[start..end.max(start)]
        };
        let stop = trimmed.iter().position(|b| !b.is_ascii_digit()).unwrap_or(trimmed.len());
        &trimmed[..stop]
    };
    if digits.is_empty() {
        return default;
    }
    let value = digits
        .iter()
        .fold(0u64, |acc, b| acc.saturating_mul(10).saturating_add((b - b'0') as u64));
    value.clamp(min, max)
}

/// `\i_know_what_i_am_doing.txt` countdown seconds: first line 1..=30,
/// default 5 (the historical constant) when the content carries no usable
/// number. The marker file is the operator's declaration of intent; the
/// number only shortens the abort window they still own.
pub fn parse_armed_seconds(content: &[u8]) -> u64 {
    parse_marker_number(content, 5, 1, 30)
}

/// `\tx-flush-lines.txt` flush cadence: 0..=4096 count-based flushes per
/// lines, 0 = stage-marker lines only. Default 16 (the historical constant).
pub fn parse_flush_lines(content: &[u8]) -> u64 {
    parse_marker_number(content, 16, 0, 4096)
}

/// Open the volume with one retry (freshly reconnected volumes may need a
/// beat), and report whether this handle is our own volume (device-path
/// fingerprint: our image path's file-name tail is compared against the
/// volume's device path, because the device-handle match is not reliable).
fn open_volume_logged(
    handle: uefi::Handle,
) -> Option<(uefi::proto::media::file::Directory, bool)> {
    // Safety: ByHandleProtocol, protocol lives as long as the call.
    let fs = unsafe {
        boot::open_protocol::<SimpleFileSystem>(
            OpenProtocolParams {
                handle,
                agent: boot::image_handle(),
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
    };
    let mut fs = match fs {
        Ok(fs) => fs,
        Err(e) => {
            info!("CHAIN: open fs {:?} failed: {:?}", handle, e.status());
            return None;
        }
    };
    let is_own = own_volume_by_probe(handle);
    let dir = match fs.open_volume() {
        Ok(d) => d,
        Err(e1) => {
            boot::stall(core::time::Duration::from_millis(200));
            match fs.open_volume() {
                Ok(d) => {
                    info!("CHAIN: open_volume recovered on {:?} after retry (first: {:?})", handle, e1.status());
                    d
                }
                Err(e2) => {
                    info!(
                        "CHAIN: open_volume failed on {:?}: {:?} then {:?} (own={})",
                        handle,
                        e1.status(),
                        e2.status(),
                        is_own
                    );
                    return None;
                }
            }
        }
    };
    Some((dir, is_own))
}

/// The firmware's own boot-option namespace (EFI_GLOBAL_VARIABLE).
const GLOBAL_VARIABLE: uefi::runtime::VariableVendor =
    uefi::runtime::VariableVendor(uefi::guid!("8be4df61-93ca-11d2-aa0d-00e098032b8c"));
/// Logged once per boot, before the first chain decision.
static BOOT_ORDER_LOGGED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Decode UCS-2LE code units until NUL (or cap) into a lossy String.
fn ucs2_lossy_prefix(bytes: &[u8], max_chars: usize) -> alloc::string::String {
    let mut s = alloc::string::String::new();
    let mut i = 0usize;
    while i + 1 < bytes.len() && s.chars().count() < max_chars {
        let c = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
        if c == 0 {
            break;
        }
        s.push(char::from_u32(c as u32).unwrap_or('?'));
        i += 2;
    }
    s
}

/// One-line summary of an EFI_LOAD_OPTION payload: attributes, description,
/// then the device-path list node by node (HD nodes report their partition
/// signature - the exact "which drive" datum; FILE nodes decode their
/// UTF-16 loader path). Never fails outward: anything malformed is logged
/// as a hex prefix instead.
fn describe_boot_option(data: &[u8]) -> alloc::string::String {
    use alloc::format;
    if data.len() < 6 {
        return format!("payload {} bytes (too short)", data.len());
    }
    let attrs = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let mut i = 4usize;
    let mut desc = alloc::string::String::new();
    while i + 1 < data.len() {
        let c = u16::from_le_bytes([data[i], data[i + 1]]);
        if c == 0 {
            i += 2;
            break;
        }
        desc.push(char::from_u32(c as u32).unwrap_or('?'));
        i += 2;
        if desc.chars().count() >= 64 {
            break;
        }
    }
    let mut out = format!("attrs={:#x} desc=\"{desc}\" path=", attrs);
    let mut shown = 0usize;
    let mut off = i;
    loop {
        if off + 4 > data.len() || shown >= 12 {
            break;
        }
        let (t, st) = (data[off], data[off + 1]);
        let len = u16::from_le_bytes([data[off + 2], data[off + 3]]) as usize;
        if len < 4 || off + len > data.len() {
            out.push_str(&format!(" [malformed node at {off:#x}"));
            break;
        }
        let payload = &data[off + 4..off + len];
        match (t, st) {
            (0x7f, _) => {
                out.push_str(" END");
                break;
            }
            (0x04, 0x04) => {
                let p = ucs2_lossy_prefix(payload, 96);
                out.push_str(&format!(" file:\"{p}\""));
            }
            (0x04, 0x01) if payload.len() >= 38 => {
                let part = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                let sig_type = payload[36];
                let sig = payload
                    .iter()
                    .skip(20)
                    .take(16)
                    .map(|b| format!("{b:02x}"))
                    .collect::<alloc::vec::Vec<_>>()
                    .join("");
                out.push_str(&format!(" hd(part{part},sig{sig_type}:{sig})"));
            }
            (t, st) => out.push_str(&format!(" node({t:#04x},{st:#04x},{len})")),
        }
        shown += 1;
        off += len;
    }
    out
}

/// LOG-ONLY dump of the firmware's BootOrder and every Boot#### option
/// (issue #142: the volume scan chains the first bootmgfw in enumeration
/// order, which on a dual-ESP machine can be the OTHER drive's leftover
/// loader; these lines let one log show both what the firmware itself
/// would have booted and what our scan picked instead). No chain decision
/// depends on anything here - every read is best-effort, every failure a
/// one-line log.
fn log_firmware_boot_order() {
    use core::sync::atomic::Ordering;
    if BOOT_ORDER_LOGGED.swap(true, Ordering::Relaxed) {
        return;
    }
    let order_name = match uefi::CString16::try_from("BootOrder") {
        Ok(n) => n,
        Err(_) => return,
    };
    let order: alloc::vec::Vec<u16> = match uefi::runtime::get_variable_boxed(&order_name, &GLOBAL_VARIABLE) {
        Ok((data, _)) => data
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect(),
        Err(e) => {
            info!("CHAIN-BOOT: BootOrder unreadable ({:?}) - firmware boot option context unavailable", e.status());
            return;
        }
    };
    info!("CHAIN-BOOT: BootOrder {} entr{}", order.len(), if order.len() == 1 { "y" } else { "ies" });
    let mut first_windows = None;
    for (rank, num) in order.iter().enumerate() {
        let name = alloc::format!("Boot{:04X}", num);
        let Ok(name_c) = uefi::CString16::try_from(name.as_str()) else {
            continue;
        };
        match uefi::runtime::get_variable_boxed(&name_c, &GLOBAL_VARIABLE) {
            Ok((data, _)) => {
                let summary = describe_boot_option(&data);
                let windows = summary.contains("bootmgfw") || summary.contains("Windows Boot Manager");
                if windows && first_windows.is_none() {
                    first_windows = Some((rank, alloc::string::String::from(&name)));
                }
                info!(
                    "CHAIN-BOOT: [{rank}] {name}: {}{}",
                    summary,
                    if windows { "  <WINDOWS>" } else { "" }
                );
            }
            Err(e) => info!("CHAIN-BOOT: [{rank}] {name} unreadable ({:?})", e.status()),
        }
    }
    match first_windows {
        Some((rank, name)) => info!("CHAIN-BOOT: firmware's first Windows target = [{rank}] {name}"),
        None => info!("CHAIN-BOOT: no Windows Boot Manager among the firmware's boot options"),
    }
}

/// The node sequence of a volume's device path with the END node stripped.
fn volume_path_bytes(handle: uefi::Handle) -> Option<Vec<u8>> {
    // Safety: GetProtocol on the SFS handle, read-only byte view.
    let dp = unsafe {
        boot::open_protocol::<DevicePath>(
            OpenProtocolParams { handle, agent: boot::image_handle(), controller: None },
            OpenProtocolAttributes::GetProtocol,
        )
    }
    .ok()?;
    Some(strip_end_node(dp.as_bytes()).to_vec())
}

/// Decode an HD node payload's signature field into text (GPT GUID or MBR
/// 4-byte ID). Payload layout: part @0..4, signature @20..36, type @36.
fn hd_sig_text(payload: &[u8]) -> Option<alloc::string::String> {
    if payload.len() < 37 {
        return None;
    }
    let sig = &payload[20..36];
    if payload[36] == 0x02 {
        // GPT: the 16-byte signature IS the partition UUID, but UUIDs
        // render with the first three fields byte-swapped.
        let guid = uefi::Guid::from_bytes(sig.try_into().ok()?);
        Some(alloc::format!("{guid}"))
    } else {
        Some(
            sig.iter()
                .take(4)
                .map(|b| alloc::format!("{b:02x}"))
                .collect::<alloc::vec::Vec<_>>()
                .join(""),
        )
    }
}

/// One-line volume identity for the chain logs: the HD node decoded -
/// partition number + signature (GUID form for GPT, 4-byte MBR form
/// otherwise) - so two ESPs on two disks can be told apart AND matched
/// against the boot option dump's hd(part,sig) fields by eye (issue #128
/// self-diagnosis ask). Falls back to a short hex fingerprint when no HD
/// node is present.
fn volume_tag(handle: uefi::Handle) -> alloc::string::String {
    let tag = (|| {
        let bytes = volume_path_bytes(handle)?;
        let mut off = 0usize;
        while off + 4 <= bytes.len() {
            let len = u16::from_le_bytes([bytes[off + 2], bytes[off + 3]]) as usize;
            if len < 4 || off + len > bytes.len() {
                break;
            }
            if bytes[off] == NODE_TYPE_MEDIA && bytes[off + 1] == NODE_SUBTYPE_HD {
                let payload = &bytes[off + 4..off + len];
                let part = payload
                    .get(..4)
                    .map(|p| u32::from_le_bytes([p[0], p[1], p[2], p[3]]))
                    .unwrap_or(0);
                let sig = hd_sig_text(payload)?;
                return Some(alloc::format!("hd(part{part},sig{sig})"));
            }
            off += len;
        }
        let n = bytes.len().min(24);
        Some(
            bytes[..n]
                .iter()
                .map(|b| alloc::format!("{b:02x}"))
                .collect::<alloc::vec::Vec<_>>()
                .join(""),
        )
    })()
    .unwrap_or_else(|| alloc::string::String::from("nopath"));
    alloc::format!("vol={}", tag)
}

/// Diagnostic render of a device-path node sequence: one node per entry,
/// with HD signatures decoded and generic payloads hexed (issue #128
/// self-diagnosis: name the nodes instead of only "matched nothing").
fn describe_nodes(bytes: &[u8]) -> alloc::string::String {
    use alloc::format;
    let mut out = alloc::string::String::new();
    let mut off = 0usize;
    let mut n = 0usize;
    while off + 4 <= bytes.len() && n < 24 {
        let (t, st) = (bytes[off], bytes[off + 1]);
        let len = u16::from_le_bytes([bytes[off + 2], bytes[off + 3]]) as usize;
        if len < 4 || off + len > bytes.len() {
            out.push_str(&format!(" [{n}] malformed(len={len})"));
            break;
        }
        let payload = &bytes[off + 4..off + len];
        match (t, st) {
            (0x7f, _) => out.push_str(&format!(" [{n}] end")),
            (0x04, 0x04) => {
                out.push_str(&format!(" [{n}] file:\"{}\"", ucs2_lossy_prefix(payload, 64)));
            }
            (0x04, 0x01) => {
                let part = payload
                    .get(..4)
                    .map(|p| u32::from_le_bytes([p[0], p[1], p[2], p[3]]))
                    .unwrap_or(0);
                match hd_sig_text(payload) {
                    Some(sig) => out.push_str(&format!(" [{n}] hd(part{part},sig{sig})")),
                    None => out.push_str(&format!(" [{n}] hd(short,len={len})")),
                }
            }
            _ => {
                out.push_str(&format!(" [{n}] {t:#04x}/{st:#04x}/{len}:"));
                for b in payload.iter().take(24) {
                    out.push_str(&format!("{b:02x}"));
                }
                if payload.len() > 24 {
                    out.push_str("..");
                }
            }
        }
        n += 1;
        off += len;
    }
    if out.is_empty() {
        out.push_str(" <empty>");
    }
    out
}

/// One device-path node at `off`: (type, subtype, payload). None when the
/// node header or length runs past the buffer.
fn node_at(bytes: &[u8], off: usize) -> Option<(u8, u8, &[u8])> {
    if off + 4 > bytes.len() {
        return None;
    }
    let len = u16::from_le_bytes([bytes[off + 2], bytes[off + 3]]) as usize;
    if len < 4 || off + len > bytes.len() {
        return None;
    }
    Some((bytes[off], bytes[off + 1], &bytes[off + 4..off + len]))
}

/// Ordinal of the first node where two node sequences differ; equal
/// sequences diverge at their shared node count.
fn first_divergent_node(a: &[u8], b: &[u8]) -> usize {
    let (mut i, mut j, mut n) = (0usize, 0usize, 0usize);
    loop {
        match (node_at(a, i), node_at(b, j)) {
            (Some((ta, sa, pa)), Some((tb, sb, pb))) => {
                if ta != tb || sa != sb || pa != pb {
                    return n;
                }
                i += 4 + pa.len();
                j += 4 + pb.len();
                n += 1;
            }
            _ => return n,
        }
    }
}

/// Parse `\chain-target.txt`: the first non-empty, non-`#` line is the
/// pinned EFI path (leading `\` required, <= 240 chars). Anything else
/// (missing file, unparsable content, no usable line) is None with the
/// reason logged - default order, never a guess.
fn pinned_chain_target() -> Option<alloc::string::String> {
    let bytes = boot_volume_read(cstr16!(r"\chain-target.txt"))?;
    let text = match core::str::from_utf8(&bytes) {
        Ok(t) => t,
        Err(_) => {
            info!("CHAIN: chain-target.txt is not UTF-8 - pin ignored, default order");
            return None;
        }
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if !line.starts_with('\\') {
            info!("CHAIN: chain-target.txt line '{line}' lacks the leading backslash - pin ignored, default order");
            return None;
        }
        if line.chars().count() > 240 {
            info!("CHAIN: chain-target.txt path over 240 chars - pin ignored, default order");
            return None;
        }
        return Some(alloc::string::String::from(line));
    }
    info!("CHAIN: chain-target.txt has no usable line - pin ignored, default order");
    None
}

/// Start the loader at `path` on the volume `handle` (read-only chainload).
fn start_loader(handle: uefi::Handle, path: &CStr16) -> Result<()> {
    // Safety: same ByHandleProtocol discipline for the device path.
    let dp = unsafe {
        boot::open_protocol::<DevicePath>(
            OpenProtocolParams {
                handle,
                agent: boot::image_handle(),
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
    }?;

    let full = match splice_file_node(&dp, path) {
        Some(v) => v,
        None => {
            info!("CHAIN: device path splice failed on {:?}", handle);
            return Err(uefi::Status::UNSUPPORTED.into());
        }
    };

    // Safety: `full` outlives the load_image call; firmware parses it.
    let full_dp = unsafe { DevicePath::from_ffi_ptr(full.as_ptr().cast()) };
    info!("CHAIN: LoadImage (FromDevicePath)");
    // Flush and close the USB log before handing control away; lines
    // logged after this point go to console only.
    crate::logging::finish();
    let loaded = boot::load_image(
        boot::image_handle(),
        LoadImageSource::FromDevicePath {
            device_path: full_dp,
            boot_policy: BootPolicy::ExactMatch,
        },
    )?;
    info!("CHAIN: StartImage -> handing control to the OS loader");
    let r = boot::start_image(loaded);
    info!("CHAIN: loader returned: {:?}", r);
    Ok(())
}

/// The volume portion of our own launch device path: every node BEFORE the
/// first FilePath node. None when the firmware exposes no file path.
fn image_volume_prefix() -> Option<Vec<u8>> {
    // Safety: GetProtocol on our own image handle during boot services.
    let li = unsafe {
        boot::open_protocol::<LoadedImage>(
            OpenProtocolParams {
                handle: boot::image_handle(),
                agent: boot::image_handle(),
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
    }
    .ok()?;
    let path = li.file_path()?;
    let bytes = path.as_bytes();
    let mut off = 0usize;
    while off + 4 <= bytes.len() {
        let len = u16::from_le_bytes([bytes[off + 2], bytes[off + 3]]) as usize;
        if len < 4 || off + len > bytes.len() {
            return None;
        }
        if bytes[off] == NODE_TYPE_MEDIA && bytes[off + 1] == NODE_SUBTYPE_FILE {
            return Some(bytes[..off].to_vec());
        }
        if bytes[off] == NODE_TYPE_END {
            return None;
        }
        off += len;
    }
    None
}

/// The device handle our image was loaded from, if the firmware reports one.
fn own_device_handle() -> Option<uefi::Handle> {
    // Safety: GetProtocol on our own image handle during boot services.
    let li = unsafe {
        boot::open_protocol::<LoadedImage>(
            OpenProtocolParams {
                handle: boot::image_handle(),
                agent: boot::image_handle(),
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
    }
    .ok()?;
    li.device()
}

/// The node sequence of `bytes` with the END node stripped.
fn dp_before_end(bytes: &[u8]) -> &[u8] {
    let mut off = 0usize;
    while off + 4 <= bytes.len() {
        let len = u16::from_le_bytes([bytes[off + 2], bytes[off + 3]]) as usize;
        if len < 4 || off + len > bytes.len() {
            break;
        }
        if bytes[off] == NODE_TYPE_END {
            break;
        }
        off += len;
    }
    &bytes[..off]
}

/// Fingerprint check: is this volume the one our image was started from?
/// Two independent signals, because neither holds on every firmware:
/// (a) the volume device path is a byte prefix of our own launch path - this
/// also covers the case that matters, the generic BOOTX64.EFI being our own
/// file (we must never relaunch ourselves); (b) the loaded-image device
/// handle names this very handle.
fn own_volume_by_probe(handle: uefi::Handle) -> bool {
    if let Some(prefix) = image_volume_prefix() {
        if !prefix.is_empty() {
            // Safety: ByHandleProtocol discipline.
            let dp = unsafe {
                boot::open_protocol::<DevicePath>(
                    OpenProtocolParams {
                        handle,
                        agent: boot::image_handle(),
                        controller: None,
                    },
                    OpenProtocolAttributes::GetProtocol,
                )
            };
            if let Ok(dp) = dp {
                if dp_before_end(dp.as_bytes()) == prefix.as_slice() {
                    return true;
                }
            }
        }
    }
    if let Some(dev) = own_device_handle() {
        if dev == handle {
            return true;
        }
    }
    false
}

/// `base` + FilePath(path) + END_ENTIRE, as raw device-path bytes.
fn splice_file_node(base: &DevicePath, path: &CStr16) -> Option<Vec<u8>> {
    let data = base.as_bytes();
    if data.len() < 4 || data[data.len() - 4..] != END_ENTIRE {
        return None; // unexpected layout; refuse to splice
    }
    let mut out = Vec::with_capacity(data.len() + 8 + 64);
    out.extend_from_slice(&data[..data.len() - 4]); // strip END_ENTIRE

    let mut name: Vec<u16> = path.as_slice().iter().map(|c| u16::from(*c)).collect();
    name.push(0); // NUL terminator
    let node_len = (4 + 2 * name.len()) as u16;
    out.push(NODE_TYPE_MEDIA);
    out.push(NODE_SUBTYPE_FILE);
    out.extend_from_slice(&node_len.to_le_bytes());
    for u in name {
        out.extend_from_slice(&u.to_le_bytes());
    }

    out.extend_from_slice(&END_ENTIRE);
    Some(out)
}

// ---------------------------------------------------------------------------
// P2-7: pre-transaction loader preload.
//
// TOTORONG's craft, field-verified by nv_unlock_win 0.2.12 (AD103, +63/66%
// real run): LoadImage the OS loader BEFORE the transaction, StartImage it
// AFTER the reconnect. The chain then cannot fail on a post-FLR LoadImage
// (firmware allocation landscape degraded), and the post-FLR SFS storm is
// skipped entirely. Risk model: the resident image pages live through the
// GPU's DMA window - gross corruption is caught by a structural PE header
// probe right before StartImage (relocation-proof: MZ + e_lfanew -> "PE\0\0",
// never patched by the loader's relocation pass). Both peers have shipped
// this shape across many real runs without an image-corruption incident.
// Opt-out marker: \tx-no-preload.txt keeps the legacy post-transaction path.
// ---------------------------------------------------------------------------

/// An OS loader image loaded before the transaction.
pub struct PreloadedOsLoader {
    handle: uefi::Handle,
    path: TargetPath,
}

/// The chain target as found: the pinned path (already UCS-2 converted), a
/// built-in candidate (static), or the firmware BootOrder's own choice
/// (issue #128/#142). Avoids a String -> CStr16 round-trip.
enum TargetPath {
    Pinned(uefi::CString16, alloc::string::String),
    BuiltIn(&'static CStr16),
    Firmware(uefi::CString16, alloc::string::String),
}

impl TargetPath {
    fn cstr(&self) -> &CStr16 {
        match self {
            TargetPath::Pinned(c, _) => c,
            TargetPath::BuiltIn(p) => p,
            TargetPath::Firmware(c, _) => c,
        }
    }
    fn label(&self) -> alloc::string::String {
        match self {
            TargetPath::Pinned(_, s) => s.clone(),
            TargetPath::BuiltIn(p) => cstr16_lossy(p),
            TargetPath::Firmware(_, s) => s.clone(),
        }
    }
}

/// The device scope of a boot option's FilePathList (issue #128). Three
/// shapes reach real firmware; two of them are "short forms" (UEFI 2.10
/// §3.3) that never align with a live volume path node 0 and therefore
/// cannot be resolved by prefix comparison:
///
/// - `Volume`: full device path prefix (head-aligned). An empty `path` is
///   the bulk form - firmware-created volume entries carrying no FILE node
///   (the QEMU/OVMF control showed the shape; bcdedit's "path" column is
///   not part of the NVRAM option): the firmware chose the VOLUME, the
///   loader finds the loader file among its candidates there;
/// - `ShortFormHd`: the list starts at an HD node - exactly what
///   `bcfg boot addp` writes ("Only the portion of the device path
///   starting with the hard drive partition is placed in the option").
///   Firmware expands it in `BmExpandPartitionDevicePath` by partition
///   identity, never by alignment. run17 (#128) is this shape: a 42-byte
///   single HD node against 66-82-byte volume paths;
/// - `ShortFormFile`: a bare FilePath node - firmware searches every
///   volume for it (`BmExpandFileDevicePath`).
///
/// None only when the list is malformed or empty.
enum OptionScope {
    Volume { prefix: alloc::vec::Vec<u8>, path: alloc::vec::Vec<u8> },
    ShortFormHd {
        /// The HD node itself.
        hd: alloc::vec::Vec<u8>,
        /// Nodes between the HD node and the FILE node (nested partitions;
        /// empty in practice).
        tail: alloc::vec::Vec<u8>,
        path: alloc::vec::Vec<u8>,
    },
    ShortFormFile { path: alloc::vec::Vec<u8> },
}

fn parse_option_device_scope(data: &[u8]) -> Option<OptionScope> {
    let mut i = 4usize; // attributes
    while i + 1 < data.len() {
        let c = u16::from_le_bytes([data[i], data[i + 1]]);
        i += 2;
        if c == 0 {
            break;
        }
    }
    let list = i;
    // Node boundaries up to END so the shape can be classified before the
    // FILE split: a leading FILE node means a bare short form, not an
    // empty device prefix.
    let mut nodes: alloc::vec::Vec<(usize, usize)> = alloc::vec::Vec::new();
    let mut off = i;
    while off + 4 <= data.len() {
        let t = data[off];
        let len = u16::from_le_bytes([data[off + 2], data[off + 3]]) as usize;
        if len < 4 || off + len > data.len() {
            return None;
        }
        if t == NODE_TYPE_END {
            break;
        }
        nodes.push((off, off + len));
        off += len;
    }
    if nodes.is_empty() {
        return None;
    }
    let file_node = nodes
        .iter()
        .position(|(s, _)| data[*s] == NODE_TYPE_MEDIA && data[*s + 1] == NODE_SUBTYPE_FILE);
    if file_node == Some(0) {
        let (s, e) = nodes[0];
        let path = ucs2_lossy_prefix(&data[s + 4..e], 240);
        if path.is_empty() || !path.starts_with('\\') {
            return None;
        }
        return Some(OptionScope::ShortFormFile { path: path.into_bytes() });
    }
    let (device_end, path) = match file_node {
        Some(k) => {
            let (s, e) = nodes[k];
            let path = ucs2_lossy_prefix(&data[s + 4..e], 240);
            if path.is_empty() || !path.starts_with('\\') {
                return None;
            }
            (s, path.into_bytes())
        }
        // No FILE node: the whole node list is the device scope (bulk).
        None => (off, alloc::vec::Vec::new()),
    };
    if device_end <= list {
        return None;
    }
    if data[list] == NODE_TYPE_MEDIA && data[list + 1] == NODE_SUBTYPE_HD {
        let hd_end = nodes[0].1;
        return Some(OptionScope::ShortFormHd {
            hd: data[list..hd_end].to_vec(),
            tail: data[hd_end..device_end].to_vec(),
            path,
        });
    }
    Some(OptionScope::Volume { prefix: data[list..device_end].to_vec(), path })
}

/// HD node identity match, EDK2 `BmMatchPartitionDevicePathNode`
/// semantics (MdeModulePkg/Library/UefiBootManagerLib/BmBoot.c): walk the
/// WHOLE volume path for an HD node and compare the four identity fields
/// only - partition number, MBR type, signature type, 16-byte signature.
/// Start/size are not identity (they move when a partition is resized or
/// re-created; firmware never compares them). GPT signatures are the
/// partition GUID, not the disk GUID.
fn hd_identity_match_in(bytes: &[u8], option_hd: &[u8], option_tail: &[u8]) -> bool {
    // 4 header + 4 part + 8 start + 8 size + 16 sig + 2 type bytes.
    const HD_LEN: usize = 42;
    if option_hd.len() < HD_LEN {
        return false;
    }
    let vol = strip_end_node(bytes);
    let mut off = 0usize;
    while off + 4 <= vol.len() {
        let len = u16::from_le_bytes([vol[off + 2], vol[off + 3]]) as usize;
        if len < 4 || off + len > vol.len() {
            return false;
        }
        if vol[off] == NODE_TYPE_MEDIA && vol[off + 1] == NODE_SUBTYPE_HD && len >= HD_LEN {
            let node = &vol[off..off + len];
            // Nodes the option carries AFTER the HD node (nested
            // partitions) must line up with the volume's remainder; the
            // empty tail is the normal case and trivially true.
            if node[4..8] == option_hd[4..8]
                && node[24..40] == option_hd[24..40]
                && node[40] == option_hd[40]
                && node[41] == option_hd[41]
                && vol[off + len..].starts_with(option_tail)
            {
                return true;
            }
        }
        off += len;
    }
    false
}

/// `hd_identity_match_in` against a live volume handle.
fn hd_identity_matches(handle: uefi::Handle, option_hd: &[u8], option_tail: &[u8]) -> bool {
    // Safety: GetProtocol discipline for a byte view of the device path.
    let Ok(dp) = (unsafe {
        boot::open_protocol::<DevicePath>(
            OpenProtocolParams { handle, agent: boot::image_handle(), controller: None },
            OpenProtocolAttributes::GetProtocol,
        )
    }) else {
        return false;
    };
    hd_identity_match_in(dp.as_bytes(), option_hd, option_tail)
}

/// `hd(partN,sig...)` for logs - the option's own identity fields.
fn describe_hd(node: &[u8]) -> alloc::string::String {
    let part = if node.len() >= 8 {
        u32::from_le_bytes([node[4], node[5], node[6], node[7]])
    } else {
        0
    };
    match hd_sig_text(node.get(4..).unwrap_or(&[])) {
        Some(sig) => alloc::format!("hd(part{part},sig{sig})"),
        None => alloc::format!("hd(part{part})"),
    }
}

/// Open the chosen volume and take `path` verbatim, or run the bulk
/// candidate scan when the option named no file.
fn chain_on_option_volume(
    handle: uefi::Handle,
    num: u16,
    rank: usize,
    path: Option<&str>,
) -> Option<(uefi::Handle, TargetPath)> {
    let (mut dir, _) = open_volume_logged(handle)?;
    match path {
        Some(p) => {
            let Ok(path_c) = uefi::CString16::try_from(p) else { return None };
            match dir.open(path_ref(&path_c), FileMode::Read, FileAttribute::empty()) {
                Ok(_) => {
                    info!(
                        "CHAIN-BOOT: firmware resolve: [{rank}] Boot{num:04X} -> '{}' on the firmware's volume - chaining its choice",
                        p
                    );
                    Some((handle, TargetPath::Firmware(path_c, alloc::string::String::from(p))))
                }
                Err(e) => {
                    info!(
                        "CHAIN-BOOT: firmware resolve: [{rank}] '{}' does not open on {:?} ({}) ({:?}) - skipped",
                        p,
                        handle,
                        volume_tag(handle),
                        e.status()
                    );
                    None
                }
            }
        }
        None => {
            // Bulk form (no FILE node - the common real-firmware shape):
            // the firmware chose the VOLUME; run the candidate scan on it.
            // The volume's own removable default is the final probe: a bulk
            // entry means "boot whatever this volume is set up to boot".
            info!(
                "CHAIN-BOOT: firmware resolve: [{rank}] Boot{num:04X} bulk device scope matched {:?} ({}) - scanning its candidates",
                handle,
                volume_tag(handle)
            );
            let mut picked: Option<&'static CStr16> = None;
            for cand in CANDIDATES.iter().copied().chain(core::iter::once(GENERIC_LAST)) {
                if dir.open(cand, FileMode::Read, FileAttribute::empty()).is_ok() {
                    picked = Some(cand);
                    break;
                }
            }
            let cand = picked?;
            let cand_str = cstr16_lossy(cand);
            let Ok(cand_c) = uefi::CString16::try_from(cand_str.as_str()) else { return None };
            info!(
                "CHAIN-BOOT: firmware resolve: [{rank}] Boot{num:04X} bulk device -> '{}' found on the firmware's volume - chaining its choice",
                cand_str
            );
            Some((handle, TargetPath::Firmware(cand_c, cand_str)))
        }
    }
}

/// Decode an option's loader path, logging the non-UTF-8 case. Empty is
/// `None` (bulk: never feed `""` to `Dir::open`, it INVALID_PARAMETERs -
/// run8 control).
fn option_file_path(path: &[u8], num: u16, rank: usize) -> Option<alloc::string::String> {
    if path.is_empty() {
        return None;
    }
    match core::str::from_utf8(path) {
        Ok(s) => Some(alloc::string::String::from(s)),
        Err(_) => {
            info!(
                "CHAIN-BOOT: firmware resolve: [{rank}] Boot{num:04X} loader path is not UTF-8 - volume candidate scan"
            );
            None
        }
    }
}

/// Does this SFS volume's device path match the option's volume prefix?
/// The prefix must be a byte-prefix of the option's FilePathList (both are
/// complete node sequences, so byte-prefix == node-aligned-prefix). Two
/// ESPs on one controller differ inside the HD node, so exactly one
/// volume matches.
fn sfs_handle_matches_prefix(handle: uefi::Handle, prefix: &[u8]) -> bool {
    if prefix.is_empty() {
        return false;
    }
    // Safety: GetProtocol discipline for a byte view of the device path.
    let Ok(dp) = (unsafe {
        boot::open_protocol::<DevicePath>(
            OpenProtocolParams { handle, agent: boot::image_handle(), controller: None },
            OpenProtocolAttributes::GetProtocol,
        )
    }) else {
        return false;
    };
    let bytes = dp.as_bytes();
    // as_bytes() includes the trailing END node; the option's FilePathList
    // prefix stops before its FILE node. Compare node bodies only.
    let vol = strip_end_node(bytes);
    !vol.is_empty() && vol.len() <= prefix.len() && vol == &prefix[..vol.len()]
}

/// Drop a trailing END-of-node sequence (any 0x7F-type node at the tail).
fn strip_end_node(bytes: &[u8]) -> &[u8] {
    let mut off = 0usize;
    while off + 4 <= bytes.len() {
        let len = u16::from_le_bytes([bytes[off + 2], bytes[off + 3]]) as usize;
        if len < 4 || off + len > bytes.len() {
            return &bytes[..off];
        }
        if bytes[off] == NODE_TYPE_END {
            return &bytes[..off];
        }
        off += len;
    }
    &bytes[..off]
}

/// Resolve the loader the FIRMWARE itself would boot next: walk BootOrder,
/// skip our own boot entry (BootCurrent) and our own volume (fingerprint),
/// take the first option whose FilePathList names a plain file that opens
/// on the matching volume. This is the machine owner's configured boot
/// preference - the dual-ESP wrong-ESP pick (#128/#142: leftover drive's
/// stale BCD -> 0xc000000f/0xc0000225) cannot survive it. Precedence:
/// operator pin > this > enumeration-order scan. Every failure is a
/// logged no-op and falls through.
/// The full volume-divergence table prints once per boot; every further
/// no-match firmware entry collapses to a one-line summary (issue: the
/// Z170 round's three messaging-scope entries × 8 volumes burned ~8 s and
/// most of the log).
static BOOT_DIVERGENCE_DUMPED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

fn firmware_resolved_target(fs_handles: &[uefi::Handle]) -> Option<(uefi::Handle, TargetPath)> {
    use alloc::format;
    let boot_current: Option<u16> = (|| {
        let Ok(n) = uefi::CString16::try_from("BootCurrent") else { return None };
        let Ok((data, _)) = uefi::runtime::get_variable_boxed(&n, &GLOBAL_VARIABLE) else { return None };
        if data.len() < 2 { return None; }
        Some(u16::from_le_bytes([data[0], data[1]]))
    })();
    let Ok(order_name) = uefi::CString16::try_from("BootOrder") else { return None };
    let order: alloc::vec::Vec<u16> = match uefi::runtime::get_variable_boxed(&order_name, &GLOBAL_VARIABLE) {
        Ok((data, _)) => data
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect(),
        Err(e) => {
            info!("CHAIN-BOOT: firmware resolve: BootOrder unreadable ({:?}) - volume scan fallback", e.status());
            return None;
        }
    };
    info!(
        "CHAIN-BOOT: firmware resolve: {} entr{}, self=Boot{:04X}{}",
        order.len(),
        if order.len() == 1 { "y" } else { "ies" },
        boot_current.unwrap_or(0xFFFF),
        if boot_current.is_none() { " (unreadable)" } else { "" }
    );
    for (rank, num) in order.iter().enumerate() {
        if Some(*num) == boot_current {
            info!("CHAIN-BOOT: firmware resolve: [{rank}] Boot{:04X} is this boot (self) - skipped", num);
            continue;
        }
        let Ok(name_c) = uefi::CString16::try_from(format!("Boot{:04X}", num).as_str()) else {
            continue;
        };
        let Ok((data, _)) = uefi::runtime::get_variable_boxed(&name_c, &GLOBAL_VARIABLE) else {
            info!("CHAIN-BOOT: firmware resolve: [{rank}] Boot{:04X} unreadable - skipped", num);
            continue;
        };
        let Some(scope) = parse_option_device_scope(&data) else {
            info!(
                "CHAIN-BOOT: firmware resolve: [{rank}] Boot{:04X} carries no parsable device path (FV-only or malformed) - skipped",
                num
            );
            continue;
        };
        match scope {
            OptionScope::Volume { prefix, path } => {
                let file_path = option_file_path(&path, *num, rank);
                let mut matched_volume = false;
                for handle in fs_handles.iter() {
                    if !sfs_handle_matches_prefix(*handle, &prefix) {
                        continue;
                    }
                    matched_volume = true;
                    if own_volume_by_probe(*handle) {
                        info!(
                            "CHAIN-BOOT: firmware resolve: [{rank}] Boot{:04X} resolves to OUR volume {:?} ({}) - skipped",
                            num,
                            handle,
                            volume_tag(*handle)
                        );
                        break;
                    }
                    match chain_on_option_volume(*handle, *num, rank, file_path.as_deref()) {
                        Some(target) => return Some(target),
                        None => break,
                    }
                }
                if !matched_volume {
                    // Self-diagnosis (issue #128): the option's node list next
                    // to every live volume path, plus the first divergence
                    // ordinal - "which byte of which node differs" in one log
                    // block. Without this a systematic mismatch (stored path
                    // vs live enumeration) only ever reads as "matched
                    // nothing". Z170 field round: firmware entries whose
                    // scope is a messaging device (UART etc., type 0x05)
                    // never match any volume - full-dumping 8 volumes per
                    // entry burned ~8 s and most of the log across three
                    // such entries, so the full block prints ONCE per boot;
                    // every further no-match entry gets a one-line summary.
                    if BOOT_DIVERGENCE_DUMPED.swap(true, core::sync::atomic::Ordering::Relaxed) {
                        info!(
                            "CHAIN-BOOT: firmware resolve: [{rank}] Boot{:04X} matched no SimpleFileSystem volume (scope {} B, {} node(s); full divergence table already printed this boot)",
                            num,
                            prefix.len(),
                            describe_nodes(&prefix).matches('[').count()
                        );
                        continue;
                    }
                    info!(
                        "CHAIN-BOOT: firmware resolve: [{rank}] Boot{:04X} matched no SimpleFileSystem volume - prefix nodes:{}",
                        num,
                        describe_nodes(&prefix)
                    );
                    for (vi, handle) in fs_handles.iter().enumerate() {
                        let Some(vol) = volume_path_bytes(*handle) else { continue };
                        let div = first_divergent_node(&prefix, &vol);
                        info!(
                            "CHAIN-BOOT: fs[{vi}] {:?} {} | first divergence node {div} (prefix {} B, volume {} B):{}",
                            handle,
                            volume_tag(*handle),
                            prefix.len(),
                            vol.len(),
                            describe_nodes(&vol)
                        );
                    }
                }
            }
            OptionScope::ShortFormHd { hd, tail, path } => {
                // UEFI §3.3 short form (what `bcfg boot addp` writes):
                // firmware expands by partition identity, not by node
                // alignment. Head-prefix comparison can never match here -
                // node 0 is HD against PciRoot on every volume (run17).
                let file_path = option_file_path(&path, *num, rank);
                let ident = describe_hd(&hd);
                info!(
                    "CHAIN-BOOT: firmware resolve: [{rank}] Boot{:04X} short-form HD option {} - matching partition identity (firmware BmExpandPartitionDevicePath)",
                    num, ident
                );
                let mut matched_volume = false;
                for handle in fs_handles.iter() {
                    if !hd_identity_matches(*handle, &hd, &tail) {
                        continue;
                    }
                    matched_volume = true;
                    if own_volume_by_probe(*handle) {
                        info!(
                            "CHAIN-BOOT: firmware resolve: [{rank}] Boot{:04X} short-form HD resolves to OUR volume {:?} ({}) - skipped",
                            num,
                            handle,
                            volume_tag(*handle)
                        );
                        break;
                    }
                    info!(
                        "CHAIN-BOOT: firmware resolve: [{rank}] Boot{:04X} short-form HD identity match on {:?} ({}) - not by path alignment",
                        num,
                        handle,
                        volume_tag(*handle)
                    );
                    if !tail.is_empty() {
                        info!(
                            "CHAIN-BOOT: firmware resolve: [{rank}] Boot{:04X} HD option carries {} trailing node byte(s) - file opened on the matched volume",
                            num,
                            tail.len()
                        );
                    }
                    match chain_on_option_volume(*handle, *num, rank, file_path.as_deref()) {
                        Some(target) => return Some(target),
                        None => break,
                    }
                }
                if !matched_volume {
                    info!(
                        "CHAIN-BOOT: firmware resolve: [{rank}] Boot{:04X} short-form HD identity {} matched no SimpleFileSystem volume",
                        num, ident
                    );
                }
            }
            OptionScope::ShortFormFile { path } => {
                // UEFI §3.3 short form: bare FilePath node. Firmware
                // searches every volume and takes the first holder
                // (`BmExpandFileDevicePath`); our own volume is kept only
                // as the last resort so it cannot beat a real target.
                let Some(p) = option_file_path(&path, *num, rank) else { continue };
                let Ok(path_c) = uefi::CString16::try_from(p.as_str()) else { continue };
                info!(
                    "CHAIN-BOOT: firmware resolve: [{rank}] Boot{:04X} short-form FILE option '{}' - searching every volume (firmware BmExpandFileDevicePath)",
                    num, p
                );
                let mut own: Option<uefi::Handle> = None;
                let mut chosen: Option<uefi::Handle> = None;
                for handle in fs_handles.iter() {
                    let Some((mut dir, _)) = open_volume_logged(*handle) else { continue };
                    if dir.open(path_ref(&path_c), FileMode::Read, FileAttribute::empty()).is_err() {
                        continue;
                    }
                    if own_volume_by_probe(*handle) {
                        info!(
                            "CHAIN-BOOT: firmware resolve: [{rank}] '{}' also opens on OUR volume {:?} ({}) - last resort",
                            p,
                            handle,
                            volume_tag(*handle)
                        );
                        own = Some(*handle);
                        continue;
                    }
                    info!(
                        "CHAIN-BOOT: firmware resolve: [{rank}] '{}' found on {:?} ({}) - chaining its choice",
                        p,
                        handle,
                        volume_tag(*handle)
                    );
                    chosen = Some(*handle);
                    break;
                }
                if let Some(handle) = chosen {
                    return Some((handle, TargetPath::Firmware(path_c, p)));
                }
                if let Some(handle) = own {
                    info!(
                        "CHAIN-BOOT: firmware resolve: [{rank}] Boot{:04X} chaining its choice from OUR volume (only holder)",
                        num
                    );
                    return Some((handle, TargetPath::Firmware(path_c, p)));
                }
                info!(
                    "CHAIN-BOOT: firmware resolve: [{rank}] Boot{:04X} short-form FILE option opens on no volume - skipped",
                    num
                );
            }
        }
    }
    info!("CHAIN-BOOT: firmware resolve: no resolvable firmware option - volume scan fallback");
    None
}

/// Borrow helper keeping the long match arm tidy.
fn path_ref(c: &uefi::CString16) -> &CStr16 {
    c
}

/// Lossy UCS-2 decode of a CStr16 (format! on uefi-rs 0.41's CStr16
/// Display rendered empty in the field; this is the explicit path).
fn cstr16_lossy(c: &CStr16) -> alloc::string::String {
    let mut s = alloc::string::String::new();
    for unit in c.as_slice() {
        let u: u16 = (*unit).into();
        if u == 0 {
            break;
        }
        s.push(char::from_u32(u as u32).unwrap_or('?'));
    }
    s
}

/// The specific-candidate scan (pinned pass 0 + built-in candidates pass 1),
/// in preload form. Deliberately separate from the proven post-transaction
/// scan in `chain_load_bootmgfw_inner` (no missed-count diagnostics, no
/// generic-pass-2 key window): converging them is a follow-up refactor, not
/// something to do surgically on a field-proven flow.
fn find_loader_target() -> Option<(uefi::Handle, TargetPath)> {
    let fs_handles = boot::locate_handle_buffer(SearchType::ByProtocol(&SimpleFileSystem::GUID)).ok()?;
    if let Some(target) = pinned_chain_target() {
        if let Ok(target_c) = uefi::CString16::try_from(target.as_str()) {
            let target_ref: &CStr16 = &target_c;
            for handle in fs_handles.iter() {
                let Some((mut dir, _)) = open_volume_logged(*handle) else {
                    continue;
                };
                if dir.open(target_ref, FileMode::Read, FileAttribute::empty()).is_ok() {
                    return Some((*handle, TargetPath::Pinned(target_c, target)));
                }
            }
        }
    }
    // Pass 0.5 (issue #128/#142): the firmware's own boot choice beats
    // enumeration order - a dual-ESP machine chains the leftover drive's
    // stale BCD otherwise.
    if let Some((handle, target)) = firmware_resolved_target(&fs_handles) {
        return Some((handle, target));
    }
    for path in CANDIDATES {
        for handle in fs_handles.iter() {
            let Some((mut dir, _)) = open_volume_logged(*handle) else {
                continue;
            };
            if dir.open(path, FileMode::Read, FileAttribute::empty()).is_ok() {
                return Some((*handle, TargetPath::BuiltIn(path)));
            }
        }
    }
    None
}

/// `LoadedImage` base/size via the raw protocol (uefi 0.41 exposes no
/// accessors; `repr(transparent)` makes the cast layout-identical).
fn loaded_image_span(handle: uefi::Handle) -> Option<(usize, u64)> {
    // Safety: GetProtocol discipline; the reference lives for this call only.
    let li = unsafe {
        boot::open_protocol::<LoadedImage>(
            OpenProtocolParams {
                handle,
                agent: boot::image_handle(),
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
    }
    .ok()?;
    let raw = &*li as *const LoadedImage as *const uefi_raw::protocol::loaded_image::LoadedImageProtocol;
    // Safety: protocol memory, fields documented as the loaded-image span.
    unsafe { Some(((*raw).image_base as usize, (*raw).image_size)) }
}

/// Structural PE header check (relocation-proof): MZ magic, sane e_lfanew,
/// PE\0\0 signature. Reads only within [base, base+size).
fn pe_header_ok(base: usize, size: u64) -> bool {
    if size < 0x40 {
        return false;
    }
    let p = base as *const u8;
    // Safety: inside the firmware-allocated image span.
    let b = unsafe { core::ptr::read(p as *const [u8; 0x40]) };
    if &b[0..2] != b"MZ" {
        return false;
    }
    let lfanew = u32::from_le_bytes([b[0x3c], b[0x3d], b[0x3e], b[0x3f]]) as usize;
    if lfanew < 0x40 || lfanew as u64 + 4 > size {
        return false;
    }
    // Safety: inside the span (checked above).
    let sig = unsafe { core::ptr::read(p.add(lfanew) as *const [u8; 4]) };
    sig == *b"PE\x00\x00"
}

/// LoadImage the OS loader now, before the transaction touches the GPU.
/// None (with a logged reason) keeps the legacy post-transaction chain.
pub fn preload_os_loader() -> Option<PreloadedOsLoader> {
    if boot_volume_has(cstr16!(r"\tx-no-preload.txt")) {
        info!("CHAIN-PRE: \\tx-no-preload.txt present - preload disabled, legacy chain path");
        return None;
    }
    info!("CHAIN-PRE: pre-transaction loader preload begin");
    let (handle, target) = find_loader_target()?;
    let path = target.label();
    let target_ref: &CStr16 = target.cstr();
    // Safety: ByHandleProtocol discipline for the device path.
    let dp = unsafe {
        boot::open_protocol::<DevicePath>(
            OpenProtocolParams {
                handle,
                agent: boot::image_handle(),
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
    }
    .ok()?;
    let full = splice_file_node(&dp, target_ref)?;
    // Safety: `full` outlives the load_image call; firmware parses it.
    let full_dp = unsafe { DevicePath::from_ffi_ptr(full.as_ptr().cast()) };
    info!("CHAIN-PRE: LoadImage '{}' (FromDevicePath)", path);
    match boot::load_image(
        boot::image_handle(),
        LoadImageSource::FromDevicePath {
            device_path: full_dp,
            boot_policy: BootPolicy::ExactMatch,
        },
    ) {
        Ok(img) => match loaded_image_span(img) {
            Some((base, size)) if pe_header_ok(base, size) => {
                info!(
                    "CHAIN-PRE: '{}' resident at {:#x} ({size:#x} bytes), PE header probe ok",
                    path, base
                );
                Some(PreloadedOsLoader { handle: img, path: target })
            }
            other => {
                info!(
                    "CHAIN-PRE: loaded image fails the PE header probe right after LoadImage ({other:?}) - discarding, legacy chain path"
                );
                let _ = boot::unload_image(img);
                None
            }
        },
        Err(e) => {
            info!("CHAIN-PRE: LoadImage failed: {:?} - legacy chain path", e.status());
            None
        }
    }
}

/// Verify the preloaded image still holds a valid PE header, then hand
/// control to it. Failure = image suspect (discarded) and the caller falls
/// back to the full post-transaction chain.
fn start_preloaded(pre: &PreloadedOsLoader) -> Result<()> {
    match loaded_image_span(pre.handle) {
        Some((base, size)) if pe_header_ok(base, size) => {
            info!(
                "CHAIN-PRE: start-time PE header probe ok - StartImage the preloaded '{}'",
                pre.path.label()
            );
            // Flush and close the USB log before handing control away.
            crate::logging::finish();
            let r = boot::start_image(pre.handle);
            info!("CHAIN-PRE: preloaded loader returned: {:?}", r);
            Ok(())
        }
        other => {
            info!(
                "CHAIN-PRE: start-time PE header probe failed ({other:?}) - image suspect, unloading"
            );
            let _ = boot::unload_image(pre.handle);
            Err(uefi::Status::INCOMPATIBLE_VERSION.into())
        }
    }
}
