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

/// AD102 BAR1-64G variant (gpu-unlock-bar1 bar1-64g-report.md, 2026-10-24,
/// eight 10de:2684 cards): the SAME four-write chain skeleton as
/// PATCHED_WORDS_AD102 with only the write operands and mailbox marker
/// swapped — CYA 0x30 + CFG 0x8000000a build the 64 GiB BAR1 construct,
/// XVE PLM + FEAT PLM open (FEAT keeps the tensor/SS unlock path alive).
/// The generated image reproduces the reference's published SHA-256
/// byte-for-byte (pinned below), i.e. this is the field-proven artifact,
/// not a novel construction. Marker 0x67B00004 distinguishes the variant.
pub const PATCHED_WORDS_AD102_BAR1_64G: &[(usize, u32)] = &[
    (0x1100, 0x00000007),
    (0xF948, 0x00000030), // write value #1 (CYA, 64 GiB construct)
    (0xF950, 0x000021B2), // gadget
    (0xF960, 0x00088724), // addr #1 CYA
    (0xF968, 0x000033FC), // gadget
    (0xF974, 0x8000000A), // value #2 (CFG, 64 GiB construct)
    (0xF97C, 0x0000256F), // gadget
    (0xF980, 0x00007A02), // gadget
    (0xF984, 0x00088DCC), // addr #2 CFG
    (0xF98C, 0x000033FC),
    (0xF998, 0xFFFFFFFF), // value #3 (XVE PLM open)
    (0xF9A0, 0x0000256F),
    (0xF9A4, 0x00007A02),
    (0xF9A8, 0x00088FF4), // addr #3 XVE PLM
    (0xF9B0, 0x000033FC),
    (0xF9BC, 0xFFFFFFFF), // value #4 (FEAT PLM open - keeps SS unlock)
    (0xF9C4, 0x0000256F),
    (0xF9C8, 0x00007A02),
    (0xF9CC, 0x00823804), // addr #4 FEAT
    (0xF9D4, 0x000033FC),
    (0xF9E0, 0x67B00004), // mailbox marker (bar1-64g variant)
    (0xF9E8, 0x0000256F),
    (0xF9EC, 0x00007A02),
    (0xF9F8, 0x000031B5),
    (0xF9FC, 0x0000771E),
];

use crate::manifest::ad102_610::V67_AD102_BAR1_64G_PAYLOAD_SHA256;

pub fn build_pinned_ad102_bar1_64g() -> Result<Vec<u8>, String> {
    let payload = build_payload(FILL_DWORD, PAYLOAD_SIZE, PATCHED_WORDS_AD102_BAR1_64G);
    let digest = sha256_hex(&payload);
    if digest != V67_AD102_BAR1_64G_PAYLOAD_SHA256 {
        return Err(format!(
            "AD102 bar1-64g payload differs from pinned SHA-256 ({digest})"
        ));
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
    fn ad102_bar1_64g_reference_payload_reproduces_published_sha256() {
        // gpu-unlock-bar1 bar1-64g-report.md published digest — the variant
        // built by re-parameterizing our four-write table is byte-identical
        // to the eight-card field-proven artifact.
        let payload = build_pinned_ad102_bar1_64g().expect("pinned payload must reproduce");
        assert_eq!(payload.len(), 0xFA00);
        assert_eq!(&payload[0..4], &0x19C_u32.to_le_bytes());
        assert_eq!(&payload[0xF948..0xF94C], &0x30_u32.to_le_bytes());
        assert_eq!(&payload[0xF960..0xF964], &0x0008_8724_u32.to_le_bytes());
        assert_eq!(&payload[0xF9E0..0xF9E4], &0x67B0_0004_u32.to_le_bytes());
    }

    #[test]
    fn ad102_bar1_64g_shares_gadget_skeleton_with_four_plm() {
        // Constitution-adjacent proof: the bar1 variant differs from the
        // M4-verified four-write chain ONLY at operand slots (four value/addr
        // pairs + the mailbox marker); every gadget word, guard and tail is
        // identical, so the variant introduces no new ROP. Write #4 (FEAT
        // PLM) is deliberately the SAME write in both chains, and #3's value
        // is all-ones in both - only the marker must differ.
        const OPERAND_SLOTS: [usize; 9] = [
            0xF948, 0xF960, 0xF974, 0xF984, 0xF998, 0xF9A8, 0xF9BC, 0xF9CC, 0xF9E0,
        ];
        assert_eq!(PATCHED_WORDS_AD102.len(), PATCHED_WORDS_AD102_BAR1_64G.len());
        for (a, b) in PATCHED_WORDS_AD102.iter().zip(PATCHED_WORDS_AD102_BAR1_64G.iter()) {
            assert_eq!(a.0, b.0, "table layouts diverge at {:#x}", a.0);
            if a.1 != b.1 {
                assert!(
                    OPERAND_SLOTS.contains(&a.0),
                    "non-operand word changed at {:#x} ({:#x} -> {:#x})",
                    a.0,
                    a.1,
                    b.1
                );
            }
        }
        // The variant discriminator itself: marker 0x67b00001 -> 0x67b00004.
        let marker = |t: &[(usize, u32)]| t.iter().find(|w| w.0 == 0xF9E0).unwrap().1;
        assert_ne!(
            marker(PATCHED_WORDS_AD102),
            marker(PATCHED_WORDS_AD102_BAR1_64G),
            "mailbox marker must distinguish the variant"
        );
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
