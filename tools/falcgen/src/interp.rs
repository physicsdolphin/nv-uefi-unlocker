//! Narrow fuc5 interpreter (G1 Track B) — plaintext-fed, MMIO-stubbed,
//! DMA-as-copy. Purpose: run the decrypted booter from its secure entry
//! along the normal (stock-signature) flow up to the signature DMA, and
//! snapshot the stack — the live frame layout, the caller chain and the
//! canary slots are then read directly off the DMEM dump. That snapshot
//! is the first-principles geometry derivation (the payload's ROP frames
//! must thread exactly those epilogues).
//!
//! Execution model: SEC entry = VA 0x100 (the NS stub's `lcall 0x100`
//! transfers there; the NS stub itself is not simulated — its job,
//! loading the secure segment, is already done in our input). Crypt
//! instructions are state stubs (csigcmp succeeds — the stock signature
//! verifies; csigclr/csecret/cxor no-op). IO reads return the seeded
//! mailbox/status values; IO writes are logged (the DMA trigger is
//! watched on the falcon DMA-register window).

use crate::decode::{decode_one, Expr, Insn, Special};
use crate::isa::FUC5_CRYPT;

pub const DMEM_SIZE: usize = 0x10000;
pub const SEC_ENTRY: u64 = 0x100;
/// The falcon's own DMA register window (AGENTS §7.2: IMEM/DMEM DMA
/// +0x110–0x128).
pub const DMA_IO_LO: u32 = 0x100;
pub const DMA_IO_HI: u32 = 0x140;

#[derive(Clone, Debug)]
pub struct DmaCopy {
    pub dst: u32,
    pub len: u32,
}

pub struct Interp {
    pub regs: [u32; 16],
    pub sp: u32,
    pub pc: u32,
    /// flags: bit0 = carry, bit1 = zero, bit2 = sign, bit3 = overflow
    pub flags: u32,
    pub preds: [bool; 8],
    pub dmem: Vec<u8>,
    pub io_read_default: u32,
    /// per-address IO read overrides (mailbox polls etc.)
    pub io_reads: Vec<(u32, u32)>,
    pub io_writes: Vec<(u32, u32)>,
    /// DMEM DMA copies performed (dst, len) in trigger order
    pub dma_copies: Vec<DmaCopy>,
    /// host bytes delivered by a DMA into DMEM (indexed by dma count)
    pub host_buffers: Vec<Vec<u8>>,
    pub halted: bool,
    pub exit_reason: String,
    pub steps: u64,
    pub max_steps: u64,
    pub trace: bool,
    /// diagnostic watches (canary-slot writes) — off by default
    pub watch_canary: bool,
    pub last_dma_sp: Option<u32>,
    /// sp value captured right after the first DMA into the payload window
    pub stop_after_dma: bool,
}

impl Interp {
    pub fn new() -> Self {
        Interp {
            regs: [0; 16],
            sp: 0xfffc,
            pc: SEC_ENTRY as u32,
            flags: 0,
            preds: [false; 8],
            dmem: vec![0; DMEM_SIZE],
            io_read_default: 0,
            io_reads: Vec::new(),
            io_writes: Vec::new(),
            dma_copies: Vec::new(),
            host_buffers: Vec::new(),
            halted: false,
            exit_reason: String::new(),
            steps: 0,
            max_steps: 20_000_000,
            trace: false,
            watch_canary: false,
            last_dma_sp: None,
            stop_after_dma: true,
        }
    }

