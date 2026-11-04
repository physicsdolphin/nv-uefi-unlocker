//! fuc5 ISA tables — a Rust transcription of envydis/falcon.c (envytools,
//! MIT) plus the decode-relevant parts of dis-intern.h. GPL-2.0-only in
//! this repo per the surrounding project; the upstream file is pure data
//! (18 tables, no per-instruction C logic) and the decode semantics live
//! in decode.rs (a port of core-dis.c's engine).
//!
//! Entry model mirrors `struct insn`: first-match linear scan of
//! `(opcode & mask) == val` with a feature-mask gate, then the entry's
//! atom ops execute in order. Every table keeps its OOPS catch-all so a
//! scan always terminates. Decoding is position-independent: any byte
//! offset can start an instruction (mid-entry gadgets).

/// Feature bits (falcon.c:27-33).
pub const F_FUC0: u8 = 0x01;
pub const F_FUC3P: u8 = 0x02;
pub const F_FUC4P: u8 = 0x04;
pub const F_CRYPT: u8 = 0x08;
pub const F_FUC5P: u8 = 0x10;
pub const F_FUCOLD: u8 = 0x20;
pub const F_FUC6P: u8 = 0x40;

/// Variant feature masks (falcon_prep); crypt is opt-in, attached to no
/// variant, so it rides alongside as a separate flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Variant {
    pub fmask: u8,
    pub crypt: bool,
}

pub const FUC5: Variant = Variant { fmask: F_FUC3P | F_FUC4P | F_FUC5P, crypt: false };
pub const FUC5_CRYPT: Variant = Variant { fmask: F_FUC3P | F_FUC4P | F_FUC5P, crypt: true };

pub fn var_ok(fmask: u8, v: Variant) -> bool {
    fmask == 0 || (v.fmask & fmask) == fmask || (fmask == F_CRYPT && v.crypt)
}

#[derive(Clone, Copy, Debug)]
pub struct Sbf {
    pub pos: u8,
    pub len: u8,
}

const fn sbf(pos: u8, len: u8) -> Sbf {
    Sbf { pos, len }
}

const SBF_NONE: Sbf = Sbf { pos: 0, len: 0 };

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sign {
    Unsigned,
    Signed,
    SlightlySigned,
    UltraSigned,
}

/// `struct rbitfield` — immediates and branch/call targets.
#[derive(Clone, Copy, Debug)]
pub struct RBitField {
    pub sbf: [Sbf; 2],
    pub mode: Sign,
    pub shr: u8,
    pub pcrel: bool,
    pub addend: u64,
    pub pospreadd: u64,
    pub wrapok: bool,
}

/// `struct bitfield` — plain bitfields (falcon has no LUT use).
#[derive(Clone, Copy, Debug)]
pub struct BitField {
    pub sbf: [Sbf; 2],
    pub mode: Sign,
    pub shr: u8,
    pub addend: u64,
    pub xorend: u64,
}

const fn rbf_u(pos: u8, len: u8) -> RBitField {
    RBitField { sbf: [sbf(pos, len), SBF_NONE], mode: Sign::Unsigned, shr: 0, pcrel: false, addend: 0, pospreadd: 0, wrapok: false }
}

const fn rbf_u_shr(pos: u8, len: u8, shr: u8) -> RBitField {
    RBitField { sbf: [sbf(pos, len), SBF_NONE], mode: Sign::Unsigned, shr, pcrel: false, addend: 0, pospreadd: 0, wrapok: false }
}

const fn rbf_s(pos: u8, len: u8) -> RBitField {
    RBitField { sbf: [sbf(pos, len), SBF_NONE], mode: Sign::Signed, shr: 0, pcrel: false, addend: 0, pospreadd: 0, wrapok: false }
}

const fn bf_u(pos: u8, len: u8) -> BitField {
    BitField { sbf: [sbf(pos, len), SBF_NONE], mode: Sign::Unsigned, shr: 0, addend: 0, xorend: 0 }
}

/// Code target fields (falcon.c:39-44).
pub static PCREL16OFF: RBitField = RBitField { sbf: [sbf(16, 16), SBF_NONE], mode: Sign::Signed, shr: 0, pcrel: true, addend: 0, pospreadd: 0, wrapok: true };
pub static PCREL8OFF: RBitField = RBitField { sbf: [sbf(16, 8), SBF_NONE], mode: Sign::Signed, shr: 0, pcrel: true, addend: 0, pospreadd: 0, wrapok: false };
pub static FPCREL16OFF: RBitField = RBitField { sbf: [sbf(24, 16), SBF_NONE], mode: Sign::Signed, shr: 0, pcrel: true, addend: 0, pospreadd: 0, wrapok: true };
pub static FPCREL8OFF: RBitField = RBitField { sbf: [sbf(24, 8), SBF_NONE], mode: Sign::Signed, shr: 0, pcrel: true, addend: 0, pospreadd: 0, wrapok: false };
pub static FFPCREL16OFF: RBitField = RBitField { sbf: [sbf(32, 16), SBF_NONE], mode: Sign::Signed, shr: 0, pcrel: true, addend: 0, pospreadd: 0, wrapok: true };
pub static FFPCREL8OFF: RBitField = RBitField { sbf: [sbf(32, 8), SBF_NONE], mode: Sign::Signed, shr: 0, pcrel: true, addend: 0, pospreadd: 0, wrapok: false };

/// Special registers (falcon.c:63-80).
pub struct SReg {
    pub num: i8,
    pub name: &'static str,
    pub fmask: u8,
}

pub static SREG_SR: &[SReg] = &[
    SReg { num: 0x0, name: "iv0", fmask: 0 },
    SReg { num: 0x1, name: "iv1", fmask: 0 },
    SReg { num: 0x3, name: "tv", fmask: 0 },
    SReg { num: 0x4, name: "sp", fmask: 0 },
    SReg { num: 0x5, name: "pc", fmask: 0 },
    SReg { num: 0x6, name: "xcbase", fmask: 0 },
    SReg { num: 0x7, name: "xdbase", fmask: 0 },
    SReg { num: 0x8, name: "flags", fmask: 0 },
    SReg { num: 0x9, name: "cx", fmask: F_CRYPT },
    SReg { num: 0xa, name: "cauth", fmask: F_CRYPT },
    SReg { num: 0xb, name: "xtargets", fmask: 0 },
    SReg { num: 0xc, name: "tstatus", fmask: F_FUC3P },
    SReg { num: 0xd, name: "cauth1", fmask: F_FUC6P | F_CRYPT },
    SReg { num: 0xe, name: "xcbase1", fmask: F_FUC6P },
    SReg { num: 0xf, name: "xdbase1", fmask: F_FUC6P },
];

/// `struct reg` (dis-intern.h:144-152). Only the fields falcon uses.
pub struct RegDef {
    pub bf: Option<BitField>,
    pub name: &'static str,
    pub specials: &'static [SReg],
    pub always_special: bool,
}

