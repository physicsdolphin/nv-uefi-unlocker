//! Booter publication + execution + selectors + FLR — final transaction
//! segment, behavioral port of unlock_610.c (434-489, 749-800, 878-920,
//! 1113-1432) and prep_610.c (boot_desc parse, booter patch, wpr_meta_pack).
#![allow(dead_code)]


use log::info;

use crate::tx::{TxError, TxGpu};
use crate::tx_fwsec::LayoutOut;

pub const WPR_META_MAGIC: u64 = 0xdc3a_ae21_371a_60b3;
pub const WPR_META_SIZE: usize = 256;
pub const V67_SIZE: usize = 0xfa00;
pub const SS0_UNLOCK: u32 = 0x8888_8888;
pub const SS1_UNLOCK: u32 = 0x8;
pub const BTL1_MAGIC: u32 = 0x314C_5442;

fn le16(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
fn le32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}
fn wr32(d: &mut [u8], o: usize, v: u32) {
    d[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn wr64(d: &mut [u8], o: usize, v: u64) {
    d[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn rd64(d: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(d[o..o + 8].try_into().unwrap())
}

/// GSP boot descriptor (prep_610.c gpu_unlock_boot_desc_parse, extended
/// with the Turing RM_RISCV_UCODE_DESC **v4** variant, 76 bytes).
pub struct BootDesc {
    pub code_offset: u32,
    pub data_offset: u32,
    pub manifest_offset: u32,
    pub image_size: u32,
    /// desc d7 (appVersion) — Turing writes it to GSP FALCON_OS after the
    /// booter load (kgspBootstrap_TU102).
    pub app_version: u32,
}

pub fn boot_desc_parse(desc: &[u8], image_size: u32) -> Result<BootDesc, TxError> {
    let f = |i: usize| le32(desc, i * 4);
    if desc.len() == 76 {
        // v4 "eb riscv boot": bootloader d1/d2 + param d3/d4 are the only
        // populated ranges on TU102; monitor/data/code (d8/d10/d12) are
        // zero and flow into WprMeta verbatim (kgspPopulateWprMeta_TU102).
        if f(0) != 4 {
            return Err(TxError::EInval);
        }
        for &i in &[1usize, 3] {
            let (off, len) = (f(i), f(i + 1));
            if len == 0 || off > image_size || len > image_size - off {
                return Err(TxError::ERange);
            }
        }
        return Ok(BootDesc {
            manifest_offset: f(8),
            data_offset: f(10),
            code_offset: f(12),
            image_size,
            app_version: f(7),
        });
    }
    if desc.len() != 84
        || f(0) != 5
        || f(14) != 1
        || f(19) != image_size
        || f(20) != 0
    {
        return Err(TxError::EInval);
    }
    for &i in &[1usize, 3, 8, 10, 12] {
        let (off, len) = (f(i), f(i + 1));
        if len == 0 || off > image_size || len > image_size - off {
            return Err(TxError::ERange);
        }
    }
    for i in 5..=7 {
        if f(i) != 0 {
            return Err(TxError::EIO); // secure-partition boot not supported
        }
    }
    for i in 15..=18 {
        if f(i) != 0 {
            return Err(TxError::EIO);
        }
    }
    Ok(BootDesc {
        manifest_offset: f(8),
        data_offset: f(10),
        code_offset: f(12),
        image_size,
        app_version: f(7),
    })
}

/// Booter bundle header (BTL1), field names follow the kernel's
/// s_allocateUcodeFromBinArchive header struct (kernel_gsp_booter.c).
pub struct BooterHdr {
    /// fields[0]/[1]: non-secure IMEM offset/size inside the image.
    pub imem_ns_off: u32,
    pub imem_ns_size: u32,
    /// fields[2]/[3]: DMEM offset/size inside the image.
    pub data_off: u32,
    pub data_size: u32,
    /// fields[5]/[6]: secure IMEM offset/size inside the image.
    pub app_code_off: u32,
    pub app_code_size: u32,
    pub patch_loc: u32,
    pub engine_id: u32,
    pub ucode_id: u32,
    pub num_sigs: u32,
    /// Per-signature size (sig_total / num_sigs): 384 (RSA) on GA10x/Ada,
    /// 16 on TU102 BOOT_DIRECT.
    pub sig_bytes: u32,
}

/// Load and validate a BTL1 booter bundle. Geometry comes from the bundle
/// header itself and is cross-checked against the per-family pins measured
/// from pinned 610.43.03 (constitution: identity pinned, no silent drift).
pub fn load_booter<'a>(bundle: &'a [u8], family: &str) -> Result<(BooterHdr, &'a [u8], &'a [u8]), TxError> {
    if bundle.len() < 72 || le32(bundle, 0) != BTL1_MAGIC || le32(bundle, 4) != 1 {
        return Err(TxError::EInval);
    }
    let h = |i: usize| le32(bundle, 8 + i * 4);
    let image_size = le32(bundle, 8 + 14 * 4);
    let sig_total = le32(bundle, 8 + 15 * 4);
    let hdr = BooterHdr {
        imem_ns_off: h(0),
        imem_ns_size: h(1),
        data_off: h(2),
        data_size: h(3),
        app_code_off: h(5),
        app_code_size: h(6),
        patch_loc: h(9),
        ucode_id: h(10),
        engine_id: h(11),
        num_sigs: h(12),
        sig_bytes: h(13),
    };
    // Kernel checks: numSigs > 0, total % numSigs == 0, patch inside image.
    if hdr.num_sigs == 0
        || sig_total == 0
        || sig_total % hdr.num_sigs != 0
        || hdr.patch_loc as u64 + (sig_total / hdr.num_sigs) as u64 > image_size as u64
    {
        info!("Booter: signature table inconsistent");
        return Err(TxError::EInval);
    }
    // IMEM/DMEM transfer ranges inside the image, 256B Falcon blocks.
    let ranges = [
        (hdr.imem_ns_off, hdr.imem_ns_size),
        (hdr.data_off, hdr.data_size),
        (hdr.app_code_off, hdr.app_code_size),
    ];
    for (off, size) in ranges {
        if off as u64 + size as u64 > image_size as u64 || off % 256 != 0 || size % 256 != 0 {
            info!("Booter: transfer range outside image ({off:#x}+{size:#x})");
            return Err(TxError::EInval);
        }
    }
    // Per-family pins (measured 2026-10-25 from pinned 610.43.03 bundles):
    // image size, patch_loc, ucode_id, engine_id, num_sigs, per-sig bytes.
    // "scrub" = the kernel-exact SEC2 scrubber BINDATA (boot-type HS).
    let (img, patch, ucode, engine, nsigs, per_sig) = match family {
        "ga102" => (60416, 35344, 3, 1, 2, 384),
        "ad102" => (56832, 32272, 3, 1, 2, 384),
        "tu10x" => (59136, 34560, 0xd, 1, 1, 16),
        "scrub" => (7424, 0x1a00, 0xd, 1, 2, 384),
        _ => {
            info!("Booter: unknown family {family}");
            return Err(TxError::EInval);
        }
    };
    if image_size != img
        || hdr.patch_loc != patch
        || hdr.ucode_id != ucode
        || hdr.engine_id != engine
        || hdr.num_sigs != nsigs
        || sig_total != per_sig * nsigs
        || bundle.len() != 72 + image_size as usize + sig_total as usize
    {
        info!(
            "Booter: geometry mismatch for {family} (image {image_size} patch {:#x} ucode {:#x} sigs {} total {sig_total})",
            hdr.patch_loc, hdr.ucode_id, hdr.num_sigs
        );
        return Err(TxError::EInval);
    }
    Ok((
        hdr,
        &bundle[72..72 + image_size as usize],
        &bundle[72 + image_size as usize..],
    ))
}

/// prep_610.c gpu_unlock_booter_patch: install the fuse-selected stock
/// signature into the patch slot (num_sigs-1-fuse_ver indexing).
pub fn booter_patch_stock(image: &mut [u8], hdr: &BooterHdr, signatures: &[u8], fuse_ver: u32) -> Result<(), TxError> {
    // Turing BOOT_DIRECT bundles carry exactly ONE signature (SIG_PROD,
    // 16B AES): there is no fuse-version table to index, so pin index 0.
    // The num_sigs<2 refusal below is the GA10x/Ada two-slot layout - it
    // rejected the whole Turing bundle (fifth TU106 fire, 2026-10-27).
    if hdr.num_sigs == 1 {
        let sig_size = hdr.sig_bytes as usize;
        let at = hdr.patch_loc as usize;
        if at + sig_size > image.len() || sig_size > signatures.len() {
            return Err(TxError::EInval);
        }
        image[at..at + sig_size].copy_from_slice(&signatures[..sig_size]);
        return Ok(());
    }
    if hdr.num_sigs < 2 || fuse_ver >= hdr.num_sigs {
        return Err(TxError::ERange);
    }
    let sig_size = hdr.sig_bytes as usize;
    let index = (hdr.num_sigs - 1 - fuse_ver) as usize;
    let at = hdr.patch_loc as usize;
    if at + sig_size > image.len() || (index + 1) * sig_size > signatures.len() {
        return Err(TxError::EInval);
    }
    image[at..at + sig_size].copy_from_slice(&signatures[index * sig_size..(index + 1) * sig_size]);
    Ok(())
}

/// Replace the patch slot with the V67 payload image (the exploit vehicle).
pub fn booter_patch_v67(image: &mut [u8], hdr: &BooterHdr, v67: &[u8]) -> Result<(), TxError> {
    if v67.len() != V67_SIZE || hdr.patch_loc as usize + V67_SIZE > image.len() {
        return Err(TxError::EInval);
    }
    image[hdr.patch_loc as usize..hdr.patch_loc as usize + V67_SIZE].copy_from_slice(v67);
    Ok(())
}

/// Radix: three-level page tables over the GSP image, carved from ONE
/// contiguous DMA block (EFI-friendly variant of the per-page C allocator;
/// the tree only needs valid per-page device addresses).
pub struct RadixBlock {
    pub dma: crate::pci::DmaBuffer,
    pub level0: u64, // device address of L0
    pub pages: usize,
}

pub fn radix_build(gpu: &TxGpu, image: &[u8]) -> Result<RadixBlock, TxError> {
    const PAGE: usize = 4096;
    let leaves = image.len().div_ceil(PAGE);
    let l2 = leaves.div_ceil(512);
    let l1 = l2.div_ceil(512).max(1);
    let tables = 1 + l1 + l2;
    let total = tables + leaves;
    let dma = gpu.pci.dma_alloc(total * PAGE).map_err(|_| TxError::ENodev)?;
    // Safety: dma.host is valid for total*PAGE bytes.
    let mem = unsafe { core::slice::from_raw_parts_mut(dma.host, total * PAGE) };
    for b in mem.iter_mut() {
        *b = 0;
    }
    let dev = dma.dev;
    let page_addr = |i: usize| dev + (i as u64) * PAGE as u64;
    // leaves
    for i in 0..leaves {
        let off = i * PAGE;
        let n = (image.len() - off).min(PAGE);
        mem[tables * PAGE + off..tables * PAGE + off + n].copy_from_slice(&image[off..off + n]);
    }
    // tables
    let mut first = 0usize;
    for level in 0..3 {
        let counts = [1usize, l1, l2, leaves];
        let child = first + counts[level];
        for i in 0..counts[level] {
            let at = i * (PAGE / 8);
            let n = (counts[level + 1] - at).min(PAGE / 8);
            for e in 0..n {
                let addr = page_addr(child + at + e);
                let dst = (first + i) * PAGE + e * 8;
                wr64(mem, dst, addr);
            }
        }
        first = child;
    }
    info!("radix: leaves={leaves} tables={tables} level0={dev:#x}");
    Ok(RadixBlock { dma, level0: dev, pages: total })
}

/// prep_610.c gpu_unlock_wpr_meta_pack, 1:1.
#[allow(clippy::too_many_arguments)]
pub fn wpr_meta_pack(
    meta: &mut [u8],
    layout: &LayoutOut,
    family: &str,
    frts_size: u64,
    signature_dma: u64,
    signature_size: u64,
    radix_dma: u64,
    image_size: u64,
    boot_dma: u64,
    boot: &BootDesc,
) -> Result<(), TxError> {
    // FB size and VGA reservation come from the layout that produced the
    // offsets - they must not be re-read from the device here (the VGA
    // register is gated on a non-console dGPU and a strict re-read EIO'd).
    let fb_size = layout.fb_size;
    let vga_offset = layout.vga_base;
    if signature_size == 0 || image_size == 0 || boot.image_size == 0
        || vga_offset >= fb_size
        || radix_dma & 4095 != 0 || boot_dma & 4095 != 0
        || layout.frts_offset >= layout.wpr_end
        || frts_size != layout.wpr_end - layout.frts_offset
        || layout.wpr_end > vga_offset
        || image_size > layout.boot_offset - layout.gsp_offset
        || boot.image_size as u64 > layout.frts_offset - layout.boot_offset
    {
        return Err(TxError::EInval);
    }
    for b in meta.iter_mut() {
        *b = 0;
    }
    wr64(meta, 0, WPR_META_MAGIC);
    wr64(meta, 8, 1);
    wr64(meta, 16, radix_dma);
    wr64(meta, 24, image_size);
    wr64(meta, 32, boot_dma);
    wr64(meta, 40, boot.image_size as u64);
    wr64(meta, 48, boot.code_offset as u64);
    wr64(meta, 56, boot.data_offset as u64);
    wr64(meta, 64, boot.manifest_offset as u64);
    wr64(meta, 72, signature_dma);
    wr64(meta, 80, signature_size);
    // TU102 heap geometry is driver-EXACT (cmp50hx v2.63/v2.73, from the
    // live driver's own WPR meta dump on 40HX 8GB: gspFwHeap=0x1f7900000+
    // 0x6900000): the booter VALIDATES these fields and rejects the whole
    // meta on mismatch - cleanly, with no signature DMA and no visible
    // status on BOOT_DIRECT (eighth fire: payload silently no-op'd). The
    // size is the driver's per-FB constant and must NOT be recomputed from
    // the aligned offsets (v2.73: the driver keeps a 0xA0000 gap between
    // heap-end and gspFwOffset; recomputing inflated the heap and broke the
    // layout). With our gspFwOffset this formula reproduces the driver dump
    // bit-for-bit: heapOffset = 0x1f7900000.
    let (nonwpr_off, nonwpr_size, wpr_start, heap_off, heap_size, flags) = if family == "tu10x" {
        const MB: u64 = 1024 * 1024;
        // TU10x driver formula (RM 615.71.09 source, issue #57): heap =
        // 8 MiB (BASE_RM) + align_1MiB(96 KiB x GiB) + 96 MiB (CLIENT),
        // osCarveout = 0 on TU102/TU106. Reproduces every hardware-verified
        // point: 8G -> 105 MiB (40HX live dump), 22G -> 107 MiB (issue #17 C
        // reference), 24G -> 107 MiB (TITAN RTX, issue #57). Replaces the
        // per-size table; the old 10G/20G rows were GA102-sourced and never
        // applied to a Turing SKU.
        let gib = (fb_size + ((1 << 30) - 1)) >> 30; // driver: ALIGN_UP64(fbSize, 1GiB) >> 30
        let hs = (8 * MB) + ((((96 << 10) * gib) + MB - 1) & !(MB - 1)) + (96 * MB);
        let ho = (layout.gsp_offset - hs) & !(MB - 1);
        let ws = ho - MB;
        info!(
            "WprMeta tu10x driver-exact heap: off={ho:#x} size={hs:#x} wpr_start={ws:#x} flags=0x1 (deploy marker: driver formula vs 40HX dmesg 0x1f7900000+0x6900000)"
        );
        (ws - MB, MB, ws, ho, hs, 0x1u64) // CLOCK_BOOST flag from the live dump
    } else {
        (
            layout.nonwpr_start,
            layout.wpr_start - layout.nonwpr_start,
            layout.wpr_start,
            layout.heap_offset,
            layout.gsp_offset - layout.heap_offset,
            0,
        )
    };
    wr64(meta, 88, nonwpr_off); // gspFwRsvdStart
    wr64(meta, 96, nonwpr_off); // nonWprHeapOffset
    wr64(meta, 104, nonwpr_size);
    wr64(meta, 112, wpr_start);
    wr64(meta, 120, heap_off);
    wr64(meta, 128, heap_size);
    wr64(meta, 136, layout.gsp_offset);
    wr64(meta, 144, layout.boot_offset);
    wr64(meta, 152, layout.frts_offset);
    wr64(meta, 160, frts_size);
    wr64(meta, 168, layout.wpr_end);
    wr64(meta, 176, fb_size);
    wr64(meta, 184, vga_offset);
    wr64(meta, 192, fb_size - vga_offset);
    meta[241] = flags as u8; // flags (241): 0x1 = GSP_FW_FLAGS_CLOCK_BOOST
    Ok(())
}

/// Stock-signature restore: meta signature slot back to the stock DMA
/// (constitution 9.5 - the payload must never remain published).
pub fn signature_restore(meta: &mut [u8], stock_dma: u64, stock_size: u64) {
    wr64(meta, 72, stock_dma);
    wr64(meta, 80, stock_size);
}

/// Selector write with read-back (unlock_610.c write_selector, 1:1).
pub fn write_selector(gpu: &TxGpu, name: &str, offset: u32, unlock: u32) -> Result<(), TxError> {
    let value = gpu.read32(offset)?;
    info!("selector read: {name}={value:#x} expected_unlock={unlock:#x}");
    if value == unlock {
        return Ok(());
    }
    info!("selector write: {name}={unlock:#x}");
    gpu.write32(offset, unlock);
    let back = gpu.read32(offset)?;
    if back == unlock {
        Ok(())
    } else {
        info!("selector FAILED readback: {name}={back:#x}");
        Err(TxError::EIO)
    }
}

/// Post-start wait semantics (canonical falcon_hs `wait_plm` split).
/// `Halt`: the booter returns normally and writes its status - replay runs
/// and the Ada payload (marker + halt tail gadgets, M4-verified). `PlmFlip`:
/// the GA102 V67's tail is the fill self-loop (0x0000019c) - the booter
/// NEVER halts and writes no mailbox marker; the canonical verdict is the
/// FEAT PLM cold->open transition, and a HALT observed during the wait is
/// the FAILURE sign (a normal return means the signature DMA missed the
/// stack, i.e. the exploit did not run). Issue #6: the GA106M payload run
/// flipped the PLM but was judged ETimeout on the HALT poll - an inverted
/// verdict. Mirrors tx::falcon_hs's wait_plm branch (unlock_610.c 403-424).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BooterWait {
    Halt,
    PlmFlip,
}

/// SEC2 booter fire, GA10x/Ada HS variant. Portal placement (the falcon
/// DMA engine accepts 64 commands and never consumes them in this window,
/// measured on both engines), then the kernel BootFromHs tail verbatim:
/// IMEM at physical 0 with virtual tags from app_code_off (dest=imemPa=0,
/// memOff=imemVa), DMEM at 0 (the patched signature rides inside the DMEM
/// section at patch_loc - data_off), BROM PKC params, BOOTVEC=imemVa,
/// WprMeta address in the mailboxes, start, then `wait` (see BooterWait).
pub fn booter_fire(
    gpu: &TxGpu,
    hdr: &BooterHdr,
    image: &[u8],
    meta_addr: u64,
    local_fb: bool,
    wait: BooterWait,
) -> Result<(), TxError> {
    const SEC2: u32 = crate::tx::eng::SEC2;
    let sig_dmem = hdr.patch_loc - hdr.data_off;
    // Kernel rule: every HS execute is preceded by an engine reset. After a
    // successful scrubber the BSI machine re-locks SEC2's PRI window
    // (post-halt reads go badf5620) - the reset re-opens it.
    crate::tx::falcon_reset(gpu, SEC2, crate::tx::eng::SEC2_FBIF, crate::tx::eng::SEC2_FALCON2, true)?;
    crate::tx::falcon_dma_unblock(gpu, SEC2, crate::tx::eng::SEC2_FBIF)?;
    if local_fb {
        // VRAM staging: switch the falcon DMA aperture from coherent sysmem
        // to LOCAL_FB (kernel_gsp_falcon_ga102.c programs the same two
        // TRANSCFG fields). Every booter read then stays on-device - no PCIe
        // transaction, no platform DMA layer in the path.
        let fbif = crate::tx::eng::SEC2_FBIF;
        let mut trans = gpu.read32(fbif)?;
        trans &= !3; // TARGET[1:0]: 1 (coherent sysmem) -> 0 (local FB); MEM_TYPE stays PHYSICAL
        gpu.write32(fbif, trans);
        let rb = gpu.read32(fbif)?;
        info!("BOOTER-VRAM: TRANSCFG TARGET=LOCAL_FB, trans={rb:#010x}");
    }
    crate::tx::falcon_authority_probe(gpu, SEC2, "SEC2");
    // Kernel BootFromHs loads ONLY the secure code segment, and lands it at
    // physical IMEM 0 (dest=imemPa=0) with virtual tags from imemVa - the
    // NS/os-code part of the image is NOT loaded at all. Our old load (NS at
    // 0 + sec at 0x100) double-tagged va 0x100: the core fetched the NS junk
    // and insta-faulted (EXCI 0x0100, TRACEPC stuck at 0x100).
    let sec = &image[hdr.app_code_off as usize..(hdr.app_code_off + hdr.app_code_size) as usize];
    crate::tx_portal::portal_load_imem(gpu, SEC2, 0, hdr.app_code_off, sec, true);
    let dmem = &image[hdr.data_off as usize..(hdr.data_off + hdr.data_size) as usize];
    crate::tx_portal::portal_load_dmem(gpu, SEC2, 0, dmem);
    gpu.write32(crate::tx::eng::SEC2_FALCON2 + 0x210, sig_dmem);
    gpu.write32(crate::tx::eng::SEC2_FALCON2 + 0x19c, hdr.engine_id);
    gpu.write32(crate::tx::eng::SEC2_FALCON2 + 0x198, hdr.ucode_id & 0xff);
    gpu.write32(crate::tx::eng::SEC2_FALCON2 + 0x180, 1); // MOD_SEL = RSA3K
    // BROM param read-back: the START drop proved SEC2 has per-register
    // write gates - verify the BROM block actually latched.
    let rom_rb = (
        gpu.read32(crate::tx::eng::SEC2_FALCON2 + 0x210)?,
        gpu.read32(crate::tx::eng::SEC2_FALCON2 + 0x19c)?,
        gpu.read32(crate::tx::eng::SEC2_FALCON2 + 0x198)?,
        gpu.read32(crate::tx::eng::SEC2_FALCON2 + 0x180)?,
    );
    info!(
        "BOOTER brom readback: PARAADDR={:#x} ENGIDMASK={:#x} UCODE_ID={:#x} MOD_SEL={:#x} (want {sig_dmem:#x}/{:#x}/{:#x}/1)",
        rom_rb.0, rom_rb.1, rom_rb.2, rom_rb.3, hdr.engine_id, hdr.ucode_id & 0xff
    );
    gpu.write32(SEC2 + 0x104, hdr.app_code_off); // BOOTVEC = imemVa
    gpu.write32(SEC2 + 0x040, meta_addr as u32);
    gpu.write32(SEC2 + 0x044, (meta_addr >> 32) as u32);
    let cpuctl = gpu.read32(SEC2 + 0x100)?;
    // EXCI clear attempt: SEC2 latches EXCI=0x01000100 across engine resets
    // (GSP clears to 0). If that latch blocks start, clearing it unlocks.
    let exci_pre = gpu.pci.bar_read_u32(gpu.bar, (SEC2 + 0x0d0) as u64);
    gpu.write32(SEC2 + 0x0d0, 0);
    let exci_post = gpu.pci.bar_read_u32(gpu.bar, (SEC2 + 0x0d0) as u64);
    let eng = gpu.pci.bar_read_u32(gpu.bar, (SEC2 + 0x000) as u64);
    info!(
        "BOOTER exci probe: pre={exci_pre:#010x} post-clear={exci_post:#010x}; ENG={eng:#010x} (cpuctl={cpuctl:#010x})"
    );
    info!(
        "BOOTER fire: sec-imem {:#x}+{:#x}@dest0 dmem {:#x}+{:#x} sig@{sig_dmem:#x} meta={meta_addr:#x} cpuctl={cpuctl:#x}",
        hdr.app_code_off, hdr.app_code_size, hdr.data_off, hdr.data_size
    );
    if cpuctl & (1 << 6) != 0 {
        gpu.write32(SEC2 + 0x130, 1 << 1);
    } else {
        gpu.write32(SEC2 + 0x100, 1 << 1);
    }
    // START-drop probe: on the GSP the same write starts the core (FWSEC
    // ran); on SEC2 the core has never shown STARTED. Read back IMMEDIATELY
    // (before any halt wait): if CPUCTL is byte-identical to the pre-start
    // value, the write was silently dropped by a PLM-class gate - the
    // single fact that redirects the whole SEC2 investigation.
    let imm0 = gpu.pci.bar_read_u32(gpu.bar, (SEC2 + 0x100) as u64);
    uefi::boot::stall(core::time::Duration::from_millis(5));
    let imm1 = gpu.pci.bar_read_u32(gpu.bar, (SEC2 + 0x100) as u64);
    info!("BOOTER start-probe: pre={cpuctl:#010x} imm={imm0:#010x} +5ms={imm1:#010x}");
    if imm0 == cpuctl && cpuctl & (1 << 6) == 0 {
        // Write appears dropped: retry once through the alias path.
        gpu.write32(SEC2 + 0x130, 1 << 1);
        let imm2 = gpu.pci.bar_read_u32(gpu.bar, (SEC2 + 0x100) as u64);
        info!("BOOTER start-probe: alias retry -> {imm2:#010x}");
    }
    let r: Result<(), TxError> = match wait {
        BooterWait::Halt => gpu.poll_us(SEC2 + 0x100, 1 << 4, 1 << 4, 5_000_000, true),
        BooterWait::PlmFlip => {
            // unlock_610.c falcon_hs wait_plm=true loop: vetted reads, the
            // PLM cold->open transition is success, HALT is -EIO. Entry PLM
            // must be closed for the flip to prove anything (claim_guards
            // EALREADY already rejects an open GA10x PLM upstream of here).
            let entry = gpu.pci.bar_read_u32(gpu.bar, crate::tx::regs::FEAT_PLM as u64);
            info!("BOOTER wait_plm: entry FEAT_PLM={entry:#010x} (cold entry must be closed; open entry cannot prove execution)");
            let mut verdict = Err(TxError::ETimeout { tolerated: false });
            let max_iters = 5_000_000 / 75 + 1;
            for i in 0..max_iters {
                let engine = gpu.pci.bar_read_u32(gpu.bar, (SEC2 + 0x100) as u64);
                if engine == !0u32 || engine & 0xffff_0000 == 0xbadf_0000 {
                    verdict = Err(TxError::ENodev);
                    break;
                }
                let plm = gpu.pci.bar_read_u32(gpu.bar, crate::tx::regs::FEAT_PLM as u64);
                if plm == !0u32 && entry != !0u32 {
                    info!("BOOTER wait_plm: PLM flip {entry:#010x} -> 0xffffffff after ~{}us - payload executed (booter stays in the fill self-loop; no halt will follow)", i * 75);
                    verdict = Ok(());
                    break;
                }
                if engine & (1 << 4) != 0 {
                    info!("BOOTER wait_plm: early PLM failed - the booter halted WITHOUT flipping the PLM (signature DMA missed the stack): cpuctl={engine:#010x} plm={plm:#010x} entry={entry:#010x}");
                    verdict = Err(TxError::EIO);
                    break;
                }
                uefi::boot::stall(core::time::Duration::from_micros(75));
            }
            verdict
        }
    };
    let cpuctl_now = gpu.pci.bar_read_u32(gpu.bar, (SEC2 + 0x100) as u64);
    let mbox0 = gpu.pci.bar_read_u32(gpu.bar, (SEC2 + 0x40) as u64);
    let exci = gpu.pci.bar_read_u32(gpu.bar, (SEC2 + 0x0d0) as u64);
    // VRAM-staging audit: if we switched the aperture to LOCAL_FB pre-start,
    // this tells us whether it survived to the end (0x114) or was re-set to
    // coherent sysmem (0x115) somewhere between our write and the meta DMA -
    // the difference decides whether VRAM staging is architecturally
    // reachable at all.
    let trans_now = gpu.read32(crate::tx::eng::SEC2_FBIF).unwrap_or(0);
    info!("BOOTER forensic: CPUCTL={cpuctl_now:#010x} MAILBOX0={mbox0:#010x} EXCI={exci:#010x} TRANSCFG={trans_now:#010x}");
    r?;
    Ok(())
}

/// SEC2 booter fire, Turing BOOT_DIRECT variant (kernel_gsp_falcon_tu102.c
/// s_prepareHsFalconDirect + kgspExecuteHsFalcon_TU102 tail). No HS
/// bootstrap, no DMA: the image is portal-copied into SEC2 IMEM/DMEM
/// (non-secure part at 0, secure part at ALIGN_UP(ns_size,256), DMEM at 0),
/// BOOTVEC=0, mailboxes carry the WprMeta address; success = HALT with
/// mailbox0 == 0. Turing BOOT_DIRECT semantics (cmp50hx
/// booter_load_tu102_direct v60-v62, proven on 40HX/50HX): the wrapper does
/// the full B-reset itself - falcon_reset (BCR=Falcon on RISC-V-capable
/// engines), FALCON_RM = chipId0, FBIF unblock - then port-loads. The meta
/// address is above 4GB by default (dma_alloc high staging, issues
/// #14/#16: sub-4GB meta reads get LOCKED by some Turing cards with booter
/// exit 0x91; the driver stages at 0x110BB0000 and passes a 64-bit mailbox
/// pair) - \tx-low-staging.txt reverts for A/B.
pub fn booter_fire_direct(gpu: &TxGpu, hdr: &BooterHdr, image: &[u8], meta_dma: u64) -> Result<(), TxError> {
    const FALCON_CPUCTL: u32 = 0x100;
    const FALCON_BOOTVEC: u32 = 0x104;
    const FALCON_MAILBOX0: u32 = 0x040;
    const FALCON_MAILBOX1: u32 = 0x044;
    // START is BIT 1 (0x2) - NV_PFALCON_FALCON_CPUCTL_STARTCPU_TRUE (cmp50hx
    // line 353, and the same value the proven fwsec_ignite/booter_fire write).
    // The stub this replaced carried 0x1: a no-op on the chip, so the booter
    // NEVER STARTED across attempts 6-9 - the HALT poll passed trivially on
    // the post-reset halted state and every downstream verdict was vacuous.
    const CPUCTL_START: u32 = 0x2;
    let base = crate::tx::eng::SEC2;

    crate::tx::falcon_reset(gpu, base, crate::tx::eng::SEC2_FBIF, crate::tx::eng::SEC2_FALCON2, false)?;
    let chip = gpu.read32(0x0)?; // PMC_BOOT_0
    gpu.write32(base + 0x084, chip); // FALCON_RM = chipId0
    crate::tx::falcon_dma_unblock(gpu, base, crate::tx::eng::SEC2_FBIF)?;

    let ns = &image[hdr.imem_ns_off as usize..(hdr.imem_ns_off + hdr.imem_ns_size) as usize];
    crate::tx_portal::portal_load_imem(gpu, base, 0, hdr.imem_ns_off, ns, false);
    let place = sec_placement(hdr);
    let sec = &image[hdr.app_code_off as usize..(hdr.app_code_off + hdr.app_code_size) as usize];
    crate::tx_portal::portal_load_imem(gpu, base, place.dest, place.tag, sec, true);
    let dmem = &image[hdr.data_off as usize..(hdr.data_off + hdr.data_size) as usize];
    crate::tx_portal::portal_load_dmem(gpu, base, 0, dmem);
    portal_audit(gpu, hdr, image, &place);

    // BOOTVEC = start of non-secure code (or the secure entry under the
    // \tx-bootvec-sec.txt switch); mailboxes = WprMeta phys addr.
    gpu.write32(base + FALCON_BOOTVEC, place.bootvec);
    gpu.write32(base + FALCON_MAILBOX0, meta_dma as u32);
    gpu.write32(base + FALCON_MAILBOX1, (meta_dma >> 32) as u32);
    info!(
        "BOOTER-DIRECT: fire meta dev={meta_dma:#x} mbox0={:#010x} mbox1={:#010x}",
        meta_dma as u32,
        meta_dma >> 32
    );
    // Alias probe + START-drop probe, mirroring the proven booter_fire: an
    // ALIAS_EN core must be started through +0x130, and a silently-dropped
    // START (the AD107M signature) must be caught BEFORE the HALT poll -
    // which otherwise passes trivially on the still-halted core.
    let cpuctl_pre = gpu.read32(base + FALCON_CPUCTL)?;
    if cpuctl_pre & (1 << 6) != 0 {
        gpu.write32(base + 0x130, 1 << 1);
    } else {
        gpu.write32(base + FALCON_CPUCTL, CPUCTL_START);
    }
    let imm0 = gpu.pci.bar_read_u32(gpu.bar, (base + FALCON_CPUCTL) as u64);
    uefi::boot::stall(core::time::Duration::from_millis(5));
    let imm1 = gpu.pci.bar_read_u32(gpu.bar, (base + FALCON_CPUCTL) as u64);
    info!("BOOTER-DIRECT start-probe: pre={cpuctl_pre:#010x} imm={imm0:#010x} +5ms={imm1:#010x}");
    // Mid-run DMEM sampler: the booter's sanitize pass (0xdead5ec2) wipes
    // every forensic trace before a post-halt read - attempt 11 proved that.
    // Sample DMEM[0x600] WHILE the core runs: the payload's fill dword
    // 0x4a7 appearing there is direct evidence the signature DMA landed
    // (meta accepted); 0xdead5ec2 appearing means sanitize reached it
    // without a payload landing (meta rejected upstream).
    let mut dma_landed = false;
    let mut sanitized = false;
    // cmp50hx v2.69: intermediate booter mailbox codes are overwritten by
    // the final status (e.g. 0x91) before halt - the only window is live
    // polling. Log every observed mailbox transition with its sample index
    // (first iteration also proves our written input actually stuck).
    let mut mbox_prev = (u32::MAX, u32::MAX);
    let mut mbox_log_count = 0u32;
    for i in 0..4000u32 {
        let m0 = gpu.pci.bar_read_u32(gpu.bar, (base + FALCON_MAILBOX0) as u64);
        let m1 = gpu.pci.bar_read_u32(gpu.bar, (base + FALCON_MAILBOX1) as u64);
        if (m0, m1) != mbox_prev {
            mbox_prev = (m0, m1);
            if mbox_log_count < 24 {
                info!("BOOTER-DIRECT: mbox live @sample {i}: mbox0={m0:#010x} mbox1={m1:#010x}");
                mbox_log_count += 1;
            }
        }
        gpu.write32(base + 0x1c0, 0x600);
        let v = gpu.read32(base + 0x1c4)?;
        gpu.write32(base + 0x1c0, 0);
        if v == 0x0000_04a7 {
            info!("BOOTER-DIRECT: *** PAYLOAD LANDED: DMEM[0x600]=0x4a7 at sample {i} ***");
            dma_landed = true;
            break;
        }
        if v == 0xdead_5ec2 {
            info!("BOOTER-DIRECT: sanitize reached DMEM[0x600] at sample {i} - no payload landed before it");
            sanitized = true;
            break;
        }
        let halted = gpu.pci.bar_read_u32(gpu.bar, (base + FALCON_CPUCTL) as u64) & 0x10 != 0;
        if halted {
            info!("BOOTER-DIRECT: core halted at sample {i}, DMEM[0x600]={v:#010x}");
            break;
        }
        uefi::boot::stall(core::time::Duration::from_micros(500));
    }
    info!("BOOTER-DIRECT: mid-run sampler verdict: dma_landed={dma_landed} sanitized_only={sanitized}");
    if imm0 == cpuctl_pre && cpuctl_pre & (1 << 6) == 0 {
        gpu.write32(base + 0x130, 1 << 1);
        let imm2 = gpu.pci.bar_read_u32(gpu.bar, (base + FALCON_CPUCTL) as u64);
        info!("BOOTER-DIRECT start-probe: alias retry -> {imm2:#010x}");
    }
    // Wait for HALT (CPUCTL bit4); read-back of mailbox0 is the verdict.
    gpu.poll_us(base + FALCON_CPUCTL, 0x10, 0x10, 5_000_000, false)
        .or(Err(TxError::ETimeout { tolerated: false }))?;
    let mbox0 = gpu.read32(base + FALCON_MAILBOX0)?;
    let mbox1 = gpu.read32(base + FALCON_MAILBOX1)?;
    let exci = gpu.read32(base + 0x0d0)?;
    info!(
        "BOOTER-DIRECT: halted, mailbox0={mbox0:#x} mailbox1={mbox1:#x} exci={exci:#010x}"
    );
    log_exci_decode(exci);
    log_mailbox_verdict(mbox0);
    // Post-halt FBIF audit (issue #1's arbitration read): 0x115/0x190 are
    // the values we programmed at fire time; anything else means the booter
    // itself reprogrammed SEC2's aperture before dying - a strong
    // localization signal for where the rejection happened.
    let trans_post = gpu.read32(crate::tx::eng::SEC2_FBIF)?;
    let ctl_post = gpu.read32(crate::tx::eng::SEC2_FBIF + 0x24)?;
    info!(
        "BOOTER-DIRECT: post-halt SEC2 FBIF TRANSCFG0={trans_post:#010x} CTL={ctl_post:#010x} (fire-time 0x115/0x190)"
    );
    // DMEM forensics - the meta-acceptance discriminator. If the booter
    // ACCEPTED the meta, it DMA'd sigSize bytes from our signature buffer
    // to DMEM 0x600: the fill dword 0x4a7 (and the payload's frame values)
    // must now sit in DMEM. If it REJECTED the meta, those words keep the
    // booter's own init residue. Read via the same DMEM ports the loader
    // used, with a per-word offset write (no auto-increment on readback).
    let dmem_words = [0x600u32, 0x6140, 0xff38, 0xff4c, 0xff50, 0x200];
    for w in dmem_words {
        gpu.write32(base + 0x1c0, w);
        let v = gpu.read32(base + 0x1c4)?;
        gpu.write32(base + 0x1c0, 0);
        info!("BOOTER-DIRECT: DMEM[{w:#06x}] = {v:#010x}");
    }
    if dma_landed {
        info!("BOOTER-DIRECT: DMEM verdict: signature DMA LANDED (fill 0x4a7 seen) - booter entered canary/ROP territory");
    } else {
        info!("BOOTER-DIRECT: DMEM verdict: signature DMA never landed - the booter rejected the meta/staging before the exploit point");
    }
    // Turing BOOT_DIRECT halt shapes (sixth TU106 fire + issues #14/#16):
    // the mailboxes carry the 64-bit WprMeta address as INPUT. A healthy
    // early halt leaves that address in mailbox0; a report_status path
    // OVERWRITES it with a code. 0x91 = meta/staging rejection family
    // (cmp50hx/chenxluo exit-0x91 dictionary: meta layout broken, WPR2
    // down, or sub-4GB sysmem reads locked). Neither shape alone proves
    // success; the verdict lands in verify_execution_tu10x (EXCI + WPR2 +
    // PLM-locked + boot-stage scratch).
    info!("BOOTER-DIRECT: verdict deferred to POST-BOOT (mailbox0 may be input carrier or status code)");
    Ok(())
}

/// Cache flush for the C-reference fire shape (wbinvd before the portal
/// verifies and again before START, mirroring the proven run).
fn wbinvd_flush() {
    // SAFETY: privileged instruction, no memory operands; boot-services
    // single-threaded context.
    unsafe { core::arch::asm!("wbinvd", options(nostack, preserves_flags)) };
}

// ---------------------------------------------------------------------------
// TU10x booter forensics (branch diag/tu10x-forensics, 2026-10-31).
//
// Three instruments for the 0x91 family (docs/tu10x-booter-0x91-matrix.md):
//  - placement logging + placement switches: PR #67's diagnostic shape
//    (Catnip202X), field-proven by issue #62's D/E rounds (the only axis
//    that ever moved EXCI);
//  - a full portal readback audit before START: the old verify covered
//    exactly one IMEM word and one DMEM word;
//  - EXCI / mailbox0 decoders built from the NS-stub disassembly
//    (docs/tu10x-booter-nsstub-disasm.md) and the authoritative EXCI
//    bitfields (open-gpu-kernel-modules dev_soe_ip.h, NV_SOE_FALCON_EXCI).
// The sim-side table copy lives in sim/src/booter_diag.rs - keep both in
// sync when the dictionary grows.
// ---------------------------------------------------------------------------

/// EXCAUSE[24:20] names (NV_SOE_FALCON_EXCI_EXCAUSE_*, Falcon IP common).
pub fn exci_cause_name(cause: u32) -> &'static str {
    match cause {
        0..=3 => "TRAPn",
        0x08 => "ILL_INS",
        0x09 => "INV_INS",
        0x0a => "MISS_INS",
        0x0b => "DHIT_INS",
        0x0d => "SP_OVERFLOW",
        0x0f => "BRKPT_INS",
        0x10 => "DMEM_MISS_INS",
        0x11 => "DMEM_DHIT_INS",
        0x12 => "DMEM_PAFAULT_INS",
        0x13 => "DMEM_PERMISSION_INS",
        0x15 => "BROM_CALL_INS",
        0x16 => "KMEM_VIOLATION_INS",
        0x17 => "BMEM_PERMISSION_INS",
        _ => "unknown",
    }
}

/// EXCI decode line with the corrected bit reading: EXCAUSE is [24:20]
/// (0x00900100 = EXPC 0x100 + INV_INS(9), NOT the previously annotated
/// 0x10/DMEM_MISS_INS), and 0x00900100 also appears in fully successful
/// runs (travis guarded evidence pack, all three attempts) - it is the
/// secure stage's terminal self-kill residue, not a verdict by itself.
fn log_exci_decode(exci: u32) {
    let expc = exci & 0x000f_ffff;
    let cause = (exci >> 20) & 0x1f;
    info!(
        "BOOTER: EXCI decode: EXPC={expc:#x} EXCAUSE={cause:#x} ({}) - also seen in successful runs, not a verdict by itself",
        exci_cause_name(cause)
    );
}

/// Post-halt mailbox0 dictionary (NS-stub disassembly + field correlation,
/// docs/tu10x-booter-nsstub-disasm.md C3): the stub writes 0x31 right before
/// `lcall 0x100` into the secure entry, the secure stage writes its own
/// codes after that. A FINAL 0x31 therefore means the secure entry never
/// executed; 0x89-0x91 mean it executed and rejected.
pub fn mailbox0_verdict(mbox0: u32) -> &'static str {
    match mbox0 {
        0x31 => "NS-stub pre-secure marker (final) - the secure entry never executed (placement/entry-level failure)",
        0x0 => "secure-stage success status",
        0x89..=0x90 => "wpr_desc_validate rejection (meta DMA readback mismatch)",
        0x91 => "secure-stage rejection family - the secure code RAN and rejected (cmp50hx dictionary)",
        0x11 | 0x0b => "secure-stage early status (ran, rejected before validation)",
        _ => "input-carrier residue or unknown",
    }
}

