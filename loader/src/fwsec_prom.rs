//! Pure PROM parser shared by the UEFI loader and host regression tests.
//! Field definitions: NVIDIA open-gpu-kernel-modules 610.43.03,
//! kernel_gsp_fwsec.c and pci_exp_table.h. No device I/O occurs here.

/// Extracted FWSEC from the card's own PROM (V2 or V3 descriptor semantics).
pub struct FwsecInfo {
    pub image_off: u32,   // image bytes within the ROM (after desc+sig area)
    pub image_size: u32,  // stored size rounded up to 256 (DMA granularity)
    pub stored_size: u32, // exact stored bytes from the descriptor
    pub imem_pa: u32,
    pub imem_va: u32,
    pub imem_size: u32,
    pub dmem_pa: u32,
    pub dmem_size: u32,
    pub data_off: u32, // DMEMOffset for V2; IMEMLoadSize for V3
    pub pkc_off: u32,  // PKC signature slot offset in DMEM
    pub engine_mask: u32,
    pub ucode_id: u32,
    pub sig_versions: u32,  // slot bitmap
    pub sig_off: u32,       // signature area within the ROM (desc + 44)
    pub interface_off: u32, // FRTS interface block offset in DMEM
    pub imem_sec_base: u32, // V2 NS/SEC split: secure segment IMEM base (0 on V3)
    pub imem_sec_size: u32, // V2 secure segment size (0 on V3)
}

