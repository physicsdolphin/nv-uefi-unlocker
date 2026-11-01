//! PE/FFS packaging primitives for the DXE driver form.
//!
//! Two jobs, both byte-exact and self-verifiable:
//!
//! 1. [`patch_subsystem`] - cargo emits `x86_64-unknown-uefi` binaries with
//!    `Subsystem = EFI_APPLICATION (0x0A)`. A boot-service driver image must
//!    carry `EFI_BOOT_SERVICE_DRIVER (0x0B)`: DxeCore derives the image type
//!    from the PE subsystem, and an application-typed image dispatched from
//!    an FV DRIVER file is reclaimed after its entry point returns - which
//!    would free the code our ReadyToBoot callback points at. A build.rs
//!    link-arg cannot be used because it is per-crate and would retype the
//!    USB application too (boot managers refuse non-application boot
//!    options), so the patch happens here, deterministically, and
//!    [`verify_nvunlock_dxe`] re-checks it.
//!
//! 2. [`build_ffs`] - wraps the PE32+ image in an FFS file: PE32 section +
//!    TRUE depex section + UI section, header checksum per the PI spec, and
//!    (audit F5) `FFS_ATTRIB_CHECKSUM` set with the file-data checksum
//!    computed, so a corrupted payload is detectable before it reaches a
//!    firmware volume. State defaults to erase-polarity-1 (the flash norm);
//!    the caller chooses via the `state` argument and must match the target
//!    FV's `EFI_FVB2_ERASE_POLARITY` (audit N10) - UEFITool normalizes on
//!    insert, raw bytes do not.
//!
//! Identity (audit F4): `build` refuses any input that does not carry the
//! driver's build-identity tag ([`DXE_BUILD_TAG`]) - the USB application
//! never contains it, so an application image cannot be packed and flashed
//! as a dispatcher-phase driver (a shape that would stall the DXE
//! dispatcher inside its 30 s confirm wait).

/// FFS file GUID of NvUnlockDxe (fixed project GUID, feasibility doc §3.2).
pub const NVUNLOCK_DXE_GUID: &str = "2E7F7609-A04C-4593-B88A-651A03812F2A";

/// The driver build-identity tag (must equal `loader::DXE_BUILD_ID`).
pub const DXE_BUILD_TAG: &[u8] = b"nv-uefi-unlocker DXE driver form 1";

pub const PE_SUBSYSTEM_APPLICATION: u16 = 0x000A;
pub const PE_SUBSYSTEM_BOOT_SERVICE_DRIVER: u16 = 0x000B;
const PE_MAGIC_PE32PLUS: u16 = 0x20B;
const PE_MACHINE_X64: u16 = 0x8664;

pub const FV_FILETYPE_DRIVER: u8 = 0x07;
pub const SECTION_PE32: u8 = 0x10;
pub const SECTION_DXE_DEPEX: u8 = 0x0B;
pub const SECTION_UI: u8 = 0x15;

/// TRUE depex: `PUSH TRUE; END` - dispatch in the first pass; the driver
/// defers its real work to the ReadyToBoot event itself.
pub const DEPEX_TRUE: &[u8] = &[0x06, 0x08];

/// Header-construction + header-valid + data-valid, inverted for erase
/// polarity 1 (flash). Must match the target FV's EFI_FVB2_ERASE_POLARITY
/// (audit N10); with polarity 0 the correct value is 0x07.
pub const STATE_ERASE_POLARITY_ONE: u8 = 0xF8;
/// FFS_ATTRIB_CHECKSUM: IntegrityCheck.File is the 8-bit data checksum
/// (audit F5), not the 0xAA placeholder.
pub const FFS_ATTRIB_CHECKSUM: u8 = 0x40;

const FFS_HEADER_LEN: usize = 24;

/// Parsed PE fields relevant to the patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeInfo {
    pub machine: u16,
    pub subsystem: u16,
    pub reloc_rva: u32,
    pub reloc_size: u32,
}

fn u16_at(d: &[u8], off: usize) -> Result<u16, String> {
    d.get(off..off + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| format!("PE truncated at {off} (u16)"))
}
fn u32_at(d: &[u8], off: usize) -> Result<u32, String> {
    d.get(off..off + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| format!("PE truncated at {off} (u32)"))
}

