//! Driver#### NVRAM registration (the "bcfg driver load" delivery form).
//!
//! Maintainer direction on PR #100: FFS volume embedding is firmware-
//! invasive; the non-invasive alternative is a UEFI Driver#### load
//! option pointing at a file on removable media - "if something goes
//! wrong, pull the stick and reboot" (the load fails non-fatally and BDS
//! continues without the driver).
//!
//! What this module does: gated by `\tx-register-driver.txt` on the boot
//! volume, register a named EFI driver file (default `\NvUnlockDxe.efi`)
//! as a Driver#### entry, so the firmware loads it every boot BEFORE
//! console connect. `\tx-unregister-driver.txt` removes our entries again
//! (one-marker uninstall, no UEFI Shell required).
//!
//! Constitution §1.4 boundary: this writes persistent NVRAM state (not a
//! Boot#### entry, but still resident configuration). It only ever runs
//! behind an explicit operator marker, the registered file lives on
//! removable media (pull the stick = inert), and uninstall is one marker
//! away. Secure Boot note: Driver#### images go through LoadImage and
//! therefore SB verification - SB-ON machines need the image signed into
//! db (unlike FV embedding, which bypasses SB).
//!
//! Manual equivalent (UEFI Shell): `bcfg driver add 0 fs0:\NvUnlockDxe.efi
//! "NvUnlock"` / `bcfg driver rm <index>` / `bcfg driver dump`.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use log::info;
use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams};
use uefi::data_types::CStr16;
use uefi::proto::loaded_image::LoadedImage;
use uefi::runtime::{self, VariableAttributes, VariableVendor};
use uefi::{guid, Handle};

/// `Driver####` / `DriverOrder` live in the global variable namespace.
const GLOBAL_VARIABLE: VariableVendor = VariableVendor(guid!("8be4df61-93ca-11d2-aa0d-00e098032b8c"));
/// The attributes every Driver#### / DriverOrder variable carries.
const NV_BS_RT: VariableAttributes = VariableAttributes::NON_VOLATILE
    .union(VariableAttributes::BOOTSERVICE_ACCESS)
    .union(VariableAttributes::RUNTIME_ACCESS);
/// Bound on the option scan: bcfg allocates options densely from 0000;
/// 512 misses in a row without a single hit means no registrations exist.
const SCAN_LIMIT: u16 = 512;

/// Handle `\tx-register-driver.txt` / `\tx-unregister-driver.txt`. Called
/// from main() early (no GPU access involved).
pub fn service_markers() {
    if crate::chainload::probe_file("\\tx-unregister-driver.txt") {
        unregister_ours();
        return;
    }
    let Some(marker) = crate::chainload::read_boot_file("\\tx-register-driver.txt") else {
        return;
    };
    let first_line = String::from_utf8_lossy(&marker)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    let file = if first_line.is_empty() {
        r"\NvUnlockDxe.efi".to_string()
    } else if first_line.starts_with('\\') {
        first_line
    } else {
        format!("\\{first_line}")
    };
    register(&file);
}

fn option_name(idx: u16) -> String {
    alloc::format!("Driver{:04}", idx)
}

/// `DriverOrder` name as an owned CString16 (runtime paths, not literals).
fn order_name() -> uefi::CString16 {
    uefi::CString16::try_from("DriverOrder").unwrap()
}

/// All existing Driver#### entries (dense scan up to the first run of
/// misses).
fn driver_entries() -> Vec<(u16, Box<[u8]>)> {
    let mut out = Vec::new();
    let mut misses = 0u16;
    for idx in 0u16..SCAN_LIMIT {
        let Ok(name) = uefi::CString16::try_from(option_name(idx).as_str()) else { break };
        match runtime::get_variable_boxed(&name, &GLOBAL_VARIABLE) {
            Ok((data, _attrs)) => {
                misses = 0;
                out.push((idx, data));
            }
            Err(_) => {
                misses += 1;
                if misses >= 16 && !out.is_empty() {
                    break;
                }
                if misses >= SCAN_LIMIT && out.is_empty() {
                    break;
                }
            }
        }
    }
    out
}

