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

#![no_std]

extern crate alloc;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build-identity tag of the DXE driver form (audit F4): referenced by the
/// driver's own log output, so it survives LTO into .rdata, and REQUIRED by
/// ffspack before it retypes any image - the USB application never contains
/// this string, so an application image cannot be packed as a driver.
pub const DXE_BUILD_ID: &str = "nv-uefi-unlocker DXE driver form 1";

pub mod banner;
// btl1_signature.rs is included via #[path] by tx_booter (and sim tests);
// a lib-level module would be a dead third copy.
pub mod chainload;
pub mod dxe;
pub mod gop;
pub mod keywait;
pub mod logging;
pub mod pci;
pub mod runner;
pub mod sample;
pub mod timing;
pub mod volumes;
#[cfg(feature = "tx")]
pub mod claim_plm;
#[cfg(feature = "tx")]
pub mod tx;
#[cfg(feature = "tx")]
pub mod tx_booter;
#[cfg(feature = "tx")]
pub mod tx_detach;
#[cfg(feature = "tx")]
pub mod tx_fwsec;
#[cfg(feature = "tx")]
pub mod tx_portal;
#[cfg(feature = "tx")]
pub mod tx_prep;
