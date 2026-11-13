//! Shared code for the two delivery forms:
//!
//! - the USB `BOOTX64.EFI` application (`main.rs`) - the default, one-shot,
//!   chain-loads Windows Boot Manager itself;
//! - the firmware-resident DXE driver (`dxe_main.rs`) - advanced users flash
//!   it into the firmware as an FFS file (`ffspack`); it fires at ReadyToBoot
//!   and never chain-loads (see `dxe` and docs/dxe-driver-ffs-feasibility.md).
//!
//! The transaction machinery is shared verbatim; the forms differ in entry
//! sequencing, volume resolution (`volumes`) and the chain-load step.
//! The post-CONFIRM phase sequence (and the transaction itself) lives in
//! `runner`; the USB application's pre-confirm sequencing stays in `main.rs`
//! because the driver form has its own authorization gates instead.

#![no_std]

extern crate alloc;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build-identity tag of the DXE driver form (audit F4): referenced by the
/// driver's own log output, so it survives LTO into .rdata, and REQUIRED by
/// ffspack before it retypes any image - the USB application never contains
/// this string, so an application image cannot be packed as a driver.
pub const DXE_BUILD_ID: &str = "nv-uefi-unlocker DXE driver form 1";

/// \tx-ultrafast.txt: verified-machine speed profile. Skips every purely
/// diagnostic pre/post probe (M1 sample, GOP probe, AUTH-SCAN, ROOT-DIR
/// dump, opener/topology dumps, post-payload FEAT sweep, HANDOFF-PCI /
/// HANDOFF-VGA-FINAL dumps, POST-GOP probe, CHAIN-MEMMAP) and folds in
/// quiet + no-canary + stage-marker-only flush (an explicit
/// \tx-flush-lines.txt still wins). NOT skippable and always kept: the
/// verdict readbacks - selector write/readback, WPR2/POST-BOOT verdicts,
/// booter load audit - because readback is the only success criterion
/// (constitution #1).
///
/// Lives at the lib crate root: every diagnostic-probe site across the
/// shared modules reads it via `crate::ultrafast()`, and both delivery
/// forms set it from their own volume probes before the logger starts.
static ULTRAFAST: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Read by every diagnostic-probe site across the crate; set once before
/// the logger exists (plain marker probe, same shape as the log-commit
/// ladder).
pub fn ultrafast() -> bool {
    ULTRAFAST.load(core::sync::atomic::Ordering::Relaxed)
}

pub fn set_ultrafast(v: bool) {
    ULTRAFAST.store(v, core::sync::atomic::Ordering::Relaxed);
}

pub mod acpi_dump;
pub mod banner;
// btl1_signature.rs is included via #[path] by tx_booter (and sim tests);
// a lib-level module would be a dead third copy.
pub mod chainload;
pub mod dxe;
pub mod dxe_policy;
pub mod gop;
pub mod identity;
pub mod identity_core;
pub mod keywait;
pub mod logging;
pub mod pci;
pub mod runner;
pub mod sample;
pub mod timing;
pub mod usbreset;
pub mod volumes;
#[cfg(feature = "tx")]
pub mod claim_plm;
#[cfg(feature = "tx")]
pub mod gop_native;
#[cfg(feature = "tx")]
pub mod gop_order;
#[cfg(feature = "tx")]
pub mod tx;
#[cfg(feature = "tx")]
pub mod tx_booter;
#[cfg(feature = "tx")]
pub mod tx_detach;
#[cfg(feature = "tx")]
pub mod tx_fwsec;
#[cfg(feature = "tx")]
pub mod tx_policy;
#[cfg(feature = "tx")]
pub mod tx_portal;
#[cfg(feature = "tx")]
pub mod tx_prep;
/// Crate-root re-export: `tx_prep` (and the transaction surface) resolve
/// the die family via `crate::asset_family`; the mapping itself lives in
/// `runner` next to the transaction phase sequence it serves.
#[cfg(feature = "tx")]
pub use runner::asset_family;
