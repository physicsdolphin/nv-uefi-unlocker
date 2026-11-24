//! M0 skeleton: banner -> single-key confirm (30s timeout) -> PCI enumeration
//! (read-only) -> chain-load Windows Boot Manager. Zero GPU writes by design.

#![no_main]
#![no_std]

extern crate alloc;
extern crate log;

use log::info;
use uefi::prelude::*;

mod acpi_dump;
mod banner;
mod chainload;
mod config;
mod ebswatch;
mod gop;
mod keywait;
mod logging;
mod pci;
#[cfg(feature = "diag")]
mod revive;
mod sample;
mod identity;
mod identity_core;
mod timing;
mod usbreset;
#[cfg(feature = "diag")]
mod tx;
#[cfg(feature = "diag")]
mod gop_native;
#[cfg(feature = "diag")]
mod gop_order;
#[cfg(feature = "tx")]
mod claim_plm;
#[cfg(feature = "diag")]
mod tx_fwsec;
#[cfg(feature = "diag")]
mod tx_booter;
#[cfg(feature = "diag")]
mod tx_detach;
#[cfg(feature = "tx")]
mod tx_portal;
#[cfg(feature = "diag")]
mod tx_prep;

const VERSION: &str = env!("CARGO_PKG_VERSION");

// The marker consts that used to live here (armed / ignore-key /
// input-reset / quiet / flush-lines) moved into config:: content and flag
// accessors - each arm reads the ini first, then falls back to the same
// marker file, with double-set refused at boot (issue #290).

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
    // \tx.ini (issue #290): parse the optional consolidated config before
    // anything else - several arms (log-commit, no-log, no-console,
    // ultrafast) must be known before the logger exists. Silent by design;
    // refusals/warnings are collected and replayed right after logger init.
    config::init();
    // diag6 (#122): the log-commit ladder is read from the stick BEFORE the
    // log exists, because the create-commit is itself a suspect. Both paths
    // are read-only file opens; if a run dies inside them there is no
    // fresh dirent at all - that is the discriminator against a death after
    // the create.
    let commit = config::log_commit();
    set_ultrafast(config::ultrafast());
    logging::set_log_disabled(config::flag(config::FlagArm::NoLog));
    logging::set_console_off(config::flag(config::FlagArm::NoConsole));
    logging::set_log_commit(commit as usize);
    logging::init(VERSION);
    // Durable (marker) line: separates a completed logger init from the
    // quiet-probe/watchdog strip that follows (#122).
    info!("BOOT: logger init returned, main continuing");
    if ultrafast() {
        info!("ULTRAFAST: \\tx-ultrafast.txt present - diagnostic probes skipped; quiet + no-canary + marker-only flush folded in; verdict readbacks unchanged");
    }
    // A refused \tx.ini stops the boot HERE: before the watchdog work, the
    // banner, any enumeration or GPU touch. Console mirror forced on so
    // no-console cannot silence a refusal; quiet is not active yet at this
    // point (it is enabled later, after this check).
    if config::refused() {
        logging::set_console_off(false);
        config::log_diagnostics();
        info!(
            "FINAL: \\tx.ini refused - nothing was written to the GPU; fix the lines above and reboot. Handing the machine back to the firmware boot manager."
        );
        logging::finish();
        return Status::SUCCESS;
    }
    config::log_diagnostics();
    info!(
        "BOOT: diag7 log-commit={} (0=v2.1.9 default, 1=+marker Close, 2=+create Close; auto-degrades to 0 if the reopen chain refuses)",
        logging::log_commit()
    );
    // Phase: QUIET gate (issue #38). The stick opts in via the marker;
    // refused when the log file could not be opened (console = only
    // record, it must stay complete).
    if config::quiet() {
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
    if let Some(n) = config::flush_lines() {
        logging::set_flush_every_lines(n as usize);
        if n == 0 {
            info!("FLUSH: flush-lines arm - count-based flush off (stage-marker lines only)");
        } else {
            info!("FLUSH: flush-lines arm - every {n} lines");
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
    if let Some(content) = config::content(config::ContentArm::IgnoreKey) {
        keywait::load_ignore_keys(&content);
    }
    // CONFIRM 之前一次性处理输入:复位/排空/(可选)USB 端口复位/安静窗口。
    // 此时还没有中止窗口,被持续上报的键(如鼠标的键盘接口)不会中止倒计时。
    let usb_list = config::content(config::ContentArm::InputReset).map(|c| usbreset::parse(&c));
    if let Some(l) = &usb_list {
        info!("USB-RESET: input-reset.txt present - {} device(s) listed", l.len());
    }
    keywait::settle_input_once(|| {
        if let Some(l) = &usb_list {
            usbreset::reset_listed(l);
        }
    });
    let confirmed = if let Some(secs) = config::armed() {
        info!(
            "CONFIRM: armed mode (i_know_what_i_am_doing, marker or tx.ini) - {secs}s countdown auto-runs, ANY key aborts"
        );
        #[cfg(feature = "diag")]
        {
            tx_prep::armed_confirm_with_prep(secs)
        }
        #[cfg(not(feature = "diag"))]
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
    #[cfg(feature = "diag")]
    let (gpus, prep_ran) = if let keywait::Confirm::Proceed = confirmed {
        match tx_prep::take_gpus() {
            Some(g) => (g, true),
            None => (pci::enumerate(), false),
        }
    } else {
        (pci::enumerate(), false)
    };
    #[cfg(not(feature = "diag"))]
    let gpus = pci::enumerate();
    #[cfg(not(feature = "diag"))]
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
        // PR #209 follow-up: with the exit-fw arm the StartImage never runs,
        // so the pre-transaction USB read would buy nothing - skip it.
        if config::exit_fw() {
            None
        } else {
            chainload::preload_os_loader()
        }
    } else {
        None
    };

    #[cfg(feature = "diag")]
    if let keywait::Confirm::Proceed = confirmed {
        // \tx-dry-run.txt (issue #146): skip the transaction entirely. Since
        // preload went opt-in (#198) the residency control is the PAIR
        // \tx-dry-run.txt + \tx-preload.txt: the loader is resident, but this
        // path never reaches an asset read, so the cache stays EMPTY - "preload
        // residency, zero GPU writes, no detach, no reconnect", not "full
        // residency". (Measured: the dry-run log has no TX-ASSET-DIAG line.)
        if config::dry_run() {
            info!("TX-DRY-RUN: dry-run arm present (marker or tx.ini) - the GPU transaction is SKIPPED this boot (zero writes, no detach; preload residency control - assets are NOT read on this path, issue #146)");
        } else {
            tx_phase(&gpus);
        }
    } else {
        drop(confirmed);
    }
    #[cfg(not(feature = "diag"))]
    {
        drop(confirmed);
    }

    // The opt-in STOP-on-hard-failure halt (tx-stop-on-failure.txt) was
    // retired 2026-11-25: the unwind path is now unconditional
    // (emergency FLR + handoff + next card), so there is no post-phase
    // halt branch here anymore.

    // Phase: EBS-WATCH (soft-lock family cross-review 2026-11-22, issues
    // #242/#278/#279): with \tx-ebs-watch.txt on the stick, wrap
    // gBS->ExitBootServices + GetMemoryMap and arm the BeforeEBS/EBS-notify
    // fanout checkpoints before any OS loader is started, so the handoff
    // that has so far only been reconstructed statically gets OBSERVED at
    // runtime (enter / fanout stages / return, MapKey counters, committed
    // \ebs-watch.log + framebuffer rows + POST/UART channels). Inert probe
    // without the marker. \tx-ebs-selftest.txt turns the chain point into a
    // self-call of the wrapped EBS (validates the instrument without
    // booting Windows; parks on success).
    ebswatch::install_if_marker();
    if ebswatch::selftest_if_marker() {
        // Getting here means the self-test's own EBS call FAILED (success
        // parks forever inside selftest): boot services are still alive,
        // so the session continues into the normal chain instead of being
        // wasted.
        info!("CHAIN: ebs-selftest fell through (EBS did not succeed) - continuing into the normal chain");
    }

    // Phase: CHAIN. Read-only LoadImage/StartImage of the ESP bootmgfw.efi.
    // The verdict lines above are in \loader.log; no pause before handing
    // off (the log file is the evidence record, not the screen).
    match chainload::chain_load_bootmgfw(preloaded) {
        Ok(()) => {
            // PR #209 follow-up: the exit-to-fw arm also returns Ok -
            // distinguish it from a genuine bootmgfw return so the evidence
            // log does not record an anomaly on every marker run.
            if config::exit_fw() {
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
/// Key naming (why "102" names a whole generation): the Ampere/Ada keys
/// are the PROVENANCE chip of the pinned asset chain - booter_load_ga102.bin
/// is the GA102 driver archive's BINDATA, ad102 likewise - while the SCOPE
/// is the entire ID span routed to the key above. NVIDIA's own GSP ELF
/// signature sections are generation-scoped instead (fwimage_ga10x /
/// fwsignature_ad10x), so the makers bridge 10x->102 when staging the
/// stick; the two schemes name two different objects (byte origin vs
/// signature scope) and greppable provenance wins for the chain binaries.
/// Turing keeps generation keys (tu10x/tu11x) split per die archive.
/// Unverified IDs now run instead of being refused: per-ID evidence
/// tiers live in the README matrix, and a first-silicon report
/// upgrades the row (issue #8 process). Shared by tx_one_card and the
/// armed-prep asset step.
#[cfg(feature = "diag")]
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
#[cfg(feature = "diag")]
enum CardOutcome {
    /// Gated out before any GPU touch - no assets, no family mapping, PciIo
    /// open failure, or the (opt-in) identity gate refused it.
    NotAttempted,
    /// Reached `run_transaction`, whatever its outcome - upstream's existing
    /// "attempted" telemetry (issue #139).
    Attempted,
}

#[cfg(feature = "diag")]
fn tx_phase(gpus: &[pci::FoundGpu]) {
    // SLOT-REVIVE pre-pass (marker-gated, see revive.rs): revive dead
    // downstream slots through their root ports before target selection.
    // Inert without \tx-revive-dead-slot.txt (returns the input unchanged).
    let gpus = revive::revive_dead_slots(gpus);
    let targets = pci::select_tx_targets(&gpus);
    if targets.is_empty() {
        info!("TX: no transaction target (selection marker refused, or no supported GPU) - chain-loading unchanged Windows");
        return;
    }
    tx::list_root_entries();
    let mut attempted = 0usize;
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
}

/// One card's full transaction. EALREADY/EWarm/EBusy/EClaimUnreadable are
/// skip-class: nothing was torn down, the multi-GPU loop continues. A
/// generic (hard) failure runs the field-proven unwind (emergency FLR,
/// bridge-BME release, handoff, next card) - the opt-in STOP variant was
/// retired with \tx-stop-on-failure.txt on 2026-11-25.
///
/// Returns `CardOutcome::Attempted` when the transaction was actually
/// ATTEMPTED (the card was eligible, its asset set reached
/// `run_transaction`, and - if the opt-in identity gate is configured - it
/// was verified), `CardOutcome::NotAttempted` when the card was gated out
/// before any GPU touch (no assets / no family mapping / PciIo open
/// failure / identity gate refusal).
/// `tx_phase` turns `Attempted`/`NotAttempted` into the summary and the
/// `FINAL: 0 card(s) unlocked` verdict (issue #139).
#[cfg(feature = "diag")]
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

    // ---- \tx-rebar ladder (2026-11-24 bench line) --------------------------
    // Rung 0 \tx-rebar-read.txt: read-only probe (config first, then the XVE
    //   MMIO mirrors one log line each - isolates the first-run EFI hang).
    // ReBAR arms - \tx-rebar.txt family or the tx.ini `rebar` key
    //   (read | scan | <select>/<advertise>): claim first (the 11-22
    //   discipline: never touch the card with firmware drivers bound - the
    //   first run hung there), then the NV_30_DXE pair with the route from
    //   config::rebar_route():
    //     default:       XVE MMIO mirrors (PciIo extended config is
    //                    UNSUPPORTED on this firmware - rung-2 round proved
    //                    it fail-closed)
    //     ecam:          ECAM-direct config access (RSDP->XSDT->MCFG)
    //   Optional +sbr_gpu: disciplined Secondary Bus Reset inside the
    //   claimed window before the pair (display engine reset, straps
    //   re-latched, BAR/command restored from the claim baseline).
    // Every flow: NO FLR, no post-write reset - the pair must survive into
    // Windows' boot enumeration; recovery = power-off.
    // select = the ReBAR control write (advisory; Windows re-arbitrates),
    // advertise = the 0x610 field[10:7] value (the real ceiling; 9 = 32G,
    // 13 = 512G per the 2026-11-24 field map).
    let rebar_setting = config::rebar();

    if matches!(rebar_setting, Some(txini::RebarSetting::Read)) {
        info!("TX: rebar read arm - rung 0: read-only rebar-face probe (config side proven, MMIO side experimental; hang there = the isolation answer)");
        tx::probe_rebar_reads(&pci, &gpu);
        tx::handoff_out(&pci);
        return CardOutcome::Attempted;
    }

    if matches!(rebar_setting, Some(txini::RebarSetting::Scan)) {
        let sbr_rung = config::flag(config::FlagArm::SbrGpu);
        info!("TX: rebar scan arm - 0x610 field-encoding sweep + cap-mirror write probe (route {}, zero BAR-assignment writes; recovery = power-off)", if sbr_rung { "claim+SBR" } else { "claim" });
        match crate::tx_detach::detach_and_claim(g.handle) {
            Ok(claim) => {
                info!("REBAR-SCAN: claimed (GOP off, screen dark by design)");
                let mut sbr_ok = true;
                if sbr_rung {
                    let baseline = claim.baseline.clone();
                    sbr_ok = revive::sbr_gpu_in_transaction(&gpu, &baseline);
                    if sbr_ok {
                        info!("REBAR-SCAN: SBR complete, device verified alive");
                    } else {
                        info!("REBAR-SCAN: SBR left the device unverifiable - halting without chain-load (power off completely, remove the stick, boot normally)");
                    }
                }
                if sbr_ok {
                    tx::run_rebar_fieldscan(&gpu);
                    info!("SELECTOR RESULT: field sweep complete - see REBAR-SCAN lines above");
                } else {
                    logging::finish();
                    loop {
                        boot::stall(core::time::Duration::from_millis(500));
                    }
                }
                drop(claim);
                tx::handoff_out(&pci);
            }
            Err(e) => {
                info!("REBAR-SCAN: claim failed ({e:?}) - refusing to touch a firmware-bound card; handing off untouched");
                tx::handoff_out(&pci);
            }
        }
        return CardOutcome::Attempted;
    }

    // rebar_then_tx (2026-11-24): run the rebar pair AND the full unlock
    // transaction in one boot. The pair lands inside the claim+SBR window
    // (same as the rebar-only flow), the claim is released, and control
    // FALLS THROUGH the rebar if-block to the transaction below - which
    // re-claims fresh. Turing's unlock chain skips FLR (FEAT PLM does not
    // survive it), so nothing in the transaction resets the pair away; the
    // SS selectors + the 32G window land together at the handoff.
    // Declared here so the flag survives past the rebar if-block's match
    // scope (the fall-through must cross it).
    let rebar_then_tx = config::flag(config::FlagArm::RebarThenTx);
    if let Some(txini::RebarSetting::Pair {
        select: rebar_idx,
        advertise: rebar_adv,
    }) = rebar_setting
    {
        let sbr_rung = config::flag(config::FlagArm::SbrGpu);
        let ecam_route = matches!(config::rebar_route(), txini::RebarRoute::Ecam);
        info!(
            "TX: rebar pair arm (target size_idx {}, advertise field {}, route {}, SBR {}) - claiming first (11-22 discipline), NO FLR, NO post-write reset; recovery = power-off (0 = that side is NOT written)",
            rebar_idx,
            rebar_adv,
            if ecam_route { "ECAM-direct" } else { "XVE-MMIO mirror" },
            if sbr_rung { "yes" } else { "no" }
        );
        match crate::tx_detach::detach_and_claim(g.handle) {
            Ok(claim) => {
                info!("REBAR: claimed (GOP off, screen dark by design from here to the Windows driver)");
                let mut sbr_ok = true;
                if sbr_rung {
                    let baseline = claim.baseline.clone();
                    sbr_ok = revive::sbr_gpu_in_transaction(&gpu, &baseline);
                    if sbr_ok {
                        info!("REBAR: SBR complete, device verified alive, config restored from claim baseline");
                    } else {
                        info!("REBAR: SBR left the device unverifiable - halting without chain-load (power off completely, remove the stick, boot normally)");
                    }
                }
                if sbr_ok {
                    // 0 on a side = that write is skipped; the ECAM route
                    // never carried an advertise field (config 0xBC0 only),
                    // so target=0 on ECAM degenerates to a claim+SBR-only
                    // round trip (still a valid straps re-latch arm).
                    let pair = if ecam_route {
                        tx::run_rebar_ecam((g.seg, g.bus, g.dev, g.fun), rebar_idx)
                    } else {
                        tx::run_rebar_mmio(&gpu, rebar_idx, rebar_adv)
                    };
                    match pair {
                        Ok(()) => {
                            let gop_reconnect = config::flag(config::FlagArm::RebarGop);
                            info!(
                                "SELECTOR RESULT: rebar pair written and readback-verified - {} before chaining",
                                if gop_reconnect { "rebuilding the console GOP (rebar-gop ablation arm)" } else { "dark handoff" }
                            );
                            if gop_reconnect {
                                if let Err(e) =
                                    crate::gop_native::recover(g.handle, gpu.pci, claim.initial_attrs)
                                {
                                    info!("REBAR: native-GOP recovery failed ({e}) - chaining dark (Windows driver will re-enable the display)");
                                }
                            }
                        }
                        Err(e) => {
                            info!("REBAR: pair failed ({e:?}) - handing off unchanged; no reset on this path, power-off clears any partial write");
                        }
                    }
                } else {
                    logging::finish();
                    loop {
                        boot::stall(core::time::Duration::from_millis(500));
                    }
                }
                drop(claim);
                if rebar_then_tx {
                    info!("REBAR: rebar_then_tx arm - claim released, FALLING THROUGH to the full unlock transaction (fresh claim inside; Turing skips FLR so the pair survives the chain)");
                    // Fall through: no handoff_out, and NOT the match-tail
                    // return below - control must leave this whole if-block
                    // to reach the transaction code. The flag escapes the
                    // match scope, so it is re-derived at the function level.
                } else {
                    tx::handoff_out(&pci);
                    return CardOutcome::Attempted;
                }
            }
            Err(e) => {
                info!("REBAR: claim failed ({e:?}) - refusing to write against a firmware-bound card (the 11-22 soft-lock class); handing off untouched");
                tx::handoff_out(&pci);
            }
        }
        if !rebar_then_tx {
            return CardOutcome::Attempted;
        }
        // rebar_then_tx: control reaches here having released the claim -
        // continue below into the asset prechecks and run_transaction.
    }

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

    // \tx-agitate-only.txt (soft-lock family cross-review 2026-11-22,
    // issues #242/#278/#279): the full-surgery ZERO-GPU-write negative
    // control - detach + EXCLUSIVE claim + COMMAND writes + the DMA
    // allocations at the transaction's sizes + bridge BME + native-GOP
    // rebuild + reconnect all run; NOTHING touches BAR0 (no reset, no
    // FWSEC, no booter, no selectors, no canary, no FLR - and no emergency
    // FLR on the error path either). This is the control the field matrix
    // was missing: every earlier zero-write arm skipped the whole surgery,
    // so "zero-write boots / transaction freezes" could not separate
    // "the unlocked GPU's live state poisons the handoff" from "our UEFI
    // surgery poisons it". A freeze under this marker indicts the surgery;
    // a clean boot leaves the live GPU state as the remaining suspect.
    if config::fwsec_only() {
        info!("TX: fwsec-only arm present (marker or tx.ini) - the transaction stops after the FWSEC stage (no booter, no selectors, no FLR; issue #286 residue bracket)");
    }
    if config::agitate_only() {
        info!("TX: agitate-only arm present (marker or tx.ini) - full-surgery zero-write control instead of the transaction (see run_agitate_only's contract)");
        let mut agitate_bridge = None;
        let outcome = tx::run_agitate_only(&gpu, &assets, g.handle, &mut agitate_bridge);
        match outcome {
            Ok(()) => {
                info!("SELECTOR RESULT: agitate-only control complete (zero GPU writes by contract)");
            }
            Err(e) => {
                info!("TX-AGITATE: control outcome: {e:?} (no emergency FLR - zero-write contract)");
            }
        }
        tx::handoff_out(&pci);
        return CardOutcome::Attempted;
    }

    #[cfg(not(feature = "tx"))]
    {
        // DIAG build: the payload/transaction machinery is not compiled in.
        // The agitate control above was the only GPU-touching branch this
        // binary carries, and it writes nothing to BAR0 by contract.
        info!("DIAG BUILD: transaction code is not compiled in - no unlock, no replay; the agitate control above was the only action this binary can take");
        tx::handoff_out(&pci);
        return CardOutcome::Attempted;
    }

    #[cfg(feature = "tx")]
    {
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
        // Generic/hard failure: the field-proven path, now unconditional
        // (the opt-in \tx-stop-on-failure.txt STOP variant was retired
        // 2026-11-25 - zero field uptake across its lifetime): run
        // emergency_flr on this card, hand off, continue to the next card.
        Err(e) => {
            info!("TX: transaction failed: {e:?} - NOT retrying this session");
            // Field report (issue #74): on a TB4/USB4 eGPU the unwind FLR can
            // leave the function off the bus afterwards - Windows then shows
            // CM_PROB_PHANTOM, fans stop, and the only recovery measured on
            // that seat is an AC cycle of the enclosure. Say so, or the first
            // person to see it concludes the card is dead.
            info!("TX: recovery note: if this card sits in an eGPU enclosure and disappears from the bus after this failure, power-cycle the enclosure (PSU off ~30 s); an internal card needs a normal power cycle");
            // Failure unwind: FLR wipes our half-states (engine resets,
            // BCR, DMA queues) so the Windows driver gets a cold-boot card.
            // Bridge release only AFTER the FLR - post-reset nothing can be
            // in flight through the bridges, so clearing is exactly as safe
            // as on the success tail; clearing earlier would be an
            // uncontrolled write under possibly-live DMA (booter 0x89
            // family), and never clearing would carry our bus-master bits
            // into Windows (dirty-BME handoff shape, #3/#9). The old
            // tx_policy module pinned this ordering; it retired with the
            // STOP variant (single-decision enums with one value left).
            tx::emergency_flr(g.handle);
            if let Some(bm) = bridge_master.as_mut() {
                bm.release();
                info!("TX: bridge BME restored after the failure FLR");
            }
            tx::handoff_out(&pci);
            CardOutcome::Attempted
        }
    }
    }
}
