//! Pure (no UEFI calls) parsing/verification logic for the opt-in identity
//! gate. Kept host-testable: `sim/tests/identity.rs` includes this file
//! verbatim via `#[path]` since `loader` only builds for
//! `x86_64-unknown-uefi` and has no test harness of its own.
use sha2::{Digest, Sha256};

pub const PROM_BYTES: usize = 1024 * 1024;

/// Parse a pinned SHA-256 reference: exactly 64 hex digits, optional
/// surrounding whitespace, at most 66 bytes total (64 + CRLF). Anything else
/// (missing, short, long, non-hex, oversized) is rejected - a malformed
/// reference must refuse, never silently fall back to "no reference".
#[allow(dead_code)]
pub fn parse_reference(raw: &[u8]) -> Option<[u8; 32]> {
    if raw.len() > 66 {
        return None;
    }
    let text = core::str::from_utf8(raw).ok()?.trim();
    if text.len() != 64 {
        return None;
    }
    let mut result = [0; 32];
    for (i, pair) in text.as_bytes().chunks_exact(2).enumerate() {
        let nibble = |b: u8| match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        };
        result[i] = nibble(pair[0])? * 16 + nibble(pair[1])?;
    }
    Some(result)
}

#[allow(dead_code)]
pub fn verify(expected: [u8; 32], measured: [u8; 32]) -> bool {
    expected == measured
}

pub fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Two independent reads must agree before the digest is trusted - an
/// unstable or failed read blocks without retrying past the second attempt.
pub fn stable_digest(mut read: impl FnMut() -> Option<[u8; 32]>) -> Option<[u8; 32]> {
    let first = read()?;
    let second = read()?;
    (first == second).then_some(first)
}
