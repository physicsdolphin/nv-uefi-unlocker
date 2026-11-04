//! fuc5 decode engine — a port of envytools envydis/core-dis.c semantics
//! (MIT) for the falcon ISA. Position-independent: `decode_one` works at
//! any byte offset; the decoded instruction depends only on the bytes at
//! that offset and the absolute position (for pcrel targets) — never on
//! surrounding instructions.

use crate::isa::{self, Op, RBitField, Variant};

/// Decoded operand expression. Only the forms falcon tables can emit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Num(i64),
    Reg(String),
    Mem {
        space: &'static str,
        idx: Option<u64>,
        inner: Box<Expr>,
    },
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    /// BITF8/16 two-piece range: [lo, hi] (hi = lo + second piece).
    VecRange(u64, u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Special {
    None,
    BTarget,
    CTarget,
}

#[derive(Clone, Debug)]
pub struct Operand {
    /// Modifier tokens accumulated before this operand's first expression
    /// (size tags like "b32", condition tags like "ne").
    pub mods: Vec<(String, bool)>,
    pub exprs: Vec<Expr>,
    pub special: Special,
}

/// One decoded instruction.
#[derive(Clone, Debug)]
pub struct Insn {
    /// Absolute position (base + offset) — pcrel targets resolve here.
    pub pos: u64,
    pub oplen: usize,
    pub name: String,
    pub name_unk: bool,
    pub operands: Vec<Operand>,
    pub endmark: bool,
    /// Truncated at the buffer end mid-opcode.
    pub eof: bool,
    /// No table entry set a length; fell back to 1 byte ("???").
    pub unk_form: bool,
    /// Bytes within `oplen` that no atom claimed.
    pub unknown: Vec<u8>,
}

impl Insn {
    pub fn is_unknown(&self) -> bool {
        self.name_unk || self.unk_form
    }

    /// Instructions whose (static) decode leaves the linear stream —
    /// pop-returns and absolute tails. `lcall`/`lbra` transfer control
    /// but the scanner treats them separately (a call returns into the
    /// next ROP frame).
    pub fn endmark(&self) -> bool {
        self.endmark
    }

    /// Branch/call targets (absolute values, pcrel already resolved).
    pub fn targets(&self) -> Vec<(Special, i64)> {
        let mut out = vec![];
        for op in &self.operands {
            if op.special != Special::None {
                if let Some(Expr::Num(v)) = op.exprs.first() {
                    out.push((op.special, *v));
                }
            }
        }
        out
    }

    /// Immediate numbers from non-target operands (call/branch targets
    /// are code addresses, not data — tagging them as DMEM immediates is
    /// scanner noise).
    pub fn data_imms(&self) -> Vec<i64> {
        let mut out = Vec::new();
        for op in &self.operands {
            if op.special != Special::None {
                continue;
            }
            for e in &op.exprs {
                walk_imms(e, &mut out);
            }
        }
        out
    }

    /// All immediate numbers appearing anywhere in the operands.
    pub fn imms(&self) -> Vec<i64> {
        let mut out = vec![];
        for op in &self.operands {
            for e in &op.exprs {
                walk_imms(e, &mut out);
            }
        }
        out
    }

    /// The data-space base register name of the first mem operand, if any.
    pub fn mem_base(&self) -> Option<&str> {
        for op in &self.operands {
            for e in &op.exprs {
                if let Expr::Mem { inner, .. } = e {
                    if let Expr::Reg(name) = &**inner {
                        return Some(name);
                    }
                }
            }
        }
        None
    }

    pub fn render(&self) -> String {
        let mut s = String::new();
        s.push_str(&self.name);
        for op in &self.operands {
            for (m, unk) in &op.mods {
                s.push(' ');
                s.push_str(m);
                if *unk {
                    s.push('?');
                }
            }
            for e in &op.exprs {
                s.push(' ');
                s.push_str(&render_expr(e));
            }
        }
        s
    }
}

pub fn render_expr(e: &Expr) -> String {
    match e {
        Expr::Num(v) => fmt_num(*v),
        Expr::Reg(name) => format!("${name}"),
        Expr::Mem { space, idx, inner } => match idx {
            Some(i) => format!("{space}{i}[{}]", render_expr(inner)),
            None => format!("{space}[{}]", render_expr(inner)),
        },
        Expr::Add(a, b) => format!("{}+{}", render_expr(a), render_expr(b)),
        Expr::Mul(a, b) => format!("{}*{}", render_expr(a), render_expr(b)),
        Expr::VecRange(a, b) => format!("{a}..{b}"),
    }
}

fn walk_imms(e: &Expr, out: &mut Vec<i64>) {
    match e {
        Expr::Num(v) => out.push(*v),
        Expr::Mem { inner, .. } => walk_imms(inner, out),
        Expr::Add(a, b) | Expr::Mul(a, b) => {
            walk_imms(a, out);
            walk_imms(b, out);
        }
        Expr::VecRange(a, b) => {
            out.push(*a as i64);
            out.push(*b as i64);
        }
        Expr::Reg(_) => {}
    }
}

pub fn fmt_num(v: i64) -> String {
    if v < 0 {
        format!("-{:#x}", -(v as i128))
    } else {
        format!("{v:#x}")
    }
}

enum Item {
    Name(&'static str, bool),
    Expr(Expr, Special),
}

struct Eval {
    a: u64,
    m: u64,
    var: Variant,
    pos: u64,
    items: Vec<Item>,
    oplen: usize,
    endmark: bool,
}

impl Eval {
    /// bf_() — extract `len` bits at `pos`, marking them used. Falcon
    /// fields stay within one u64 word (max bit 39), so the cross-word
    /// path of the C original is unreachable here.
    fn bf(&mut self, pos: u8, len: u8) -> u64 {
        debug_assert!(len < 64 && (pos as u16 + len as u16) <= 64);
        let m = if len >= 64 { !0u64 } else { ((1u64 << len) - 1) << pos };
        self.m |= m;
        (self.a & m) >> pos
    }

    fn get_bf(&mut self, bf: &isa::BitField) -> u64 {
        let mut res = 0u64;
        let mut pos = bf.shr as u32;
        for p in [bf.sbf[0], bf.sbf[1]] {
            res |= self.bf(p.pos, p.len) << pos;
            pos += p.len as u32;
        }
        res = apply_sign(res, pos, bf.mode);
        res ^= bf.xorend;
        res.wrapping_add(bf.addend)
    }

    /// Raw rbitfield value (sign modes applied, shr pre-shift included).
    fn get_rbf(&mut self, bf: &RBitField) -> u64 {
        let mut res = 0u64;
        let mut pos = bf.shr as u32;
        for p in [bf.sbf[0], bf.sbf[1]] {
            res |= self.bf(p.pos, p.len) << pos;
            pos += p.len as u32;
        }
        apply_sign(res, pos, bf.mode)
    }

    /// rbitfield → final value. pcrel fields resolve against the
    /// instruction position (easm_substpos_expr + cfold in C).
    fn rbf_value(&mut self, bf: &RBitField) -> i64 {
        let res = self.get_rbf(bf);
        if bf.pcrel {
            // expr = POS [+pospreadd] [& -(1<<shr)] [+ res] [+ addend];
            // every falcon pcrel form has pospreadd = 0, shr = 0.
            (self.pos as i64).wrapping_add(res as i64).wrapping_add(bf.addend as i64)
        } else {
            res.wrapping_add(bf.addend) as i64
        }
    }

    fn reg_expr(&mut self, r: &isa::RegDef) -> Expr {
        let num = r.bf.map(|b| self.get_bf(&b)).unwrap_or(0);
        for s in r.specials {
            if !isa::var_ok(s.fmask, self.var) {
                continue;
            }
            if num == s.num as u64 {
                return Expr::Reg(s.name.to_string());
            }
        }
        if r.bf.is_some() {
            Expr::Reg(format!("{}{}", r.name, num))
        } else {
            Expr::Reg(r.name.to_string())
        }
    }

    fn mem_expr(&mut self, mf: &isa::MemForm) -> Expr {
        let mut expr = mf.reg.map(|r| self.reg_expr(r));
        if let Some(bf) = mf.imm {
            let v = self.get_rbf(bf) as i64;
            let imm = Expr::Num(v);
            expr = Some(match expr {
                Some(e) => Expr::Add(Box::new(e), Box::new(imm)),
                None => imm,
            });
        }
        if let Some(r2) = mf.reg2 {
            let mut e = self.reg_expr(r2);
            if mf.reg2shr > 0 {
                e = Expr::Mul(Box::new(e), Box::new(Expr::Num(1 << mf.reg2shr)));
            }
            expr = Some(match expr {
                Some(base) => Expr::Add(Box::new(base), Box::new(e)),
                None => e,
            });
        }
        let inner = expr.unwrap_or(Expr::Num(0));
        let idx = mf.idx.map(|b| self.get_bf(b));
        Expr::Mem { space: mf.name, idx, inner: Box::new(inner) }
    }

    fn eval(&mut self, ops: &[Op]) {
        for op in ops {
            match op {
                Op::Tab(t) => {
                    let entry = select(t, self.a, self.var)
                        .expect("falcon tables all end with a catch-all");
                    self.m |= entry.mask;
                    self.eval(entry.ops);
                }
                Op::Oplen(n) => self.oplen = *n as usize,
                Op::Name(s) => self.items.push(Item::Name(s, false)),
                Op::Reg(r) => {
                    let e = self.reg_expr(r);
                    self.items.push(Item::Expr(e, Special::None));
                }
                Op::RImm(bf) => {
                    let v = self.rbf_value(bf);
                    self.items.push(Item::Expr(Expr::Num(v), Special::None));
                }
                Op::BTarget(bf) => {
                    let v = self.rbf_value(bf);
                    self.items.push(Item::Expr(Expr::Num(v), Special::BTarget));
                }
                Op::CTarget(bf) => {
                    let v = self.rbf_value(bf);
                    self.items.push(Item::Expr(Expr::Num(v), Special::CTarget));
                }
                Op::Imm(bf) => {
                    let v = self.get_bf(bf) as i64;
                    self.items.push(Item::Expr(Expr::Num(v), Special::None));
                }
                Op::Mem(mf) => {
                    let e = self.mem_expr(mf);
                    self.items.push(Item::Expr(e, Special::None));
                }
                Op::Bf(pair) => {
                    let n1 = self.get_bf(&pair[0]);
                    let n2 = n1.wrapping_add(self.get_bf(&pair[1]));
                    self.items.push(Item::Expr(Expr::VecRange(n1, n2), Special::None));
                }
                Op::Endmark => self.endmark = true,
                Op::Unk(s) => self.items.push(Item::Name(s, true)),
            }
        }
    }
}

fn apply_sign(res: u64, pos: u32, mode: isa::Sign) -> u64 {
    match mode {
        isa::Sign::Unsigned => res,
        isa::Sign::Signed => {
            if pos > 0 && res & (1 << (pos - 1)) != 0 {
                res.wrapping_sub(1 << pos)
            } else {
                res
            }
        }
        isa::Sign::SlightlySigned => {
            if pos > 1 && res & (1 << (pos - 1)) != 0 && res & (1 << (pos - 2)) != 0 {
                res.wrapping_sub(1 << pos)
            } else {
                res
            }
        }
        isa::Sign::UltraSigned => res.wrapping_sub(1 << pos),
    }
}

fn select<'t>(
    tab: &'t [isa::InsnEntry],
    a: u64,
    var: Variant,
) -> Option<&'t isa::InsnEntry> {
    tab.iter()
        .find(|e| (a & e.mask) == e.val && isa::var_ok(e.fmask, var))
}

