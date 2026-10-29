//! Behavioral fake-MMIO GPU model + transaction scenarios.
//!
//! Behavior port of gpu-unlock tests/test_unlock_adapter.c (GPL-2.0-only;
//! independent Rust implementation, knobs and quirks preserved 1:1 where
//! the hardware dictates them):
//! - SS1 is a 4-bit register (writes mask to 0xf, as on the card)
//! - engine reset registers propagate after N reads (reset_delay)
//! - SEC2 boot modes: 0 = full boot opens FEAT_OVR_PLM (the payload
//!   effect), 2 = PLM stuck open pre-transaction, 3 = HALT
//! - FWSEC-FRTS start installs WPR2 (the guard's "already initialized")
//! - selector_bad: SS0/SS1 writes silently dropped (privilege failure)
//! - GFW completion can arrive late (FB size register pops after N reads)
//!
//! The PLM write-filter itself (host L0 writes dropped unless PLM open) is
//! the core privilege model and lives here too.

#[path = "../../loader/src/tu102_profile.rs"]
pub mod tu102_profile;
#[cfg(test)]
mod tu102_profile_tests;

/// BAR0 offsets (pinned 610.43.03 GA102 headers via gpu-unlock gfw_eval.h;
/// cross-checked against task brief §3.2).
pub mod regs {
    pub const GFW_PLM: u32 = 0x0011_8128;
    pub const GFW_BOOT: u32 = 0x0011_8234;
    pub const GFW_CPUCTL: u32 = 0x0011_0100;
    pub const FB_SIZE_MIB: u32 = 0x0011_83a4;
    pub const MMU_PLM: u32 = 0x001f_a7c8;
    pub const WPR2_LO: u32 = 0x001f_a824;
    pub const WPR2_HI: u32 = 0x001f_a828;
    pub const WPR_PLM: u32 = 0x001f_a7c4; // Ada chain write #3
    pub const WPRCFG_PLM: u32 = 0x001f_a7cc; // Ada chain write #1 (value 0xfffff0ff!)
    pub const FBPA_PLM: u32 = 0x009a_0148; // Ada chain write #2
    pub const SEC2_BASE: u32 = 0x0084_0000;
    pub const SEC2_CPUCTL: u32 = SEC2_BASE + 0x100;
    pub const SEC2_BOOTVEC: u32 = SEC2_BASE + 0x130;
    pub const SEC2_RESET: u32 = SEC2_BASE + 0x3c0;
    pub const FEAT_PLM: u32 = 0x0082_3804;
    pub const FEAT_SS0: u32 = 0x0082_381c;
    pub const FEAT_SS1: u32 = 0x0082_3820;

    pub const GFW_BOOT_COMPLETED: u32 = 0xff;
    pub const CPUCTL_HALTED: u32 = 0x10;
}

/// What the fake SEC2 does when started.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum BootMode {
    /// Full boot: the V67 effect — FEAT_OVR_PLM opens to 0xffffffff.
    #[default]
    Full,
    /// Ada four-write chain (single fire, gpu-unlock-ada AD102 payload):
    /// WPR_CFG=0xfffff0ff, FBPA/WPR/FEAT=0xffffffff, then marker+HALT.
    AdaFourWrite,
    /// GA102 canonical V67 (single FEAT PLM write + fill self-loop tail):
    /// the PLM opens and the core KEEPS RUNNING in the self-loop - no
    /// marker, no halt, ever (issue #6: the canonical wait is the PLM
    /// flip; a halt during the wait is the failure sign).
    Ga102SelfLoop,
    /// GA102 pathology: the core spins (PC in the fill loop) but the PLM
    /// never opens - the ROP's write was dropped. The wait_plm verdict
    /// must be a timeout, not a success.
    SelfLoopNoFlip,
    /// PLM stuck open before the transaction (guard: -EALREADY analog).
    PlmStuckOpen,
    /// SEC2 HALTs after start (marker path).
    Halt,
}

