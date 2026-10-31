//! M3-class transaction machinery — behavioral port of unlock_610.c
//! (falcon_reset / poll_us / falcon_dma / falcon_hs prevalidation) to the
//! uefi-rs PciIo BAR0 window. Compiled ONLY with `--features tx`;
#![allow(dead_code)] // pre-wired ahead of the M3 gate; test surface lives in sim/
//! the default shipped binary carries zero GPU-write paths (constitution 1.7).

use core::time::Duration;
use uefi::boot;
use log::info;
use uefi::boot::stall;

use crate::pci::PciIo;

/// Falcon engine bases (BAR0 offsets).
pub mod eng {
    pub const GSP: u32 = 0x0011_0000;
    pub const SEC2: u32 = 0x0084_0000;
    /// SEC2 fbif / falcon2 sub-bases (unlock_610.c falcon_eng for SEC2).
    pub const SEC2_FBIF: u32 = 0x0084_0600;
    pub const SEC2_FALCON2: u32 = 0x0084_1000;
}

pub mod regs {
    pub const GFW_PLM: u32 = 0x0011_8128;
    pub const GFW_BOOT: u32 = 0x0011_8234;
    pub const GFW_CPUCTL: u32 = 0x0011_0100;
    pub const FB_SIZE_MIB: u32 = 0x0011_83a4;
    /// BSI_SCRATCH15: bits 31:29 = scrubber/handoff state (1 = DONE, the
    /// firmware-window state the community chains enter through).
    pub const BSI_SCRATCH15: u32 = 0x0011_80fc;
    pub const MMU_PLM: u32 = 0x001f_a7c8;
    pub const WPR2_LO: u32 = 0x001f_a824;
    pub const WPR2_HI: u32 = 0x001f_a828;
    pub const MMU_LOCK_LO: u32 = 0x001f_a82c;
    pub const MMU_LOCK_HI: u32 = 0x001f_a830;
    pub const FEAT_PLM: u32 = 0x0082_3804;
    pub const FEAT_SS0: u32 = 0x0082_381c;
    pub const FEAT_SS1: u32 = 0x0082_3820;
    /// Effective-speed readout (nine 3-bit fields; all 0 = full rate).
    pub const FEAT_READOUT_1: u32 = 0x0082_3818;
    // Ada four-write chain
    pub const WPR_PLM: u32 = 0x001f_a7c4;
    pub const WPRCFG_PLM: u32 = 0x001f_a7cc;
    pub const FBPA_PLM: u32 = 0x009a_0148;

    pub const GFW_BOOT_COMPLETED: u32 = 0xff;
    pub const CPUCTL_HALTED: u32 = 0x10;
    /// Falcon DMA command for an IMEM block (unlock_610 dma_cmd(true)).
    pub const DMA_CMD_IMEM: u32 = 6 << 8 | 0x14;
    /// DMEM variant (dma_cmd(false)).
    pub const DMA_CMD_DMEM: u32 = 6 << 8; // gpu_unlock_dma_cmd(false): plain 0x600
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TxError {
    /// Register read came back inaccessible (0xffffffff / 0xbadf****).
    EIO,
    ENodev,
    /// Poll budget exhausted; `tolerated` marks the HW-bug-3419321 path.
    ETimeout { tolerated: bool },
    EInval,
    ERange,
    EBusy,
    EAlready,
    /// FEAT PLM transport read failed during claim; do not reset over an
    /// unknown pre-existing state in the generic failure handler.
    EClaimUnreadable,
    /// Warm (non-D0) entry: aborted before any GPU write, nothing to undo.
    EWarm,
}

pub fn inaccessible(value: u32) -> bool {
    value == !0u32 || (value & 0xffff_0000) == 0xbadf_0000
}

/// BAR0-backed engine view.
pub struct TxGpu<'a> {
    pub pci: &'a PciIo,
    pub bar: u8,
}

impl TxGpu<'_> {
    pub fn read32(&self, off: u32) -> Result<u32, TxError> {
        let v = self.pci.bar_read_u32(self.bar, off as u64);
        if inaccessible(v) {
            return Err(TxError::EIO);
        }
        Ok(v)
    }

    /// Raw BAR0 read: the poison word is returned, not turned into an error.
    /// For registers where a BADF value is tolerable and the caller has to
    /// classify it (layout's VGA workspace: only bit3 matters, upstream only
    /// rejects ~0 there).
    pub fn read32_raw(&self, off: u32) -> u32 {
        self.pci.bar_read_u32(self.bar, off as u64)
    }

    /// THE write path of the whole project — exists only in `tx` builds.
    pub fn write32(&self, off: u32, v: u32) {
        self.pci.bar_write_u32(self.bar, off as u64, v);
    }

    /// poll_us port: mask/expect/budget semantics identical to the reference,
    /// including the tolerated-timeout distinction (log wording kept apart).
    pub fn poll_us(
        &self,
        off: u32,
        mask: u32,
        expect: u32,
        budget_us: u32,
        timeout_is_failure: bool,
    ) -> Result<(), TxError> {
        let step_us = 75u32; // usleep_range(50, 100) midpoint
        let max_iters = budget_us / step_us + 1;
        for _ in 0..max_iters {
            let value = self.pci.bar_read_u32(self.bar, off as u64);
            if inaccessible(value) {
                info!("poll inaccessible: reg={off:#x} last={value:#x} mask={mask:#x} expected={expect:#x} budget_us={budget_us}");
                return Err(TxError::EIO);
            }
            if value & mask == expect {
                return Ok(());
            }
            stall(Duration::from_micros(step_us as u64));
        }
        info!(
            "poll timeout{}: reg={off:#x} mask={mask:#x} expected={expect:#x} budget_us={budget_us}",
            if timeout_is_failure { "" } else { " tolerated" }
        );
        Err(TxError::ETimeout { tolerated: !timeout_is_failure })
    }
}

/// Falcon engine reset (unlock_610.c falcon_reset, 1:1):
/// pre-reset HWCFG2 wait (timeout tolerated, HW bug 3419321) -> reset
/// pulse with 10 propagation reads each way -> scrub wait -> optional
/// RISC-V<->Falcon BCR switch -> chip id into the boot vector (+0x84).
pub fn falcon_reset(gpu: &TxGpu, base: u32, _fbif: u32, falcon2: u32, switch_to_falcon: bool) -> Result<(), TxError> {    info!("Falcon reset begin: base={base:#x} switch_to_falcon={switch_to_falcon}");
    // Pre-reset wait; ETIMEDOUT tolerated.
    if let Err(e) = gpu.poll_us(base + 0xf4, 1 << 31, 1 << 31, 1000, false) {
        match e {
            TxError::ETimeout { tolerated: true } => {
                info!("Falcon pre-reset timeout tolerated per HW bug 3419321");
            }
            other => return Err(other),
        }
    }
    let engine = gpu.read32(base + 0x3c0)?;
    gpu.write32(base + 0x3c0, engine | 1);
    for _ in 0..10 {
        let _ = gpu.pci.bar_read_u32(gpu.bar, (base + 0x3c0) as u64);
    }
    gpu.write32(base + 0x3c0, engine & !1u32);
    for _ in 0..10 {
        let _ = gpu.pci.bar_read_u32(gpu.bar, (base + 0x3c0) as u64);
    }
    // Scrub wait (must complete).
    gpu.poll_us(base + 0xf4, 1 << 12, 0, 1_000_000, true)?;
    let hwcfg = gpu.read32(base + 0xf4)?;
    if hwcfg & (1 << 10) != 0 {
        if !switch_to_falcon {
            // Turing: NO explicit core switch exists - kflcnSwitchToFalcon_
            // TU102 is a soft status only, the BCR VALID bit never asserts,
            // and a readback stuck at 0x0 is NORMAL (cmp50hx v66 measured;
            // third TU106 fire timed out right here waiting for it). Park
            // BCR at Falcon and move on.
            gpu.write32(falcon2 + 0x668, 0);
            for _ in 0..16 {
                let _ = gpu.read32(falcon2 + 0x668)?;
            }
            boot::stall(core::time::Duration::from_millis(10));
        } else {
            // RISC-V-capable GSP: force the Falcon core explicitly (kernel
            // kflcnResetIntoFalcon_GA102 semantics - BCR_CTRL bit4 CORE_SELECT
            // 0=Falcon/1=RISCV, bit0 VALID reports the switch completed). The
            // GFW leaves the RISC-V side after boot; a bare "bit4 reads 0"
            // does NOT prove a completed Falcon selection, and a stale RISC-V
            // selection stalls the Falcon-compat DMA queue (observed: 32-slot
            // XFER queue never drains).
            let bcr = gpu.read32(falcon2 + 0x668)?;
            info!("Falcon reset: BCR before = {bcr:#010x} (valid={} core_select={})",
                bcr & 1, bcr >> 4 & 1);
            let falcon_confirmed = (bcr & 1) != 0 && (bcr >> 4 & 1) == 0;
            if !falcon_confirmed {
                gpu.write32(falcon2 + 0x668, 0);
                gpu.poll_us(falcon2 + 0x668, 1, 1, 1_000_000, true)?;
                let bcr2 = gpu.read32(falcon2 + 0x668)?;
                info!("Falcon reset: BCR after switch = {bcr2:#010x}");
                // NOTE: a second engine reset cycle here was tried (2026-10-26)
                // and REVERTED: the reset wiped the BCR switch back to 0x110 AND
                // hung the HWCFG bit12 scrub wait on both engines (EIO). The
                // switch-after-reset order matches unlock_610.c exactly.
            }
        }
    }
    let chip = gpu.read32(0)?;
    gpu.write32(base + 0x84, chip);
    info!("Falcon reset done: base={base:#x} hwcfg={hwcfg:#x} chip={chip:#x}");
    Ok(())
}

/// Open the engine's FBIF/DMA path for physical, ctx-less access
/// (unlock_610.c falcon_dma_load trio + kernel kflcnDisableCtxReq):
/// FBIF_CTL.ALLOW_PHYS_NO_CTX, DMACTL.REQUIRE_CTX=0, TRANSCFG(0)=
/// COHERENT_SYSMEM|PHYSICAL. The scrubber/ucode's own runtime DMA dies
/// behind REQUIRE_CTX=1 (requests queue but are never serviced - the
/// "2x32-slot stall" signature). Every portaled image still needs this
/// because the ucode itself DMA-translates through the same FBIF.
pub fn falcon_dma_unblock(gpu: &TxGpu, base: u32, fbif: u32) -> Result<(), TxError> {
    let mut ctl = gpu.read32(fbif + 0x24)?;
    ctl |= 1 << 7;
    gpu.write32(fbif + 0x24, ctl);
    gpu.write32(base + 0x10c, 0);
    let mut trans = gpu.read32(fbif)?;
    trans = (trans & !7) | 5;
    gpu.write32(fbif, trans);
    // Read-back (constitution 1.1): a gated FBIF domain drops these
    // silently and every later DMA fetch dies at translation.
    let (ctl_rb, trans_rb, dmactl_rb) =
        (gpu.read32(fbif + 0x24)?, gpu.read32(fbif)?, gpu.read32(base + 0x10c)?);
    info!(
        "FALCON-UNBLOCK: FBIF_CTL={ctl_rb:#010x} TRANSCFG0={trans_rb:#010x} DMACTL={dmactl_rb:#010x} (want ctl bit7=1, trans&7=5, dmactl=0)"
    );
    if ctl_rb & (1 << 7) == 0 || trans_rb & 7 != 5 || dmactl_rb != 0 {
        return Err(TxError::EIO);
    }
    Ok(())
}

