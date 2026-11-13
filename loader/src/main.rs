//! USB-application form: banner -> single-key confirm (30s timeout) ->
//! read-only probes -> chain-load Windows Boot Manager. Zero GPU writes by
//! default; `tx` builds are the gated artifacts. The firmware-resident DXE
//! driver form is `dxe_main.rs`; both share everything through `lib.rs` and
//! the `runner` phase sequence.

#![no_main]
#![no_std]

extern crate alloc;
extern crate log;

use loader::{acpi_dump, banner, chainload, keywait, logging, pci, runner, set_ultrafast, timing, ultrafast, usbreset, VERSION};
#[cfg(feature = "tx")]
use loader::tx_prep;
use log::info;
use uefi::prelude::*;
use uefi::CStr16;

/// CONFIRM semantics switch (armed mode): when this file exists on the
/// stick root, the operator has declared intent ON THE STICK ITSELF - the
/// wait becomes a short countdown that auto-proceeds, any key aborts. The
/// first marker line may set the countdown seconds (0..=30, default 5;
/// 0 = no wait, PROCEED as soon as the read-only prep finishes).
const ARMED_MARKER: &CStr16 = cstr16!(r"\i_know_what_i_am_doing.txt");
/// Phantom-key ignore list (one key per line, written as an ABORT line
/// prints it). Loaded before the input settle so it covers every key wait.
const IGNORE_KEY_MARKER: &CStr16 = cstr16!(r"\tx-ignore-key.txt");
/// Issue #38: verdict-lines-only console output. Some iGPU GOPs render
/// ~1 line/s, so the full console mirror cost minutes per run; the log
/// file always records everything either way.
const INPUT_RESET_MARKER: &CStr16 = cstr16!(r"\input-reset.txt");
const QUIET_MARKER: &CStr16 = cstr16!(r"\tx-quiet.txt");
/// Issue #38 follow-up: count-based log-flush cadence knob (0..=4096
/// lines, 0 = stage-marker lines only; default 16). Marker-line flushes
/// are unaffected - they are the crash-forensics anchors.
const FLUSH_LINES_MARKER: &CStr16 = cstr16!(r"\tx-flush-lines.txt");
const CONFIRM_WAIT: u64 = 30;