/// Register `file` (a path on the SAME volume the loader booted from) as a
/// Driver#### load option. Idempotent for an identical file path.
fn register(file: &str) {
    info!("NVRAM-DRIVER: register {file} requested (\\tx-register-driver.txt)");
    let Some(volume_prefix) = boot_volume_prefix() else {
        info!("NVRAM-DRIVER: boot volume device path unavailable - not registering");
        return;
    };
    let Some(file_dp) = file_device_path(&volume_prefix, file) else {
        info!("NVRAM-DRIVER: cannot build the file device path - not registering");
        return;
    };

    // Idempotence: an entry whose FilePathList matches is a no-op.
    for (idx, data) in driver_entries() {
        if load_option_file_path(&data).as_deref() == Some(file_dp.as_slice()) {
            info!("NVRAM-DRIVER: Driver{:04} already points at {file} - nothing to do", idx);
            return;
        }
    }

    // First free index (bcfg semantics: dense numbering).
    let used: Vec<u16> = driver_entries().iter().map(|(i, _)| *i).collect();
    let Some(idx) = (0u16..SCAN_LIMIT).find(|i| !used.contains(i)) else {
        info!("NVRAM-DRIVER: no free Driver#### slot");
        return;
    };

    // EFI_LOAD_OPTION: UINT32 Attributes, UINT16 FilePathListLength,
    // CHAR16 Description (empty), FilePathList, no optional data.
    let mut opt = Vec::new();
    opt.extend_from_slice(&0u32.to_le_bytes());
    opt.extend_from_slice(&(file_dp.len() as u16).to_le_bytes());
    opt.extend_from_slice(&0u16.to_le_bytes()); // empty description terminator
    opt.extend_from_slice(&file_dp);

    // Preserve and extend DriverOrder.
    let order: Vec<u16> = match runtime::get_variable_boxed(&order_name(), &GLOBAL_VARIABLE) {
        Ok((data, _)) => data
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect(),
        Err(_) => Vec::new(),
    };
    let mut new_order = order.clone();
    new_order.push(idx);

    let Ok(name) = uefi::CString16::try_from(option_name(idx).as_str()) else { return };
    match runtime::set_variable(&name, &GLOBAL_VARIABLE, NV_BS_RT, &opt) {
        Ok(()) => {
            let order_bytes: Vec<u8> = new_order.iter().flat_map(|v| v.to_le_bytes()).collect();
            match runtime::set_variable(&order_name(), &GLOBAL_VARIABLE, NV_BS_RT, &order_bytes) {
                Ok(()) => info!(
                    "NVRAM-DRIVER: registered as Driver{:04} (order {:?}) - pull the stick to disable, \\tx-unregister-driver.txt to remove",
                    idx, new_order
                ),
                Err(e) => {
                    // Roll the orphan option back: never leave an
                    // unorderable entry behind.
                    let _ = runtime::delete_variable(&name, &GLOBAL_VARIABLE);
                    info!("NVRAM-DRIVER: DriverOrder write failed {:?} - rolled back", e.status());
                }
            }
        }
        Err(e) => info!("NVRAM-DRIVER: Driver{:04} write failed {:?}", idx, e.status()),
    }
}

