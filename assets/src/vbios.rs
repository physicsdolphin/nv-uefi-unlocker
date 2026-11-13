//! VBIOS BIT -> falcon ucode table -> FWSEC descriptor parser.
//!
//! Ported 1:1 from open-gpu-kernel-modules
//! src/nvidia/src/kernel/gpu/gsp/kernel_gsp_fwsec.c (first-hand authority;
//! BIT header/token walk, expansion-ROM relocation, descriptor V2/V3).
//! Regression anchor: NVOC's TU106 `1.rom` measurements (docs
//! reverse-engineering/vbios/tu106-fwsec-ucode-decode.md).

#[derive(Debug)]
pub enum VbiosError {
    NotAVbios,
    BitHeaderNotFound,
    BadChecksum,
    UnsupportedBitHeaderSize(u8),
    NoFalconTable,
    NoFwsecEntry { debug: bool },
    UnsupportedDescVersion(u8),
    FwsecDescVersion { version: u8 },
    Truncated,
}

impl std::fmt::Display for VbiosError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAVbios => write!(f, "not a VBIOS image (no 0xAA55)"),
            Self::BitHeaderNotFound => write!(f, "BIT header not found"),
            Self::UnsupportedBitHeaderSize(size) => {
                write!(f, "unsupported BIT header size {size}; expected 12")
            }
            Self::BadChecksum => write!(f, "BIT header checksum mismatch"),
            Self::NoFalconTable => write!(f, "falcon ucode table (token 0x70 v2) not found"),
            Self::NoFwsecEntry { debug } => write!(f, "no FWSEC_{} entry", if *debug { "DBG" } else { "PROD" }),
            Self::UnsupportedDescVersion(v) => write!(f, "unsupported descriptor version {v}"),
            Self::FwsecDescVersion { version } => write!(
                f,
                "FWSEC with descriptor V{version} - kernel_gsp_fwsec.c accepts V2/V3 only \
                 (\"FWSEC should be either V2 or V3\"); extraction refused at the source"
            ),
            Self::Truncated => write!(f, "image truncated"),
        }
    }
}

pub const APP_ID_PRE_OS: u8 = 0x01;
pub const APP_ID_FIRMWARE_SEC_LIC: u8 = 0x05;
pub const APP_ID_FWSEC_DBG: u8 = 0x45;
pub const APP_ID_FWSEC_PROD: u8 = 0x85;

