//! Synthetic BIT/NPDE regressions; no firmware assets or device I/O.
use assets::vbios::{self, VbiosError};

const BIT: usize = 0x80;
const TABLE: usize = 0x700;

fn put16(rom: &mut [u8], at: usize, value: u16) {
    rom[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn checksum(rom: &mut [u8]) {
    rom[BIT + 11] = 0;
    let end = BIT + rom[BIT + 8] as usize;
    rom[BIT + 11] = rom[BIT..end].iter().fold(0u8, |s, b| s.wrapping_sub(*b));
}

fn synthetic_rom(rev: u16) -> Vec<u8> {
    let mut rom = vec![0; 0x1000];
    // PCIR strides directly to 0x600; NPDE instead visits the EFI image at
    // 0x200 first. Only the latter produces expansion adjust 0x400.
    for (at, code, length, sub) in [
        (0, 0, 3, 1),
        (0x200, 3, 2, 2),
        (0x600, 0xe0, 5, 5),
    ] {
        put16(&mut rom, at, 0xaa55);
        put16(&mut rom, at + 0x18, 0x20);
        rom[at + 0x20..at + 0x24].copy_from_slice(b"PCIR");
        put16(&mut rom, at + 0x2a, 0x18);
        put16(&mut rom, at + 0x30, length);
        rom[at + 0x34] = code;
        rom[at + 0x40..at + 0x44].copy_from_slice(b"NPDE");
        put16(&mut rom, at + 0x44, rev);
        put16(&mut rom, at + 0x46, 12);
        put16(&mut rom, at + 0x48, sub);
    }
    rom[BIT..BIT + 6].copy_from_slice(b"\xff\xb8BIT\0");
    rom[BIT + 8..BIT + 11].copy_from_slice(&[12, 6, 1]);
    checksum(&mut rom);
    rom[BIT + 12..BIT + 18].copy_from_slice(&[0x70, 2, 4, 0, 0xe0, 0]);
    rom[0xe0..0xe4].copy_from_slice(&((TABLE - 0x400) as u32).to_le_bytes());
    rom[TABLE..TABLE + 6].copy_from_slice(&[1, 6, 6, 1, 2, 60]);
    rom[TABLE + 6] = vbios::APP_ID_FWSEC_PROD;
    rom
}

#[test]
fn supported_npde_revisions_follow_subimage_length() {
    for rev in [0x100, 0x101] {
        let rom = synthetic_rom(rev);
        assert_ne!(rom[BIT + 11], 0);
        assert_eq!(vbios::find_expansion_offset(&rom).unwrap(), 0x400);
        let (entries, expansion, table) = vbios::find_ucode_table(&rom).unwrap();
        assert_eq!((expansion, table), (0x400, TABLE));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].app_id, vbios::APP_ID_FWSEC_PROD);
    }
}

#[test]
fn checksum_does_not_extend_declared_token_count() {
    let mut rom = synthetic_rom(0x100);
    rom[BIT + 10] = 0;
    checksum(&mut rom);
    assert_ne!(rom[BIT + 11], 0);
    // A valid-looking undeclared token follows the zero-entry header.
    assert!(matches!(
        vbios::find_ucode_table(&rom),
        Err(VbiosError::NoFalconTable)
    ));
}

#[test]
fn extended_bit_header_is_refused_even_with_valid_checksum() {
    let mut rom = synthetic_rom(0x100);
    rom[BIT + 8] = 14;
    rom.copy_within(BIT + 12..BIT + 18, BIT + 14);
    rom[BIT + 12..BIT + 14].fill(0);
    checksum(&mut rom);
    assert_eq!(vbios::find_bit_header(&rom).unwrap(), BIT);
    assert!(matches!(
        vbios::find_ucode_table(&rom),
        Err(VbiosError::UnsupportedBitHeaderSize(14))
    ));
}

#[test]
fn unknown_npde_revision_follows_bounded_common_prefix() {
    let rom = synthetic_rom(0x777);
    assert_eq!(vbios::find_expansion_offset(&rom).unwrap(), 0x400);
}

#[test]
fn private_last_image_stops_before_trailing_expansion() {
    for rev in [0x100, 0x101, 0x777] {
        let mut rom = synthetic_rom(rev);
        rom[0x24a] = 0x80; // EFI image is last; trailing EXT must not be used.
        assert!(matches!(
            vbios::find_expansion_offset(&rom),
            Err(VbiosError::BitHeaderNotFound)
        ));
    }
}

#[test]
fn invalid_npde_lengths_are_refused() {
    for (field, value) in [(0x48, 0), (0x48, 0xffff), (0x46, 9), (0x46, 10), (0x46, 0xffff)] {
        let mut rom = synthetic_rom(0x777);
        put16(&mut rom, field, value);
        assert!(matches!(
            vbios::find_expansion_offset(&rom),
            Err(VbiosError::Truncated)
        ));
    }
    let mut rom = synthetic_rom(0x777);
    rom.truncate(0x4a); // subimageLen exists, but the termination byte does not.
    assert!(matches!(
        vbios::find_expansion_offset(&rom),
        Err(VbiosError::Truncated)
    ));
}
