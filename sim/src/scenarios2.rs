//! Transaction scenarios (batch 2): full state machine + fault injection.
//! Each scenario asserts BOTH the phase error (when expected) AND the
//! unwind contract (no DMA leak, BME restored) — a failing transaction
//! that leaks resources is a bug, not a fault-path success.

use crate::transaction::{Faults, PhaseError, Transaction};
use crate::{regs, FakeGpu};

fn run(faults: Faults) -> Result<crate::transaction::Report, PhaseError> {
    let mut gpu = FakeGpu::new_locked_cold(None);
    Transaction::new(&mut gpu, faults).run()
}

#[test]
fn t01_full_happy_path() {
    let report = run(Faults::default()).expect("happy path");
    assert!(report.unwind_clean && report.selectors_written && report.flr_done);
    assert_eq!(report.dma_pages_allocated, 2);
    assert_eq!(report.dma_pages_freed, 2);
}

#[test]
fn t02_plm_already_open_refused() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    gpu.force_write(regs::FEAT_PLM, 0xffff_ffff);
    let err = Transaction::new(&mut gpu, Faults::default()).run().unwrap_err();
    assert_eq!(err.phase, "claim");
    assert!(err.reason.contains("EALREADY"));
}

#[test]
fn t03_wpr2_pre_armed_refused() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    gpu.force_write(regs::WPR2_HI, 0x0400_0000);
    let err = Transaction::new(&mut gpu, Faults::default()).run().unwrap_err();
    assert_eq!(err.phase, "claim");
    assert!(err.reason.contains("EBUSY"));
}

#[test]
fn t04_marker_missing_fails_triad_and_unwinds() {
    let err = run(Faults { marker_missing: true, ..Default::default() }).unwrap_err();
    assert_eq!(err.phase, "verify");
    assert!(err.reason.contains("triad"));
}

#[test]
fn t05_halt_not_set_fails_triad() {
    let err = run(Faults { halt_not_set: true, ..Default::default() }).unwrap_err();
    assert_eq!(err.phase, "verify");
}

#[test]
fn t06_selector_write_dropped_fails_with_readback() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    gpu.selector_bad = 1;
    let err = Transaction::new(&mut gpu, Faults::default()).run().unwrap_err();
    assert_eq!(err.phase, "selectors");
    assert!(err.reason.contains("SS0"));
}

#[test]
fn t07_wpr2_geometry_mismatch_detected() {
    let err = run(Faults { wpr2_mismatch: true, ..Default::default() }).unwrap_err();
    assert_eq!(err.phase, "fwsec");
    assert!(err.reason.contains("mismatch"));
}

#[test]
fn t08_alloc_fail_first_page() {
    let err = run(Faults { fail_alloc_at: Some(1), ..Default::default() }).unwrap_err();
    assert_eq!(err.phase, "dma");
}

#[test]
fn t09_alloc_fail_second_page() {
    let err = run(Faults { fail_alloc_at: Some(2), ..Default::default() }).unwrap_err();
    assert_eq!(err.phase, "dma");
}

#[test]
fn t10_set_master_fails_at_claim() {
    let err = run(Faults { master_fail: true, ..Default::default() }).unwrap_err();
    assert_eq!(err.phase, "claim");
    assert!(err.reason.contains("set_master"));
}

#[test]
fn t11_flr_failure_still_unwinds() {
    let err = run(Faults { fail_flr: true, ..Default::default() }).unwrap_err();
    assert_eq!(err.phase, "flr");
}

#[test]
fn t12_bcr_switch_stuck() {
    let err = run(Faults { switch_stuck: true, ..Default::default() }).unwrap_err();
    assert_eq!(err.phase, "booter");
    assert!(err.reason.contains("BCR"));
}

#[test]
fn t13_gfw_never_completes() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    gpu.force_write(regs::GFW_BOOT, 0);
    gpu.gfw_late_reads = 5_000; // beyond the transaction's poll budget
    let err = Transaction::new(&mut gpu, Faults::default()).run().unwrap_err();
    assert_eq!(err.phase, "gfw_wait");
    assert!(err.reason.contains("ETIMEDOUT"));
}

#[test]
fn t14_reset_never_propagates() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    gpu.reset_delay = 100; // beyond the 16-read assert budget
    let err = Transaction::new(&mut gpu, Faults::default()).run().unwrap_err();
    assert_eq!(err.phase, "booter");
    assert!(err.reason.contains("reset"));
}

