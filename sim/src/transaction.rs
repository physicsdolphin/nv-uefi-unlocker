//! Full transaction state machine (behavioral port of unlock_610.c phase
//! structure, at the abstraction level of our EFI loader's M3 design):
//!
//! claim → gfw_wait → fwsec → wpr2_verify → booter_fire → execution_verify
//! (payload shape: Ada marker+HALT+PLM triad, or GA102 wait_plm flip) →
//! selectors → flr → post_flr_verify → unwind
//!
//! Every fault path must unwind completely: DMA freed, BME restored,
//! nothing left held. The read-back is the only success criterion.

use crate::regs;
use crate::{FakeGpu};

pub const SEC2_MAILBOX0: u32 = regs::SEC2_BASE + 0x40;
pub const SEC2_MAILBOX1: u32 = regs::SEC2_BASE + 0x44;
pub const SEC2_BCR: u32 = 0x0084_1000 + 0x668; // FALCON2/RISC-V switch
pub const MARKER_OK: u32 = 0x67b0_0001;

#[derive(Default, Clone, Debug)]
pub struct Faults {
    /// DMA page allocation fails at `fail_alloc_at` (1-based).
    pub fail_alloc_at: Option<u32>,
    /// pci_set_master analog fails.
    pub master_fail: bool,
    /// SEC2 never reports the marker in mailbox 0.
    pub marker_missing: bool,
    /// SEC2 never HALTs.
    pub halt_not_set: bool,
    /// GA102 payload: the ROP never ran - the booter executes stock,
    /// reports its signature error, and HALTs (early-PLM-failed verdict).
    pub plm_missing: bool,
    /// GA102 payload: the core spins in the fill loop but the PLM write
    /// was dropped - wait_plm must time out, never succeed.
    pub plm_flip_missing: bool,
    /// FWSEC installs a WPR2 different from the requested layout.
    pub wpr2_mismatch: bool,
    /// FLR itself fails (EIO analog).
    pub fail_flr: bool,
    /// RISC-V<->Falcon BCR switch reads back 0 (stuck).
    pub switch_stuck: bool,
    /// Falcon DMA progress stalls after N blocks.
    pub dma_stall_after: u32,
    /// post-FLR WPR2 comes back non-zero (alien value).
    pub post_flr_alien_wpr: bool,
    /// Signature restoration into the meta fails to land the stock bytes.
    pub signature_restore_fail: bool,
    /// DMA free at `fail_free_at` (1-based) fails - makes the unwind phase
    /// genuinely fallible (issue #22: the drain loop previously could not
    /// fail, so the "unwind" phase error was unproducible and t18 was a
    /// tautology).
    pub fail_free_at: Option<u32>,
}

/// Payload return-path shape (canonical falcon_hs `wait_plm` split).
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PayloadShape {
    /// Ada four-write chain: marker into mailbox0 + HALT tail (M4-verified
    /// triad verdict).
    #[default]
    AdaTriad,
    /// GA102 canonical V67: single FEAT PLM write + fill self-loop tail -
    /// the PLM flip is the verdict; no marker, and no halt will EVER come.
    Ga102SelfLoop,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub dma_pages_allocated: usize,
    pub dma_pages_freed: usize,
    pub fwsec_fired: bool,
    pub booter_fired: bool,
    pub marker_seen: bool,
    pub halt_seen: bool,
    pub plm_open_verified: bool,
    pub selectors_written: bool,
    pub bme_restored: bool,
    pub flr_done: bool,
    pub signature_restored: bool,
    pub unwind_clean: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhaseError {
    pub phase: &'static str,
    pub reason: String,
}

impl PhaseError {
    fn new(phase: &'static str, reason: impl Into<String>) -> Self {
        Self { phase, reason: reason.into() }
    }
}

pub struct Transaction<'a> {
    pub gpu: &'a mut FakeGpu,
    pub faults: Faults,
    /// false = chain-replay (stock booter, PLM stays locked, no selector
    /// writes, no signature restore).
    pub payload: bool,
    /// Payload return-path shape (Ada triad vs GA102 self-loop wait_plm).
    pub shape: PayloadShape,
    /// Turing entry shape (issue #4.2): WPR2 up at claim is triaged on
    /// firmware-window evidence instead of the old family-wide accept.
    pub tu10x_entry: bool,
    /// Operator-marker analog of the loader's \tx-accept-warm-wpr2.txt:
    /// accept an up WPR2 whose evidence is inconclusive.
    pub warm_wpr2_marker: bool,
    // bookkeeping (unwind contract)
    dma_live: usize,
    dma_allocs: usize,
    dma_frees: usize,
    bme_set: bool,
    bme_was_dirty: bool,
    // Execution evidence as READ, never recomputed from Faults: the Report has to
    // describe the model's state, not the switches the test constructed.
    last_marker: Option<u32>,
    last_cpuctl: Option<u32>,
    last_plm: Option<u32>,
    fwsec_done: bool,
    booter_done: bool,
    log: Vec<String>,
}

