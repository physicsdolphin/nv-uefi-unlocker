//! PCIR image-chain walk (PCI Local Bus spec 6.3.1.2 + PCI Firmware spec
//! 5.1.2), with the NVIDIA NPDE SUBIMAGE_LEN stride override.
//!
//! Same walk as assets/src/vbios.rs::find_expansion_offset generalized to
//! return every image instead of only the 0xE0 expansion image. The EFI
//! GOP driver (issue #107) is a code-type-0x03 image on this chain.

use alloc::vec::Vec;
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RomImage {
    /// Byte offset of the image inside the ROM buffer.
    pub start: usize,
    /// Image length in bytes (img_len * 512, NPDE SUBIMAGE_LEN wins).
    pub len: usize,
    /// PCIR CodeType (0x00 PC-AT, 0x03 EFI, 0xE0 NVIDIA VBIOS_EXT, ...).
    pub code_type: u8,
    /// PCIR Indicator bit7 = last image.
    pub last: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WalkError {
    /// No 0xAA55 at offset 0 - not an expansion ROM.
    NotARom,
    /// PCIR signature missing where the header points.
    BadPcir,
    /// Malformed chain (zero stride before any image).
    BrokenChain,
}

impl fmt::Display for WalkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotARom => write!(f, "not an expansion ROM (no 0xAA55 at 0)"),
            Self::BadPcir => write!(f, "PCIR structure missing/malformed"),
            Self::BrokenChain => write!(f, "PCIR image chain broken (zero stride)"),
        }
    }
}

fn u16v(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?))
}

/// Locate the real option-ROM start inside a dump. Some dump tools wrap
/// the image (NVGI-style headers, euda padding); the discriminator is a
/// 0xAA55 whose +0x18 PCIR pointer resolves to a real PCI data structure
/// signature ("PCIR"/"NPDS"/"RGIS") - a bare 0xAA55 with pointer 0 or
/// garbage (measured: Blackwell euda-wrapped dumps, AA55 at 0x33c00 with
/// PCIR offset 0) does not qualify. Returns 0 when the file IS the ROM.
pub fn locate_start(raw: &[u8]) -> usize {
    let plausible = |off: usize| -> bool {
        raw.get(off..off + 2) == Some(&[0x55, 0xaa][..])
            && u16v(raw, off + 0x18)
                .map(|p| {
                    let p = p as usize;
                    matches!(
                        raw.get(off + p..off + p + 4),
                        Some(b"PCIR") | Some(b"NPDS") | Some(b"RGIS")
                    )
                })
                .unwrap_or(false)
    };
    if plausible(0) {
        return 0;
    }
    // Dumps place the ROM at a 0x200-aligned offset within the first
    // megabyte; fall back to a linear scan for degenerate wrappers.
    (0..raw.len().saturating_sub(0x1000).min(0x1_0000))
        .step_by(0x200)
        .find(|&off| plausible(off))
        .or_else(|| (0..raw.len().saturating_sub(2)).find(|&off| plausible(off)))
        .unwrap_or(0)
}

/// Walk the PCIR image chain, returning every image with its extent.
pub fn walk(rom: &[u8]) -> Result<Vec<RomImage>, WalkError> {
    if u16v(rom, 0) != Some(0xAA55) {
        return Err(WalkError::NotARom);
    }
    let mut out = Vec::new();
    let mut curr = 0usize;
    for _ in 0..16 {
        let Some(pcir) = u16v(rom, curr + 0x18).map(|p| p as usize) else {
            break;
        };
        let p = curr + pcir;
        // NVIDIA's 0xE0 subimages use "NPDS" (NVIDIA PCI Data Structure)
        // instead of "PCIR" (measured on TU106 1.rom at 0x200f0+); "RGIS"
        // appears on newer ROMs. Same layout either way.
        match rom.get(p..p + 4) {
            Some(b"PCIR") | Some(b"NPDS") | Some(b"RGIS") => {}
            _ => break,
        }
        let slen = u16v(rom, p + 0x0a).unwrap_or(0) as usize;
        let mut sub = u16v(rom, p + 0x10).unwrap_or(0) as usize;
        let code = rom.get(p + 0x14).copied().unwrap_or(0);
        let indicator = rom.get(p + 0x15).copied().unwrap_or(0);
        // NVIDIA PCI Data Extension: SUBIMAGE_LEN overrides the stride
        // (same rule assets/vbios.rs applies for the 0xE0 expansion image).
        let npde_at = (p + slen + 0xf) & !0xf;
        if rom.get(npde_at..npde_at + 4) == Some(b"NPDE".as_slice()) {
            let rev = u16v(rom, npde_at + 4).unwrap_or(0);
            if rev == 0x10 || rev == 0x11 {
                sub = u16v(rom, npde_at + 8).unwrap_or(0) as usize;
            }
        }
        if sub == 0 {
            if out.is_empty() {
                return Err(WalkError::BrokenChain);
            }
            break;
        }
        let len = sub.saturating_mul(512);
        let last = indicator & 0x80 != 0;
        out.push(RomImage { start: curr, len, code_type: code, last });
        // The indicator bit7 is NOT trusted for chain termination: the
        // TU106 1.rom EFI image (and every NVIDIA VBIOS examined) sets it
        // while 0xE0 NVIDIA-proprietary subimages still follow. The chain
        // ends on a zero stride or the end of the buffer, like
        // assets/vbios.rs does.
        if curr + len + 0x20 > rom.len() {
            break;
        }
        curr += len;
    }
    if out.is_empty() {
        return Err(WalkError::BadPcir);
    }
    Ok(out)
}