#[test]
fn t15_post_flr_alien_wpr() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    let mut faults = Faults::default();
    faults.post_flr_alien_wpr = true;
    // Fake the alien value appearing at FLR.
    // (The fake clears WPR2 at force_write(0); emulate alien by making hi non-zero
    // through the fault branch in flr_and_post — modeled as fault -> error.)
    let err = Transaction::new(&mut gpu, faults).run().unwrap_err();
    assert_eq!(err.phase, "post_flr");
}

#[test]
fn t16_ss_drift_does_not_break_writeback() {
    // AD107 measurement: SS0/SS1 cold values drift between boots; the
    // transaction's explicit writes + read-backs are drift-proof.
    let mut gpu = FakeGpu::new_locked_cold(Some((0x1714_2071, 0x5)));
    assert_eq!(gpu.read(regs::FEAT_SS0), 0x1714_2071, "drifted cold value");
    let report = Transaction::new(&mut gpu, Faults::default()).run().expect("drift tolerated");
    assert!(report.selectors_written);
    assert_eq!(gpu.read(regs::FEAT_SS0), 0x8888_8888, "explicit target lands");
    assert_eq!(gpu.read(regs::FEAT_SS1), 0x8);
}

#[test]
fn t17_sec2_halt_mode_fails_triad() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    gpu.boot_mode = crate::BootMode::Halt;
    let err = Transaction::new(&mut gpu, Faults::default()).run().unwrap_err();
    assert_eq!(err.phase, "verify");
}

#[test]
fn t18_unwind_contract_on_every_failure() {
    // A resource leak surfaces as phase "unwind"; every fault must instead
    // report its own phase with the unwind contract honored internally.
    for faults in [
        Faults { marker_missing: true, ..Default::default() },
        Faults { wpr2_mismatch: true, ..Default::default() },
        Faults { fail_alloc_at: Some(2), ..Default::default() },
        Faults { switch_stuck: true, ..Default::default() },
        Faults { fail_flr: true, ..Default::default() },
        Faults { halt_not_set: true, ..Default::default() },
    ] {
        let mut gpu = FakeGpu::new_locked_cold(None);
        let err = Transaction::new(&mut gpu, faults).run().expect_err("fault must fail");
        assert_ne!(err.phase, "unwind", "leak masked the real failure: {err:?}");
    }
}

/// gpu-unlock-ada AD102-family four-write chain on AD107M: single fire
/// opens WPR_CFG (0xfffff0ff — bits [11:8] deliberately clear), FBPA, WPR
/// and FEAT PLMs; read-backs decide, including the not-all-ones value.
#[test]
fn t19_ada_four_write_chain() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    gpu.boot_mode = crate::BootMode::AdaFourWrite;
    let report = Transaction::new(&mut gpu, Faults::default()).run().expect("ada chain");
    assert!(report.unwind_clean);
    assert_eq!(gpu.read(regs::WPRCFG_PLM), 0xffff_f0ff, "WPR_CFG: not all-ones!");
    assert_eq!(gpu.read(regs::FBPA_PLM), 0xffff_ffff);
    assert_eq!(gpu.read(regs::WPR_PLM), 0xffff_ffff);
    assert_eq!(gpu.read(regs::FEAT_PLM), 0xffff_ffff);
    assert_eq!(gpu.read(regs::FEAT_SS0), 0x8888_8888);
    assert_eq!(gpu.read(regs::FEAT_SS1), 0x8);
}

/// Chain-replay mode (Turing M3 first-fire shape): stock booter, no
/// payload. Verdict = mailbox0==0 + HALT + WPR2 up; the FEAT PLM stays
/// locked, no selectors are written, nothing to restore before the FLR.
#[test]
fn t20_replay_happy_path() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    let (plm_cold, ss0_cold) = (gpu.read(regs::FEAT_PLM), gpu.read(regs::FEAT_SS0));
    let report = Transaction::new_replay(&mut gpu, Faults::default()).run().expect("replay happy path");
    assert!(report.unwind_clean && report.flr_done);
    assert!(!report.selectors_written, "replay must not write selectors");
    assert!(!report.marker_seen && !report.plm_open_verified, "no payload triad in replay");
    assert_ne!(plm_cold, 0xffff_ffff, "fixture sanity: cold PLM is not open");
    assert_eq!(gpu.read(regs::FEAT_PLM), plm_cold, "PLM must stay locked in replay");
    assert_eq!(gpu.read(regs::FEAT_SS0), ss0_cold, "replay must not touch SS0");
}

