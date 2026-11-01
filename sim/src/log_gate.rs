//! Quiet-mode console gate (issue #38): reference matcher + shape tests.
//!
//! `loader/src/logging.rs`'s `CONSOLE_QUIET_MARKERS`, `console_visible` and
//! `quiet_effective` must stay textually identical to the copies here
//! (loader is UEFI-target-only and cannot run host tests; sim is the test
//! bench).
//!
//! The screen budget this buys: a TU106 tx run logs 415+ flushed lines and
//! slow iGPU GOPs render ~1 line/s - the whitelist keeps the operator-
//! facing verdicts (banner, confirm, per-card target/RESULT/HANDOFF,
//! FINAL, CHAIN) and drops the DETACH/topology/RECONNECT chatter that only
//! the evidence file needs.

pub const CONSOLE_QUIET_MARKERS: &[&str] = &[
    "nv-uefi-unlocker", "TX BUILD", // version + build-class banner
    "PROMPT", "CONFIRM", // interaction
    "PHASE", "SELECTOR", "HANDOFF", // stage boundaries
    "RESULT", "FINAL", "TX:", "CHAIN", "QUIET", // verdicts
];

pub fn console_visible(line: &str) -> bool {
    CONSOLE_QUIET_MARKERS.iter().any(|m| line.contains(m))
}

pub fn quiet_effective(request: bool, file_open: bool) -> bool {
    request && file_open
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_and_interaction_stay_visible() {
        assert!(console_visible(
            "[     0ms] INFO : nv-uefi-unlocker v2.1.9 (built 2026-11-01)"
        ));
        assert!(console_visible(
            "[     1ms] INFO :  ** TX BUILD: CONTAINS GPU-WRITE PATHS **"
        ));
        assert!(console_visible(
            "[     2ms] INFO : PROMPT: PRESS SPACE or ENTER to CONTINUE; other key / 30s no-key = ABORT (unlocks NOTHING)"
        ));
        assert!(console_visible(
            "[  3012ms] INFO : CONFIRM: Enter pressed, proceeding"
        ));
        assert!(console_visible("[  3013ms] INFO : PHASE: CONFIRM"));
    }

    #[test]
    fn verdicts_stay_visible() {
        // Per-card verdicts - the quiet screen must answer "did it work".
        assert!(console_visible(
            "[ 45123ms] INFO : TX: ===== target 1/1: 10de:1f07 @ 0000:01:00.0 ====="
        ));
        assert!(console_visible(
            "[ 45124ms] INFO : TX: device 10de:1f07 -> asset family 'tu10x'"
        ));
        assert!(console_visible(
            "[ 45125ms] INFO : TX: aborted on warm/resume entry before any GPU write"
        ));
        assert!(console_visible(
            "[ 45126ms] INFO : TX: transaction failed: EBusy - NOT retrying this session"
        ));
        assert!(console_visible(
            "[ 98000ms] INFO : SELECTOR RESULT: transaction complete"
        ));
        assert!(console_visible("[ 98001ms] INFO : FINAL: read-only run complete"));
        assert!(console_visible(
            "[ 98002ms] INFO : CHAIN: failed: NotFound (no bootmgfw on this stick)"
        ));
    }

    #[test]
    fn chatter_is_suppressed() {
        // The lines the evidence file keeps but the quiet screen drops.
        assert!(!console_visible(
            "[ 10000ms] INFO : DETACH: stage GOP handle Handle(7f12) protocols 5"
        ));
        assert!(!console_visible(
            "[ 10001ms] INFO : DIAG GOP-TOPOLOGY: Handle(7f12) devpath_len=36 on_target=true protocols=3"
        ));
        assert!(!console_visible(
            "[ 10002ms] INFO : DETACH: config baseline (vendor=0x10de dev=0x1f07 cmd=0x0007 - MEM=true BME=false)"
        ));
        assert!(!console_visible(
            "[ 10003ms] INFO : RECONNECT: ConnectController ok (recursive=true)"
        ));
        assert!(!console_visible(
            "[     4ms] INFO : WATCHDOG: disabled"
        ));
        assert!(!console_visible(
            "[ 10004ms] INFO : PM: entry PMCSR=0x0000 state=D0 (cap@0x78)"
        ));
    }

    #[test]
    fn timestamp_prefix_never_affects_matching() {
        // The markers live in the message; a missing or differently padded
        // stamp must not flip any decision.
        assert!(console_visible("SELECTOR RESULT: transaction complete"));
        assert!(!console_visible("DIAG GOP-TOPOLOGY: no GOP handles"));
        assert!(console_visible(
            "[9999999ms] INFO : PHASE: CONFIRM"
        ));
    }

    #[test]
    fn quiet_requires_a_live_log_file() {
        // set_quiet's contract: the request is refused (console stays
        // complete) when the file could not be opened.
        assert!(quiet_effective(true, true));
        assert!(!quiet_effective(true, false));
        assert!(!quiet_effective(false, true));
        assert!(!quiet_effective(false, false));
    }
}
