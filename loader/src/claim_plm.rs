//! Register-specific interpretation of a transport-aware FEAT PLM read.
//!
//! All ones is the open value for this register, unlike ordinary BAR reads.
//! A failed PCI I/O read must never be treated as an open or locked PLM.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimPlm {
    Open,
    NotOpen,
    Inaccessible,
    ReadFailed,
}

pub fn classify(value: Option<u32>) -> ClaimPlm {
    match value {
        None => ClaimPlm::ReadFailed,
        Some(0xffff_ffff) => ClaimPlm::Open,
        Some(v) if v & 0xffff_0000 == 0xbadf_0000 => ClaimPlm::Inaccessible,
        Some(_) => ClaimPlm::NotOpen,
    }
}
