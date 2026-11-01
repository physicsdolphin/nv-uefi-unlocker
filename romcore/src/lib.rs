//! PCI expansion-ROM parsing shared by every crate in this workspace.
//!
//! Why this crate exists: the ROM/EFI-image grammar is needed twice with
//! byte-identical semantics - host side (assets/ GOP extractor, tests) and
//! device side (loader/src/gop_native.rs, x86_64-unknown-uefi, no_std).
//! A `sim/src/devpath.rs`-style textual mirror does not scale to a stream
//! decoder, so the parser lives here once, no_std + alloc, and every
//! consumer (including the host tests) exercises the same code path.
//!
//! Scope (issue #107 CSM-compat line):
//! - [`walk`]: PCIR image-chain walk (code types 0x00/0x03/0xE0/... with
//!   the NVIDIA NPDE SUBIMAGE_LEN stride override).
//! - [`efirom`]: UEFI-spec 13.4.2 EFI PCI expansion-ROM header + the
//!   {compressed, uncompressed} size pair of the payload at
//!   EfiImageHeaderOffset.
//! Decompression is deliberately NOT implemented here: the firmware's own
//! EFI_DECOMPRESS_PROTOCOL does it at runtime (loader side), so the Rust
//! surface stays parse-only.
//! - [`pe`]: PE/COFF sanity decode (MZ, machine, optional-header magic,
//!   subsystem, SizeOfImage) used to validate a candidate GOP image
//!   before anything is loaded.
//!
//! All parsing is bounds-checked: every offset taken from the image goes
//! through `get` helpers that refuse out-of-range reads.

#![no_std]

#[cfg(test)]
extern crate std;

extern crate alloc;

pub mod efirom;
pub mod pe;
pub mod walk;
