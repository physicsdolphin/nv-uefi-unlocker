//! Firmware-resident DXE driver binary (`nvunlock-dxe`).
//!
//! Ships as `NvUnlockDxe.ffs` (built by `ffspack`, which also patches the
//! PE subsystem to EFI_BOOT_SERVICE_DRIVER) for advanced users to insert
//! into their firmware image manually. See docs/dxe-driver-ffs-feasibility.md
//! and `dxe` for the dispatch/gating design.

#![no_main]
#![no_std]

extern crate alloc;

use uefi::prelude::*;

#[entry]
fn dxe_main() -> Status {
    loader::dxe::driver_entry()
}