/// Replay with a booter error status (0x89 = WprMeta rejected on DMA
/// readback): the verdict fails on the status alone - read-back is the
/// only success criterion, never the absence of an exception.
#[test]
fn t21_replay_booter_error_status_fails() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    let err = Transaction::new_replay(&mut gpu, Faults { marker_missing: true, ..Default::default() })
        .run()
        .unwrap_err();
    assert_eq!(err.phase, "verify");
    assert!(err.reason.contains("status"), "expected the booter-status verdict: {err:?}");
}

/// Replay where the stock booter never halts: EIO at verify, clean unwind.
#[test]
fn t22_replay_halt_missing_fails() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    let err = Transaction::new_replay(&mut gpu, Faults { halt_not_set: true, ..Default::default() })
        .run()
        .unwrap_err();
    assert_eq!(err.phase, "verify");
    assert!(err.reason.contains("halted"));
}

/// GA102 canonical payload (issue #6 root cause, now the fix's contract):
/// single PLM write + fill self-loop tail. The verdict is the PLM flip -
/// NO marker, NO halt (the core keeps running by design); the triad
/// verdict that failed the GA106M run must never gate this shape.
#[test]
fn t23_ga102_self_loop_plm_flip_happy_path() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    gpu.boot_mode = crate::BootMode::Ga102SelfLoop;
    let report = Transaction::new_ga102_payload(&mut gpu, Faults::default())
        .run()
        .expect("GA102 self-loop payload: PLM flip is the whole verdict");
    assert!(report.unwind_clean && report.selectors_written && report.flr_done);
    assert!(report.plm_open_verified);
    assert!(!report.marker_seen, "GA102 payload writes no marker");
    assert!(!report.halt_seen, "GA102 payload never halts (self-loop)");
    assert_eq!(gpu.read(regs::FEAT_PLM), 0xffff_ffff);
    assert_eq!(gpu.read(regs::FEAT_SS0), 0x8888_8888, "selectors from the open PLM");
    assert_eq!(gpu.read(regs::FEAT_SS1), 0x8);
}

/// GA102 payload missed the stack: the booter executes stock, reports its
/// signature error and HALTs with the PLM still locked - the canonical
/// "early PLM failed" verdict, NOT a success and NOT a timeout.
#[test]
fn t24_ga102_payload_early_halt_is_failure() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    gpu.boot_mode = crate::BootMode::Halt;
    let err = Transaction::new_ga102_payload(&mut gpu, Faults { plm_missing: true, ..Default::default() })
        .run()
        .unwrap_err();
    assert_eq!(err.phase, "verify");
    assert!(err.reason.contains("early PLM failed"), "{err:?}");
    assert_ne!(err.phase, "unwind", "failure must still unwind clean");
}

/// GA102 payload spinning in the fill loop with the write dropped: the
/// wait must time out - a spinning core alone proves nothing.
#[test]
fn t25_ga102_payload_spin_without_flip_times_out() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    gpu.boot_mode = crate::BootMode::SelfLoopNoFlip;
    let err = Transaction::new_ga102_payload(&mut gpu, Faults { plm_flip_missing: true, ..Default::default() })
        .run()
        .unwrap_err();
    assert_eq!(err.phase, "verify");
    assert!(err.reason.contains("ETIMEDOUT"), "{err:?}");
}

/// The old bug as a regression test: running the GA102 self-loop shape
/// through the Ada triad verdict fails a correct run (exactly issue #6).
#[test]
fn t26_ga102_shape_rejected_by_ada_triad_proves_the_shapes_differ() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    gpu.boot_mode = crate::BootMode::Ga102SelfLoop;
    let err = Transaction::new(&mut gpu, Faults::default()).run().unwrap_err();
    assert_eq!(err.phase, "verify");
    assert!(err.reason.contains("triad"), "Ada triad must NOT pass a GA102 self-loop run: {err:?}");
}