/// Differential authority scan (candidate C): read the same falcon PRI
/// offsets on BOTH engines and log them side by side with poison bucketing.
/// Motivation (probe-proven 2026-10-26): SEC2 CPUCTL START writes are
/// dropped while GSP accepts the identical write, and SEC2 CPUCTL reads
/// badf5620 while GSP reads 0x10 - mapping the read-gate pattern across the
/// block localizes the authority gate. +0x3c4 is TOTORONG's reset-PLM
/// candidate. Data ports (IMEMD/DMEMD) are skipped to avoid side effects.
pub fn authority_scan(gpu: &TxGpu) {
    const GSP: u32 = 0x0011_0000;
    const SEC2: u32 = 0x0084_0000;
    const GSP2: u32 = 0x0011_1000;
    const SEC22: u32 = 0x0084_1000;
    const OFFS: &[(&str, u32)] = &[
        ("ENG", 0x000),
        ("MBOX0", 0x040),
        ("MBOX1", 0x044),
        ("EXCI", 0x0d0),
        ("HWCFG", 0x0f4),
        ("CPUCTL", 0x100),
        ("BOOTVEC", 0x104),
        ("CAPS", 0x108),
        ("DMACTL", 0x10c),
        ("DMATRFBASE", 0x110),
        ("DMATRFCMD", 0x118),
        ("DMATRFFBOFFS", 0x11c),
        ("DMATRFBASE1", 0x120),
        ("CPUCTL_ALIAS", 0x130),
        ("TRACEPC", 0x14c),
        ("IMEMC", 0x180),
        ("IMEMT", 0x188),
        ("DMEMC", 0x1c0),
        ("UNK300", 0x300),
        ("ENG_RESET", 0x3c0),
        ("RESET_PLM?", 0x3c4),
        ("UNK440", 0x440),
        ("UNK4E4", 0x4e4),
        ("UNK4F4", 0x4f4),
        ("UNK520", 0x520),
        ("UNK524", 0x524),
    ];
    const OFFS2: &[(&str, u32)] = &[
        ("MOD_SEL", 0x180),
        ("UCODE_ID", 0x198),
        ("ENGIDMASK", 0x19c),
        ("PARAADDR", 0x210),
        ("RISCV388", 0x388),
        ("BCR", 0x668),
    ];
    let bucket = |v: u32| match v {
        0xffff_ffff => "FFFF",
        v if v & 0xbadf_0000 == 0xbadf_0000 => "BADF",
        _ => "ok",
    };
    info!("AUTH-SCAN begin (differential GSP vs SEC2)");
    for (name, off) in OFFS {
        let g = gpu.pci.bar_read_u32(gpu.bar, (GSP + off) as u64);
        let s = gpu.pci.bar_read_u32(gpu.bar, (SEC2 + off) as u64);
        info!("AUTH-SCAN falcon +{off:#05x} {name:13} gsp={g:#010x}[{}] sec2={s:#010x}[{}]", bucket(g), bucket(s));
    }
    for (name, off) in OFFS2 {
        let g = gpu.pci.bar_read_u32(gpu.bar, (GSP2 + off) as u64);
        let s = gpu.pci.bar_read_u32(gpu.bar, (SEC22 + off) as u64);
        info!("AUTH-SCAN falcon2 +{off:#05x} {name:13} gsp={g:#010x}[{}] sec2={s:#010x}[{}]", bucket(g), bucket(s));
    }
    info!("AUTH-SCAN end");
}

/// Post-reset authority probe for one falcon: mini-scan the registers that
/// matter for start, then a SAFE write-stickiness test on CPUCTL - write
/// ALIAS_EN (bit6) WITHOUT START (bit0) and read back. If bit6 sticks,
/// CPUCTL writes work and the START bit itself is gated/broken (retry via
/// alias); if it does not stick, the register is write-protected entirely.
/// Returns the post-probe CPUCTL value.
pub fn falcon_authority_probe(gpu: &TxGpu, base: u32, name: &str) -> u32 {
    const OFFS: &[(&str, u32)] = &[
        ("CPUCTL", 0x100),
        ("CPUCTL_ALIAS", 0x130),
        ("DMACTL", 0x10c),
        ("BOOTVEC", 0x104),
        ("EXCI", 0x0d0),
        ("HWCFG", 0x0f4),
        ("ENG_RESET", 0x3c0),
        ("RESET_PLM?", 0x3c4),
    ];
    for (n, off) in OFFS {
        let v = gpu.pci.bar_read_u32(gpu.bar, (base + off) as u64);
        info!("AUTH-POST {name} +{off:#05x} {n:13} = {v:#010x}{}", {
            if v & 0xbadf_0000 == 0xbadf_0000 { "[BADF]" } else if v == 0xffff_ffff { "[FFFF]" } else { "" }
        });
    }
    let pre = gpu.pci.bar_read_u32(gpu.bar, (base + 0x100) as u64);
    if pre & 0xbadf_0000 == 0xbadf_0000 {
        info!("AUTH-POST {name}: CPUCTL unreadable, write probe skipped");
        return pre;
    }
    // SAFE probe: ALIAS_EN without START.
    gpu.write32(base + 0x100, pre | 0x40);
    let stuck = gpu.pci.bar_read_u32(gpu.bar, (base + 0x100) as u64);
    info!("AUTH-POST {name}: CPUCTL write-probe {pre:#010x} -> {stuck:#010x} ({})", 
        if stuck & 0x40 != 0 { "bit6 STUCK - writes work" } else { "bit6 dropped - write-protected" });
    if stuck & 0x40 == 0 {
        // restore; leave the register as found
        gpu.write32(base + 0x100, pre);
    }
    stuck
}

/// DMATRFMOFFS is 24 bits, FBOFFS is 32 bits, and BASE is 49 bits.
/// Reject truncation/wrap before submitting any part of a transfer.
pub fn falcon_dma_valid(dest: u32, mem_off: u32, src: u64, size: u32) -> bool {
    size != 0
        && (size as u64 | dest as u64 | mem_off as u64 | src) & 255 == 0
        && dest as u64 + size as u64 <= 1 << 24
        && mem_off as u64 + size as u64 <= 1 << 32
        && src < 1 << 49
        && mem_off as u64 + size as u64 <= (1u64 << 49) - src
}

/// Falcon DMA transfer (unlock_610.c falcon_dma, 1:1): idle poll ->
/// program source address -> per-256B chunks -> done poll.
pub fn falcon_dma(
    gpu: &TxGpu,
    base: u32,
    mut dest: u32,
    mut mem_off: u32,
    src: u64,
    size: u32,
    cmd: u32,
) -> Result<(), TxError> {
    info!("Falcon DMA: base={base:#x} dest={dest:#x} offset={mem_off:#x} src={src:#x} size={size:#x} cmd={cmd:#x}");
    if !falcon_dma_valid(dest, mem_off, src, size) {
        return Err(TxError::EInval);
    }
    gpu.poll_us(base + 0x118, 1, 0, 1_000_000, true)?;
    gpu.write32(base + 0x110, (src >> 8) as u32);
    gpu.write32(base + 0x128, ((src >> 8) >> 32) as u32 & 0x1ff);
    let mut done = 0u32;
    while done < size {
        // Wait for FIFO space; on stall, dump state and try a kick round:
        // re-write XFER_EXT_BASE (same value) - some Falcon generations
        // require the base register touch to re-arm the transfer engine
        // after the 32-slot queue fills.
        if let Err(e) = gpu.poll_us(base + 0x118, 1, 0, 1_000_000, true) {
            info!("Falcon DMA progress: done={done} total={size}");
            dma_stall_dump(gpu, base);
            let mut recovered = false;
            for round in 0..3 {
                boot::stall(core::time::Duration::from_millis(1000));
                // Kick 1: re-arm the external base register.
                gpu.write32(base + 0x110, (src >> 8) as u32);
                gpu.write32(base + 0x128, ((src >> 8) >> 32) as u32 & 0x1ff);
                // Kick 2: nudge the block-on-fifo register (0 then re-check).
                gpu.write32(base + 0x10c, 0);
                if let Ok(()) = gpu.poll_us(base + 0x118, 1, 0, 1_000_000, true) {
                    info!("Falcon DMA: recovered after kick round {round} - resuming at done={done}");
                    recovered = true;
                    break;
                }
                dma_stall_dump(gpu, base);
            }
            if !recovered {
                return Err(e);
            }
        }
        // PORT experiment: blocks 32+ issue on PORT 1 (cmd bits12:14) - if
        // the stall is port-0-local queue state, the transfer completes.
        let cmd_port = if done >= 32 * 256 { cmd | (1 << 12) } else { cmd };
        gpu.write32(base + 0x114, dest);
        gpu.write32(base + 0x11c, mem_off);
        gpu.write32(base + 0x118, cmd_port);
        done += 256;
        dest += 256;
        mem_off += 256;
        // Golden-sample trace: full register picture at block boundaries
        // (blocks 0-2 right after issue, and the 30-34 window where the
        // queue stalls) - pins the semantics of 0x118 bit0 and 0x120
        // empirically instead of trusting cross-generation docs.
        let blk = done / 256;
        if blk <= 3 || (30..=34).contains(&blk) {
            let c118 = gpu.pci.bar_read_u32(gpu.bar, (base + 0x118) as u64);
            let c120 = gpu.pci.bar_read_u32(gpu.bar, (base + 0x120) as u64);
            let c11c = gpu.pci.bar_read_u32(gpu.bar, (base + 0x11c) as u64);
            info!("DMA-BLK {blk}: 0x118={c118:#010x} 0x120={c120:#010x} 0x11c={c11c:#010x}");
        }
    }
    gpu.poll_us(base + 0x118, 2, 2, 1_000_000, true)
}

/// Read-only state dump at a DMA stall (EXCI has the exception reason).
fn dma_stall_dump(gpu: &TxGpu, base: u32) {
    let r = |off: u32| gpu.pci.bar_read_u32(gpu.bar, (base + off) as u64);
    let cpuctl = r(0x100);
    let dmactl = r(0x10c);
    let cmd_last = r(0x118);
    let exci = r(0x0d0);
    let rm = r(0x084);
    let bootvec = r(0x104);
    let hwcfg = r(0x0f4);
    let xfer_status = r(0x120);
    info!(
        "DMA-STALL dump: CPUCTL={cpuctl:#010x} (halt={} stop={} iready={}) DMACTL={dmactl:#010x} CMD={cmd_last:#010x} EXCI={exci:#010x} RM={rm:#010x} BOOTVEC={bootvec:#010x} HWCFG={hwcfg:#010x}",
        cpuctl >> 4 & 1,
        cpuctl >> 5 & 1,
        cpuctl >> 3 & 1,
    );
    info!(
        "DMA-STALL: XFER_STATUS={xfer_status:#010x} (loads_pending={} stores_pending={} pending={})",
        xfer_status >> 24 & 7,
        xfer_status >> 16 & 7,
        xfer_status >> 1 & 1,
    );
    // EXCI field decode (falcon v4): bits8-16 exception, 17:16 substatus (best effort)
    if exci != 0 {
        info!("DMA-STALL: EXCI code={:#05x} sub={:#03x}", exci >> 8 & 0xffff, exci >> 24 & 0xff);
    }
}

/// falcon_hs geometry prevalidation (before any reset or transfer):
/// Range = image-offset arithmetic; Inval = DMA geometry rejected.
pub fn hs_prevalidate(
    image: u64,
    code_off: u32,
    code_size: u32,
    code_va: u32,
    code_pa: u32,
    data_off: u32,
    data_size: u32,
    data_pa: u32,
) -> Result<u64, TxError> {
    let (code_off, code_va, data_off) = (code_off as u64, code_va as u64, data_off as u64);
    if image > u64::MAX - code_off
        || image + code_off < code_va
        || image > u64::MAX - data_off
    {
        return Err(TxError::ERange);
    }
    let code_src = image + code_off - code_va;
    if !falcon_dma_valid(code_pa, code_va as u32, code_src, code_size)
        || (data_size != 0 && !falcon_dma_valid(data_pa, 0, image + data_off, data_size))
    {
        return Err(TxError::EInval);
    }
    Ok(code_src)
}