    pub fn seed_meta_sig_len(&mut self, len: u32) {
        // WprMeta at DMEM 0x400 (nlzy table): the loader-shaped meta
        // (tx_booter.rs wpr_meta_pack) with a self-consistent TU102 8G
        // layout — the booter validates the relationships (frts < wpr_end,
        // image <= boot-gsp, heap = 8MiB + align(96KiB*GiB) + 96MiB) and
        // the expected-value table before touching the signature DMA.
        let mut wr64 = |a: usize, v: u64| self.dmem[a..a + 8].copy_from_slice(&v.to_le_bytes());
        wr64(0x400, 0xdc3a_ae21_371a_60b3); // WPR_META_MAGIC
        wr64(0x408, 1); // version
        wr64(0x410, 0x2_0000_0000); // radix_dma
        wr64(0x418, 0x1c0_0000); // image_size (gsp RM)
        wr64(0x420, 0x1e0_0000); // boot_dma
        wr64(0x428, 0x6000); // boot image_size
        wr64(0x430, 0x1800); // boot code_offset
        wr64(0x438, 0x800); // boot data_offset
        wr64(0x440, 0x0); // boot manifest_offset
        wr64(0x448, 0x1e8_0000); // signature_dma
        wr64(0x450, len as u64); // signature_size
        wr64(0x458, 0x1f8_0000); // gspFwRsvdStart
        wr64(0x460, 0x1f8_0000); // nonWprHeapOffset
        wr64(0x468, 0x690_0000); // nonWprHeapSize (105 MiB, 8G)
        wr64(0x470, 0x260_0000); // wpr_start
        wr64(0x478, 0x1f8_0000); // heapOffset
        wr64(0x480, 0x690_0000); // heapSize
        wr64(0x488, 0x1e8_0000); // gspOffset
        wr64(0x490, 0x1e0_0000); // bootOffset
        wr64(0x498, 0x260_0000); // frtsOffset
        wr64(0x4a0, 0x1_a0_0000); // frtsSize (wpr_end - frts)
        wr64(0x4a8, 0x260_0000); // wprEnd
        wr64(0x4b0, 0x2_0000_0000); // fbSize (8G)
        wr64(0x4b8, 0x1_fef0_0000); // vgaOffset
        wr64(0x4c0, 0x110_0000); // fbSize - vgaOffset
        self.dmem[0x400 + 241] = 0x1; // flags: CLOCK_BOOST
        // The expected-values check (0x8b1: ld D[$r0=0xd500] == 1) reads a
        // runtime-populated table entry — the booter's earlier stages store
        // the descriptor results here.
        self.wr8(0xd500, 0x1);
        // Pre-SEC machine state the NS stub leaves behind (we start at the
        // SEC entry, so the NS stub's side effects must be seeded):
        // - D[0x6140] = the global canary value (NS canary sub, 0xd2408)
        // - D[0x1650]/D[0x1658] = mailbox0/mailbox1 snapshots; mailbox0
        //   carries the WprMeta address the loader pushed.
        self.wr8(0x6140, 0xd2408);
        self.wr8(0x1650, 0x400);
        self.wr8(0x1654, 0);
        self.wr8(0x1658, 0);
        self.wr8(0x165c, 0);
        // FWSEC-established IO state the booter polls for (the loader
        // fires FWSEC before the booter; the poll at 0x358 waits for
        // bit8 of [0x10100] to clear — the FWSEC-completion flag).
        self.io_reads.push((0x10100, 0x1000));
        // IO 0xd000/0xd400/0xd500 = the loader->booter command mailboxes
        // (0x8235 = "read IO[$r10] and deliver via the canary block into
        // $r10"). The booter spins at 0x9c8 until the loader pushes a
        // non-zero command; the real loader writes the boot parameters
        // here. Seed non-zero triggers, refine as the flow reveals their
        // semantics.
        self.io_reads.push((0xd400, 0x1));
        // IO 0xd500: the booter caches its read into DMEM[0xd500] (the st
        // at 0x9f5 with $r0 leaked from the canary block) and the caller's
        // expected-values check then reads the cache — hardware holds 1.
        self.io_reads.push((0xd500, 0x1));

        // The loader-provided boot descriptor at DMEM 0x200 (the NS stub
        // snapshots mailboxes there; "any non-zero skips the default fill").
        self.wr8(0x200, 0x1);
        self.wr8(0x204, 0x400);
        self.wr8(0x208, 0);
        self.wr8(0x20c, 0);
        // The canary seed: the SEC scans D[0x6130..0x6140) (the crypt
        // context's key slots, right after `cxset 0x0`) for the first
        // non-zero dword and adopts it as the canary (stored to D[0x6140]
        // at 0x4ca). Our crypt stubs are no-ops, so the seed area must be
        // planted; any non-zero value is self-consistent (the store and
        // every canary check use the same one).
        self.wr8(0x6130, 0xd2408);
    }

