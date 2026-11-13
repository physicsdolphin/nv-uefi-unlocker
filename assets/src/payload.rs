//! V67 signature-image payload generator (data-only; the words are ROP
//! gadget operands, semantics per gpu-unlock docs + cmp170hx rop-chain).
//!
//! Reference target: GA102 / 610.43.03 rejoin15 (pinned SHA-256 in
//! manifest). TU106 payloads, if ever built, come from a Turing-pinned
//! asset audit — never by editing these constants (task brief §9.1).

use crate::manifest::ga102_610::V67_PAYLOAD_SHA256;
use crate::manifest::sha256_hex;

pub const PAYLOAD_SIZE: usize = 0xFA00;
pub const FILL_DWORD: u32 = 0x0000_019C; // self-loop instruction (observable stop)

/// (offset, value) — offset in BYTES within the payload image.
pub const PATCHED_WORDS_GA102: &[(usize, u32)] = &[
    (0x1100, 0x00000007),
    (0xF948, 0xFFFFFFFF), // write value
    (0xF950, 0x00000D44),
    (0xF960, 0x00823804), // FEAT_OVR_PLM address
    (0xF968, 0x00001FCE),
    (0xF974, 0x00000000),
    (0xF97C, 0x00001101),
    (0xF980, 0x000084C8),
    (0xF984, 0x00008E18),
    (0xF98C, 0x000084C8),
    (0xF990, 0x00000000),
    (0xF998, 0x00001FCE),
    (0xF9A4, 0x0000FFBC),
    (0xF9AC, 0x00005789),
    (0xF9BC, 0x00000D44),
    (0xF9CC, 0x00000003),
    (0xF9D4, 0x00001FCE),
    (0xF9E8, 0x00000D52),
    (0xF9EC, 0x000081EE), // terminator
];

pub fn build_payload(fill: u32, size: usize, patched: &[(usize, u32)]) -> Vec<u8> {
    assert_eq!(size % 4, 0, "payload size must be dword-aligned");
    let mut out = vec![0u8; size];
    for chunk in out.as_chunks_mut::<4>().0 {
        chunk.copy_from_slice(&fill.to_le_bytes());
    }
    for &(off, value) in patched {
        assert!(off + 4 <= size, "patch word outside payload");
        out[off..off + 4].copy_from_slice(&value.to_le_bytes());
    }
    out
}

/// Build the pinned GA102 rejoin15 payload and verify its SHA-256.
pub fn build_pinned_ga102() -> Result<Vec<u8>, String> {
    let payload = build_payload(FILL_DWORD, PAYLOAD_SIZE, PATCHED_WORDS_GA102);
    let digest = sha256_hex(&payload);
    if digest != V67_PAYLOAD_SHA256 {
        return Err(format!("generated V67 image differs from pinned SHA-256 ({digest})"));
    }
    Ok(payload)
}

/// AD102 four-write chain (gpu-unlock-ada generate_v67.py, TOTORONG-derived):
/// ONE fire performs four chained PLM writes + the mailbox marker.
/// Gadgets: 0x21b2, 0x33fc, 0x256f, 0x7a02 (+ tail 0x31b5/0x771e).
/// Note WPR_CFG opens to 0xFFFFF0FF — bits [11:0] region deliberately not
/// all-ones (cmp170hx caution); FBPA/WPR/FEAT go to 0xFFFFFFFF.
pub const PATCHED_WORDS_AD102: &[(usize, u32)] = &[
    (0x1100, 0x00000007),
    (0xF948, 0xFFFFF0FF), // write value #1 (WPR_CFG)
    (0xF950, 0x000021B2), // gadget
    (0xF960, 0x001FA7CC), // addr #1 WPR_CFG
    (0xF968, 0x000033FC), // gadget
    (0xF974, 0xFFFFFFFF), // value #2 (FBPA)
    (0xF97C, 0x0000256F), // gadget
    (0xF980, 0x00007A02), // gadget
    (0xF984, 0x009A0148), // addr #2 FBPA
    (0xF98C, 0x000033FC),
    (0xF998, 0xFFFFFFFF), // value #3 (WPR)
    (0xF9A0, 0x0000256F),
    (0xF9A4, 0x00007A02),
    (0xF9A8, 0x001FA7C4), // addr #3 WPR
    (0xF9B0, 0x000033FC),
    (0xF9BC, 0xFFFFFFFF), // value #4 (FEAT)
    (0xF9C4, 0x0000256F),
    (0xF9C8, 0x00007A02),
    (0xF9CC, 0x00823804), // addr #4 FEAT
    (0xF9D4, 0x000033FC),
    (0xF9E0, 0x67B00001), // mailbox marker
    (0xF9E8, 0x0000256F),
    (0xF9EC, 0x00007A02),
    (0xF9F8, 0x000031B5),
    (0xF9FC, 0x0000771E),
];

