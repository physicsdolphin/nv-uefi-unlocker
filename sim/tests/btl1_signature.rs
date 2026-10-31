// Exercise the actual pure helper compiled by the UEFI loader with host tests.
#[path = "../../loader/src/btl1_signature.rs"]
mod btl1_signature;

#[test]
fn ada_ampere_two_signatures_use_384_byte_slots() {
    assert_eq!(btl1_signature::per_signature_size(2, 768, 768), Some(384));
}

#[test]
fn turing_single_signature_retains_16_byte_slot() {
    assert_eq!(btl1_signature::per_signature_size(1, 16, 16), Some(16));
}

#[test]
fn inconsistent_duplicate_totals_are_rejected() {
    assert_eq!(btl1_signature::per_signature_size(2, 384, 768), None);
}

#[test]
fn zero_and_non_divisible_signature_tables_are_rejected() {
    assert_eq!(btl1_signature::per_signature_size(0, 768, 768), None);
    assert_eq!(btl1_signature::per_signature_size(2, 0, 0), None);
    assert_eq!(btl1_signature::per_signature_size(2, 767, 767), None);
}
