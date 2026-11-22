//! Textually-identical mirror of loader/src/ebswatch.rs's pure logic: the
//! write-budget policy (the self-perturbation guard) and the CRC-32 used for
//! the post-patch gBS header fixup. Do not reformat one side without the
//! other - these tests pin the behavior the EFI side must match.

/// Line classes with independent budgets (mirror of ebswatch::WatchLine).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WatchLine {
    Enter,
    Before,
    ErrReturn,
}

/// Budget check for one line class (mirror of ebswatch::file_write_allowed).
/// Enter<=1 + Before<=1 collapse into at most ONE manufactured
/// INVALID_PARAMETER retry; ErrReturn<=2 because beyond that a persistent
/// INVALID_PARAMETER loop is genuinely third-party.
pub fn file_write_allowed(kind: WatchLine, count: usize) -> bool {
    match kind {
        WatchLine::Enter => count <= 1,
        WatchLine::Before => count <= 1,
        WatchLine::ErrReturn => count <= 2,
    }
}

/// CRC-32 (reflected 0xEDB88320, init/xorout 0xffffffff) - the UEFI table
/// header checksum algorithm (edk2 `CalculateCrc32`, MdePkg BaseLib
/// CheckSum.c). Mirror of ebswatch::crc32.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    crc ^ 0xffff_ffff
}

#[test]
fn t_ebs_watch_budget_one_manufactured_retry() {
    use WatchLine::*;
    assert!(file_write_allowed(Enter, 1));
    assert!(
        !file_write_allowed(Enter, 2),
        "enter#2 must be budgeted out - enter#1 + before#1 collapse into at most ONE manufactured MapKey retry"
    );
    assert!(file_write_allowed(Before, 1));
    assert!(
        !file_write_allowed(Before, 2),
        "BeforeEBS fires exactly once per boot (edk2 mExitBootServicesCalled is never reset on a retry)"
    );
    assert!(file_write_allowed(ErrReturn, 1));
    assert!(file_write_allowed(ErrReturn, 2));
    assert!(
        !file_write_allowed(ErrReturn, 3),
        "error-return writes stop after two - a persistent INVALID_PARAMETER loop beyond that is genuinely third-party and must not be fed by us"
    );
}

#[test]
fn t_crc32_standard_vectors() {
    assert_eq!(crc32(b""), 0x0000_0000);
    assert_eq!(crc32(b"123456789"), 0xcbf4_3926, "standard CRC-32 check vector");
    // The edk2 shape: over a table with the crc field zeroed - deterministic
    // and independent of the input length.
    let a = crc32(b"payload-with-\x00-nulls");
    let b = crc32(b"payload-with-\x00-nulls");
    assert_eq!(a, b);
}
