//! FWSEC ignition phase (unlock_610.c 921-1026) + helpers (vbios_610.c,
//! prep_610.c), no_std ports. Part of the `tx` feature.
#![allow(dead_code)] // pre-wired ahead of the gate; behavior twin in sim/assets



use log::info;
use uefi::boot;

use crate::pci::PciIo;
use crate::tx::{hs_prevalidate, inaccessible, TxError, TxGpu};

/// Extracted FWSEC from the card's own PROM (V3 descriptor semantics).
pub struct FwsecInfo {
    pub image_off: u32,     // image bytes within the ROM (after desc+sig area)
    pub image_size: u32,    // stored size rounded up to 256 (DMA granularity)
    pub stored_size: u32,   // exact stored bytes from the descriptor
    pub imem_pa: u32,
    pub imem_va: u32,
    pub imem_size: u32,
    pub dmem_pa: u32,
    pub dmem_size: u32,
    pub data_off: u32,      // DMEM offset within the image (= IMEM size)
    pub pkc_off: u32,       // PKC signature slot offset in DMEM
    pub engine_mask: u32,
    pub ucode_id: u32,
    pub sig_versions: u32,  // slot bitmap
    pub sig_off: u32,       // signature area within the ROM (desc + 44)
    pub interface_off: u32, // FRTS interface block offset in DMEM
    pub imem_sec_base: u32, // V2 NS/SEC split: secure segment IMEM base (0 on V3)
    pub imem_sec_size: u32, // V2 secure segment size (0 on V3)
}

