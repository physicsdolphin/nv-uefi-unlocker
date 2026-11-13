//! Opt-in pre-transaction identity gate (not wired up by default - see
//! `IdentityOutcome::NotConfigured`). Measures the SHA-256 of the fixed
//! 1 MiB PROM window at `BAR0+0x300000` (already consumed by FWSEC) and,
//! only if `\tx-prom-sha256.txt` exists on the boot volume, requires it to
//! match before `run_transaction` may be called. The file is per-operator
//! config, not protection against a malicious USB stick, and a matching
//! PCI id + byte-identical PROM on a different physical card is
//! indistinguishable by this scheme - it pins "this exact ROM image",
//! nothing stronger.
use crate::{identity_core, pci::PciIo};
use log::info;
use uefi::Status;

fn read_digest(pci: &PciIo) -> Option<[u8; 32]> {
    // Never enable memory decode just to make an identity read work.
    if pci.config_read_u16(0x04) & 2 == 0 {
        return None;
    }
    let mut words = alloc::vec![0u32; identity_core::PROM_BYTES / 4];
    if pci.bar_read_u32_buf(0, 0x300000, &mut words) != Status::SUCCESS {
        return None;
    }
    if words[0] & 0xffff != 0xaa55 {
        return None;
    }
    // x86_64 UEFI is little endian; allocation is fully initialized, bounded.
    let bytes = unsafe {
        core::slice::from_raw_parts(words.as_ptr().cast::<u8>(), identity_core::PROM_BYTES)
    };
    Some(identity_core::digest(bytes))
}

/// Read-only measurement, usable from the always-shipped SAMPLE phase too
/// (so an operator can read the value to enroll, without the `tx` feature).
pub fn measure(pci: &PciIo) -> Option<[u8; 32]> {
    let digest = identity_core::stable_digest(|| read_digest(pci))?;
    let hex: alloc::string::String = digest.iter().map(|b| alloc::format!("{b:02x}")).collect();
    info!(
        "IDENTITY: PROM offset=0x300000 bytes=1048576 SHA256={hex} stable=true (measurement only)"
    );
    Some(digest)
}

/// Enrollment surface for the SAMPLE phase: measure ONLY when
/// `\tx-prom-sha256.txt` exists. An unconditional measurement would be two
/// full 1 MiB PROM traversals per card per DEFAULT boot - the speed track
/// already measured this axis (the targeted-PROM-read attempt was reverted
/// on exactly this ground, >128 KiB walks net-negative on TU106, issue #38),
/// so default boots must pay zero. Enrollment is then simply: put a
/// placeholder value in the file, boot once (read-only build or
/// tx-select-bdf-pinned card), copy the logged IDENTITY: value back.
pub fn measure_if_enrolled(pci: &PciIo) -> bool {
    if crate::chainload::boot_volume_read(uefi::cstr16!(r"\tx-prom-sha256.txt")).is_none() {
        return false;
    }
    measure(pci).is_some()
}

#[cfg(feature = "tx")]
pub struct VerifiedIdentity<'a> {
    _pci: &'a PciIo,
}

#[cfg(feature = "tx")]
pub enum IdentityOutcome<'a> {
    /// `\tx-prom-sha256.txt` is absent: the gate is not configured for this
    /// run, so the transaction proceeds exactly as it always has (the
    /// field-proven default, unchanged by this feature's presence).
    NotConfigured,
    /// Reference present and matched this card's measured PROM SHA-256.
    Verified(VerifiedIdentity<'a>),
    /// Reference present but unreadable/unparsable, or the measured PROM
    /// was unreadable/unstable, or it did not match. Refuses before any
    /// GPU write - not skip-class, the operator asked for a check and it
    /// failed.
    Refused,
}

#[cfg(feature = "tx")]
pub fn authorize(pci: &PciIo) -> IdentityOutcome<'_> {
    let Some(raw) = crate::chainload::boot_volume_read(uefi::cstr16!(r"\tx-prom-sha256.txt"))
    else {
        return IdentityOutcome::NotConfigured;
    };
    let Some(expected) = identity_core::parse_reference(&raw) else {
        info!("IDENTITY: \\tx-prom-sha256.txt present but unparsable (want 64 hex digits) - REFUSED");
        return IdentityOutcome::Refused;
    };
    let Some(measured) = measure(pci) else {
        info!("IDENTITY: PROM unreadable or unstable across two reads - REFUSED");
        return IdentityOutcome::Refused;
    };
    if !identity_core::verify(expected, measured) {
        info!("IDENTITY: measured PROM SHA256 does not match \\tx-prom-sha256.txt - REFUSED");
        return IdentityOutcome::Refused;
    }
    info!("IDENTITY: pinned PROM SHA256 MATCH - transaction eligible");
    IdentityOutcome::Verified(VerifiedIdentity { _pci: pci })
}