#[derive(Default)]
pub struct FakeGpu {
    pub regs: Vec<u32>,
    /// Resets: writes take effect only after `reset_delay` reads.
    pub reset_delay: u32,
    reset_pending: [Option<(usize, u32)>; 2], // (reg index, reads so far)
    /// Drop SS0 (1) / SS1 (2) host writes silently.
    pub selector_bad: u32,
    /// What SEC2 start does.
    pub boot_mode: BootMode,
    /// FWSEC-FRTS WPR2 installed on GSP start: (lo, hi).
    pub fwsec_wpr: Option<(u32, u32)>,
    /// GFW completes only after this many reads of GFW_BOOT (0 = already done).
    pub gfw_late_reads: u32,
    gfw_reads: u32,
}

impl FakeGpu {
    /// Cold powered-on GA102-class card with the unlock already effective
    /// (fixture mirrors test_unlock_adapter.c setup()).
    /// Locked cold card as the transaction finds it (guards pass).
    /// `drift` models the AD107-measured boot-to-boot SS0/SS1 variance:
    /// selector override registers are NOT deterministic cold.
    pub fn new_locked_cold(drift: Option<(u32, u32)>) -> Self {
        let mut gpu = Self::new_unlocked_fixture();
        gpu.force_write(regs::FEAT_PLM, 0xffff_ff8f); // locked baseline
        let (ss0, ss1) = drift.unwrap_or((0x0000_0001, 0x0000_0001));
        gpu.force_write(regs::FEAT_SS0, ss0);
        gpu.force_write(regs::FEAT_SS1, ss1 & 0xf);
        gpu.force_write(regs::WPR2_LO, 0x1fff_fe00); // disarmed window
        gpu.force_write(regs::WPR2_HI, 0);
        gpu
    }

    pub fn new_unlocked_fixture() -> Self {
        let mut gpu = Self {
            regs: vec![0; 0xA00_000 / 4],
            boot_mode: BootMode::Full,
            ..Self::default()
        };
        gpu.w32(regs::GFW_BOOT, regs::GFW_BOOT_COMPLETED); // GFW done
        gpu.w32(regs::GFW_CPUCTL, regs::CPUCTL_HALTED);
        gpu.w32(regs::FB_SIZE_MIB, 0x6000);
        gpu.w32(regs::FEAT_PLM, 0xffff_fff8 | 0x7); // fully open fixture
        gpu.w32(regs::FEAT_SS0, 0x8888_8888);
        gpu.w32(regs::FEAT_SS1, 0x0);
        gpu.w32(regs::WPR2_LO, 0xffff_fff0); // pre-armed window, hi=0
        gpu.w32(regs::WPR2_HI, 0);
        gpu.regs[0] = 0x1720_00a1; // PMC_BOOT_0 chip id
        gpu
    }

    fn idx(&self, off: u32) -> usize {
        (off / 4) as usize
    }
    fn w32(&mut self, off: u32, v: u32) {
        let i = self.idx(off);
        self.regs[i] = v;
    }
    /// Bypass the behavior model (fixture setup / fault injection only).
    pub fn force_write(&mut self, off: u32, v: u32) {
        self.w32(off, v);
    }

    pub fn read(&mut self, off: u32) -> u32 {
        let i = self.idx(off);
        // GFW late completion: FB size pops once boot completes.
        if off == regs::GFW_BOOT && self.gfw_late_reads > 0 {
            self.gfw_reads += 1;
            if self.gfw_reads >= self.gfw_late_reads {
                self.w32(regs::GFW_BOOT, regs::GFW_BOOT_COMPLETED);
                self.w32(regs::FB_SIZE_MIB, 0x6000);
            }
        }
        // Reset propagation: pending reset completes after N reads.
        for eng in 0..2 {
            if let Some((reg, seen)) = self.reset_pending[eng] {
                if reg == i {
                    let seen = seen + 1;
                    self.reset_pending[eng] = Some((reg, seen));
                    if seen >= self.reset_delay {
                        self.regs[i] = 1; // RESET_TRUE analog
                        self.reset_pending[eng] = None;
                    }
                }
            }
        }
        self.regs[i]
    }

