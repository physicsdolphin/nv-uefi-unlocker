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

pub fn chain_load_bootmgfw() -> Result<()> {
    info!("PHASE: CHAIN");
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
                        info!("CHAIN: pinned loader found on {:?}: {}", handle, target);
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
                info!("CHAIN: loader found on {:?}: {}", handle, path);
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
        info!("CHAIN: generic loader present on {:?}: {}", handle, GENERIC_LAST);
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
