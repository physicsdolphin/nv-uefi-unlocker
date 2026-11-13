//! Pure failure-handling policy for the TX phase, shared by the EFI side
//! (`main.rs::tx_one_card`) and the host-side tests
//! (`sim/tests/tx_policy.rs` - same `#[path]` include trick as
//! `claim_plm.rs`; this file must stay free of UEFI dependencies).
//!
//! The knobs live in `main.rs` orchestration, which sim cannot execute - so
//! the decision surface is pinned HERE and exercised in main.rs through
//! these functions. If the call sites drift from this contract, change both
//! together (the tests document each site).
//!
//! Skip-class errors (EALREADY / EWarm / EBusy / EClaimUnreadable) never
//! reach the generic-failure policy at all - each arm in main.rs documents
//! why that class takes no FLR. The policy below covers everything else.

/// What a GENERIC (hard) transaction failure does - an `Err(e)` outside the
/// skip-class set, past the asset/identity gates.
pub enum HardFailureAction {
    /// Field-proven default: `emergency_flr` on THIS card, then
    /// `handoff_out`, then the multi-GPU loop continues to the next card.
    /// The upstream survey pinned this as a regression guard - the marker's
    /// absence must never silently change it.
    FlrAndContinue,
    /// Opt-in `\tx-stop-on-failure.txt`: no FLR (a reset issued while the
    /// firmware may still be mid-operation is itself an uncontrolled write
    /// to a card whose state is no longer trusted), no handoff, no next
    /// card, and `main` must not chain-load Windows - halt for a full
    /// power cycle.
    Stop,
}

/// Decision for `main.rs::tx_one_card`'s generic-failure arm.
pub fn hard_failure_action(stop_on_failure_marker_present: bool) -> HardFailureAction {
    if stop_on_failure_marker_present {
        HardFailureAction::Stop
    } else {
        HardFailureAction::FlrAndContinue
    }
}

/// Whether - and where - the BridgeMaster guard is released on the generic
/// failure path. Ordering is the whole point: releasing at the failure
/// site itself clears bus-master under possibly-live DMA (booter 0x89
/// family); never releasing carries our bus-master bits across the handoff
/// into Windows (dirty-BME shape, known-pit #3/#9). The success tail always
/// releases (inside `run_transaction`, same settle point as the DMA
/// releases) and is not modeled here.
pub enum BridgeReleaseOnFailure {
    /// Yes - but only AFTER `emergency_flr` completes: post-reset nothing
    /// can be in flight through the bridges, so clearing is exactly as safe
    /// as on the success tail.
    AfterFlr,
    /// No - the opt-in STOP path never releases: no FLR means possibly-live
    /// DMA, and the machine is about to halt for a power cycle, so nothing
    /// downstream observes the bridges either way.
    Never,
}

/// Decision for the failure path's bridge-BME release, derived from the
/// failure action (so the STOP path and the release ordering cannot drift
/// apart).
pub fn bridge_release_on_failure(action: HardFailureAction) -> BridgeReleaseOnFailure {
    match action {
        HardFailureAction::FlrAndContinue => BridgeReleaseOnFailure::AfterFlr,
        HardFailureAction::Stop => BridgeReleaseOnFailure::Never,
    }
}