/// Issue #4.2 escape hatch: Turing entry with WPR2 up but WITHOUT the
/// firmware-window evidence (GFW completed + GSP not started) refuses unless
/// the operator drops this marker on the stick root - a deliberate, per-stick
/// accept of the ambiguous warm state.
pub const ACCEPT_WARM_WPR2_MARKER: &str = "\\tx-accept-warm-wpr2.txt";

/// Cold-state guards (task brief 9.6 semantics).
/// `post_flr`: we just FLR'd the function ourselves - the GPU's own re-boot
/// re-provisions WPR2 (measured 0xe00), which is NOT the "already used"
/// dirty state the guard exists to catch. PLM check stays.
/// Turing + WPR2-up without a self-FLR (issue #4.2): WPR2-up at entry is the
/// mainline provisioned state there, but WPR2 alone cannot tell a firmware
/// window from "a driver used this card this power cycle". The old code
/// blanket-accepted the whole family; the accept is now evidence-based:
/// GFW_BOOT completed + GSP not started (a dormant card in a boot-firmware
/// window) accepts with the evidence logged; a still-started GSP or an
/// in-progress GFW refuses (-EBUSY) unless `ACCEPT_WARM_WPR2_MARKER` opts in
/// per stick. The BSI handoff is LOGGED but not gated: it is the scrubber
/// FSM (OpenRM kgspExecuteScrubberIfNeeded_AD102, DONE=3), not a
/// WPR2-provenance discriminator - the TU106 firmware-window entry measures
/// handoff=1 there (evidence/tu106-triage-regression), so anchoring the
/// accept on any handoff value would mis-refuse legitimate entries.
pub fn claim_guards(gpu: &TxGpu, family: &str, post_flr: bool) -> Result<(), TxError> {
    // Preflight: is BAR0 decoding at all? Right after the MEM enable a powered
    // dGPU answers; one parked by the EC in D3cold (iGPU-only boot: no GOP on
    // the dGPU) returns all-ones forever. Without this check the failure would
    // surface later as a confusing Turing-PLM "unreadable".
    let chip = gpu.read32_raw(0x0000_0000);
    if chip == !0u32 {
        info!("claim: BAR0 all-ones after MEM enable - the dGPU is powered off (D3cold); boot with the dGPU as console (hybrid shutdown with both GPUs awake, or MUX in dGPU-only) and re-run");
        return Err(TxError::ENodev);
    }
    // FEAT_PLM EALREADY: family-specific. GA10x/Ada: 0x823804 (AON - the
    // unlock survives FLR there). Turing: 0x409650, and the unlock does NOT
    // survive FLR (13th fire) - so a live Turing unlock must be detected and
    // SKIPPED, never reset over.
    let feat_plm_addr = if family == "tu10x" { 0x0040_9650 } else { regs::FEAT_PLM };
    // Same-block corroboration BEFORE the PLM is consulted (Catnip review,
    // PR #65): a dead block answers all-ones for every address, so PLM=
    // all-ones alone cannot distinguish "open" from "not answering". The
    // corroborator must traverse the SAME register block as the PLM — the
    // cluster's SS0 (0x82381c GA10x / 0x409664 Turing). PMC_BOOT_0 lives in
    // a different block and proves nothing about this one; it is logged by
    // the chip preflight above and nothing more. Semantics: transport
    // failure or all-ones on the corroborator = dead block ->
    // EClaimUnreadable (the PLM is never consulted); ANY other value,
    // INCLUDING BADF, is an answered transaction and proves the block alive
    // (the Turing locked cluster answers badf5040 on every register at PL0).
    let corroborator_addr = if family == "tu10x" { 0x0040_9664 } else { regs::FEAT_SS0 };
    match gpu.pci.bar_try_read_u32(gpu.bar, corroborator_addr as u64) {
        None => {}
        Some(v) if v == !0u32 => {
            info!(
                "claim: FEAT block corroborator (SS0 at {corroborator_addr:#010x}) dead (all-ones/transport fail) - the PLM is never consulted"
            );
            return Err(TxError::EClaimUnreadable);
        }
        _ => {}
    }
    // read32() rejects all ones, which is precisely the OPEN value here —
    // the EALREADY branch was dead for open PLMs (PR #65). The transport
    // failure case is preserved separately from valid register contents.
    use crate::claim_plm::{classify, ClaimPlm};
    match classify(gpu.pci.bar_try_read_u32(gpu.bar, feat_plm_addr as u64)) {
        ClaimPlm::Open => {
            info!("claim: EALREADY (FEAT PLM open at {feat_plm_addr:#010x}) - already unlocked, skip (da96e3f)");
            return Err(TxError::EAlready);
        }
        ClaimPlm::Inaccessible => info!("claim: FEAT PLM BADF response - EALREADY check skipped"),
        ClaimPlm::ReadFailed => return Err(TxError::EClaimUnreadable),
        ClaimPlm::NotOpen => {}
    }
    let hi = gpu.read32(regs::WPR2_HI)?;
    if hi != 0 && !post_flr {
        if family != "tu10x" {
            info!("claim: EBUSY (WPR2 hi={hi:#x})");
            return Err(TxError::EBusy);
        }
        // Turing triage (issue #4.2): gather the evidence, log it, decide.
        let lo = gpu.read32(regs::WPR2_LO)?;
        let gfw = gpu.read32(regs::GFW_BOOT)?;
        let bsi = gpu.read32(regs::BSI_SCRATCH15)?;
        let handoff = bsi >> 29 & 7;
        let cpuctl = gpu.read32(regs::GFW_CPUCTL)?;
        info!(
            "claim: Turing WPR2-up triage: lo={lo:#x} hi={hi:#x} GFW_BOOT={gfw:#x} BSI handoff={handoff} GSP CPUCTL={cpuctl:#010x}"
        );
        // START bits (bit0 canonical, bit1 the TU106-measured alias path) -
        // either still set means a GSP image is executing on this card.
        if cpuctl & 0x3 != 0 {
            info!("claim: EBUSY (GSP CPUCTL shows a started GSP session, CPUCTL={cpuctl:#x}) - a driver-era GSP may still hold the card; power off first");
            return Err(TxError::EBusy);
        }
        if gfw == regs::GFW_BOOT_COMPLETED {
            info!("claim: WPR2 up at entry, firmware-window state (GFW completed, GSP idle; BSI handoff={handoff}/DONE=3, logged not gated) - accepted");
        } else if probe_file(ACCEPT_WARM_WPR2_MARKER) {
            info!("claim: WPR2 up at entry, GFW_BOOT={gfw:#x} not completed (warm/FLR residue shape) - accepted by operator marker {ACCEPT_WARM_WPR2_MARKER}");
        } else {
            info!("claim: EBUSY (Turing WPR2-up but GFW_BOOT={gfw:#x} not completed - warm/FLR residue, e.g. the measured stuck 0x101; BSI handoff={handoff}) - power-cycle the machine first, or drop {ACCEPT_WARM_WPR2_MARKER} on the stick to accept the warm state deliberately");
            return Err(TxError::EBusy);
        }
    } else if hi != 0 {
        if family == "tu10x" {
            info!("claim: WPR2 hi={hi:#x} up at entry (self-FLR re-provisioned - accepted)");
        } else {
            info!("claim: post-FLR WPR2 hi={hi:#x} (self-induced, accepted)");
        }
    }
    Ok(())
}

/// GFW completion wait (bounded poll; 2.05s per gfw_eval.h).
pub fn gfw_wait(gpu: &TxGpu) -> Result<(), TxError> {
    gpu.poll_us(regs::GFW_BOOT, 0xff, regs::GFW_BOOT_COMPLETED, 2_050_000, true)
}

/// C-reference host probe writes (issue #17/#62 matrix, \tx-host-probe.txt).
/// The reference's own log proves these writes change some writable bits in
/// the FEAT PLM/SS block, so it cannot be reduced to a read-only check; the
/// side effects are the experiment. PLM ends at 0xfffffffe (not
/// 0xffffffff), so the upstream Turing EALREADY guard cannot false-trigger
/// on a later run; SS0 ends at the probe pattern, not the unlock value.
fn host_probe_reference(gpu: &TxGpu) {
    const PLM: u32 = 0x0040_9650;
    const SS0: u32 = 0x0040_9664;
    const SS1: u32 = 0x0040_966c;
    info!("HOST-PROBE: \\tx-host-probe.txt present - replaying the reference write sequence (write-surface probe, side effects intended)");
    for (name, reg, v) in [("PLM", PLM, u32::MAX), ("SS0", SS0, 0x8888_8888), ("SS1", SS1, 8)] {
        gpu.write32(reg, v);
        let rb = gpu.read32(reg).unwrap_or(0xbadf_0000);
        info!("HOST-PROBE: open pass {name} <- {v:#010x} readback={rb:#010x}");
    }
    let (s0, s1) = (gpu.read32(SS0).unwrap_or(0), gpu.read32(SS1).unwrap_or(0));
    if s0 == 0x8888_8888 && s1 == 8 {
        info!("HOST-PROBE: SS already at unlock values after the open pass - write surface reachable");
    }
    for (name, reg, v) in [
        ("PLM", PLM, 0u32),
        ("PLM", PLM, 0xffff_fffe),
        ("SS0", SS0, 0x1111_1111),
        ("SS1", SS1, 0),
    ] {
        gpu.write32(reg, v);
        let rb = gpu.read32(reg).unwrap_or(0xbadf_0000);
        info!("HOST-PROBE: settle pass {name} <- {v:#010x} readback={rb:#010x}");
    }
}

/// C-reference PTIMER seed (issue #17/#62 matrix, \tx-ptimer-seed.txt). PLM
/// anti-replay validation needs sane GPU time; the reference seeds from the
/// RTC but OMITS the day-of-month field ("month start + h:m:s"). The quirk
/// is preserved verbatim to reproduce the proven run - this is a reference
/// replay, not a precise wall clock.
fn ptimer_seed_reference(gpu: &TxGpu) {
    let Some(t) = uefi::runtime::get_time().ok() else {
        info!("PTIMER-SEED: RTC unavailable - skipped");
        return;
    };
    let year = u64::from(t.year().max(1970));
    let month = u64::from(t.month().clamp(1, 12));
    let leap = |y: u64| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mut days = 0u64;
    for y in 1970..year {
        days += 365 + u64::from(leap(y));
    }
    for m in 1..month {
        days += [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][(m - 1) as usize];
        if m == 2 && leap(year) {
            days += 1;
        }
    }
    let ns = (days * 86400
        + u64::from(t.hour()) * 3600
        + u64::from(t.minute()) * 60
        + u64::from(t.second()))
        * 1_000_000_000;
    gpu.write32(0x0000_9410, (ns >> 32) as u32);
    gpu.write32(0x0000_9400, ns as u32);
    info!("PTIMER-SEED: reference seed={ns:#x} (month-start + h:m:s, day field omitted per the reference)");
}