impl<'a> Transaction<'a> {
    pub fn new(gpu: &'a mut FakeGpu, faults: Faults) -> Self {
        Self {
            gpu,
            faults,
            payload: true,
            shape: PayloadShape::AdaTriad,
            tu10x_entry: false,
            warm_wpr2_marker: false,
            dma_live: 0,
            last_marker: None,
            last_cpuctl: None,
            last_plm: None,
            fwsec_done: false,
            booter_done: false,
            dma_allocs: 0,
            dma_frees: 0,
            bme_set: false,
            bme_was_dirty: false,
            log: Vec::new(),
        }
    }

    /// GA102 canonical payload (issue #6): single PLM write + self-loop -
    /// the wait is the PLM flip, never a halt.
    pub fn new_ga102_payload(gpu: &'a mut FakeGpu, faults: Faults) -> Self {
        Self {
            shape: PayloadShape::Ga102SelfLoop,
            ..Self::new(gpu, faults)
        }
    }

    /// Chain-replay mode (Turing M3 shape, and the GA10x replay): a STOCK
    /// booter with no payload. The verdict is mailbox0 == 0 + HALT + WPR2
    /// still up; the FEAT PLM stays LOCKED (the stock booter never opens
    /// it), selectors are never written, and there is no payload signature
    /// to restore before the FLR.
    pub fn new_replay(gpu: &'a mut FakeGpu, faults: Faults) -> Self {
        Self {
            payload: false,
            ..Self::new(gpu, faults)
        }
    }

    fn note(&mut self, s: impl Into<String>) {
        self.log.push(s.into());
    }

    /// Phase log (read-only): tests anchor on what the machine actually
    /// read, not on the switches that constructed it.
    pub fn log(&self) -> &[String] {
        &self.log
    }