    fn rd8(&self, a: usize) -> u32 {
        if a + 4 <= self.dmem.len() {
            u32::from_le_bytes(self.dmem[a..a + 4].try_into().unwrap())
        } else {
            0
        }
    }

    fn wr8(&mut self, a: usize, v: u32) {
        if self.trace && self.watch_canary && a == 0x6140 {
            println!("    [canary-watch] step {} D[0x6140] <- {v:#x} at pc={:#x}", self.steps, self.pc);
        }
        if self.trace && self.watch_canary && v == 0xd000 && a >= 0xff00 {
            println!("    [d000-watch] step {} D[{a:#x}] <- {v:#x} at pc={:#x}", self.steps, self.pc);
        }
        if a + 4 <= self.dmem.len() {
            self.dmem[a..a + 4].copy_from_slice(&v.to_le_bytes());
        }
    }

    fn pop(&mut self) -> u32 {
        self.sp = self.sp.wrapping_add(4);
        self.rd8((self.sp - 4) as usize)
    }

    fn push(&mut self, v: u32) {
        self.sp = self.sp.wrapping_sub(4);
        self.wr8(self.sp as usize, v);
    }

    fn io_read(&self, addr: u32) -> u32 {
        for (a, v) in &self.io_reads {
            if *a == addr {
                return *v;
            }
        }
        // Register read-back: the booter writes a register then reads it
        // back (e.g. the falcon-enable word 0x7fff -> 0x14000 feeds the
        // mpush mask). The last write to the address is the value.
        for (a, v) in self.io_writes.iter().rev() {
            if *a == addr {
                return *v;
            }
        }
        self.io_read_default
    }

    fn set_zcs(&mut self, v: u32) {
        let z = (v == 0) as u32;
        let s = (v & 0x8000_0000 != 0) as u32;
        self.flags = (z << 1) | (s << 2);
    }

    fn eval(&self, e: &Expr) -> u32 {
        match e {
            Expr::Num(v) => *v as u32,
            Expr::Reg(name) => self.reg_by_name(name),
            Expr::Mem { inner, .. } => {
                let a = self.eval(inner) as usize;
                self.rd8(a)
            }
            Expr::Add(a, b) => self.eval(a).wrapping_add(self.eval(b)),
            Expr::Mul(a, b) => self.eval(a).wrapping_mul(self.eval(b)),
            Expr::VecRange(lo, hi) => ((((*hi & 0xffff) << 16) | (*lo & 0xffff)) as u32),
        }
    }


    fn reg_by_name(&self, name: &str) -> u32 {
        // Expr::Reg stores bare names ("r9", "sp", "flags", "iv0", ...)
        // — the "$" is added only at render time.
        if let Some(n) = name.strip_prefix('r') {
            if let Ok(i) = n.parse::<usize>() {
                return self.regs.get(i).copied().unwrap_or(0);
            }
        }
        match name {
            "sp" => self.sp,
            _ => 0,
        }
    }

    fn set_reg(&mut self, name: &str, v: u32) {
        if let Some(n) = name.strip_prefix('r') {
            if let Ok(i) = n.parse::<usize>() {
                if i < 16 {
                    self.regs[i] = v;
                    return;
                }
            }
        }
        if name == "sp" {
            self.sp = v;
        }
    }

    fn cond(&self, mods: &[(String, bool)], regs: &[u32]) -> Option<bool> {
        // branch condition from the modifier tags (tabp/tabfl names) or
        // the register-compare form.
        for (m, _) in mods {
            let f = self.flags;
            let c = f & 1 != 0;
            let z = f & 2 != 0;
            let s = f & 4 != 0;
            let o = f & 8 != 0;
            let r = match m.as_str() {
                "b" | "c" => c,
                "o" => o,
                "s" => s,
                "e" | "z" => z,
                "a" => !c && !z,
                "be" | "na" => c || z,
                "ae" | "nb" | "nc" => !c,
                "no" => !o,
                "ns" => !s,
                "ne" | "nz" => !z,
                "g" | "le" | "l" | "ge" => s != o, // approximate signed
                "p0" | "p1" | "p2" | "p3" | "p4" | "p5" | "p6" | "p7" => {
                    let i: usize = m[1..].parse().unwrap_or(0);
                    self.preds[i]
                }
                _ => continue,
            };
            return Some(r);
        }
        // register-compare form: bra sz $rX imm cond target
        if regs.len() >= 2 {
            let (a, b) = (regs[0], regs[1]);
            for (m, _) in mods {
                let r = match m.as_str() {
                    "e" => a == b,
                    "ne" => a != b,
                    _ => continue,
                };
                return Some(r);
            }
        }
        None
    }