use crate::manifest::ad102_610::V67_AD102_PAYLOAD_SHA256;

pub fn build_pinned_ad102() -> Result<Vec<u8>, String> {
    let payload = build_payload(FILL_DWORD, PAYLOAD_SIZE, PATCHED_WORDS_AD102);
    let digest = sha256_hex(&payload);
    if digest != V67_AD102_PAYLOAD_SHA256 {
        return Err(format!("AD102 payload differs from pinned SHA-256 ({digest})"));
    }
    Ok(payload)
}

/// TU102 (Turing) V67 signature image — GPL-2.0-only behavioral port of
/// nlzy/turing-unlock firmware/generate_v67.py build_payload() (610.43.03,
/// mutex-free variant; the same image is live-proven by cmp50hx-unlock
/// efi-unlock, 2026-09-10). Chained writes: FEAT_PLM=0xffffffff then
/// WPR2_HI=0, chained through the booter's write mid-entry 0x10B9; the
/// tail frames bake the ACR mutex-3 release (guard 0x6140 canary, tokens
/// 0xCBD/0xC95/0xE85, main SP 0xFFF0 / token 0xFFFF) and land on the
/// secure teardown 0x7C43. Fill 0x4A7 (TU102; GA102's is 0x19C).
/// Reference offsets are dmem-0x600; ours are payload bytes.
/// The reference writes teardown 0x7C43 at 0xF97C in its frame loop and
/// then overwrites that dword with 0xCBD — the flat list records the
/// post-overwrite value, matching byte-for-byte.
pub const FILL_DWORD_TU102: u32 = 0x0000_04A7;
pub const PATCHED_WORDS_TU102: &[(usize, u32)] = &[
    (0x1100, 0x0000_0007), // f100 restore
    (0xF938, 0x0000_6140), // guard
    (0xF940, 0xFFFF_FFFF), // write value #1 (FEAT_PLM open)
    (0xF944, 0x0040_9650), // addr #1 FEAT_PLM
    (0xF94C, 0x0000_10B9), // write mid-entry
    (0xF950, 0x0000_6140), // guard (chained frame 0)
    (0xF958, 0x0000_0000), // write value #2 (WPR2_HI=0)
    (0xF95C, 0x001F_A828), // addr #2 WPR2_HI
    (0xF964, 0x0000_10B9), // write mid-entry
    (0xF968, 0x0000_6140), // guard (chained frame 1)
    (0xF974, 0x0000_0000),
    (0xF97C, 0x0000_0CBD), // mutex token (reference: 0x7C43 then overwritten)
    (0xF980, 0x0000_0001),
    (0xF984, 0x0000_0000),
    (0xF988, 0x0000_8E18),
    (0xF98C, 0x0000_0000),
    (0xF990, 0x0000_04A7), // fill
    (0xF994, 0x0000_0C95),
    (0xF998, 0x0000_6140),
    (0xF99C, 0x0000_0000),
    (0xF9A0, 0x0000_0000),
    (0xF9A4, 0x0000_0001),
    (0xF9A8, 0x0000_04A7), // fill
    (0xF9AC, 0x0000_1F89),
    (0xF9B0, 0x0000_6140),
    (0xF9B4, 0x0000_6140),
    (0xF9B8, 0x0000_FFFF), // token = main SP 0xFFF0 + 0xF
    (0xF9BC, 0x0000_04A7), // fill
    (0xF9C0, 0x0000_1F89),
    (0xF9C4, 0x0000_6140),
    (0xF9C8, 0x0000_6140),
    (0xF9CC, 0x0000_0000),
    (0xF9D0, 0x0000_04A7), // fill
    (0xF9D4, 0x0000_0E85),
    (0xF9D8, 0x0000_1F89),
    (0xF9DC, 0x0000_6140),
    (0xF9E0, 0x0000_6140),
    (0xF9E4, 0x0000_0000),
    (0xF9E8, 0x0000_04A7), // fill
    (0xF9EC, 0x0000_7EA4),
    (0xF9F8, 0x0000_0000), // main SP + 8
];

