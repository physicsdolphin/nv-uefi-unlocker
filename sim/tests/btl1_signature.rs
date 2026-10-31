// Exercise the actual pure helper compiled by the UEFI loader with host tests.
#[path = "../../loader/src/btl1_signature.rs"]
mod btl1_signature;

use btl1_signature::SlotError;

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

// --- slot selection and the byte-exact copy, i.e. what booter_patch_stock runs ---

/// Two 384-byte signatures with distinguishable fill (0x11 = slot 0, 0x22 = slot 1).
fn two_slot_table() -> Vec<u8> {
    let mut table = vec![0x11u8; 384];
    table.extend_from_slice(&[0x22u8; 384]);
    table
}

#[test]
fn slot_ranges_match_the_reference_fuse_indexing() {
    // GA10x/Ada: two slots, the fuse version counts down from the newest one.
    assert_eq!(btl1_signature::slot_range(2, 384, 1, 768), Ok((0, 384)));
    assert_eq!(btl1_signature::slot_range(2, 384, 0, 768), Ok((384, 384)));
    // Turing BOOT_DIRECT: one 16-byte signature, index pinned to 0.
    assert_eq!(btl1_signature::slot_range(1, 16, 0, 16), Ok((0, 16)));
    // Pre-fix behaviour, expressed with the values those bundles carry: the
    // consumer passed the TABLE total (768) as the per-signature size. Index 1
    // then needs (1+1)*768 bytes of a 768-byte table and is rejected, while
    // index 0 slices 768 bytes where the slot holds 384.
    assert_eq!(
        btl1_signature::slot_range(2, 768, 0, 768),
        Err(SlotError::TableInconsistent)
    );
    assert_eq!(btl1_signature::slot_range(2, 768, 1, 768), Ok((0, 768)));
}

#[test]
fn fuse_ver_one_copies_only_the_first_signature_into_the_slot() {
    let signatures = two_slot_table();
    let mut image = vec![0xAAu8; 512];
    btl1_signature::install_stock_signature(&mut image, 64, 2, 384, 1, &signatures)
        .expect("fuse_ver 1 selects signature 0");
    assert!(image[64..448].iter().all(|&b| b == 0x11), "slot must hold sig[0]");
    assert!(image[448..].iter().all(|&b| b == 0xAA), "nothing may be written past the slot");
    assert!(image[..64].iter().all(|&b| b == 0xAA), "nothing may be written before the slot");
}

#[test]
fn fuse_ver_zero_copies_the_second_signature_instead_of_failing() {
    let signatures = two_slot_table();
    let mut image = vec![0xAAu8; 512];
    btl1_signature::install_stock_signature(&mut image, 64, 2, 384, 0, &signatures)
        .expect("fuse_ver 0 selects signature 1");
    assert!(image[64..448].iter().all(|&b| b == 0x22), "slot must hold sig[1]");
    assert!(image[448..].iter().all(|&b| b == 0xAA), "the copy must not spill past the slot");
}

#[test]
fn a_slot_that_does_not_fit_the_image_is_rejected() {
    let signatures = two_slot_table();
    let mut image = vec![0xAAu8; 400]; // patch_loc 64 + 384 = 448 > 400
    assert_eq!(
        btl1_signature::install_stock_signature(&mut image, 64, 2, 384, 1, &signatures),
        Err(SlotError::TableInconsistent)
    );
    assert!(image.iter().all(|&b| b == 0xAA), "a rejected patch must leave the image untouched");
}

#[test]
fn out_of_range_fuse_versions_and_empty_metadata_are_rejected() {
    assert_eq!(
        btl1_signature::slot_range(2, 384, 2, 768),
        Err(SlotError::IndexOutOfRange)
    );
    assert_eq!(
        btl1_signature::slot_range(0, 384, 0, 768),
        Err(SlotError::IndexOutOfRange)
    );
    assert_eq!(
        btl1_signature::slot_range(2, 0, 0, 768),
        Err(SlotError::TableInconsistent)
    );
    assert_eq!(
        btl1_signature::slot_range(2, 384, 0, 767),
        Err(SlotError::TableInconsistent)
    );
}