fn log_mailbox_verdict(mbox0: u32) {
    info!("BOOTER: mailbox0 verdict: {mbox0:#x} - {}", mailbox0_verdict(mbox0));
}

/// ASCII number parse ("0x200" hex or "512" decimal) for \tx-sec-base.txt.
fn parse_num_ascii(raw: &[u8]) -> Option<u32> {
    let mut hex = false;
    let mut val: u32 = 0;
    let mut any = false;
    for &b in raw {
        match b {
            0 | b'\n' | b'\r' | b' ' | b'\t' => continue,
            b'x' | b'X' if !any => hex = true,
            b'0'..=b'9' => {
                let d = (b - b'0') as u32;
                val = val.checked_mul(if hex { 16 } else { 10 })?.checked_add(d)?;
                any = true;
            }
            b'a'..=b'f' if hex => {
                let d = (b - b'a' + 10) as u32;
                val = val.checked_mul(16)?.checked_add(d)?;
                any = true;
            }
            b'A'..=b'F' if hex => {
                let d = (b - b'A' + 10) as u32;
                val = val.checked_mul(16)?.checked_add(d)?;
                any = true;
            }
            _ => return None,
        }
    }
    any.then_some(val)
}

/// Secure-segment placement resolver + BOOTER-PLACE log. The default is
/// byte-identical to the field-proven placement (dest and tag are two
/// quantities that happen to be equal). \tx-sec-base.txt moves dest AND tag
/// together - moving only one manufactures exactly the fetch fault this
/// diagnostic hunts for; \tx-bootvec-sec.txt starts the core at the secure
/// entry instead of the NS stub. Shape and both switches credit PR #67
/// (Catnip202X), field-tested in issue #62's D/E rounds.
struct SecPlacement {
    dest: u32,
    tag: u32,
    bootvec: u32,
    bootvec_sec: bool,
}

