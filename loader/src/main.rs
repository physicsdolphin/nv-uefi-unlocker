//! M0 skeleton: banner -> single-key confirm (30s timeout) -> PCI enumeration
//! (read-only) -> chain-load Windows Boot Manager. Zero GPU writes by design.

#![no_main]
#![no_std]

extern crate alloc;
extern crate log;

use log::info;
use uefi::prelude::*;
use uefi::CStr16;

mod acpi_dump;
mod banner;
mod chainload;
mod gop;
mod keywait;
mod logging;
mod pci;
mod sample;
mod identity;
mod identity_core;
mod timing;
mod usbreset;
#[cfg(feature = "tx")]
mod tx;
#[cfg(feature = "tx")]
mod tx_policy;
#[cfg(feature = "tx")]
mod gop_native;
#[cfg(feature = "tx")]
mod gop_order;
#[cfg(feature = "tx")]
mod claim_plm;
#[cfg(feature = "tx")]
mod tx_fwsec;
#[cfg(feature = "tx")]
mod tx_booter;
#[cfg(feature = "tx")]
mod tx_detach;
#[cfg(feature = "tx")]
mod tx_portal;
#[cfg(feature = "tx")]
mod tx_prep;

const VERSION: &str = env!("CARGO_PKG_VERSION");

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

/// \tx-ultrafast.txt: verified-machine speed profile. Skips every purely
/// diagnostic pre/post probe (M1 sample, GOP probe, AUTH-SCAN, ROOT-DIR
/// dump, opener/topology dumps, post-payload FEAT sweep, HANDOFF-PCI /
/// HANDOFF-VGA-FINAL dumps, POST-GOP probe, CHAIN-MEMMAP) and folds in
/// quiet + no-canary + stage-marker-only flush (an explicit
/// \tx-flush-lines.txt still wins). NOT skippable and always kept: the
/// verdict readbacks - selector write/readback, WPR2/POST-BOOT verdicts,
/// booter load audit - because readback is the only success criterion
/// (constitution #1).
static ULTRAFAST: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Read by every diagnostic-probe site across the crate; set once in
/// `main` before the logger exists (plain marker probe, same shape as the
/// log-commit ladder above).
pub fn ultrafast() -> bool {
    ULTRAFAST.load(core::sync::atomic::Ordering::Relaxed)
}