pub static REG0_R: RegDef = RegDef { bf: Some(bf_u(0, 4)), name: "r", specials: &[], always_special: false };
pub static REG1_R: RegDef = RegDef { bf: Some(bf_u(12, 4)), name: "r", specials: &[], always_special: false };
pub static REG2_R: RegDef = RegDef { bf: Some(bf_u(8, 4)), name: "r", specials: &[], always_special: false };
pub static REG3_R: RegDef = RegDef { bf: Some(bf_u(20, 4)), name: "r", specials: &[], always_special: false };
pub static PRED1_R: RegDef = RegDef { bf: Some(bf_u(8, 3)), name: "p", specials: &[], always_special: false };
pub static PRED2_R: RegDef = RegDef { bf: Some(bf_u(16, 3)), name: "p", specials: &[], always_special: false };
pub static CREG1_R: RegDef = RegDef { bf: Some(bf_u(16, 3)), name: "c", specials: &[], always_special: false };
pub static CREG2_R: RegDef = RegDef { bf: Some(bf_u(20, 3)), name: "c", specials: &[], always_special: false };
pub static SREG1_R: RegDef = RegDef { bf: Some(bf_u(12, 4)), name: "s", specials: SREG_SR, always_special: true };
pub static SREG2_R: RegDef = RegDef { bf: Some(bf_u(8, 4)), name: "s", specials: SREG_SR, always_special: true };
pub static SP_R: RegDef = RegDef { bf: None, name: "sp", specials: &[], always_special: false };
pub static FLAGS_R: RegDef = RegDef { bf: None, name: "flags", specials: &[], always_special: false };

/// Immediate fields (falcon.c:118-132).
pub static IMM16OFF: RBitField = rbf_u(16, 16);
pub static IMM8OFF: RBitField = rbf_u(16, 8);
pub static IMM16SOFF: RBitField = rbf_s(16, 16);
pub static IMM8SOFF: RBitField = rbf_s(16, 8);
pub static IMM16WOFF: RBitField = RBitField { sbf: [sbf(16, 16), SBF_NONE], mode: Sign::Unsigned, shr: 0, pcrel: false, addend: 0, pospreadd: 0, wrapok: true };
pub static IMM16HOFF: RBitField = rbf_u_shr(16, 16, 16);
pub static IMM8HOFF: RBitField = rbf_u_shr(16, 8, 16);
pub static FIMM8SOFF: RBitField = rbf_s(8, 8);
pub static FIMM16SOFF: RBitField = rbf_s(8, 16);
pub static FIMM24SOFF: RBitField = rbf_s(8, 24);
pub static FIMM16OFF: RBitField = rbf_u(8, 16);
pub static FIMM24OFF: RBitField = rbf_u(8, 24);
pub static FIMM32OFF: RBitField = rbf_u(8, 32);
pub static STRAPOFF: BitField = bf_u(8, 2);
pub static CIMM2OFF: BitField = bf_u(20, 6);

/// BITF8/16 two-piece ranges (falcon.c:148-151).
pub static BITF8BF: [BitField; 2] = [bf_u(16, 5), bf_u(21, 3)];
pub static BITF16BF: [BitField; 2] = [bf_u(16, 5), bf_u(21, 5)];

/// Memory forms (falcon.c:157-202).
pub struct MemForm {
    pub name: &'static str,
    /// falcon never sets an index bitfield on its mem forms; kept for
    /// struct fidelity with C `struct mem`.
    pub idx: Option<&'static BitField>,
    pub reg: Option<&'static RegDef>,
    pub imm: Option<&'static RBitField>,
    pub reg2: Option<&'static RegDef>,
    pub reg2shr: u8,
}

pub static OFF8_BF: RBitField = rbf_u_shr(16, 8, 0);
pub static OFF16_BF: RBitField = rbf_u_shr(16, 8, 1);
pub static OFF32_BF: RBitField = rbf_u_shr(16, 8, 2);

macro_rules! memform {
    ($n:ident, $reg:expr, $imm:expr, $reg2:expr, $shr:expr) => {
        pub static $n: MemForm = MemForm { name: "D", idx: None, reg: $reg, imm: $imm, reg2: $reg2, reg2shr: $shr };
    };
}

memform!(DATAR_M, Some(&REG1_R), None, None, 0);
memform!(DATARI8_M, Some(&REG1_R), Some(&OFF8_BF), None, 0);
memform!(DATARI16_M, Some(&REG1_R), Some(&OFF16_BF), None, 0);
memform!(DATARI32_M, Some(&REG1_R), Some(&OFF32_BF), None, 0);
memform!(DATARR8_M, Some(&REG1_R), None, Some(&REG2_R), 0);
memform!(DATARR16_M, Some(&REG1_R), None, Some(&REG2_R), 1);
memform!(DATARR32_M, Some(&REG1_R), None, Some(&REG2_R), 2);
memform!(DATARRALT8_M, Some(&REG1_R), None, Some(&REG3_R), 0);
memform!(DATARRALT16_M, Some(&REG1_R), None, Some(&REG3_R), 1);
memform!(DATARRALT32_M, Some(&REG1_R), None, Some(&REG3_R), 2);
memform!(DATASP8_M, Some(&SP_R), Some(&OFF8_BF), None, 0);
memform!(DATASP16_M, Some(&SP_R), Some(&OFF16_BF), None, 0);
memform!(DATASP32_M, Some(&SP_R), Some(&OFF32_BF), None, 0);
memform!(DATASPR8_M, Some(&SP_R), None, Some(&REG2_R), 0);
memform!(DATASPR16_M, Some(&SP_R), None, Some(&REG2_R), 1);
memform!(DATASPR32_M, Some(&SP_R), None, Some(&REG2_R), 2);

pub static IOR_M: MemForm = MemForm { name: "I", idx: None, reg: Some(&REG1_R), imm: None, reg2: None, reg2shr: 0 };
pub static IORR_M: MemForm = MemForm { name: "I", idx: None, reg: Some(&REG1_R), imm: None, reg2: Some(&REG2_R), reg2shr: 2 };
pub static IORI_M: MemForm = MemForm { name: "I", idx: None, reg: Some(&REG1_R), imm: Some(&OFF32_BF), reg2: None, reg2shr: 0 };