fn sec_placement(hdr: &BooterHdr) -> SecPlacement {
    let default_dest = hdr.imem_ns_size.div_ceil(256) * 256;
    let mut p = SecPlacement { dest: default_dest, tag: hdr.app_code_off, bootvec: 0, bootvec_sec: false };
    let override_txt = crate::tx::read_boot_file("\\tx-sec-base.txt")
        .and_then(|raw| parse_num_ascii(&raw))
        .filter(|v| *v != 0 && (*v & 0xff) == 0);
    if let Some(v) = override_txt {
        p.dest = v;
        p.tag = v;
    }
    if crate::tx::probe_file("\\tx-bootvec-sec.txt") {
        p.bootvec = p.dest;
        p.bootvec_sec = true;
    }
    if let Some(v) = override_txt {
        info!("BOOTER-PLACE: sec-base override Some({v:#x}) - dest AND tag moved together");
    }
    info!(
        "BOOTER-PLACE: ns 0x0+{:#x} -> dest 0 tag 0x0 | sec {:#x}+{:#x} -> dest {:#x} tag {:#x} (default dest {:#x}, tag==dest: {}, override: {}) | bootvec {:#x} ({})",
        hdr.imem_ns_size, hdr.app_code_off, hdr.app_code_size, p.dest, p.tag, default_dest,
        p.tag == p.dest, override_txt.is_some(), p.bootvec,
        if p.bootvec_sec { "secure entry" } else { "ns stub" }
    );
    p
}

