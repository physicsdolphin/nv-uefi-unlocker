//! USB-application form: banner -> single-key confirm (30s timeout) -> PCI
//! enumeration (read-only) -> chain-load Windows Boot Manager. Zero GPU
//! writes by default; `tx` builds are the gated artifacts. The
//! firmware-resident DXE driver form is `dxe_main.rs`; both share
//! everything through `lib.rs` and the `runner` phase sequence.

#![no_main]
#![no_std]

extern crate alloc;
extern crate log;

use log::info;
use loader::{banner, chainload, keywait, logging, runner, timing, VERSION};
use uefi::prelude::*;
use uefi::CStr16;

/// CONFIRM semantics switch (armed mode): when this file exists on the
/// stick root, the operator has declared intent ON THE STICK ITSELF - the
/// wait becomes a short countdown that auto-proceeds, any key aborts. The
/// first marker line may set the countdown seconds (1..=30, default 5).
const ARMED_MARKER: &CStr16 = cstr16!(r"\i_know_what_i_am_doing.txt");
/// Issue #38: verdict-lines-only console output. Some iGPU GOPs render
/// ~1 line/s, so the full console mirror cost minutes per run; the log
/// file always records everything either way.
const QUIET_MARKER: &CStr16 = cstr16!(r"\tx-quiet.txt");
/// Issue #38 follow-up: count-based log-flush cadence knob (0..=4096
/// lines, 0 = stage-marker lines only; default 16). Marker-line flushes
/// are unaffected - they are the crash-forensics anchors.
const FLUSH_LINES_MARKER: &CStr16 = cstr16!(r"\tx-flush-lines.txt");
const CONFIRM_WAIT: u64 = 30;

#[entry]
fn main() -> Status {
    timing::init();
    logging::init(VERSION);
    // Phase: QUIET gate (issue #38). The stick opts in via the marker;
    // refused when the log file could not be opened (console = only
    // record, it must stay complete).
    if chainload::boot_volume_has(QUIET_MARKER) {
        if logging::set_quiet(true) {
            info!(
                "QUIET: \\tx-quiet.txt present - console shows verdict lines only; every line still lands in \\loader.log"
            );
        } else {
            info!(
                "QUIET: \\tx-quiet.txt present but the log file is unavailable - console stays complete (it is the only record)"
            );
        }
    }
    // The boot manager arms a 5-minute watchdog before StartImage; a slow
    // or stalled transaction must not be cut short by a firmware reset.
    match uefi::boot::set_watchdog_timer(0, 0x10000, None) {
        Ok(()) => info!("WATCHDOG: disabled"),
        Err(e) => info!("WATCHDOG: disable failed: {:?}", e.status()),
    }
    banner::print(VERSION);

    // Phase: FLUSH knob (issue #38 follow-up). Optional per-stick cadence
    // for the count-based log flushes; stage-marker flushes are unaffected.
    if let Some(content) = chainload::boot_volume_read(FLUSH_LINES_MARKER) {
        let n = chainload::parse_flush_lines(&content);
        logging::set_flush_every_lines(n as usize);
        if n == 0 {
            info!("FLUSH: \\tx-flush-lines.txt - count-based flush off (stage-marker lines only)");
        } else {
            info!("FLUSH: \\tx-flush-lines.txt - every {n} lines");
        }
    }

    // Phase: CONFIRM. TX builds: ONLY Space or Enter proceeds - timeout or
    // any other key aborts into chain-load with zero GPU writes (issue #4.1:
    // the old code proceeded on any key AND on timeout, so a stick left in a
    // machine could run the transaction unattended). Read-only builds keep
    // the M0 semantics. Armed mode (marker file on the stick root) inverts
    // this: an N-second countdown auto-proceeds (first marker line sets N,
    // 1..=30, default 5), any key aborts - the file is the operator's
    // explicit declaration of intent, so unattended runs are opt-in per
    // stick, never a default. TX builds run the countdown WITH the read-only
    // prep interleaved (enumerate / GOP probe / sample / select / asset
    // reads into the tx.rs read-through cache): the first GPU touch still
    // happens only after PROCEED, and aborting mid-prep writes nothing.
    let confirmed = if let Some(content) = chainload::boot_volume_read(ARMED_MARKER) {
        let secs = chainload::parse_armed_seconds(&content);
        info!(
            "CONFIRM: \\i_know_what_i_am_doing.txt present - ARMED: {secs}s countdown auto-runs, ANY key aborts"
        );
        #[cfg(feature = "tx")]
        {
            loader::tx_prep::armed_confirm_with_prep(secs)
        }
        #[cfg(not(feature = "tx"))]
        {
            keywait::wait_confirm_armed(secs)
        }
    } else {
        info!("CONFIRM: no armed marker - Space/Enter/{CONFIRM_WAIT}s confirm semantics");
        keywait::wait_confirm(CONFIRM_WAIT)
    };

    // Phase: ENUMERATE / GOP-PROBE / SAMPLE / TX (shared with the DXE form).
    runner::phases(confirmed);

    // Phase: CHAIN. Read-only LoadImage/StartImage of the ESP bootmgfw.efi.
    // The verdict lines above are in \loader.log; no pause before handing
    // off (the log file is the evidence record, not the screen).
    match chainload::chain_load_bootmgfw() {
        Ok(()) => info!("CHAIN: bootmgfw returned unexpectedly"),
        Err(e) => info!("CHAIN: failed: {:?}", e.status()),
    }

    info!("FINAL: read-only run complete; nothing was written to the GPU, nothing to restore. Safe to reboot.");
    logging::finish();
    Status::SUCCESS
}