/// Offset of the Subsystem u16 inside the image.
fn subsystem_offset(pe: &[u8]) -> Result<usize, String> {
    if pe.get(0..2) != Some(b"MZ") {
        return Err("no MZ signature".into());
    }
    let e_lfanew = u32_at(pe, 0x3c)? as usize;
    if pe.get(e_lfanew..e_lfanew + 4) != Some(b"PE\0\0") {
        return Err("no PE signature".into());
    }
    let machine = u16_at(pe, e_lfanew + 4)?;
    if machine != PE_MACHINE_X64 {
        return Err(format!("machine {machine:#06x} is not X64"));
    }
    let opt = e_lfanew + 24;
    let magic = u16_at(pe, opt)?;
    if magic != PE_MAGIC_PE32PLUS {
        return Err(format!("optional header magic {magic:#06x} is not PE32+"));
    }
    Ok(opt + 68)
}

/// Read the fields the patch cares about.
pub fn pe_info(pe: &[u8]) -> Result<PeInfo, String> {
    let e_lfanew = u32_at(pe, 0x3c)? as usize;
    let opt = e_lfanew + 24;
    // PE32+ data directories start at opt+112; index 5 = BaseRelocationTable.
    let (reloc_rva, reloc_size) = {
        let b = pe
            .get(opt + 112 + 5 * 8..opt + 112 + 5 * 8 + 8)
            .ok_or("PE truncated (data directories)")?;
        (
            u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            u32::from_le_bytes([b[4], b[5], b[6], b[7]]),
        )
    };
    Ok(PeInfo {
        machine: u16_at(pe, e_lfanew + 4)?,
        subsystem: u16_at(pe, subsystem_offset(pe)?)?,
        reloc_rva,
        reloc_size,
    })
}

/// Does the image carry the DXE build-identity tag? (audit F4: the gate
/// that keeps the USB application out of the firmware path.)
pub fn has_build_identity(pe: &[u8]) -> bool {
    pe.windows(DXE_BUILD_TAG.len()).any(|w| w == DXE_BUILD_TAG)
}

/// Retype the image 0x0A -> 0x0B in place. Refuses anything that is not an
/// unpatched X64 PE32+ APPLICATION carrying the DXE build identity, and
/// refuses images without a relocation directory (FV drivers must be
/// relocatable).
pub fn patch_subsystem(pe: &mut [u8]) -> Result<u16, String> {
    if !has_build_identity(pe) {
        return Err(format!(
            "input does not carry the DXE build-identity tag '{}'- this is not the nvunlock-dxe build (audit F4: never pack the USB application into firmware)",
            String::from_utf8_lossy(DXE_BUILD_TAG)
        ));
    }
    let info = pe_info(pe)?;
    if info.subsystem != PE_SUBSYSTEM_APPLICATION {
        return Err(format!(
            "subsystem is {:#06x}, expected EFI_APPLICATION (0x0A) - already patched or not the cargo app layout",
            info.subsystem
        ));
    }
    if info.reloc_size == 0 {
        return Err("image has no relocation directory - DxeCore cannot place a fixed-address FV driver".into());
    }
    let off = subsystem_offset(pe)?;
    pe[off..off + 2].copy_from_slice(&PE_SUBSYSTEM_BOOT_SERVICE_DRIVER.to_le_bytes());
    Ok(PE_SUBSYSTEM_BOOT_SERVICE_DRIVER)
}

/// Parse a hyphenated GUID string into its EFI memory-form bytes
/// (first three fields little-endian, rest big-endian). Byte-safe on
/// non-ASCII input (audit N10: never slices through a UTF-8 boundary).
pub fn guid_bytes(s: &str) -> Result<[u8; 16], String> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5
        || parts[0].len() != 8
        || parts[1].len() != 4
        || parts[2].len() != 4
        || parts[3].len() != 4
        || parts[4].len() != 12
    {
        return Err(format!("GUID '{s}' is not the 8-4-4-4-12 form"));
    }
    let le_field = |p: &str| -> Result<Vec<u8>, String> {
        let b = p.as_bytes();
        if !b.iter().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("GUID field '{p}' has non-hex characters"));
        }
        (0..b.len() / 2)
            .map(|i| {
                u8::from_str_radix(core::str::from_utf8(&b[i * 2..i * 2 + 2]).unwrap(), 16)
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<u8>, String>>()
    };
    let mut out = [0u8; 16];
    for (i, b) in le_field(parts[0])?.into_iter().rev().enumerate() {
        out[i] = b;
    }
    for (i, b) in le_field(parts[1])?.into_iter().rev().enumerate() {
        out[4 + i] = b;
    }
    for (i, b) in le_field(parts[2])?.into_iter().rev().enumerate() {
        out[6 + i] = b;
    }
    for (i, b) in le_field(parts[3])?.into_iter().enumerate() {
        out[8 + i] = b;
    }
    for (i, b) in le_field(parts[4])?.into_iter().enumerate() {
        out[10 + i] = b;
    }
    Ok(out)
}