/// Full portal readback audit BEFORE START (both fire paths). Walks the NS
/// stub, the whole secure segment and the DMEM head word-by-word through the
/// same read ports the loader uses, FNV-1a-hashing against the image bytes.
/// The secure segment is AES ciphertext on disk (entropy 7.96) - a MATCH
/// proves the portal write landed bit-exact, so a later INV_INS at the
/// entry is a fetch/decrypt-side difference, not a lost write. A DENIED
/// secure readback (badf/all-ones) is an observation, not a failure.
fn portal_audit(gpu: &TxGpu, hdr: &BooterHdr, image: &[u8], p: &SecPlacement) {
    const IMEMC: u32 = 0x180;
    const IMEMD: u32 = 0x184;
    const DMEMC: u32 = 0x1c0;
    const DMEMD: u32 = 0x1c4;
    let base = crate::tx::eng::SEC2;
    let fnv = |h: u32, w: u32| (h ^ w).wrapping_mul(0x0100_0193);
    let denied = |v: u32| v == u32::MAX || (v & 0xffff_0000) == 0xbadf_0000;

    // NS stub: VA 0..ns_size must read back exactly (it is what the core
    // executes first - a mismatch here is fatal evidence on its own).
    let ns = &image[hdr.imem_ns_off as usize..(hdr.imem_ns_off + hdr.imem_ns_size) as usize];
    let (mut h, mut ok, mut denied_at) = (0x811c_9dc5u32, true, u32::MAX);
    for w in 0..(ns.len() / 4) {
        gpu.write32(base + IMEMC, (w * 4) as u32);
        let v = gpu.read32(base + IMEMD).unwrap_or(u32::MAX);
        if denied(v) { denied_at = (w * 4) as u32; break; }
        if v != u32::from_le_bytes(ns[w * 4..w * 4 + 4].try_into().unwrap()) && ok {
            ok = false;
            info!("BOOTER-AUDIT: ns first mismatch at va {:#x}: got {v:#010x}", w * 4);
        }
        h = fnv(h, v);
    }
    if denied_at != u32::MAX {
        info!("BOOTER-AUDIT: ns readback DENIED at va {denied_at:#x} (read port refused)");
    } else {
        info!("BOOTER-AUDIT: ns readback {} (fnv {:010x})", if ok { "MATCH" } else { "MISMATCH" }, h);
    }

    // Secure segment: ciphertext we wrote - expect it back bit-exact.
    let sec = &image[hdr.app_code_off as usize..(hdr.app_code_off + hdr.app_code_size) as usize];
    let (mut h, mut ok, mut denied_at) = (0x811c_9dc5u32, true, u32::MAX);
    for w in 0..(sec.len() / 4) {
        gpu.write32(base + IMEMC, p.dest + (w * 4) as u32);
        let v = gpu.read32(base + IMEMD).unwrap_or(u32::MAX);
        if denied(v) { denied_at = p.dest + (w * 4) as u32; break; }
        if v != u32::from_le_bytes(sec[w * 4..w * 4 + 4].try_into().unwrap()) && ok {
            ok = false;
            info!("BOOTER-AUDIT: sec first mismatch at va {:#x}: got {v:#010x}", p.dest + (w * 4) as u32);
        }
        h = fnv(h, v);
    }
    if denied_at != u32::MAX {
        info!("BOOTER-AUDIT: sec readback DENIED at va {denied_at:#x} (secure pages may refuse NS reads - observation, not a failure)");
    } else {
        info!("BOOTER-AUDIT: sec readback {} (fnv {:010x}, {:x} bytes walked)", if ok { "MATCH" } else { "MISMATCH" }, h, sec.len());
    }

    // DMEM head: first 256B of the data segment.
    let dmem = &image[hdr.data_off as usize..(hdr.data_off + hdr.data_size) as usize];
    let (mut h, mut ok) = (0x811c_9dc5u32, true);
    for w in 0..64u32 {
        gpu.write32(base + DMEMC, w * 4);
        let v = gpu.read32(base + DMEMD).unwrap_or(u32::MAX);
        if v != u32::from_le_bytes(dmem[(w * 4) as usize..(w * 4 + 4) as usize].try_into().unwrap()) && ok {
            ok = false;
            info!("BOOTER-AUDIT: dmem first mismatch at {:#x}: got {v:#010x}", w * 4);
        }
        h = fnv(h, v);
    }
    info!("BOOTER-AUDIT: dmem readback {} (fnv {:010x})", if ok { "MATCH" } else { "MISMATCH" }, h);
}