/// Report fields must describe the model's state, not the `Faults` the test
/// constructed. These four used to be `true` literals or `!faults.x`; the
/// assertions below are anchored on registers the transaction actually read,
/// so a state machine that skips a phase can no longer report it as done.
#[test]
fn t27_report_fields_come_from_reads_not_faults() {
    // Happy path: every phase ran, the triad was read back.
    let mut gpu = FakeGpu::new_locked_cold(None);
    let report = Transaction::new(&mut gpu, Faults::default())
        .run()
        .expect("happy path");
    assert!(report.fwsec_fired, "fwsec ran");
    assert!(report.booter_fired, "booter ran");
    assert!(report.marker_seen, "marker was read back");
    assert!(report.halt_seen, "HALT was read back");
    assert!(report.plm_open_verified, "PLM was read back open");

    // Replay: stock booter halts with no marker and leaves the PLM locked.
    let mut gpu2 = FakeGpu::new_locked_cold(None);
    let replay = Transaction::new_replay(&mut gpu2, Faults::default())
        .run()
        .expect("replay");
    assert!(replay.fwsec_fired && replay.booter_fired, "both phases still ran");
    assert!(!replay.marker_seen, "no marker in replay - it must not be invented");
    assert!(replay.halt_seen, "the stock booter halts");
    assert!(!replay.plm_open_verified, "replay leaves the PLM locked");
}

// ---------------------------------------------------- issue #4.2 Turing WPR2-up triage

/// Turing mainline entry shape: WPR2 already provisioned, GFW completed,
/// GSP idle - the state the community chains enter through (TU106 measured
/// 2026-11-01: entry BSI handoff=1 of DONE=3, so the handoff is logged but
/// never gated). The triage must accept it (with evidence) and the full
/// transaction must still go green end-to-end.
#[test]
fn t28_turing_wpr2_up_firmware_window_accepts() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    gpu.force_write(regs::WPR2_HI, 0x0400_0000); // provisioned window
    gpu.force_write(regs::BSI_SCRATCH15, 1 << 29); // handoff=1, as measured
    let mut txn = Transaction::new(&mut gpu, Faults::default());
    txn.tu10x_entry = true;
    let report = txn.run().expect("mainline entry must accept and complete");
    assert!(report.unwind_clean && report.flr_done);
    assert!(
        txn.log().iter().any(|l| l.contains("firmware-window state proven")),
        "the accept must carry its evidence in the log"
    );
}

/// Warm/FLR residue: WPR2 up and GFW_BOOT stuck mid-boot (the measured
/// post-reset 0x101 shape). The old code accepted this family-wide; the
/// triage must refuse (-EBUSY analog).
#[test]
fn t29_turing_wpr2_up_gfw_incomplete_refused() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    gpu.force_write(regs::WPR2_HI, 0x0400_0000);
    gpu.force_write(regs::GFW_BOOT, 0x101); // stuck in-progress (measured)
    let mut txn = Transaction::new(&mut gpu, Faults::default());
    txn.tu10x_entry = true;
    let err = txn.run().unwrap_err();
    assert_eq!(err.phase, "claim");
    assert!(err.reason.contains("GFW_BOOT not completed"));
}

/// Same residue state, operator marker present (the
/// \tx-accept-warm-wpr2.txt analog): accepted deliberately, chain green.
#[test]
fn t30_turing_wpr2_up_marker_accepts() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    gpu.force_write(regs::WPR2_HI, 0x0400_0000);
    gpu.force_write(regs::GFW_BOOT, 0x101); // stuck in-progress (measured)
    gpu.gfw_late_reads = 2; // the settling warm state completes right after claim
    let mut txn = Transaction::new(&mut gpu, Faults::default());
    txn.tu10x_entry = true;
    txn.warm_wpr2_marker = true;
    let report = txn.run().expect("operator marker must override the triage");
    assert!(report.unwind_clean);
    assert!(
        txn.log().iter().any(|l| l.contains("operator marker")),
        "the deliberate accept must be logged as such"
    );
}

/// A still-started GSP is a live session regardless of the BSI handoff -
/// the strongest refuse signal the triage has.
#[test]
fn t31_turing_wpr2_up_live_gsp_refused() {
    let mut gpu = FakeGpu::new_locked_cold(None);
    gpu.force_write(regs::WPR2_HI, 0x0400_0000);
    gpu.force_write(regs::BSI_SCRATCH15, 1 << 29); // handoff lies DONE
    gpu.force_write(regs::GFW_CPUCTL, 0x2); // START still set
    let mut txn = Transaction::new(&mut gpu, Faults::default());
    txn.tu10x_entry = true;
    txn.warm_wpr2_marker = true; // even the marker must not override a live GSP
    let err = txn.run().unwrap_err();
    assert_eq!(err.phase, "claim");
    assert!(err.reason.contains("GSP session live"));
}
