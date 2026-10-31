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

/// Why the fuse-selected stock signature could not be installed.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum SlotError {
    /// The fuse version selects a slot the table does not have (`-ERANGE`).
    IndexOutOfRange,
    /// Table, slot or image cannot hold the copy (`-EINVAL`).
    TableInconsistent,
}

/// `(offset, length)` of the fuse-selected signature inside the signature table.
///
/// Turing BOOT_DIRECT bundles carry exactly ONE signature (SIG_PROD, 16 B AES)
/// and no fuse-version table, so index 0 is pinned. The multi-slot layout is
/// GA10x/Ada, where the fuse version counts DOWN from the newest slot
/// (`num_sigs - 1 - fuse_ver`), matching prep_610.c gpu_unlock_booter_patch.
///
/// `num_sigs == 0` and `sig_bytes == 0` are unreachable after load_booter's
/// validation (the total is non-zero and divisible by the count); they fail
/// closed here rather than copying nothing silently.
pub(super) fn slot_range(
    num_sigs: u32,
    sig_bytes: u32,
    fuse_ver: u32,
    signature_table_len: usize,
) -> Result<(usize, usize), SlotError> {
    if num_sigs == 0 {
        return Err(SlotError::IndexOutOfRange);
    }
    if sig_bytes == 0 {
        return Err(SlotError::TableInconsistent);
    }
    let index = if num_sigs == 1 {
        0usize
    } else {
        if fuse_ver >= num_sigs {
            return Err(SlotError::IndexOutOfRange);
        }
        (num_sigs - 1 - fuse_ver) as usize
    };
    let len = sig_bytes as usize;
    let from = index.checked_mul(len).ok_or(SlotError::TableInconsistent)?;
    let end = from.checked_add(len).ok_or(SlotError::TableInconsistent)?;
    if end > signature_table_len {
        return Err(SlotError::TableInconsistent);
    }
    Ok((from, len))
}

/// Copy the fuse-selected stock signature into `patch_loc` of the booter image.
pub(super) fn install_stock_signature(
    image: &mut [u8],
    patch_loc: u32,
    num_sigs: u32,
    sig_bytes: u32,
    fuse_ver: u32,
    signatures: &[u8],
) -> Result<(), SlotError> {
    let (from, len) = slot_range(num_sigs, sig_bytes, fuse_ver, signatures.len())?;
    let at = patch_loc as usize;
    let end = at.checked_add(len).ok_or(SlotError::TableInconsistent)?;
    if end > image.len() {
        return Err(SlotError::TableInconsistent);
    }
    image[at..end].copy_from_slice(&signatures[from..from + len]);
    Ok(())
}