/// C-reference SEC2 booter fire for Turing BOOT_DIRECT (issue #17/#62
/// matrix, marker \tx-sec2-c-load.txt). Byte-exact mirror of the
/// hardware-proven reference path (chenxluo/tu102-22g-uefi-unlock
/// fire_direct, 2026-10-29, the exact #17 card) in everything it does
/// DIFFERENTLY from booter_fire_direct, where the load placement itself
/// (NS@0, SEC@align256(ns), DMEM@0, BOOTVEC=0) is identical and kept:
/// 1. no engine reset - entry must already be halted (checked, never
///    assumed); the reference never resets SEC2;
/// 2. FBIF CTL bit7 (ALLOW_PHYS_NO_CTX) preserve-set + DMACTL=0 WITHOUT
///    touching TRANSCFG (the default path additionally forces
///    TRANSCFG0=(x&!7)|5; the reference leaves the as-POSTed aperture);
/// 3. a fully QUIET 3s window after START - the default path samples
///    DMEM/mailboxes mid-run through the same portal registers; the
///    reference performs zero MMIO until the halt poll;
/// 4. direct CPUCTL=2 START with no alias-retry branch, plus the
///    reference's pre-START portal verify readbacks.
pub fn booter_fire_direct_c(gpu: &TxGpu, hdr: &BooterHdr, image: &[u8], meta_dma: u64) -> Result<(), TxError> {
    let base = crate::tx::eng::SEC2;
    // Entry precondition (reference fire_direct): SEC2 halted and readable.
    // No reset is performed - if the PRI window is closed or the core runs,
    // this variant refuses instead of "fixing" the state.
    let cpuctl = gpu.pci.bar_read_u32(gpu.bar, (base + 0x100) as u64);
    if cpuctl == !0u32 || cpuctl & 0xffff_0000 == 0xbadf_0000 || cpuctl & (1 << 4) == 0 {
        info!(
            "BOOTER-C: entry precondition failed (CPUCTL={cpuctl:#010x}, want halted+readable) - refusing (the reference path never resets)"
        );
        return Err(TxError::EIO);
    }
    info!("BOOTER-C: entry halted ok (CPUCTL={cpuctl:#010x}) - no reset, as-POSTed state kept");
    // Reference preamble: FBIF CTL bit7 preserve-set, DMACTL = 0. TRANSCFG
    // stays exactly as the POST/BSI handoff left it.
    let ctl = gpu.read32(base + 0x624)?;
    gpu.write32(base + 0x624, ctl | (1 << 7));
    gpu.write32(base + 0x10c, 0);
    info!("BOOTER-C: FBIF CTL {ctl:#010x} -> {:#010x} (bit7), DMACTL=0, TRANSCFG untouched", ctl | (1 << 7));
    // Portal loads - identical placement to booter_fire_direct (and the same
    // placement switches + readback audit, so the two paths stay comparable
    // in the #17/#62 matrix rounds).
    let ns = &image[hdr.imem_ns_off as usize..(hdr.imem_ns_off + hdr.imem_ns_size) as usize];
    crate::tx_portal::portal_load_imem(gpu, base, 0, hdr.imem_ns_off, ns, false);
    let place = sec_placement(hdr);
    let sec = &image[hdr.app_code_off as usize..(hdr.app_code_off + hdr.app_code_size) as usize];
    crate::tx_portal::portal_load_imem(gpu, base, place.dest, place.tag, sec, true);
    let dmem = &image[hdr.data_off as usize..(hdr.data_off + hdr.data_size) as usize];
    crate::tx_portal::portal_load_dmem(gpu, base, 0, dmem);
    portal_audit(gpu, hdr, image, &place);
    wbinvd_flush();
    // Reference verify readbacks BEFORE start (imem word0, dmem 0x200).
    gpu.write32(base + 0x180, 0);
    let imem_rb = gpu.read32(base + 0x184)?;
    let ns_word0 = u32::from_le_bytes(ns[..4].try_into().unwrap());
    if imem_rb != ns_word0 {
        info!("BOOTER-C: imem verify mismatch: {imem_rb:#010x} != {ns_word0:#010x}");
        return Err(TxError::EIO);
    }
    gpu.write32(base + 0x1c0, 0x200);
    let dmem_rb = gpu.read32(base + 0x1c4)?;
    gpu.write32(base + 0x1c0, 0);
    if dmem.len() >= 0x204 {
        let dmem_word = u32::from_le_bytes(dmem[0x200..0x204].try_into().unwrap());
        if dmem_rb != dmem_word {
            info!("BOOTER-C: dmem verify mismatch: {dmem_rb:#010x} != {dmem_word:#010x}");
            return Err(TxError::EIO);
        }
    }
    info!("BOOTER-C: portal verify ok (imem {imem_rb:#010x} dmem {dmem_rb:#010x})");
    // BOOTVEC = 0 (or the secure entry under \tx-bootvec-sec.txt), mailboxes
    // = meta, direct START (the reference writes CPUCTL=2 unconditionally;
    // the alias retry lives in the default path).
    gpu.write32(base + 0x104, place.bootvec);
    gpu.write32(base + 0x040, meta_dma as u32);
    gpu.write32(base + 0x044, (meta_dma >> 32) as u32);
    wbinvd_flush();
    info!(
        "BOOTER-C: fire meta dev={meta_dma:#x} mbox0={:#010x} mbox1={:#010x} BOOTVEC=0 START (quiet window next - no MMIO for 3s)",
        meta_dma as u32,
        meta_dma >> 32
    );
    gpu.write32(base + 0x100, 2);
    // QUIET window: 3s of zero MMIO (the reference's exact shape), then the
    // bounded halt poll, then a 1s settle. NO mid-run portal sampling.
    uefi::boot::stall(core::time::Duration::from_micros(3_000_000));
    gpu.poll_us(base + 0x100, 1 << 4, 1 << 4, 2_000_000, true)?;
    uefi::boot::stall(core::time::Duration::from_micros(1_000_000));
    let cpu_end = gpu.read32(base + 0x100)?;
    let mbox0 = gpu.read32(base + 0x040)?;
    let mbox1 = gpu.read32(base + 0x044)?;
    let exci = gpu.read32(base + 0x0d0)?;
    info!("BOOTER-C: post-quiet CPUCTL={cpu_end:#010x} mailbox0={mbox0:#x} mailbox1={mbox1:#x} exci={exci:#010x} (verdict lands in POST-BOOT)");
    log_exci_decode(exci);
    log_mailbox_verdict(mbox0);
    Ok(())
}