fn u8v(d: &[u8], o: usize) -> Option<u8> {
    d.get(o).copied()
}
fn u16v(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?))
}
fn u32v(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

/// Expansion-ROM relocate, kernel semantics (kernel_gsp_vbios_tu102.c
/// s_locateExpansionRoms): walk PCI data structures with the NVIDIA "NPDE"
/// extension's SUBIMAGE_LEN as the chaining stride, find the code-type-0xE0
/// (VBIOS_EXT) image; expansionRomOffset = ext_image_offset - base_image_size.
/// Verified: TU106 1.rom -> 0x11600 (NVOC-measured adjust); ASUS AD107M
/// 9507.1640 -> 0x14e00.
pub fn find_expansion_offset(image: &[u8]) -> Result<usize, VbiosError> {
    if u16v(image, 0).ok_or(VbiosError::NotAVbios)? != 0xAA55 {
        return Err(VbiosError::NotAVbios);
    }
    let mut curr = 0usize;
    let mut base_size: Option<usize> = None;
    for _ in 0..16 {
        let pcir = u16v(image, curr + 0x18).ok_or(VbiosError::Truncated)? as usize;
        let p = curr.checked_add(pcir).ok_or(VbiosError::Truncated)?;
        let pci = image.get(p..).and_then(|d| d.get(..0x16))
            .ok_or(VbiosError::Truncated)?;
        match &pci[..4] {
            b"PCIR" | b"NPDS" | b"RGIS" => {}
            _ => break,
        }
        let slen = u16v(pci, 0x0a).unwrap() as usize;
        let imglen = u16v(pci, 0x10).unwrap() as usize;
        let code = pci[0x14];
        let mut last = pci[0x15] & 0x80 != 0;
        let mut sub = imglen;
        let npde_at = p.checked_add(slen)
            .and_then(|n| n.checked_add(0xf))
            .ok_or(VbiosError::Truncated)? & !0xf;
        let mut metadata_end = p.checked_add(slen.max(0x16))
            .ok_or(VbiosError::Truncated)?;
        if image.get(npde_at..).and_then(|d| d.get(..4)) == Some(b"NPDE".as_slice()) {
            let ext = &image[npde_at..];
            let rev = u16v(ext, 4).ok_or(VbiosError::Truncated)?;
            let len = u16v(ext, 6).ok_or(VbiosError::Truncated)? as usize;
            // Common prefix: subimageLen is in 512-byte blocks, not bytes.
            // Unknown revisions require the termination byte as well.
            if len < 10 || (rev != 0x100 && rev != 0x101 && len < 11) {
                return Err(VbiosError::Truncated);
            }
            ext.get(..len).ok_or(VbiosError::Truncated)?;
            sub = u16v(ext, 8).ok_or(VbiosError::Truncated)? as usize;
            if len >= 11 {
                last = ext[10] & 0x80 != 0;
            } else if sub < imglen {
                last = false;
            }
            metadata_end = npde_at.checked_add(len).ok_or(VbiosError::Truncated)?;
        }
        let bytes = sub.checked_mul(512).ok_or(VbiosError::Truncated)?;
        let next = curr.checked_add(bytes).ok_or(VbiosError::Truncated)?;
        if bytes == 0 || next > image.len() || metadata_end > next {
            return Err(VbiosError::Truncated);
        }
        if code == 0xE0 {
            let base = base_size.ok_or(VbiosError::BitHeaderNotFound)?;
            return curr.checked_sub(base).ok_or(VbiosError::Truncated);
        }
        if code == 0x00 && base_size.is_none() {
            base_size = Some(bytes);
        }
        if last {
            break;
        }
        curr = next;
    }
    Err(VbiosError::BitHeaderNotFound)
}

/// BIT header location: scan for 0xB8FF + "BIT\0" with a zero-sum header
/// checksum (kernel_gsp_fwsec.c s_vbiosFindBitHeader).
pub fn find_bit_header(image: &[u8]) -> Result<usize, VbiosError> {
    let mut addr = 0usize;
    while addr + 12 <= image.len() {
        if u16v(image, addr) == Some(0xB8FF) && &image[addr + 2..addr + 6] == b"BIT\0" {
            let header_size = u8v(image, addr + 8).unwrap_or(0) as usize;
            if header_size >= 12 && addr + header_size <= image.len() {
                let sum: u32 = image[addr..addr + header_size].iter().map(|&b| b as u32).sum();
                if sum & 0xFF == 0 {
                    return Ok(addr);
                }
            }
        }
        addr += 2;
    }
    Err(VbiosError::BitHeaderNotFound)
}

pub struct UcodeEntry {
    pub app_id: u8,
    pub target_id: u8,
    pub desc_off: u32, // relative to expansion image
}

/// Walk BIT -> token 0x70 (falcon data v2) -> ucode table.
/// Returns (entries, expansion_offset, table_abs_offset).
pub fn find_ucode_table(image: &[u8]) -> Result<(Vec<UcodeEntry>, usize, usize), VbiosError> {
    let bit = find_bit_header(image)?;
    let header_size = u8v(image, bit + 8).unwrap() as usize;
    eprintln!("BIT: header at {bit:#x}, HeaderSize={header_size} (expected 12)");
    if header_size != 12 {
        return Err(VbiosError::UnsupportedBitHeaderSize(header_size as u8));
    }
    let token_size = u8v(image, bit + 9).unwrap() as usize;
    // BIT_HEADER_V1_00: TokenEntries is u8 at +10; +11 is HeaderChksum.
    let token_entries = u8v(image, bit + 10).unwrap_or(0) as usize;
    if !(6..=8).contains(&token_size) {
        return Err(VbiosError::NoFalconTable);
    }
    let expansion = find_expansion_offset(image)?;

    for tok in 0..token_entries {
        let t = bit + header_size + tok * token_size;
        let token_id = u8v(image, t).unwrap_or(0);
        let data_version = u8v(image, t + 1).unwrap_or(0);
        if token_id != 0x70 || data_version != 2 {
            continue;
        }
        // 6-byte token: DataSize u16 @2, DataPtr u16 @4; 8-byte: u32 @4.
        let data_ptr = if token_size >= 8 {
            u32v(image, t + 4).ok_or(VbiosError::Truncated)? as usize
        } else {
            u16v(image, t + 4).ok_or(VbiosError::Truncated)? as usize
        };
        let table_abs = expansion + u32v(image, data_ptr).ok_or(VbiosError::Truncated)? as usize;
        // ucode table header (6B): ver, hdr_size, entry_size, count, desc_ver, desc_size
        if u8v(image, table_abs).unwrap_or(0) != 1 {
            continue;
        }
        let hdr_size = u8v(image, table_abs + 1).unwrap_or(0) as usize;
        let entry_size = u8v(image, table_abs + 2).unwrap_or(0) as usize;
        let count = u8v(image, table_abs + 3).unwrap_or(0) as usize;
        if hdr_size < 6 || entry_size < 6 {
            continue;
        }
        let mut entries = Vec::new();
        for e in 0..count {
            let at = table_abs + hdr_size + e * entry_size;
            entries.push(UcodeEntry {
                app_id: u8v(image, at).ok_or(VbiosError::Truncated)?,
                target_id: u8v(image, at + 1).ok_or(VbiosError::Truncated)?,
                desc_off: u32v(image, at + 2).ok_or(VbiosError::Truncated)?,
            });
        }
        return Ok((entries, expansion, table_abs));
    }
    Err(VbiosError::NoFalconTable)
}

/// Descriptor V2 (60B, Turing) per FALCON_UCODE_DESC_V2.
/// Descriptor V1 (40 bytes, Pascal and older) - AGENTS 7.3:
/// Stored@4 / Uncompressed@8 / VirtualEntry@12 / IMEMPhys@16 / IMEMLoad@20 /
/// DMEMPhys@32 / DMEMLoad@36. Documented since the first task brief and, until
/// now, not implemented: every V1 image returned UnsupportedDescVersion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DescV1 {
    /// vDesc bit2: the image is AES ciphertext (key lives in the fuses).
    pub encrypted: bool,
    pub stored_size: u32,
    pub uncompressed_size: u32,
    pub virtual_entry: u32,
    pub imem_phys_base: u32,
    pub imem_load_size: u32,
    pub dmem_phys_base: u32,
    pub dmem_load_size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DescV2 {
    /// vDesc bit2: see DescV1::encrypted.
    pub encrypted: bool,
    pub stored_size: u32,
    pub uncompressed_size: u32,
    pub virtual_entry: u32,
    pub interface_offset: u32,
    pub imem_phys_base: u32,
    pub imem_load_size: u32,
    pub imem_virt_base: u32,
    pub imem_sec_base: u32,
    pub imem_sec_size: u32,
    pub dmem_offset: u32,
    pub dmem_phys_base: u32,
    pub dmem_load_size: u32,
}

/// Descriptor V3 (44B, GA10x+) per FALCON_UCODE_DESC_V3.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DescV3 {
    /// vDesc bit2: see DescV1::encrypted.
    pub encrypted: bool,
    pub stored_size: u32,
    pub pkc_data_offset: u32,
    pub interface_offset: u32,
    pub imem_phys_base: u32,
    pub imem_load_size: u32,
    pub imem_virt_base: u32,
    pub dmem_phys_base: u32,
    pub dmem_load_size: u32,
    pub engine_id_mask: u16,
    pub ucode_id: u8,
    pub signature_count: u8,
    pub signature_versions: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Desc {
    V1(DescV1),
    V2(DescV2),
    V3(DescV3),
}

pub fn parse_desc(image: &[u8], expansion: usize, entry: &UcodeEntry) -> Result<Desc, VbiosError> {
    let at = expansion + entry.desc_off as usize;
    let vdesc = u32v(image, at).ok_or(VbiosError::Truncated)?;
    let version = ((vdesc >> 8) & 0xFF) as u8;
    let size = (vdesc >> 16) as u16 as usize;
    // Brief 7.3 makes encryption the first classification step ("any blob gets
    // an entropy/header judgement before analysis"): a ciphertext FWSEC must not
    // be packaged as if it were usable, and the caller cannot tell unless the bit
    // is surfaced. Recorded on every variant.
    let encrypted = vdesc & (1 << 2) != 0;
    match version {
        1 if size >= 40 && at + 40 <= image.len() => {
            let f = |i: usize| u32v(image, at + 4 + i * 4).unwrap_or(0);
            Ok(Desc::V1(DescV1 {
                encrypted,
                stored_size: f(0),
                uncompressed_size: f(1),
                virtual_entry: f(2),
                imem_phys_base: f(3),
                imem_load_size: f(4),
                // words 5/6 are undocumented in 7.3's V1 layout; DMEM sits at
                // byte 32/36, i.e. words 7 and 8.
                dmem_phys_base: u32v(image, at + 32).unwrap_or(0),
                dmem_load_size: u32v(image, at + 36).unwrap_or(0),
            }))
        }
        2 if size >= 60 && at + 60 <= image.len() => {
            let f = |i: usize| u32v(image, at + 4 + i * 4).unwrap_or(0);
            Ok(Desc::V2(DescV2 {
                encrypted,
                stored_size: f(0),
                uncompressed_size: f(1),
                virtual_entry: f(2),
                interface_offset: f(3),
                imem_phys_base: f(4),
                imem_load_size: f(5),
                imem_virt_base: f(6),
                imem_sec_base: f(7),
                imem_sec_size: f(8),
                dmem_offset: f(9),
                dmem_phys_base: f(10),
                dmem_load_size: f(11),
            }))
        }
        3 if size >= 44 && at + 44 <= image.len() => Ok(Desc::V3(DescV3 {
            encrypted,
            stored_size: u32v(image, at + 4).unwrap_or(0),
            pkc_data_offset: u32v(image, at + 8).unwrap_or(0),
            interface_offset: u32v(image, at + 12).unwrap_or(0),
            imem_phys_base: u32v(image, at + 16).unwrap_or(0),
            imem_load_size: u32v(image, at + 20).unwrap_or(0),
            imem_virt_base: u32v(image, at + 24).unwrap_or(0),
            dmem_phys_base: u32v(image, at + 28).unwrap_or(0),
            dmem_load_size: u32v(image, at + 32).unwrap_or(0),
            engine_id_mask: u16v(image, at + 36).unwrap_or(0),
            ucode_id: u8v(image, at + 38).unwrap_or(0),
            signature_count: u8v(image, at + 39).unwrap_or(0),
            signature_versions: u16v(image, at + 40).unwrap_or(0),
        })),
        v => Err(VbiosError::UnsupportedDescVersion(v)),
    }
}

/// Find FWSEC descriptor (PROD or DBG). Returns (entry, desc).
pub fn find_fwsec(image: &[u8], debug: bool) -> Result<(UcodeEntry, Desc), VbiosError> {
    let (entries, expansion, _) = find_ucode_table(image)?;
    let want = if debug { APP_ID_FWSEC_DBG } else { APP_ID_FWSEC_PROD };
    for e in entries {
        // Only the requested entry is parsed. The old loop also parsed the
        // FIRMWARE_SEC_LIC (0x05) entry and then dropped the result, but its
        // error escaped through `?` - so a card whose licence container uses
        // an older descriptor revision failed FWSEC extraction with
        // "unsupported descriptor version" even with a valid 0x85 entry
        // right behind it.
        if e.app_id != want {
            continue;
        }
        let desc = parse_desc(image, expansion, &e)?;
        // OpenRM's kernel_gsp_fwsec.c parses only V2/V3 for FWSEC and defines no
        // V1 struct (V1/V4 exist only as version macros), so a V1-labeled FWSEC
        // is never a valid extraction. Before the V1 arm existed this failed as
        // UnsupportedDescVersion; now that V1 parses, refuse it explicitly here
        // so a 0x85/V1 combination stays a loud failure at extraction time
        // instead of a silent package the loader would have to reject later.
        if let Desc::V1(DescV1 { .. }) = desc {
            return Err(VbiosError::FwsecDescVersion { version: 1 });
        }
        return Ok((e, desc));
    }
    Err(VbiosError::NoFwsecEntry { debug })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TU106 1.rom regression anchor (skips when the NVOC checkout is
    /// absent): FWSEC_PROD must be V2 with the measured geometry.
    #[test]
    fn tu106_rom_when_present() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../nvoc/1.rom");
        let Ok(img) = std::fs::read(&p) else { return };
        assert_eq!(u16v(&img, 0), Some(0xAA55));

        let expansion = find_expansion_offset(&img).expect("expansion offset");
        // Kernel semantics: 0xE0 image (0x20000) minus base length (0xea00).
        assert_eq!(expansion, 0x11600, "TU106 adjust (NVOC measured 0x11600)");

        let bit = find_bit_header(&img).expect("BIT header");
        assert_eq!(bit, 0x1b0, "TU106 BIT header at 0x1b0");

        let (entries, exp, table) = find_ucode_table(&img).expect("ucode table");
        assert_eq!(exp, 0x20000);
        assert_eq!(table, 0x11600 + 0xeb24, "ucode table abs offset");
        assert_eq!(entries.len(), 16, "TU106 ucode entries");
        assert!(entries.iter().any(|e| e.app_id == APP_ID_FWSEC_PROD));
        assert!(entries.iter().any(|e| e.app_id == APP_ID_PRE_OS));

        let (_, desc) = find_fwsec(&img, false).expect("FWSEC_PROD");
        let Desc::V2(v2) = desc else { panic!("TU106 FWSEC must be V2") };
        assert_eq!(v2.imem_load_size, 0x9600, "IMEMLoadSize (NVOC measured)");
        assert_eq!(v2.imem_sec_base, 0x300, "IMEMSecBase");
        assert_eq!(v2.imem_sec_size, 0x9300, "IMEMSecSize: 0x300+0x9300=0x9600");
        assert_eq!(v2.dmem_load_size, 0x410, "DMEMLoadSize");
        assert_eq!(v2.stored_size, 0x9a10, "stored = IMEM+DMEM");

        let (_, dbg_desc) = find_fwsec(&img, true).expect("FWSEC_DBG");
        assert_eq!(dbg_desc, desc, "TU106 debug track bit-identical geometry");
    }

    /// ASUS AD107M laptop VBIOS (user-provided dump, NVGI wrapper at 0x9400;
    /// first-64KiB CRC 0xd4a7daa2 == the M1 NV_PROM sample from the card).
    #[test]
    fn ad107m_rom_when_present() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../nvoc/reverse/VBIOS/ASUS_4060_laptop_OC_95071640F7.rom");
        let Ok(raw) = std::fs::read(&p) else { return };
        let start = raw.windows(2).position(|w| w == [0x55, 0xaa]).expect("AA55");
        let img = &raw[start..];

        let expansion = find_expansion_offset(img).expect("expansion");
        assert_eq!(expansion, 0x14e00, "AD107M adjust (0x24a00 - 0xfc00)");
        let bit = find_bit_header(img).expect("BIT");
        assert_eq!(bit, 0x1b0);
        let (entries, _, table) = find_ucode_table(img).expect("ucode table");
        assert_eq!(table, 0x14e00 + 0x8c74f);
        assert_eq!(entries.len(), 16);
        assert!(entries.iter().any(|e| e.app_id == APP_ID_FWSEC_PROD));

        let (_, desc) = find_fwsec(img, false).expect("FWSEC_PROD");
        let Desc::V3(v3) = desc else { panic!("AD107M FWSEC must be V3 (BOOT_FROM_HS)") };
        assert_eq!(v3.stored_size, 0x10280);
        assert_eq!(v3.imem_phys_base, 0xf500);
        assert_eq!(v3.signature_count, 2, "two RSA3K slots; fuse version picks one");
        assert_eq!(v3.signature_versions, 0x3, "slots 0 and 1 present");
        // Debug track is a SEPARATE descriptor with the same geometry.
        let (_, dbg_desc) = find_fwsec(img, true).expect("FWSEC_DBG");
        assert_eq!(dbg_desc, desc, "AD107M debug track bit-identical geometry");
    }

    /// Synthetic minimal VBIOS: two-image expansion ROM, one BIT header with a
    /// single 0x70/v2 token, and a ucode table whose FIRST entry is the
    /// FIRMWARE_SEC_LIC (0x05) container with a V1 descriptor, followed by the
    /// real FWSEC_PROD (0x85) V2 entry. Regression for the walk that used to
    /// abort on the LIC entry (`?` on a result it then discarded).
    fn synthetic_vbios_lic_then_fwsec() -> Vec<u8> {
        let mut rom = vec![0u8; 0x800];
        rom[0] = 0x55; // option-ROM signature 0xAA55 (LE) - find_expansion_offset requires it
        rom[1] = 0xAA;
        // Image 0 (code type 0x00): sets the expansion-RAM base size.
        rom[0x18..0x1a].copy_from_slice(&0x20u16.to_le_bytes()); // PCIR pointer
        rom[0x20..0x24].copy_from_slice(b"PCIR");
        rom[0x2a..0x2c].copy_from_slice(&0x18u16.to_le_bytes()); // struct length
        rom[0x30..0x32].copy_from_slice(&1u16.to_le_bytes()); // image length x512
        rom[0x34] = 0x00; // code type: not the last image
        // Image 1 at 0x200 (code type 0xE0): expansion offset = 0x200-0x200 = 0.
        rom[0x218..0x21a].copy_from_slice(&0x20u16.to_le_bytes());
        rom[0x220..0x224].copy_from_slice(b"PCIR");
        rom[0x22a..0x22c].copy_from_slice(&0x18u16.to_le_bytes());
        rom[0x230..0x232].copy_from_slice(&1u16.to_le_bytes());
        rom[0x234] = 0xE0;
        // BIT header @0x400: 12-byte header, stride 8, one token, zero-sum.
        let bit = 0x400usize;
        rom[bit] = 0xFF; // little-endian 0xB8FF signature
        rom[bit + 1] = 0xB8;
        rom[bit + 2..bit + 6].copy_from_slice(b"BIT\0");
        rom[bit + 6] = 0x55; // checksum padding byte
        rom[bit + 8] = 12; // header size
        rom[bit + 9] = 8; // token stride
        rom[bit + 10..bit + 12].copy_from_slice(&1u16.to_le_bytes()); // one token
        // Token 0x70 / data version 2 -> pointer 0x500 -> table at 0x600.
        rom[bit + 12] = 0x70;
        rom[bit + 13] = 0x02;
        rom[bit + 14..bit + 16].copy_from_slice(&4u16.to_le_bytes());
        rom[bit + 16..bit + 20].copy_from_slice(&0x500u32.to_le_bytes());
        rom[0x500..0x504].copy_from_slice(&0x600u32.to_le_bytes());
        // ucode table header: ver 1, hdr 6, entry 8, count 2.
        rom[0x600] = 1;
        rom[0x601] = 6;
        rom[0x602] = 8;
        rom[0x603] = 2;
        rom[0x604] = 1;
        rom[0x605] = 0x2c; // descriptor version marker (unused by the parser)
        // entry 0: FIRMWARE_SEC_LIC -> V1 descriptor (version byte 1).
        rom[0x606] = APP_ID_FIRMWARE_SEC_LIC;
        rom[0x608..0x60c].copy_from_slice(&0x700u32.to_le_bytes());
        rom[0x700..0x704].copy_from_slice(&0x0028_0101u32.to_le_bytes()); // 40B, ver 1
        // entry 1: FWSEC_PROD -> V2 descriptor at 0x780, stored size 0x1234.
        rom[0x60e] = APP_ID_FWSEC_PROD;
        rom[0x610..0x614].copy_from_slice(&0x780u32.to_le_bytes());
        rom[0x780..0x784].copy_from_slice(&0x003C_0201u32.to_le_bytes()); // 60B, ver 2
        rom[0x784..0x788].copy_from_slice(&0x1234u32.to_le_bytes());
        rom
    }

    #[test]
    fn fwsec_walk_skips_the_lic_container() {
        let rom = synthetic_vbios_lic_then_fwsec();
        let (entry, desc) = find_fwsec(&rom, false).expect("FWSEC_PROD must be found behind the LIC entry");
        assert_eq!(entry.app_id, APP_ID_FWSEC_PROD);
        match desc {
            Desc::V2(v2) => assert_eq!(v2.stored_size, 0x1234),
            other => panic!("expected the V2 descriptor, got {other:?}"),
        }
    }
    /// V1 (40-byte) descriptors used to return UnsupportedDescVersion for every
    /// image, although AGENTS 7.3 documents the layout in full. Field-by-field
    /// pin against that table: Stored@4 Uncompressed@8 VirtualEntry@12
    /// IMEMPhys@16 IMEMLoad@20 DMEMPhys@32 DMEMLoad@36.
    ///
    /// Primary-source bounds on this arm (verified against OpenRM, 2026-10-29):
    /// kernel_gsp_fwsec.c parses only V2 (60B) and V3 (44B) for FWSEC - verbatim
    /// "FWSEC should be either V2 or V3" - and defines no V1 struct (V1/V4
    /// survive only as version macros). NVIDIA's Tegra nvgpu does parse a
    /// falcon_ucode_desc_v1, but its is 48 bytes WITH interface_offset and word
    /// 0 unioned with stored_size - so "V1" is not one stable layout even across
    /// NVIDIA's own drivers. The 40-byte/no-interface table remains unverified
    /// against discrete Pascal silicon; this arm is inspection-only and
    /// find_fwsec refuses a V1-labeled FWSEC (see fwsec_v1_is_refused).
    #[test]
    fn desc_v1_layout_is_decoded() {
        let mut rom = vec![0u8; 0x80];
        rom[0..4].copy_from_slice(&0x0028_0101u32.to_le_bytes()); // 40B, ver 1, header
        rom[4..8].copy_from_slice(&0x1111_1111u32.to_le_bytes()); // stored
        rom[8..12].copy_from_slice(&0x2222_2222u32.to_le_bytes()); // uncompressed
        rom[12..16].copy_from_slice(&0x3333_3333u32.to_le_bytes()); // virtual entry
        rom[16..20].copy_from_slice(&0x4444_4444u32.to_le_bytes()); // imem phys
        rom[20..24].copy_from_slice(&0x5555_5555u32.to_le_bytes()); // imem load
        rom[32..36].copy_from_slice(&0x7777_7777u32.to_le_bytes()); // dmem phys
        rom[36..40].copy_from_slice(&0x8888_8888u32.to_le_bytes()); // dmem load
        let entry = UcodeEntry { app_id: APP_ID_FWSEC_PROD, target_id: 0, desc_off: 0 };
        match parse_desc(&rom, 0, &entry).expect("V1 must parse") {
            Desc::V1(d) => {
                assert_eq!(d.stored_size, 0x1111_1111);
                assert_eq!(d.uncompressed_size, 0x2222_2222);
                assert_eq!(d.virtual_entry, 0x3333_3333);
                assert_eq!(d.imem_phys_base, 0x4444_4444);
                assert_eq!(d.imem_load_size, 0x5555_5555);
                assert_eq!(d.dmem_phys_base, 0x7777_7777);
                assert_eq!(d.dmem_load_size, 0x8888_8888);
                assert!(!d.encrypted, "bit2 clear in this fixture");
            }
            other => panic!("expected V1, got {other:?}"),
        }
    }

    /// A FWSEC (0x85) entry carrying a V1 descriptor is not a valid extraction:
    /// the FWSEC parser in kernel_gsp_fwsec.c accepts V2/V3 only. Before the V1
    /// arm existed this failed as UnsupportedDescVersion; now that V1 parses,
    /// the guard keeps the failure loud at extraction time.
    #[test]
    fn fwsec_v1_is_refused() {
        let mut rom = synthetic_vbios_lic_then_fwsec();
        // Swap entry 1's descriptor from V2 (60B) to V1 (40B), same offset.
        rom[0x780..0x784].copy_from_slice(&0x0028_0101u32.to_le_bytes());
        match find_fwsec(&rom, false) {
            Err(VbiosError::FwsecDescVersion { version }) => assert_eq!(version, 1),
            Ok(_) => panic!("V1-labeled FWSEC must be refused"),
            Err(other) => panic!("expected FwsecDescVersion, got {other}"),
        }
    }
    /// It used to be parsed and dropped, so a ciphertext FWSEC could be packaged
    /// as if it were usable.
    #[test]
    fn desc_encrypted_bit_is_surfaced() {
        let mut rom = vec![0u8; 0x80];
        // 60-byte V2 descriptor, header bit + encrypted bit set.
        rom[0..4].copy_from_slice(&0x003C_0205u32.to_le_bytes());
        rom[4..8].copy_from_slice(&0x1000u32.to_le_bytes());
        let entry = UcodeEntry { app_id: APP_ID_FWSEC_PROD, target_id: 0, desc_off: 0 };
        match parse_desc(&rom, 0, &entry).expect("V2 must parse") {
            Desc::V2(d) => assert!(d.encrypted, "bit2 must surface as encrypted=true"),
            other => panic!("expected V2, got {other:?}"),
        }
    }

}
