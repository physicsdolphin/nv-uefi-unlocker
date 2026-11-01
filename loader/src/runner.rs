//! The shared post-CONFIRM phase sequence, moved verbatim out of the app
//! main so the firmware-resident DXE driver replays exactly the same
//! ENUMERATE / GOP-PROBE / SAMPLE / TX phases (docs/dxe-driver-ffs-feasibility.md).

use log::info;

use crate::keywait;
use crate::{gop, pci, sample};
#[cfg(feature = "tx")]
use crate::{tx, tx_prep};

/// ENUMERATE / GOP-PROBE / SAMPLE / TX. The armed prep (TX builds) may have
/// already run enumeration, the GOP probe and the sample inside the
/// countdown window - `tx_prep::take_gpus` reports that.
pub fn phases(confirmed: keywait::Confirm) {
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
    let (gpus, prep_ran) = (pci::enumerate(), false);

    // Phase: GOP-PROBE (read-only; which console driver owns which GPU).
    if !prep_ran {
        gop::enumerate_gop();
    }

    // Phase: SAMPLE (M1: read-only cold-state register sweep, first card).
    if !prep_ran {
        if let Some(gpu) = gpus.first() {
            sample::run(gpu);
        } else {
            info!("SAMPLE: no NVIDIA GPU present, skipped (negative control path)");
        }
    }

    // Phase: TX (feature-gated transaction; only in explicit tx builds).
    // NOTE: inside tx_phase each card's console drivers are detached
    // (TOTORONG craft) - the screen goes dark from that card's transaction
    // until Windows boots. The log file keeps recording and is the
    // evidence record. Multi-GPU: every supported card runs in order.
    #[cfg(feature = "tx")]
    if let keywait::Confirm::Proceed = confirmed {
        tx_phase(&gpus);
        // The prefetch cache is discardable working memory (audit M4): the
        // application exits anyway, the resident driver must free it.
        tx::asset_cache_clear();
    }
    #[cfg(not(feature = "tx"))]
    let _ = confirmed;
}

/// Die-family-pure asset mapping by PCI device ID (constitution 5: no
/// cross-family payloads — a GA102 payload on AD102 HALTs and opens
/// nothing). Names on the boot volume: <asset>_<family>.bin.
///
/// 2026-10-28: the whitelist is OPEN for the full NVIDIA driver list
/// of the three transaction families (pinned open-gpu-kernel-modules
/// README "Compatible GPUs" table, 217 IDs, cross-checked against
/// upstream's supported_ids[]). The blocks are die-family-pure:
///   0x1e02..0x1fff, 0x2182..0x21d1 = Turing
///     TU102/104/106/116/117 (20/16-series, TITAN/Quadro RTX,
///     CMP 30/40/50HX; the 21xx span covers every TU116 SKU incl.
///     1650 SUPER 2187 and the odd 21c2 variant);
///   0x2200..0x22ff, 0x2400..0x25ff = Ampere GA10x (GA102 3090/3080/
///     CMP 90HX/A6000-class, GA103 2414, GA104 3070s/CMP 70HX/A4500,
///     GA106/107 3060/3050 lines + mobile; 0x23xx = Hopper, absent
///     on purpose);
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
pub(crate) fn asset_family(dev: u16) -> Option<&'static str> {
    match dev {
        0x2600..=0x28ff => Some("ad102"),
        0x2200..=0x22ff | 0x2400..=0x25ff => Some("ga102"),
        0x1e02..=0x1fff | 0x2182..=0x21d1 => Some("tu10x"),
        _ => None,
    }
}

#[cfg(feature = "tx")]
fn tx_phase(gpus: &[pci::FoundGpu]) {
    let targets = pci::select_tx_targets(gpus);
    if targets.is_empty() {
        info!("TX: no transaction target (selection marker refused, or no supported GPU) - chain-loading unchanged Windows");
        return;
    }
    tx::log_overrides();
    tx::list_root_entries();
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
        tx_one_card(g);
    }
    info!("TX: multi-GPU pass complete ({} target(s) processed)", targets.len());
}