fn le16(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
/// Bounds-checked variants: every offset below comes out of device data (the
/// card's own PROM), so a malformed token/table/descriptor must end the walk
/// with None, not panic the firmware app or index past the staging buffer.
fn u16v(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?))
}
fn u32v(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

fn le32(d: &[u8], o: usize) -> u32 {
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
            if rev == 0x10 || rev == 0x11 {
                sub = le16(rom, npde_at + 8) as usize;
            }
        }
        if code == 0xE0 {
            return curr.checked_sub(base_size?);
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

/// Compact BIT -> token 0x70 -> ucode table -> FWSEC walk (Ada V3 path).
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
    let hdr_size = *rom.get(bit + 8)? as usize;
    let stride = *rom.get(bit + 9)? as usize;
    let tokens = *rom.get(bit + 10)? as usize;
    if !(6..=8).contains(&stride) {
        return None;
    }
    for t in 0..tokens {
        let tok = bit + hdr_size + t * stride;
        if rom.get(tok..tok + 2) != Some(&[0x70, 0x02][..]) {
            continue;
        }
        let data_ptr = if stride >= 8 {
            u32v(rom, tok + 4)? as usize
        } else {
            u16v(rom, tok + 4)? as usize
        };
        let table = expansion.checked_add(u32v(rom, data_ptr)? as usize)?;
        if rom.get(table) != Some(&1) {
            continue;
        }
        let esz = *rom.get(table + 2)? as usize;
        let count = *rom.get(table + 3)? as usize;
        for e in 0..count {
            let at = table + 6 + e * esz;
            if *rom.get(at)? != want_app {
                continue;
            }
            let desc_off = expansion.checked_add(u32v(rom, at + 2)? as usize)?;
            let vdesc = u32v(rom, desc_off)?;
            let ver = (vdesc >> 8) & 0xFF;
            let size = vdesc >> 16;
            // vDesc bit2: the image is AES ciphertext (the key lives in the
            // fuses). Nothing can be done with it here - it cannot be
            // disassembled for diagnostics and it will not ignite - so name it
            // and refuse instead of handing an opaque blob to the fire path and
            // letting it fail as a geometry error. Task brief 7.3 makes this the
            // first classification step for exactly this reason.
            if vdesc & (1 << 2) != 0 {
                info!(
                    "FWSEC-TRACE: app {want_app:#04x} descriptor at {desc_off:#x} is ENCRYPTED (vDesc bit2, ver {ver}) - refusing: a ciphertext FWSEC cannot be analysed or ignited"
                );
                return None;
            }
            if want_app != 0x85 && ver != 3 {
                continue; // non-FWSEC entries only parsed in V3 (scrubber 0x49)
            }
            if ver == 2 {
                // Turing V2 (60B, BOOT_WITH_LOADER). Field offsets per the
                // community-proven TU106/TU102 measurements (cmp50hx
                // unlock50x v68: iface@0x10, imemLoad@0x18, imemSecBase@0x20,
                // imemSecSize@0x24, dmemLoad@0x30, dmemOff@0x34; task brief
                // §7.3 "InterfaceOffset 插在@16，IMEMSecBase@32/36").
                // NVOC TU106 self-check: stored 0x9a10 = imemLoad 0x9600 +
                // dmem 0x410. The old parse here read iface@12/imem@20/32 —
                // those are VirtualEntry/IMEMPhys/IMEMSecBase, wrong slots.
                if size < 60 || desc_off + 60 > rom.len() {
                    return None;
                }
                let stored = le32(rom, desc_off + 4);
                let sec_base = le32(rom, desc_off + 32);
                let sec_size = le32(rom, desc_off + 36);
                let dmem_load = le32(rom, desc_off + 48);
                let dmem_off_img = le32(rom, desc_off + 52);
                return Some(FwsecInfo {
                    image_off: desc_off as u32 + size,
                    image_size: (stored + 255) & !255,
                    stored_size: stored,
                    imem_pa: le32(rom, desc_off + 20),
                    imem_size: le32(rom, desc_off + 24),
                    imem_va: le32(rom, desc_off + 24),
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

/// gpu_unlock_fuse_version: bit position + 1 of the low-16 value.
pub fn fuse_version(raw: u32) -> u32 {
    let mut value = raw & 0xffff;
    if value == 0 {
        return 0;
    }
    let mut index = 0;
    while value > 1 {
        value >>= 1;
        index += 1;
    }
    index + 1
}

/// Signature slot offset = popcount of version bits BELOW the fuse version.
pub fn fwsec_sig_offset(versions: u32, fuse_ver: u32, sig_size: u32) -> Result<u32, TxError> {
    if fuse_ver >= 16 || versions > 0xffff {
        return Err(TxError::EInval);
    }
    if versions & (1 << fuse_ver) == 0 {
        return Err(TxError::ERange);
    }
    Ok((versions & ((1 << fuse_ver) - 1)).count_ones() * sig_size)
}

/// gpu_unlock_fwsec_patch_frts: locate the audited V3 mapper in FWSEC DMEM
/// and inject the FRTS command carrying our layout address.
pub fn fwsec_patch_frts(image: &mut [u8], fw: &FwsecInfo, frts_offset: u64) -> Result<(), TxError> {
    if frts_offset == 0 || frts_offset & 0xfff != 0 || (frts_offset >> 12) > u32::MAX as u64 {
        return Err(TxError::EInval);
    }
    let d0 = fw.data_off as usize;
    let io = fw.interface_off as usize;
    if d0 + fw.dmem_size as usize > image.len() || io + 4 > fw.dmem_size as usize {
        return Err(TxError::EInval);
    }
    let dmem = &mut image[d0..d0 + fw.dmem_size as usize];
    if dmem[io] != 1 || dmem[io + 1] != 4 || dmem[io + 2] != 8 {
        return Err(TxError::EIO); // unsupported interface
    }
    let entries = dmem[io + 3] as usize;
    let mut off = io + 4;
    if entries < 2 || off + entries * 8 > dmem.len() {
        return Err(TxError::EInval);
    }
    let mut mapper = None;
    for _ in 0..entries {
        let dest = le32(dmem, off + 4) as usize;
        if le32(dmem, off) == 4 {
            if mapper.is_some() || dest + 64 > dmem.len() {
                return Err(TxError::EInval);
            }
            mapper = Some(dest);
        }
        off += 8;
    }
    let mapper = mapper.ok_or(TxError::EIO)?;
    if le16(dmem, mapper + 4) != 3 || le16(dmem, mapper + 6) < 64 {
        return Err(TxError::EIO); // only the audited V3 mapper layout
    }
    let cmd_off = le32(dmem, mapper + 8) as usize;
    let cmd_size = le32(dmem, mapper + 12) as usize;
    if cmd_size < 44 || cmd_off + cmd_size > dmem.len() {
        return Err(TxError::EInval);
    }
    let mut cmd = [0u8; 44];
    cmd[0..4].copy_from_slice(&1u32.to_le_bytes());
    cmd[4..8].copy_from_slice(&24u32.to_le_bytes());
    cmd[20..24].copy_from_slice(&2u32.to_le_bytes());
    cmd[24..28].copy_from_slice(&1u32.to_le_bytes());
    cmd[28..32].copy_from_slice(&20u32.to_le_bytes());
    cmd[32..36].copy_from_slice(&((frts_offset >> 12) as u32).to_le_bytes());
    cmd[36..40].copy_from_slice(&0x100u32.to_le_bytes());
    cmd[40..44].copy_from_slice(&2u32.to_le_bytes());
    dmem[cmd_off..cmd_off + 44].copy_from_slice(&cmd);
    dmem[mapper + 44..mapper + 48].copy_from_slice(&0x15u32.to_le_bytes());
    info!("FWSEC mapper patched: frts={frts_offset:#x} cmd@{cmd_off:#x}");
    Ok(())
}

/// Layout register adapter (twin of the tested sim::layout).
pub struct LayoutOut {
    /// FB size and VGA reservation as read by the layout. The WprMeta needs
    /// both again later; re-reading them there used to EIO on the (tolerated)
    /// gated VGA register on a non-console dGPU.
    pub fb_size: u64,
    pub vga_base: u64,
    pub wpr_end: u64,
    pub frts_offset: u64,
    pub boot_offset: u64,
    pub gsp_offset: u64,
    pub heap_offset: u64,
    pub wpr_start: u64,
    pub nonwpr_start: u64,
}

pub fn layout_from_registers(
    gpu: &TxGpu,
    family: &str,
    image_size: u64,
    boot_size: u64,
) -> Result<LayoutOut, TxError> {
    let fb_raw = gpu.read32_raw(0x0011_83a4);
    let vga_raw = gpu.read32_raw(0x0062_5f04);
    let plm_raw = gpu.read32_raw(0x001f_a7c8);
    let lo_raw = gpu.read32_raw(0x001f_a82c);
    let hi_raw = gpu.read32_raw(0x001f_a830);
    // Log the cold board inputs even when the layout is rejected - the state
    // cannot be reconstructed once the NVIDIA driver has bound.
    info!(
        "layout inputs: fb_mib={fb_raw:#010x} vga={vga_raw:#010x} mmu_plm={plm_raw:#010x} mmu_lo={lo_raw:#010x} mmu_hi={hi_raw:#010x} image={image_size:#x} boot={boot_size:#x}"
    );
    // FB_SIZE_MIB is PLM-LOCKED at PL0 on Turing (measured 2026-10-26/27:
    // badf5040, unlike Ada where it reads) - first fire failed exactly here.
    // Driver-exact source instead: NV_PFB_PRI_MMU_LOCAL_MEMORY_RANGE is
    // programmed by VBIOS devinit with the USABLE FB window and the GSP
    // booter validates WprMeta.fbSize against the same value. Turing
    // overrides neither the register nor the decode: turing/tu102/dev_fb.h
    // keeps the GP102 definition (0x100ce0, LOWER_MAG[9:4], LOWER_SCALE
    // [3:0], ECC_MODE[30]) and kern_mem_sys_tu102.c defines no
    // ReadUsableFbSize body, so TU102 runs kmemsysReadUsableFbSize_GP102
    // verbatim: mag << (scale+20), 15/16 when ECC is on. Field-verified
    // bit-exact on every current-build Turing log: 0x2c9 -> 22528 MiB
    // twice (#17 and #84's 0x1e04 22G board), 0x208 -> 8192 MiB (#62).
    // Do NOT read issue #57's 0x11 as an LMR value - that is the booter
    // mailbox status (meta/staging rejection), misattributed here before.
    // Decode, log, and never guess.
    let fb_mib = if inaccessible(fb_raw) {
        if family == "tu10x" {
            const FB_MIB_TU106_1F07: u32 = 8192;
            const LOCAL_MEMORY_RANGE: u32 = 0x0010_0ce0;
            let lmr = gpu.read32_raw(LOCAL_MEMORY_RANGE);
            let decoded_mib = if !inaccessible(lmr) {
                let mag = (lmr >> 4) & 0x3f; // [9:4] LOWER_MAG
                let scale = lmr & 0xf; // [3:0]
                let bytes = (mag as u64) << (scale + 20);
                let bytes = if lmr & (1 << 30) != 0 { bytes / 16 * 15 } else { bytes };
                let mib = (bytes >> 20) as u32;
                (mib >= 256 && mib <= 64 * 1024).then_some(mib)
            } else {
                None
            };
            match decoded_mib {
                Some(mib) => {
                    info!(
                        "layout: LOCAL_MEMORY_RANGE {lmr:#010x} decoded usable FB = {mib} MiB (ecc={}) - driver source",
                        lmr & (1 << 30) != 0
                    );
                    mib
                }
                None => {
                    // \tu10x-fb-gb.txt override (issue #17): manual escape
                    // hatch when the register decode is gated or implausible.
                    // Plain decimal GiB; everything downstream (vga/frts/wpr/
                    // heap) keys off this one number.
                    let override_mib = crate::tx::read_boot_file("\\tu10x-fb-gb.txt")
                        .and_then(|b| core::str::from_utf8(&b).ok().map(str::trim).and_then(|s| s.parse::<u64>().ok()))
                        .filter(|g| *g >= 1 && *g <= 64);
                    match override_mib {
                        Some(gib) => {
                            info!("layout: \\tu10x-fb-gb.txt override: FB size = {gib} GiB (register decode skipped: {lmr:#010x})");
                            (gib * 1024) as u32
                        }
                        None => {
                            // Config 0x00 = vendor|device<<16; print the id so a
                            // field log identifies the SKU without the marker.
                            let dev = (gpu.pci.config_read_u32(0) >> 16) as u16;
                            info!(
                                "layout: FB_SIZE_MIB PLM-locked on Turing ({fb_raw:#010x}), LOCAL_MEMORY_RANGE {lmr:#010x} undecodable on 10de:{dev:04x} - no driver-exact FB source; defaulting to 8192 MiB - write \\tu10x-fb-gb.txt (GiB) or extend the fallback with a measured card"
                            );
                            FB_MIB_TU106_1F07
                        }
                    }
                }
            }
        } else {
            return Err(TxError::EIO);
        }
    } else {
        fb_raw
    };
    if inaccessible(lo_raw) || inaccessible(hi_raw) {
        return Err(TxError::EIO);
    }
    // MMU_PLM is PLM-locked at PL0 on Turing too (second TU106 fire,
    // 2026-10-27: badf5040). It only feeds the bit0 lock-bit sanity check -
    // the actual layout input is the lo/hi lock window, which reads fine
    // and is bit-identical to the AD107M values the M4 chain verified
    // end-to-end. Skip the check where the register is hidden.
    let (plm, plm_check) = if inaccessible(plm_raw) {
        if family == "tu10x" {
            info!(
                "layout: MMU_PLM PLM-locked on Turing ({plm_raw:#010x}) - skipping the lock-bit sanity check (lo/hi window remains the layout input)"
            );
            (0u32, false)
        } else {
            return Err(TxError::EIO);
        }
    } else {
        (plm_raw, true)
    };
    let lo = lo_raw;
    let hi = hi_raw;
    if fb_mib == 0 || fb_mib == !0u32 || (plm_check && plm & 1 == 0) {
        return Err(TxError::ENodev);
    }
    // The VGA workspace pointer is the one layout input a poison value is
    // tolerable for: only bit3 decides, and bit3=0 (also the value on a
    // healthy card) means "no workspace pointer, reserve the FB top 1 MiB".
    // Measured 2026-10-26: a hybrid-shutdown boot with the dGPU as console
    // reads this register fine and takes the bit3=0 path (vga = FB top); a
    // boot where the iGPU owned the console left it gated (0xbadf1301) on the
    // same machine. Both land on the same fallback value.
    let vga = if inaccessible(vga_raw) {
        info!(
            "layout: VGA workspace gated ({vga_raw:#010x}) - dGPU was not the console owner; using the FB-top fallback (bit3=0 path, same value as a healthy card)"
        );
        0
    } else {
        vga_raw
    };
    let usable_fb = (fb_mib as u64) << 20;
    if usable_fb < 256 << 20 {
        info!("layout rejected: usable FB below 256 MiB");
        return Err(TxError::ERange);
    }
    let mut vga_base = usable_fb - (1 << 20);
    if vga & (1 << 3) != 0 {
        vga_base = ((vga >> 8) as u64) << 16;
        if vga_base < usable_fb - (1 << 20) {
            vga_base = usable_fb - 0x20000;
        }
    }
    if vga_base >= usable_fb {
        info!("layout rejected: VGA base at or above usable FB");
        return Err(TxError::ERange);
    }
    let mmu_lock_low = ((lo >> 4) as u64) << 12;
    let mmu_hi = ((hi >> 4) as u64) << 12;
    let mmu_top = (mmu_hi.saturating_add(0xffff)) & !0xffffu64;
    let mmu_lock_valid = mmu_hi >= mmu_lock_low && (mmu_top >> 20) == (usable_fb >> 20);
    if !mmu_lock_valid {
        info!("layout mmu: lock ignored (high address is not the FB top)");
    }
    let reserved = if mmu_lock_valid && mmu_lock_low < vga_base { mmu_lock_low } else { vga_base };

    let wpr_end = reserved & !(128 * 1024 - 1);
    let frts = wpr_end - (1 << 20);
    let boot = (frts - boot_size) & !4095u64;
    let gsp = (boot - image_size) & !(64 * 1024 - 1);
    let heap_off = (gsp - (88 << 20)) & !(1024 * 1024 - 1);
    let wpr_start = heap_off - (1 << 20);
    let nonwpr_start = wpr_start - (1 << 20);
    if nonwpr_start >= usable_fb
        || wpr_end > usable_fb
        || wpr_start >= wpr_end
        || gsp >= boot || boot >= frts || frts >= wpr_end
        || nonwpr_start < usable_fb - (256 << 20)
    {
        info!("layout rejected: reservation calculation");
        return Err(TxError::ERange);
    }
    info!("layout: fb={usable_fb:#x} vga={vga_base:#x} frts={frts:#x} wpr=[{wpr_start:#x},{wpr_end:#x}]");
    Ok(LayoutOut {
        fb_size: usable_fb,
        vga_base,
        wpr_end,
        frts_offset: frts,
        boot_offset: boot,
        gsp_offset: gsp,
        heap_offset: heap_off,
        wpr_start,
        nonwpr_start,
    })
}

/// FWSEC ignition phase (unlock_610.c 921-1026, 1:1): read PROM, parse,
/// patch FRTS for our layout, install the fuse-version signature slot,
/// fire on the GSP, verify scratch + WPR2 against the layout.
pub fn fwsec_ignite(gpu: &TxGpu, layout: &LayoutOut) -> Result<(), TxError> {
    const GSP_BASE: u32 = 0x0011_0000;
    const GSP_FBIF: u32 = 0x0011_0600;
    const GSP_FALCON2: u32 = 0x0011_1000;
    info!("FWSEC: reading PROM at {:#x}", 0x300000);

    let mapped = 1024 * 1024usize;
    let mut rom_words = alloc::vec![0u32; mapped / 4];
    let st = gpu.pci.bar_read_u32_buf(gpu.bar, 0x300000, rom_words.as_mut());
    if st != uefi::Status::SUCCESS {
        info!("FWSEC-TRACE: PROM bulk read failed {:?}", st);
        return Err(TxError::ENodev);
    }
    let rom: &[u8] = unsafe { core::slice::from_raw_parts(rom_words.as_ptr() as *const u8, mapped) };
    info!("FWSEC: PROM bytes={mapped} signature={:#06x}", le16(rom, 0));

    let fw = match prom_find_fwsec(&rom, false) {
        Some(f) => f,
        None => { info!("FWSEC-TRACE: PROM parse found no FWSEC_PROD V3"); return Err(TxError::ENodev); }
    };
    if fw.engine_mask != 0x400 {
        info!("FWSEC-TRACE: unsupported engine mask={:#x}", fw.engine_mask);
        return Err(TxError::ENodev);
    }
    info!(
        "FWSEC: imem={:#x}/{:#x} dmem={:#x}/{:#x} pkc={:#x} sigs={:#x} ucode={}",
        fw.imem_pa, fw.imem_size, fw.dmem_pa, fw.dmem_size, fw.pkc_off, fw.sig_versions, fw.ucode_id
    );
    info!(
        "FWSEC: stored={:#x} dma_image={:#x} (256-rounded, tail zeroed) data_off={:#x}",
        fw.stored_size, fw.image_size, fw.data_off
    );

    info!("FWSEC-TRACE: alloc {} bytes for FWSEC image", fw.image_size);
    let mut dma = match gpu.pci.dma_alloc(fw.image_size as usize) {
        Ok(d) => d,
        Err(st) => { info!("FWSEC-TRACE: dma_alloc FAILED {:?}", st); return Err(TxError::ENodev); }
    };
    // The descriptor's payload window must fit the 1 MiB PROM staging buffer:
    // prom_find_ucode checks the *descriptor* bounds, not the image it points
    // at, so an over-long `stored` would read past the buffer and DMA whatever
    // follows it into an engine that then executes it.
    if (fw.image_off as usize)
        .checked_add(fw.stored_size as usize)
        .map_or(true, |end| end > rom.len())
    {
        info!("FWSEC-TRACE: image window {:#x}+{:#x} outside the staged ROM ({:#x})", fw.image_off, fw.stored_size, rom.len());
        return Err(TxError::ENodev);
    }
    unsafe {
        // Zero the whole block first (rounded tail must be 0), then copy
        // the exact stored-size payload.
        core::ptr::write_bytes(dma.host, 0, fw.image_size as usize);
        core::ptr::copy_nonoverlapping(
            rom.as_ptr().add(fw.image_off as usize),
            dma.host,
            fw.stored_size as usize,
        );
    }
    {
        let image = unsafe { core::slice::from_raw_parts_mut(dma.host, fw.image_size as usize) };
        fwsec_patch_frts(image, &fw, layout.frts_offset)?;
    }

    let fuse_raw = gpu.read32(0x0082_41c0 + (fw.ucode_id - 1) * 4)?;
    let version = fuse_version(fuse_raw);
    info!("FWSEC fuse: raw={fuse_raw:#x} version={version}");
    let sig_at = fwsec_sig_offset(fw.sig_versions, version, 384)?;
    let d0 = fw.data_off as usize;
    unsafe {
        core::ptr::copy_nonoverlapping(
            rom.as_ptr().add(fw.sig_off as usize + sig_at as usize),
            dma.host.add(d0 + fw.pkc_off as usize),
            384,
        );
    }
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

    // Portal placement instead of falcon_hs DMA: the GSP falcon DMA engine
    // accepts 64 commands (2x32 slots) and never consumes them even with
    // FBIF unblocked + DMACTL=0 (measured 2026-10-26). The register portal
    // is proven on both engines; everything after placement (BROM PKC
    // params, BOOTVEC, start, HALT) matches the kernel HS sequence.
    const GSP_SEC: u32 = 1 << 28;
    crate::tx::falcon_reset(gpu, GSP_BASE, GSP_FBIF, GSP_FALCON2, true)?;
    crate::tx::falcon_dma_unblock(gpu, GSP_BASE, GSP_FBIF)?;
    crate::tx::falcon_authority_probe(gpu, GSP_BASE, "GSP");
    let image = unsafe { core::slice::from_raw_parts(dma.host, fw.image_size as usize) };
    let imem = &image[..fw.imem_size as usize];
    {
        // Kernel FWSEC DMA commands carry SEC=1 for the whole IMEM.
        let blocks = imem.len().div_ceil(256);
        let mut tag = fw.imem_va >> 8;
        for blk in 0..blocks {
            let off = blk * 256;
            gpu.write32(GSP_BASE + 0x180, ((fw.imem_pa + off as u32) & 0x00ff_fffc) | GSP_SEC | 1 << 24);
            gpu.write32(GSP_BASE + 0x188, tag & 0xffff);
            tag += 1;
            for w in 0..64 {
                let b = off + w * 4;
                let v = if b + 4 <= imem.len() {
                    u32::from_le_bytes([imem[b], imem[b + 1], imem[b + 2], imem[b + 3]])
                } else {
                    let mut tmp = [0u8; 4];
                    for (k, t) in tmp.iter_mut().enumerate() {
                        if b + k < imem.len() {
                            *t = imem[b + k];
                        }
                    }
                    u32::from_le_bytes(tmp)
                };
                gpu.write32(GSP_BASE + 0x184, v);
            }
        }
        info!("FWSEC portal: IMEM {} blocks ({} bytes, secure)", blocks, imem.len());
    }
    let dmem = &image[fw.data_off as usize..(fw.data_off + fw.dmem_size) as usize];
    crate::tx_portal::portal_load_dmem(gpu, GSP_BASE, fw.dmem_pa, dmem);
    // BROM PKC params (kernel _kgspSetupPkcParams) + BOOTVEC + start.
    gpu.write32(GSP_FALCON2 + 0x210, fw.pkc_off);
    gpu.write32(GSP_FALCON2 + 0x19c, fw.engine_mask);
    gpu.write32(GSP_FALCON2 + 0x198, fw.ucode_id & 0xff);
    gpu.write32(GSP_FALCON2 + 0x180, 1); // MOD_SEL = ALGO_RSA3K
    gpu.write32(GSP_BASE + 0x104, fw.imem_va);
    // kflcnStartCpu semantics: ALIAS_EN set -> start via the alias register.
    let cpuctl_pre = gpu.pci.bar_read_u32(gpu.bar, (GSP_BASE + 0x100) as u64);
    if cpuctl_pre & (1 << 6) != 0 {
        gpu.write32(GSP_BASE + 0x130, 1 << 1);
    } else {
        gpu.write32(GSP_BASE + 0x100, 1 << 1);
    }
    let r = gpu.poll_us(GSP_BASE + 0x100, 1 << 4, 1 << 4, 5_000_000, true);
    let cpuctl = gpu.pci.bar_read_u32(gpu.bar, (GSP_BASE + 0x100) as u64);
    let exci = gpu.pci.bar_read_u32(gpu.bar, (GSP_BASE + 0x0d0) as u64);
    let mbox0 = gpu.pci.bar_read_u32(gpu.bar, (GSP_BASE + 0x40) as u64);
    info!("FWSEC portal forensic: CPUCTL={cpuctl:#010x} MAILBOX0={mbox0:#010x} EXCI={exci:#010x}");

    let scratch = gpu.read32(0x1400 + 0x0e * 4)?;
    info!("FWSEC result: scratch={scratch:#x} expected_hi16=0");
    let wpr_lo = gpu.pci.bar_read_u32(gpu.bar, 0x001f_a824);
    let wpr_hi = gpu.pci.bar_read_u32(gpu.bar, 0x001f_a828);
    info!(
        "FWSEC WPR2: low={wpr_lo:#x} high={wpr_hi:#x} expected_page={:#x} end={:#x}",
        layout.frts_offset >> 12, layout.wpr_end
    );
    r?;
    if scratch >> 16 != 0 {
        return Err(TxError::EIO);
    }
    if (wpr_hi >> 4) == 0
        || (wpr_hi >> 4) < (wpr_lo >> 4)
        || ((wpr_hi >> 4) as u64) << 12 >= layout.wpr_end
        || (wpr_lo >> 4) as u64 != layout.frts_offset >> 12
    {
        return Err(TxError::EIO);
    }
    let _ = dma.release(gpu.pci);
    info!("FWSEC: WPR2 verified");
    Ok(())
}

/// allow unused-import warnings for pre-wired items
const _: () = {
    let _ = hs_prevalidate as fn(u64, u32, u32, u32, u32, u32, u32, u32) -> Result<u64, TxError>;
    let _: Option<&PciIo> = None;
};

/// Turing FWSEC ignition (BOOT_WITH_LOADER). Port of cmp50hx-unlock
/// efi-unlock fwsec_boot_gsp_50hx (v66-v70 lineage): proven on TU106 40HX
/// (v66 real-machine measurements) and TU102 50HX (2026-09-10 live A/B).
/// Differences from the GA10x/Ada HS path above:
/// - FWSEC is a WITH_LOADER-type ucode (kernel_gsp_fwsec.c:741). Host
///   writes/DMA into SECURE IMEM are scrubbed - disproven on card across
///   v66-v69. A generic falcon BL (768B, g_bindata_ksec2GetBinArchiveBlUcode
///   _TU102, works for SEC2 and GSP) runs at the top of GSP IMEM and DMAs
///   the secure code from the host buffer inside secure context.
/// - The V2 image is NS+SEC split: NS [0..sec_base) -> IMEM 0 unsecured,
///   SEC [sec_base..+sec_size) -> IMEM sec_base secured (driver
///   s_imemCopyTo_TU102 semantics; tags = address>>8).
/// - No BROM PKC params (WITH_LOADER has no PARAADDR/ENGIDMASK semantics);
///   BOOTVEC = the BL start tag << 8 (blStartTag 0xfd -> 0xfd00).
/// - FBIF TRANSCFG0/4 = 0x15 (COHERENT_SYSMEM | PHYSICAL). v1.52 lesson:
///   bare 0x5 leaves MEM_TYPE virtual and the BL DMA never completes.
/// - Signature rides inside the blob data segment (prod bytes); the
///   explicit per-fuse sig placement of the V3 path does not apply.
pub fn fwsec_ignite_tu10x(gpu: &TxGpu, layout: &LayoutOut, bl: &[u8]) -> Result<(), TxError> {
    const GSP: u32 = 0x0011_0000;
    const FALCON2: u32 = 0x0011_1000;
    const BL_START_TAG: u32 = 0xfd;
    const BL_CODE_SIZE: usize = 0x200; // blImgHeader.blCodeSize (768B blob = 0x200 code + 0x100 data)
    if bl.len() < BL_CODE_SIZE {
        info!("FWSEC-TU10X: sec2 BL blob too small ({} bytes)", bl.len());
        return Err(TxError::EInval);
    }
    let bsi = gpu.pci.bar_read_u32(gpu.bar, 0x0011_80fc);
    info!("FWSEC-TU10X: BSI_SCRATCH15={bsi:#010x} handoff={} (DONE=3)", bsi >> 29 & 7);

    info!("FWSEC-TU10X: reading PROM at {:#x}", 0x300000);
    let mapped = 1024 * 1024usize;
    let mut rom_words = alloc::vec![0u32; mapped / 4];
    let st = gpu.pci.bar_read_u32_buf(gpu.bar, 0x300000, rom_words.as_mut());
    if st != uefi::Status::SUCCESS {
        info!("FWSEC-TU10X: PROM bulk read failed {:?}", st);
        return Err(TxError::ENodev);
    }
    let rom: &[u8] = unsafe { core::slice::from_raw_parts(rom_words.as_ptr() as *const u8, mapped) };
    let fw = match prom_find_fwsec(&rom, false) {
        Some(f) => f,
        None => { info!("FWSEC-TU10X: PROM parse found no FWSEC_PROD (V2/V3)"); return Err(TxError::ENodev); }
    };
    if fw.engine_mask != 0x400 {
        info!("FWSEC-TU10X: unsupported engine mask={:#x}", fw.engine_mask);
        return Err(TxError::ENodev);
    }
    if fw.imem_sec_base == 0 || fw.imem_sec_size == 0
        || fw.imem_sec_base + fw.imem_sec_size != fw.imem_size {
        info!(
            "FWSEC-TU10X: unexpected V2 split (sec {:#x}+{:#x} != imem {:#x}) - NS+SEC load cannot proceed",
            fw.imem_sec_base, fw.imem_sec_size, fw.imem_size
        );
        return Err(TxError::EInval);
    }
    info!(
        "FWSEC-TU10X: imem ns 0..{:#x} + sec {:#x}+{:#x} dmem {:#x} @img {:#x} iface@{:#x} stored={:#x}",
        fw.imem_sec_base, fw.imem_sec_base, fw.imem_sec_size,
        fw.dmem_size, fw.data_off, fw.interface_off, fw.stored_size
    );

    if (fw.image_off as usize)
        .checked_add(fw.stored_size as usize)
        .map_or(true, |end| end > rom.len())
    {
        info!("FWSEC-TU10X: image window {:#x}+{:#x} outside the staged ROM ({:#x})", fw.image_off, fw.stored_size, rom.len());
        return Err(TxError::ENodev);
    }
    // v2.1.8: the FWSEC staging band is LOW by default (issue #74 rounds
    // 15-28: >=4GiB staging is unreachable on TB platforms - the BL's FBIF
    // DMA never returns, showing as "WPR2 never came up" with zero residue -
    // while sub-4GB FWSEC staging is field-proven everywhere it was tried:
    // #16 (v2.1.1 era), #12/#14 (v0.2.0 era), #74's all-low rounds, two
    // independent implementations on the same card). The cmp50hx v2.63
    // "<4GB locked" note concerns LOCKED-STATE BOOTER reads, never the FWSEC
    // BL. Meta/signature/radix keep the global >4GB gate.
    // \tx-fwsec-high-staging.txt restores the old high band for ablation.
    let mut dma = if !crate::tx::probe_file("\\tx-fwsec-high-staging.txt") && crate::pci::high_staging() {
        let restore = crate::pci::high_staging();
        crate::pci::set_high_staging(false);
        let d = gpu.pci.dma_alloc(fw.image_size as usize);
        crate::pci::set_high_staging(restore);
        let d = d.map_err(|_| TxError::ENodev)?;
        info!(
            "FWSEC-TU10X: staging allocated in the LOW band by default (dev={:#x}) - >=4GiB is unreachable on some platforms (issue #74); \\tx-fwsec-high-staging.txt restores high",
            d.dev
        );
        d
    } else {
        gpu.pci.dma_alloc(fw.image_size as usize).map_err(|_| TxError::ENodev)?
    };
    unsafe {
        core::ptr::write_bytes(dma.host, 0, fw.image_size as usize);
        core::ptr::copy_nonoverlapping(
            rom.as_ptr().add(fw.image_off as usize),
            dma.host,
            fw.stored_size as usize,
        );
    }
    {
        let image = unsafe { core::slice::from_raw_parts_mut(dma.host, fw.image_size as usize) };
        // V2 mapper walk: iface -> DMEMMAPPER(id4) -> cmd_in; writes
        // readVbiosDesc{1,24,0,0,0,2} + frtsRegionDesc{1,20,frts>>12,0x100,2}
        // and mapper.init_cmd = 0x15 (FRTS) - same cmd shape the 50HX port
        // hand-writes at its 0x360/0x3b0 constants.
        fwsec_patch_frts(image, &fw, layout.frts_offset)?;
    }
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

    // 1. GSP reset, Turing order (50HX): IRQMSET=0 -> reset/scrub/BCR=Falcon
    //    (falcon_reset; TU106 GSP is RISC-V so the BCR forces Falcon) ->
    //    FALCON_RM = chipId0 (PMC_BOOT_0: TU102 0x162000A1 / TU106 0x166000A1).
    gpu.write32(GSP + 0x080, 0); // IRQMSET=0 (50HX E1-trace step)
    crate::tx::falcon_reset(gpu, GSP, GSP + 0x600, FALCON2, false)?;
    let chip = gpu.read32(0x0)?; // PMC_BOOT_0
    gpu.write32(GSP + 0x084, chip); // FALCON_RM
    let rm_rb = gpu.pci.bar_read_u32(gpu.bar, (GSP + 0x084) as u64);
    let cpuctl = gpu.pci.bar_read_u32(gpu.bar, (GSP + 0x100) as u64);
    info!("FWSEC-TU10X: GSP reset done: rm={rm_rb:#010x} cpuctl={cpuctl:#010x} chip={chip:#010x}");
    if cpuctl & 0xBADF_0000 != 0 {
        info!("FWSEC-TU10X: GSP locked (BADF) after reset");
        return Err(TxError::EIO);
    }

    // 2. FBIF for the BL host-DMA aperture (ctxDma=4 baked into the BL).
    let ctl = gpu.read32(GSP + 0x624)?;
    gpu.write32(GSP + 0x624, ctl | 1 << 7); // FBIF_CTL DISABLE_CTX_REQ
    gpu.write32(GSP + 0x10C, 0); // DMACTL
    gpu.write32(GSP + 0x600, 0x15); // TRANSCFG0 = COHERENT_SYSMEM|PHYSICAL
    gpu.write32(GSP + 0x610, 0x15); // TRANSCFG4 (the BL's ctxDma index)
    let t4 = gpu.read32(GSP + 0x610)?;
    info!("FWSEC-TU10X: FBIF trans4={t4:#010x} (want 0x15) dmactl cleared");

    // 3. BL DESC (RM_FLCN_BL_DMEM_DESC, 0x54 bytes) -> DMEM 0.
    let mut desc = [0u8; 0x54];
    let put32 = |d: &mut [u8], off: usize, v: u32| d[off..off + 4].copy_from_slice(&v.to_le_bytes());
    let data_phys = dma.dev + fw.data_off as u64;
    put32(&mut desc, 0x20, 4); // ctxDma (TRANSCFG index 4)
    put32(&mut desc, 0x24, dma.dev as u32); // codeDma lo
    put32(&mut desc, 0x28, (dma.dev >> 32) as u32); // codeDma hi
    put32(&mut desc, 0x2C, 0); // nsOff (imemNsPa)
    put32(&mut desc, 0x30, fw.imem_sec_base); // nsSize
    put32(&mut desc, 0x34, fw.imem_sec_base); // secOff (imemSecPa)
    put32(&mut desc, 0x38, fw.imem_sec_size); // secSize
    put32(&mut desc, 0x3C, 0); // codeEntryPoint (IMEM 0)
    put32(&mut desc, 0x40, data_phys as u32); // dataDma lo
    put32(&mut desc, 0x44, (data_phys >> 32) as u32); // dataDma hi
    put32(&mut desc, 0x48, fw.dmem_size); // dataSize
    crate::tx_portal::portal_load_dmem(gpu, GSP, 0, &desc);

    // 4. BL code -> IMEM top (s_setupLoader: dst block = IMEM blocks - code
    // blocks), tags from blStartTag 0xfd; the BL starts at its LINK address
    // 0xfd00 regardless of the write destination.
    let imem_blocks = gpu.read32(GSP + 0x108)? & 0x1ff;
    if (imem_blocks as usize) * 256 < BL_CODE_SIZE {
        info!("FWSEC-TU10X: IMEM too small for BL ({imem_blocks} blocks)");
        return Err(TxError::EInval);
    }
    let bl_dst = ((imem_blocks - (BL_CODE_SIZE / 256) as u32) << 8) as u32;
    let bl_vec = BL_START_TAG << 8;
    info!(
        "FWSEC-TU10X: BL at IMEM dst={bl_dst:#x} (hwcfg {imem_blocks} blocks), BOOTVEC={bl_vec:#x}{}",
        if bl_dst == bl_vec { "" } else { "  <== dst != 0xfd00 (50HX saw 0xfd00 on TU102/TU106)" }
    );
    crate::tx_portal::portal_load_imem(gpu, GSP, bl_dst, bl_vec, &bl[..BL_CODE_SIZE], false);

    // 5. BOOTVEC + STARTCPU (alias path when the card exposes it, same
    // probe as the HS fire above), then poll WPR2 - FWSEC-with-loader is
    // verdicted by the FRTS result, not by an early HALT.
    gpu.write32(GSP + 0x104, bl_vec);
    let cpuctl_pre = gpu.pci.bar_read_u32(gpu.bar, (GSP + 0x100) as u64);
    if cpuctl_pre & (1 << 6) != 0 {
        gpu.write32(GSP + 0x130, 1 << 1);
    } else {
        gpu.write32(GSP + 0x100, 1 << 1);
    }
    info!("FWSEC-TU10X: STARTCPU, polling WPR2 up (5s budget)...");
    let mut up = false;
    let (mut wpr_lo, mut wpr_hi) = (0u32, 0u32);
    for i in 0..5000u32 {
        wpr_lo = gpu.pci.bar_read_u32(gpu.bar, 0x001f_a824);
        wpr_hi = gpu.pci.bar_read_u32(gpu.bar, 0x001f_a828);
        if (wpr_hi >> 4) != 0 && ((wpr_lo >> 4) as u64 == layout.frts_offset >> 12) {
            info!("FWSEC-TU10X: *** WPR2 UP lo={wpr_lo:#x} hi={wpr_hi:#x} after {}ms ***", i);
            up = true;
            break;
        }
        uefi::boot::stall(core::time::Duration::from_millis(1));
    }
    if !up {
        let cpuctl = gpu.pci.bar_read_u32(gpu.bar, (GSP + 0x100) as u64);
        let exci = gpu.pci.bar_read_u32(gpu.bar, (GSP + 0x0d0) as u64);
        let mbox0 = gpu.pci.bar_read_u32(gpu.bar, (GSP + 0x40) as u64);
        let scratch = gpu.read32(0x1400 + 0x0e * 4).unwrap_or(0);
        info!(
            "FWSEC-TU10X: WPR2 never came up - lo={wpr_lo:#x} hi={wpr_hi:#x} CPUCTL={cpuctl:#010x} EXCI={exci:#010x} MAILBOX0={mbox0:#010x} scratch={scratch:#010x}"
        );
        let _ = dma.release(gpu.pci);
        return Err(TxError::EIO);
    }
    // Completion criteria (v2.1.8, field-proven by issue #62's R1 round and
    // promoted to the DEFAULT): exact WPR2 lo+hi, then GSP HALT (10s), then
    // the FWSEC result registers - the kernel shape
    // (kgspExecuteHsFalcon_TU102 + kgspExecuteFwsec_TU102) and every
    // field-proven community chain (nlzy, PR #49 lineage, travis guarded).
    // #62 measured WPR2-up at 3ms but HALT at 156ms: the old path fired the
    // booter ~150ms inside the FWSEC tail and lost that race deterministically
    // per machine. #57's R1/R2 rounds proved the wait harmless where the race
    // was not the disease (halt at 87ms, identical outcome).
    // \tx-no-gsp-halt-gate.txt restores the legacy WPR2-match-only behavior
    // for ablation. (The retired \tx-fwsec-gsp-halt.txt marker was this gate;
    // \tx-fwsec-hold-dma.txt was retired with the page-reuse axis it probed -
    // eliminated by the #14-vs-ProxVZ identical-layout evidence.)
    if crate::tx::probe_file("\\tx-no-gsp-halt-gate.txt") {
        info!("FWSEC-TU10X: \\tx-no-gsp-halt-gate.txt present - legacy completion (WPR2 match only, no HALT wait, no result check)");
        let _ = dma.release(gpu.pci);
        info!("FWSEC-TU10X: WPR2 verified (FRTS at {:#x}) [legacy gate]", layout.frts_offset);
        return Ok(());
    }
    let want_hi = ((layout.frts_offset + 0xe_0000) >> 8) as u32;
    if wpr_hi != want_hi {
        info!(
            "FWSEC-TU10X: GSP-HALT gate: WPR2 hi={wpr_hi:#x} != exact FRTS window end {want_hi:#x} (frts+0xe0000) - refusing (reference shape demands exact lo AND hi)"
        );
        let _ = dma.release(gpu.pci);
        return Err(TxError::EIO);
    }
    let cpuctl0 = gpu.pci.bar_read_u32(gpu.bar, (GSP + 0x100) as u64);
    let mut halted = cpuctl0 & 0x10 != 0;
    let mut waited = 0u32;
    while !halted && waited < 10_000 {
        uefi::boot::stall(core::time::Duration::from_millis(1));
        waited += 1;
        halted = gpu.pci.bar_read_u32(gpu.bar, (GSP + 0x100) as u64) & 0x10 != 0;
    }
    // Kernel parity continues past the halt: kgspExecuteFwsec_TU102
    // (open-gpu-kernel-modules kernel_gsp_frts_tu102.c:486-530) reads the
    // FWSEC RESULT registers after kflcnWaitForHalt - mailbox0/1 plus
    // scratch0e[31:16] FRTS_ERR_CODE (NV_VBIOS_FWSECLIC_SCRATCH_INDEX_0E).
    let mb0 = gpu.pci.bar_read_u32(gpu.bar, (GSP + 0x40) as u64);
    let mb1 = gpu.pci.bar_read_u32(gpu.bar, (GSP + 0x44) as u64);
    let scratch = gpu.read32(0x1400 + 0x0e * 4).unwrap_or(0);
    info!(
        "FWSEC-TU10X: GSP-HALT gate: CPUCTL {cpuctl0:#010x} -> halted={halted} after {waited}ms (exact WPR2 lo={wpr_lo:#x} hi={wpr_hi:#x})"
    );
    info!(
        "FWSEC-TU10X: FWSEC result (kernel shape): MAILBOX0={mb0:#010x} MAILBOX1={mb1:#010x} scratch0e={scratch:#010x} frts_err={:#06x}",
        scratch >> 16
    );
    if !halted {
        info!("FWSEC-TU10X: GSP did not HALT within 10s - refusing to fire the booter on a running FWSEC (reference shape)");
        let _ = dma.release(gpu.pci);
        return Err(TxError::EIO);
    }
    if scratch >> 16 != 0 {
        info!("FWSEC-TU10X: scratch0e FRTS_ERR_CODE={:#06x} nonzero - FWSEC itself reported an error (kernel: NV_ERR_GENERIC); refusing", scratch >> 16);
        let _ = dma.release(gpu.pci);
        return Err(TxError::EIO);
    }
    let _ = dma.release(gpu.pci);
    info!("FWSEC-TU10X: WPR2 verified (FRTS at {:#x}) [GSP-HALT gate]", layout.frts_offset);
    Ok(())
}

/// Kernel kgspExecuteScrubberIfNeeded_AD102 port: when the BSI scrubber
/// handoff is not DONE, the scrubber ucode must run on SEC2 first - until
/// then the GSP/SEC2 domains stay in their post-GFW gated state (SEC2_CPUCTL
/// badf5620; DMA fetches die behind REQUIRE_CTX).
///
/// The ucode is the KERNEL-EXACT artifact (ksec2GetBinArchiveSecurescrubUcode
/// BINDATA → BTL1, `\\scrubber_ad10x.bin`): image 7424B, ucode 0xd, 2×RSA-384.
/// The VBIOS PROM app 0x49 we tried first is a DIFFERENT ucode (22528B) that
/// is not what advances the handoff (measured 2026-10-26).
pub fn scrubber_if_needed(gpu: &TxGpu) -> Result<(), TxError> {
    const SEC2_BASE: u32 = 0x0084_0000;
    const SEC2_FBIF: u32 = 0x0084_0600;
    const SEC2_FALCON2: u32 = 0x0084_1000;

    let bsi = gpu.pci.bar_read_u32(gpu.bar, 0x0011_80fc);
    let handoff = bsi >> 29 & 7;
    info!("SCRUB: BSI_SCRATCH15={bsi:#010x} handoff={handoff} (DONE=3)");
    if handoff >= 3 {
        info!("SCRUB: handoff already DONE - skipping");
        return Ok(());
    }

    let bundle = match crate::tx::read_asset("\\scrubber_ad10x.bin") {
        Ok(Some(b)) => b,
        Ok(None) => {
            info!("SCRUB: \\scrubber_ad10x.bin missing on boot volume");
            return Err(TxError::ENodev);
        }
        Err(()) => {
            info!("SCRUB: \\scrubber_ad10x.bin is present but unreadable - refusing to fire a partial image");
            return Err(TxError::ENodev);
        }
    };
    let (hdr, image, sigs) = crate::tx_booter::load_booter(&bundle, "scrub")?;
    info!(
        "SCRUB: bundle ok - code {:#x}+{:#x}@va{:#x} data {:#x}+{:#x} patch {:#x} ucode {:#x}",
        hdr.app_code_off, hdr.app_code_size, hdr.app_code_off, hdr.data_off, hdr.data_size, hdr.patch_loc, hdr.ucode_id
    );

    // Signature slot: SEC2 per-ucode fuse (kernel ksec2ReadUcodeFuseVersion:
    // FUSE_OPT_FPF_SEC2_UCODE1_VERSION + 4*(ucodeId-1)), then stock patch.
    let fuse_raw = gpu.read32(0x0082_4140 + (hdr.ucode_id - 1) * 4)?;
    let version = fuse_version(fuse_raw);
    info!("SCRUB: fuse_raw={fuse_raw:#010x} version={version} sigs={}", hdr.num_sigs);
    let mut image = alloc::vec::Vec::from(image);
    crate::tx_booter::booter_patch_stock(&mut image, &hdr, sigs, version)?;

    // HS geometry (kernel BootFromHs mapping): IMEM lands at physical 0
    // with virtual tags from imemVa; DMEM at 0; BROM sig lives at
    // patch_loc - dataOffset inside DMEM (0 here).
    let sig_dmem = hdr.patch_loc - hdr.data_off;

    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

    // Reset + open the engine's DMA path (the scrubber's own FB scrub
    // translates through this FBIF), then portal-place the image - content
    // placement is equivalent to the kernel's DMA, the BROM validates from
    // IMEM/DMEM + the armed BROM registers.
    crate::tx::falcon_reset(gpu, SEC2_BASE, SEC2_FBIF, SEC2_FALCON2, true)?;
    crate::tx::falcon_dma_unblock(gpu, SEC2_BASE, SEC2_FBIF)?;
    // Kernel BootFromHs placement: secure code only, physical dest 0,
    // virtual tags from imemVa (the NS part of the image is not loaded).
    crate::tx_portal::portal_load_imem(
        gpu, SEC2_BASE, 0, hdr.app_code_off,
        &image[hdr.app_code_off as usize..(hdr.app_code_off + hdr.app_code_size) as usize], true,
    );
    crate::tx_portal::portal_load_dmem(
        gpu, SEC2_BASE, 0,
        &image[hdr.data_off as usize..(hdr.data_off + hdr.data_size) as usize],
    );

    // BROM params (kernel _kgspSetupPkcParams via unlock_610.c offsets).
    gpu.write32(SEC2_FALCON2 + 0x210, sig_dmem);
    gpu.write32(SEC2_FALCON2 + 0x19c, hdr.engine_id);
    gpu.write32(SEC2_FALCON2 + 0x198, hdr.ucode_id & 0xff);
    gpu.write32(SEC2_FALCON2 + 0x180, 1); // MOD_SEL = ALGO_RSA3K
    gpu.write32(SEC2_BASE + 0x104, hdr.app_code_off); // BOOTVEC = imemVa
    let cpuctl = gpu.read32(SEC2_BASE + 0x100)?;
    info!("SCRUB start: cpuctl={cpuctl:#x}");
    if cpuctl & (1 << 6) != 0 {
        gpu.write32(SEC2_BASE + 0x130, 1 << 1);
    } else {
        gpu.write32(SEC2_BASE + 0x100, 1 << 1);
    }
    let r = gpu.poll_us(SEC2_BASE + 0x100, 1 << 4, 1 << 4, 5_000_000, true);

    // Post-start forensics: HALT alone is not proof (a failed launch also
    // leaves the core halted) - read-back is the verdict.
    let cpuctl_now = gpu.pci.bar_read_u32(gpu.bar, (SEC2_BASE + 0x100) as u64);
    let exci = gpu.pci.bar_read_u32(gpu.bar, (SEC2_BASE + 0x0d0) as u64);
    let tracepc = gpu.pci.bar_read_u32(gpu.bar, (SEC2_BASE + 0x14c) as u64);
    let imemc = gpu.pci.bar_read_u32(gpu.bar, (SEC2_BASE + 0x180) as u64);
    let dmectl2 = gpu.pci.bar_read_u32(gpu.bar, (SEC2_BASE + 0x10c) as u64);
    let mbox0 = gpu.pci.bar_read_u32(gpu.bar, (SEC2_BASE + 0x40) as u64);
    info!(
        "SCRUB forensic: CPUCTL={cpuctl_now:#010x} MAILBOX0={mbox0:#010x} EXCI={exci:#010x} TRACEPC={tracepc:#010x} IMEMC0={imemc:#010x} DMACTL={dmectl2:#010x}"
    );
    // SEC2 DMEM dump via portal read (interface block + DMEM top window).
    let io = sig_dmem;
    let mut dump = alloc::vec![0u32; 32];
    crate::tx_portal::portal_read_dmem(gpu, SEC2_BASE, io, &mut dump);
    let nonzero: usize = dump.iter().filter(|&&w| w != 0).count();
    info!("SCRUB forensic: SEC2 DMEM@{io:#x} nonzero words = {nonzero}/32; first 8:");
    for i in 0..8 {
        info!("  DMEM[{i}] = {:#010x}", dump[i]);
    }
    let top = hdr.data_size & !0x7f;
    let mut stack = alloc::vec![0u32; 16];
    crate::tx_portal::portal_read_dmem(gpu, SEC2_BASE, top.saturating_sub(64), &mut stack);
    info!("SCRUB forensic: SEC2 DMEM top window @{:#x}: nonzero {}/16", top.saturating_sub(64), stack.iter().filter(|&&w| w != 0).count());
    for (i, w) in stack.iter().enumerate() {
        if *w != 0 {
            info!("  TOP[{i}] = {w:#010x}");
        }
    }

    // Completion: BSI handoff advances to DONE (poll generously).
    let mut handoff_now = 0;
    for _ in 0..50 {
        let b = gpu.pci.bar_read_u32(gpu.bar, 0x0011_80fc);
        handoff_now = b >> 29 & 7;
        if handoff_now >= 3 {
            break;
        }
        boot::stall(core::time::Duration::from_millis(100));
    }
    r?;
    if handoff_now < 3 {
        info!("SCRUB: SEC2 fired but handoff stuck at {handoff_now}");
        return Err(TxError::ETimeout { tolerated: false });
    }
    info!("SCRUB: handoff DONE - SEC2/GSP domains un-gated");
    Ok(())
}