/// falcon_hs full port (unlock_610.c 347-432): prevalidate -> reset ->
/// FBIF physical mode -> code/data DMAs -> signature/selection regs ->
/// boot vector + mailboxes -> start (CPUCTL alias path aware) -> wait.
/// `image` is the DMA (device) address of the firmware image.
pub fn falcon_hs(
    gpu: &TxGpu,
    base: u32,
    fbif: u32,
    falcon2: u32,
    code_off: u32,
    code_size: u32,
    code_va: u32,
    code_pa: u32,
    data_off: u32,
    data_size: u32,
    data_pa: u32,
    sig_dmem: u32,
    engine_mask: u32,
    ucode_id: u32,
    image: u64,
    mailbox0: u32,
    mailbox1: u32,
    wait_plm: bool,
) -> Result<(), TxError> {
    // Transfer sizes are rounded up to the 256B Falcon DMA granularity
    // (the image tail is zero-filled by the caller, so the extra bytes
    // transfer zeros - same semantics as the reference's rounded image).
    let code_size = (code_size + 255) & !255;
    let data_size = (data_size + 255) & !255;
    info!("Falcon HS: image={image:#x} code={code_size:#x}@va={code_va:#x} data={data_size:#x}@off={data_off:#x} sig_dmem={sig_dmem:#x} eng={engine_mask:#x} ucode={ucode_id}");
    let code_src = hs_prevalidate(image, code_off, code_size, code_va, code_pa, data_off, data_size, data_pa)?;
    info!("Falcon HS: prevalidate ok (code_src={code_src:#x})");
    falcon_reset(gpu, base, fbif, falcon2, true)?;

    // FBIF: physical addressing mode.
    let mut trans = gpu.read32(fbif + 0x24)?;
    trans |= 1 << 7;
    gpu.write32(fbif + 0x24, trans);
    gpu.write32(base + 0x10c, 0);
    // Read-back (constitution 1.1): UC_BLOCK_ON_FIFO must actually clear.
    let block_on_fifo = gpu.read32(base + 0x10c).unwrap_or(0);
    let caps = gpu.pci.bar_read_u32(gpu.bar, (base + 0x108) as u64);
    info!(
        "Falcon HS: BLOCK_ON_FIFO after write0 = {block_on_fifo:#010x}; CAPS={caps:#010x} (code={}K data={}K fifo={}K slots={})",
        (caps >> 0 & 0x1ff) * 256 / 1024,
        (caps >> 9 & 0x1ff) * 256 / 1024,
        (caps >> 18 & 0xff) * 256 / 1024,
        caps >> 26 & 0x3f
    );
    let mut trans = gpu.read32(fbif)?;
    trans = (trans & !7u32) | 5;
    gpu.write32(fbif, trans);
    // FBIF write-back verification (constitution 1.1): if the FBIF domain
    // is gated (BSI handoff != DONE), these writes may be silently dropped
    // and every DMA fetch would be rejected at translation = the queue
    // fills but nothing is consumed.
    let ctl_rb = gpu.read32(fbif + 0x24);
    let trans_rb = gpu.read32(fbif)?;
    let dmactl_rb = gpu.read32(base + 0x10c).unwrap_or(0);
    info!(
        "Falcon HS: FBIF verify - CTL(+24)={:010x} TRANSCFG(0)={:010x} DMACTL={:010x}",
        ctl_rb.unwrap_or(0),
        trans_rb,
        dmactl_rb
    );
    // Post-reset IMEM state probes (read-only): page-tag/scrub hypothesis
    // for the 8KB DMA stall. IMEMC0 carries SECRET(bit28)/BLK(bits23:8);
    // a 250ms settle lets any asynchronous scrub finish before DMA.
    boot::stall(core::time::Duration::from_millis(250));
    let imemc0 = gpu.pci.bar_read_u32(gpu.bar, (base + 0x180) as u64);
    let hw = gpu.read32(base + 0xf4)?;
    let bsi = gpu.pci.bar_read_u32(gpu.bar, regs::BSI_SCRATCH15.into());
    info!(
        "Falcon HS: post-reset IMEMC0={imemc0:#010x} (secret={} scrub_active={} blk={}) HWCFG={hw:#010x} BSI_SCRATCH15={bsi:#010x} (scrubber_handoff={})",
        imemc0 >> 28 & 1,
        imemc0 >> 31 & 1,
        imemc0 >> 8 & 0xffff,
        bsi >> 29 & 7,
    );

    falcon_dma(gpu, base, code_pa, code_va, code_src, code_size, regs::DMA_CMD_IMEM)?;
    if data_size != 0 {
        falcon_dma(gpu, base, data_pa, 0, image + data_off as u64, data_size, regs::DMA_CMD_DMEM)?;
    }

    // Signature + engine selection, then boot vector and mailboxes.
    gpu.write32(falcon2 + 0x210, sig_dmem);
    gpu.write32(falcon2 + 0x19c, engine_mask);
    gpu.write32(falcon2 + 0x198, ucode_id & 0xff);
    gpu.write32(falcon2 + 0x180, 1);
    gpu.write32(base + 0x104, code_va);
    gpu.write32(base + 0x40, mailbox0);
    gpu.write32(base + 0x44, mailbox1);

    let cpuctl = gpu.read32(base + 0x100)?;
    info!("Falcon start: base={base:#x} cpuctl={cpuctl:#x} code_va={code_va:#x} sig_dmem={sig_dmem:#x} engine_mask={engine_mask:#x} ucode={ucode_id} mailbox={mailbox1:#x}:{mailbox0:#x}");
    if cpuctl & (1 << 6) != 0 {
        gpu.write32(base + 0x130, 1 << 1); // CPUCTL alias start path
    } else {
        gpu.write32(base + 0x100, 1 << 1);
    }

    if wait_plm {
        // ~5s budget at the poll step rate; the PLM cold->open transition
        // is the payload-execution proof.
        let max_iters = 5_000_000 / 75 + 1;
        for _ in 0..max_iters {
            let engine = gpu.pci.bar_read_u32(gpu.bar, (base + 0x100) as u64);
            if engine == !0u32 || engine & 0xffff_0000 == 0xbadf_0000 {
                return Err(TxError::ENodev);
            }
            let plm = gpu.pci.bar_read_u32(gpu.bar, regs::FEAT_PLM as u64);
            if plm == !0u32 {
                return Ok(()); // cold entry was closed: this transition proves execution
            }
            if engine & (1 << 4) != 0 {
                info!("early PLM failed: cpuctl={engine:#x} plm={plm:#x}");
                return Err(TxError::EIO);
            }
            stall(Duration::from_micros(75));
        }
        return Err(TxError::ETimeout { tolerated: false });
    }
    // Non-PLM wait: HALT bit.
    gpu.poll_us(base + 0x100, 1 << 4, 1 << 4, 5_000_000, true)
}

/// List every entry in the boot-volume root: names + sizes as the FIRMWARE
/// sees them (kills hidden-extension / rename / placement hypotheses in
/// one log).
pub fn list_root_entries() {

    // Directory = FileHandle; read entries RAW (RegularFile::read) and parse
    // the EFI_FILE_INFO layout ourselves: size/file_size/physical_size
    // (3x u64 = 24), 3x Time (16B each = 48), attribute u64 = 72, name
    // UCS-2 from offset 80. This bypasses uefi's entry API, whose error
    // path instantiates CStr16 helpers that reference wcslen under LTO.
    let Ok(mut fs) = boot::get_image_file_system(boot::image_handle()) else {
        info!("TX-ASSET-DIAG: get_image_file_system FAILED");
        return;
    };
    let Ok(mut dir) = fs.open_volume() else {
        info!("TX-ASSET-DIAG: open_volume FAILED");
        return;
    };
    // Directory is a newtype over RegularFile; transmute the reference to
    // reach RegularFile::read directly (same layout, repr(transparent)),
    // bypassing the entry API whose error path references wcslen under LTO.
    // Safety: Directory(RegularFile) is repr(transparent) over the same
    // FileProtocolV1 handle; read is valid on directory handles per UEFI.
    #[allow(unused_imports)]
    use uefi::proto::media::file::RegularFile;
    let raw: &mut RegularFile = unsafe { core::mem::transmute(&mut dir) };
    loop {
        let mut buf = [0u8; 512];
        let n = match raw.read(&mut buf) {
            Ok(n) => n,
            Err(e) => { info!("TX-ROOT-DIR: raw read err {:?}", e.status()); break; }
        };
        if n == 0 {
            break; // end of directory
        }
        // EFI_FILE_INFO fixed part = 80 bytes: Size(8) + FileSize(8) +
        // PhysicalSize(8) + 3x EFI_TIME(48) + Attribute(8); FileName[] is the
        // CHAR16 array that starts at 80, not at 72 (reading 72 lands on the
        // Attribute field: 0x20 = archive -> one space, then the name's NUL is
        // never reached because the byte after Attribute is the first UCS-2
        // character, so every entry logged a blank name).
        const INFO_FIXED_LEN: usize = 80;
        if n < INFO_FIXED_LEN + 2 {
            info!("TX-ROOT-DIR: short entry {n}");
            continue;
        }
        let file_size = u64::from_le_bytes(buf[8..16].try_into().unwrap());
        let mut name = [0u8; 96];
        let mut m = 0usize;
        let mut chars = 0usize;
        while INFO_FIXED_LEN + chars + 1 < n && m + 1 < name.len() {
            let u = u16::from_le_bytes([buf[INFO_FIXED_LEN + chars], buf[INFO_FIXED_LEN + chars + 1]]);
            if u == 0 { break; }
            name[m] = if (0x20..0x7f).contains(&u) { u as u8 } else { b'?' };
            m += 1;
            chars += 2;
        }
        let name = core::str::from_utf8(&name[..m]).unwrap_or("?");
        info!("TX-ROOT-DIR: {name} size={file_size}");
    }
}