/// Booter status codes 0x89-0x90 belong to `wpr_desc_validate` (cmp170hx
/// falcon-and-booter status table; 0x8e/0x8f are that function's later
/// 0x1ffff-alignment and 0xfff-field checks): the WprMeta the booter
/// DMA-read back did not match what we packed. The booter stage is the
/// transaction's first host-DMA consumer, and issue #1 (28E1, AMD desktop)
/// traced exactly this failure to firmware IOMMU (AMD-Vi) translation
/// during boot services - resolved by disabling it. SVM (AMD-V) verified
/// harmless with it on (R9000P 7945HX); Intel VT-x/VT-d likewise.
///
/// The CPU-side readback of the meta page turns the inference into direct
/// evidence: our pack is a plain DRAM store, so an intact magic while the
/// booter reports a bad descriptor pins the fault on the device-to-host
/// read path (firmware IOMMU remap). A corrupt magic instead indicts the
/// buffer itself - our bug, not the platform.
fn wpr_desc_rejected_hint(mbox0: u32, meta: &[u8]) {
    // 0x91 family (docs/tu10x-booter-nsstub-disasm.md C3): the secure stage
    // RAN and wrote this - unlike 0x89-0x90 the readback of OUR meta page
    // cannot localize it alone. Point at the two ablation axes instead.
    if mbox0 == 0x91 {
        info!("POST-BOOT: status 0x91 = secure-stage rejection (the secure code executed and wrote this - cmp50hx wpr_desc_validate family)");
        info!("POST-BOOT: next axes (docs/tu10x-c-parity-diff.md B4): cold boot + \\tx-fwsec-gsp-halt.txt first, then \\tx-fwsec-hold-dma.txt - capture the BOOTER-AUDIT lines either way");
    }
    if mbox0 == 0x31 {
        info!("POST-BOOT: status 0x31 = NS-stub pre-secure marker (final) - the secure entry never executed (placement/entry level, see PR #67 D/E rounds)");
    }
    if (0x89..=0x90).contains(&mbox0) {
        info!("POST-BOOT: status 0x89-0x90 = WPR descriptor (WprMeta) rejected on DMA readback");
        info!("POST-BOOT: platform hint - turn firmware IOMMU (AMD-Vi) OFF in BIOS (SVM/AMD-V and Intel VT-x/VT-d are verified harmless)");
        if meta.len() >= 16 {
            let got = rd64(meta, 0);
            if got == WPR_META_MAGIC {
                info!("POST-BOOT: CPU readback of the meta page: magic intact ({got:#018x}) - our write landed in DRAM, the booter's DMA read is what came back garbage");
            } else {
                info!("POST-BOOT: CPU readback of the meta page: magic CORRUPT (got {got:#018x}, want {:018x}) - buffer-side problem, not platform DMA", WPR_META_MAGIC);
            }
        }
    }
}

