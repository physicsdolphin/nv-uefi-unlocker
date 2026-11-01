//! Pure, shared DXE authorization encoding. Legacy NVUX1 grants are rejected.
//! The digest covers the manifest AND presence/content of every control file.
extern crate alloc;
use alloc::vec::Vec;
use sha2::{Digest, Sha256};

pub const MAGIC: &[u8; 5] = b"NVUX2";
pub const ARM_LEN: usize = 53;
pub const ATTEMPT_LIMIT: u64 = 2;
pub const MANIFEST: &str = "\\dxe-manifest.txt";
// Deny unknown driver configuration by default. Add new controls here before
// exposing them to a resident transaction; absence is part of the digest too.
pub const CONFIG_PATHS: &[&str] = &[
    "\\tx-dxe-arm.txt",
    "\\tx-dxe-noconfirm.txt",
    "\\tx-quiet.txt",
    "\\tx-flush-lines.txt",
    "\\tx-select-bdf.txt",
    "\\tx-accept-warm-wpr2.txt",
    "\\tx-force-continue.txt",
    "\\tx-low-staging.txt",
    "\\tx-no-pm-preflight.txt",
    "\\pre-flr-on.txt",
    "\\tx-require-d0.txt",
    "\\tx-gop-disconnect.txt",
    "\\no-reconnect.txt",
    "\\tu10x-no-reconnect.txt",
    "\\tu10x-no-ss.txt",
    "\\tx-fwsec-high-staging.txt",
    "\\tx-fwsec-vram-staging.txt",
    "\\tx-no-gsp-halt-gate.txt",
    "\\tx-reconnect-nonrecursive.txt",
    "\\tx-sec-base.txt",
    "\\tx-bootvec-sec.txt",
    "\\tx-host-probe.txt",
    "\\tx-ptimer-seed.txt",
    "\\tx-no-bridge-master.txt",
    "\\tx-meta-in-vram.txt",
    "\\tx-sec2-c-load.txt",
    "\\tu10x-fb-gb.txt",
    MANIFEST,
];
pub type Snapshot = Vec<Option<Vec<u8>>>;

pub fn fingerprint(snapshot: &Snapshot) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"NvUnlockDxe policy v2\0");
    for (path, value) in CONFIG_PATHS.iter().zip(snapshot) {
        h.update((path.len() as u64).to_le_bytes());
        h.update(path.as_bytes());
        match value {
            None => h.update([0]),
            Some(bytes) => {
                h.update([1]);
                h.update((bytes.len() as u64).to_le_bytes());
                h.update(bytes);
            }
        }
    }
    h.finalize().into()
}

pub fn encode(identity: [u8; 16], snapshot: &Snapshot) -> Vec<u8> {
    let mut out = Vec::with_capacity(ARM_LEN);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&identity);
    out.extend_from_slice(&fingerprint(snapshot));
    out
}

pub fn decode(bytes: &[u8]) -> Option<([u8; 16], [u8; 32])> {
    if bytes.len() != ARM_LEN || &bytes[..5] != MAGIC {
        return None;
    }
    Some((
        bytes[5..21].try_into().ok()?,
        bytes[21..53].try_into().ok()?,
    ))
}

pub fn attempt_count(bytes: Option<&[u8]>) -> Option<u64> {
    let Some(bytes) = bytes else { return Some(0) };
    if bytes.is_empty() || bytes.len() > 20 {
        return None;
    }
    bytes.iter().try_fold(0u64, |n, &b| {
        if !b.is_ascii_digit() {
            return None;
        }
        n.checked_mul(10)?.checked_add(u64::from(b - b'0'))
    })
}

/// Verify exactly the in-memory bytes used by the transaction, including
/// optional absence (a removed expected payload must not downgrade modes).
pub fn verify_assets<'a>(
    manifest: &[u8],
    entries: impl Iterator<Item = (&'a str, Option<&'a [u8]>)>,
) -> Result<(), &'static str> {
    use sha2::{Digest, Sha256};
    let text = core::str::from_utf8(manifest).map_err(|_| "manifest is not UTF-8")?;
    for (path, bytes) in entries {
        let matches: alloc::vec::Vec<_> = text
            .lines()
            .filter(|line| {
                line.len() >= 66
                    && line.as_bytes()[..64].iter().all(u8::is_ascii_hexdigit)
                    && line[64..].trim_start() == path
            })
            .collect();
        match bytes {
            None if matches.is_empty() => {} // explicit replay/optional absence
            Some(bytes) if matches.len() == 1 => {
                let hex: alloc::string::String = Sha256::digest(bytes)
                    .iter()
                    .map(|b| alloc::format!("{b:02x}"))
                    .collect();
                if !matches[0][..64].eq_ignore_ascii_case(&hex) {
                    return Err("asset digest mismatch");
                }
            }
            _ => return Err("manifest presence mismatch or duplicate asset entry"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authorization_pins_presence_contents_manifest_and_version() {
        let mut s = alloc::vec![None; CONFIG_PATHS.len()];
        let baseline = fingerprint(&s);
        s[0] = Some(Vec::new());
        assert_ne!(baseline, fingerprint(&s), "empty present != absent");
        let empty = fingerprint(&s);
        s[0] = Some(b"1".to_vec());
        assert_ne!(empty, fingerprint(&s));
        let arm = encode([7; 16], &s);
        assert_eq!(decode(&arm), Some(([7; 16], fingerprint(&s))));
        s[CONFIG_PATHS.len() - 1] = Some(b"changed manifest".to_vec());
        assert_ne!(decode(&arm).unwrap().1, fingerprint(&s));
        assert!(decode(b"NVUX1abcdefghijklmnop").is_none());
    }
    #[test]
    fn malformed_counter_fails_closed() {
        assert_eq!(attempt_count(None), Some(0));
        assert_eq!(attempt_count(Some(b"2")), Some(2));
        for b in [
            b"".as_slice(),
            b"garbage",
            b"1oops",
            b"18446744073709551616",
        ] {
            assert_eq!(attempt_count(Some(b)), None);
        }
    }
}
