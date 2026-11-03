//! Host-side test of the opt-in identity gate's pure logic
//! (`loader/src/identity_core.rs`). `loader` only builds for
//! `x86_64-unknown-uefi` and has no test harness of its own, so - same
//! `#[path]` trick already used by `claim_plm.rs` - this test includes the
//! real, unmodified source file verbatim.
extern crate alloc;
#[path = "../../loader/src/identity_core.rs"]
mod identity_core;
use identity_core::*;

#[test]
fn missing_or_invalid_reference_blocks() {
    for raw in [&b""[..], &b"abc"[..], &[b'g'; 64][..], &[b'0'; 65][..]] {
        assert!(parse_reference(raw).is_none());
    }
}

#[test]
fn reference_roundtrip_and_mismatch() {
    let expected = parse_reference(&[b'a'; 64]).unwrap();
    assert_eq!(expected, [0xaa; 32]);
    assert!(verify(expected, expected));
    assert!(!verify(expected, [0xbb; 32]));
}

#[test]
fn digest_known_vector_and_full_window_tail() {
    assert_eq!(
        digest(b"abc"),
        parse_reference(b"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
            .unwrap()
    );
    let mut prom = vec![0; PROM_BYTES];
    let before = digest(&prom);
    prom[PROM_BYTES - 1] = 1;
    assert_ne!(before, digest(&prom));
}

#[test]
fn failed_or_unstable_reads_block_without_retry() {
    for (first, second) in [
        (None, Some([1; 32])),
        (Some([1; 32]), None),
        (Some([1; 32]), Some([2; 32])),
    ] {
        let mut reads = 0;
        assert!(stable_digest(|| {
            reads += 1;
            if reads == 1 { first } else { second }
        })
        .is_none());
        assert!(reads <= 2);
    }
    assert_eq!(stable_digest(|| Some([3; 32])), Some([3; 32]));
}

#[test]
fn reference_rejects_extra_content_and_accepts_line_endings() {
    let mut raw = vec![b'a'; 64];
    raw.extend_from_slice(b"\r\n");
    assert!(parse_reference(&raw).is_some());
    raw.extend_from_slice(b"x");
    assert!(parse_reference(&raw).is_none());
    let mut oversized = vec![b' '; 2048];
    oversized.extend_from_slice(&[b'a'; 64]);
    assert!(parse_reference(&oversized).is_none());
}