/// Execution verdict. Payload mode: family-dependent. GA102 (canonical V67,
/// single FEAT PLM write + fill self-loop tail): the PLM flip is the whole
/// verdict - no mailbox marker exists, and the WPRCFG/FBPA/WPR words are
/// Ada-chain writes this blob does not carry. Ada (four-write chain): the
/// four-PLM read-back plus the mailbox marker. Chain-replay mode (stock
/// booter): the booter's own status (mailbox0 == 0, kernel
/// s_executeBooterUcode semantics) plus WPR2 still up - the stock booter
/// never touches the PLM selectors, so demanding unlock values here would
/// fail a correct run. `meta` is the host view of the published WprMeta DMA
/// page, for the failure-path readback in `wpr_desc_rejected_hint`.
pub fn verify_execution(gpu: &TxGpu, family: &str, payload_mode: bool, meta: &[u8]) -> Result<(), TxError> {
    let plm = gpu.pci.bar_read_u32(gpu.bar, crate::tx::regs::FEAT_PLM as u64);
    let ss0 = gpu.pci.bar_read_u32(gpu.bar, crate::tx::regs::FEAT_SS0 as u64);
    let ss1 = gpu.pci.bar_read_u32(gpu.bar, crate::tx::regs::FEAT_SS1 as u64);
    info!("POST-BOOT: FEAT_PLM={plm:#x} SS0={ss0:#x} SS1={ss1:#x}");
    // Factory compute-throttle readout, sampled BEFORE the selector writes:
    // on Turing the FEAT cluster is PL0-locked (open-questions #1), so a
    // payload-opened PLM is the first readable moment - and once write_selector
    // runs, FEAT_READOUT_1 reflects our unlock values and the factory state is
    // gone. Replay mode keeps it as the negative control (expected: still
    // badf-locked, the stock booter never opens the FEAT PLM).
    let readout1 = gpu.pci.bar_read_u32(gpu.bar, crate::tx::regs::FEAT_READOUT_1 as u64);
    info!("POST-BOOT: FACTORY FEAT_READOUT_1={readout1:#010x} (0 = full speed; badf5040 = PLM still closed)");
    let mbox0 = gpu.pci.bar_read_u32(gpu.bar, (crate::tx::eng::SEC2 + 0x40) as u64);
    let wpr_hi = gpu.pci.bar_read_u32(gpu.bar, 0x001f_a828);
    info!("POST-BOOT: SEC2 MAILBOX0={mbox0:#010x} WPR2_HI={wpr_hi:#x}");
    if !payload_mode {
        if mbox0 != 0 {
            info!("POST-BOOT: booter status {mbox0:#x} (nonzero = failed)");
            wpr_desc_rejected_hint(mbox0, meta);
            return Err(TxError::EIO);
        }
        if wpr_hi >> 4 == 0 {
            info!("POST-BOOT: WPR2 down after stock boot");
            return Err(TxError::EIO);
        }
        return Ok(());
    }
    if plm != 0xffff_ffff {
        info!("POST-BOOT: PLM still locked - the payload's four writes did not happen");
        wpr_desc_rejected_hint(mbox0, meta);
        return Err(TxError::EIO);
    }
    let wprcfg = gpu.pci.bar_read_u32(gpu.bar, crate::tx::regs::WPRCFG_PLM as u64);
    let fbpa = gpu.pci.bar_read_u32(gpu.bar, crate::tx::regs::FBPA_PLM as u64);
    let wpr = gpu.pci.bar_read_u32(gpu.bar, crate::tx::regs::WPR_PLM as u64);
    info!("POST-BOOT: WPRCFG={wprcfg:#x} FBPA={fbpa:#x} WPR={wpr:#x}");
    if family == "ga102" {
        // GA102 canonical payload: ONE write (FEAT_OVR_PLM). The ga102
        // patched-word table (assets payload.rs) carries no marker and no
        // WPRCFG/FBPA/WPR writes; those logs are informational here. The
        // SS0/SS1 selectors follow host-side from the open PLM (issue #6).
        return Ok(());
    }
    if wprcfg != 0xffff_f0ff || wpr != 0xffff_ffff {
        return Err(TxError::EIO);
    }
    // FBPA is observational: on every AD10x card tested (2705/2805/
    // 28e1 community data + our 28e0 cold value) the write sticks at
    // 0xffffff0f - a flip is welcome, being stuck is NOT a failure.
    if fbpa != 0xffff_ff0f && fbpa != 0xffff_ffff {
        return Err(TxError::EIO);
    }
    if mbox0 != 0x67b0_0001 {
        info!("POST-BOOT: mailbox0 marker={mbox0:#x}");
        wpr_desc_rejected_hint(mbox0, meta);
        return Err(TxError::EIO);
    }
    Ok(())
}

/// Turing execution verdict. Chain-replay (stock booter): mailbox0 == 0
/// (kernel s_executeBooterUcode semantics) plus WPR2 still up - the stock
/// booter never touches the FEAT PLM, so demanding unlock values here would
/// fail a correct run. Payload mode (TU102 V67): the payload's two writes
/// are FEAT_PLM=0xffffffff + WPR2_HI=0 plus the ACR mutex-3 release - the
/// GA10x marker/WPRCFG triad does not exist on Turing; the factory
/// FEAT_READOUT_1 sample lands right after the PLM check, before any
/// selector write destroys the factory state.
/// Turing FEAT block addresses - NOT the GA10x/Ada 0x823xxx block! The
/// community-proven TU102 payload writes PLM at 0x409650 (cmp50hx lines
/// 294-296: PLM 0x409650 / SS0 0x409664 / SS1 0x40966c), and our M1 sweep
/// of 0x823800-0x82382f returning 13/13 badf5040 measured some OTHER
/// locked block. Attempts 11/12 most likely FIRED (WPR2_HI=0 is payload
/// write #2's target state) while we verified the wrong register.
pub const TU10X_FEAT_PLM: u32 = 0x0040_9650;
pub const TU10X_FEAT_SS0: u32 = 0x0040_9664;
pub const TU10X_FEAT_SS1: u32 = 0x0040_966c;

