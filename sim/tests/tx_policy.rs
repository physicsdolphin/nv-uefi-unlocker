//! Host-side tests of the TX failure-handling policy
//! (`loader/src/tx_policy.rs`). The knobs it models live in
//! `main.rs::tx_one_card` orchestration, which sim cannot execute - this
//! file pins the CONTRACT (marker semantics + release ordering) so a
//! drift on either the EFI side or the policy side trips a test here.
//!
//! Scenarios map to the review's fault-injection asks:
//! - failure WITHOUT the marker -> emergency FLR + continue (the
//!   field-proven default, unchanged);
//! - failure WITH \tx-stop-on-failure.txt -> no FLR, no handoff, no chain,
//!   and correspondingly NO bridge-BME release (never under possibly-live
//!   DMA);
//! - default-failure bridge release happens only AFTER the FLR;
//! - the success tail always releases (modeled inside run_transaction; the
//!   policy only covers the failure paths).
#[path = "../../loader/src/tx_policy.rs"]
mod tx_policy;

use tx_policy::*;

#[test]
fn marker_absent_keeps_the_field_proven_default() {
    // The upstream survey pinned "failure -> emergency FLR + next card" as a
    // regression guard; an absent marker must not flip it.
    for marker in [false] {
        assert!(matches!(
            hard_failure_action(marker),
            HardFailureAction::FlrAndContinue
        ));
    }
}

#[test]
fn marker_present_stops_instead_of_flr() {
    assert!(matches!(
        hard_failure_action(true),
        HardFailureAction::Stop
    ));
}

#[test]
fn default_failure_releases_bridges_only_after_the_flr() {
    let action = hard_failure_action(false);
    assert!(matches!(
        bridge_release_on_failure(action),
        BridgeReleaseOnFailure::AfterFlr
    ));
}

#[test]
fn stop_path_never_releases_bridges() {
    // No FLR on the STOP path means possibly-live DMA - clearing
    // bus-master there would be the uncontrolled-write direction, and the
    // machine halts for a power cycle anyway.
    let action = hard_failure_action(true);
    assert!(matches!(
        bridge_release_on_failure(action),
        BridgeReleaseOnFailure::Never
    ));
}