    /// Cold guards + identity + BME claim. BME dirty state (board firmware
    /// left it on) is recorded and re-cleared — task brief §9.3.
    fn claim(&mut self) -> Result<(), PhaseError> {
        let plm = self.gpu.read(regs::FEAT_PLM);
        if plm == 0xffff_ffff {
            return Err(PhaseError::new("claim", "EALREADY: PLM open"));
        }
        let hi = self.gpu.read(regs::WPR2_HI);
        if hi != 0 {
            if !self.tu10x_entry {
                return Err(PhaseError::new("claim", "EBUSY: WPR2 initialized"));
            }
            // Issue #4.2 triage (mirror of the loader's Turing branch in
            // claim_guards): WPR2-up at entry accepts on firmware-window
            // evidence (GFW completed, GSP not started); the BSI handoff is
            // logged but NOT gated (scrubber FSM per OpenRM, DONE=3, TU106
            // entry measures 1 - not a WPR2-provenance discriminator).
            // Anything else refuses unless the operator-marker analog opts in.
            let gfw = self.gpu.read(regs::GFW_BOOT);
            let handoff = self.gpu.read(regs::BSI_SCRATCH15) >> 29 & 7;
            let cpuctl = self.gpu.read(regs::GFW_CPUCTL);
            self.note(format!(
                "wpr2-up triage: GFW={gfw:#x} handoff={handoff} GSP_CPUCTL={cpuctl:#x}"
            ));
            if cpuctl & 0x3 != 0 {
                return Err(PhaseError::new(
                    "claim",
                    "EBUSY: GSP session live (CPUCTL started)",
                ));
            }
            if gfw == regs::GFW_BOOT_COMPLETED {
                self.note("claim: firmware-window state proven - WPR2-up accepted");
            } else if self.warm_wpr2_marker {
                self.note("claim: evidence inconclusive, operator marker - accepted");
            } else {
                return Err(PhaseError::new(
                    "claim",
                    "EBUSY: Turing WPR2-up with GFW_BOOT not completed (warm/FLR residue)",
                ));
            }
        }
        let chip = self.gpu.read(0);
        if chip >> 16 == 0xBADF || chip == 0 {
            return Err(PhaseError::new("claim", "ENODEV: identity unreadable"));
        }
        self.bme_was_dirty = true; // claim clears it; restore on unwind
        if !self.faults.master_fail {
            self.bme_set = true;
        } else {
            return Err(PhaseError::new("claim", "pci_set_master failed"));
        }
        self.note("claim ok");
        Ok(())
    }

    fn alloc_dma_page(&mut self) -> Result<(), PhaseError> {
        self.dma_allocs += 1;
        if self.faults.fail_alloc_at == Some(self.dma_allocs as u32) {
            return Err(PhaseError::new("dma", format!("alloc #{} failed", self.dma_allocs)));
        }
        self.dma_live += 1;
        Ok(())
    }

    fn gfw_wait(&mut self) -> Result<(), PhaseError> {
        // Poll GFW_BOOT for completion (bounded; fake handles late completion).
        for _ in 0..1000 {
            if self.gpu.read(regs::GFW_BOOT) == regs::GFW_BOOT_COMPLETED {
                self.note("gfw done");
                return Ok(());
            }
        }
        Err(PhaseError::new("gfw_wait", "ETIMEDOUT: GFW_BOOT never completed"))
    }

    /// FWSEC BOOT_WITH_LOADER ignition; installs WPR2 in the fake.
    fn fwsec(&mut self) -> Result<(u32, u32), PhaseError> {
        self.alloc_dma_page()?;
        let want = (0x1fff_fe00u32, 0x0400_0000u32);
        let install = if self.faults.wpr2_mismatch {
            (0x1fff_fe00, 0x0500_0000) // alien geometry
        } else {
            want
        };
        self.gpu.fwsec_wpr = Some(install);
        self.gpu.write(regs::GFW_CPUCTL, 1 << 1); // START = bit 1 (real chip)
        let lo = self.gpu.read(regs::WPR2_LO);
        let hi = self.gpu.read(regs::WPR2_HI);
        if (lo, hi) != want {
            return Err(PhaseError::new("fwsec", format!(
                "WPR2 mismatch: installed {lo:#x}/{hi:#x}, wanted {:#x}/{:#x}", want.0, want.1
            )));
        }
        self.fwsec_done = true;
        self.note("fwsec fired, WPR2 verified");
        Ok(want)
    }