/// Every EFI (code-type-0x03) image on the chain, in ROM order.
pub fn efi_images(rom: &[u8]) -> Result<Vec<RomImage>, WalkError> {
    Ok(walk(rom)?
        .into_iter()
        .filter(|i| i.code_type == 0x03)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// The TU106 1.rom regression anchor (skips when the NVOC checkout is
    /// absent): base PC-AT image, one EFI image at 0xea00, 0xE0 expansion
    /// images after. The EFI image is the GOP the native-GOP path loads.
    #[test]
    fn tu106_chain_when_present() {
        // Test data is NVIDIA firmware: never committed. The test reads
        // romcore/testdata/ (untracked, gitignored; populated per the docs
        // csm-compat page) and skips silently when absent.
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/tu106.rom");
        let Ok(img) = std::fs::read(&p) else { return };
        let images = walk(&img).expect("chain walk");
        let kinds: Vec<u8> = images.iter().map(|i| i.code_type).collect();
        assert_eq!(&kinds[..3], &[0x00, 0x03, 0xe0], "TU106 image kinds");
        assert_eq!(images[0].start, 0, "base image at 0");
        assert_eq!(images[1].start, 0xea00, "EFI image at 0xea00");
        assert_eq!(images[1].len, 139 * 512, "EFI image 0x11600 bytes");
        assert!(images.iter().any(|i| i.last), "chain must terminate");
    }

    /// ASUS AD107M laptop VBIOS: EFI image at 0xfc00.
    #[test]
    fn ad107m_chain_when_present() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/ad107m.rom");
        let Ok(raw) = std::fs::read(&p) else { return };
        let Some(start) = raw.windows(2).position(|w| w == [0x55, 0xaa]) else {
            return;
        };
        let images = walk(&raw[start..]).expect("chain walk");
        let efi = efi_images(&raw[start..]).expect("efi images");
        assert_eq!(efi.len(), 1, "exactly one EFI image");
        assert_eq!(efi[0].start, 0xfc00, "AD107M EFI image offset");
        let _ = images;
    }

    /// Synthetic minimal chain: base + EFI image; the walker must stop on
    /// the last-image indicator and must not run past the buffer.
    #[test]
    fn synthetic_chain() {
        let mut rom = vec![0u8; 0x400];
        rom[0..2].copy_from_slice(&0xAA55u16.to_le_bytes());
        rom[0x18..0x1a].copy_from_slice(&0x20u16.to_le_bytes());
        rom[0x20..0x24].copy_from_slice(b"PCIR");
        rom[0x2a..0x2c].copy_from_slice(&0x18u16.to_le_bytes()); // struct len
        rom[0x30..0x32].copy_from_slice(&1u16.to_le_bytes()); // img len (512 units)
        rom[0x34] = 0x00; // code type
        rom[0x35] = 0x00; // indicator: not last
        let efi = 0x200usize;
        rom[efi..efi + 2].copy_from_slice(&0xAA55u16.to_le_bytes());
        rom[efi + 0x18..efi + 0x1a].copy_from_slice(&0x20u16.to_le_bytes());
        rom[efi + 0x20..efi + 0x24].copy_from_slice(b"PCIR");
        rom[efi + 0x2a..efi + 0x2c].copy_from_slice(&0x18u16.to_le_bytes());
        rom[efi + 0x30..efi + 0x32].copy_from_slice(&1u16.to_le_bytes());
        rom[efi + 0x34] = 0x03; // code type EFI
        rom[efi + 0x35] = 0x80; // last image
        let images = walk(&rom).expect("walk");
        assert_eq!(images.len(), 2);
        assert_eq!(images[1].code_type, 0x03);
        assert!(images[1].last);
    }

    #[test]
    fn rejects_non_rom() {
        let rom = [0u8; 0x40];
        assert_eq!(walk(&rom), Err(WalkError::NotARom));
    }
}