/// Decode one instruction starting at `off` bytes into `buf`; `base` is
/// the absolute VA of buf[0] (pcrel targets resolve against it).
pub fn decode_one(buf: &[u8], off: usize, base: u64, var: Variant) -> Insn {
    let mut a: u64 = 0;
    for i in 0..8usize {
        if off + i < buf.len() {
            a |= (buf[off + i] as u64) << (i * 8);
        }
    }
    let mut st = Eval {
        a,
        m: 0,
        var,
        pos: base + off as u64,
        items: Vec::new(),
        oplen: 0,
        endmark: false,
    };
    let entry = select(isa::ROOT, a, var).expect("root table ends with a catch-all");
    st.m |= entry.mask;
    st.eval(entry.ops);

    // Parse the item stream (dis_parse_sinsn): leading exprs are
    // prefixes (falcon emits none), the first name is the mnemonic,
    // later names are modifiers of the NEXT operand.
    let mut name = String::from("???");
    let mut name_unk = false;
    let mut prefixes: Vec<Expr> = Vec::new();
    let mut operands: Vec<Operand> = Vec::new();
    let mut mods: Vec<(String, bool)> = Vec::new();
    let mut seen_name = false;
    for item in st.items.drain(..) {
        match item {
            Item::Name(s, isunk) => {
                if !seen_name {
                    seen_name = true;
                    name = s.to_string();
                    name_unk = isunk;
                } else {
                    mods.push((s.to_string(), isunk));
                }
            }
            Item::Expr(e, special) => {
                if !seen_name {
                    prefixes.push(e);
                    continue;
                }
                // Each expression is its own operand carrying the mods
                // accumulated since the previous operand (dis_parse_sinsn:
                // every expr closes one operand; consecutive exprs never
                // merge — the interpreter depends on this distinction).
                let op = Operand { mods: std::mem::take(&mut mods), exprs: vec![e], special };
                operands.push(op);
            }
        }
    }
    let mut oplen = st.oplen;
    let mut unk_form = false;
    if oplen == 0 {
        unk_form = true;
        oplen = 1; // opunit
    }
    let eof = off + oplen > buf.len();
    // Unknown bits: claimed = a & !m, reported per byte within oplen.
    let unclaimed = a & !st.m;
    let mut unknown = Vec::new();
    for i in 0..oplen.min(8) {
        unknown.push(((unclaimed >> (i * 8)) & 0xff) as u8);
    }

    Insn {
        pos: base + off as u64,
        oplen,
        name,
        name_unk,
        operands,
        endmark: st.endmark,
        eof,
        unk_form,
        unknown,
    }
}