fn section(body_type: u8, body: &[u8]) -> Result<Vec<u8>, String> {
    let size = 4 + body.len();
    if size >= 0xFF_FFFF {
        return Err(format!("section type {body_type:#04x} exceeds the 24-bit size field"));
    }
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(&(size as u32).to_le_bytes()[..3]);
    out.push(body_type);
    out.extend_from_slice(body);
    Ok(out)
}

/// Build the FFS file. `state` must match the target FV's erase polarity
/// (default [`STATE_ERASE_POLARITY_ONE`]; audit N10).
pub fn build_ffs(
    pe: &[u8],
    guid: &[u8; 16],
    ui_name: &str,
    state: u8,
) -> Result<Vec<u8>, String> {
    let mut sections = section(SECTION_PE32, pe)?;
    let push_aligned = |buf: &mut Vec<u8>, sec: Vec<u8>| -> Result<(), String> {
        while buf.len() % 4 != 0 {
            buf.push(0);
        }
        buf.extend_from_slice(&sec);
        Ok(())
    };
    push_aligned(&mut sections, section(SECTION_DXE_DEPEX, DEPEX_TRUE)?)?;
    let ui = alloc_ui(ui_name)?;
    push_aligned(&mut sections, section(SECTION_UI, &ui)?)?;

    let total = FFS_HEADER_LEN + sections.len();
    if total >= 0xFF_FFFF {
        return Err(format!("FFS file too large for the 24-bit size field: {total}"));
    }

    let mut ffs = Vec::with_capacity(total);
    ffs.extend_from_slice(guid);
    ffs.push(0); // IntegrityCheck.File, patched below (data checksum)
    ffs.push(0); // IntegrityCheck.Header, patched below
    ffs.push(FV_FILETYPE_DRIVER);
    ffs.push(FFS_ATTRIB_CHECKSUM); // Attributes: file-data checksum enabled
    ffs.extend_from_slice(&(total as u32).to_le_bytes()[..3]);
    ffs.push(state);
    assert_eq!(ffs.len(), FFS_HEADER_LEN);
    ffs.extend_from_slice(&sections);

    // File-data checksum (audit F5): the sum of all bytes after the header,
    // plus IntegrityCheck.File, is zero.
    let data_sum: u8 = ffs[FFS_HEADER_LEN..].iter().fold(0u8, |a, b| a.wrapping_add(*b));
    ffs[16] = data_sum.wrapping_neg();

    // Header checksum: the sum of all header bytes EXCLUDING State (23) and
    // IntegrityCheck.File (16) must be zero.
    let sum: u8 = ffs[..FFS_HEADER_LEN]
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 16 && *i != 23)
        .map(|(_, b)| u16::from(*b))
        .sum::<u16>() as u8;
    ffs[17] = (!sum).wrapping_add(1);
    Ok(ffs)
}

fn alloc_ui(ui_name: &str) -> Result<Vec<u8>, String> {
    let mut ui: Vec<u8> = ui_name.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
    ui.extend_from_slice(&[0, 0]); // UCS-2 NUL terminator
    Ok(ui)
}

/// A re-parsed FFS file.
#[derive(Debug)]
pub struct FfsSummary {
    pub file_type: u8,
    pub state: u8,
    pub attributes: u8,
    pub sections: Vec<(u8, usize)>, // (type, body length)
    pub pe_subsystem: Option<u16>,
    pub data_checksum_ok: bool,
}