/// Shared PciIo access helper for the pre-transaction FLR experiment.
/// Returns whether an FLR was actually performed (drives the WPR2
/// cold-state guard in claim_guards - issue #4.2).
fn pre_flr_experiment(handle: uefi::Handle) -> bool {
    // Opt-in via a flag file on the boot volume. The kernel reference flow
    // (and unlock_610.c) run the scrubber/FWSEC chain WITHOUT any pre-FLR -
    // and our measured post-FLR state (GFW_BOOT stuck 0x101, both chips)
    // is an environment the kernel never sees. Default OFF so the next
    // round tests the kernel-matching sequence.
    if !probe_file("\\pre-flr-on.txt") {
        info!("PRE-FLR: disabled (no \\pre-flr-on.txt) - kernel-matching flow");
        return false;
    }
    info!("PRE-FLR experiment: resetting function to clean state");
    use uefi::boot::{OpenProtocolAttributes, OpenProtocolParams};
    // Safety: GetProtocol during boot, single-threaded; the FLR runs while
    // the scoped protocol handle is alive, then the reference is dropped.
    let mut scoped = match unsafe {
        boot::open_protocol::<crate::pci::PciIo>(
            OpenProtocolParams {
                handle,
                agent: boot::image_handle(),
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
    } {
        Ok(p) => p,
        Err(_) => { info!("PRE-FLR: cannot open PciIo"); return false; }
    };
    flr_once(&mut scoped);

    // The FLR triggers the GPU's own re-boot: GFW restarts and re-provisions
    // WPR2 (measured: gfw_boot=0x101 in-progress, wpr2_hi=0xe00). WAIT for
    // that boot to settle before anything else (reference wait_post_flr_gfw).
    let mut settled = false;
    for i in 0..100 {
        let gfw = scoped.bar_read_u32(0, 0x0011_8234);
        if gfw == 0xff {
            info!("PRE-FLR: GFW re-boot completed after {} polls (100ms each)", i + 1);
            settled = true;
            break;
        }
        boot::stall(core::time::Duration::from_millis(100));
    }
    if !settled {
        info!("PRE-FLR: GFW did not re-complete within 10s - continuing anyway");
    }
    let bsi = scoped.bar_read_u32(0, regs::BSI_SCRATCH15.into());
    info!("PRE-FLR: post-settle BSI_SCRATCH15={bsi:#010x} (handoff={})", bsi >> 29 & 7);
    true
}

/// Does this path exist as a readable regular file? No size gate and no content:
/// the UI switches are zero-byte marker files by design (`\tx-require-d0.txt`
/// is documented as "drop an empty file"), so anything that treats "size 0" as
/// "absent" silently disables them. Kept separate from the asset reader, whose
/// size rules are about DMA windows, not about intent.
pub fn probe_file(path: &str) -> bool {
    let Ok(path_c) = uefi::CString16::try_from(path) else {
        return false;
    };
    use uefi::proto::media::file::{File, FileMode};
    let Ok(mut fs) = boot::get_image_file_system(boot::image_handle()) else {
        return false;
    };
    let Ok(mut dir) = fs.open_volume() else {
        return false;
    };
    dir.open(&path_c, FileMode::Read, uefi::proto::media::file::FileAttribute::empty())
        .is_ok()
}

/// Formatted variant: substitutes `{}` with `family` then reads.
pub fn read_boot_file_fmt(pattern: &str, family: &str) -> Option<alloc::vec::Vec<u8>> {
    let path = alloc::format!("{}", pattern.replacen("{}", family, 1));
    read_boot_file(&path)
}

/// Read one ASSET (never a marker) with the three states kept apart:
/// `Ok(None)` = not on the stick (the caller may legitimately run without it),
/// `Err(())` = present but unusable (empty, over the cap, short read, media
/// pulled). Collapsing that second state into `None` is how a damaged `v67-*.bin`
/// silently downgrades an unlock run to chain-replay instead of refusing.
pub fn read_asset(path: &str) -> Result<Option<alloc::vec::Vec<u8>>, ()> {
    match read_boot_file_inner(path) {
        Inner::Absent => Ok(None),
        Inner::Failed => Err(()),
        Inner::Content(v) => Ok(Some(v)),
    }
}

/// Formatted variant of [`read_asset`].
pub fn read_asset_fmt(pattern: &str, family: &str) -> Result<Option<alloc::vec::Vec<u8>>, ()> {
    let path = alloc::format!("{}", pattern.replacen("{}", family, 1));
    read_asset(&path)
}

enum Inner {
    Absent,
    Failed,
    Content(alloc::vec::Vec<u8>),
}

/// Read one file from the boot volume with per-step diagnostics.
pub fn read_boot_file(path: &str) -> Option<alloc::vec::Vec<u8>> {
    match read_boot_file_inner(path) {
        Inner::Content(v) => Some(v),
        _ => None,
    }
}

fn read_boot_file_inner(path: &str) -> Inner {
    // Runtime path: CString16::try_from (cstr16! only takes literals).
    let path_c = match uefi::CString16::try_from(path) {
        Ok(p) => p,
        Err(_) => { info!("TX-ASSET-DIAG: {path} not UCS-2 convertible"); return Inner::Failed; }
    };
    let path: &uefi::CStr16 = &path_c;
    use alloc::vec::Vec;
    use uefi::proto::media::file::{File, FileInfo, FileMode, FileAttribute};

    let mut fs = match boot::get_image_file_system(boot::image_handle()) {
        Ok(fs) => fs,
        Err(e) => { info!("TX-ASSET-DIAG: {path} fs open failed {:?}", e.status()); return Inner::Failed; }
    };
    let mut dir = match fs.open_volume() {
        Ok(d) => d,
        Err(e) => { info!("TX-ASSET-DIAG: {path} open_volume failed {:?}", e.status()); return Inner::Failed; }
    };
    let file = match dir.open(path, FileMode::Read, FileAttribute::empty()) {
        Ok(f) => f,
        Err(e) => {
            // NOT_FOUND is the one error that means "the stick does not carry it".
            if e.status() == uefi::Status::NOT_FOUND {
                info!("TX-ASSET-DIAG: {path} absent");
                return Inner::Absent;
            }
            info!("TX-ASSET-DIAG: {path} OPEN failed {:?}", e.status());
            return Inner::Failed;
        }
    };
    let mut file = match file.into_regular_file() {
        Some(f) => f,
        None => { info!("TX-ASSET-DIAG: {path} not a regular file"); return Inner::Failed; }
    };
    let size = match file.get_boxed_info::<FileInfo>() {
        Ok(info) => info.file_size() as usize,
        Err(e) => { info!("TX-ASSET-DIAG: {path} get_info failed {:?}", e.status()); return Inner::Failed; }
    };
    // An asset is never empty: the DMA copies use fixed windows, and a FAT entry
    // that reports 0 or an absurd size is a damaged stick, not a flag file.
    const MAX_ASSET_BYTES: usize = 192 << 20;
    if size == 0 || size > MAX_ASSET_BYTES {
        info!("TX-ASSET-DIAG: {path} size {size} outside 1..={MAX_ASSET_BYTES} - refusing");
        return Inner::Failed;
    }
    let mut out = Vec::with_capacity(size);
    let mut chunk = [0u8; 65536];
    let mut got = 0usize;
    while got < size {
        let want = (size - got).min(chunk.len());
        match file.read(&mut chunk[..want]) {
            Ok(0) => { info!("TX-ASSET-DIAG: {path} EOF at {got}/{size}"); break; }
            Ok(n) => { out.extend_from_slice(&chunk[..n]); got += n; }
            Err(e) => { info!("TX-ASSET-DIAG: {path} read err at {got}/{size}: {:?}", e.status()); return Inner::Failed; }
        }
    }
    if got != size {
        info!("TX-ASSET-DIAG: {path} short read {got}/{size} - refusing (damaged stick or pulled media)");
        return Inner::Failed;
    }
    info!("TX-ASSET-DIAG: {path} loaded {}/{} bytes", out.len(), size);
    Inner::Content(out)
}

/// Assets for the transaction, loaded from the boot volume (tx builds
/// carry their bundles next to the app; assets pipeline produces them).
pub struct TxAssets {
    /// PCI device-id family key ('ad102' | 'ga102' | 'tu10x').
    pub family: &'static str,
    /// Booter bundle (BTL1), e.g. booter_load_ad102.bin.
    pub booter: alloc::vec::Vec<u8>,
    /// GSP boot descriptor (gsp_boot_desc_ad102.bin, 84 bytes).
    pub boot_desc: Option<alloc::vec::Vec<u8>>,
    /// GSP bootloader image (gsp_boot_ad102.bin) — the boot image.
    pub gsp_boot: alloc::vec::Vec<u8>,
    /// GSP-RM firmware image (.fwimage of gsp_ga10x.bin) — the radix payload.
    pub gsp_image: alloc::vec::Vec<u8>,
    /// Stock signature section (.fwsignature_ad10x, 4096 bytes).
    pub stock_sig: alloc::vec::Vec<u8>,
    /// Optional V67 payload (present = four-write unlock; absent = chain replay).
    pub v67: Option<alloc::vec::Vec<u8>>,
    /// Generic falcon BL (sec2_bl_tu10x.bin, 768B) - Turing FWSEC
    /// BOOT_WITH_LOADER only; the GA10x/Ada HS path ignores it.
    pub bl: Option<alloc::vec::Vec<u8>>,
}

/// Asset size validation BEFORE any GPU interaction (issue #4.3): the DMA
/// copies use fixed windows (V67_SIZE into sig_dma, 4096-byte stock_dma),
/// so a damaged stick must be refused here, not over/under-copied on the
/// device. Runs before pm_preflight/detach - a validation failure means
/// nothing was touched and there is nothing to unwind.
pub fn validate_asset_sizes(assets: &TxAssets) -> Result<(), TxError> {
    use crate::tx_booter as tb;
    if let Some(v67) = &assets.v67 {
        if v67.len() != tb::V67_SIZE {
            info!(
                "TX: v67 payload is {} bytes, want exactly {} (0xfa00) - refusing to run (truncated or damaged stick?)",
                v67.len(),
                tb::V67_SIZE
            );
            return Err(TxError::EInval);
        }
    }
    if assets.stock_sig.is_empty() || assets.stock_sig.len() > 4096 {
        info!(
            "TX: fwsignature_{}.bin is {} bytes, want 1..=4096 (the stock_dma window) - refusing to run (damaged stick?)",
            assets.family,
            assets.stock_sig.len()
        );
        return Err(TxError::EInval);
    }
    Ok(())
}

/// Transaction, phases mirror sim::transaction and the C reference:
/// guards -> gfw -> layout -> FWSEC ignition (WPR2) -> [booter fire ->
/// triad -> selectors -> signature restore -> FLR + post-FLR].
/// The bracketed tail lands with the booter-publication tranche.
pub fn run_transaction(gpu: &TxGpu, assets: &TxAssets, pci_handle: uefi::Handle) -> Result<(), TxError> {
    info!("TX: BEGIN (phases: detach claim pre-flr scrub gfw layout fwsec booter verify selectors restore flr)");

    // Experimental override (\tx-force-continue.txt): on a booter-stage
    // failure, continue the unwind with a selector probe instead of
    // aborting. Default OFF; see the FORCE-CONTINUE banner at the failure
    // site for what the experiment can and cannot show.
    let force_continue = probe_file("\\tx-force-continue.txt");
    if force_continue {
        info!("TX: force-continue marker present - booter-stage failures fall through to the selector probe");
    }

    // Above-4GB staging gate (issues #14/#16): Turing booters exit 0x91 on
    // some cards when the WprMeta lands below 4GB; the high path is the
    // driver-exact configuration. Gated to 'tu10x' so GA10x/Ada keep the
    // allocation path they were field-verified with. \tx-low-staging.txt
    // reverts even tu10x to the legacy path (A/B testing).
    let high_staging = assets.family == "tu10x" && !probe_file("\\tx-low-staging.txt");
    crate::pci::set_high_staging(high_staging);
    info!("TX: >4GB staging gate: {high_staging} (family {})", assets.family);

    // Resume-entry preflight FIRST, before anything is detached or written:
    // a non-D0 entry means the firmware did not POST this card (sleep/resume
    // or fast-startup handoff), which changes what the Windows driver will
    // do with the state we leave behind.
    // \tx-no-pm-preflight.txt skips it for the #17/#62 ablation axis: the
    // guarded C-parity build compiled it out (travis overlay T2, Phase B) -
    // one PMCSR force-D0 config write is the only behavioral difference on a
    // cold D0 entry.
    if probe_file("\\tx-no-pm-preflight.txt") {
        info!("TX: \\tx-no-pm-preflight.txt present - PM preflight skipped (guarded-overlay parity, one PMCSR write)");
    } else {
        crate::tx_detach::pm_preflight(gpu.pci).map_err(|e| {
            info!("TX: PM preflight: {e}");
            TxError::EWarm
        })?;
    }

    // TOTORONG craft: all console + vendor drivers off the device, PciIo
    // exclusively claimed, IO|MEM|BME enabled, config baseline captured.
    // (Black screen starts here by design; the log file keeps recording.)
    if let Err(e) = crate::tx_detach::detach_and_claim(pci_handle) {
        info!("TX: detach+claim FAILED: {e}");
        return Err(TxError::ENodev);
    }
    info!("TX: detach+claim ok");

    // PRE-FLR (reference never does this, but it also never runs in a
    // firmware window with BSI handoff=1): force the card through a full
    // function reset. The FLR triggers the GPU's own re-boot (GFW restarts,
    // WPR2 re-provisioned) - pre_flr_experiment waits for that boot to
    // settle and re-reads the BSI handoff.
    let pre_flr_did_flr = pre_flr_experiment(pci_handle);

    // WPR2 cold-state guard (issue #4.2): only a pre-FLR WE performed makes
    // an up WPR2 "self-induced" - anything else goes through the guards.
    // Turing no longer blanket-accepts a family-wide bypass: WPR2-up at
    // entry is triaged inside claim_guards on firmware-window evidence
    // (GFW/BSI proof accepts, warm-driver residue refuses unless
    // \tx-accept-warm-wpr2.txt opts in).
    let post_flr = pre_flr_did_flr;
    claim_guards(gpu, assets.family, post_flr)?;

    // Differential authority map while both engines sit in the as-booted
    // state: localizes the SEC2 start-authority gate before any reset.
    authority_scan(gpu);

    // Issue #17/#62 matrix (Turing, marker-gated, default OFF): two
    // environment steps the C reference performs and the plain path never
    // does. \tx-host-probe.txt replays its host probe writes (its own log
    // proves they change some PLM/SS writable bits - NOT a read-only
    // diagnostic); \tx-ptimer-seed.txt seeds PTIMER from the RTC (PLM
    // anti-replay validation needs sane GPU time; the reference omits the
    // day-of-month field - preserved verbatim, not a precise wall clock).
    if assets.family == "tu10x" {
        if read_boot_file("\\tx-host-probe.txt").is_some() {
            host_probe_reference(gpu);
        }
        if read_boot_file("\\tx-ptimer-seed.txt").is_some() {
            ptimer_seed_reference(gpu);
        }
    }

    // The kernel AD102 driver runs the SEC2 scrubber before FWSEC, but the
    // community unlock chains (unlock_610.c "GA102 has no scrubber" + every
    // gpu-unlock-ada card through 28e1) boot FWSEC/booter WITHOUT one - and
    // their entry state equals ours (GFW done, BSI handoff=1). Attempt it,
    // log the verdict, proceed either way: whether the chain crosses the
    // gated state decides the next fix round. Turing: the proven cmp50hx
    // chain has NO scrubber step and the AD10x ucode EIO'd on TU106
    // (first fire 2026-10-27) - don't fire a foreign binary at it.
    if assets.family == "tu10x" {
        info!("TX-tu10x: scrubber step skipped (not part of the proven Turing chain; BSI handoff state logged above)");
    } else if let Err(e) = crate::tx_fwsec::scrubber_if_needed(gpu) {
        info!(
            "TX: scrubber did not complete ({e:?}) - proceeding without it (community chain shape)"
        );
    }

    let boot_size = assets.gsp_boot.len() as u64;
    // layout_610.c in.image_size = the GSP ELF payload (the radix image,
    // 84 MB on AD10x), NOT the boot loader. Passing the boot size here
    // sized the gsp region for 36 KiB and wpr_meta_pack then (correctly)
    // refused: 84 MB > boot_offset - gsp_offset (EInval, 2026-10-26 log).
    let gsp_image_size = assets.gsp_image.len() as u64;
    let layout = crate::tx_fwsec::layout_from_registers(gpu, assets.family, gsp_image_size, boot_size)?;
    info!("TX: layout ok (frts={:#x})", layout.frts_offset);

    if assets.family == "tu10x" {
        // Turing has no AD10x scrubber step in the proven chain (cmp50hx
        // runs FWSEC/booter without one), and the AD10x scrubber ucode is a
        // different binary that EIO'd on TU106 (first fire, 2026-10-27).
        // WPR2-up entry is the MAINLINE path there (POST or FLR reboot
        // provisions it): skip the fire instead of refusing.
        // Entry state is logged in full (matrix A4 gap 5): the ran/skipped
        // split and the cold/warm cycle are per-machine constants and every
        // failing-machine log needs this line for the cross-machine table.
        let wpr_hi = gpu.read32(regs::WPR2_HI)?;
        let wpr_lo = gpu.read32(regs::WPR2_LO)?;
        if wpr_hi >> 4 != 0 {
            info!(
                "TX-tu10x: entry WPR2 lo={wpr_lo:#x} hi={wpr_hi:#x} (already up, POST/FLR provisioned) - FWSEC fire SKIPPED (cmp50hx mainline path) [fwsec=skipped]"
            );
        } else {
            info!(
                "TX-tu10x: entry WPR2 lo={wpr_lo:#x} hi={wpr_hi:#x} (cold) - FWSEC will be fired by the loader [fwsec=ran]"
            );
            let bl = match &assets.bl {
                Some(b) => b,
                None => {
                    info!("TX: tu10x requires \\sec2_bl_tu10x.bin (generic falcon BL) for the WITH_LOADER FWSEC fire");
                    return Err(TxError::ENodev);
                }
            };
            crate::tx_fwsec::fwsec_ignite_tu10x(gpu, &layout, bl)?;
        }
    } else {
        crate::tx_fwsec::fwsec_ignite(gpu, &layout)?;
    }
    info!("TX: fwsec ok (WPR2 verified)");

    use crate::tx_booter as tb;

    // Boot descriptor + booter bundle validation (per-family pinned
    // geometry; family comes from the PCI device-id mapping in tx_phase).
    let boot_image_len = assets.gsp_boot.len() as u32;
    let desc = assets.boot_desc.as_ref().ok_or(TxError::EInval)?;
    let mut boot = tb::boot_desc_parse(desc, boot_image_len)?;
    if assets.family == "tu10x" {
        // cmp50hx-proven BL topology (live-driver WPR meta dump 2026-08-21 +
        // their proven runs): first 0x6000 bytes of the GspRmBoot image with
        // code@0x1800 / data@0x800 / manifest@0x0. Our v4-desc 4KB image has
        // never been consumed by a booter - use the proven input set.
        if boot_image_len < 0x6000 {
            info!("TX: tu10x gsp_boot must be the 0x6000-byte proven GspRmBoot image");
            return Err(TxError::EInval);
        }
        boot.image_size = 0x6000;
        boot.code_offset = 0x1800;
        boot.data_offset = 0x800;
        boot.manifest_offset = 0x0;
        info!("TX: tu10x BL topology pinned: size 0x6000 code@0x1800 data@0x800 manifest@0x0");
    }
    let (hdr, booter_image, booter_sigs) = tb::load_booter(&assets.booter, assets.family)?;
    info!("TX: booter bundle ok (image {} sigs {})", booter_image.len(), booter_sigs.len());

    // DMA allocations: booter image, v67-or-stock signature, meta, stock, boot image.
    let payload_mode = assets.v67.is_some();
    let sig_size = if payload_mode { tb::V67_SIZE } else { assets.stock_sig.len() };
    let booter_len = booter_image.len();
    let mut booter_dma = gpu.pci.dma_alloc(booter_len).map_err(|_| TxError::ENodev)?;
    let mut sig_dma = gpu.pci.dma_alloc(sig_size.max(4096)).map_err(|_| TxError::ENodev)?;
    let mut meta_dma = gpu.pci.dma_alloc(tb::WPR_META_SIZE).map_err(|_| TxError::ENodev)?;
    let mut stock_dma = gpu.pci.dma_alloc(4096).map_err(|_| TxError::ENodev)?;
    let mut boot_dma = gpu.pci.dma_alloc(assets.gsp_boot.len()).map_err(|_| TxError::ENodev)?;

    unsafe {
        // The IMAGE always carries the STOCK signature: the BROM validates
        // the booter before it runs. Payload mode aims the WprMeta
        // signature DMA at the V67 blob instead - the booter's own
        // (unbounded) signature copy overflows its DMEM stack: that IS the
        // exploit (patching the image with V67 is impossible anyway -
        // 0xFA00 > any booter image, EInval).
        core::ptr::copy_nonoverlapping(booter_image.as_ptr(), booter_dma.host, booter_len);
        {
            // Turing FUSE registers are PLM-locked at PL0 (badf5040) - and
            // the Turing BOOT_DIRECT bundle carries exactly ONE signature
            // (SIG_PROD, 16B AES at patch_loc), so the per-fuse slot
            // selection is moot; index 0 is the only choice (the community
            // path patches the literal SIG_PROD bytes likewise). Fourth
            // TU106 fire failed exactly on this read (2026-10-27).
            let fuse_ver = if hdr.num_sigs == 1 {
                0
            } else {
                let fuse_raw = gpu.read32(0x0082_4140 + (hdr.ucode_id - 1) * 4)?;
                crate::tx_fwsec::fuse_version(fuse_raw)
            };
            info!("TX: booter sig fuse_ver={fuse_ver} (payload_mode={payload_mode})");
            let image = core::slice::from_raw_parts_mut(booter_dma.host, booter_len);
            tb::booter_patch_stock(image, &hdr, booter_sigs, fuse_ver)?;
        }
        match &assets.v67 {
            Some(v67) => core::ptr::copy_nonoverlapping(v67.as_ptr(), sig_dma.host, tb::V67_SIZE),
            None => core::ptr::copy_nonoverlapping(assets.stock_sig.as_ptr(), sig_dma.host, assets.stock_sig.len()),
        }
        core::ptr::copy_nonoverlapping(assets.stock_sig.as_ptr(), stock_dma.host, assets.stock_sig.len());
        core::ptr::copy_nonoverlapping(assets.gsp_boot.as_ptr(), boot_dma.host, assets.gsp_boot.len());
    }

    // Radix over the GSP-RM image (84 MiB) — one big contiguous DMA block.
    let radix = tb::radix_build(gpu, &assets.gsp_image)?;

    // WprMeta publication (first signature call) then fire.
    {
        let meta = unsafe { core::slice::from_raw_parts_mut(meta_dma.host, tb::WPR_META_SIZE) };
        tb::wpr_meta_pack(meta, &layout, assets.family, 1 << 20,
                          sig_dma.dev, sig_size as u64, radix.level0,
                          assets.gsp_image.len() as u64, boot_dma.dev, &boot)?;
    }
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    info!("TX: WprMeta published (payload_mode={payload_mode})");

    // Bus master on the bridges above the GPU, with the DMA path logged before
    // and after. The guard lives until run_transaction returns, early error
    // returns included, then clears what it set.
    // \tx-no-bridge-master.txt skips the bridge writes (guarded-overlay parity
    // T3): on boards whose bridges already carry BME the enable is a no-op
    // anyway - the marker exists to rule the write itself out on boards where
    // the log above shows it actually set a bit.
    // The guard is hoisted out of the branch so it lives until run_transaction
    // returns, as the comment above promises - bound inside the else block it
    // would drop at the closing brace and clear BME again before the booter
    // fires (booter 0x89 family, PR #87).
    crate::pci::log_dma_path("before-bridge-bme", gpu.pci);
    let _bridge_master;
    if probe_file("\\tx-no-bridge-master.txt") {
        info!("TX: \\tx-no-bridge-master.txt present - BridgeMaster::enable skipped (guarded-overlay parity)");
        _bridge_master = None;
    } else {
        _bridge_master = Some(crate::pci::BridgeMaster::enable(gpu.pci));
    }
    crate::pci::log_dma_path("after-bridge-bme", gpu.pci);

    // VRAM fallback staging (\tx-meta-in-vram.txt): put everything the booter
    // DMA-reads (meta, signature, radix + RM image, boot image) into the GPU's
    // own framebuffer at FB 16 MiB and switch its FBIF aperture to LOCAL_FB,
    // so every booter read stays on-device - no PCIe transaction, immune to
    // bridges without bus master, IOMMUs, and any other platform DMA layer.
    // One BAR1 bulk copy per stage, magic spot-readback, sysmem fallback.
    const VRAM_BASE: u64 = 0x0100_0000;
    const OFF_SIG: u64 = 0x1000;
    const OFF_STOCK: u64 = 0x1_1000;
    const OFF_BOOT: u64 = 0x1_2000;
    const OFF_RADIX: u64 = 0x2_0000;
    let mut meta_addr = meta_dma.dev;
    let mut vram_fb = false;
    if probe_file("\\tx-meta-in-vram.txt") {
        // Retarget the radix tables in place: every entry moves by the same
        // delta (tables and leaves are copied as one block).
        let leaves = assets.gsp_image.len().div_ceil(4096);
        let sys_leaf0 = radix.dma.dev + ((radix.pages - leaves) * 4096) as u64;
        let vram_leaf0 = VRAM_BASE + OFF_RADIX + ((radix.pages - leaves) * 4096) as u64;
        let delta = vram_leaf0 as i64 - sys_leaf0 as i64;
        {
            let n = (radix.pages - leaves) * 4096;
            let tbl = unsafe { core::slice::from_raw_parts_mut(radix.dma.host, n) };
            for e in 0..n / 8 {
                let at = e * 8;
                let v = u64::from_le_bytes(tbl[at..at + 8].try_into().unwrap());
                if v != 0 {
                    tbl[at..at + 8].copy_from_slice(&((v as i64 + delta) as u64).to_le_bytes());
                }
            }
        }
        {
            let meta = unsafe { core::slice::from_raw_parts_mut(meta_dma.host, tb::WPR_META_SIZE) };
            tb::wpr_meta_pack(meta, &layout, assets.family, 1 << 20,
                              VRAM_BASE + OFF_SIG, sig_size as u64, VRAM_BASE + OFF_RADIX,
                              assets.gsp_image.len() as u64, VRAM_BASE + OFF_BOOT, &boot)?;
        }
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        let stages: [(u64, u64, u64); 5] = [
            (VRAM_BASE, meta_dma.host as u64, tb::WPR_META_SIZE as u64),
            (VRAM_BASE + OFF_SIG, sig_dma.host as u64, sig_size as u64),
            (VRAM_BASE + OFF_STOCK, stock_dma.host as u64, assets.stock_sig.len() as u64),
            (VRAM_BASE + OFF_BOOT, boot_dma.host as u64, assets.gsp_boot.len() as u64),
            (VRAM_BASE + OFF_RADIX, radix.dma.host as u64, (radix.pages * 4096) as u64),
        ];
        let mut written = true;
        for (dst, host, len) in stages {
            let buf = unsafe { core::slice::from_raw_parts(host as *const u8, len as usize) };
            if gpu.pci.bar_write_buf(1, dst, buf) != uefi::Status::SUCCESS {
                written = false;
                break;
            }
        }
        let (lo, hi) = (gpu.pci.bar_read_u32(1, VRAM_BASE), gpu.pci.bar_read_u32(1, VRAM_BASE + 4));
        if written && lo == 0x371a_60b3 && hi == 0xdc3a_ae21 {
            info!("TX-VRAM: fallback staged at FB {VRAM_BASE:#x} (meta/sig/stock/boot/radix+image, magic readback ok) - booter aperture switches to LOCAL_FB, its reads never cross PCIe");
            meta_addr = VRAM_BASE;
            vram_fb = true;
        } else {
            info!("TX-VRAM: staging failed (write status or magic readback {lo:#010x} {hi:#010x}) - staying on sysmem meta at {:#x} (the bridge BME fix covers the platform path)", meta_dma.dev);
        }
    }

    // Portal placement needs the HOST pointer to the patched image.
    let booter_view = unsafe { core::slice::from_raw_parts(booter_dma.host, booter_len) };
    // Turing boots the booter DIRECT (BOOT_DIRECT, bBootFromHs=FALSE): the
    // whole image is port-loaded NS+SEC+DMEM with BOOTVEC=0; the GA10x/Ada
    // BootFromHs path loads only the secure segment through BROM params.
    if assets.family == "tu10x" {
        if read_boot_file("\\tx-sec2-c-load.txt").is_some() {
            // Issue #17/#62 matrix: full C-reference fire (no reset, FBIF
            // CTL bit7 without TRANSCFG rewrite, 3s quiet window). Default
            // OFF - the field-proven shape above stays the default.
            info!("TX-tu10x: \\tx-sec2-c-load.txt present - C-reference SEC2 fire (no reset, quiet window)");
            tb::booter_fire_direct_c(gpu, &hdr, booter_view, meta_addr)?;
        } else {
            tb::booter_fire_direct(gpu, &hdr, booter_view, meta_addr)?;
        }
    } else {
        // Canonical wait split (unlock_610.c falcon_hs): the GA102 V67's
        // tail is the fill self-loop - the booter never halts and writes no
        // marker, so payload mode there waits on the PLM flip (HALT during
        // the wait = failure). Ada payload halts with its marker (M4);
        // replay always halts. Issue #6: the GA106M run flipped the PLM but
        // died on this poll when it waited for HALT unconditionally.
        let wait = if payload_mode && assets.family == "ga102" {
            tb::BooterWait::PlmFlip
        } else {
            tb::BooterWait::Halt
        };
        tb::booter_fire(gpu, &hdr, booter_view, meta_addr, vram_fb, wait)?;
    }
    if meta_addr != meta_dma.dev {
        let lo = gpu.pci.bar_read_u32(1, meta_addr);
        let hi = gpu.pci.bar_read_u32(1, meta_addr + 4);
        info!("TX-VRAM: post-fire BAR1 readback: {lo:#010x} {hi:#010x} (want 0x371a60b3 0xdc3aae21 - the booter only reads this page, so intact = nothing stomped it)");
    }
    info!("TX: booter fired");

    let meta_view = unsafe { core::slice::from_raw_parts(meta_dma.host, tb::WPR_META_SIZE) };
    let booter_ok = if assets.family == "tu10x" {
        tb::verify_execution_tu10x(gpu, payload_mode, meta_view).is_ok()
    } else {
        tb::verify_execution(gpu, assets.family, payload_mode, meta_view).is_ok()
    };
    if !booter_ok && !force_continue {
        // Canonical failure-path invariant (unlock_610.c restores the stock
        // signature before the unwind; -EPROTO otherwise): never leave a
        // V67-published meta page behind. Host-memory hygiene - the FLR in
        // the main.rs unwind kills the only consumer - but keep the shape.
        let meta = unsafe { core::slice::from_raw_parts_mut(meta_dma.host, tb::WPR_META_SIZE) };
        tb::signature_restore(meta, stock_dma.dev, assets.stock_sig.len() as u64);
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        info!("TX: stock signature restored (failure-path unwind)");
        return Err(TxError::EIO);
    }
    if booter_ok {
        info!("TX: execution verified");
    } else {
        // FORCE-CONTINUE theory of the experiment: the payload ROP sits
        // strictly downstream of wpr_desc_validate (validate meta -> THEN
        // signature DMA -> stack smash -> four writes), so a 0x89 means the
        // V67 blob was never DMA'd and no unlock is reachable on this run.
        // What continues is pure observation: two selector writes under a
        // locked PLM, expected to be silently dropped (readback = fuse
        // values), then the same FLR + handoff unwind as a plain failure.
        info!("TX: FORCE-CONTINUE: booter failed but the marker file demands the unwind - selector probe under locked PLM follows (expected readback: fuse values unchanged; the payload ROP never ran)");
    }

    if payload_mode {
        if booter_ok {
            // Isolation experiment (\tu10x-no-ss.txt): PLM open ONLY, no SS
            // writes - separates "PLM open" from "the 0x88888888/0x8 values"
            // as the Code-43 variable. Turing SS0/SS1 live at 0x409664/6c,
            // NOT the GA10x/Ada 0x82381c/820 (cmp50hx lines 294-296).
            let no_ss = assets.family == "tu10x"
                && crate::tx::probe_file("\\tu10x-no-ss.txt");
            let (ss0_at, ss1_at) = if assets.family == "tu10x" {
                (tb::TU10X_FEAT_SS0, tb::TU10X_FEAT_SS1)
            } else {
                (0x0082_381c, 0x0082_3820)
            };
            if no_ss {
                info!("SELECTOR RESULT: SS writes SKIPPED (tu10x-no-ss isolation marker) - PLM open only");
            }
            if !no_ss {
                tb::write_selector(gpu, "SS0", ss0_at, tb::SS0_UNLOCK)?;
                tb::write_selector(gpu, "SS1", ss1_at, tb::SS1_UNLOCK)?;
                info!("SELECTOR RESULT: SS0={:#x} SS1={:#x}", tb::SS0_UNLOCK, tb::SS1_UNLOCK);
            }
            // Turing: with the payload's PLM open, sweep the real FEAT block
            // - the factory clamp state (open question #10) is readable here
            // for the first time, BEFORE the FLR wipes the override inputs.
            if assets.family == "tu10x" {
                for off in (0x0040_9600..0x0040_9700).step_by(4) {
                    let v = gpu.pci.bar_read_u32(gpu.bar, off as u64);
                    if v != 0 {
                        info!("TU10X FEAT sweep 0x{off:06x} = {v:#010x}");
                    }
                }
            }
            // Stock signature restore BEFORE FLR. In VRAM mode the booter
            // consumed the VRAM copy, so repack with the VRAM stock address
            // and re-publish the page over BAR1.
            let meta = unsafe { core::slice::from_raw_parts_mut(meta_dma.host, tb::WPR_META_SIZE) };
            if vram_fb {
                tb::signature_restore(meta, VRAM_BASE + OFF_STOCK, assets.stock_sig.len() as u64);
                core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
                let buf = unsafe { core::slice::from_raw_parts(meta_dma.host, tb::WPR_META_SIZE) };
                let _ = gpu.pci.bar_write_buf(1, meta_addr, buf);
                info!("TX: stock signature restored (VRAM meta page re-published)");
            } else {
                tb::signature_restore(meta, stock_dma.dev, assets.stock_sig.len() as u64);
                core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
                info!("TX: stock signature restored");
    // TU10x: RE-LOCK the FEAT block PLM before handoff. The 13th/14th fires
    // proved the Windows Turing driver dies with 0x409650 left at 0xffffffff
    // (Run A/B), and runs fine with it locked (Run C) - the PLM is only the
    // WRITE gate; the SS override values are the payload. Open -> write ->
    // close, then read back: if SS0/SS1 keep the unlock values behind the
    // re-locked gate, the unlock ships protected; if they revert, the
    // override is gate-bound and the Turing unlock needs the open state
    // (which the driver rejects - the CMP references are headless cards
    // where no display driver ever re-validates this block).
    if assets.family == "tu10x" && booter_ok {
        gpu.pci.bar_write_u32(0, 0x0040_9650 as u64, 0xffff_ff8f);
        let plm_rb = gpu.pci.bar_read_u32(0, 0x0040_9650 as u64);
        let ss0_rb = gpu.pci.bar_read_u32(0, 0x0040_9664 as u64);
        let ss1_rb = gpu.pci.bar_read_u32(0, 0x0040_966c as u64);
        // Once the payload opened the block, bits 5:6 resist host clears:
        // 0xffffffef is the proven handoff end state (TU106 and TU104
        // identical); a full 0xffffff8f readback would be the surprise.
        let plm_note = match plm_rb {
            0xffff_ffef => "known-good (bits 5:6 sticky-open)",
            0xffff_ff8f => "fully closed",
            _ => "UNEXPECTED",
        };
        info!(
            "TU10X PLM relock: plm={plm_rb:#010x} {plm_note} SS0={ss0_rb:#010x} SS1={ss1_rb:#010x} unlock_persist={}",
            ss0_rb == 0x8888_8888 && ss1_rb == 0x8
        );
    }
            }
        } else {
            // Probe: the same two writes, but a readback mismatch is the
            // EXPECTED verdict under a locked PLM and must not abort the
            // unwind. No signature restore - the booter never consumed the
            // meta page.
            let (ss0_at, ss1_at) = if assets.family == "tu10x" {
                (tb::TU10X_FEAT_SS0, tb::TU10X_FEAT_SS1)
            } else {
                (0x0082_381c, 0x0082_3820)
            };
            let _ = tb::write_selector(gpu, "SS0", ss0_at, tb::SS0_UNLOCK);
            let _ = tb::write_selector(gpu, "SS1", ss1_at, tb::SS1_UNLOCK);
            info!("FORCE-SELECTOR probe done (the selector read lines above carry the verdict)");
        }
    }

    if assets.family == "tu10x" {
        // NO FLR on Turing: the FEAT PLM does NOT survive FLR there (13th
        // fire 2026-10-27: post-FLR plm relocked to 0xffffff8f, unlike Ada
        // where the AON island holds the unlock across FLR). The booter's
        // own sanitize (0xdead5ec2) is the cleanup - exactly the cmp50hx
        // chain shape, which hands the pre-OS state to the OS with zero
        // Xid and no reset.
        info!("TX-tu10x: FLR skipped (Turing FEAT PLM does not survive FLR - the unlock ships as-is; booter sanitize was the cleanup)");
    } else {
        let flr_expect_unlock = payload_mode && booter_ok;
        tb::flr_phase(gpu, assets.family, flr_expect_unlock)?;
        info!("POST-FLR: verified{}", if flr_expect_unlock { " (selectors survive AON)" } else { "" });
    }

    // Read-only handoff PCI snapshot (issue #61 ask #2): the config state
    // Windows will inherit, logged before the reconnect touches anything.
    crate::pci::diag_handoff_pci(gpu.pci);

    // Restore the display path before handing off (TOTORONG craft): the
    // FLR-rebooted function re-binds its vendor GOP/driver stack. Non-fatal
    // - bootmgfw self-establishes a console regardless.
    // Turing: reconnect ON by default. The full 16-fire series + the
    // GSP-mode Windows closure (2026-10-27) showed the TDRs came from the
    // LEGACY driver path (EnableGpuFirmware unset), not from the GOP
    // devinit - with the GSP-mode driver the unlock session is stable, and
    // the reconnect just restores the boot console earlier (Run A's
    // shape-change when it was removed turned out to be coincidental).
    // A \tu10x-no-reconnect.txt marker skips it again for experiments.
    if assets.family == "tu10x" && crate::tx::probe_file("\\tu10x-no-reconnect.txt") {
        info!("TX-tu10x: reconnect skipped (tu10x-no-reconnect marker)");
    } else {
        crate::tx_detach::reconnect_all(pci_handle);
    }

    // Unwind: release every DMA buffer.
    let _ = booter_dma.release(gpu.pci);
    let _ = sig_dma.release(gpu.pci);
    let _ = meta_dma.release(gpu.pci);
    let _ = stock_dma.release(gpu.pci);
    let _ = boot_dma.release(gpu.pci);
    let mut radix = radix;
    let _ = radix.dma.release(gpu.pci);
    info!("FINAL: transaction complete, all DMA released");
    Ok(())
}

/// Save PMCSR and pin the function to D0 before an FLR (PCIe: FLR is only
/// defined in D0; Linux's pci_reset_function also forces D0 first).
pub fn pm_flr_pre(pci: &crate::pci::PciIo) -> Option<(u8, u16)> {
    let cap = pci.pm_cap()?;
    let saved = pci.pmcsr(cap);
    if saved & 0x3 != 0 {
        info!("PM: pre-FLR state was D{} (PMCSR={saved:#06x}) - forcing D0 for the reset", saved & 0x3);
        let _ = pci.pmcsr_force_d0(cap);
    }
    Some((cap, saved))
}

/// Restore the pre-FLR PMCSR with the state field pinned to D0 and verify.
pub fn pm_flr_post(pci: &crate::pci::PciIo, saved: Option<(u8, u16)>) {
    let Some((cap, pmcsr)) = saved else { return };
    pci.pmcsr_write(cap, pmcsr & !0x3);
    stall(Duration::from_millis(10));
    let rb = pci.pmcsr(cap);
    info!("PM: post-FLR PMCSR={rb:#06x} state=D{} (entry {pmcsr:#06x})", rb & 0x3);
}

/// One Function-Level Reset with spec timing and command restore.
/// Per reference post-FLR expectations: PCI identity intact, GFW completion
/// persists, WPR2 cleared, engines halted - the "cold-boot-equivalent" state.
pub fn flr_once(pci: &crate::pci::PciIo) {
    let pm = pm_flr_pre(pci);
    // pci_save_state analog: command + BAR0-5 config dwords (0x04..0x28).
    // PCIe FLR resets these to defaults - WITHOUT restore, the device no
    // longer decodes its BARs and every memory read master-aborts to 0xff
    // (measured: gfw_boot=0xffffffff after FLR). This is the concrete
    // content of the Linux dev's "PCI re-discovery" point.
    let mut saved = [0u32; 10]; // 0x04 command, 0x08..0x0c, 0x10..0x24 BARs
    for (i, slot) in saved.iter_mut().enumerate() {
        *slot = pci.config_read_u32(0x04 + i as u32 * 4);
    }
    // TOTORONG craft (port.c GuFlr): save Device Control (our old code wrote
    // 0x0000 over it, wiping MPS/MRR), wait Transactions Pending clear,
    // set the FLR bit preserving the rest of the register, then 100 ms of
    // NO access at all (PCI spec: the function need not respond).
    let pcie = crate::pci::pcie_cap_or_measured(pci);
    let devcap = pci.config_read_u32(pcie as u32 + 0x04);
    // PCI_EXP_DEVCAP_FLR = bit 28 (Linux pci_regs.h; gpu-unlock's compat.h
    // agrees). Bit 15 is the low bit of the L1 Acceptable Latency field
    // (DEVCAP[17:15]), so testing it warned on every card whose L1 latency
    // happened to be 0 or 2.
    if devcap & (1 << 28) == 0 {
        info!("FLR: DEVCAP={devcap:#010x} does not advertise FLR (bit 28 clear) - the reset below may be a no-op");
    }
    let devctl = pci.config_read_u16(pcie as u32 + 0x08);
    for _ in 0..1000 {
        let devsta = pci.config_read_u16(pcie as u32 + 0x0a);
        if devsta == 0xffff || devsta & 0x20 == 0 {
            break;
        }
        boot::stall(core::time::Duration::from_millis(1));
    }
    pci.cfg_write_u16(0x04, 0x0003); // decode on for the restore, BME off
    pci.cfg_write_u16(pcie as u32 + 0x08, devctl | 0x8000);
    boot::stall(core::time::Duration::from_millis(100)); // spec FLR time, silence
    pci.cfg_write_u16(pcie as u32 + 0x08, devctl); // restore MPS/MRR, clear FLR bit
    // Wait for the function to answer config again before touching BARs.
    for _ in 0..1000 {
        let id = pci.config_read_u32(0x00);
        if id != 0xffff_ffff && id != 0 && id & 0xffff == 0x10de {
            break;
        }
        boot::stall(core::time::Duration::from_millis(1));
    }
    // pci_restore_state analog, TOTORONG order: decode OFF while BARs move,
    // high BAR dwords BEFORE low dwords, command (never BME) last + verify.
    pci.cfg_write_u16(0x04, 0x0000);
    for i in (2..10).rev() {
        pci.cfg_write_u32(0x04 + i as u32 * 4, saved[i]);
    }
    pci.cfg_write_u16(0x04, (saved[0] as u16) & !0x0004);
    let cmd = pci.config_read_u16(0x04);
    let sta = pci.config_read_u16(pcie as u32 + 0x0a);
    let gfw = pci.bar_read_u32(0, 0x0011_8234);
    let wpr_hi = pci.bar_read_u32(0, 0x001f_a828);
    info!(
        "FLR: cmd={cmd:#06x} devsta={sta:#06x} (pending={}) gfw_boot={gfw:#x} wpr2_hi={wpr_hi:#x}",
        sta & 0x200 != 0,
    );
    pm_flr_post(pci, pm);
}

/// Last observable state before the chain-load, in one line. Everything the
/// Windows side will later read back is logged here so the two records can be
/// compared directly (a mismatch after boot localizes the consumer that undid
/// the writes - driver reinit, rail cut, or nothing at all).
pub fn handoff_out(pci: &crate::pci::PciIo) {
    let rd = |off: u32| pci.bar_read_u32(0, off as u64);
    let (ss0, ss1) = (rd(regs::FEAT_SS0), rd(regs::FEAT_SS1));
    let readout = rd(regs::FEAT_READOUT_1);
    let plm = rd(regs::FEAT_PLM);
    // Turing's FEAT block lives at 0x4096xx - the GA10x/Ada reads above hit
    // a different block there, so log the Turing quadruple too (readout at
    // 0x409668, issue #42: ssdiag prints this from Windows; the EFI side
    // needs the same value for the before/after comparison).
    let (tss0, tss1, tplm, treadout) = (
        rd(crate::tx_booter::TU10X_FEAT_SS0),
        rd(crate::tx_booter::TU10X_FEAT_SS1),
        rd(crate::tx_booter::TU10X_FEAT_PLM),
        rd(crate::tx_booter::TU10X_FEAT_READOUT),
    );
    let wpr_hi = rd(regs::WPR2_HI);
    let (pmcsr, dstate) = match pci.pm_cap() {
        Some(cap) => {
            let v = pci.pmcsr(cap);
            (v, v & 0x3)
        }
        None => (0u16, 0xffu16),
    };
    info!(
        "HANDOFF-OUT: ss0={ss0:#010x} ss1={ss1:#x} readout1={readout:#010x} feat_plm={plm:#010x} wpr2_hi={wpr_hi:#x} pmcsr={pmcsr:#06x} dstate=D{dstate} | tu10x: plm={tplm:#010x} ss0={tss0:#010x} ss1={tss1:#010x} readout={treadout:#010x} | tu10x_unlock={} ss0_unlock={} ss1_unlock={} plm_open={}",
        // Relock-aware (issues #12/#14): the shipped tu10x chain closes the
        // PLM gate after the SS writes (open -> write -> close), so the
        // success shape is the SS override persisted behind a closed gate -
        // plm_open is reported separately, and a still-open PLM (0xffffffff)
        // is covered here too since the SS reads are the same predicate.
        tss0 == crate::tx_booter::SS0_UNLOCK && tss1 == crate::tx_booter::SS1_UNLOCK,
        ss0 == crate::tx_booter::SS0_UNLOCK,
        ss1 == crate::tx_booter::SS1_UNLOCK,
        plm == 0xffff_ffff,
    );
}

/// Emergency FLR on any transaction failure: wipes every half-state we may
/// have left (engine resets, BCR switches, full DMA queues, SEC bit state)
/// so the Windows driver receives a cold-boot-equivalent card. TOTORONG's
/// unconditional end-of-transaction FLR serves the same purpose.
pub fn emergency_flr(pci_handle: uefi::Handle) {
    use uefi::boot::{OpenProtocolAttributes, OpenProtocolParams};
    // Safety: GetProtocol; the exclusive claim has been dropped by now.
    let pci = match unsafe {
        boot::open_protocol::<crate::pci::PciIo>(
            OpenProtocolParams {
                handle: pci_handle,
                agent: boot::image_handle(),
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
    } {
        Ok(p) => p,
        Err(_) => {
            info!("EMERGENCY-FLR: cannot open PciIo - powering off is the only recovery");
            return;
        }
    };
    flr_once(&pci);
}