pub(super) fn le16(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
pub(super) fn le32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

fn find_bit_header(rom: &[u8]) -> Option<usize> {
    let mut a = 0usize;
    while a + 12 <= rom.len() {
        if le16(rom, a) == 0xB8FF && &rom[a + 2..a + 6] == b"BIT\0" {
            let hs = rom[a + 8] as usize;
            if hs >= 12 && a + hs <= rom.len() {
                let sum: u32 = rom[a..a + hs].iter().map(|&b| b as u32).sum();
                if sum & 0xFF == 0 {
                    return Some(a);
                }
            }
        }
        a += 2;
    }
    None
}

/// Expansion-ROM relocate, kernel semantics (twin of the tested
/// assets::vbios::find_expansion_offset).
fn find_expansion_offset(rom: &[u8]) -> Option<usize> {
    let mut curr = 0usize;
    let mut base_size = None;
    for _ in 0..16 {
        let pcir = le16(rom, curr + 0x18) as usize;
        let p = curr + pcir;
        match rom.get(p..p + 4) {
            Some(b"PCIR") | Some(b"NPDS") | Some(b"RGIS") => {}
            _ => return None,
        }
        let slen = le16(rom, p + 0x0a) as usize;
        let imglen = le16(rom, p + 0x10) as usize;
        let code = rom[p + 0x14];
        let mut sub = imglen;
        let npde_at = p + slen + 0xf & !0xf;
        if rom.get(npde_at..npde_at + 4) == Some(b"NPDE".as_slice()) {
            let rev = le16(rom, npde_at + 4);
            if rev == 0x100 || rev == 0x101 {
                sub = le16(rom, npde_at + 8) as usize;
            }
        }
        if code == 0xE0 {
            return Some(curr - base_size?);
        }
        if code == 0x00 && base_size.is_none() {
            base_size = Some(sub * 512);
        }
        if sub == 0 || curr + sub * 512 + 0x20 > rom.len() {
            return None;
        }
        curr += sub * 512;
    }
    None
}

/// Compact BIT -> token 0x70 -> ucode table -> FWSEC walk (V2/V3).
pub fn prom_find_fwsec(rom: &[u8], debug: bool) -> Option<FwsecInfo> {
    prom_find_ucode(rom, if debug { 0x45 } else { 0x85 })
}

/// Scrubber ucode (app 0x49) - the kernel runs it on SEC2 BEFORE FWSEC when
/// BSI SCRUBBER_HANDOFF < DONE (kgspExecuteScrubberIfNeeded_AD102).
pub fn prom_find_scrubber(rom: &[u8]) -> Option<FwsecInfo> {
    prom_find_ucode(rom, 0x49)
}

fn prom_find_ucode(rom: &[u8], want_app: u8) -> Option<FwsecInfo> {
    let bit = find_bit_header(rom)?;
    let expansion = find_expansion_offset(rom)?;
    let hdr_size = rom[bit + 8] as usize;
    let stride = rom[bit + 9] as usize;
    let tokens = rom[bit + 10] as usize;
    if !(6..=8).contains(&stride) {
        return None;
    }
    for t in 0..tokens {
        let tok = bit + hdr_size + t * stride;
        if rom.get(tok..tok + 2) != Some(&[0x70, 0x02][..]) {
            continue;
        }
        let data_ptr = if stride >= 8 {
            le32(rom, tok + 4) as usize
        } else {
            le16(rom, tok + 4) as usize
        };
        let table = expansion + le32(rom, data_ptr) as usize;
        if rom.get(table) != Some(&1) {
            continue;
        }
        let esz = rom[table + 2] as usize;
        let count = rom[table + 3] as usize;
        for e in 0..count {
            let at = table + 6 + e * esz;
            if *rom.get(at)? != want_app {
                continue;
            }
            let desc_off = expansion + le32(rom, at + 2) as usize;
            let vdesc = le32(rom, desc_off);
            let ver = (vdesc >> 8) & 0xFF;
            let size = vdesc >> 16;
            if want_app != 0x85 && ver != 3 {
                continue; // non-FWSEC entries only parsed in V3 (scrubber 0x49)
            }
            if ver == 2 {
                // NVIDIA FALCON_UCODE_DESC_V2 (60 bytes): IMEMVirtBase is
                // at +28 and DMEMOffset is at +40. +24 is IMEMLoadSize;
                // +52 is altIMEMLoadSize, which may differ from DMEMOffset.
                if size < 60 || desc_off + 60 > rom.len() {
                    return None;
                }
                let stored = le32(rom, desc_off + 4);
                let sec_base = le32(rom, desc_off + 32);
                let sec_size = le32(rom, desc_off + 36);
                let dmem_load = le32(rom, desc_off + 48);
                let dmem_off_img = le32(rom, desc_off + 40);
                return Some(FwsecInfo {
                    image_off: desc_off as u32 + size,
                    image_size: (stored + 255) & !255,
                    stored_size: stored,
                    imem_pa: le32(rom, desc_off + 20),
                    imem_size: le32(rom, desc_off + 24),
                    imem_va: le32(rom, desc_off + 28),
                    dmem_pa: 0,
                    dmem_size: dmem_load,
                    data_off: dmem_off_img,
                    pkc_off: le32(rom, desc_off + 8),
                    engine_mask: 0x400,
                    ucode_id: 9,
                    sig_versions: 0,
                    sig_off: desc_off as u32 + 44,
                    interface_off: le32(rom, desc_off + 16),
                    imem_sec_base: sec_base,
                    imem_sec_size: sec_size,
                });
            }
            if ver != 3 || size < 44 || desc_off + 44 > rom.len() {
                return None; // other versions unsupported
            }
            // Field semantics aligned 1:1 with the TESTED assets::vbios V3
            // mapping (real-machine log caught a semantic mixup: imem/dmem
            // phys bases read as sizes -> imem=0x0/0xf500).
            let imem_phys = le32(rom, desc_off + 16);
            let imem_load = le32(rom, desc_off + 20);
            let dmem_phys = le32(rom, desc_off + 28);
            let dmem_load = le32(rom, desc_off + 32);
            // Reference semantics (vbios_610.c): the DMA image size is the
            // STORED size rounded UP to 256 (Falcon DMA granularity); the
            // tail is zero-filled by us.
            let stored = le32(rom, desc_off + 4);
            return Some(FwsecInfo {
                image_off: desc_off as u32 + size,
                image_size: (stored + 255) & !255,
                stored_size: stored,
                imem_pa: imem_phys,
                imem_size: imem_load,
                imem_va: le32(rom, desc_off + 24),
                dmem_pa: dmem_phys,
                dmem_size: dmem_load,
                // image layout is [IMEM | DMEM]: DMEM begins after IMEM load
                data_off: imem_load,
                pkc_off: le32(rom, desc_off + 8),
                engine_mask: le16(rom, desc_off + 36) as u32,
                ucode_id: rom[desc_off + 38] as u32,
                sig_versions: le16(rom, desc_off + 40) as u32,
                sig_off: desc_off as u32 + 44,
                interface_off: le32(rom, desc_off + 12),
                imem_sec_base: 0,
                imem_sec_size: 0,
            });
        }
    }
    None
}
