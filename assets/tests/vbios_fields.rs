//! Synthetic format regressions: no firmware or card-specific dumps required.
use assets::vbios::{self, Desc, VbiosError};

// Compile exactly the parser used by the UEFI loader, without device I/O.
#[allow(dead_code)]
#[path = "../../loader/src/fwsec_prom.rs"]
mod prom;

const BIT: usize = 0x80;
const TOKEN: usize = BIT + 12;
const TABLE: usize = 0x700;
const DESC: usize = 0x780;

fn put16(rom: &mut [u8], offset: usize, value: u16) {
    rom[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put32(rom: &mut [u8], offset: usize, value: u32) {
    rom[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn checksum(rom: &mut [u8]) {
    rom[BIT + 11] = 0;
    rom[BIT + 11] = 0u8.wrapping_sub(
        rom[BIT..BIT + 12]
            .iter()
            .fold(0u8, |sum, byte| sum.wrapping_add(*byte)),
    );
}

fn image_header(rom: &mut [u8], offset: usize, code: u8, length: u16, sub: u16, rev: u16) {
    put16(rom, offset, 0xaa55);
    put16(rom, offset + 0x18, 0x20);
    let pci = offset + 0x20;
    rom[pci..pci + 4].copy_from_slice(b"PCIR");
    put16(rom, pci + 0x0a, 0x18);
    put16(rom, pci + 0x10, length);
    rom[pci + 0x14] = code;
    let npde = offset + 0x40;
    rom[npde..npde + 4].copy_from_slice(b"NPDE");
    put16(rom, npde + 4, rev);
    put16(rom, npde + 6, 12);
    put16(rom, npde + 8, sub);
}

fn synthetic_rom(rev: u16) -> Vec<u8> {
    let mut rom = vec![0; 0x1000];
    // PCIR says the first image spans 3 blocks, but NPDE says 1 block.
    // The 2-block EFI image then puts VBIOS_EXT at 0x600; adjust = 0x400.
    image_header(&mut rom, 0, 0, 3, 1, rev);
    image_header(&mut rom, 0x200, 3, 2, 2, rev);
    image_header(&mut rom, 0x600, 0xe0, 5, 5, rev);
    rom[BIT..BIT + 6].copy_from_slice(b"\xff\xb8BIT\0");
    rom[BIT + 8] = 12;
    rom[BIT + 9] = 8;
    rom[BIT + 10] = 1;
    checksum(&mut rom);
    rom[TOKEN..TOKEN + 4].copy_from_slice(&[0x70, 2, 4, 0]);
    put32(&mut rom, TOKEN + 4, 0xe0);
    put32(&mut rom, 0xe0, (TABLE - 0x400) as u32);
    rom[TABLE..TABLE + 6].copy_from_slice(&[1, 6, 6, 1, 2, 60]);
    rom[TABLE + 6] = 0x85;
    put32(&mut rom, TABLE + 8, (DESC - 0x400) as u32);
    put32(&mut rom, DESC, (60 << 16) | (2 << 8) | 1);
    // Distinct field values expose aliases hidden when the values coincide.
    let fields = [
        0x200, 0x200, 0, 0x10, 0x100, 0x100, 0x400, 0x440, 0xc0, 0x180, 0, 0x40, 0x300, 0x50,
    ];
    for (i, value) in fields.into_iter().enumerate() {
        put32(&mut rom, DESC + 4 + i * 4, value);
    }
    rom
}

#[test]
fn npde_10_subimage_length_is_used_in_both_parsers() {
    assert_npde(0x100);
}

#[test]
fn npde_11_subimage_length_is_used_in_both_parsers() {
    assert_npde(0x101);
}

fn assert_npde(rev: u16) {
    let rom = synthetic_rom(rev);
    assert_eq!(vbios::find_expansion_offset(&rom).unwrap(), 0x400);
    let (_, expansion, table) = vbios::find_ucode_table(&rom).unwrap();
    assert_eq!((expansion, table), (0x400, TABLE));
    let fw = prom::prom_find_fwsec(&rom, false).expect("loader finds V2 descriptor");
    assert_eq!(fw.image_off, (DESC + 60) as u32);
}

#[test]
fn old_misencoded_revisions_do_not_override_pcir_length() {
    for rev in [0x10, 0x11] {
        let rom = synthetic_rom(rev);
        assert_eq!(vbios::find_expansion_offset(&rom).unwrap(), 0);
        assert!(prom::prom_find_fwsec(&rom, false).is_none());
    }
}

#[test]
fn checksum_byte_does_not_extend_token_count() {
    let mut rom = synthetic_rom(0x100);
    rom[BIT + 10] = 0;
    checksum(&mut rom);
    assert_ne!(rom[BIT + 11], 0);
    // The valid-looking token remains immediately after the zero-entry table.
    // Treating HeaderChksum as the count high byte incorrectly discovers it.
    assert!(matches!(
        vbios::find_ucode_table(&rom),
        Err(VbiosError::NoFalconTable)
    ));
    assert!(prom::prom_find_fwsec(&rom, false).is_none());
}

#[test]
fn declared_token_with_nonzero_checksum_is_still_found() {
    let rom = synthetic_rom(0x100);
    assert_ne!(rom[BIT + 11], 0);
    let (entries, _, _) = vbios::find_ucode_table(&rom).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].app_id, vbios::APP_ID_FWSEC_PROD);
    assert!(prom::prom_find_fwsec(&rom, false).is_some());
}

#[test]
fn loader_v2_offsets_agree_with_host_descriptor_parser() {
    let rom = synthetic_rom(0x100);
    let (_, Desc::V2(v2)) = vbios::find_fwsec(&rom, false).unwrap() else {
        panic!("expected a V2 descriptor")
    };
    let fw = prom::prom_find_fwsec(&rom, false).unwrap();
    assert_eq!(fw.imem_size, 0x100);
    assert_eq!(fw.imem_va, 0x400);
    assert_eq!(fw.data_off, 0x180);
    assert_eq!(fw.imem_va, v2.imem_virt_base);
    assert_eq!(fw.data_off, v2.dmem_offset);
    assert_ne!(fw.imem_va, fw.imem_size);
    assert_ne!(fw.data_off, 0x300); // altIMEMLoadSize at +52 is not DMEMOffset.
}

#[test]
fn extracted_production_parser_retains_v3_field_mapping() {
    let mut rom = synthetic_rom(0x100);
    put32(&mut rom, DESC, (44 << 16) | (3 << 8) | 1);
    let fields = [0x200, 0x30, 0x10, 0x200, 0x100, 0x400, 0x80, 0x40];
    for (i, value) in fields.into_iter().enumerate() {
        put32(&mut rom, DESC + 4 + i * 4, value);
    }
    put16(&mut rom, DESC + 36, 0x400);
    rom[DESC + 38] = 9;
    rom[DESC + 39] = 2;
    put16(&mut rom, DESC + 40, 3);
    let (_, Desc::V3(v3)) = vbios::find_fwsec(&rom, false).unwrap() else {
        panic!("expected a V3 descriptor")
    };
    let fw = prom::prom_find_fwsec(&rom, false).unwrap();
    assert_eq!(fw.image_off, (DESC + 44) as u32);
    assert_eq!(fw.imem_va, v3.imem_virt_base);
    assert_eq!(fw.data_off, v3.imem_load_size);
    assert_eq!(fw.dmem_pa, v3.dmem_phys_base);
    assert_eq!(fw.engine_mask, u32::from(v3.engine_id_mask));
    assert_eq!(fw.sig_versions, u32::from(v3.signature_versions));
}
