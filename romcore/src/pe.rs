//! PE/COFF sanity decode for candidate GOP images (MZ, machine, optional
//! header magic, subsystem, SizeOfImage). Nothing here trusts the image:
//! these checks only decide whether a buffer is shaped like the x64
//! boot-services driver we are about to LoadImage.

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PeError {
    NoMz,
    NoPeSignature,
    NotPe32Plus,
    Truncated,
}

impl fmt::Display for PeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMz => write!(f, "no MZ signature"),
            Self::NoPeSignature => write!(f, "no PE\\0\\0 signature at e_lfanew"),
            Self::NotPe32Plus => write!(f, "not a PE32+ (0x20b) image"),
            Self::Truncated => write!(f, "PE headers truncated"),
        }
    }
}

/// x86-64 machine type.
pub const MACHINE_X64: u16 = 0x8664;
/// EFI_BOOT_SERVICE_DRIVER subsystem.
pub const SUBSYSTEM_EFI_BOOT_SERVICE_DRIVER: u16 = 11;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeInfo {
    pub machine: u16,
    pub subsystem: u16,
    /// SizeOfImage from the optional header.
    pub size_of_image: u32,
    /// AddressOfEntryPoint.
    pub entry_point: u32,
}

fn u16v(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?))
}
fn u32v(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

/// Parse and sanity-check a PE32+ image header.
pub fn parse(image: &[u8]) -> Result<PeInfo, PeError> {
    if u16v(image, 0) != Some(0x5A4D) {
        return Err(PeError::NoMz);
    }
    let lfanew = u32v(image, 0x3c).ok_or(PeError::Truncated)? as usize;
    if image.get(lfanew..lfanew + 4) != Some(b"PE\0\0".as_slice()) {
        return Err(PeError::NoPeSignature);
    }
    // COFF header: machine @+4, optional header @+24.
    let machine = u16v(image, lfanew + 4).ok_or(PeError::Truncated)?;
    let opt = lfanew + 24;
    let magic = u16v(image, opt).ok_or(PeError::Truncated)?;
    if magic != 0x20b {
        return Err(PeError::NotPe32Plus);
    }
    // PE32+ optional header: SizeOfImage @+56, Subsystem @+68.
    Ok(PeInfo {
        machine,
        subsystem: u16v(image, opt + 68).ok_or(PeError::Truncated)?,
        size_of_image: u32v(image, opt + 56).ok_or(PeError::Truncated)?,
        entry_point: u32v(image, opt + 16).ok_or(PeError::Truncated)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    /// Minimal valid PE32+ header fixture.
    fn fixture() -> Vec<u8> {
        let mut b = vec![0u8; 0x200];
        b[0] = b'M';
        b[1] = b'Z';
        b[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        b[0x80..0x84].copy_from_slice(b"PE\0\0");
        b[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes()); // machine
        b[0x98..0x9a].copy_from_slice(&0x20bu16.to_le_bytes()); // PE32+ magic
        b[0x9a..0x9c].copy_from_slice(&9u16.to_le_bytes()); // linker
        b[0xa8..0xac].copy_from_slice(&0x1000u32.to_le_bytes()); // entry (opt+16)
        b[0xd0..0xd4].copy_from_slice(&0x20000u32.to_le_bytes()); // SizeOfImage
        b[0xdc..0xde].copy_from_slice(&11u16.to_le_bytes()); // subsystem
        b
    }

    #[test]
    fn parses_valid_header() {
        let info = parse(&fixture()).expect("parse");
        assert_eq!(info.machine, MACHINE_X64);
        assert_eq!(info.subsystem, SUBSYSTEM_EFI_BOOT_SERVICE_DRIVER);
        assert_eq!(info.size_of_image, 0x20000);
        assert_eq!(info.entry_point, 0x1000);
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse(&[0u8; 64]), Err(PeError::NoMz));
        let mut b = fixture();
        b[0x80] = b'X';
        assert_eq!(parse(&b), Err(PeError::NoPeSignature));
        let mut b = fixture();
        b[0x98..0x9a].copy_from_slice(&0x10bu16.to_le_bytes()); // PE32, not PE32+
        assert_eq!(parse(&b), Err(PeError::NotPe32Plus));
    }
}
