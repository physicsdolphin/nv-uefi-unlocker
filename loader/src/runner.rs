//! The shared post-CONFIRM phase sequence, shared verbatim by the USB
//! application (`main.rs`) and the firmware-resident DXE driver
//! (`dxe_main.rs` via `dxe`): ENUMERATE / GOP-PROBE / SAMPLE / preload /
//! TX (docs/dxe-driver-ffs-feasibility.md). The content tracks main's
//! application flow one-to-one; only the ownership differs - the app
//! continues into chain-load, the driver returns to the firmware.

use log::info;
use uefi::cstr16;

use crate::chainload;
use crate::gop;
use crate::keywait;
use crate::pci;
use crate::sample;
use crate::ultrafast;
#[cfg(feature = "tx")]
use crate::identity;
#[cfg(feature = "tx")]
use crate::tx;
#[cfg(feature = "tx")]
use crate::tx_policy;
#[cfg(feature = "tx")]
use crate::tx_prep;

/// What `phases` hands back to the form-specific tail: the app decides
/// between STOP and chain-load, the driver logs and returns to firmware.
pub struct PhaseOutcome {
    /// Opt-in `\tx-stop-on-failure.txt` hard failure: the session must not
    /// act on the card again - the app must not chain-load either.
    pub hard_stop: bool,
    /// P2-7 preloaded OS loader image (`chainload::preload_os_loader`),
    /// consumed by the app's chain step. The driver form ignores it.
    pub preloaded: Option<chainload::PreloadedOsLoader>,
}

/// ENUMERATE / GOP-PROBE / SAMPLE / preload / TX. The armed prep (TX
/// builds) may have already run enumeration, the GOP probe and the sample
/// inside the countdown window - `tx_prep::take_gpus` reports that.
pub fn phases(confirmed: keywait::Confirm) -> PhaseOutcome {
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

    // Phase: TX (feature-gated transaction; only in explicit tx builds).
    // NOTE: inside tx_phase each card's console drivers are detached
    // (TOTORONG craft) - the screen goes dark from that card's transaction
    // until Windows boots. The log file keeps recording and is the
    // evidence record. Multi-GPU: every supported card runs in order.
    // \tx-dry-run.txt (issue #146): skip the transaction entirely. Since
    // preload went opt-in (#198) the residency control is the PAIR
    // \tx-dry-run.txt + \tx-preload.txt: the loader is resident, but this
    // path never reaches an asset read, so the cache stays EMPTY - "preload
    // residency, zero GPU writes, no detach, no reconnect", not "full
    // residency". (Measured: the dry-run log has no TX-ASSET-DIAG line.)
    #[cfg(feature = "tx")]
    let hard_stop = if let keywait::Confirm::Proceed = confirmed {
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

    PhaseOutcome { hard_stop, preloaded }
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
pub fn asset_family(dev: u16) -> Option<&'static str> {
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
    /// failure occurred: the whole multi-GPU run stops here, and the caller
    /// (`phases`) must not chain-load Windows either.
    HardStop,
}

/// Returns `true` iff a `CardOutcome::HardStop` occurred, in which case the
/// caller (`phases`) must not chain-load Windows.
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
    // hygiene, and CHAIN-MEMMAP logs the map after the free. The DXE driver
    // form frees at the end of its phase tail (`dxe`) instead - it never
    // chains.
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
    // Asset digest verification, driver form only (audit M1): the armed
    // volume's \dxe-manifest.txt must match every byte that is about to be
    // DMA'd - including the scrubber, which the transaction would otherwise
    // load lazily AFTER detach/claim. The application form keeps
    // stick-trust semantics (the operator holds the stick they prepared;
    // the maker verified at build time).
    if crate::volumes::armed().is_some() {
        // Cache-backed re-reads: the bytes verified here are the bytes the
        // transaction will use (single read-through cache, no second copy).
        let direct_v67 = tx::read_asset("\\v67-direct-ss.bin");
        let family_v67 = tx::read_asset(&alloc::format!("\\v67-{family}-four-plm.bin"));
        let scrubber = if tx::family_is_turing(family) {
            None
        } else {
            Some(tx::read_asset("\\scrubber_ad10x.bin"))
        };
        let mut entries: alloc::vec::Vec<(alloc::string::String, Option<alloc::vec::Vec<u8>>)> =
            alloc::vec::Vec::new();
        entries.push((alloc::format!("\\booter_load_{family}.bin"), Some(assets.booter.clone())));
        entries.push((alloc::format!("\\gsp_boot_desc_{family}.bin"), assets.boot_desc.clone()));
        entries.push((alloc::format!("\\gsp_boot_{family}.bin"), Some(assets.gsp_boot.clone())));
        entries.push((alloc::format!("\\fwimage_{family}.bin"), Some(assets.gsp_image.clone())));
        entries.push((alloc::format!("\\fwsignature_{family}.bin"), Some(assets.stock_sig.clone())));
        for (what, read) in [
            ("\\v67-direct-ss.bin", &direct_v67),
            (&*alloc::format!("\\v67-{family}-four-plm.bin"), &family_v67),
        ] {
            match read {
                Ok(v) => entries.push((alloc::string::String::from(what), v.clone())),
                Err(()) => {
                    info!("TX: {what} is present but unreadable (damaged stick or pulled media) - aborting before any GPU interaction");
                    return CardOutcome::NotAttempted;
                }
            }
        }
        if let Some(bl) = assets.bl.as_ref() {
            entries.push((alloc::format!("\\sec2_bl_{family}.bin"), Some(bl.clone())));
        }
        if let Some(Ok(v)) = &scrubber {
            entries.push((alloc::string::String::from("\\scrubber_ad10x.bin"), v.clone()));
        }
        if let Some(Err(())) = &scrubber {
            info!("TX: \\scrubber_ad10x.bin is present but unreadable (damaged stick or pulled media) - aborting before any GPU interaction");
            return CardOutcome::NotAttempted;
        }
        if let Err(e) = tx::verify_asset_digests(
            entries.iter().map(|(p, b)| (p.as_str(), b.as_deref())),
        ) {
            info!("TX: {e} - aborting before any GPU interaction");
            return CardOutcome::NotAttempted;
        }
    }
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