pub fn verify_execution_tu10x(gpu: &TxGpu, payload_mode: bool, meta: &[u8]) -> Result<(), TxError> {
    let plm = gpu.pci.bar_read_u32(gpu.bar, TU10X_FEAT_PLM as u64);
    let ss0 = gpu.pci.bar_read_u32(gpu.bar, TU10X_FEAT_SS0 as u64);
    let ss1 = gpu.pci.bar_read_u32(gpu.bar, TU10X_FEAT_SS1 as u64);
    info!("POST-BOOT: FEAT_PLM={plm:#x} SS0={ss0:#x} SS1={ss1:#x} (Turing 0x4096xx block)");
    // Factory compute-throttle readout: on Turing this is only readable once
    // the payload has opened the FEAT PLM (open-questions #1/#10). In replay
    // mode it stays badf-locked and doubles as the negative control.
    let readout1 = gpu.pci.bar_read_u32(gpu.bar, crate::tx::regs::FEAT_READOUT_1 as u64);
    info!("POST-BOOT: FACTORY FEAT_READOUT_1={readout1:#010x} (0 = full speed; badf5040 = PLM still closed)");
    let mbox0 = gpu.pci.bar_read_u32(gpu.bar, (crate::tx::eng::SEC2 + 0x40) as u64);
    let wpr_hi = gpu.pci.bar_read_u32(gpu.bar, 0x001f_a828);
    info!("POST-BOOT: SEC2 MAILBOX0={mbox0:#010x} WPR2_HI={wpr_hi:#x}");
    if !payload_mode {
        // Turing BOOT_DIRECT: mailbox0/1 keep the meta INPUT (the address we
        // passed), not a status - the sixth fire proved mbox0 == meta dev
        // address on a clean halt. Replay verdict = hard evidence only:
        // no SEC2 exception, WPR2 still up, PLM still locked; the boot-stage
        // scratch state is logged for the record.
        let exci = gpu.pci.bar_read_u32(gpu.bar, (crate::tx::eng::SEC2 + 0x0d0) as u64);
        let gsp_cpu = gpu.pci.bar_read_u32(gpu.bar, (crate::tx::eng::GSP + 0x100) as u64);
        let scratch14 = gpu.pci.bar_read_u32(gpu.bar, 0x0011_80f8);
        let bsi = gpu.pci.bar_read_u32(gpu.bar, 0x0011_80fc);
        info!(
            "POST-BOOT: SEC2 EXCI={exci:#010x} GSP_CPUCTL={gsp_cpu:#010x} SCRATCH14={scratch14:#010x} BSI15={bsi:#010x} (pre-chain BSI15=0x202001bf)"
        );
        if exci != 0 {
            info!("POST-BOOT: SEC2 exception during booter run - booter did not complete cleanly");
            return Err(TxError::EIO);
        }
        if wpr_hi >> 4 == 0 {
            info!("POST-BOOT: WPR2 down after stock boot");
            return Err(TxError::EIO);
        }
        info!("POST-BOOT: replay verdict ok (no exception, WPR2 up, PLM stays locked)");
        return Ok(());
    }
    if plm != 0xffff_ffff {
        info!("POST-BOOT: PLM still locked - the payload's writes did not happen");
        wpr_desc_rejected_hint(mbox0, meta);
        return Err(TxError::EIO);
    }
    Ok(())
}

/// FLR phase (unlock_610.c unlock_transaction_reset, EFI adaptation):
/// pre-FLR selector verify -> BME clear -> config-space FLR trigger ->
/// post-FLR GFW/WPR2/selector survival. PCI config via PciIo.
pub fn flr_phase(gpu: &TxGpu, family: &str, payload_mode: bool) -> Result<(), TxError> {
    // FEAT SS0/SS1 addresses are family-specific (Turing 0x409664/6c vs
    // GA10x/Ada 0x82381c/20) and PLM-locked at PL0 until a payload opens
    // the FEAT PLM - in replay the read failing is expected, not an error.
    let (ss0_at, ss1_at) = if family == "tu10x" {
        (TU10X_FEAT_SS0, TU10X_FEAT_SS1)
    } else {
        (crate::tx::regs::FEAT_SS0, crate::tx::regs::FEAT_SS1)
    };
    let ss0 = gpu.read32(ss0_at).unwrap_or(0);
    let ss1 = gpu.read32(ss1_at).unwrap_or(0);
    info!("pre-FLR selectors: ss0={ss0:#x} ss1={ss1:#x} (0 = locked, expected in replay)");
    if payload_mode && (ss0 != SS0_UNLOCK || ss1 != SS1_UNLOCK) {
        return Err(TxError::EIO);
    }
    // FLR is only defined in D0 (PCIe); on a resume entry the function may
    // still sit in D3hot where the previous session parked it.
    let pm = crate::tx::pm_flr_pre(gpu.pci);
    // pci_save_state analog: FLR resets the BARs to 0 - without restoring
    // them the post-FLR BAR polls read 0xffffffff (first flr_phase entry
    // ever failed exactly there, 2026-10-26).
    let mut saved = [0u32; 10]; // 0x04 command, 0x08..0x0c, 0x10..0x24 BARs
    for (i, slot) in saved.iter_mut().enumerate() {
        *slot = gpu.pci.config_read_u32(0x04 + i as u32 * 4);
    }
    // BME clear (PCI command bit 2), then FLR (Device Control bit 15) -
    // TOTORONG craft: preserve the rest of Device Control (our old write
    // of 0x8000 clobbered MPS/MRR), wait Transactions Pending first, and
    // keep 100 ms of total silence after the FLR write (PCI spec).
    gpu.pci.cfg_write_u16(4, 0x0007 & !0x0004); // MMIO+port IO on, BME off
    let cmd = gpu.pci.config_read_u16(0x04);
    if cmd & 0x0004 != 0 {
        info!("FLR: BME stuck");
        return Err(TxError::EIO);
    }
    // PCIe capability by walk (Device Control at +0x08), not by the measured
    // 0x78 constant: a wrong offset would write the FLR bit into an unrelated
    // dword and the reset would silently never happen.
    let pcie = crate::pci::pcie_cap_or_measured(gpu.pci);
    let devcap = gpu.pci.config_read_u32(pcie as u32 + 0x04);
    // PCI_EXP_DEVCAP_FLR = bit 28, not 15: bit 15 is the low bit of the L1
    // Acceptable Latency field, so the old test warned on cards with L1
    // latency 0 or 2 regardless of their FLR support.
    if devcap & (1 << 28) == 0 {
        info!("FLR: DEVCAP={devcap:#010x} does not advertise FLR (bit 28 clear) - continuing (the reset may be a no-op, and a payload verdict must not be lost to it)");
    }
    let devctl = gpu.pci.config_read_u16(pcie as u32 + 0x08);
    for _ in 0..1000 {
        let devsta = gpu.pci.config_read_u16(pcie as u32 + 0x0a);
        if devsta == 0xffff || devsta & 0x20 == 0 {
            break;
        }
        uefi::boot::stall(core::time::Duration::from_millis(1));
    }
    // FLR bit 15 of Device Control at the walked offset.
    gpu.pci.cfg_write_u16(pcie as u32 + 0x08, devctl | 0x8000);
    info!("PCIe FLR triggered");
    uefi::boot::stall(core::time::Duration::from_millis(100)); // no access of any kind to the function
    gpu.pci.cfg_write_u16(pcie as u32 + 0x08, devctl);
    // Wait for config to answer before any BAR access (TOTORONG GuFlr tail).
    for _ in 0..1000 {
        let id = gpu.pci.config_read_u32(0x00);
        if id != 0xffff_ffff && id != 0 && id & 0xffff == 0x10de {
            break;
        }
        uefi::boot::stall(core::time::Duration::from_millis(1));
    }
    // pci_restore_state analog: decode off, high BAR dwords first, command
    // (never BME) last - then the post-FLR BAR polls work.
    gpu.pci.cfg_write_u16(4, 0x0000);
    for i in (2..10).rev() {
        gpu.pci.cfg_write_u32(0x04 + i as u32 * 4, saved[i]);
    }
    gpu.pci.cfg_write_u16(4, (saved[0] as u16) & !0x0004);
    let cmd_rb = gpu.pci.config_read_u16(0x04);
    info!("FLR: pci state restored (cmd={cmd_rb:#06x})");
    // Post-FLR: the chip reboots itself; GFW_BOOT cycles through states and
    // the cores settle into HALT afterwards (observed: CPUCTL 0x0 while
    // rebooting - the immediate HALT check fired too early). Poll HALT
    // generously; a timeout is logged but NOT fatal - the FLR itself is the
    // hardware guarantee that our states are wiped.
    let _ = gpu.poll_us(crate::tx::regs::GFW_BOOT, 0xff, 0xff, 5_000_000, false);
    match gpu.poll_us(crate::tx::regs::GFW_CPUCTL, 0x10, 0x10, 10_000_000, false) {
        Ok(()) => info!("POST-FLR: GFW halted the cores"),
        Err(_) => {
            let cpuctl = gpu.read32(crate::tx::regs::GFW_CPUCTL).unwrap_or(0);
            info!("POST-FLR: GSP HALT not observed within 10s (cpuctl={cpuctl:#x}) - continuing (FLR is the state guarantee)");
        }
    }
    let lo = gpu.pci.bar_read_u32(gpu.bar, 0x001f_a824 as u64);
    let hi = gpu.pci.bar_read_u32(gpu.bar, 0x001f_a828 as u64);
    info!("POST-FLR: WPR2={lo:#x}/{hi:#x}");
    if hi >> 4 != 0 {
        return Err(TxError::EIO);
    }
    let ss0 = gpu.read32(ss0_at).unwrap_or(0);
    let ss1 = gpu.read32(ss1_at).unwrap_or(0);
    info!("POST-FLR selectors: ss0={ss0:#x} ss1={ss1:#x} (0 = locked, expected in replay)");
    if payload_mode && (ss0 != SS0_UNLOCK || ss1 != SS1_UNLOCK) {
        return Err(TxError::EIO); // AON survival violated
    }
    crate::tx::pm_flr_post(gpu.pci, pm);
    Ok(())
}