    /// reg indexes referenced by an operand (for the bra compare form)
    fn reg_nums(&self, insn: &Insn) -> Vec<u32> {
        let mut out = vec![];
        for op in &insn.operands {
            for e in &op.exprs {
                collect_regs(e, &mut out);
            }
        }
        out
    }

    /// Execute one instruction. Returns Err on an unimplemented shape.
    fn step_once(&mut self, buf: &[u8], base: u64) -> Result<(), String> {
        let off = (self.pc as u64).checked_sub(base).ok_or("pc below base")? as usize;
        if off >= buf.len() {
            self.halted = true;
            self.exit_reason = format!("pc ran off the image at {:#x}", self.pc);
            return Ok(());
        }
        let insn = decode_one(buf, off, base, FUC5_CRYPT);
        let next = self.pc + insn.oplen as u32;
        if self.trace {
            println!("[{:>8}] {:06x} {}  [r0={:#x} r2={:#x} r9={:#x} sp={:#x}]", self.steps, self.pc, insn.render(), self.regs[0], self.regs[2], self.regs[9], self.sp);
        }
        let name = insn.name.as_str();
        let ops = &insn.operands;
        let mut jumped = false;

        // helper closures as plain code below (borrow split: read regs first)
        let imm = |i: usize| -> Option<u32> {
            ops.get(i).and_then(|o| o.exprs.first()).and_then(|e| match e {
                Expr::Num(v) => Some(*v as u32),
                _ => None,
            })
        };
        let expr = |i: usize| -> Option<Expr> { ops.get(i).and_then(|o| o.exprs.first().cloned()) };

        match name {
            "mov" | "movw" | "sethi" | "hswap" | "clear" | "not" | "neg" => {
                // dst reg, src (imm or reg)
                if let (Some(Expr::Reg(d)), src) = (expr(0), expr(1)) {
                    let v = match name {
                        "clear" => 0,
                        "not" => !self.eval(src.as_ref().unwrap_or(&Expr::Num(0))),
                        "neg" => (self.eval(src.as_ref().unwrap_or(&Expr::Num(0))) as i32).wrapping_neg() as u32,
                        "sethi" => (imm(1).unwrap_or(0) << 16) & 0xffff_0000,
                        "hswap" => {
                            let v = self.eval(src.as_ref().unwrap_or(&Expr::Num(0)));
                            (v << 16) | (v >> 16)
                        }
                        _ => self.eval(src.as_ref().unwrap_or(&Expr::Num(0))),
                    };
                    // movw writes 16 bits
                    let v = if name == "movw" { v & 0xffff } else { v };
                    self.set_reg(&d, v);
                } else if let (Some(Expr::Mem { .. }), _) = (expr(0), expr(1)) {
                    // mov between mem forms — not expected on this path
                } else {
                    return Err(format!("unparsed mov at {:#x}: {}", self.pc, insn.render()));
                }
            }
            "add" | "adc" | "sub" | "sbb" | "and" | "or" | "xor" | "shl" | "shr" | "sar"
            | "shlc" | "shrc" | "cmpu" | "cmps" | "cmp" | "mulu" | "muls" | "sext" | "xbit"
            | "div" | "mod" | "extr" | "extrs" | "ins" => {
                // three-address: op dst, a, b — b may be imm or reg; also
                // the two-address forms (dst is src-a) and the SP forms
                // (add $sp, imm)
                let dst = expr(0);
                let (a, b) = if insn.operands.len() >= 3 {
                    (expr(1), expr(2))
                } else {
                    (dst.clone(), expr(1))
                };
                let av = a.as_ref().map(|e| self.eval(e));
                let bv = b.as_ref().map(|e| self.eval(e));
                if let (Some(Expr::Reg(d)), Some(av), Some(bv)) = (dst, av, bv) {
                    if d == "sp" {
                        // add $sp, imm (frame alloc) / sub
                        let delta = match name {
                            "add" => (bv as i32),
                            "sub" => -(bv as i32),
                            _ => 0,
                        };
                        self.sp = (self.sp as i32).wrapping_add(delta) as u32;
                        self.pc = next;
                        return Ok(());
                    }
                    let (v, fl) = alu(name, av, bv, self.flags);
                    self.flags = fl;
                    // compares set flags only — the "dst" operand is the
                    // compare's left side, never written
                    if !matches!(name, "cmp" | "cmpu" | "cmps") {
                        self.set_reg(&d, v);
                    }
                } else {
                    return Err(format!("unparsed alu at {:#x}: {} | ops={:?}", self.pc, insn.render(), insn.operands));
                }
            }
            "setp" => {
                // setp cond, reg  |  setp dst, a, b: evaluate and set pred
                if let Some(Expr::Reg(d)) = expr(0) {
                    if let Some(i) = d.strip_prefix('p').and_then(|s| s.parse::<usize>().ok()) {
                        if i < 8 {
                            let a = expr(1).map(|e| self.eval(&e)).unwrap_or(0);
                            let b = expr(2).map(|e| self.eval(&e)).unwrap_or(0);
                            self.preds[i] = a == b;
                        }
                    }
                } else {
                    // setp <flag-cond> $rN: set flags from reg (approx: z)
                    let v = expr(1).map(|e| self.eval(&e)).unwrap_or(0);
                    self.set_zcs(v);
                }
            }
            "ld" => {
                // ld dst, [mem]
                if let (Some(Expr::Reg(d)), Some(m)) = (expr(0), expr(1)) {
                    if let Expr::Mem { inner, .. } = &m {
                        let a = self.eval(inner) as usize;
                        let v = self.rd8(a);
                        self.set_reg(&d, v);
                    } else {
                        return Err(format!("ld non-mem at {:#x}", self.pc));
                    }
                } else {
                    return Err(format!("unparsed ld at {:#x}: {}", self.pc, insn.render()));
                }
            }
            "st" => {
                // st [mem], src
                if let (Some(m), src) = (expr(0), expr(1)) {
                    if let Expr::Mem { inner, .. } = &m {
                        let a = self.eval(inner) as usize;
                        let v = src.as_ref().map(|e| self.eval(e)).unwrap_or(0);
                        self.wr8(a, v);
                    } else {
                        return Err(format!("st non-mem at {:#x}", self.pc));
                    }
                } else {
                    return Err(format!("unparsed st at {:#x}: {}", self.pc, insn.render()));
                }
            }
            "iowr" | "iowrs" => {
                // iowr [I mem], src
                if let (Some(m), src) = (expr(0), expr(1)) {
                    if let Expr::Mem { inner, .. } = &m {
                        let a = self.eval(inner);
                        let v = src.as_ref().map(|e| self.eval(e)).unwrap_or(0);
                        self.io_writes.push((a, v));
                        if (DMA_IO_LO..=DMA_IO_HI).contains(&a) && self.trace {
                            println!("    dma-reg write {a:#x} <- {v:#x}");
                        }
                    }
                }
            }
            "iords" | "iord" => {
                if let (Some(Expr::Reg(d)), Some(m)) = (expr(0), expr(1)) {
                    if let Expr::Mem { inner, .. } = &m {
                        let a = self.eval(inner);
                        let v = self.io_read(a);
                        self.set_reg(&d, v);
                    }
                }
            }
            "lcall" => {
                if let Some((_, tgt)) = insn.targets().first() {
                    self.push(next);
                    self.pc = *tgt as u32;
                    jumped = true;
                }
            }
            "call" => {
                if let Some((_, tgt)) = insn.targets().first() {
                    self.push(next);
                    self.pc = *tgt as u32;
                    jumped = true;
                }
            }
            "lbra" | "bra" => {
                // forms: lbra abs | bra pcrel-cond | bra abs | bra $pN tgt |
                // bra sz $rX imm cond tgt (register-compare; the cond tag
                // rides on the TARGET operand's mods)
                let mut all_mods: Vec<(String, bool)> = Vec::new();
                let mut cmp: Vec<u32> = Vec::new();
                for op in &insn.operands {
                    all_mods.extend(op.mods.iter().cloned());
                    if op.special == Special::None {
                        for e in &op.exprs {
                            if let Expr::Reg(_) = e {
                                // reg operand: value comes from the machine
                            } else {
                                cmp.push(self.eval(e));
                            }
                        }
                    }
                }
                let regs = self.reg_nums(&insn);
                let rval = regs.first().and_then(|&i| self.regs.get(i as usize).copied());
                if !cmp.is_empty() {
                    if let Some(rv) = rval {
                        cmp.insert(0, rv);
                    }
                }
                if let Some(&(_, tgt)) = insn.targets().first() {
                    if name == "lbra" {
                        self.pc = tgt as u32;
                        jumped = true;
                    } else if !cmp.is_empty() && rval.is_some() {
                        // register-compare form: bra sz $rX imm cond tgt —
                        // takes precedence over the flag-condition reading
                        // of the same tags ("ne" here compares $rX with the
                        // immediate, not the flags).
                        let c = all_mods.iter().find_map(|(m, _)| match m.as_str() {
                            "e" => Some(cmp[0] == cmp[1]),
                            "ne" => Some(cmp[0] != cmp[1]),
                            _ => None,
                        });
                        if c == Some(true) {
                            self.pc = tgt as u32;
                            jumped = true;
                        }
                    } else if let Some(c) = self.cond(&all_mods, &cmp) {
                        if c {
                            self.pc = tgt as u32;
                            jumped = true;
                        }
                    } else if all_mods.is_empty() {
                        // unconditional
                        self.pc = tgt as u32;
                        jumped = true;
                    } else {
                        return Err(format!("unresolved bra cond at {:#x}: {}", self.pc, insn.render()));
                    }
                } else if let Some(e0) = expr(0) {
                    if matches!(e0, Expr::Reg(_)) {
                        // bra $rN — register-indirect jump
                        let t = self.eval(&e0);
                        self.pc = t;
                        jumped = true;
                    } else {
                        return Err(format!("bra without target at {:#x}: {}", self.pc, insn.render()));
                    }
                } else {
                    return Err(format!("bra without target at {:#x}: {}", self.pc, insn.render()));
                }
            }
            "ret" | "iret" => {
                self.pc = self.pop();
                jumped = true;
            }
            "exit" => {
                self.halted = true;
                self.exit_reason = "exit".into();
                return Ok(());
            }
            "push" => {
                let v = expr(0).map(|e| self.eval(&e)).unwrap_or(0);
                self.push(v);
            }
            "pop" => {
                if let Some(Expr::Reg(d)) = expr(0) {
                    let v = self.pop();
                    self.set_reg(&d, v);
                }
            }
            "mpush" => {
                // push $r0..$rN (the register NUMBER names the top of the
                // saved range). Verified against three canary-guarded
                // functions (main/0x1b3b/0x8235): only the range reading
                // balances prologue vs mpopaddret epilogue for all of them.
                if let Some(Expr::Reg(d)) = expr(0) {
                    if let Some(n) = d.strip_prefix('r').and_then(|s| s.parse::<usize>().ok()) {
                        for i in 0..=n.min(15) {
                            self.push(self.regs[i]);
                        }
                    }
                }
            }
            "mpop" | "mpopret" | "mpopadd" | "mpopaddret" => {
                // Stack arithmetic only: sp += 4*(N+1) [the saved range],
                // + imm, and for the *ret forms pc = [sp] (sp += 4). The
                // register restores are deliberately NOT modeled: the
                // epilogue pop must not clobber $r1, which carries
                // 0x8235's IO-read return value to the caller's loop-exit
                // condition — register values are otherwise unobservable
                // by the geometry derivation.
                if let Some(Expr::Reg(d)) = expr(0) {
                    let n = d.strip_prefix('r').and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
                    self.sp = self.sp.wrapping_add(4 * (n as u32 + 1));
                    if let Some(w) = imm(1) {
                        self.sp = self.sp.wrapping_add(w);
                    }
                    if name == "mpopret" || name == "mpopaddret" {
                        self.pc = self.rd8(self.sp as usize);
                        self.sp = self.sp.wrapping_add(4);
                        jumped = true;
                    }
                } else {
                    return Err(format!("unparsed {} at {:#x}", name, self.pc));
                }
            }
            "xdst" | "xdld" | "xdst2" => {
                // XDMA parameter staging: record the pair; the copy fires
                // on xdwait.
                let a = expr(0).map(|e| self.eval(&e)).unwrap_or(0);
                let b = expr(1).map(|e| self.eval(&e)).unwrap_or(0);
                self.io_writes.push((0xf0 << 16 | a, b)); // trace marker
            }
            "xdwait" | "xdfence" | "xcwait" | "sleep" | "xdwait2" => {
                // DMA completion: perform the pending host->DMEM copy for
                // the payload window if one was staged.
                if !self.dma_copies.is_empty() || self.take_dma() {
                    if self.stop_after_dma && self.last_dma_sp.is_none() {
                        self.last_dma_sp = Some(self.sp);
                        self.halted = true;
                        self.exit_reason = "dma-completed (geometry snapshot point)".into();
                        return Ok(());
                    }
                }
            }
            "trap" => {
                self.halted = true;
                self.exit_reason = "trap".into();
                return Ok(());
            }
            "cxset" | "cmov" | "cxsin" | "cxsout" | "crnd" | "cs0begin" | "cs0exec"
            | "cs1begin" | "cs1exec" | "cchmod" | "cxor" | "cadd" | "cand" | "crev"
            | "cgfmul" | "csecret" | "ckeyreg" | "ckexp" | "ckrexp" | "cenc" | "cdec"
            | "cimov" | "cixsin" | "cixsout" | "cirnd" | "cis0begin" | "cis0exec"
            | "cis1begin" | "cis1exec" | "cichmod" | "cixor" | "ciadd" | "ciand"
            | "cirev" | "cigfmul" | "cisecret" | "cikeyreg" | "cikexp" | "cikrexp"
            | "cienc" | "cidec" | "cisigcmp" | "cisigenc" | "cisigclr" | "csigclr"
            | "csigenc" => {
                // crypt state stubs: no-ops for the stack layout. csigcmp
                // (signature verify) succeeds — the stock signature is
                // valid in this model.
            }
            "ptlb" | "vtlb" | "itlb" | "bset" | "bclr" | "btgl" => {
                // bset/bclr/btgl FLAGS, $rN: flag bit ops (approx)
                if let Some(Expr::Reg(d)) = expr(0) {
                    if d == "$flags" {
                        let bit = expr(1).map(|e| self.eval(&e)).unwrap_or(0);
                        let f = self.flags;
                        self.flags = match name {
                            "bset" => f | (1 << bit),
                            "bclr" => f & !(1 << bit),
                            _ => f ^ (1 << bit),
                        };
                    } else {
                        let a = expr(0).map(|e| self.eval(&e)).unwrap_or(0);
                        let b = expr(1).map(|e| self.eval(&e)).unwrap_or(0);
                        let v = match name {
                            "bset" => a | (1 << (b & 31)),
                            "bclr" => a & !(1 << (b & 31)),
                            _ => a ^ (1 << (b & 31)),
                        };
                        if let Some(Expr::Reg(dd)) = expr(0) {
                            // dst-first forms write the register
                            self.set_reg(&dd, v);
                        }
                    }
                }
            }
            other => {
                return Err(format!("unimplemented insn `{other}` at {:#x}: {}", self.pc, insn.render()));
            }
        }
        let _ = jumped;
        self.pc = if jumped { self.pc } else { next };
        Ok(())
    }

