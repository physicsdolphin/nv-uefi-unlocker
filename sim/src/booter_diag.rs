//! TU10x booter forensics decode tables (branch diag/tu10x-forensics).
//!
//! Host-testable copy of the runtime dictionaries in
//! `loader/src/tx_booter.rs` (loader is a UEFI-target crate and excluded
//! from `cargo xt`). The tables come from:
//! - EXCI bitfields: open-gpu-kernel-modules `dev_soe_ip.h`
//!   (NV_SOE_FALCON_EXCI, Falcon IP common) - EXPC[19:0], EXCAUSE[24:20].
//! - mailbox0 dictionary: the NS-stub disassembly of the pinned
//!   `booter_load_tu10x.bin` (docs/tu10x-booter-nsstub-disasm.md) plus the
//!   field correlation (issue #62 healthy run shows 0x31 live then 0x0
//!   final; PR #67 D/E rounds end at 0x31 with a broken secure entry).
//!
//! KEEP IN SYNC with loader/src/tx_booter.rs when the dictionary grows.

/// EXCAUSE[24:20] value -> authoritative name.
pub fn exci_cause_name(cause: u32) -> &'static str {
    match cause {
        0..=3 => "TRAPn",
        0x08 => "ILL_INS",
        0x09 => "INV_INS",
        0x0a => "MISS_INS",
        0x0b => "DHIT_INS",
        0x0d => "SP_OVERFLOW",
        0x0f => "BRKPT_INS",
        0x10 => "DMEM_MISS_INS",
        0x11 => "DMEM_DHIT_INS",
        0x12 => "DMEM_PAFAULT_INS",
        0x13 => "DMEM_PERMISSION_INS",
        0x15 => "BROM_CALL_INS",
        0x16 => "KMEM_VIOLATION_INS",
        0x17 => "BMEM_PERMISSION_INS",
        _ => "unknown",
    }
}

/// Split an EXCI read into (EXPC[19:0], EXCAUSE[24:20], name).
pub fn exci_decode(exci: u32) -> (u32, u32, &'static str) {
    let expc = exci & 0x000f_ffff;
    let cause = (exci >> 20) & 0x1f;
    (expc, cause, exci_cause_name(cause))
}

/// Post-halt mailbox0 classification (see module docs).
pub fn mailbox0_verdict(mbox0: u32) -> &'static str {
    match mbox0 {
        0x31 => "NS-stub pre-secure marker (final) - the secure entry never executed (placement/entry-level failure)",
        0x0 => "secure-stage success status",
        0x89..=0x90 => "wpr_desc_validate rejection (meta DMA readback mismatch)",
        0x91 => "secure-stage rejection family - the secure code RAN and rejected (cmp50hx dictionary)",
        0x11 | 0x0b => "secure-stage early status (ran, rejected before validation)",
        _ => "input-carrier residue or unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The failure-family signature (issues #17/#57/#62/ProxVirtualZone):
    /// EXPC stuck at the secure entry 0x100, cause 9 = INV_INS - NOT the
    /// previously annotated 0x10/DMEM_MISS_INS.
    #[test]
    fn exci_00900100_is_inv_ins_at_0x100() {
        assert_eq!(exci_decode(0x0090_0100), (0x100, 0x09, "INV_INS"));
    }

    /// PR #67 D/E rounds (sec base moved to 0x200): same PC, cause 0x0a.
    #[test]
    fn exci_00a00100_is_miss_ins() {
        assert_eq!(exci_decode(0x00a0_0100), (0x100, 0x0a, "MISS_INS"));
    }

    /// PR #67 F round (secure=false write): EXPC ran off into 0xa2863.
    #[test]
    fn exci_50aa2863_expc_runs_off() {
        let (expc, cause, _) = exci_decode(0x50aa_2863);
        assert_eq!(expc, 0xa_2863);
        assert_eq!(cause, 0x0a);
    }

    /// TU106 healthy BOOTER-DIRECT halt: exci 0x0.
    #[test]
    fn exci_zero_decodes_cleanly() {
        assert_eq!(exci_decode(0), (0, 0, "TRAPn"));
    }

    #[test]
    fn mailbox0_dictionary_field_values() {
        // 0x31: stub marker stuck = entry-level failure (#62 D/E rounds).
        assert!(mailbox0_verdict(0x31).contains("never executed"));
        // 0x91: the hard-core family final value - secure ran and rejected.
        assert!(mailbox0_verdict(0x91).contains("RAN and rejected"));
        // 0x11 (TITAN RTX >4GB round) / 0x0B (ProxVirtualZone): secure-stage
        // early statuses, both must classify as "ran".
        assert!(mailbox0_verdict(0x11).contains("ran"));
        assert!(mailbox0_verdict(0x0b).contains("ran"));
        // 0x0: success (travis guarded attempt-003).
        assert!(mailbox0_verdict(0x0).contains("success"));
        // 0x89: the platform-DMA (firmware IOMMU) rejection stays distinct.
        assert!(mailbox0_verdict(0x89).contains("wpr_desc_validate"));
        // Input carrier (a meta address) must not misclassify.
        assert!(mailbox0_verdict(0x1000_1f00).contains("input-carrier"));
    }
}