/// Parse and structurally validate an FFS file: header checksum, bounded
/// section walk (4-byte alignment respected, never indexes past the end -
/// audit N9), and the file-data checksum when FFS_ATTRIB_CHECKSUM is set.
pub fn verify_ffs(ffs: &[u8]) -> Result<FfsSummary, String> {
    if ffs.len() < FFS_HEADER_LEN {
        return Err("shorter than an FFS header".into());
    }
    let size = ffs[20] as usize | (ffs[21] as usize) << 8 | (ffs[22] as usize) << 16;
    if size != ffs.len() {
        return Err(format!("header size {size} != file length {}", ffs.len()));
    }
    let sum: u8 = ffs[..FFS_HEADER_LEN]
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 16 && *i != 23)
        .map(|(_, b)| u16::from(*b))
        .sum::<u16>() as u8;
    if sum != 0 {
        return Err(format!("header checksum mismatch (sum {sum:#04x})"));
    }
    let attributes = ffs[19];
    let data_checksum_ok = if attributes & FFS_ATTRIB_CHECKSUM != 0 {
        let data_sum: u8 =
            ffs[FFS_HEADER_LEN..].iter().fold(0u8, |a, b| a.wrapping_add(*b));
        data_sum.wrapping_add(ffs[16]) == 0
    } else {
        ffs[16] == FILE_CHECKSUM_PLACEHOLDER
    };

    let mut off = FFS_HEADER_LEN;
    let mut sections = Vec::new();
    let mut pe_subsystem = None;
    while off < ffs.len() {
        // Bounded alignment walk (audit N9): never index past the end.
        while off < ffs.len() && off % 4 != 0 {
            if ffs[off] != 0 {
                return Err(format!("non-zero alignment pad at {off}"));
            }
            off += 1;
        }
        if off >= ffs.len() {
            return Err(format!("section data ends mid-alignment at {off}"));
        }
        if off + 4 > ffs.len() {
            return Err(format!("trailing garbage at {off}"));
        }
        let sec_size = ffs[off] as usize | (ffs[off + 1] as usize) << 8 | (ffs[off + 2] as usize) << 16;
        let sec_type = ffs[off + 3];
        if sec_size < 4 || off + sec_size > ffs.len() {
            return Err(format!("section at {off}: bad size {sec_size}"));
        }
        if sec_type == SECTION_PE32 {
            let body = &ffs[off + 4..off + sec_size];
            pe_subsystem = Some(u16_at(body, subsystem_offset(body)?)?);
        }
        sections.push((sec_type, sec_size - 4));
        off += sec_size;
    }

    Ok(FfsSummary {
        file_type: ffs[18],
        state: ffs[23],
        attributes,
        sections,
        pe_subsystem,
        data_checksum_ok,
    })
}

const FILE_CHECKSUM_PLACEHOLDER: u8 = 0xAA;