fn set_ultrafast(v: bool) {
    ULTRAFAST.store(v, core::sync::atomic::Ordering::Relaxed);
}

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

    // Phase: ENUMERATE (read-only). ALL NVIDIA display adapters, in
    // firmware enumeration order. The armed prep (TX builds) already ran
    // this, the GOP probe and the sample inside the countdown window.
    #[cfg(feature = "tx")]
    let (gpus, prep_ran) = if let keywait::Confirm::Proceed = confirmed {
        match tx_prep::take_gpus() {
            Some(g) => (g, true),
            None => (pci::enumerate(), false),
        }
    } else {
        (pci::enumerate(), false)
    };
    #[cfg(not(feature = "tx"))]
    let gpus = pci::enumerate();
    #[cfg(not(feature = "tx"))]
    let prep_ran = false;

    // Phase: GOP-PROBE (read-only; which console driver owns which GPU).
    if !prep_ran {
        if ultrafast() {
            info!("GOP-PROBE: ultrafast - skipped");
        } else {
            gop::enumerate_gop();
        }
    }

    // Phase: SAMPLE (M1: read-only cold-state register sweep). Bug fix:
    // this used to always run gpus.first(), ignoring tx-select-bdf.txt - on
    // a multi-GPU box the read-only sweep could silently sample a different
    // card than the one the TX phase pins. select_sample_targets mirrors
    // select_tx_targets' marker semantics so both phases agree.
    if !prep_ran {
        if ultrafast() {
            info!("SAMPLE: ultrafast - skipped");
        } else {
            let targets = pci::select_sample_targets(&gpus);
            if targets.is_empty() {
                info!("SAMPLE: no NVIDIA GPU selected or present, skipped (negative control path)");
            } else {
                for (idx, gpu) in targets.iter().enumerate() {
                    if targets.len() > 1 {
                        info!(
                            "SAMPLE: target {}/{} (10de:{:04x} @ {:04x}:{:02x}:{:02x}.{})",
                            idx + 1,
                            targets.len(),
                            gpu.device_id,
                            gpu.seg,
                            gpu.bus,
                            gpu.dev,
                            gpu.fun
                        );
                    }
                    sample::run(gpu);
                }
            }
        }
    }

    // Phase: TX (feature-gated transaction; only in explicit tx builds).
    // NOTE: inside tx_phase each card's console drivers are detached
    // (TOTORONG craft) - the screen goes dark from that card's transaction
    // until Windows boots. The log file keeps recording and is the
    // evidence record. Multi-GPU: every supported card runs in order.
    // P2-7: LoadImage the OS loader before the transaction touches the GPU.
    // OPT-IN since #198 (see chainload::preload_os_loader): the preloaded
    // image is the one variable that separates every post-handoff freeze
    // report from the surviving runs, so the default is the legacy
    // post-transaction chain. The opt-in path keeps the PE header probe and
    // falls back to the legacy chain on any doubt. Pure chain-side behavior
    // (no GPU writes), so it exists in every build.
    let preloaded = if let keywait::Confirm::Proceed = confirmed {
        // PR #209 follow-up: with \tx-exit-fw.txt the StartImage never runs,
        // so the pre-transaction USB read would buy nothing - skip it.
        if chainload::boot_volume_has(cstr16!(r"\tx-exit-fw.txt")) {
            None
        } else {
            chainload::preload_os_loader()
        }
    } else {
        None
    };

    #[cfg(feature = "tx")]
    let hard_stop = if let keywait::Confirm::Proceed = confirmed {
        // \tx-dry-run.txt (issue #146): skip the transaction entirely. Since
        // preload went opt-in (#198) the residency control is the PAIR
        // \tx-dry-run.txt + \tx-preload.txt: the loader is resident, but this
        // path never reaches an asset read, so the cache stays EMPTY - "preload
        // residency, zero GPU writes, no detach, no reconnect", not "full
        // residency". (Measured: the dry-run log has no TX-ASSET-DIAG line.)
        if chainload::boot_volume_has(cstr16!(r"\tx-dry-run.txt")) {
            info!("TX-DRY-RUN: \\tx-dry-run.txt present - the GPU transaction is SKIPPED this boot (zero writes, no detach; preload residency control - assets are NOT read on this path, issue #146)");
            false
        } else {
            tx_phase(&gpus)
        }
    } else {
        false
    };
    #[cfg(not(feature = "tx"))]
    {
        drop(confirmed);
    }
    #[cfg(not(feature = "tx"))]
    let hard_stop = false;

    // Opt-in STOP-on-hard-failure (see tx_one_card / \tx-stop-on-failure.txt):
    // a hard TX failure with that marker present means this session must not
    // act on the card again or hand off on its own - the firmware may still
    // be mid-operation. No further GPU interaction, no chain-load; recovery
    // is manual (full power-off, remove the stick, normal boot).
    if hard_stop {
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
    match chainload::chain_load_bootmgfw(preloaded) {
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


/// Die-family-pure asset mapping by PCI device ID (constitution 5: no
/// cross-family payloads — a GA102 payload on AD102 HALTs and opens
/// nothing). Names on the boot volume: <asset>_<family>.bin.
///
/// 2026-10-28: the whitelist is OPEN for the full NVIDIA driver list
/// of the three transaction families (pinned open-gpu-kernel-modules
/// README "Compatible GPUs" table, 217 IDs, cross-checked against
/// upstream's supported_ids[]). The blocks are die-family-pure:
///   0x1e02..0x1f80 = Turing tu10x (TU102/104/106: 20-series, TITAN/
///     Quadro RTX, CMP 40/50HX), 0x1f81..0x1fff + 0x2182..0x21d1 = Turing
///     tu11x (TU116/117: GTX 16 series + CMP 30HX — die-keyed booter
///     archive, issue #151 route);
///   0x2200..=0x223f, 0x2400..=0x25ff = Ampere GA10x (GA102 3090/3080/
///     CMP 90HX/A6000-class, GA103 2414, GA104 3070s/3070Ti/CMP 70HX/A4500,
///     GA106/107 3060/3050 lines + mobile; 0x23xx = Hopper, absent
///     on purpose). The 0x2240..=0x22ff tail was carved out (2026-11-02,
///     pci.ids-verified): it holds no GA102 cores - only GA104/GA106/GA107
///     HD-audio functions (228b/228e/2291), Ada/Blackwell audio functions
///     (22ba..=be/22e8..=ec), Tegra/Orin/THOR/GB10 ARM blocks
///     (2296..=9e/22ce..=d1/22d8/22e6) and GH100 H100 NVSwitch (22a3 -
///     Hopper, the same cross-family class as the removed 20c2/20f1).
///     The class-03/function-0 enumeration filter already skips every
///     one of these; the cap is defense-in-depth;
///   0x2600..0x28ff = Ada AD10x (GeForce 40-series desktop+mobile,
///     RTX-workstation/L-class, the AD106M "RTX 3050 A", and
///     upstream-unlisted IDs such as the 4070M modded-card 2820).
///   0x2bxx+ = Blackwell, out of scope. GA100 (0x20b0 A100, 0x20c2,
///     0x20f1 A30 per cmp170hx p2p inventory) is NOT GA10x and stays
///     outside the Ampere ranges - the 20c2/20f1 arms that previously
///     routed to ga102 were a cross-family hazard and are removed.
/// Unverified IDs now run instead of being refused: per-ID evidence
/// tiers live in the README matrix, and a first-silicon report
/// upgrades the row (issue #8 process). Shared by tx_one_card and the
/// armed-prep asset step.
#[cfg(feature = "tx")]
fn asset_family(dev: u16) -> Option<&'static str> {
    match dev {
        0x2600..=0x28ff => Some("ad102"),
        0x2200..=0x223f | 0x2400..=0x25ff => Some("ga102"),
        // Turing splits per die-keyed booter archive (issue #151): the
        // driver ships TU102 (tu10x: TU102/104/106) and TU116 (tu11x:
        // TU116/117, GTX 16 series + CMP 30HX) BooterLoadUcode archives;
        // 1f81 is the first TU117 ID, so the 1exx span splits there.
        0x1e02..=0x1f80 => Some("tu10x"),
        0x1f81..=0x1fff | 0x2182..=0x21d1 => Some("tu11x"),
        _ => None,
    }
}

/// Outcome of one card's transaction attempt (see `tx_one_card`).
#[cfg(feature = "tx")]
enum CardOutcome {
    /// Gated out before any GPU touch - no assets, no family mapping, PciIo
    /// open failure, or the (opt-in) identity gate refused it.
    NotAttempted,
    /// Reached `run_transaction`, whatever its outcome - upstream's existing
    /// "attempted" telemetry (issue #139).
    Attempted,
    /// Opt-in `\tx-stop-on-failure.txt` is present and a genuine hard
    /// failure occurred: the whole multi-GPU run stops here, and `main`
    /// must not chain-load Windows either.
    HardStop,
}

/// Returns `true` iff a `CardOutcome::HardStop` occurred, in which case the
/// caller (`main`) must not chain-load Windows.
#[cfg(feature = "tx")]
fn tx_phase(gpus: &[pci::FoundGpu]) -> bool {
    let targets = pci::select_tx_targets(gpus);
    if targets.is_empty() {
        info!("TX: no transaction target (selection marker refused, or no supported GPU) - chain-loading unchanged Windows");
        return false;
    }
    tx::list_root_entries();
    let mut attempted = 0usize;
    let mut hard_stop = false;
    for (i, g) in targets.iter().enumerate() {
        info!(
            "TX: ===== target {}/{}: 10de:{:04x} @ {:04x}:{:02x}:{:02x}.{} =====",
            i + 1,
            targets.len(),
            g.device_id,
            g.seg,
            g.bus,
            g.dev,
            g.fun
        );
        match tx_one_card(g) {
            CardOutcome::NotAttempted => {}
            CardOutcome::Attempted => attempted += 1,
            CardOutcome::HardStop => {
                attempted += 1;
                hard_stop = true;
                info!(
                    "TX: hard stop on target {}/{} - the remaining {} target(s) will NOT run this session",
                    i + 1,
                    targets.len(),
                    targets.len() - (i + 1)
                );
                break;
            }
        }
    }
    let skipped = targets.len() - attempted;
    // "processed" used to mean "walked past", which read as success in a report
    // where no transaction ran at all (issue #139). Say what actually happened.
    info!(
        "TX: multi-GPU pass complete ({} target(s): {attempted} attempted, {skipped} skipped - per-card verdicts above)",
        targets.len()
    );
    if attempted == 0 {
        info!(
            "TX: FINAL: 0 card(s) unlocked - no transaction was attempted (reason(s) above); \
nothing was written, nothing was detached"
        );
    }
    // The asset cache is freed on the way into the chain, not here
    // (chainload::chain_load_bootmgfw -> tx::handoff_free_assets, default
    // since v2.1.11): the abort path and \tx-dry-run.txt need the same
    // hygiene, and CHAIN-MEMMAP logs the map after the free.
    hard_stop
}

/// One card's full transaction. EALREADY/EWarm/EBusy/EClaimUnreadable are
/// skip-class: nothing was torn down, the multi-GPU loop continues. A
/// generic (hard) failure keeps the field-proven default (emergency FLR,
/// continue to the next card) UNLESS the opt-in `\tx-stop-on-failure.txt`
/// marker is present, in which case it becomes a `CardOutcome::HardStop`
/// instead - see that file's doc comment and the `CardOutcome` enum above.
///
/// Returns `CardOutcome::Attempted` when the transaction was actually
/// ATTEMPTED (the card was eligible, its asset set reached
/// `run_transaction`, and - if the opt-in identity gate is configured - it
/// was verified), `CardOutcome::NotAttempted` when the card was gated out
/// before any GPU touch (no assets / no family mapping / PciIo open
/// failure / identity gate refusal), and `CardOutcome::HardStop` as above.
/// `tx_phase` turns `Attempted`/`NotAttempted` into the summary and the
/// `FINAL: 0 card(s) unlocked` verdict (issue #139).
#[cfg(feature = "tx")]
fn tx_one_card(g: &pci::FoundGpu) -> CardOutcome {
    use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams};
    let proto = unsafe {
        boot::open_protocol::<pci::PciIo>(
            OpenProtocolParams { handle: g.handle, agent: boot::image_handle(), controller: None },
            OpenProtocolAttributes::GetProtocol,
        )
    };
    let Ok(pci) = proto else {
        info!("TX: open PciIo failed");
        return CardOutcome::NotAttempted;
    };
    let gpu = tx::TxGpu { pci: &pci, bar: 0 };

    // Per-family asset selection by PCI device ID - see asset_family.
    let dev = g.device_id;
    let Some(family) = asset_family(dev) else {
        info!("TX: device 10de:{dev:04x} has no asset family mapping - skipping");
        return CardOutcome::NotAttempted;
    };
    info!("TX: device 10de:{dev:04x} -> asset family '{family}'");

    // Stick-completeness precheck (field case: a localhost-built stick made
    // on a 40-series-only machine carries no ga10x assets, so a 30-series
    // card in a multi-GPU target machine is silently skipped with no
    // diagnosis). Name the missing family and the fix before the per-asset
    // loop turns Ok(None) into an unexplained skip.
    let booter_probe = tx::read_asset_fmt("\\booter_load_{}.bin", family);
    if matches!(booter_probe, Ok(None)) {
        info!(
            "TX: family '{family}' assets are NOT at this stick's volume root - this card \
would be skipped. Rebuild with: .\\make-usbroot.ps1 -Family all, then copy the CONTENTS of \
.\\usbroot (the maker's default -Out) to the stick root and keep EFI/BOOT/BOOTX64.EFI: a \
folder one level down is invisible to the loader (issues #139/#142)"
        );
        info!(
            "TX: card 10de:{dev:04x} NOT touched - asset set for family '{family}' absent, \
so no transaction was attempted and nothing was written"
        );
        return CardOutcome::NotAttempted;
    }

    // Assets: `Ok(None)` = the stick does not carry it (skip this card);
    // `Err` = present but unusable (empty / over the size cap / short read /
    // media pulled) - that must abort BEFORE anything is detached, never
    // degrade the run to the other write mode.
    macro_rules! must_asset {
        ($what:literal, $e:expr) => {
            match $e {
                Ok(v) => v,
                Err(()) => {
                    info!(concat!(
                        "TX: ", $what, " is present but unreadable (damaged stick or pulled media)",
                        " - aborting before any GPU interaction"
                    ));
                    return CardOutcome::NotAttempted;
                }
            }
        };
    }
    let booter = must_asset!("booter_load", tx::read_asset_fmt("\\booter_load_{}.bin", family));
    let boot_desc = must_asset!("gsp_boot_desc", tx::read_asset_fmt("\\gsp_boot_desc_{}.bin", family));
    let gsp_boot = must_asset!("gsp_boot", tx::read_asset_fmt("\\gsp_boot_{}.bin", family));
    let gsp_image = must_asset!("fwimage", tx::read_asset_fmt("\\fwimage_{}.bin", family));
    let stock_sig = must_asset!("fwsignature", tx::read_asset_fmt("\\fwsignature_{}.bin", family));
    // \v67-direct-ss.bin override (issues #17/#62): the four-plm payload
    // (b0ec9029…) and the CMP-lineage direct-SS payload (de7288bc…, xrip
    // cmp50hx-unlock blobs/v67_payload.bin — the SAME word table the C
    // reference ran and PR #49's compat path verified on the #17 card)
    // differ byte-wise; everything else in the two chains is hash-identical
    // (booter image, gsp_boot slice, sec2_bl - issue #62 comment
    // 6514938280). This marker swaps ONLY the payload. Same strict
    // semantics as every other asset: present-but-unreadable aborts, it
    // never degrades to the family-pinned payload.
    let v67 = match tx::read_asset("\\v67-direct-ss.bin") {
        Ok(Some(v)) => {
            // The direct-SS payload is the TU10x-lineage word table (xrip
            // cmp50hx-unlock). On any other family it is material
            // mismatch by construction - issue #151's lesson: refusing is
            // the only safe behavior, never feed it across die lines.
            if family != "tu10x" {
                info!("TX: \\v67-direct-ss.bin is TU10x-only (family {family}) - refusing to mix die-keyed material, remove the marker");
                return CardOutcome::NotAttempted;
            }
            Some(v)
        }
        Ok(None) => must_asset!("v67 payload", tx::read_asset_fmt("\\v67-{}-four-plm.bin", family)),
        Err(()) => {
            info!("TX: \\v67-direct-ss.bin is present but unreadable (damaged stick or pulled media) - aborting before any GPU interaction");
            return CardOutcome::NotAttempted;
        }
    };
    let bl = if tx::family_is_turing(family) {
        must_asset!("sec2_bl", tx::read_asset_fmt("\\sec2_bl_{}.bin", family))
    } else {
        None
    };
    let (Some(booter), Some(gsp_boot), Some(gsp_image), Some(stock_sig)) =
        (booter, gsp_boot, gsp_image, stock_sig) else {
        info!("TX: assets missing for family '{family}' (booter_load_*/gsp_boot_*/fwimage_*/fwsignature_*) - skipping");
        return CardOutcome::NotAttempted;
    };
    if v67.is_none() {
        info!("TX: NO V67 file - chain-replay mode (no unlock writes)");
    } else {
        info!("TX: V67 present - unlock mode (family-specific write chain)");
    }
    if tx::family_is_turing(family) && bl.is_none() {
        info!("TX: {} missing \\sec2_bl_{}.bin (WITH_LOADER FWSEC needs the generic falcon BL) - skipping", family, family);
        return CardOutcome::NotAttempted;
    }
    // Turing track enabled 2026-10-27 (user M3 go): the chain is the
    // cmp50hx-proven TU106 recipe - FWSEC BOOT_WITH_LOADER via the generic
    // falcon BL, BOOT_DIRECT booter fire, replay verdict mbox0==0 + WPR2.
    // First fire = chain-replay (no V67 at root) unless the payload file is
    // staged deliberately.
    let assets = tx::TxAssets { family, booter, boot_desc, gsp_boot, gsp_image, stock_sig, v67, bl };
    // Size validation before ANY GPU interaction (issue #4.3): a damaged
    // stick must be refused here - the DMA copies use fixed windows.
    if let Err(e) = tx::validate_asset_sizes(&assets) {
        info!("TX: asset validation failed ({e:?}) - nothing was touched, chain-loading Windows");
        return CardOutcome::NotAttempted;
    }

    // Opt-in pre-transaction identity gate (see identity.rs). Absent
    // \tx-prom-sha256.txt = not configured, proceed exactly as before
    // (field-proven default). Present-but-refused = refuse here, before any
    // GPU interaction - this is NOT skip-class, the operator asked for a
    // check and it failed, so it is reported as not-attempted rather than
    // silently treated the same as "no supported assets".
    let identity = match identity::authorize(&pci) {
        identity::IdentityOutcome::NotConfigured => None,
        identity::IdentityOutcome::Verified(v) => Some(v),
        identity::IdentityOutcome::Refused => {
            info!("IDENTITY: gate configured (\\tx-prom-sha256.txt present) but refused (see above) - transaction skipped before any GPU interaction");
            return CardOutcome::NotAttempted;
        }
    };

    // BridgeMaster guard slot: run_transaction fills it mid-transaction (after
    // WprMeta publication) and releases it on the clean success tail. On a
    // failure return the guard comes back still armed and THIS caller releases
    // it - but only after emergency_flr (post-reset, nothing can be in flight
    // through the bridges). Skip-class errors never reach the enable point, so
    // the slot is None there and the arms below do nothing.
    let mut bridge_master = None;
    match tx::run_transaction(&gpu, &assets, g.handle, identity, &mut bridge_master) {
        Ok(()) => {
            info!("SELECTOR RESULT: transaction complete");
            tx::handoff_out(&pci);
            CardOutcome::Attempted
        }
        // Already-open PLM = already unlocked or not cold: skip (success
        // class, gpu-unlock-ada da96e3f), definitely no FLR - it would wipe
        // a live unlock.
        Err(tx::TxError::EAlready) => {
            info!("TX: already unlocked / not cold (EALREADY) - skipped, nothing to unwind");
            tx::handoff_out(&pci);
            CardOutcome::Attempted
        }
        // The claim read cannot establish whether an unlock is already
        // present. Detach/claim and an optional pre-FLR may have happened;
        // do not add a failure reset merely because this read failed.
        Err(tx::TxError::EClaimUnreadable) => {
            info!("TX: claim PLM transport read failed - transaction skipped, no additional failure reset");
            tx::handoff_out(&pci);
            CardOutcome::Attempted
        }
        // Warm entry with the require-D0 switch: nothing was written, the
        // card was not even reset - the clean "come back after a Restart"
        // outcome. No emergency FLR (FLR from a non-D0 state is exactly the
        // undefined case).
        Err(tx::TxError::EWarm) => {
            info!("TX: aborted on warm/resume entry before any GPU write - chain-loading unchanged Windows (use a Restart or cold boot)");
            tx::handoff_out(&pci);
            CardOutcome::Attempted
        }
        // The cold-state guard refused: WPR2 was already initialized this power
        // cycle, i.e. a driver owned this card before we did. By here the card
        // has already been touched in CONFIG space - detach_and_claim ran first
        // (console drivers disconnected, IO|MEM|BME enabled through the COMMAND
        // register, config baseline captured) - but no MMIO register was
        // written and nothing was reset. No FLR: WPR2-up means a driver
        // initialized the card, and resetting it would be a state change we
        // would then have to reconstruct. The captured baseline is not restored
        // either; the handoff leaves it to the OS driver, which re-initialises
        // the function anyway.
        Err(tx::TxError::EBusy) => {
            info!("TX: refused (EBUSY: WPR2 already initialized this power cycle) - config space touched + console detached by the claim, no MMIO write, no reset, baseline not restored");
            tx::handoff_out(&pci);
            CardOutcome::Attempted
        }
        // Generic/hard failure. Field-proven default (marker absent): run
        // emergency_flr and hand off, same as always. Opt-in
        // \tx-stop-on-failure.txt (see that file + CardOutcome::HardStop):
        // per the safety-constitution discussion in the PR, a reset or a
        // freed resource issued while the firmware may still be
        // mid-operation is itself an uncontrolled write to a card whose
        // state is no longer trusted - so with the marker present, do
        // neither: no FLR, no handoff, no next card, no chain-load. This is
        // a deliberate tradeoff against the default's guaranteed-boot
        // behavior, which is why it is opt-in rather than a silent flip.
        Err(e) => {
            // The failure policy lives in tx_policy (pure, sim-tested); this
            // arm is its EFI-side execution.
            match tx_policy::hard_failure_action(tx::probe_file("\\tx-stop-on-failure.txt")) {
                tx_policy::HardFailureAction::Stop => {
                    info!("TX: transaction failed: {e:?} - opt-in STOP (\\tx-stop-on-failure.txt present): no FLR, no handoff, no next card, no chain-load this session");
                    // The claim muted the console mirror; the failure path's
                    // re-enable lives in handoff_out - run it here too so the
                    // frozen mid-transaction state is at least logged and
                    // visible on whatever console surface still exists. The
                    // GOP itself is NOT reconnected: that would be acting
                    // again on a card whose state is no longer trusted - the
                    // screen may stay dark and loader.log is the sole record.
                    tx::handoff_out(&pci);
                    // Bridge release = tx_policy::BridgeReleaseOnFailure::
                    // Never on this path: clearing bus-master under
                    // possibly-live DMA is the same uncontrolled write the
                    // FLR would be, and the machine is about to halt for a
                    // power cycle, so nothing downstream observes the bridges.
                    CardOutcome::HardStop
                }
                tx_policy::HardFailureAction::FlrAndContinue => {
                    info!("TX: transaction failed: {e:?} - NOT retrying this session");
                    // Field report (issue #74): on a TB4/USB4 eGPU the unwind FLR can
                    // leave the function off the bus afterwards - Windows then shows
                    // CM_PROB_PHANTOM, fans stop, and the only recovery measured on
                    // that seat is an AC cycle of the enclosure. Say so, or the first
                    // person to see it concludes the card is dead.
                    info!("TX: recovery note: if this card sits in an eGPU enclosure and disappears from the bus after this failure, power-cycle the enclosure (PSU off ~30 s); an internal card needs a normal power cycle");
                    // Failure unwind: FLR wipes our half-states (engine resets,
                    // BCR, DMA queues) so the Windows driver gets a cold-boot
                    // card. Bridge release = tx_policy::AfterFlr on this path:
                    // only AFTER the FLR - post-reset nothing can be in flight
                    // through the bridges, so clearing is exactly as safe as on
                    // the success tail; clearing earlier would be an
                    // uncontrolled write under possibly-live DMA (booter 0x89
                    // family), and never clearing would carry our bus-master
                    // bits into Windows (dirty-BME handoff shape, #3/#9).
                    tx::emergency_flr(g.handle);
                    if let tx_policy::BridgeReleaseOnFailure::AfterFlr =
                        tx_policy::bridge_release_on_failure(tx_policy::HardFailureAction::FlrAndContinue)
                    {
                        if let Some(bm) = bridge_master.as_mut() {
                            bm.release();
                            info!("TX: bridge BME restored after the failure FLR");
                        }
                    }
                    tx::handoff_out(&pci);
                    CardOutcome::Attempted
                }
            }
        }
    }
}