/// One card's full transaction. Every outcome continues the multi-GPU
/// loop: EALREADY/EWarm are skip-class, a hard failure gets its own
/// emergency FLR on THIS card only - the next card still runs.
#[cfg(feature = "tx")]
fn tx_one_card(g: &pci::FoundGpu) {
    use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams};
    let proto = unsafe {
        boot::open_protocol::<pci::PciIo>(
            OpenProtocolParams { handle: g.handle, agent: boot::image_handle(), controller: None },
            OpenProtocolAttributes::GetProtocol,
        )
    };
    let Ok(pci) = proto else {
        info!("TX: open PciIo failed");
        return;
    };
    let gpu = tx::TxGpu { pci: &pci, bar: 0 };

    // Per-family asset selection by PCI device ID - see asset_family.
    let dev = g.device_id;
    let Some(family) = asset_family(dev) else {
        info!("TX: device 10de:{dev:04x} has no asset family mapping - skipping");
        return;
    };
    info!("TX: device 10de:{dev:04x} -> asset family '{family}'");

    // Live media re-probe (audit N7): the read-through cache survives a
    // media pull, so "pulled media aborts before GPU interaction" is
    // re-established by re-opening every required file right here.
    if !tx::live_asset_presence(family) {
        info!("TX: media re-probe failed after prep - aborting before any GPU interaction");
        return;
    }

    // Assets: `Ok(None)` = the volume does not carry it (skip this card);
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
                    return;
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
    // Which v67 file the bytes came from (audit M1 digest check must verify
    // the file that was actually used, not the family default).
    let mut v67_name = alloc::format!("\\v67-{family}-four-plm.bin");
    let v67 = match tx::read_asset("\\v67-direct-ss.bin") {
        Ok(Some(v)) => {
            v67_name = "\\v67-direct-ss.bin".into();
            Some(v)
        }
        Ok(None) => must_asset!("v67 payload", tx::read_asset_fmt("\\v67-{}-four-plm.bin", family)),
        Err(()) => {
            info!("TX: \\v67-direct-ss.bin is present but unreadable (damaged stick or pulled media) - aborting before any GPU interaction");
            return;
        }
    };
    let bl = if family == "tu10x" {
        must_asset!("sec2_bl", tx::read_asset("\\sec2_bl_tu10x.bin"))
    } else {
        None
    };
    let (Some(booter), Some(gsp_boot), Some(gsp_image), Some(stock_sig)) =
        (booter, gsp_boot, gsp_image, stock_sig) else {
        info!("TX: assets missing for family '{family}' (booter_load_*/gsp_boot_*/fwimage_*/fwsignature_*) - skipping");
        return;
    };
    if v67.is_none() {
        info!("TX: NO V67 file - chain-replay mode (no unlock writes)");
    } else {
        info!("TX: V67 present - unlock mode (family-specific write chain)");
    }
    if family == "tu10x" && bl.is_none() {
        info!("TX: tu10x missing \\sec2_bl_tu10x.bin (WITH_LOADER FWSEC needs the generic falcon BL) - skipping");
        return;
    }
    // Turing track enabled 2026-10-27 (user M3 go): the chain is the
    // cmp50hx-proven TU106 recipe - FWSEC BOOT_WITH_LOADER via the generic
    // falcon BL, BOOT_DIRECT booter fire, replay verdict mbox0==0 + WPR2.
    // First fire = chain-replay (no V67 at root) unless the payload file is
    // staged deliberately.
    let assets = tx::TxAssets { family, booter, boot_desc, gsp_boot, gsp_image, stock_sig, v67, bl };
    // Asset digest verification, driver form only (audit M1): the armed
    // volume's \dxe-manifest.txt must match every byte that is about to be
    // DMA'd. The application form keeps stick-trust semantics.
    if crate::volumes::armed().is_some() {
        let mut entries: alloc::vec::Vec<(alloc::string::String, &[u8])> = alloc::vec::Vec::new();
        for (name, bytes) in [
            (alloc::format!("\\booter_load_{family}.bin"), assets.booter.as_slice()),
            (alloc::format!("\\gsp_boot_desc_{family}.bin"), assets.boot_desc.as_deref().unwrap_or(&[])),
            (alloc::format!("\\gsp_boot_{family}.bin"), assets.gsp_boot.as_slice()),
            (alloc::format!("\\fwimage_{family}.bin"), assets.gsp_image.as_slice()),
            (alloc::format!("\\fwsignature_{family}.bin"), assets.stock_sig.as_slice()),
            (v67_name.clone(), assets.v67.as_deref().unwrap_or(&[])),
        ] {
            entries.push((name, bytes));
        }
        if assets.family == "tu10x" {
            if let Some(bl) = assets.bl.as_deref() {
                entries.push(("\\sec2_bl_tu10x.bin".into(), bl));
            }
        }
        if let Err(e) =
            tx::verify_asset_digests(entries.iter().map(|(p, b)| (p.as_str(), *b)))
        {
            info!("TX: {e} - aborting before any GPU interaction");
            return;
        }
    }
    // Size validation before ANY GPU interaction (issue #4.3): a damaged
    // volume must be refused here - the DMA copies use fixed windows.
    if let Err(e) = tx::validate_asset_sizes(&assets) {
        info!("TX: asset validation failed ({e:?}) - nothing was touched, chain-loading Windows");
        return;
    }
    match tx::run_transaction(&gpu, &assets, g.handle) {
        Ok(()) => {
            info!("SELECTOR RESULT: transaction complete");
            tx::handoff_out(&pci);
        }
        // Already-open PLM = already unlocked or not cold: skip (success
        // class, gpu-unlock-ada da96e3f), definitely no FLR - it would wipe
        // a live unlock.
        Err(tx::TxError::EAlready) => {
            info!("TX: already unlocked / not cold (EALREADY) - skipped, nothing to unwind");
            tx::handoff_out(&pci);
        }
        // The claim read cannot establish whether an unlock is already
        // present. Detach/claim and an optional pre-FLR may have happened;
        // do not add a failure reset merely because this read failed.
        Err(tx::TxError::EClaimUnreadable) => {
            info!("TX: claim PLM transport read failed - transaction skipped, no additional failure reset");
            tx::handoff_out(&pci);
        }
        // Warm entry with the require-D0 switch: nothing was written, the
        // card was not even reset - the clean "come back after a Restart"
        // outcome. No emergency FLR (FLR from a non-D0 state is exactly the
        // undefined case).
        Err(tx::TxError::EWarm) => {
            info!("TX: aborted on warm/resume entry before any GPU write - chain-loading unchanged Windows (use a Restart or cold boot)");
            tx::handoff_out(&pci);
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
        }
        Err(e) => {
            info!("TX: transaction failed: {e:?} - NOT retrying this session");
            // Failure unwind: FLR wipes our half-states (engine resets, BCR,
            // DMA queues) so the OS driver gets a cold-boot card.
            tx::emergency_flr(g.handle);
            tx::handoff_out(&pci);
        }
    }
}