/// Atom ops (dis-intern.h macros → enum). The decode-side behavior of
/// each lives in decode.rs; Unk covers both `OOPS` ("???") and `U(...)`.
#[derive(Clone, Copy)]
pub enum Op {
    Tab(&'static [InsnEntry]),
    Oplen(u8),
    Name(&'static str),
    Reg(&'static RegDef),
    RImm(&'static RBitField),
    BTarget(&'static RBitField),
    CTarget(&'static RBitField),
    Imm(&'static BitField),
    Mem(&'static MemForm),
    Bf(&'static [BitField; 2]),
    Endmark,
    Unk(&'static str),
}

pub struct InsnEntry {
    pub val: u64,
    pub mask: u64,
    pub ops: &'static [Op],
    pub fmask: u8,
}

const fn e(val: u64, mask: u64, ops: &'static [Op]) -> InsnEntry {
    InsnEntry { val, mask, ops, fmask: 0 }
}

const fn ef(val: u64, mask: u64, ops: &'static [Op], fmask: u8) -> InsnEntry {
    InsnEntry { val, mask, ops, fmask }
}

const OOPS: Op = Op::Unk("???");

/// tabp (falcon.c:204-229) — branch predicates.
pub static TABP: &[InsnEntry] = &[
    e(0x00000000, 0x00001800, &[Op::Reg(&PRED1_R)]),
    e(0x00000800, 0x00001f00, &[Op::Name("b")]),
    e(0x00000800, 0x00001f00, &[Op::Name("c")]),
    e(0x00000900, 0x00001f00, &[Op::Name("o")]),
    e(0x00000a00, 0x00001f00, &[Op::Name("s")]),
    e(0x00000b00, 0x00001f00, &[Op::Name("e")]),
    e(0x00000b00, 0x00001f00, &[Op::Name("z")]),
    e(0x00000c00, 0x00001f00, &[Op::Name("a")]),
    e(0x00000d00, 0x00001f00, &[Op::Name("be")]),
    e(0x00000d00, 0x00001f00, &[Op::Name("na")]),
    e(0x00000e00, 0x00001f00, &[Op::Endmark]),
    e(0x00001000, 0x00001800, &[Op::Name("not"), Op::Reg(&PRED1_R)]),
    e(0x00001800, 0x00001f00, &[Op::Name("ae")]),
    e(0x00001800, 0x00001f00, &[Op::Name("nb")]),
    e(0x00001800, 0x00001f00, &[Op::Name("nc")]),
    e(0x00001900, 0x00001f00, &[Op::Name("no")]),
    e(0x00001a00, 0x00001f00, &[Op::Name("ns")]),
    e(0x00001b00, 0x00001f00, &[Op::Name("ne")]),
    e(0x00001b00, 0x00001f00, &[Op::Name("nz")]),
    ef(0x00001c00, 0x00001f00, &[Op::Name("g")], F_FUC3P),
    ef(0x00001d00, 0x00001f00, &[Op::Name("le")], F_FUC3P),
    ef(0x00001e00, 0x00001f00, &[Op::Name("l")], F_FUC3P),
    ef(0x00001f00, 0x00001f00, &[Op::Name("ge")], F_FUC3P),
    e(0, 0, &[OOPS]),
];

/// tabfl (falcon.c:231-245) — flag conditions.
pub static TABFL: &[InsnEntry] = &[
    e(0x00000000, 0x00180000, &[Op::Reg(&PRED2_R)]),
    e(0x00080000, 0x001f0000, &[Op::Name("c")]),
    e(0x00090000, 0x001f0000, &[Op::Name("o")]),
    e(0x000a0000, 0x001f0000, &[Op::Name("s")]),
    e(0x000b0000, 0x001f0000, &[Op::Name("z")]),
    e(0x00100000, 0x001f0000, &[Op::Name("ie0")]),
    e(0x00110000, 0x001f0000, &[Op::Name("ie1")]),
    ef(0x00120000, 0x001f0000, &[Op::Name("ie2")], F_FUC4P),
    e(0x00140000, 0x001f0000, &[Op::Name("is0")]),
    e(0x00150000, 0x001f0000, &[Op::Name("is1")]),
    ef(0x00160000, 0x001f0000, &[Op::Name("is2")], F_FUC4P),
    e(0x00180000, 0x001f0000, &[Op::Name("ta")]),
    e(0, 0, &[OOPS]),
];

/// F()-generated two-way tables (falcon.c:247-253).
pub static TABI: &[InsnEntry] = &[
    e(0, 1, &[Op::RImm(&IMM8OFF)]),
    e(1, 1, &[Op::RImm(&IMM16OFF)]),
    e(0, 0, &[OOPS]),
];
pub static TABIS: &[InsnEntry] = &[
    e(0, 1, &[Op::RImm(&IMM8SOFF)]),
    e(1, 1, &[Op::RImm(&IMM16SOFF)]),
    e(0, 0, &[OOPS]),
];
pub static TABIH: &[InsnEntry] = &[
    e(0, 1, &[Op::RImm(&IMM8HOFF)]),
    e(1, 1, &[Op::RImm(&IMM16HOFF)]),
    e(0, 0, &[OOPS]),
];
pub static TABBT: &[InsnEntry] = &[
    e(0, 1, &[Op::BTarget(&PCREL8OFF)]),
    e(1, 1, &[Op::BTarget(&PCREL16OFF)]),
    e(0, 0, &[OOPS]),
];
pub static TABABT: &[InsnEntry] = &[
    e(0, 1, &[Op::BTarget(&IMM8OFF)]),
    e(1, 1, &[Op::BTarget(&IMM16WOFF)]),
    e(0, 0, &[OOPS]),
];
pub static TABCT: &[InsnEntry] = &[
    e(0, 1, &[Op::CTarget(&IMM8OFF)]),
    e(1, 1, &[Op::CTarget(&IMM16WOFF)]),
    e(0, 0, &[OOPS]),
];
pub static TABOL0: &[InsnEntry] = &[
    e(0, 1, &[Op::Oplen(3)]),
    e(1, 1, &[Op::Oplen(4)]),
    e(0, 0, &[OOPS]),
];

/// tabcocmd (falcon.c:255-310) — crypt coprocessor direct ops.
pub static TABCOCMD: &[InsnEntry] = &[
    e(0x00000000, 0x80000000, &[Op::Name("cxset"), Op::RImm(&IMM8OFF)]),
    e(0x84000000, 0xfc000000, &[Op::Name("cmov"), Op::Reg(&CREG1_R), Op::Reg(&CREG2_R)]),
    e(0x88000000, 0xfc000000, &[Op::Name("cxsin"), Op::Reg(&CREG1_R)]),
    e(0x8c000000, 0xfc000000, &[Op::Name("cxsout"), Op::Reg(&CREG1_R)]),
    e(0x90000000, 0xfc000000, &[Op::Name("crnd"), Op::Reg(&CREG1_R)]),
    e(0x94000000, 0xfc000000, &[Op::Name("cs0begin"), Op::Imm(&CIMM2OFF)]),
    e(0x98000000, 0xfc000000, &[Op::Name("cs0exec"), Op::Imm(&CIMM2OFF)]),
    e(0x9c000000, 0xfc000000, &[Op::Name("cs1begin"), Op::Imm(&CIMM2OFF)]),
    e(0xa0000000, 0xfc000000, &[Op::Name("cs1exec"), Op::Imm(&CIMM2OFF)]),
    e(0xa8000000, 0xfc000000, &[Op::Name("cchmod"), Op::Reg(&CREG1_R), Op::Imm(&CIMM2OFF)]),
    e(0xac000000, 0xfc000000, &[Op::Name("cxor"), Op::Reg(&CREG1_R), Op::Reg(&CREG2_R)]),
    e(0xb0000000, 0xfc000000, &[Op::Name("cadd"), Op::Reg(&CREG1_R), Op::Imm(&CIMM2OFF)]),
    e(0xb4000000, 0xfc000000, &[Op::Name("cand"), Op::Reg(&CREG1_R), Op::Reg(&CREG2_R)]),
    e(0xb8000000, 0xfc000000, &[Op::Name("crev"), Op::Reg(&CREG1_R), Op::Reg(&CREG2_R)]),
    e(0xbc000000, 0xfc000000, &[Op::Name("cgfmul"), Op::Reg(&CREG1_R), Op::Reg(&CREG2_R)]),
    e(0xc0000000, 0xfc000000, &[Op::Name("csecret"), Op::Reg(&CREG1_R), Op::Imm(&CIMM2OFF)]),
    e(0xc4000000, 0xfc000000, &[Op::Name("ckeyreg"), Op::Reg(&CREG1_R)]),
    e(0xc8000000, 0xfc000000, &[Op::Name("ckexp"), Op::Reg(&CREG1_R), Op::Reg(&CREG2_R)]),
    e(0xcc000000, 0xfc000000, &[Op::Name("ckrexp"), Op::Reg(&CREG1_R), Op::Reg(&CREG2_R)]),
    e(0xd0000000, 0xfc000000, &[Op::Name("cenc"), Op::Reg(&CREG1_R), Op::Reg(&CREG2_R)]),
    e(0xd4000000, 0xfc000000, &[Op::Name("cdec"), Op::Reg(&CREG1_R), Op::Reg(&CREG2_R)]),
    e(0xd8000000, 0xfc000000, &[Op::Name("csigcmp"), Op::Reg(&CREG1_R), Op::Reg(&CREG2_R)]),
    e(0xdc000000, 0xfc000000, &[Op::Name("csigenc"), Op::Reg(&CREG1_R), Op::Reg(&CREG2_R)]),
    e(0xe0000000, 0xfc000000, &[Op::Name("csigclr")]),
    e(0, 0, &[OOPS]),
];

/// tabsz + the five data addressing-mode selector tables (falcon.c:312-352).
pub static TABSZ: &[InsnEntry] = &[
    e(0x00000000, 0x000000c0, &[Op::Name("b8")]),
    e(0x00000040, 0x000000c0, &[Op::Name("b16")]),
    e(0x00000080, 0x000000c0, &[Op::Name("b32")]),
    e(0, 0, &[OOPS]),
];
pub static TABDATARI: &[InsnEntry] = &[
    e(0x00000000, 0x000000c0, &[Op::Mem(&DATARI8_M)]),
    e(0x00000040, 0x000000c0, &[Op::Mem(&DATARI16_M)]),
    e(0x00000080, 0x000000c0, &[Op::Mem(&DATARI32_M)]),
    e(0, 0, &[OOPS]),
];
pub static TABDATASP: &[InsnEntry] = &[
    e(0x00000000, 0x000000c0, &[Op::Mem(&DATASP8_M)]),
    e(0x00000040, 0x000000c0, &[Op::Mem(&DATASP16_M)]),
    e(0x00000080, 0x000000c0, &[Op::Mem(&DATASP32_M)]),
    e(0, 0, &[OOPS]),
];
pub static TABDATARR: &[InsnEntry] = &[
    e(0x00000000, 0x000000c0, &[Op::Mem(&DATARR8_M)]),
    e(0x00000040, 0x000000c0, &[Op::Mem(&DATARR16_M)]),
    e(0x00000080, 0x000000c0, &[Op::Mem(&DATARR32_M)]),
    e(0, 0, &[OOPS]),
];
pub static TABDATARRALT: &[InsnEntry] = &[
    e(0x00000000, 0x000000c0, &[Op::Mem(&DATARRALT8_M)]),
    e(0x00000040, 0x000000c0, &[Op::Mem(&DATARRALT16_M)]),
    e(0x00000080, 0x000000c0, &[Op::Mem(&DATARRALT32_M)]),
    e(0, 0, &[OOPS]),
];
pub static TABDATASPR: &[InsnEntry] = &[
    e(0x00000000, 0x000000c0, &[Op::Mem(&DATASPR8_M)]),
    e(0x00000040, 0x000000c0, &[Op::Mem(&DATASPR16_M)]),
    e(0x00000080, 0x000000c0, &[Op::Mem(&DATASPR32_M)]),
    e(0, 0, &[OOPS]),
];

/// tabsi (falcon.c:354-485) — reached for op bytes 0x00-0xbf.
pub static TABSI: &[InsnEntry] = &[
    ef(0x00000000, 0x0000003f, &[Op::Oplen(3), Op::Name("st"), Op::Tab(TABSZ), Op::Tab(TABDATARI), Op::Reg(&REG2_R)], F_FUCOLD),
    ef(0x00000000, 0x00000030, &[Op::Oplen(3), OOPS, Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R), Op::RImm(&IMM8OFF)], F_FUCOLD),

    ef(0x00000000, 0x000000f0, &[Op::Oplen(2), Op::Name("mov"), Op::Reg(&REG0_R), Op::RImm(&FIMM8SOFF)], F_FUC5P),
    ef(0x00000040, 0x000000f0, &[Op::Oplen(3), Op::Name("mov"), Op::Reg(&REG0_R), Op::RImm(&FIMM16SOFF)], F_FUC5P),
    ef(0x00000080, 0x000000f0, &[Op::Oplen(4), Op::Name("mov"), Op::Reg(&REG0_R), Op::RImm(&FIMM24SOFF)], F_FUC5P),

    e(0x00000010, 0x0000003f, &[Op::Oplen(3), Op::Name("add"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),
    e(0x00000011, 0x0000003f, &[Op::Oplen(3), Op::Name("adc"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),
    e(0x00000012, 0x0000003f, &[Op::Oplen(3), Op::Name("sub"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),
    e(0x00000013, 0x0000003f, &[Op::Oplen(3), Op::Name("sbb"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),
    e(0x00000014, 0x0000003f, &[Op::Oplen(3), Op::Name("shl"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),
    e(0x00000015, 0x0000003f, &[Op::Oplen(3), Op::Name("shr"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),
    e(0x00000017, 0x0000003f, &[Op::Oplen(3), Op::Name("sar"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),
    e(0x00000018, 0x0000003f, &[Op::Oplen(3), Op::Name("ld"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Tab(TABDATARI)]),
    e(0x0000001c, 0x0000003f, &[Op::Oplen(3), Op::Name("shlc"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),
    e(0x0000001d, 0x0000003f, &[Op::Oplen(3), Op::Name("shrc"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),
    e(0x00000010, 0x00000030, &[Op::Oplen(3), OOPS, Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R)]),

    ef(0x00000020, 0x0000003f, &[Op::Oplen(4), Op::Name("add"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)], F_FUCOLD),
    ef(0x00000021, 0x0000003f, &[Op::Oplen(4), Op::Name("adc"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)], F_FUCOLD),
    ef(0x00000022, 0x0000003f, &[Op::Oplen(4), Op::Name("sub"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)], F_FUCOLD),
    ef(0x00000023, 0x0000003f, &[Op::Oplen(4), Op::Name("sbb"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)], F_FUCOLD),
    ef(0x00000020, 0x00000030, &[Op::Oplen(4), OOPS, Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)], F_FUCOLD),

    ef(0x00000020, 0x0000003f, &[Op::Oplen(2), Op::Name("st"), Op::Tab(TABSZ), Op::Mem(&DATAR_M), Op::Reg(&REG2_R)], F_FUC5P),
    ef(0x00000021, 0x0000003f, &[Op::Oplen(2), Op::Name("st"), Op::Tab(TABSZ), Op::Tab(TABDATASPR), Op::Reg(&REG1_R)], F_FUC5P),
    ef(0x00000024, 0x0000003f, &[Op::Oplen(2), Op::Name("cmpu"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)], F_FUC5P),
    ef(0x00000025, 0x0000003f, &[Op::Oplen(2), Op::Name("cmps"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)], F_FUC5P),
    ef(0x00000026, 0x0000003f, &[Op::Oplen(2), Op::Name("cmp"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)], F_FUC5P),
    ef(0x00000020, 0x00000030, &[Op::Oplen(2), OOPS, Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)], F_FUC5P),

    e(0x00000130, 0x00000f3f, &[Op::Tab(TABOL0), Op::Name("st"), Op::Tab(TABSZ), Op::Tab(TABDATASP), Op::Reg(&REG1_R)]),
    e(0x00000430, 0x00000f3e, &[Op::Tab(TABOL0), Op::Name("cmpu"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x00000530, 0x00000f3e, &[Op::Tab(TABOL0), Op::Name("cmps"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Tab(TABIS)]),
    ef(0x00000630, 0x00000f3e, &[Op::Tab(TABOL0), Op::Name("cmp"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Tab(TABI)], F_FUC3P),
    e(0x00000030, 0x0000003e, &[Op::Tab(TABOL0), OOPS, Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Tab(TABI)]),

    ef(0x00000032, 0x0000003f, &[Op::Oplen(2), Op::Name("mov"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R)], F_FUC5P),

    ef(0x00000033, 0x00000f3f, &[Op::Oplen(4), Op::Name("bra"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF), Op::Name("e"), Op::BTarget(&FPCREL8OFF)], F_FUC5P),
    ef(0x00000433, 0x00000f3f, &[Op::Oplen(4), Op::Name("bra"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF), Op::Name("ne"), Op::BTarget(&FPCREL8OFF)], F_FUC5P),
    ef(0x00000933, 0x00000f3f, &[Op::Oplen(5), Op::Name("bra"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF), Op::Name("e"), Op::BTarget(&FPCREL16OFF)], F_FUC5P),
    ef(0x00000a33, 0x00000f3f, &[Op::Oplen(5), Op::Name("bra"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF), Op::Name("e"), Op::BTarget(&FFPCREL8OFF)], F_FUC5P),
    ef(0x00000b33, 0x00000f3f, &[Op::Oplen(6), Op::Name("bra"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF), Op::Name("e"), Op::BTarget(&FFPCREL16OFF)], F_FUC5P),
    ef(0x00000d33, 0x00000f3f, &[Op::Oplen(5), Op::Name("bra"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF), Op::Name("ne"), Op::BTarget(&FPCREL16OFF)], F_FUC5P),
    ef(0x00000e33, 0x00000f3f, &[Op::Oplen(5), Op::Name("bra"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF), Op::Name("ne"), Op::BTarget(&FFPCREL8OFF)], F_FUC5P),
    ef(0x00000f33, 0x00000f3f, &[Op::Oplen(6), Op::Name("bra"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF), Op::Name("ne"), Op::BTarget(&FFPCREL16OFF)], F_FUC5P),

    e(0x00000034, 0x00000f3f, &[Op::Oplen(3), Op::Name("ld"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Tab(TABDATASP)]),
    e(0x00000034, 0x0000003f, &[Op::Oplen(3), OOPS, Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),

    ef(0x00000035, 0x0000003f, &[Op::Oplen(3), Op::Name("st"), Op::Tab(TABSZ), Op::Tab(TABDATARI), Op::Reg(&REG2_R)], F_FUC5P),

    e(0x00000036, 0x00000f3e, &[Op::Tab(TABOL0), Op::Name("add"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x00000136, 0x00000f3e, &[Op::Tab(TABOL0), Op::Name("adc"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x00000236, 0x00000f3e, &[Op::Tab(TABOL0), Op::Name("sub"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x00000336, 0x00000f3e, &[Op::Tab(TABOL0), Op::Name("sbb"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x00000436, 0x00000f3f, &[Op::Tab(TABOL0), Op::Name("shl"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x00000536, 0x00000f3f, &[Op::Tab(TABOL0), Op::Name("shr"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x00000736, 0x00000f3f, &[Op::Tab(TABOL0), Op::Name("sar"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x00000c36, 0x00000f3f, &[Op::Tab(TABOL0), Op::Name("shlc"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x00000d36, 0x00000f3f, &[Op::Tab(TABOL0), Op::Name("shrc"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x00000036, 0x0000003e, &[Op::Tab(TABOL0), OOPS, Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Tab(TABI)]),

    ef(0x00000038, 0x000f003f, &[Op::Oplen(3), Op::Name("st"), Op::Tab(TABSZ), Op::Mem(&DATAR_M), Op::Reg(&REG2_R)], F_FUCOLD),
    ef(0x00010038, 0x000f003f, &[Op::Oplen(3), Op::Name("st"), Op::Tab(TABSZ), Op::Tab(TABDATASPR), Op::Reg(&REG1_R)], F_FUCOLD),
    ef(0x00040038, 0x000f003f, &[Op::Oplen(3), Op::Name("cmpu"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)], F_FUCOLD),
    ef(0x00050038, 0x000f003f, &[Op::Oplen(3), Op::Name("cmps"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)], F_FUCOLD),
    ef(0x00060038, 0x000f003f, &[Op::Oplen(3), Op::Name("cmp"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)], F_FUC3P | F_FUCOLD),
    ef(0x00000038, 0x0000003f, &[Op::Oplen(3), OOPS, Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)], F_FUCOLD),

    ef(0x0000000038, 0x0f0000003f, &[Op::Oplen(5), Op::Name("add"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)], F_FUC5P),
    ef(0x0100000038, 0x0f0000003f, &[Op::Oplen(5), Op::Name("adc"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)], F_FUC5P),
    ef(0x0200000038, 0x0f0000003f, &[Op::Oplen(5), Op::Name("sub"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)], F_FUC5P),
    ef(0x0300000038, 0x0f0000003f, &[Op::Oplen(5), Op::Name("sbb"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)], F_FUC5P),
    ef(0x00000038, 0x0000003f, &[Op::Oplen(5), OOPS, Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)], F_FUC5P),

    e(0x00000039, 0x000f003f, &[Op::Oplen(3), Op::Name("not"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R)]),
    e(0x00010039, 0x000f003f, &[Op::Oplen(3), Op::Name("neg"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R)]),
    ef(0x00020039, 0x000f003f, &[Op::Oplen(3), Op::Name("movf"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R)], F_FUC0),
    ef(0x00020039, 0x000f003f, &[Op::Oplen(3), Op::Name("mov"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R)], F_FUC3P | F_FUCOLD),
    e(0x00030039, 0x000f003f, &[Op::Oplen(3), Op::Name("hswap"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R)]),
    e(0x00000039, 0x0000003f, &[Op::Oplen(3), OOPS, Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Reg(&REG1_R)]),

    e(0x0000003a, 0x000f003f, &[Op::Oplen(3), Op::Name("ld"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Tab(TABDATASPR)]),
    e(0x0000003a, 0x0000003f, &[Op::Oplen(3), OOPS, Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),

    e(0x0000003b, 0x000f003f, &[Op::Oplen(3), Op::Name("add"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x0001003b, 0x000f003f, &[Op::Oplen(3), Op::Name("adc"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x0002003b, 0x000f003f, &[Op::Oplen(3), Op::Name("sub"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x0003003b, 0x000f003f, &[Op::Oplen(3), Op::Name("sbb"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x0004003b, 0x000f003f, &[Op::Oplen(3), Op::Name("shl"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x0005003b, 0x000f003f, &[Op::Oplen(3), Op::Name("shr"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x0007003b, 0x000f003f, &[Op::Oplen(3), Op::Name("sar"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000c003b, 0x000f003f, &[Op::Oplen(3), Op::Name("shlc"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000d003b, 0x000f003f, &[Op::Oplen(3), Op::Name("shrc"), Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x0000003b, 0x0000003f, &[Op::Oplen(3), OOPS, Op::Tab(TABSZ), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),

    e(0x0000003c, 0x000f003f, &[Op::Oplen(3), Op::Name("add"), Op::Tab(TABSZ), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x0001003c, 0x000f003f, &[Op::Oplen(3), Op::Name("adc"), Op::Tab(TABSZ), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x0002003c, 0x000f003f, &[Op::Oplen(3), Op::Name("sub"), Op::Tab(TABSZ), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x0003003c, 0x000f003f, &[Op::Oplen(3), Op::Name("sbb"), Op::Tab(TABSZ), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x0004003c, 0x000f003f, &[Op::Oplen(3), Op::Name("shl"), Op::Tab(TABSZ), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x0005003c, 0x000f003f, &[Op::Oplen(3), Op::Name("shr"), Op::Tab(TABSZ), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x0007003c, 0x000f003f, &[Op::Oplen(3), Op::Name("sar"), Op::Tab(TABSZ), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x0008003c, 0x000f003f, &[Op::Oplen(3), Op::Name("ld"), Op::Tab(TABSZ), Op::Reg(&REG3_R), Op::Tab(TABDATARR)]),
    ef(0x0009003c, 0x000f003f, &[Op::Oplen(3), Op::Name("st"), Op::Tab(TABSZ), Op::Tab(TABDATARRALT), Op::Reg(&REG2_R)], F_FUC5P),
    e(0x000c003c, 0x000f003f, &[Op::Oplen(3), Op::Name("shlc"), Op::Tab(TABSZ), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000d003c, 0x000f003f, &[Op::Oplen(3), Op::Name("shrc"), Op::Tab(TABSZ), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x0000003c, 0x0000003f, &[Op::Oplen(3), OOPS, Op::Tab(TABSZ), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),

    e(0x0000003d, 0x00000f3f, &[Op::Oplen(2), Op::Name("not"), Op::Tab(TABSZ), Op::Reg(&REG1_R)]),
    e(0x0000013d, 0x00000f3f, &[Op::Oplen(2), Op::Name("neg"), Op::Tab(TABSZ), Op::Reg(&REG1_R)]),
    ef(0x0000023d, 0x00000f3f, &[Op::Oplen(2), Op::Name("movf"), Op::Tab(TABSZ), Op::Reg(&REG1_R)], F_FUC0),
    ef(0x0000023d, 0x00000f3f, &[Op::Oplen(2), Op::Name("mov"), Op::Tab(TABSZ), Op::Reg(&REG1_R)], F_FUC3P),
    e(0x0000033d, 0x00000f3f, &[Op::Oplen(2), Op::Name("hswap"), Op::Tab(TABSZ), Op::Reg(&REG1_R)]),
    e(0x0000043d, 0x00000f3f, &[Op::Oplen(2), Op::Name("clear"), Op::Tab(TABSZ), Op::Reg(&REG1_R)]),
    ef(0x0000053d, 0x00000f3f, &[Op::Oplen(2), Op::Name("setf"), Op::Tab(TABSZ), Op::Reg(&REG1_R)], F_FUC3P),
    e(0x0000003d, 0x0000003f, &[Op::Oplen(2), OOPS, Op::Tab(TABSZ), Op::Reg(&REG1_R)]),

    ef(0x0000003e, 0x000000ff, &[Op::Oplen(4), Op::Name("lbra"), Op::BTarget(&FIMM24OFF)], F_FUC4P),
    ef(0x0000007e, 0x000000ff, &[Op::Oplen(4), Op::Name("lcall"), Op::CTarget(&FIMM24OFF)], F_FUC4P),
    ef(0x000000be, 0x000000ff, &[Op::Oplen(4), OOPS, Op::RImm(&FIMM24OFF)], F_FUC4P),

    ef(0x0000003f, 0x0000003f, &[Op::Oplen(2), Op::Name("ld"), Op::Tab(TABSZ), Op::Reg(&REG2_R), Op::Mem(&DATAR_M)], F_FUC5P),

    e(0, 0, &[OOPS, Op::Tab(TABSZ)]),
];

/// tabm (falcon.c:487-677) — the root table.
pub static TABM: &[InsnEntry] = &[
    e(0x00000000, 0x000000c0, &[Op::Tab(TABSI)]),
    e(0x00000040, 0x000000c0, &[Op::Tab(TABSI)]),
    e(0x00000080, 0x000000c0, &[Op::Tab(TABSI)]),

    e(0x000000c0, 0x000000ff, &[Op::Oplen(3), Op::Name("mulu"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),
    e(0x000000c1, 0x000000ff, &[Op::Oplen(3), Op::Name("muls"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8SOFF)]),
    e(0x000000c2, 0x000000ff, &[Op::Oplen(3), Op::Name("sext"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),
    ef(0x000000c3, 0x000000ff, &[Op::Oplen(3), Op::Name("extrs"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::Bf(&BITF8BF)], F_FUC3P),
    e(0x000000c4, 0x000000ff, &[Op::Oplen(3), Op::Name("and"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),
    e(0x000000c5, 0x000000ff, &[Op::Oplen(3), Op::Name("or"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),
    e(0x000000c6, 0x000000ff, &[Op::Oplen(3), Op::Name("xor"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),
    ef(0x000000c7, 0x000000ff, &[Op::Oplen(3), Op::Name("extr"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::Bf(&BITF8BF)], F_FUC3P),
    e(0x000000c8, 0x000000ff, &[Op::Oplen(3), Op::Name("xbit"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),
    ef(0x000000cb, 0x000000ff, &[Op::Oplen(3), Op::Name("ins"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::Bf(&BITF8BF)], F_FUC3P),
    ef(0x000000cc, 0x000000ff, &[Op::Oplen(3), Op::Name("div"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)], F_FUC3P),
    ef(0x000000cd, 0x000000ff, &[Op::Oplen(3), Op::Name("mod"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)], F_FUC3P),
    e(0x000000ce, 0x000000ff, &[Op::Oplen(3), Op::Name("iords"), Op::Reg(&REG2_R), Op::Mem(&IORI_M)]),
    e(0x000000cf, 0x000000ff, &[Op::Oplen(3), Op::Name("iord"), Op::Reg(&REG2_R), Op::Mem(&IORI_M)]),
    e(0x000000c0, 0x000000f0, &[Op::Oplen(3), OOPS, Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),

    ef(0x000000d0, 0x000000ff, &[Op::Oplen(3), Op::Name("iowr"), Op::Mem(&IORI_M), Op::Reg(&REG2_R)], F_FUCOLD),
    ef(0x000000d1, 0x000000ff, &[Op::Oplen(3), Op::Name("iowrs"), Op::Mem(&IORI_M), Op::Reg(&REG2_R)], F_FUC3P | F_FUCOLD),
    ef(0x000000d0, 0x000000f0, &[Op::Oplen(3), OOPS, Op::Reg(&REG1_R), Op::Reg(&REG2_R), Op::RImm(&IMM8OFF)], F_FUCOLD),

    ef(0x000000d0, 0x000000f0, &[Op::Oplen(5), Op::Name("mov"), Op::Reg(&REG0_R), Op::RImm(&FIMM32OFF)], F_FUC5P),

    e(0x000000e0, 0x000000ff, &[Op::Oplen(4), Op::Name("mulu"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)]),
    e(0x000000e1, 0x000000ff, &[Op::Oplen(4), Op::Name("muls"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16SOFF)]),
    ef(0x000000e3, 0x000000ff, &[Op::Oplen(4), Op::Name("extrs"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::Bf(&BITF16BF)], F_FUC3P),
    e(0x000000e4, 0x000000ff, &[Op::Oplen(4), Op::Name("and"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)]),
    e(0x000000e5, 0x000000ff, &[Op::Oplen(4), Op::Name("or"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)]),
    e(0x000000e6, 0x000000ff, &[Op::Oplen(4), Op::Name("xor"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)]),
    ef(0x000000e7, 0x000000ff, &[Op::Oplen(4), Op::Name("extr"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::Bf(&BITF16BF)], F_FUC3P),
    ef(0x000000eb, 0x000000ff, &[Op::Oplen(4), Op::Name("ins"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::Bf(&BITF16BF)], F_FUC3P),
    ef(0x000000ec, 0x000000ff, &[Op::Oplen(4), Op::Name("div"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)], F_FUC3P),
    ef(0x000000ed, 0x000000ff, &[Op::Oplen(4), Op::Name("mod"), Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)], F_FUC3P),
    e(0x000000e0, 0x000000f0, &[Op::Oplen(4), OOPS, Op::Reg(&REG2_R), Op::Reg(&REG1_R), Op::RImm(&IMM16OFF)]),

    e(0x000000f0, 0x00000ffe, &[Op::Tab(TABOL0), Op::Name("mulu"), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x000001f0, 0x00000ffe, &[Op::Tab(TABOL0), Op::Name("muls"), Op::Reg(&REG1_R), Op::Tab(TABIS)]),
    e(0x000002f0, 0x00000fff, &[Op::Tab(TABOL0), Op::Name("sext"), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x000003f0, 0x00000ffe, &[Op::Tab(TABOL0), Op::Name("sethi"), Op::Reg(&REG1_R), Op::Tab(TABIH)]),
    e(0x000004f0, 0x00000ffe, &[Op::Tab(TABOL0), Op::Name("and"), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x000005f0, 0x00000ffe, &[Op::Tab(TABOL0), Op::Name("or"), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x000006f0, 0x00000ffe, &[Op::Tab(TABOL0), Op::Name("xor"), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    ef(0x000007f0, 0x00000ffe, &[Op::Tab(TABOL0), Op::Name("mov"), Op::Reg(&REG1_R), Op::Tab(TABIS)], F_FUCOLD),
    ef(0x000007f1, 0x00000fff, &[Op::Tab(TABOL0), Op::Name("movw"), Op::Reg(&REG1_R), Op::RImm(&IMM16WOFF)], F_FUCOLD),
    e(0x000009f0, 0x00000fff, &[Op::Tab(TABOL0), Op::Name("bset"), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x00000af0, 0x00000fff, &[Op::Tab(TABOL0), Op::Name("bclr"), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x00000bf0, 0x00000fff, &[Op::Tab(TABOL0), Op::Name("btgl"), Op::Reg(&REG1_R), Op::Tab(TABI)]),
    e(0x00000cf0, 0x00000fff, &[Op::Tab(TABOL0), Op::Name("xbit"), Op::Reg(&REG1_R), Op::Reg(&FLAGS_R), Op::Tab(TABFL)]),
    e(0x000000f0, 0x000000fe, &[Op::Tab(TABOL0), OOPS, Op::Reg(&REG1_R), Op::Tab(TABI)]),

    e(0x000008f2, 0x00000fff, &[Op::Oplen(3), Op::Name("setp"), Op::Tab(TABFL), Op::Reg(&REG1_R)]),
    ef(0x00010cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cimov"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00020cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cixsin"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00030cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cixsout"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00040cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cirnd"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00050cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cis0begin"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00060cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cis0exec"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00070cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cis1begin"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00080cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cis1exec"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x000a0cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cichmod"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x000b0cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cixor"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x000c0cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("ciadd"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x000d0cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("ciand"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x000e0cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cirev"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x000f0cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cigfmul"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00100cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cisecret"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00110cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cikeyreg"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00120cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cikexp"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00130cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cikrexp"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00140cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cienc"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00150cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cidec"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00160cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cisigcmp"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00170cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cisigenc"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00180cf2, 0x001f0fff, &[Op::Oplen(3), Op::Name("cisigclr"), Op::Reg(&REG1_R)], F_CRYPT),
    ef(0x00000cf2, 0x00000fff, &[Op::Oplen(3), OOPS, Op::Reg(&REG1_R)], F_CRYPT),
    e(0x000000f2, 0x000000ff, &[Op::Oplen(3), OOPS, Op::Reg(&REG1_R), Op::RImm(&IMM8OFF)]),

    ef(0x000000f3, 0x000000ff, &[Op::Oplen(3), Op::Name("call"), Op::CTarget(&FIMM16OFF)], F_FUC5P),

    e(0x000000f4, 0x000020fe, &[Op::Tab(TABOL0), Op::Name("bra"), Op::Tab(TABP), Op::Tab(TABBT)]),
    e(0x000020f4, 0x00003ffe, &[Op::Tab(TABOL0), Op::Name("bra"), Op::Tab(TABABT), Op::Endmark]),
    e(0x000021f4, 0x00003fff, &[Op::Tab(TABOL0), Op::Name("call"), Op::Tab(TABCT)]),
    ef(0x000021f5, 0x00003fff, &[Op::Tab(TABOL0), Op::Name("call"), Op::Tab(TABCT)], F_FUCOLD),
    e(0x000028f4, 0x00003fff, &[Op::Tab(TABOL0), Op::Name("sleep"), Op::Tab(TABFL)]),
    e(0x000030f4, 0x00003ffe, &[Op::Tab(TABOL0), Op::Name("add"), Op::Reg(&SP_R), Op::Tab(TABIS)]),
    e(0x000031f4, 0x00003fff, &[Op::Tab(TABOL0), Op::Name("bset"), Op::Reg(&FLAGS_R), Op::Tab(TABFL)]),
    e(0x000032f4, 0x00003fff, &[Op::Tab(TABOL0), Op::Name("bclr"), Op::Reg(&FLAGS_R), Op::Tab(TABFL)]),
    e(0x000033f4, 0x00003fff, &[Op::Tab(TABOL0), Op::Name("btgl"), Op::Reg(&FLAGS_R), Op::Tab(TABFL)]),
    ef(0x00003cf4, 0x00003fff, &[Op::Tab(TABOL0), Op::Name("cxset"), Op::RImm(&IMM8OFF)], F_CRYPT),
    ef(0x00003cf5, 0x00003fff, &[Op::Tab(TABOL0), Op::Tab(TABCOCMD)], F_CRYPT),
    e(0x000000f4, 0x000000fe, &[Op::Tab(TABOL0), OOPS, Op::Tab(TABI)]),

    ef(0x000000f6, 0x000000ff, &[Op::Oplen(3), Op::Name("iowr"), Op::Mem(&IORI_M), Op::Reg(&REG2_R)], F_FUC5P),
    ef(0x000000f7, 0x000000ff, &[Op::Oplen(3), Op::Name("iowrs"), Op::Mem(&IORI_M), Op::Reg(&REG2_R)], F_FUC5P),

    e(0x000000f8, 0x00000fff, &[Op::Oplen(2), Op::Name("ret"), Op::Endmark]),
    e(0x000001f8, 0x00000fff, &[Op::Oplen(2), Op::Name("iret"), Op::Endmark]),
    e(0x000002f8, 0x00000fff, &[Op::Oplen(2), Op::Name("exit"), Op::Endmark]),
    e(0x000003f8, 0x00000fff, &[Op::Oplen(2), Op::Name("xdwait")]),
    e(0x000006f8, 0x00000fff, &[Op::Oplen(2), Op::Name("xdfence")]),
    e(0x000007f8, 0x00000fff, &[Op::Oplen(2), Op::Name("xcwait")]),
    e(0x000008f8, 0x00000cff, &[Op::Oplen(2), Op::Name("trap"), Op::Imm(&STRAPOFF)]),
    e(0x000000f8, 0x000000ff, &[Op::Oplen(2), OOPS]),

    e(0x000000f9, 0x00000fff, &[Op::Oplen(2), Op::Name("push"), Op::Reg(&REG1_R)]),
    e(0x000001f9, 0x00000fff, &[Op::Oplen(2), Op::Name("add"), Op::Reg(&SP_R), Op::Reg(&REG1_R)]),
    ef(0x000002f9, 0x00000fff, &[Op::Oplen(2), Op::Name("mpush"), Op::Reg(&REG1_R)], F_FUC5P),
    e(0x000004f9, 0x00000fff, &[Op::Oplen(2), Op::Name("bra"), Op::Reg(&REG1_R)]),
    e(0x000005f9, 0x00000fff, &[Op::Oplen(2), Op::Name("call"), Op::Reg(&REG1_R)]),
    ef(0x000008f9, 0x00000fff, &[Op::Oplen(2), Op::Name("itlb"), Op::Reg(&REG1_R)], F_FUC3P),
    e(0x000009f9, 0x00000fff, &[Op::Oplen(2), Op::Name("bset"), Op::Reg(&FLAGS_R), Op::Reg(&REG1_R)]),
    e(0x00000af9, 0x00000fff, &[Op::Oplen(2), Op::Name("bclr"), Op::Reg(&FLAGS_R), Op::Reg(&REG1_R)]),
    e(0x00000bf9, 0x00000fff, &[Op::Oplen(2), Op::Name("btgl"), Op::Reg(&FLAGS_R), Op::Reg(&REG1_R)]),
    e(0x000000f9, 0x000000ff, &[Op::Oplen(2), OOPS, Op::Reg(&REG1_R)]),

    e(0x000000fa, 0x000f00ff, &[Op::Oplen(3), Op::Name("iowr"), Op::Mem(&IOR_M), Op::Reg(&REG2_R)]),
    ef(0x000100fa, 0x000f00ff, &[Op::Oplen(3), Op::Name("iowrs"), Op::Mem(&IOR_M), Op::Reg(&REG2_R)], F_FUC3P),
    e(0x000400fa, 0x000f00ff, &[Op::Oplen(3), Op::Name("xcld"), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000500fa, 0x000f00ff, &[Op::Oplen(3), Op::Name("xdld"), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000600fa, 0x000f00ff, &[Op::Oplen(3), Op::Name("xdst"), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    ef(0x000700fa, 0x000f00ff, &[Op::Oplen(3), Op::Unk("unkfa/7"), Op::Reg(&REG2_R), Op::Reg(&REG1_R)], F_FUC6P),
    e(0x000800fa, 0x000f00ff, &[Op::Oplen(3), Op::Name("setp"), Op::Reg(&REG2_R), Op::Reg(&REG1_R)]),
    e(0x000000fa, 0x000000ff, &[Op::Oplen(3), OOPS, Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),

    ef(0x000000fb, 0x000007ff, &[Op::Oplen(2), Op::Name("mpop"), Op::Reg(&REG1_R)], F_FUC5P),
    ef(0x000001fb, 0x000007ff, &[Op::Oplen(2), Op::Name("mpopret"), Op::Reg(&REG1_R), Op::Endmark], F_FUC5P),
    ef(0x000002fb, 0x000007ff, &[Op::Oplen(4), Op::Name("mpopadd"), Op::Reg(&REG1_R), Op::RImm(&IMM16SOFF)], F_FUC5P),
    ef(0x000003fb, 0x000007ff, &[Op::Oplen(4), Op::Name("mpopaddret"), Op::Reg(&REG1_R), Op::RImm(&IMM16SOFF), Op::Endmark], F_FUC5P),
    ef(0x000004fb, 0x000007ff, &[Op::Oplen(3), Op::Name("mpopadd"), Op::Reg(&REG1_R), Op::RImm(&IMM8SOFF)], F_FUC5P),
    ef(0x000005fb, 0x000007ff, &[Op::Oplen(3), Op::Name("mpopaddret"), Op::Reg(&REG1_R), Op::RImm(&IMM8SOFF), Op::Endmark], F_FUC5P),

    e(0x000000fc, 0x00000fff, &[Op::Oplen(2), Op::Name("pop"), Op::Reg(&REG1_R)]),
    e(0x000000fc, 0x000000ff, &[Op::Oplen(2), OOPS, Op::Reg(&REG1_R)]),

    e(0x000000fd, 0x000f00ff, &[Op::Oplen(3), Op::Name("mulu"), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000100fd, 0x000f00ff, &[Op::Oplen(3), Op::Name("muls"), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000200fd, 0x000f00ff, &[Op::Oplen(3), Op::Name("sext"), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000400fd, 0x000f00ff, &[Op::Oplen(3), Op::Name("and"), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000500fd, 0x000f00ff, &[Op::Oplen(3), Op::Name("or"), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000600fd, 0x000f00ff, &[Op::Oplen(3), Op::Name("xor"), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000900fd, 0x000f00ff, &[Op::Oplen(3), Op::Name("bset"), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000a00fd, 0x000f00ff, &[Op::Oplen(3), Op::Name("bclr"), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000b00fd, 0x000f00ff, &[Op::Oplen(3), Op::Name("btgl"), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000000fd, 0x000000ff, &[Op::Oplen(3), OOPS, Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),

    e(0x000000fe, 0x000f00ff, &[Op::Oplen(3), Op::Name("mov"), Op::Reg(&SREG2_R), Op::Reg(&REG1_R)]),
    e(0x000100fe, 0x000f00ff, &[Op::Oplen(3), Op::Name("mov"), Op::Reg(&REG2_R), Op::Reg(&SREG1_R)]),
    ef(0x000200fe, 0x000f00ff, &[Op::Oplen(3), Op::Name("ptlb"), Op::Reg(&REG2_R), Op::Reg(&REG1_R)], F_FUC3P),
    ef(0x000300fe, 0x000f00ff, &[Op::Oplen(3), Op::Name("vtlb"), Op::Reg(&REG2_R), Op::Reg(&REG1_R)], F_FUC3P),
    ef(0x000500fe, 0x000f00ff, &[Op::Oplen(3), Op::Unk("unkfe/5"), Op::Reg(&REG2_R), Op::Reg(&REG1_R)], F_FUC6P),
    e(0x000c00fe, 0x000f00ff, &[Op::Oplen(3), Op::Name("xbit"), Op::Reg(&REG2_R), Op::Reg(&FLAGS_R), Op::Reg(&REG1_R)]),
    e(0x000000fe, 0x000000ff, &[Op::Oplen(3), OOPS, Op::Reg(&REG2_R), Op::Reg(&REG1_R)]),

    e(0x000000ff, 0x000f00ff, &[Op::Oplen(3), Op::Name("mulu"), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000100ff, 0x000f00ff, &[Op::Oplen(3), Op::Name("muls"), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000200ff, 0x000f00ff, &[Op::Oplen(3), Op::Name("sext"), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    ef(0x000300ff, 0x000f00ff, &[Op::Oplen(3), Op::Name("extrs"), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)], F_FUC3P),
    e(0x000400ff, 0x000f00ff, &[Op::Oplen(3), Op::Name("and"), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000500ff, 0x000f00ff, &[Op::Oplen(3), Op::Name("or"), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    e(0x000600ff, 0x000f00ff, &[Op::Oplen(3), Op::Name("xor"), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    ef(0x000700ff, 0x000f00ff, &[Op::Oplen(3), Op::Name("extr"), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)], F_FUC3P),
    e(0x000800ff, 0x000f00ff, &[Op::Oplen(3), Op::Name("xbit"), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),
    ef(0x000c00ff, 0x000f00ff, &[Op::Oplen(3), Op::Name("div"), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)], F_FUC3P),
    ef(0x000d00ff, 0x000f00ff, &[Op::Oplen(3), Op::Name("mod"), Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)], F_FUC3P),
    e(0x000e00ff, 0x000f00ff, &[Op::Oplen(3), Op::Name("iords"), Op::Reg(&REG3_R), Op::Mem(&IORR_M)]),
    e(0x000f00ff, 0x000f00ff, &[Op::Oplen(3), Op::Name("iord"), Op::Reg(&REG3_R), Op::Mem(&IORR_M)]),
    e(0x000000ff, 0x000000ff, &[Op::Oplen(3), OOPS, Op::Reg(&REG3_R), Op::Reg(&REG1_R), Op::Reg(&REG2_R)]),

    e(0, 0, &[OOPS]),
];

/// falcon_isa_s — root table and limits. opunit = 1 byte, maxoplen = 6
/// bytes (dis-isas falcon.c:719-725; MAXOPLEN*8 = 16 bytes assembled but
/// only 6 are shown).
pub const MAXOPLEN: usize = 6;
pub const ROOT: &[InsnEntry] = TABM;