use crate::manifest::tu10x_610::V67_TU10X_PAYLOAD_SHA256;

pub fn build_pinned_tu102() -> Result<Vec<u8>, String> {
    let payload = build_payload(FILL_DWORD_TU102, PAYLOAD_SIZE, PATCHED_WORDS_TU102);
    let digest = sha256_hex(&payload);
    if digest != V67_TU10X_PAYLOAD_SHA256 {
        return Err(format!("TU102 payload differs from pinned SHA-256 ({digest})"));
    }
    Ok(payload)
}

/// TU116/TU117 (GTX 16 series / CMP 30HX) V67 signature image — issue #151.
/// GPL-2.0-only behavioral port of the TU102 frame table (same lineage as
/// nlzy/turing-unlock and cmpunlocker cpu-rm generate_v67.py: fill 0x4a7,
/// guard 0x6140, chained FEAT_PLM/WPR2_HI writes through the 0x10B9
/// mid-entry, ACR mutex-3 release tail) with ONE re-anchored word from our
/// own TU116 audit: the decrypted TU116 debug secure segment (key family
/// #6) places the park self-loop (`lbra`) at 0x7E9A where TU102 has it at
/// 0x7EA4. Every other referenced entry site was checked against that
/// segment: 0x10B9 and 0x1F89 byte-identical, 0xC95 byte-identical,
/// 0xCBD/0xE85 entry-identical (only in-function lcall immediates differ,
/// and the payload never executes those bytes), canary address 0x6140
/// identical, DMEM data segments 99.3% identical (176/25344 bytes).
/// NOT field-proven: the booter image and its 16B prod signature are
/// die-specific (manifest::tu11x_610, pinned TU116 archive) and issue #151
/// is the verification path — read-back is the only verdict.
pub const PATCHED_WORDS_TU11X: &[(usize, u32)] = &[
    (0x1100, 0x0000_0007), // f100 restore
    (0xF938, 0x0000_6140), // guard
    (0xF940, 0xFFFF_FFFF), // write value #1 (FEAT_PLM open)
    (0xF944, 0x0040_9650), // addr #1 FEAT_PLM
    (0xF94C, 0x0000_10B9), // write mid-entry
    (0xF950, 0x0000_6140), // guard (chained frame 0)
    (0xF958, 0x0000_0000), // write value #2 (WPR2_HI=0)
    (0xF95C, 0x001F_A828), // addr #2 WPR2_HI
    (0xF964, 0x0000_10B9), // write mid-entry
    (0xF968, 0x0000_6140), // guard (chained frame 1)
    (0xF974, 0x0000_0000),
    (0xF97C, 0x0000_0CBD), // mutex token
    (0xF980, 0x0000_0001),
    (0xF984, 0x0000_0000),
    (0xF988, 0x0000_8E18),
    (0xF98C, 0x0000_0000),
    (0xF990, 0x0000_04A7), // fill
    (0xF994, 0x0000_0C95),
    (0xF998, 0x0000_6140),
    (0xF99C, 0x0000_0000),
    (0xF9A0, 0x0000_0000),
    (0xF9A4, 0x0000_0001),
    (0xF9A8, 0x0000_04A7), // fill
    (0xF9AC, 0x0000_1F89),
    (0xF9B0, 0x0000_6140),
    (0xF9B4, 0x0000_6140),
    (0xF9B8, 0x0000_FFFF), // token = main SP 0xFFF0 + 0xF
    (0xF9BC, 0x0000_04A7), // fill
    (0xF9C0, 0x0000_1F89),
    (0xF9C4, 0x0000_6140),
    (0xF9C8, 0x0000_6140),
    (0xF9CC, 0x0000_0000),
    (0xF9D0, 0x0000_04A7), // fill
    (0xF9D4, 0x0000_0E85),
    (0xF9D8, 0x0000_1F89),
    (0xF9DC, 0x0000_6140),
    (0xF9E0, 0x0000_6140),
    (0xF9E4, 0x0000_0000),
    (0xF9E8, 0x0000_04A7), // fill
    (0xF9EC, 0x0000_7E9A), // park (TU116 lbra site; TU102's is 0x7EA4)
    (0xF9F8, 0x0000_0000), // main SP + 8
];