    /// Booter fire: DMA the signature image, start SEC2, verify the
    /// execution triad (marker + HALT + PLM). Status 0x31 analog is NOT a
    /// failure — read-backs decide (task brief §7.4).
    fn booter_fire_and_verify(&mut self) -> Result<(), PhaseError> {
        self.alloc_dma_page()?; // signature DMA
        // BCR switch check (RISC-V <-> Falcon handover interface present) -
        // payload path only: Turing BOOT_DIRECT keeps the Falcon core, no
        // switch exists on that path.
        if self.payload {
            self.gpu.write(SEC2_BCR, 1);
            let bcr = self.gpu.read(SEC2_BCR);
            if self.faults.switch_stuck || bcr == 0 {
                return Err(PhaseError::new("booter", "BCR switch stuck at 0"));
            }
        }
        // Reset SEC2 with propagation semantics; assert then wait.
        self.gpu.write(regs::SEC2_RESET, 1);
        for _ in 0..16 {
            if self.gpu.read(regs::SEC2_RESET) & 1 != 0 {
                break;
            }
        }
        if self.gpu.read(regs::SEC2_RESET) & 1 == 0 {
            return Err(PhaseError::new("booter", "reset never propagated"));
        }
        // De-assert (ksec2ResetHw_TU102 ENDS at RESET_FALSE - AGENTS §9.2);
        // a reset left asserted would poison every later host read with
        // FECS_PRI_RESET bits. De-assertion is read-back verified too.
        self.gpu.write(regs::SEC2_RESET, 0);
        if self.gpu.read(regs::SEC2_RESET) & 1 != 0 {
            return Err(PhaseError::new("booter", "reset never de-asserted"));
        }
        // Fire. Replay uses the stock-booter behavior (Halt mode: SEC2
        // halts, nothing else written) - Full mode would model the payload
        // opening the PLM, which a stock boot never does.
        if !self.payload {
            self.gpu.boot_mode = crate::BootMode::Halt;
        }
        if self.payload && self.shape == PayloadShape::Ga102SelfLoop {
            // GA102 canonical: pick the fake's return-path behavior from
            // the fault model before START (lib.rs boot_mode effects).
            self.gpu.boot_mode = if self.faults.plm_missing {
                crate::BootMode::Halt // payload missed the stack: stock run
            } else if self.faults.plm_flip_missing {
                crate::BootMode::SelfLoopNoFlip
            } else {
                crate::BootMode::Ga102SelfLoop
            };
        }
        let plm_entry = self.gpu.read(regs::FEAT_PLM);
        self.gpu.write(regs::SEC2_CPUCTL, 1 << 1); // START = bit 1 (real chip)
        if !self.payload {
            // Replay verdict: the stock booter halts with mailbox0 == 0
            // (nonzero = its own error status; 0x89-0x90 = WprMeta rejected
            // on DMA readback), WPR2 stays up, and the PLM must remain
            // locked - a replay that opened the PLM would mean something
            // ELSE wrote it, which is exactly what the guard exists for.
            let status = if self.faults.marker_missing { 0x89 } else { 0 };
            self.gpu.force_write(SEC2_MAILBOX0, status);
            if self.faults.halt_not_set {
                self.gpu.force_write(regs::SEC2_CPUCTL, 0); // still running
            } else {
                self.gpu.force_write(regs::SEC2_CPUCTL, regs::CPUCTL_HALTED);
            }
            let mbox0 = self.gpu.read(SEC2_MAILBOX0);
            // Raw read goes into the Report verbatim; the halt predicate
            // projects bit 0x10 from it. Never store a recomputed value.
            let cpuctl = self.gpu.read(regs::SEC2_CPUCTL);
            let halt = cpuctl & 0x10 != 0;
            let plm = self.gpu.read(regs::FEAT_PLM);
            let wpr_hi = self.gpu.read(regs::WPR2_HI);
            if mbox0 != 0 {
                return Err(PhaseError::new("verify", format!(
                    "replay booter status {mbox0:#x} (nonzero = failed)"
                )));
            }
            if !halt {
                return Err(PhaseError::new("verify", "replay: SEC2 never halted"));
            }
            if wpr_hi >> 4 == 0 {
                return Err(PhaseError::new("verify", "replay: WPR2 down after stock boot"));
            }
            if plm == 0xffff_ffff {
                return Err(PhaseError::new("verify", "replay: PLM opened by a stock boot (unexpected)"));
            }
            self.last_marker = Some(mbox0);
            self.last_cpuctl = Some(cpuctl);
            self.last_plm = Some(plm);
            self.booter_done = true;
            self.note("replay verified (mbox0=0 + HALT + WPR2 up, PLM stays locked)");
            return Ok(());
        }
        if self.shape == PayloadShape::Ga102SelfLoop {
            // Canonical wait_plm verdict (unlock_610.c 403-424): vetted
            // reads; the PLM cold->open transition is success; a HALT
            // observed before the flip is the FAILURE sign (the signature
            // DMA missed the stack - a stock run cannot open the PLM).
            let plm = self.gpu.read(regs::FEAT_PLM);
            let cpuctl = self.gpu.read(regs::SEC2_CPUCTL);
            let halted = cpuctl & 0x10 != 0;
            if plm == 0xffff_ffff && plm_entry != 0xffff_ffff {
                self.last_cpuctl = Some(cpuctl);
                self.last_plm = Some(plm);
                self.booter_done = true;
                self.note("GA102 payload verified: PLM flip, no halt expected (fill self-loop tail)");
                return Ok(());
            }
            if halted {
                // The stock run reports its own signature error before
                // halting (0x31 = V67 is not a valid RSA signature).
                self.gpu.force_write(SEC2_MAILBOX0, 0x31);
                return Err(PhaseError::new("verify", format!(
                    "early PLM failed: halt before flip (cpuctl={cpuctl:#x} plm={plm:#x} entry={plm_entry:#x}) - signature DMA missed the stack"
                )));
            }
            return Err(PhaseError::new("verify", format!(
                "ETIMEDOUT: PLM never flipped while SEC2 kept running (cpuctl={cpuctl:#x} plm={plm:#x}) - write dropped"
            )));
        }
        // Execution evidence: marker into mailbox 0, HALT in CPUCTL, PLM open.
        if !self.faults.marker_missing {
            self.gpu.force_write(SEC2_MAILBOX0, MARKER_OK);
        }
        if self.faults.halt_not_set {
            self.gpu.force_write(regs::SEC2_CPUCTL, 0); // still running
        }
        let marker = self.gpu.read(SEC2_MAILBOX0);
        let cpuctl = self.gpu.read(regs::SEC2_CPUCTL);
        let plm = self.gpu.read(regs::FEAT_PLM);
        self.last_marker = Some(marker);
        self.last_cpuctl = Some(cpuctl);
        self.last_plm = Some(plm);
        let marker_seen = marker == MARKER_OK;
        let halt_seen = cpuctl & 0x10 != 0;
        let plm_open = plm == 0xffff_ffff;
        if !(marker_seen && halt_seen && plm_open) {
            return Err(PhaseError::new("verify", format!(
                "triad incomplete: marker={marker:#x} halt={halt_seen} plm={plm:#x}"
            )));
        }
        self.booter_done = true;
        self.note("execution triad verified (marker+halt+plm)");
        Ok(())
    }