    fn take_dma(&mut self) -> bool {
        // The DMA model: any IO write into the falcon DMA window stages a
        // copy; xdwait completes the most recent one whose destination
        // matches the payload window (DMEM 0x600..).
        let mut fired = false;
        let mut i = 0;
        while i < self.io_writes.len() {
            let (a, v) = self.io_writes[i];
            if (DMA_IO_LO..=DMA_IO_HI).contains(&a) {
                // reconstruct (dst, len) pairs heuristically: the writes in
                // trigger order are src/src-hi/dst/len/cmd; we only need
                // dst=0x600-class copies.
                if v == 0x600 {
                    let len = self
                        .io_writes
                        .iter()
                        .skip(i)
                        .find(|(a2, _)| *a2 == a + 0x14)
                        .map(|(_, v2)| *v2)
                        .unwrap_or(0x10);
                    let n = (len as usize).min(DMEM_SIZE - 0x600);
                    let host = self.host_buffers.first().cloned().unwrap_or_default();
                    for (k, b) in host.iter().take(n).enumerate() {
                        self.dmem[0x600 + k] = *b;
                    }
                    self.dma_copies.push(DmaCopy { dst: 0x600, len: n as u32 });
                    fired = true;
                }
            }
            i += 1;
        }
        fired
    }