/// Parse an envydis lst (linear sweep text) into (file-offset, mnemonic)
/// pairs. Lines: `ADDRESS: XX XX [??] [B] [C] mnemonic operands ...`.
/// Only lines with an 8-hex-digit address count.
pub fn parse_lst(text: &str) -> Vec<(u64, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let b = line.as_bytes();
        if b.len() < 9 || !b[..8].iter().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        let Ok(off) = u64::from_str_radix(&line[..8], 16) else { continue };
        let mut mn: Option<String> = None;
        for tok in line[9..].split_whitespace() {
            let is_bytes = (tok.len() == 2 && tok.bytes().all(|c| c.is_ascii_hexdigit()))
                || tok == "??"
                // envydis prints single-char call/branch target marks
                // (C/B columns) between the hex bytes and the text.
                || tok == "B"
                || tok == "C";
            if !is_bytes {
                mn = Some(tok.to_string());
                break;
            }
        }
        out.push((off, mn.unwrap_or_else(|| "???".to_string())));
    }
    out
}

/// Linear decode from offset 0 (the envydis driver's no-labels path).
pub fn decode_linear(buf: &[u8], base: u64, var: Variant) -> Vec<Insn> {
    let mut out = Vec::new();
    let mut off = 0usize;
    while off < buf.len() {
        let insn = decode_one(buf, off, base, var);
        off += insn.oplen;
        out.push(insn);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isa::FUC5;

    fn one(bytes: &[u8], base: u64) -> Insn {
        decode_one(bytes, 0, base, FUC5)
    }

    #[test]
    fn mov_imm_canary_gadget() {
        // the TU102 guard gadget at 0x1f89: mov $r15, 0x6140
        let i = one(&[0x8f, 0x40, 0x61, 0x00], 0x1f89);
        assert_eq!(i.name, "mov");
        assert_eq!(i.oplen, 4);
        assert!(!i.is_unknown());
        assert_eq!(i.imms(), vec![0x6140]);
    }

    #[test]
    fn mid_entry_mov_decodes() {
        // entering the TU102 write mid-entry (0x10b9): 04 c5 -> mov $r4, -0x3b.
        // the harvested length-table scanner missed this; the positional
        // decoder handles it by construction.
        let i = one(&[0x04, 0xc5], 0x10b9);
        assert_eq!(i.name, "mov");
        assert_eq!(i.oplen, 2);
        assert_eq!(i.imms(), vec![-0x3b]);
    }

    #[test]
    fn lbra_self_park() {
        // the TU102 park at 0x7ea4: lbra 0x7ea4 (absolute 24-bit target)
        let i = one(&[0x3e, 0xa4, 0x7e, 0x00], 0x7ea4);
        assert_eq!(i.name, "lbra");
        assert_eq!(i.oplen, 4);
        let (sp, v) = i.targets()[0];
        assert_eq!(sp, Special::BTarget);
        assert_eq!(v, 0x7ea4);
    }

    #[test]
    fn lcall_absolute() {
        let i = one(&[0x7e, 0x3b, 0x0e, 0x00], 0x10c1);
        assert_eq!(i.name, "lcall");
        let (sp, v) = i.targets()[0];
        assert_eq!(sp, Special::CTarget);
        assert_eq!(v, 0xe3b);
    }

    #[test]
    fn pcrel_bra_resolves_against_pos() {
        // bra e +7 at pos 0x1090 -> 0x1097 (lst: bra b8 $r10 0x0 ne 0xf9c
        // shape; here the plain cond form)
        let i = one(&[0xf4, 0x0b, 0x07], 0x1090);
        assert_eq!(i.name, "bra");
        assert_eq!(i.oplen, 3);
        let (sp, v) = i.targets()[0];
        assert_eq!(sp, Special::BTarget);
        assert_eq!(v, 0x1090 + 7);
    }

    #[test]
    fn mpopaddret_frame_shape() {
        // fb 25 04 -> mpopaddret $r2, 0x4, endmark, 3 bytes
        let i = one(&[0xfb, 0x25, 0x04], 0xcc4);
        assert_eq!(i.name, "mpopaddret");
        assert_eq!(i.oplen, 3);
        assert!(i.endmark());
    }

    #[test]
    fn iowrs_mailbox_write() {
        // f7 9f 00 -> iowrs I[$r9+0x0], $r15 (the NS stub's mailbox write)
        let i = one(&[0xf7, 0x9f, 0x00], 0x7c);
        assert_eq!(i.name, "iowrs");
        assert_eq!(i.oplen, 3);
        assert!(matches!(i.operands[0].exprs[0], Expr::Mem { .. }));
    }

    #[test]
    fn st_dmem_form() {
        // a0 9f -> st b32 D[$r9], $r15
        let i = one(&[0xa0, 0x9f], 0xf8e);
        assert_eq!(i.name, "st");
        assert_eq!(i.oplen, 2);
        assert!(matches!(i.operands[0].exprs[0], Expr::Mem { .. }));
    }

    #[test]
    fn unknown_form_reports_and_advances() {
        // f5 3c 00 e0: envydis says ??? with a 4-byte window (matched via
        // the f4 catch-all) and an unknown-bits report
        let i = one(&[0xf5, 0x3c, 0x00, 0xe0], 0x107);
        assert!(i.is_unknown());
        assert_eq!(i.oplen, 4);
        assert_eq!(i.unknown, vec![0x00, 0x3c, 0x00, 0x00]);
    }

    #[test]
    fn linear_decode_matches_envydis_stream() {
        // the plaintext's first three instructions per the lst
        let buf = [
            0x89, 0xfc, 0xff, 0x00, // mov $r9 0xfffc
            0xfe, 0x94, 0x00,       // mov $sp $r9
            0xf5, 0x3c, 0x00, 0xe0, // ??? (4 bytes)
        ];
        let got = decode_linear(&buf, 0x100, FUC5);
        assert_eq!(got.len(), 3);
        assert_eq!((got[0].name.as_str(), got[0].oplen), ("mov", 4));
        assert_eq!((got[1].name.as_str(), got[1].oplen), ("mov", 3));
        assert!(got[2].is_unknown());
        assert_eq!(got[2].oplen, 4);
    }

    #[test]
    fn scanner_finds_park_store_and_iowr() {
        // synthetic: st D[$r9] $r15; iowrs I[$r9] $r15; lbra self at 0x300
        let park = [0x3e_u8, 0x05, 0x01, 0x00]; // lbra 0x105 = its own VA (absolute)
        let mut buf = vec![0xa0, 0x9f, 0xf7, 0x9f, 0x00];
        buf.extend_from_slice(&park);
        let base = 0x100;
        let map = crate::scan::scan(&buf, base, FUC5);
        assert!(map.get(&(base + 0)).unwrap().tags.contains(&crate::scan::Tag::DmemStore));
        assert!(map.get(&(base + 2)).unwrap().tags.contains(&crate::scan::Tag::MmioWrite));
        assert!(map.get(&(base + 5)).unwrap().tags.contains(&crate::scan::Tag::ParkSelf));
    }
}