    pub fn write(&mut self, off: u32, v: u32) {
        // Privilege model: FEAT block writes are dropped at L0 unless PLM open.
        let plm_open = self.regs[self.idx(regs::FEAT_PLM)] == 0xffff_ffff;
        if matches!(off, regs::FEAT_SS0 | regs::FEAT_SS1) && !plm_open {
            return; // silently dropped, as the hardware does
        }
        if off == regs::FEAT_SS0 && self.selector_bad == 1 {
            return;
        }
        if off == regs::FEAT_SS1 {
            if self.selector_bad == 2 {
                return;
            }
            self.w32(off, v & 0xf); // 4-bit register, as on the card
            return;
        }

        self.w32(off, v);

        // Engine reset: arm propagation delay when asserting.
        if off == regs::SEC2_RESET && v & 1 != 0 && self.reset_delay > 0 {
            let i = self.idx(off);
            self.regs[i] = 0;
            self.reset_pending[0] = Some((i, 0));
        }

        // GSP start (FWSEC BOOT_WITH_LOADER analog): install WPR2.
        if off == regs::GFW_CPUCTL && v & 2 != 0 { // START = bit 1 (real chip)
            if let Some((lo, hi)) = self.fwsec_wpr {
                self.w32(regs::WPR2_LO, lo);
                self.w32(regs::WPR2_HI, hi);
            }
            self.w32(regs::GFW_CPUCTL, regs::CPUCTL_HALTED);
        }

        // SEC2 start: the payload effect. Full mode = payload executed:
        // marker path ends with SEC2 HALTED and the PLM opened (triad).
        if off == regs::SEC2_CPUCTL && v & 2 != 0 { // START = bit 1 (real chip)
            match self.boot_mode {
                BootMode::Full => {
                    self.w32(regs::FEAT_PLM, 0xffff_ffff);
                    self.w32(regs::SEC2_CPUCTL, regs::CPUCTL_HALTED);
                }
                BootMode::AdaFourWrite => {
                    // Single-fire four-write chain + marker, then HALT.
                    self.w32(regs::WPRCFG_PLM, 0xffff_f0ff); // [11:8] left clear!
                    self.w32(regs::FBPA_PLM, 0xffff_ffff);
                    self.w32(regs::WPR_PLM, 0xffff_ffff);
                    self.w32(regs::FEAT_PLM, 0xffff_ffff);
                    self.w32(regs::SEC2_CPUCTL, regs::CPUCTL_HALTED);
                }
                BootMode::Ga102SelfLoop => {
                    // One write (FEAT PLM), then the fill self-loop: the
                    // core keeps running, mailbox0 keeps the meta pointer.
                    self.w32(regs::FEAT_PLM, 0xffff_ffff);
                    self.w32(regs::SEC2_CPUCTL, 0);
                }
                BootMode::SelfLoopNoFlip => {
                    // Spinning in the fill loop, write dropped: no PLM, no
                    // halt - the wait must time out.
                    self.w32(regs::SEC2_CPUCTL, 0);
                }
                BootMode::PlmStuckOpen => self.w32(regs::SEC2_CPUCTL, !0u32), // C boot_mode 2
                BootMode::Halt => self.w32(regs::SEC2_CPUCTL, regs::CPUCTL_HALTED),
            }
        }
    }
}

/// Transaction guards (task brief §9.6 semantics, -EBUSY / -EALREADY kept
/// as error strings until the real error enum lands).
pub fn wpr2_cold_guard(gpu: &mut FakeGpu) -> Result<(), &'static str> {
    let hi = gpu.read(regs::WPR2_HI);
    if hi != 0 {
        return Err("EBUSY: WPR2 already initialized this power cycle");
    }
    Ok(())
}

pub fn plm_cold_guard(gpu: &mut FakeGpu) -> Result<(), &'static str> {
    let plm = gpu.read(regs::FEAT_PLM);
    if plm == 0xffff_ffff {
        return Err("EALREADY: FEAT PLM already open (unlocked or not cold)");
    }
    Ok(())
}

/// Write-and-read-back — the ONLY success criterion (constitution §1).
pub fn write_verified(gpu: &mut FakeGpu, off: u32, v: u32) -> Result<u32, &'static str> {
    gpu.write(off, v);
    let back = gpu.read(off);
    if back != v {
        return Err("write did not stick (privilege drop or masked field)");
    }
    Ok(back)
}

#[cfg(test)]
mod scenarios;
#[cfg(test)]
mod scenarios2;

pub mod devpath;
pub mod dma;
pub mod layout;
pub mod radix;
pub mod transaction;
