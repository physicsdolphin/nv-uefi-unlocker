//! ROP gadget scanner — the Rust successor of the python prototype
//! (gadget_scan.py). From every byte offset, decode forward until a
//! control-transfer terminator (max budget), accumulate primitive tags
//! along the path. The positional decoder covers mid-entry offsets by
//! construction (no harvested length table to miss encodings) — that is
//! what recovers the mid-entry sites the python version missed.

use crate::decode::decode_one;
use crate::isa::Variant;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tag {
    MmioWrite,
    DmemStore,
    DmemLoad,
    /// A DMEM-range immediate (0x1000..0x10000) — the canary-slot marker.
    DmemImm(u64),
    /// `lbra` targeting the entry itself — the park.
    ParkSelf,
    LBra,
    LCall,
    /// Pop-return class (ret/iret/exit/mpopret/mpopaddret).
    Ret,
}

#[derive(Clone, Debug)]
pub struct Candidate {
    pub va: u64,
    pub len: usize,
    pub text: String,
    pub tags: Vec<Tag>,
}

const MAX_INSNS: usize = 24;

fn path_tags(entry_va: u64, buf: &[u8], off: usize, base: u64, var: Variant) -> Option<Vec<Tag>> {
    let mut tags: Vec<Tag> = Vec::new();
    let mut p = off;
    for _ in 0..MAX_INSNS {
        if p >= buf.len() {
            return None;
        }
        let insn = decode_one(buf, p, base, var);
        // Tolerate bytes the fuc5 table cannot decode: NVIDIA's booter
        // toolchain emits encodings the 2012-era envydis tables lack (the
        // mid-entry gadget streams walk through them; the silicon decodes
        // them fine). Skip and continue — the golden payload tables are
        // the arbiter of whether a candidate is real.
        if insn.is_unknown() {
            p += insn.oplen.max(1);
            continue;
        }
        match insn.name.as_str() {
            "iowr" | "iowrs" => tags.push(Tag::MmioWrite),
            "st" => tags.push(Tag::DmemStore),
            "ld" => tags.push(Tag::DmemLoad),
            "lbra" => tags.push(Tag::LBra),
            "lcall" => tags.push(Tag::LCall),
            _ => {}
        }
        if insn.endmark()
            || matches!(insn.name.as_str(), "ret" | "iret" | "exit" | "mpopret" | "mpopaddret")
        {
            tags.push(Tag::Ret);
        }
        for v in insn.data_imms() {
            if (0x1000..0x10000).contains(&v) {
                tags.push(Tag::DmemImm(v as u64));
            }
        }
        if insn.name == "lbra" {
            for (_, tgt) in insn.targets() {
                if tgt == entry_va as i64 {
                    tags.push(Tag::ParkSelf);
                }
            }
        }
        // Terminators: control leaves this stream (a call returns into
        // the NEXT ROP frame, so it still terminates the gadget path).
        if insn.endmark()
            || matches!(insn.name.as_str(), "ret" | "iret" | "exit" | "mpopret" | "mpopaddret" | "lcall" | "lbra")
        {
            return Some(tags);
        }
        p += insn.oplen;
    }
    None
}

/// Scan every byte offset of `buf` (base = VA of buf[0]) for ROP-interest
/// instruction paths. Unknown forms are skipped. Results keyed by VA.
pub fn scan(buf: &[u8], base: u64, var: Variant) -> BTreeMap<u64, Candidate> {
    let mut out = BTreeMap::new();
    for off in 0..buf.len() {
        let Some(mut tags) = path_tags(base + off as u64, buf, off, base, var) else {
            continue;
        };
        tags.sort();
        tags.dedup();
        if tags.is_empty() {
            continue;
        }
        let insn = decode_one(buf, off, base, var);
        out.insert(
            base + off as u64,
            Candidate { va: base + off as u64, len: insn.oplen, text: insn.render(), tags },
        );
    }
    out
}

/// Check that specific VAs were found and report their tags.
pub fn check_sites(map: &BTreeMap<u64, Candidate>, sites: &[u64]) -> Vec<(u64, Option<Vec<Tag>>)> {
    sites
        .iter()
        .map(|va| (*va, map.get(va).map(|c| c.tags.clone())))
        .collect()
}
