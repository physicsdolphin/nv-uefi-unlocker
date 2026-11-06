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
    /// secure-service mailbox state (IO 0xd000-0xd500, handshake derived
    /// from the booter's own request functions 0x900/0x9a1/0xa54):
    ///   submit  IO[0xd100] <- id | 0x10000        (read-type request)
    ///   ready   IO[0xd000] bits 0x1c0000 set      (polled before submit)
    ///   doorbel IO[0xd400].low != 0               (response posted)
    ///   ack     IO[0xd400] <- 0x100000
    ///   error   IO[0xd000] bits 0xc00000 clear    (checked after ack)
    ///   value   IO[0xd500]                        (the response word)
    /// write-type calls (no 0x10000 bit) write IO[0xd200] <- value then
    /// IO[0xd100] <- id, and need no completion wait.
    pub param_pending: bool,
    pub param_id: u32,
    /// per-id response values (seeded: what the SEC2 service would answer)
    pub param_values: Vec<(u32, u32)>,
    /// fuse row select (written to IO 0x1c100; data read at 0x1c200)
    pub fuse_row: u32,
    /// per-row fuse values (seeded: production-card state)
    pub fuse_rows: Vec<(u32, u32)>,
    /// the write-type payload (IO 0xd200) awaiting the 0xd100 submit
    pub last_d200: u32,
    /// XDMA state: the page-base register (set by `mov $xdbase`) and the
    /// staged chunks awaiting xdwait
    pub xdbase: u32,
    pub xd_pending: Vec<(bool, u32, u32)>,
    /// optional execution breakpoint (stop before executing this PC)
    pub break_pc: Option<u32>,
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
            param_pending: false,
            param_id: 0,
            param_values: Vec::new(),
            fuse_row: 0,
            fuse_rows: Vec::new(),
            last_d200: 0,
            xdbase: 0,
            xd_pending: Vec::new(),
            break_pc: None,
        }
    }

    pub fn seed_meta_sig_len(&mut self, len: u32) {
        // WprMeta at DMEM 0x400 (nlzy table): the loader-shaped meta
        // (tx_booter.rs wpr_meta_pack) with a self-consistent TU102
        // 11-GiB (2080 Ti) layout — the booter validates the relationships
        // (frts = wpr_end - frts_offset, image <= boot-gsp, heap =
        // 8MiB + align1M(96KiB*GiB) + 96MiB, and the L2 fuse vs fbSize)
        // before touching the signature DMA. Field offsets per
        // wpr_meta_pack: 88 gspFwRsvdStart, 96 nonWprHeapOffset, 104
        // nonWprHeapSize, 112 wpr_start, 120 heapOffset, 128 heapSize,
        // 136 gspOffset, 144 bootOffset, 152 frtsOffset, 160 frtsSize,
        // 168 wprEnd, 176 fbSize, 184 vgaOffset, 192 fbSize-vgaOffset.
        let mut wr64 = |a: usize, v: u64| self.dmem[a..a + 8].copy_from_slice(&v.to_le_bytes());
        wr64(0x400, 0xdc3a_ae21_371a_60b3); // WPR_META_MAGIC
        wr64(0x408, 1); // version
        wr64(0x410, 0x2_0000_0000); // radix_dma
        wr64(0x418, 0x1c0_0000); // image_size (gsp RM)
        wr64(0x420, 0x2adf_a000); // boot_dma
        wr64(0x428, 0x6000); // boot image_size
        wr64(0x430, 0x1800); // boot code_offset
        wr64(0x438, 0x800); // boot data_offset
        wr64(0x440, 0x0); // boot manifest_offset
        wr64(0x448, 0x1e8_0000); // signature_dma (sysmem)
        wr64(0x450, len as u64); // signature_size
        wr64(0x458, 0x2250_0000); // gspFwRsvdStart
        wr64(0x460, 0x2250_0000); // nonWprHeapOffset
        wr64(0x468, 0x10_0000); // nonWprHeapSize (1 MiB)
        wr64(0x470, 0x2260_0000); // wpr_start
        wr64(0x478, 0x2270_0000); // heapOffset
        wr64(0x480, 0x6a0_0000); // heapSize (106 MiB, 11 GiB formula)
        wr64(0x488, 0x291f_a000); // gspOffset
        wr64(0x490, 0x2adf_a000); // bootOffset
        wr64(0x498, 0x2ae0_0000); // frtsOffset
        wr64(0x4a0, 0x10_0000); // frtsSize (= wprEnd - frts, the 1-MiB FRTS)
        wr64(0x4a8, 0x2af0_0000); // wprEnd
        wr64(0x4b0, 0x2c00_0000); // fbSize (11 GiB, TU102)
        wr64(0x4b8, 0x2af0_0000); // vgaOffset
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
        // Fuse block (falcon IO 0x1c100 = row addr, 0x1c000 = cmd/trigger,
        // 0x1c200 = row data; reader = 0xf27). Row 0xa00 = status word:
        // bits 20..28 must read one of the valid fuse states {0x162,
        // 0x164, 0x166} (check at 0x77d2; anything else reports status
        // 0xf at 0x77e0). Row 0x21434 = the secure-debug disable fuse:
        // bit0 = 1 on production (check at 0x11fb, failure = 0x6e).
        self.fuse_rows.push((0xa00, 0x1620_0000));
        self.fuse_rows.push((0x21434, 1));
        // 0x118128: bit0 = 1 required (check at 0x114e; failure = 0x74)
        self.fuse_rows.push((0x118128, 1));
        // 0x118234: low 8 bits = 0xff required (check at 0x115c)
        self.fuse_rows.push((0x118234, 0xff));
        // The PRI-read service rows (reader 0xf27 reads the row through
        // IO 0x1c200): the live WPR2 window registers. 0x1fa81c/0x1fa820
        // = the FWSEC FRTS bounds (read1 > read2 required, check at
        // 0x29c3; failure = status 0x5). 0x1fa824/0x1fa828 = the FWSEC
        // 1-MiB WPR2 window in 256-byte units: (HI-LO) must be 0xE00
        // ((HI-LO)<<8 + 0x20000 = exactly 1 MiB, check at 0x2a72; failure
        // = the classic status 0x91). Values follow this meta's frts
        // (0x2ae00000). Row 0x11821c bit17 = WPR2-established flag
        // (0x2a34; clear = 0x91 outright).
        self.fuse_rows.push((0x1fa81c, 0x2af0_0000));
        self.fuse_rows.push((0x1fa820, 0x2ae0_0000));
        self.fuse_rows.push((0x1fa824, 0x2ae0_00));
        self.fuse_rows.push((0x1fa828, 0x2aee_00));
        self.fuse_rows.push((0x11821c, 0x2_0000));
        // 0x100ce0 = the L2 configuration fuse (triple-read stability
        // pattern at 0x2091; the booter computes mask = (bits4..9) <<
        // (low nibble) and requires (mask64 << 20) == meta fbSize —
        // check at 0x214f, failure = status 0x73). TU102 11-GiB:
        // 11 << 6 = 0x2c0, <<20 = 0x2c000000 ✓. Low nibble 6, field 0xb.
        self.fuse_rows.push((0x100ce0, 0xb6));
        // 0x20800/0x20804: both need low nibble 8, bits 4..7 = 8, bits
        // 8..9 = 0x300 (checks at 0x1831-0x1862; failure = status 0x1b).
        self.fuse_rows.push((0x20800, 0x388));
        self.fuse_rows.push((0x20804, 0x388));
        // IO 0x4f00 (host +0x13c): a security-state word — bits 8..15 must
        // read 1 (check at 0x123b; anything else reports status 0x32).
        self.io_reads.push((0x4f00, 0x100));
        // Secure-service mailbox responses (ids queried by this booter):
        // 0x114 = capability word, must read 1 (checked at 0x8b5);
        // 0x3f04 = FWSEC status word: (v & 0x304) == 0x300 gates the
        //          wait loop at 0x7a4, bits16..18 ∈ {0,4} at 0x7b0, bit30
        //          clear at 0x7bd, non-negative at 0x7c3;
        // 0x104  = FWSEC product-boot done flag — its bit0 IS the wait
        //          loop's exit condition (0x768 returns `value & 1` of the
        //          SECOND query, 0x7ec; 0x879 spins until it reads 1);
        // 0x3f14 = completion poll (0x706): bit0 == 1 exits the loop.
        self.param_values.push((0x114, 1));
        self.param_values.push((0x3f04, 0x300));
        self.param_values.push((0x104, 1));
        self.param_values.push((0x3f14, 1));
        // 0x8e04 = the expected-values table header for stage 0 (query at
        // 0xb75: id = 0x8e04 + stage*0x70): the low byte is the entry
        // count — must be 1..0xfe (0x00/0xff report status 0x1a at 0xc04).
        self.param_values.push((0x8e04, 1));

        // The loader-provided boot descriptor at DMEM 0x200 (the NS stub
        // snapshots mailboxes there; "any non-zero skips the default fill").
        self.wr8(0x200, 0x1);
        self.wr8(0x204, 0x400);
        self.wr8(0x208, 0);
        self.wr8(0x20c, 0);
        // The descriptor block at DMEM 0x600 (validated by the block-mover
        // 0x2d5e): header u16@+2 = 2 (0x2d78; failure = 0x1d), u32@+0x10
        // <= 0x200 (0x2e59). The five enables live at block+0x880..0x890
        // (the mover's size classes 0xa-0xe copy them out one by one at
        // 0x30b0-0x3110; each must read 1, checked at 0x3125 — failure =
        // 0xb). Block+0x878 = the expected-values entry count (<= 0xb,
        // 0x2f54); block+0x820.. = the table itself.
        self.wr8(0x602, 2);
        self.wr8(0x610, 0x10);
        for a in [0xe14u32, 0xe16, 0xe18, 0xe1a, 0xe1c] {
            self.wr8(a as usize, 1);
        }
        for a in [0xe80u32, 0xe84, 0xe88, 0xe8c, 0xe90] {
            self.wr8(a as usize, 1);
        }
        self.wr8(0xe78, 1);
        self.wr8(0xe20, 1);
        // The canary seed: the SEC scans D[0x6130..0x6140) (the crypt
        // context's key slots, right after `cxset 0x0`) for the first
        // non-zero dword and adopts it as the canary (stored to D[0x6140]
        // at 0x4ca). Our crypt stubs are no-ops, so the seed area must be
        // planted; any non-zero value is self-consistent (the store and
        // every canary check use the same one).
        self.wr8(0x6130, 0xd2408);
    }

    fn rd8(&self, a: usize) -> u32 {
        let v = if a + 4 <= self.dmem.len() {
            u32::from_le_bytes(self.dmem[a..a + 4].try_into().unwrap())
        } else {
            0
        };
        v
    }

    fn rd_width(&self, a: usize, w: u8) -> u32 {
        let v = self.rd8(a);
        match w {
            1 => v & 0xff,
            2 => v & 0xffff,
            _ => v,
        }
    }

    fn wr8(&mut self, a: usize, v: u32) {
        self.wr_width(a, v, 4);
    }

    fn wr_width(&mut self, a: usize, v: u32, w: u8) {
        if self.trace && self.watch_canary && a == 0x6140 {
            println!("    [canary-watch] step {} D[0x6140] <- {v:#x} at pc={:#x}", self.steps, self.pc);
        }
        let n = match w {
            1 => 1,
            2 => 2,
            _ => 4,
        };
        if a + n <= self.dmem.len() {
            let bytes = v.to_le_bytes();
            self.dmem[a..a + n].copy_from_slice(&bytes[..n]);
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
        // secure-service mailbox (see the struct doc): ready bits always
        // set, error bits clear; the doorbell follows the pending flag;
        // the response value is looked up per the last query id.
        match addr {
            0xd000 => return 0x1c0000,
            0xd400 => return self.param_pending as u32,
            0xd500 => {
                return self
                    .param_values
                    .iter()
                    .find(|(id, _)| *id == self.param_id)
                    .map(|(_, v)| *v)
                    .unwrap_or(0);
            }
            // fuse row data: the row selected by the last 0x1c100 write
            0x1c200 => {
                return self
                    .fuse_rows
                    .iter()
                    .find(|(row, _)| *row == self.fuse_row)
                    .map(|(_, v)| *v)
                    .unwrap_or(0);
            }
            _ => {}
        }
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
        if name == "xdbase" {
            self.xdbase = v;
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
        if self.break_pc == Some(self.pc) {
            self.halted = true;
            self.exit_reason = format!("breakpoint at {:#x}", self.pc);
            return Ok(());
        }
        let off = (self.pc as u64).checked_sub(base).ok_or("pc below base")? as usize;
        if off >= buf.len() {
            self.halted = true;
            self.exit_reason = format!("pc ran off the image at {:#x}", self.pc);
            return Ok(());
        }
        let insn = decode_one(buf, off, base, FUC5_CRYPT);
        let next = self.pc + insn.oplen as u32;
        if self.trace {
            println!("[{:>8}] {:06x} {}  [r0={:#x} r2={:#x} r9={:#x} r10={:#x} r12={:#x} r13={:#x} r14={:#x} sp={:#x}]", self.steps, self.pc, insn.render(), self.regs[0], self.regs[2], self.regs[9], self.regs[10], self.regs[12], self.regs[13], self.regs[14], self.sp);
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
                        // sethi sets bits 31:16 and PRESERVES the low half
                        // (the block-mover's enable extraction at 0x2ea6
                        // relies on it: ld b16 then sethi $r9 0x0 keeps the
                        // loaded value in bits 15:0)
                        "sethi" => {
                            let old = self.reg_by_name(&d);
                            (imm(1).unwrap_or(0) << 16) | (old & 0xffff)
                        }
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
                // bit-field forms take a lo..hi range as their third
                // operand — handle before the generic alu path (the range
                // encodes as a VecRange, not a number)
                if matches!(name, "extr" | "extrs" | "ins") {
                    if let (Some(Expr::Reg(d)), Some(src), Some(Expr::VecRange(lo, hi))) =
                        (expr(0), expr(1).map(|e| self.eval(&e)), expr(2))
                    {
                        let (lo, hi) = (lo as u32, hi as u32);
                        let width = hi - lo + 1;
                        let mask = if width >= 32 { u32::MAX } else { (1u32 << width) - 1 };
                        let v = match name {
                            "extr" => (src >> lo) & mask,
                            "extrs" => {
                                let x = (src >> lo) & mask;
                                if width < 32 && (x >> (width - 1)) & 1 != 0 {
                                    x | !mask
                                } else {
                                    x
                                }
                            }
                            // ins dst, src, lo..hi: splice src's low width
                            // bits into dst at lo
                            _ => {
                                let dv = self.reg_by_name(&d);
                                (dv & !(mask << lo)) | ((src & mask) << lo)
                            }
                        };
                        self.set_reg(&d, v);
                        self.pc = next;
                        return Ok(());
                    }
                    return Err(format!("unparsed {} at {:#x}: {}", name, self.pc, insn.render()));
                }
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
                // ld dst, [mem] — the b8/b16/b32 size mod picks the width
                // (zero-extended); getting this wrong corrupts neighbours:
                // a st b8 that writes 4 bytes clobbered the validator's
                // frts copy at 0xff64 (the phantom 0x8d hunt, 2026-11-07).
                if let (Some(Expr::Reg(d)), Some(m)) = (expr(0), expr(1)) {
                    if let Expr::Mem { inner, .. } = &m {
                        let a = self.eval(inner) as usize;
                        let w = mem_width(&insn.operands[0].mods);
                        let v = self.rd_width(a, w);
                        self.set_reg(&d, v);
                    } else {
                        return Err(format!("ld non-mem at {:#x}", self.pc));
                    }
                } else {
                    return Err(format!("unparsed ld at {:#x}: {}", self.pc, insn.render()));
                }
            }
            "st" => {
                // st [mem], src — width per the size mod on the mem operand
                if let (Some(m), src) = (expr(0), expr(1)) {
                    if let Expr::Mem { inner, .. } = &m {
                        let a = self.eval(inner) as usize;
                        let v = src.as_ref().map(|e| self.eval(e)).unwrap_or(0);
                        let w = mem_width(&insn.operands[0].mods);
                        self.wr_width(a, v, w);
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
                        if a == 0xd100 {
                            self.param_id = v & 0xffff;
                            if v & 0x10000 != 0 {
                                self.param_pending = true;
                            } else if let Some(slot) = self
                                .param_values
                                .iter_mut()
                                .find(|(id, _)| *id == self.param_id)
                            {
                                slot.1 = self.last_d200;
                            } else {
                                self.param_values.push((self.param_id, self.last_d200));
                            }
                        } else if a == 0xd400 {
                            self.param_pending = false; // the ack
                        } else if a == 0xd200 {
                            self.last_d200 = v; // the write-type payload
                        } else if a == 0x1c100 {
                            self.fuse_row = v; // fuse row select
                        }
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
                // Full restore: pop $rN..$r0 (reverse of the ascending
                // range-push), sp += imm [the prologue's canary slot], and
                // for the *ret forms pc = [sp]. The return value travels in
                // $r10 (moved there BEFORE the pop by every epilogue, e.g.
                // 0x8235's `mov $r10 $r1` at 0x82ca), so the restore of
                // r0/r1 is caller-visible contract, not clobbering: 0x900's
                // spin reads r1 = the restored 0x1c0000 mask, and 0x80f
                // re-reads its saved arg1 from r0 after 0x768's GetParam
                // calls.
                if let Some(Expr::Reg(d)) = expr(0) {
                    let n = d.strip_prefix('r').and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
                    for i in (0..=n.min(15)).rev() {
                        self.regs[i] = self.pop();
                    }
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
                // XDMA chunk staging (driver 0x8070): each instruction = one
                // 0x100-byte chunk. op0 = the offset within the $xdbase
                // page, op1 = (aperture_tag << 16) | source offset. xdld =
                // sysmem->DMEM (the signature/RM-image loads), xdst =
                // DMEM->sysmem (the radix build). The copy fires on xdwait.
                let a = expr(0).map(|e| self.eval(&e)).unwrap_or(0);
                let b = expr(1).map(|e| self.eval(&e)).unwrap_or(0);
                self.xd_pending.push((name == "xdld", a, b));
                self.io_writes.push((0xf0 << 16 | a, b)); // trace marker
            }
            "xdwait" | "xdfence" | "xcwait" | "sleep" | "xdwait2" => {
                // DMA completion: fire the staged XDMA chunks (0x100 bytes
                // each, dst = (xdbase&0xffff)<<8 | offset), then the legacy
                // falcon-DMA-window model. The geometry snapshot point =
                // the first chunk whose destination reaches the stack
                // region (0xff00+) — that is the moment the payload
                // overflows into the live frames.
                let mut touched_stack = false;
                let staged = std::mem::take(&mut self.xd_pending);
                // the signature DMA = the load into the 0x600 page (xdbase
                // = 0x600>>8 = 6); other XDMA users (the meta validator's
                // descriptors, the radix build) must not trigger the
                // snapshot
                let sig_page = self.xdbase & 0xffff == 6;
                for (is_load, off, _packed) in &staged {
                    let dst = ((self.xdbase & 0xffff) << 8) + off;
                    if *is_load {
                        let host = self.host_buffers.first().cloned().unwrap_or_default();
                        for k in 0..0x100usize {
                            let b = host.get((off + k as u32) as usize % host.len().max(1))
                                .copied()
                                .unwrap_or(0);
                            self.wr8((dst + k as u32) as usize, b as u32);
                        }
                        self.dma_copies.push(DmaCopy { dst, len: 0x100 });
                    }
                    if sig_page && *is_load && dst + 0x100 > 0xff00 {
                        touched_stack = true;
                    }
                }
                if touched_stack && self.stop_after_dma && self.last_dma_sp.is_none() {
                    self.last_dma_sp = Some(self.sp);
                    self.halted = true;
                    self.exit_reason = "dma reached the stack (geometry snapshot point)".into();
                    return Ok(());
                }
                let _ = self.take_dma();
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
        "add" => {
            let (v, c) = a.overflowing_add(b);
            (v, c)
        }
        // adc/sbb consume the incoming carry (flags bit0) — the booter's
        // 64-bit arithmetic (wprEnd-1MiB, the window deltas) rides on it
        "adc" => {
            let (t, c1) = a.overflowing_add(b);
            let (v, c2) = t.overflowing_add(_old & 1);
            (v, c1 || c2)
        }
        "sbb" => {
            let (t, c1) = a.overflowing_sub(b);
            let (v, c2) = t.overflowing_sub(_old & 1);
            (v, c1 || c2)
        }
        "sub" | "cmpu" | "cmp" | "cmps" => (a.wrapping_sub(b), a < b),
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

/// Mem access width from the size mod on the mem operand (b8/b16/b32;
/// the default render without a mod is b32).
fn mem_width(mods: &[(String, bool)]) -> u8 {
    for (m, _) in mods {
        match m.as_str() {
            "b8" => return 1,
            "b16" => return 2,
            "b32" => return 4,
            _ => {}
        }
    }
    4
}
