//! EFI PCI expansion-ROM header (UEFI spec 13.4.2) + payload sizing.
//!
//! Empirically pinned on TU106 (1.rom, EFI image @0xea00) and AD107M
//! (ASUS_4060_laptop OC 9507.1640, @0xfc00): both carry EfiSignature
//! 0x0EF1, EfiSubsystem 11 (boot-services driver), EfiMachineType 0x8664,
//! CompressionType 1 and a payload at EfiImageHeaderOffset whose first
//! two u32 are {compressed size, uncompressed size} of a UEFI-compression
//! stream (EFI_DECOMPRESS_PROTOCOL format, Version 1).

use core::fmt;

/// UEFI spec: EFI_PCI_EXPANSION_ROM_HEADER_EFISIGNATURE.
pub const EFI_SIGNATURE: u32 = 0x0EF1;
/// UEFI spec: EFI_PCI_EXPANSION_ROM_HEADER_COMPRESSED.
pub const COMPRESSION_EFI_STANDARD: u16 = 1;
/// EFI subsystem value for a boot-services-driver image.
pub const SUBSYSTEM_BOOT_SERVICE_DRIVER: u16 = 11;
/// x86-64 PE machine type.
pub const MACHINE_X64: u16 = 0x8664;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EfiRomError {
    NotEfiImage,
    Truncated,
    BadPayloadHeader,
}

impl fmt::Display for EfiRomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotEfiImage => write!(f, "not an EFI expansion-ROM image (no 0xAA55/0x0EF1)"),
            Self::Truncated => write!(f, "EFI image truncated"),
            Self::BadPayloadHeader => write!(f, "compressed payload header malformed"),
        }
    }
}

fn u16v(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?))
}
fn u32v(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

/// Parsed EFI PCI expansion-ROM image header.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EfiRomHeader {
    /// Image length in 512-byte units (InitializationSize).
    pub size512: u16,
    pub efi_subsystem: u16,
    pub efi_machine_type: u16,
    pub compression_type: u16,
    /// Offset of the image payload inside the ROM image.
    pub image_header_offset: usize,
}

impl EfiRomHeader {
    /// Parse the header at the start of a code-type-0x03 ROM image.
    pub fn parse(image: &[u8]) -> Result<Self, EfiRomError> {
        if u16v(image, 0) != Some(0xAA55) || u32v(image, 4) != Some(EFI_SIGNATURE) {
            return Err(EfiRomError::NotEfiImage);
        }
        Ok(Self {
            size512: u16v(image, 2).ok_or(EfiRomError::Truncated)?,
            efi_subsystem: u16v(image, 8).ok_or(EfiRomError::Truncated)?,
            efi_machine_type: u16v(image, 0x0a).ok_or(EfiRomError::Truncated)?,
            compression_type: u16v(image, 0x0c).ok_or(EfiRomError::Truncated)?,
            image_header_offset: u16v(image, 0x16).ok_or(EfiRomError::Truncated)? as usize,
        })
    }
}

/// The decompressable payload located at EfiImageHeaderOffset:
/// {u32 compressed size, u32 uncompressed size} + the stream itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EfiPayload {
    /// Bytes of compressed stream (excluding this 8-byte header).
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    /// Byte offset of the stream (the 8-byte header's end).
    pub stream_offset: usize,
}

pub fn payload(image: &[u8], header: &EfiRomHeader) -> Result<EfiPayload, EfiRomError> {
    let at = header.image_header_offset;
    if at + 8 > image.len() {
        return Err(EfiRomError::Truncated);
    }
    let compressed = u32v(image, at).ok_or(EfiRomError::BadPayloadHeader)?;
    let uncompressed = u32v(image, at + 4).ok_or(EfiRomError::BadPayloadHeader)?;
    if compressed == 0 || uncompressed == 0 {
        return Err(EfiRomError::BadPayloadHeader);
    }
    Ok(EfiPayload {
        compressed_size: compressed,
        uncompressed_size: uncompressed,
        stream_offset: at + 8,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// TU106 EFI image: header fields measured in the walk test, payload
    /// {0x11447 compressed, 0x21290 uncompressed}.
    #[test]
    fn tu106_header_and_payload_when_present() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/tu106.rom");
        let Ok(img) = std::fs::read(&p) else { return };
        let image = &img[0xea00..];
        let h = EfiRomHeader::parse(image).expect("header");
        assert_eq!(h.size512, 139);
        assert_eq!(h.efi_subsystem, SUBSYSTEM_BOOT_SERVICE_DRIVER);
        assert_eq!(h.efi_machine_type, MACHINE_X64);
        assert_eq!(h.compression_type, COMPRESSION_EFI_STANDARD);
        assert_eq!(h.image_header_offset, 0x50);
        let pl = payload(image, &h).expect("payload");
        assert_eq!(pl.compressed_size, 0x11447);
        assert_eq!(pl.uncompressed_size, 0x21290);
    }

    #[test]
    fn rejects_legacy_image() {
        // A PC-AT (code 0x00) image: 0xAA55 but no 0x0EF1 at +4.
        let mut rom = vec![0u8; 0x40];
        rom[0..2].copy_from_slice(&0xAA55u16.to_le_bytes());
        assert_eq!(EfiRomHeader::parse(&rom), Err(EfiRomError::NotEfiImage));
    }
}