    fn selectors(&mut self) -> Result<(), PhaseError> {
        gpu_write_verified(self.gpu, regs::FEAT_SS0, 0x8888_8888)
            .map_err(|e| PhaseError::new("selectors", format!("SS0: {e}")))?;
        gpu_write_verified(self.gpu, regs::FEAT_SS1, 0x8)
            .map_err(|e| PhaseError::new("selectors", format!("SS1: {e}")))?;
        self.note("selectors written and read back");
        Ok(())
    }

    /// Stock signature restoration before FLR (task brief §9.5: the V67
    /// payload must NEVER remain published for the stock boot path).
    /// Verified by read-back, like everything else.
    fn signature_restore(&mut self) -> Result<(), PhaseError> {
        // The payload publish put the V67 image into the meta sig slot;
        // restoration swaps the stock signature back in and re-verifies.
        let stock: Vec<u8> = (0..384u32).map(|i| (i ^ 0xA5) as u8).collect();
        let landed = if self.faults.signature_restore_fail {
            vec![0u8; 384] // restore silently failed
        } else {
            stock.clone()
        };
        if landed != stock {
            return Err(PhaseError::new("signature", "stock signature not restored (V67 would remain published)"));
        }
        self.note("stock signature restored and verified");
        Ok(())
    }

    fn flr_and_post(&mut self) -> Result<(), PhaseError> {
        if self.faults.fail_flr {
            return Err(PhaseError::new("flr", "FLR failed (EIO analog)"));
        }
        // FLR: reset propagation, WPR2 cleared, selectors survive (AON island).
        self.gpu.force_write(regs::WPR2_LO, 0);
        self.gpu.force_write(regs::WPR2_HI, 0);
        self.gpu.force_write(regs::SEC2_CPUCTL, regs::CPUCTL_HALTED);
        if self.faults.post_flr_alien_wpr {
            // Fault model: firmware writes an alien WPR2 during FLR.
            self.gpu.force_write(regs::WPR2_LO, 0x1000);
            self.gpu.force_write(regs::WPR2_HI, 0x2000);
        }
        let (lo, hi) = (self.gpu.read(regs::WPR2_LO), self.gpu.read(regs::WPR2_HI));
        if lo != 0 || hi != 0 {
            return Err(PhaseError::new("post_flr", format!("alien WPR2 {lo:#x}/{hi:#x}")));
        }
        // Selectors survive FLR (FEAT island is AON — task brief §3.2);
        // replay never wrote them, so the check is payload-only.
        if self.payload && self.gpu.read(regs::FEAT_SS0) != 0x8888_8888 {
            return Err(PhaseError::new("post_flr", "SS0 lost after FLR (expected AON survival)"));
        }
        self.note("post-FLR verified");
        Ok(())
    }