use crate::manifest::tu11x_610::V67_TU11X_PAYLOAD_SHA256;

pub fn build_pinned_tu116() -> Result<Vec<u8>, String> {
    let payload = build_payload(FILL_DWORD_TU102, PAYLOAD_SIZE, PATCHED_WORDS_TU11X);
    let digest = sha256_hex(&payload);
    if digest != V67_TU11X_PAYLOAD_SHA256 {
        return Err(format!("TU116 payload differs from pinned SHA-256 ({digest})"));
    }
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ga102_reference_payload_reproduces_pinned_sha256() {
        // GPL-2.0-only behavioral port check: identical bytes to the
        // reference generator output.
        let payload = build_pinned_ga102().expect("pinned payload must reproduce");
        assert_eq!(payload.len(), 0xFA00);
        assert_eq!(&payload[0..4], &0x19C_u32.to_le_bytes());
        assert_eq!(&payload[0xF9EC..0xF9F0], &0x81EE_u32.to_le_bytes());
    }

    #[test]
    fn tu102_reference_payload_reproduces_pinned_sha256() {
        // nlzy/turing-unlock generate_v67.py ground truth (pinned sha in
        // manifest::tu10x_610) — validates the hand-transcribed patch table.
        let payload = build_pinned_tu102().expect("pinned payload must reproduce");
        assert_eq!(payload.len(), 0xFA00);
        assert_eq!(&payload[0..4], &0x4A7_u32.to_le_bytes());
        assert_eq!(&payload[0xF940..0xF944], &0xFFFF_FFFF_u32.to_le_bytes());
        assert_eq!(&payload[0xF944..0xF948], &0x0040_9650_u32.to_le_bytes());
        assert_eq!(&payload[0xF95C..0xF960], &0x001F_A828_u32.to_le_bytes());
        assert_eq!(&payload[0xF97C..0xF980], &0x0CBD_u32.to_le_bytes());
    }

    #[test]
    fn tu116_payload_reproduces_pinned_sha256() {
        // TU102 table with the park word re-anchored to the TU116 debug
        // segment (0x7EA4 -> 0x7E9A); sha pinned in manifest::tu11x_610.
        // RE-derived, NOT field-proven — issue #151 is the verification path.
        let payload = build_pinned_tu116().expect("pinned payload must reproduce");
        assert_eq!(payload.len(), 0xFA00);
        assert_eq!(&payload[0..4], &0x4A7_u32.to_le_bytes());
        assert_eq!(&payload[0xF944..0xF948], &0x0040_9650_u32.to_le_bytes());
        assert_eq!(&payload[0xF9EC..0xF9F0], &0x7E9A_u32.to_le_bytes());
    }

    #[test]
    fn tu11x_table_differs_from_tu10x_by_park_word_only() {
        assert_eq!(PATCHED_WORDS_TU102.len(), PATCHED_WORDS_TU11X.len());
        let diffs: Vec<_> = PATCHED_WORDS_TU102
            .iter()
            .zip(PATCHED_WORDS_TU11X.iter())
            .filter(|(a, b)| a.0 != b.0 || a.1 != b.1)
            .collect();
        assert_eq!(diffs, [(&{ (0xF9EC, 0x0000_7EA4u32) }, &{ (0xF9EC, 0x0000_7E9Au32) })]);
    }

    #[test]
    fn small_generator_semantics() {
        let p = build_payload(0xAA, 16, &[(4, 0x11223344)]);
        assert_eq!(&p[0..4], &0xAA_u32.to_le_bytes());
        assert_eq!(&p[4..8], &0x11223344_u32.to_le_bytes());
        assert_eq!(&p[12..16], &0xAA_u32.to_le_bytes());
    }

    #[test]
    #[should_panic]
    fn out_of_range_patch_rejected() {
        build_payload(0, 8, &[(6, 1)]);
    }
}