/// Strict validation for OUR artifact (audit F4/F5/N10): everything
/// [`verify_ffs`] checks, plus file type, erase polarity, depex bytes,
/// exactly one PE32/DEPEX/UI section each, the wrapped image's subsystem
/// and the presence of the build-identity tag inside the PE body.
pub fn verify_nvunlock_dxe(ffs: &[u8]) -> Result<FfsSummary, String> {
    let summary = verify_ffs(ffs)?;
    if summary.file_type != FV_FILETYPE_DRIVER {
        return Err(format!("file type {:#04x} is not FV_FILETYPE_DRIVER", summary.file_type));
    }
    if summary.state != STATE_ERASE_POLARITY_ONE {
        return Err(format!(
            "state {:#04x} does not match the built erase polarity - the artifact was modified or built for another FV",
            summary.state
        ));
    }
    if !summary.data_checksum_ok {
        return Err("file-data checksum mismatch - the payload is corrupted (audit F5)".into());
    }
    let count = |t: u8| summary.sections.iter().filter(|(ty, _)| *ty == t).count();
    if count(SECTION_PE32) != 1 {
        return Err(format!("expected exactly one PE32 section, found {}", count(SECTION_PE32)));
    }
    if count(SECTION_DXE_DEPEX) != 1 {
        return Err("expected exactly one DXE_DEPEX section".into());
    }
    if count(SECTION_UI) != 1 {
        return Err("expected exactly one UI section".into());
    }
    // Walk the sections again to inspect contents.
    let mut off = FFS_HEADER_LEN;
    while off < ffs.len() {
        while off < ffs.len() && off % 4 != 0 {
            off += 1;
        }
        if off + 4 > ffs.len() {
            break;
        }
        let sec_size = ffs[off] as usize | (ffs[off + 1] as usize) << 8 | (ffs[off + 2] as usize) << 16;
        let sec_type = ffs[off + 3];
        let body = ffs.get(off + 4..off + sec_size).ok_or("section out of range")?;
        match sec_type {
            SECTION_DXE_DEPEX if body != DEPEX_TRUE => {
                return Err(format!("depex bytes {:02x?} are not the built TRUE depex", body));
            }
            SECTION_PE32 => {
                if !has_build_identity(body) {
                    return Err("wrapped PE does not carry the DXE build-identity tag".into());
                }
                if summary.pe_subsystem != Some(PE_SUBSYSTEM_BOOT_SERVICE_DRIVER) {
                    return Err(format!(
                        "wrapped PE subsystem is {:?}, expected 0x0B",
                        summary.pe_subsystem
                    ));
                }
            }
            _ => {}
        }
        off += sec_size;
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    /// Minimal PE32+: DOS header + PE signature + COFF + full optional
    /// header including the data directories, carrying the build tag.
    fn synth_pe(subsystem: u16, reloc_size: u32) -> Vec<u8> {
        let mut pe = vec![0u8; 0x200];
        pe[0] = b'M';
        pe[1] = b'Z';
        pe[0x3c..0x40].copy_from_slice(&0x40u32.to_le_bytes());
        pe[0x40..0x44].copy_from_slice(b"PE\0\0");
        pe[0x44..0x46].copy_from_slice(&PE_MACHINE_X64.to_le_bytes()); // machine
        pe[0x46..0x48].copy_from_slice(&1u16.to_le_bytes()); // sections
        pe[0x54..0x56].copy_from_slice(&240u16.to_le_bytes()); // opt header size
        let opt = 0x40 + 24;
        pe[opt..opt + 2].copy_from_slice(&PE_MAGIC_PE32PLUS.to_le_bytes());
        pe[opt + 68..opt + 70].copy_from_slice(&subsystem.to_le_bytes());
        let dd = opt + 112;
        pe[dd + 5 * 8..dd + 5 * 8 + 4].copy_from_slice(&0x1000u32.to_le_bytes());
        pe[dd + 5 * 8 + 4..dd + 5 * 8 + 8].copy_from_slice(&reloc_size.to_le_bytes());
        // The build-identity tag (audit F4) in a body region.
        let at = 0x180;
        pe[at..at + DXE_BUILD_TAG.len()].copy_from_slice(DXE_BUILD_TAG);
        pe
    }

    #[test]
    fn patch_retypes_application_to_boot_service_driver() {
        let mut pe = synth_pe(PE_SUBSYSTEM_APPLICATION, 0x2f0);
        assert_eq!(patch_subsystem(&mut pe).unwrap(), PE_SUBSYSTEM_BOOT_SERVICE_DRIVER);
        assert_eq!(pe_info(&pe).unwrap().subsystem, PE_SUBSYSTEM_BOOT_SERVICE_DRIVER);
    }

    #[test]
    fn patch_refuses_app_without_identity() {
        // The application build has no tag: ffspack must refuse it (audit F4).
        let mut pe = synth_pe(PE_SUBSYSTEM_APPLICATION, 0x2f0);
        let at = 0x180;
        pe[at..at + DXE_BUILD_TAG.len()].fill(0x20);
        assert!(patch_subsystem(&mut pe).is_err());
    }

    #[test]
    fn patch_refuses_wrong_subsystem_or_missing_relocs() {
        let mut pe = synth_pe(PE_SUBSYSTEM_BOOT_SERVICE_DRIVER, 0x10);
        assert!(patch_subsystem(&mut pe).is_err(), "already-driver image must be refused");
        let mut pe = synth_pe(PE_SUBSYSTEM_APPLICATION, 0);
        assert!(patch_subsystem(&mut pe).is_err(), "reloc-less image must be refused");
    }

    #[test]
    fn guid_bytes_use_efi_mixed_endianness() {
        let b = guid_bytes(NVUNLOCK_DXE_GUID).unwrap();
        assert_eq!(b[..4], [0x09, 0x76, 0x7F, 0x2E]); // 2E7F7609 LE
        assert_eq!(b[4..6], [0x4C, 0xA0]); // A04C LE
        assert_eq!(b[6..8], [0x93, 0x45]); // 4593 LE
        assert_eq!(b[8..10], [0xB8, 0x8A]); // B88A BE
        assert_eq!(b[10..], [0x65, 0x1A, 0x03, 0x81, 0x2F, 0x2A]);
        assert!(guid_bytes("nope").is_err());
        assert!(guid_bytes("€€ab-1234-1234-1234-123456789abc").is_err(), "multi-byte input must error, not panic (audit N10)");
    }

    #[test]
    fn ffs_roundtrip_checksums_and_strict_verify() {
        let mut pe = synth_pe(PE_SUBSYSTEM_APPLICATION, 0x2f0);
        patch_subsystem(&mut pe).unwrap();
        // Make the PE length non-4-aligned so section padding is exercised.
        pe.push(0x90);
        let ffs = build_ffs(&pe, &guid_bytes(NVUNLOCK_DXE_GUID).unwrap(), "NvUnlockDxe", STATE_ERASE_POLARITY_ONE).unwrap();

        let sum: u8 = ffs[..24]
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 16 && *i != 23)
            .map(|(_, b)| u16::from(*b))
            .sum::<u16>() as u8;
        assert_eq!(sum, 0, "header checksum rule: sum with exclusions is zero");
        assert_eq!(ffs[19], FFS_ATTRIB_CHECKSUM, "data checksum attribute set (audit F5)");
        let data_sum: u8 = ffs[24..].iter().fold(0u8, |a, b| a.wrapping_add(*b));
        assert_eq!(data_sum.wrapping_add(ffs[16]), 0, "file-data checksum rule");

        let parsed = verify_nvunlock_dxe(&ffs).unwrap();
        assert_eq!(parsed.file_type, FV_FILETYPE_DRIVER);
        assert_eq!(parsed.pe_subsystem, Some(PE_SUBSYSTEM_BOOT_SERVICE_DRIVER));
        assert!(parsed.data_checksum_ok);
        let types: Vec<u8> = parsed.sections.iter().map(|(t, _)| *t).collect();
        assert_eq!(types, vec![SECTION_PE32, SECTION_DXE_DEPEX, SECTION_UI]);
    }

    #[test]
    fn verify_rejects_payload_corruption() {
        let mut pe = synth_pe(PE_SUBSYSTEM_APPLICATION, 0x2f0);
        patch_subsystem(&mut pe).unwrap();
        let mut ffs = build_ffs(&pe, &guid_bytes(NVUNLOCK_DXE_GUID).unwrap(), "NvUnlockDxe", STATE_ERASE_POLARITY_ONE).unwrap();
        // Flip one byte inside the PE body: the data checksum must catch it
        // (audit F5 - the pre-fix test blessed this).
        let mid = 24 + ffs.len() / 2;
        ffs[mid] ^= 0xff;
        assert!(verify_nvunlock_dxe(&ffs).is_err());
        assert!(verify_ffs(&ffs).unwrap().data_checksum_ok == false);
    }

    #[test]
    fn verify_handles_30_byte_file_without_panic() {
        // Audit N9: a 30-byte, checksum-valid FFS must produce an error,
        // never an index panic.
        let mut tiny = vec![0u8; 30];
        tiny[18] = FV_FILETYPE_DRIVER;
        tiny[20..23].copy_from_slice(&30u32.to_le_bytes()[..3]);
        let sum: u8 = tiny[..24]
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 16 && *i != 23)
            .map(|(_, b)| u16::from(*b))
            .sum::<u16>() as u8;
        tiny[17] = (!sum).wrapping_add(1);
        let r = verify_ffs(&tiny);
        assert!(r.is_err() || r.unwrap().sections.len() <= 1, "must not panic");
    }

    #[test]
    fn digest_is_stable_for_fixed_inputs() {
        let mut pe = synth_pe(PE_SUBSYSTEM_APPLICATION, 0x2f0);
        patch_subsystem(&mut pe).unwrap();
        let a = build_ffs(&pe, &guid_bytes(NVUNLOCK_DXE_GUID).unwrap(), "NvUnlockDxe", STATE_ERASE_POLARITY_ONE).unwrap();
        let b = build_ffs(&pe, &guid_bytes(NVUNLOCK_DXE_GUID).unwrap(), "NvUnlockDxe", STATE_ERASE_POLARITY_ONE).unwrap();
        assert_eq!(a, b);
        let mut h = Sha256::new();
        h.update(&a);
        assert_eq!(h.finalize().len(), 32);
    }
}