/// Remove every Driver#### entry whose FilePathList lives on the volume we
/// booted from (our registrations only ever point there), then rewrite
/// DriverOrder without them.
fn unregister_ours() {
    info!("NVRAM-DRIVER: unregister requested (\\tx-unregister-driver.txt)");
    let Some(volume_prefix) = boot_volume_prefix() else {
        info!("NVRAM-DRIVER: boot volume device path unavailable - nothing removed");
        return;
    };
    let mut removed = Vec::new();
    for (idx, data) in driver_entries() {
        if let Some(fp) = load_option_file_path(&data) {
            if fp.starts_with(&volume_prefix) {
                                let Ok(name) = uefi::CString16::try_from(option_name(idx).as_str()) else { continue };
                    match runtime::delete_variable(&name, &GLOBAL_VARIABLE) {
                    Ok(()) => {
                        removed.push(idx);
                        info!("NVRAM-DRIVER: removed Driver{:04}", idx);
                    }
                    Err(e) => info!("NVRAM-DRIVER: Driver{:04} delete failed {:?}", idx, e.status()),
                }
            }
        }
    }
    if removed.is_empty() {
        info!("NVRAM-DRIVER: no entries of ours found - nothing changed");
        return;
    }
    if let Ok((data, _)) = runtime::get_variable_boxed(&order_name(), &GLOBAL_VARIABLE) {
        let order: Vec<u16> = data
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .filter(|v| !removed.contains(v))
            .collect();
        let order_bytes: Vec<u8> = order.iter().flat_map(|v| v.to_le_bytes()).collect();
        let _ = runtime::set_variable(&order_name(), &GLOBAL_VARIABLE, NV_BS_RT, &order_bytes);
        info!("NVRAM-DRIVER: DriverOrder rewritten to {:?}", order);
    }
}

/// Device-path prefix of the volume the loader booted from: the loaded
/// image's FilePath with its final FilePath node (the BOOTX64.EFI name)
/// cut off. The loader only ever registers files on its own volume, so
/// this prefix identifies "our" entries for the unregister pass.
fn boot_volume_prefix() -> Option<Vec<u8>> {
    // Safety: GetProtocol open on our own image handle, read-only use.
    let li = unsafe {
        boot::open_protocol::<LoadedImage>(
            OpenProtocolParams { handle: boot::image_handle(), agent: boot::image_handle(), controller: None },
            OpenProtocolAttributes::GetProtocol,
        )
    }
    .ok()?;
    let dp = li.file_path()?;
    let bytes = dp.as_bytes();
    // Find the LAST FilePath node (type 0x04, subtype 0x04) before END.
    let mut last_file_node: Option<(usize, usize)> = None;
    let mut i = 0usize;
    while i + 4 <= bytes.len() {
        let len = u16::from_le_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        if len < 4 || i + len > bytes.len() {
            break;
        }
        if bytes[i] == 0x04 && bytes[i + 1] == 0x04 {
            last_file_node = Some((i, len));
        }
        if bytes[i] == 0x7f {
            break;
        }
        i += len;
    }
    let (start, _) = last_file_node?;
    if start == 0 {
        return None;
    }
    Some(bytes[..start].to_vec())
}

/// Volume prefix + FilePath node for `file` + END.
fn file_device_path(volume_prefix: &[u8], file: &str) -> Option<Vec<u8>> {
    let mut payload: Vec<u8> = file.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
    payload.extend_from_slice(&0u16.to_le_bytes());
    if payload.len() > 0xffff - 4 {
        return None;
    }
    let node_len = 4 + payload.len();
    let mut out = Vec::with_capacity(volume_prefix.len() + node_len + 4);
    out.extend_from_slice(volume_prefix);
    out.push(0x04); // MEDIA type
    out.push(0x04); // MEDIA_FILEPATH
    out.extend_from_slice(&(node_len as u16).to_le_bytes());
    out.extend_from_slice(&payload);
    out.extend_from_slice(&[0x7f, 0xff, 0x04, 0x00]); // END_ENTIRE
    Some(out)
}

/// Extract the FilePathList of an EFI_LOAD_OPTION blob (skips Attributes +
/// FilePathListLength + NUL-terminated Description).
fn load_option_file_path(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 7 {
        return None;
    }
    let fp_len = u16::from_le_bytes([data[4], data[5]]) as usize;
    let mut i = 6usize;
    while i + 2 <= data.len() {
        let ch = u16::from_le_bytes([data[i], data[i + 1]]);
        i += 2;
        if ch == 0 {
            break;
        }
    }
    if i + fp_len > data.len() {
        return None;
    }
    Some(data[i..i + fp_len].to_vec())
}