#[entry]
fn main() -> Status {
    timing::init();
    // diag6 (#122): the log-commit ladder is read from the stick BEFORE the
    // log exists, because the create-commit is itself a suspect. Both probes
    // are read-only directory opens; if a run dies inside them there is no
    // fresh dirent at all - that is the discriminator against a death after
    // the create.
    let commit = if chainload::boot_volume_has(cstr16!(r"\tx-log-commit.txt")) {
        2
    } else if chainload::boot_volume_has(cstr16!(r"\tx-log-commit-lines.txt")) {
        1
    } else {
        0
    };
    set_ultrafast(chainload::boot_volume_has(cstr16!(r"\tx-ultrafast.txt")));
    logging::set_log_disabled(chainload::boot_volume_has(cstr16!(r"\tx-no-log.txt")));
    logging::set_console_off(chainload::boot_volume_has(cstr16!(r"\tx-no-console.txt")));
    logging::set_log_commit(commit);
    logging::init(VERSION);
    // Durable (marker) line: separates a completed logger init from the
    // quiet-probe/watchdog strip that follows (#122).
    info!("BOOT: logger init returned, main continuing");
    if ultrafast() {
        info!("ULTRAFAST: \\tx-ultrafast.txt present - diagnostic probes skipped; quiet + no-canary + marker-only flush folded in; verdict readbacks unchanged");
    }
    info!(
        "BOOT: diag7 log-commit={} (0=v2.1.9 default, 1=+marker Close, 2=+create Close; auto-degrades to 0 if the reopen chain refuses)",
        logging::log_commit()
    );
    // Phase: QUIET gate (issue #38). The stick opts in via the marker;
    // refused when the log file could not be opened (console = only
    // record, it must stay complete).
    if chainload::boot_volume_has(QUIET_MARKER) || ultrafast() {
        if logging::set_quiet(true) {
            info!(
                "QUIET: quiet profile on (\\tx-quiet.txt or \\tx-ultrafast.txt) - console shows verdict lines only; every line still lands in \\loader.log"
            );
        } else {
            info!(
                "QUIET: quiet profile requested but the log file is unavailable - console stays complete (it is the only record)"
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

    // Phase: DIAG identity dumps (#122/#170). Read-only, before anything can
    // hang: names the console's anchor device (GPU-GOP vs VGA/UART) and the
    // USB controller hosting this stick (xHCI vs EHCI) so a hang stays
    // attributable even when the screen freezes.
    if !ultrafast() {
        pci::diag_console_identity();
        pci::diag_boot_volume_controller();
    }
    // Same recursive connect pass the chain phase runs, moved ahead of every
    // enumeration: firmware with a lazy Fast Boot connect policy may only
    // have bound the filesystem surface partially, and #122's 14% early
    // hang logs nothing after the banner - a failed SFS path would look
    // exactly like that. Idempotent; NVIDIA display handles stay skipped.
    chainload::reconnect_block_devices("EARLY");

    // Phase: ACPI-DUMP (\tx-acpi-dump.txt, read-only). Runs before the
    // confirm so an aborted run still produces the tables; independent of
    // any GPU. See acpi_dump.rs for why this exists (power-ownership
    // questions end in the platform AML, FAQ #relock).
    acpi_dump::run_if_marker();

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
    } else if ultrafast() {
        // Explicit cadence marker wins; ultrafast only supplies a default.
        logging::set_flush_every_lines(0);
        info!("FLUSH: ultrafast - count-based flush off (stage-marker lines only)");
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
    // Optional phantom-key ignore list; loaded before any key is read so the
    // settle below and every later wait already honour it. Esc stays usable.
    if let Some(content) = chainload::boot_volume_read(IGNORE_KEY_MARKER) {
        keywait::load_ignore_keys(&content);
    }
    // CONFIRM 之前一次性处理输入:复位/排空/(可选)USB 端口复位/安静窗口。
    // 此时还没有中止窗口,被持续上报的键(如鼠标的键盘接口)不会中止倒计时。
    let usb_list = chainload::boot_volume_read(INPUT_RESET_MARKER).map(|c| usbreset::parse(&c));
    if let Some(l) = &usb_list {
        info!("USB-RESET: input-reset.txt present - {} device(s) listed", l.len());
    }
    keywait::settle_input_once(|| {
        if let Some(l) = &usb_list {
            usbreset::reset_listed(l);
        }
    });
    let confirmed = if let Some(content) = chainload::boot_volume_read(ARMED_MARKER) {
        let secs = chainload::parse_armed_seconds(&content);
        info!(
            "CONFIRM: \\i_know_what_i_am_doing.txt present - ARMED: {secs}s countdown auto-runs, ANY key aborts"
        );
        #[cfg(feature = "tx")]
        {
            tx_prep::armed_confirm_with_prep(secs)
        }
        #[cfg(not(feature = "tx"))]
        {
            keywait::wait_confirm_armed(secs)
        }
    } else {
        info!("CONFIRM: no armed marker - Space/Enter/{CONFIRM_WAIT}s confirm semantics");
        keywait::wait_confirm(CONFIRM_WAIT)
    };

    // Phase: ENUMERATE / GOP-PROBE / SAMPLE / preload / TX - the shared
    // sequence in `runner`, verbatim the same one the firmware-resident DXE
    // driver replays. The armed prep (TX builds) may have already run
    // enumeration, the GOP probe and the sample inside the countdown window.
    let outcome = runner::phases(confirmed);

    // Opt-in STOP-on-hard-failure (see runner::tx_one_card / \tx-stop-on-failure.txt):
    // a hard TX failure with that marker present means this session must not
    // act on the card again or hand off on its own - the firmware may still
    // be mid-operation. No further GPU interaction, no chain-load; recovery
    // is manual (full power-off, remove the stick, normal boot).
    if outcome.hard_stop {
        info!("STOP: hard TX failure with \\tx-stop-on-failure.txt armed - halting without FLR, without trying another GPU, without chain-loading Windows");
        info!("STOP: power off completely, remove the USB stick, then boot normally. Do not retry this session.");
        logging::finish();
        loop {
            uefi::boot::stall(core::time::Duration::from_millis(500));
        }
    }

    // Phase: CHAIN. Read-only LoadImage/StartImage of the ESP bootmgfw.efi.
    // The verdict lines above are in \loader.log; no pause before handing
    // off (the log file is the evidence record, not the screen).
    match chainload::chain_load_bootmgfw(outcome.preloaded) {
        Ok(()) => {
            // PR #209 follow-up: the exit-to-fw marker also returns Ok -
            // distinguish it from a genuine bootmgfw return so the evidence
            // log does not record an anomaly on every marker run.
            if chainload::boot_volume_has(cstr16!(r"\tx-exit-fw.txt")) {
                info!("CHAIN: exit-to-fw marker - handed back to the firmware boot manager (expected)");
            } else {
                info!("CHAIN: bootmgfw returned unexpectedly");
            }
        }
        Err(e) => info!("CHAIN: failed: {:?}", e.status()),
    }

    info!("FINAL: read-only run complete; nothing was written to the GPU, nothing to restore. Safe to reboot.");
    logging::finish();
    Status::SUCCESS
}

