// Compile the production classifier, rather than duplicate its behavior.
#[path = "../../loader/src/claim_plm.rs"]
mod claim_plm;

use claim_plm::{classify, ClaimPlm};

#[test]
fn open_plm_is_detected_despite_all_ones_value() {
    assert_eq!(classify(Some(0xffff_ffff)), ClaimPlm::Open);
}

#[test]
fn failed_transport_is_not_an_open_or_locked_register() {
    assert_eq!(classify(None), ClaimPlm::ReadFailed);
}

#[test]
fn badf_responses_keep_the_existing_unreadable_policy() {
    for value in [0xbadf_0000, 0xbadf_5040, 0xbadf_1100, 0xbadf_ffff] {
        assert_eq!(classify(Some(value)), ClaimPlm::Inaccessible);
    }
}

#[test]
fn closed_and_other_non_poison_values_are_not_open() {
    for value in [0xffff_ff8f, 0xffff_ffef, 0, 0xbade_ffff, 0xbae0_0000] {
        assert_eq!(classify(Some(value)), ClaimPlm::NotOpen);
    }
}

// Catnip's dead-window case (PR #65 review): a fully dead window returns
// all-ones for every address, PLM included, and must NOT classify as an
// open PLM. The guarantee lives in claim_guards' ORDER, not the classifier:
// the chip preflight (PMC_BOOT_0 via read32_raw) refuses an all-ones chip id
// before the PLM is ever classified. Documented here so the invariant has a
// test-adjacent anchor; the ordering itself is loader-side.
#[test]
fn dead_window_ordering_is_documented() {
    // classifier-level truth: all-ones PLM alone reads as Open...
    assert_eq!(classify(Some(0xffff_ffff)), ClaimPlm::Open);
    // ...which is only sound because claim_guards runs the chip preflight
    // FIRST (chip == !0 -> ENodev) before classifying the PLM.
}