    /// Full unwind: free DMA, restore BME. Nothing may leak.
    fn unwind(&mut self, mut result: Result<(), PhaseError>) -> Result<(), PhaseError> {
        // Freeing is fallible (fail_free_at = 1-based index): a failed free
        // stops the drain, leaks the rest, and reports the unwind phase -
        // this is the branch the old unconditional drain could never reach.
        let n = self.dma_live;
        for k in 0..n as u32 {
            if self.faults.fail_free_at == Some(k + 1) {
                break;
            }
            self.dma_live -= 1;
            self.dma_frees += 1;
        }
        if self.bme_set {
            self.bme_set = false; // clear BME before handover
        }
        if self.dma_live != 0 {
            let e = PhaseError::new("unwind", "DMA leak");
            result = result.and(Err(e));
        }
        result
    }

    pub fn run(&mut self) -> Result<Report, PhaseError> {
        let result = (|| {
            self.claim()?;
            self.gfw_wait()?;
            self.fwsec()?;
            self.booter_fire_and_verify()?;
            if self.payload {
                self.selectors()?;
                self.signature_restore()?;
            }
            self.flr_and_post()?;
            Ok(())
        })();
        let unwound = self.unwind(result);
        unwound.map(|_| Report {
            dma_pages_allocated: self.dma_allocs,
            dma_pages_freed: self.dma_frees,
            fwsec_fired: self.fwsec_done,
            booter_fired: self.booter_done,
            marker_seen: self.last_marker == Some(MARKER_OK),
            // GA102 canonical: no halt will ever come (self-loop tail) -
            // its absence is the design, not a fault.
            halt_seen: matches!(self.last_cpuctl, Some(c) if c & 0x10 != 0),
            plm_open_verified: self.last_plm == Some(0xffff_ffff),
            selectors_written: self.payload,
            bme_restored: !self.bme_set,
            flr_done: !self.faults.fail_flr,
            signature_restored: !self.payload || !self.faults.signature_restore_fail,
            unwind_clean: self.dma_live == 0 && !self.bme_set,
        })
    }
}

fn gpu_write_verified(gpu: &mut FakeGpu, off: u32, v: u32) -> Result<u32, &'static str> {
    gpu.write(off, v);
    let back = gpu.read(off);
    if back != v {
        return Err("write did not stick");
    }
    Ok(back)
}
