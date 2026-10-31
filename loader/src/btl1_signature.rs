//! BTL1 signature metadata, shared unchanged with host-side regression tests.
//!
//! Field 13 and trailer field 15 both contain the TOTAL signature byte count.
//! The consumer must use the byte count of one selected signature instead.

pub(super) fn per_signature_size(
    num_sigs: u32,
    header_total: u32,
    trailer_total: u32,
) -> Option<u32> {
    if num_sigs == 0
        || trailer_total == 0
        || header_total != trailer_total
        || trailer_total % num_sigs != 0
    {
        return None;
    }
    Some(trailer_total / num_sigs)
}