    /// Run from the current state. On return, `last_dma_sp` holds the SP
    /// captured right after the first payload-window DMA (the geometry
    /// snapshot), or the run halted/exhausted for another reason.
    pub fn run(&mut self, buf: &[u8], base: u64) {
        while !self.halted && self.steps < self.max_steps {
            self.steps += 1;
            if let Err(e) = self.step_once(buf, base) {
                self.halted = true;
                self.exit_reason = e;
                return;
            }
            if self.stop_after_dma && self.last_dma_sp.is_some() {
                return;
            }
        }
        if !self.halted {
            self.exit_reason = "step budget exhausted".into();
            self.halted = true;
        }
    }
}

fn collect_regs(e: &Expr, out: &mut Vec<u32>) {
    match e {
        Expr::Reg(name) => {
            if let Some(n) = name.strip_prefix('r') {
                if let Ok(i) = n.parse::<u32>() {
                    out.push(i);
                }
            }
        }
        Expr::Add(a, b) | Expr::Mul(a, b) => {
            collect_regs(a, out);
            collect_regs(b, out);
        }
        Expr::Mem { inner, .. } => collect_regs(inner, out),
        _ => {}
    }
}

/// ALU semantics — returns (result, new flags). Approximate flag model:
/// carry/zero/sign; overflow only where trivially computable.
fn alu(name: &str, a: u32, b: u32, _old: u32) -> (u32, u32) {
    let (v, c) = match name {
        "add" | "adc" => {
            let (v, c1) = a.overflowing_add(b);
            let (_v2, c2) = if name == "adc" { a.overflowing_add(b).0.overflowing_add(1) } else { (v, false) };
            (v, c1 || c2)
        }
        "sub" | "sbb" | "cmpu" | "cmp" | "cmps" => (a.wrapping_sub(b), a < b),
        "and" => (a & b, false),
        "or" => (a | b, false),
        "xor" => (a ^ b, false),
        "shl" | "shlc" => (a.wrapping_shl(b & 31), (a << b.min(31)) > 0xffff_ffff_u32 && b > 0),
        "shr" | "shrc" => (a.wrapping_shr(b & 31), a & 1 != 0 && b > 0),
        "sar" => (((a as i32) >> (b & 31)) as u32, false),
        "mulu" => {
            let r = (a as u64 * b as u64) as u32;
            (r, (a as u64 * b as u64) > 0xffff_ffff)
        }
        "muls" => (((a as i32).wrapping_mul(b as i32)) as u32, false),
        "sext" => (((a as i8) as i32) as u32, false),
        "xbit" => (((a >> (b & 31)) & 1), false),
        "div" => (if b == 0 { 0 } else { a / b }, false),
        "mod" => (if b == 0 { 0 } else { a % b }, false),
        "extr" | "extrs" | "ins" => (a, false),
        _ => (a, false),
    };
    let z = (v == 0) as u32;
    let s = (v & 0x8000_0000 != 0) as u32;
    (v, c as u32 | z << 1 | s << 2)
}
