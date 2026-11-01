//! PCI enumeration via a hand-written raw `EFI_PCI_IO_PROTOCOL` wrapper.
//!
//! uefi 0.26 (latest on crates.io at project start) ships no `PciIo` protocol
//! type, and neither does `uefi_raw`; per task brief §5 we drop to the raw
//! protocol definition. Layout follows UEFI spec 2.x, PCI I/O Protocol.
//!
//! M0 scope: config-space READS only (vendor/device/class + capability walk).
//! There is deliberately no write helper in this file.

use core::ffi::c_void;
#[cfg(feature = "tx")]
use core::time::Duration;
use log::info;
#[cfg(feature = "tx")]
use uefi::boot::AllocateType;
use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams, SearchType};
#[cfg(feature = "tx")]
use uefi::mem::memory_map::MemoryMap as _;
#[cfg(feature = "tx")]
use uefi::mem::memory_map::MemoryType;
use uefi::proto::unsafe_protocol;
use uefi::{Identify, Status};

pub const NVIDIA_VENDOR_ID: u16 = 0x10de;

/// Per-family gate for above-4GB staging (tx.rs sets it for 'tu10x' at the
/// top of each card's transaction; every other family keeps the exact
/// allocation path it was field-verified with). Default OFF.
#[cfg(feature = "tx")]
static TX_HIGH_STAGING: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Enable above-4GB staging for the current card's transaction.
#[cfg(feature = "tx")]
pub fn set_high_staging(on: bool) {
    TX_HIGH_STAGING.store(on, core::sync::atomic::Ordering::Relaxed);
}

/// Current above-4GB staging gate (read back for band decoupling: v2.1.8
/// allocates the FWSEC staging in the LOW band by default - issue #74
/// proved >=4GiB unreachable on TB platforms - and restores the gate for
/// the meta/signature/radix allocations).
#[cfg(feature = "tx")]
pub fn high_staging() -> bool {
    TX_HIGH_STAGING.load(core::sync::atomic::Ordering::Relaxed)
}

#[cfg(feature = "tx")]
static MEMMAP_CAPTURED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Capture the memory map once per boot - every descriptor ending at/above
/// 2 GiB, ALL types, not just conventional: what the firmware did with the
/// high RAM is the open question behind the per-board 0x91 differences
/// (issues #16/#17/#19). Runs before the first allocation so it lands in
/// every log, success or failure.
#[cfg(feature = "tx")]
fn capture_memmap_once() {
    if MEMMAP_CAPTURED.swap(true, core::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let Ok(mmap) = boot::memory_map(MemoryType::BOOT_SERVICES_DATA) else {
        info!("MEMMAP: capture failed");
        return;
    };
    let mut lines = 0u32;
    let mut above4g = 0u32;
    for d in mmap.entries() {
        let end = d.phys_start.saturating_add((d.page_count as u64) * 4096);
        if d.ty == MemoryType::CONVENTIONAL && d.phys_start >= 0x1_0000_0000 {
            above4g += 1;
        }
        if end >= 0x8000_0000 && lines < 32 {
            info!(
                "MEMMAP: {:#x}-{:#x} ({} MiB) type={:?} attr={:?}",
                d.phys_start,
                end,
                ((d.page_count as u64) * 4096) >> 20,
                d.ty,
                d.att
            );
            lines += 1;
        }
    }
    info!("MEMMAP: {above4g} conventional descriptors above 4GB");
}

/// EFI_PCI_IO_PROTOCOL_WIDTH (only the fixed-width variants are named here).
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)] // Uint64 arrives with M1 BAR0 64-bit reads
#[repr(u32)]
pub enum PciWidth {
    Uint8 = 0,
    Uint16,
    Uint32,
    Uint64,
}

/// Safety: `this` must point to a live `PciIoRaw` for the call duration.
type AccessMemFn =
    unsafe extern "efiapi" fn(*mut PciIoRaw, PciWidth, bar_index: u8, offset: u64, count: usize, buffer: *mut c_void) -> Status;
/// Safety: `this` must point to a live `PciIoRaw` for the call duration.
type AccessCfgFn =
    unsafe extern "efiapi" fn(*mut PciIoRaw, PciWidth, offset: u32, count: usize, buffer: *mut c_void) -> Status;

#[repr(C)]
struct MemAccess {
    read: AccessMemFn,
    write: AccessMemFn,
}

#[repr(C)]
struct CfgAccess {
    read: AccessCfgFn,
    write: AccessCfgFn,
}

/// Safety: `this` must point to a live `PciIoRaw` for the call duration.
pub type MapFn = unsafe extern "efiapi" fn(
    *mut PciIoRaw,
    operation: u32, // EFI_PCI_IO_PROTOCOL_OPERATION
    host: *mut c_void,
    bytes: *mut usize,
    device: *mut u64,
    mapping: *mut *mut c_void,
) -> Status;
pub type UnmapFn = unsafe extern "efiapi" fn(*mut PciIoRaw, mapping: *mut c_void) -> Status;
pub type AllocBufFn = unsafe extern "efiapi" fn(
    *mut PciIoRaw,
    alloc_type: u32,   // EFI_ALLOCATE_TYPE
    memory_type: u32,  // EFI_MEMORY_TYPE
    pages: usize,
    host: *mut *mut c_void,
    attributes: u64,
) -> Status;
pub type FreeBufFn = unsafe extern "efiapi" fn(*mut PciIoRaw, pages: usize, host: *mut c_void) -> Status;

#[repr(C)]
pub struct PciIoRaw {
    /// EDK2 MdePkg/Include/Protocol/PciIo.h: the struct opens with TWO poll
    /// fields (PollMem, PollIo). A previous revision had only one, shifting
    /// every slot: `pci.read` landed on `Io.Write` (an unintended write
    /// primitive, rejected by firmware as UNSUPPORTED on BAR-less devices)
    /// and `get_location` landed on `Flush` (SUCCESS + untouched out-params).
    /// Offsets below are pinned by compile-time asserts.
    _poll_mem: *const c_void,
    _poll_io: *const c_void,
    mem: MemAccess,
    _io: MemAccess,
    pci: CfgAccess,
    _copy_mem: *const c_void,
    map: MapFn,
    unmap: UnmapFn,
    allocate_buffer: AllocBufFn,
    free_buffer: FreeBufFn,
    _flush: *const c_void,
    get_location:
        unsafe extern "efiapi" fn(*mut PciIoRaw, *mut usize, *mut usize, *mut usize, *mut usize) -> Status,
    /// EfiPciIoAttributeOperation(Get/Enable/...), arg, arg, status out
    attributes: unsafe extern "efiapi" fn(*mut PciIoRaw, u32, u64, u64, *mut Status) -> Status,
    _get_bar_attributes: *const c_void,
    _set_bar_attributes: *const c_void,
    rom_size: u64,
    rom_image: *const c_void,
}

// Layout pins against EDK2 PciIo.h (pointer = 8 bytes on x86_64 UEFI).
const _: () = assert!(core::mem::offset_of!(PciIoRaw, pci) == 6 * 8); // slots: PollMem,PollIo,Mem{r,w},Io{r,w} -> Pci.Read
const _: () = assert!(core::mem::offset_of!(PciIoRaw, get_location) == 14 * 8);
const _: () = assert!(core::mem::offset_of!(PciIoRaw, attributes) == 15 * 8);
const _: () = assert!(core::mem::offset_of!(PciIoRaw, rom_size) == 18 * 8);
const _: () = assert!(core::mem::size_of::<PciIoRaw>() == 20 * 8);

#[repr(transparent)]
#[unsafe_protocol("4cf5b200-68b8-4ca5-9eec-b23e3f50029a")]
pub struct PciIo(PciIoRaw);

impl PciIo {
    /// Config-space read of `count` scalars of the given width.
    /// Status is returned to the caller; helpers below log failures
    /// (constitution §1: never swallow a read-back result silently).
    fn config_read_raw(&self, width: PciWidth, offset: u32, count: usize, buf: *mut c_void) -> Status {
        // Safety: exclusive-borrowed protocol, buffers outlive the call.
        unsafe { (self.0.pci.read)(&self.0 as *const _ as *mut _, width, offset, count, buf) }
    }

    pub fn config_read_u16(&self, offset: u32) -> u16 {
        let mut v = 0u16;
        let st = self.config_read_raw(PciWidth::Uint16, offset, 1, &mut v as *mut _ as *mut c_void);
        if st != Status::SUCCESS {
            log::error!("cfg_read16({:#06x}) status {:?}", offset, st);
        }
        v
    }

    pub fn config_read_u32(&self, offset: u32) -> u32 {
        let mut v = 0u32;
        let st = self.config_read_raw(PciWidth::Uint32, offset, 1, &mut v as *mut _ as *mut c_void);
        if st != Status::SUCCESS {
            log::error!("cfg_read32({:#06x}) status {:?}", offset, st);
        }
        v
    }

    pub fn config_read_u8(&self, offset: u32) -> u8 {
        let mut v = 0u8;
        let st = self.config_read_raw(PciWidth::Uint8, offset, 1, &mut v as *mut _ as *mut c_void);
        if st != Status::SUCCESS {
            log::error!("cfg_read8({:#06x}) status {:?}", offset, st);
        }
        v
    }

    /// BAR memory read of `count` u32 scalars via `Mem.Read` (M1 sampler's
    /// only window into the GPU: BAR0 MMIO). Status is returned; helpers log.
    fn mem_read_raw(&self, width: PciWidth, bar: u8, offset: u64, count: usize, buf: *mut c_void) -> Status {
        // Safety: exclusive-borrowed protocol, buffers outlive the call.
        unsafe { (self.0.mem.read)(&self.0 as *const _ as *mut _, width, bar, offset, count, buf) }
    }

    /// BAR memory write of `count` u32 scalars via `Mem.Write` (the only
    /// host-to-VRAM bulk path: BAR1 into the framebuffer).
    #[allow(dead_code)] // BAR write path: consumed by the TX transaction only
    fn mem_write_raw(&self, width: PciWidth, bar: u8, offset: u64, count: usize, buf: *mut c_void) -> Status {
        // Safety: exclusive-borrowed protocol; caller owns the source buffer.
        unsafe { (self.0.mem.write)(&self.0 as *const _ as *mut _, width, bar, offset, count, buf) }
    }

    /// Read one u32 from a BAR window (BAR0 for all M1 registers).
    pub fn bar_read_u32(&self, bar: u8, offset: u64) -> u32 {
        let mut v = 0u32;
        let st = self.mem_read_raw(PciWidth::Uint32, bar, offset, 1, &mut v as *mut _ as *mut c_void);
        if st != Status::SUCCESS {
            log::error!("bar_read32(bar {bar}, {offset:#x}) status {st:?}");
        }
        v
    }

    /// Read one u32, returning None when the device rejects the access
    /// (misaligned offset, decode error). Used by sweep loops that must
    /// distinguish "read failed" from "value zero".
    pub fn bar_try_read_u32(&self, bar: u8, offset: u64) -> Option<u32> {
        let mut v = 0u32;
        let st = self.mem_read_raw(PciWidth::Uint32, bar, offset, 1, &mut v as *mut _ as *mut c_void);
        if st != Status::SUCCESS {
            log::error!("bar_read32(bar {bar}, {offset:#x}) status {st:?}");
            return None;
        }
        Some(v)
    }

    /// PCI attributes enable (tx builds: TOTORONG ClaimRestore step).
    ///
    /// NOTE: op 1 is EfiPciIoAttributeOperation**Set** in the EDK2 enum
    /// (Get=0, Set=1, Enable=2) - this call SETS the whole attribute word
    /// to `mask`, which is why the historical call sites pass exactly
    /// 0x0007. Kept as-is (behavior verified on hardware across the whole
    /// transaction history); new code uses [`Self::attributes_enable_op`].
    #[cfg(feature = "tx")]

    #[cfg(feature = "tx")]
    pub fn attributes_enable(&self, mask: u64) {
        // op 1 = EfiPciIoAttributeOperationSet; Arg1 = mask
        let mut st = Status::SUCCESS;
        unsafe {
            let _called = (self.0.attributes)(&self.0 as *const _ as *mut _, 1, mask, 0, &mut st);
        }
        if st != Status::SUCCESS {
            log::error!("attributes_enable({mask:#x}) status {st:?}");
        }
    }

    /// Proper EfiPciIoAttributeOperationEnable (op 2): OR the given bits
    /// into the device's attribute set without touching the rest.
    #[cfg(feature = "tx")]
    pub fn attributes_enable_op(&self, mask: u64) -> Status {
        let mut st = Status::SUCCESS;
        unsafe {
            let _ = (self.0.attributes)(&self.0 as *const _ as *mut _, 2, mask, 0, &mut st);
        }
        st
    }

    /// EfiPciIoAttributeOperationGet (op 0): the firmware-tracked attribute
    /// set (EDK2 keeps it in the PciIo device struct, not in config space).
    /// Captured before the claim so the native-GOP handoff can restore the
    /// VGA forwarding subset it cleared.
    #[cfg(feature = "tx")]
    pub fn attributes_get(&self) -> Option<u64> {
        let mut attrs = 0u64;
        let mut st = Status::SUCCESS;
        // Safety: exclusive-borrowed protocol; Result is an out-param
        // pointer passed in Arg2 per EDK2 PciIoAttributes.
        unsafe {
            let _ = (self.0.attributes)(
                &self.0 as *const _ as *mut _,
                0,
                0,
                &mut attrs as *mut u64 as u64,
                &mut st,
            );
        }
        if st != Status::SUCCESS {
            log::error!("attributes_get status {st:?}");
            return None;
        }
        Some(attrs)
    }

    /// The PciBus-shadowed option ROM (EFI_PCI_IO_PROTOCOL RomImage/
    /// RomSize fields, pinned at slots 19/18 of PciIoRaw). This is the
    /// copy the firmware fetched at enumeration - the native-GOP path
    /// extracts the card's own GOP driver from it. BAR0+0x300000 reads
    /// the live PROM; the shadow is preferred because it is exactly the
    /// bytes PciBus would have dispatched.
    pub fn rom_image(&self) -> Option<&[u8]> {
        if self.0.rom_image.is_null() || self.0.rom_size == 0 {
            return None;
        }
        // Safety: RomImage points into a firmware-allocated buffer of
        // RomSize bytes, valid for the boot services lifetime.
        Some(unsafe {
            core::slice::from_raw_parts(self.0.rom_image as *const u8, self.0.rom_size as usize)
        })
    }

    /// THE BAR write primitive — consumed only by `tx` builds. M0/M1
    /// binaries contain no call sites (constitution §1.7 audit anchor).
    #[cfg(feature = "tx")]
    pub fn bar_write_u32(&self, bar: u8, offset: u64, v: u32) {
        // Safety: exclusive-borrowed protocol, buffers outlive the call.
        unsafe {
            let _ = (self.0.mem.write)(&self.0 as *const _ as *mut _, PciWidth::Uint32, bar, offset, 1, &v as *const _ as *mut c_void);
        };
    }

    /// PCI config-space dword write (tx builds: BAR restore after FLR).
    #[cfg(feature = "tx")]
    pub fn cfg_write_u32(&self, offset: u32, v: u32) {
        // Safety: exclusively borrowed protocol; offset must be 4-aligned.
        unsafe {
            let _ = (self.0.pci.write)(&self.0 as *const _ as *mut _, PciWidth::Uint32, offset, 1, &v as *const _ as *mut c_void);
        };
    }

    /// PCI config-space write (tx builds: BME clear + FLR trigger +
    /// COMMAND restore when the Attributes API is refused).
    #[cfg(feature = "tx")]
    pub fn cfg_write_u16(&self, offset: u32, v: u16) {
        // Safety: exclusively borrowed protocol.
        unsafe {
            let _ = (self.0.pci.write)(&self.0 as *const _ as *mut _, PciWidth::Uint16, offset, 1, &v as *const _ as *mut c_void);
        }
    }

    /// Walk the legacy capability list for the Power Management capability
    /// (ID 0x01). Read-only. `None` = no PM cap, which means the D-state
    /// cannot be evaluated at all (the device is then assumed powered on).
    #[cfg(feature = "tx")]
    pub fn pm_cap(&self) -> Option<u8> {
        if self.config_read_u16(0x06) & 0x10 == 0 {
            return None; // STATUS.CAP_LIST absent
        }
        let mut ptr = self.config_read_u8(0x34) & 0xfc;
        let mut guard = 0;
        while ptr != 0 && guard < 48 {
            if self.config_read_u8(ptr.into()) == 0x01 {
                return Some(ptr);
            }
            ptr = self.config_read_u8(ptr as u32 + 1) & 0xfc;
            guard += 1;
        }
        None
    }

    /// Walk the legacy capability list for the PCI Express capability
    /// (ID 0x10). The FLR path must locate it instead of assuming 0x78: that
    /// offset is right on every card measured here, but nothing guarantees it,
    /// and "Device Control bit 15" written into an unrelated dword is not a
    /// reset - it is an arbitrary config-space write that leaves the function
    /// untouched. Same iteration guard as `pm_cap` (a self-referential chain
    /// would otherwise hang the firmware app with no log line).
    #[cfg(feature = "tx")]
    pub fn pcie_cap(&self) -> Option<u8> {
        if self.config_read_u16(0x06) & 0x10 == 0 {
            return None; // STATUS.CAP_LIST absent
        }
        let mut ptr = self.config_read_u8(0x34) & 0xfc;
        let mut guard = 0;
        while ptr != 0 && guard < 48 {
            if self.config_read_u8(ptr.into()) == 0x10 {
                return Some(ptr);
            }
            ptr = self.config_read_u8(ptr as u32 + 1) & 0xfc;
            guard += 1;
        }
        None
    }

    /// PMCSR read (PM capability + 4). Bits 1:0 = PowerState.
    #[cfg(feature = "tx")]
    pub fn pmcsr(&self, cap: u8) -> u16 {
        self.config_read_u16(cap as u32 + 4)
    }

    /// PMCSR write, caller supplies the exact value.
    #[cfg(feature = "tx")]
    pub fn pmcsr_write(&self, cap: u8, v: u16) {
        self.cfg_write_u16(cap as u32 + 4, v);
    }

    /// Request D0 by clearing PMCSR.PowerState (all other bits preserved)
    /// and verify by readback. True only when the state field reads back 0.
    /// A D3hot -> D0 transition may take up to 10 ms (PCI PM spec).
    #[cfg(feature = "tx")]
    pub fn pmcsr_force_d0(&self, cap: u8) -> bool {
        let cur = self.pmcsr(cap);
        if cur & 0x3 == 0 {
            return true;
        }
        self.pmcsr_write(cap, cur & !0x3);
        boot::stall(Duration::from_millis(20));
        self.pmcsr(cap) & 0x3 == 0
    }

    /// Coherent common-buffer DMA allocation (AllocateBuffer + Map).
    /// TOTORONG craft uses EfiReservedMemoryType, but consumer firmware
    /// (observed on AD107M) rejects it with INVALID_PARAMETER — fall back
    /// through legal coherent memory types before giving up.
    #[cfg(feature = "tx")]
    pub fn dma_alloc(&self, len: usize) -> Result<DmaBuffer, Status> {
        capture_memmap_once();
        // Above-4GB staging first, gated per family (see set_high_staging):
        // issues #14/#16 (2026-10-28) - the booter's first host DMA pulls
        // the WprMeta into SEC2 DMEM, and on some Turing cards a sub-4GB
        // meta is rejected with halt code 0x91 (cmp50hx v2.63: a locked
        // card blocks sub-4GB sysmem reads - "exit 0x91"; chenxluo v61
        // moved all TU102 SEC buffers above 4GB) while identical bytes pass
        // below 4GB elsewhere. The reference driver itself stages the meta
        // above 4GB (live dump 0x110BB0000) and passes a 64-bit mailbox
        // pair, so the high path is the driver-exact configuration.
        // \tx-low-staging.txt forces the legacy PciIo path for A/B testing.
        if !TX_HIGH_STAGING.load(core::sync::atomic::Ordering::Relaxed) {
            info!("DMA alloc: family not gated for >4GB staging - legacy PciIo path");
        } else if crate::tx::probe_file("\\tx-low-staging.txt") {
            info!("DMA alloc: \\tx-low-staging.txt present - legacy sub-4GB PciIo path forced");
        } else if let Ok(b) = self.dma_alloc_high(len) {
            return Ok(b);
        } else {
            info!("DMA alloc: >4GB allocation failed - falling back to PciIo path");
        }
        // Canonical EFI_MEMORY_TYPE numbering (UEFI spec): BS-Data=4,
        // RS-Data=6. EfiReserved(0) is included only to reproduce the
        // documented EDK2 rejection for the record; the legal set for
        // PciIo.AllocateBuffer is exactly {EfiBootServicesData,
        // EfiRuntimeServicesData} (PciRootBridgeIo.c).
        const TYPES: [(u32, &str); 3] = [
            (4, "EfiBootServicesData"),
            (6, "EfiRuntimeServicesData"),
            (0, "EfiReserved(spec-rejected)"),
        ];
        let pages = len.div_ceil(4096);
        for (mt, name) in TYPES {
            let mut host: *mut c_void = core::ptr::null_mut();
            // Safety: out-params outlive the call; protocol exclusively borrowed.
            let st = unsafe {
                (self.0.allocate_buffer)(
                    &self.0 as *const _ as *mut _,
                    0, // AllocateAnyPages
                    mt,
                    pages,
                    &mut host,
                    0,
                )
            };
            if st != Status::SUCCESS || host.is_null() {
                log::info!("DMA alloc: memtype {name} rejected ({st:?})");
                continue;
            }
            let mut bytes = pages * 4096;
            let mut device: u64 = 0;
            let mut mapping: *mut c_void = core::ptr::null_mut();
            // Safety: as above; host block is valid for `pages` pages.
            let st = unsafe {
                (self.0.map)(
                    &self.0 as *const _ as *mut _,
                    2, // EfiPciOperationBusMasterCommonBuffer
                    host,
                    &mut bytes,
                    &mut device,
                    &mut mapping,
                )
            };
            if st != Status::SUCCESS {
                log::info!("DMA map: memtype {name} failed ({st:?})");
                let _st = unsafe { (self.0.free_buffer)(&self.0 as *const _ as *mut _, pages, host) };
                continue;
            }
            // The band is derived from the actual device address, not from
            // which branch ran (issue #19: a "HIGH" label on a sub-4GB
            // address is worse than no label).
            let band = if device >= 0x1_0000_0000 { "HIGH" } else { "LOW" };
            log::info!("DMA alloc: {pages} pages memtype={name} dev={device:#x} [{band}] (PciIo)");
            return Ok(DmaBuffer { host: host as *mut u8, dev: device, mapping, pages, len, released: false, high: false });
        }
        Err(Status::OUT_OF_RESOURCES)
    }

    /// Above-4GB staging allocation (raw boot-services pages). The device
    /// address is the physical address: the falcon FBIF is programmed in
    /// PHYSICAL mode and EDK2 PciIo.Map is an identity mapping without an
    /// IOMMU driver (issue #1 arbitration), so there is no Map bookkeeping
    /// on this path - `release` frees via boot::free_pages. Turing GSP/SEC2
    /// are 64-bit DMA masters (the driver itself stages at 0x110BB0000).
    #[cfg(feature = "tx")]
    fn dma_alloc_high(&self, len: usize) -> Result<DmaBuffer, Status> {
        const FOUR_GB: u64 = 0x1_0000_0000;
        let pages = len.div_ceil(4096);
        // MaxAddress(u64::MAX) is NOT "allocate at the top of RAM": DxeCore's
        // CoreFindFreePagesI selects the highest free range UNDER the cap
        // (Target=0, best match = DescEnd > Target), and on the field boards
        // that range sat at 1-2 GiB - v2.1.1's "HIGH" allocations landed at
        // 0x81e83000 et al (issues #17/#19). Walk the memory map ourselves
        // and AllocateAddress inside a conventional descriptor at/above
        // 4 GiB: per spec this lands exactly at the requested address or
        // fails, so placement is deterministic on any firmware.
        let mmap = boot::memory_map(MemoryType::BOOT_SERVICES_DATA)
            .map_err(|_| Status::OUT_OF_RESOURCES)?;
        let mut candidates = 0u32;
        let mut failed = 0u32;
        for d in mmap.entries() {
            if d.ty != MemoryType::CONVENTIONAL {
                continue;
            }
            let base = d.phys_start;
            let end = base.saturating_add((d.page_count as u64) * 4096);
            let start = base.max(FOUR_GB);
            if start >= end || (end - start) < (pages as u64) * 4096 {
                continue;
            }
            candidates += 1;
            if let Ok(ptr) = boot::allocate_pages(
                AllocateType::Address(start),
                MemoryType::BOOT_SERVICES_DATA,
                pages,
            ) {
                let dev = ptr.as_ptr() as usize as u64;
                let band = if dev >= FOUR_GB { "HIGH" } else { "LOW" };
                info!(
                    "DMA alloc: {pages} pages memtype=EfiBootServicesData {band} dev={dev:#x} (descriptor {base:#x}+{:#x})", (d.page_count as u64) * 4096
                );
                return Ok(DmaBuffer {
                    host: ptr.as_ptr(),
                    dev,
                    mapping: core::ptr::null_mut(),
                    pages,
                    len,
                    released: false,
                    high: true,
                });
            }
            failed += 1;
            if failed >= 4 {
                break;
            }
        }
        // Full map picture is already captured once per boot (capture_memmap_once,
        // called from dma_alloc before any allocation).
        info!(
            "DMA alloc: no usable >4GB conventional space (candidates={candidates} failed={failed})"
        );
        Err(Status::NOT_FOUND)
    }

    /// Bulk u32 read from a BAR window (used for the PROM CRC pass).
    pub fn bar_read_u32_buf(&self, bar: u8, offset: u64, buf: &mut [u32]) -> Status {
        self.mem_read_raw(PciWidth::Uint32, bar, offset, buf.len(), buf.as_mut_ptr() as *mut c_void)
    }

    /// Bulk u32 write into a BAR window (VRAM fallback staging via BAR1).
    /// `buf.len()` must be a multiple of 4.
    #[allow(dead_code)] // BAR write path: consumed by the TX transaction only
    pub fn bar_write_buf(&self, bar: u8, offset: u64, buf: &[u8]) -> Status {
        self.mem_write_raw(PciWidth::Uint32, bar, offset, buf.len() / 4, buf.as_ptr() as *mut c_void)
    }

    /// (segment, bus, device, function) as UINTN scalars per spec.
    pub fn get_location(&self) -> Option<(usize, usize, usize, usize)> {
        let (mut seg, mut bus, mut dev, mut fun) = (0usize, 0usize, 0usize, 0usize);
        // Safety: out-params outlive the call and are correctly typed.
        let st = unsafe {
            (self.0.get_location)(&self.0 as *const _ as *mut _, &mut seg, &mut bus, &mut dev, &mut fun)
        };
        if st != Status::SUCCESS {
            log::error!("get_location status {:?}", st);
            return None;
        }
        Some((seg, bus, dev, fun))
    }
}

/// Coherent DMA buffer. Release is EXPLICIT (`release`) so the transaction's
/// unwind can verify nobody freed twice or leaked — no silent Drop.
#[cfg(feature = "tx")]
#[allow(dead_code)] // consumed by the fwsec/booter wiring tranche
pub struct DmaBuffer {
    pub host: *mut u8,
    pub dev: u64,
    pub mapping: *mut c_void,
    pub pages: usize,
    pub len: usize,
    released: bool,
    /// true = raw boot-services allocation above 4GB (dma_alloc_high):
    /// `release` must free via boot::free_pages, skipping the PciIo
    /// unmap/free_buffer pair (there is no mapping to undo).
    high: bool,
}

#[cfg(feature = "tx")]
impl DmaBuffer {
    /// Unmap + free. Idempotent (second call is a no-op returning true).
    /// Returns false if the firmware rejected the unmap — caller must log
    /// and refuse to proceed (unwind contract).
    #[allow(dead_code)] // consumed by the fwsec/booter wiring tranche
    pub fn release(&mut self, pci: &PciIo) -> bool {
        if self.released {
            return true;
        }
        self.released = true;
        if self.high {
            // Raw boot-services allocation (dma_alloc_high): no PciIo
            // mapping to undo - free via the matching boot service.
            return match core::ptr::NonNull::new(self.host) {
                Some(p) => {
                    // Safety: live allocation from boot::allocate_pages.
                    unsafe { boot::free_pages(p, self.pages) }.is_ok()
                }
                None => false,
            };
        }
        // Safety: buffer still mapped/allocated; protocol outlives boot.
        unsafe {
            let s1 = (pci.0.unmap)(&pci.0 as *const _ as *mut _, self.mapping);
            let s2 = (pci.0.free_buffer)(&pci.0 as *const _ as *mut _, self.pages, self.host as *mut c_void);
            s1 == Status::SUCCESS && s2 == Status::SUCCESS
        }
    }
}

/// One NVIDIA display adapter found by [`enumerate`] (function 0 only).
#[derive(Clone, Copy)]
pub struct FoundGpu {
    pub handle: uefi::Handle,
    pub device_id: u16,
    pub seg: usize,
    pub bus: usize,
    pub dev: usize,
    pub fun: usize,
}

/// Enumerate all PCI(e) devices; NVIDIA (10de) ones get the capability walk.
/// Read-only: config-space reads only, zero writes (constitution §1.7/M0).
/// Returns EVERY NVIDIA display adapter (class 03, function 0) found, in
/// firmware enumeration order.
///
/// Multi-GPU (2026-10-27): the caller decides what runs - the TX phase
/// processes every supported card in this order (see `select_tx_targets`),
/// the M1 sample takes the first. A `\tx-select-bdf.txt` marker
/// ("bb:dd.f" or "dddd:bb:dd.f", hex) pins exactly one card instead.
pub fn enumerate() -> alloc::vec::Vec<FoundGpu> {
    info!("PHASE: ENUMERATE (read-only)");
    let handles = match boot::locate_handle_buffer(SearchType::ByProtocol(&PciIo::GUID)) {
        Ok(h) => h,
        Err(e) => {
            info!("ENUMERATE: locate_handle_buffer failed: {:?}", e.status());
            return alloc::vec::Vec::new();
        }
    };
    info!("ENUMERATE: {} PciIo handles", handles.len());

    let mut candidates: alloc::vec::Vec<FoundGpu> = alloc::vec::Vec::new();
    for handle in handles.iter() {
        // Safety: GetProtocol performs no driver binding; protocol
        // interface stays valid while the handle lives (all of boot time).
        let proto = unsafe {
            boot::open_protocol::<PciIo>(
                OpenProtocolParams {
                    handle: *handle,
                    agent: boot::image_handle(),
                    controller: None,
                },
                OpenProtocolAttributes::GetProtocol,
            )
        };
        let pci = match proto {
            Ok(p) => p,
            Err(e) => {
                info!("ENUMERATE: open_protocol failed on {:?}: {:?}", handle, e.status());
                continue;
            }
        };

        // Location first: it doubles as an interface sanity probe.
        let (seg, bus, dev, fun) = match pci.get_location() {
            Some(loc) => loc,
            None => {
                info!("ENUMERATE: no location on {:?}, skip", handle);
                continue;
            }
        };

        let vendor = pci.config_read_u16(0x00);
        let device = pci.config_read_u16(0x02);
        if vendor == 0xffff || vendor == 0x0000 {
            info!("PCI {:04x}:{:02x}:{:02x}.{} empty slot (vendor {:#06x})", seg, bus, dev, fun, vendor);
            continue;
        }
        let class = pci.config_read_u32(0x08) >> 16; // class<<8 | subclass
        info!(
            "PCI {:04x}:{:02x}:{:02x}.{} {:04x}:{:04x} class {:06x}",
            seg, bus, dev, fun, vendor, device, class
        );

        if vendor == NVIDIA_VENDOR_ID {
            // Candidate = NVIDIA display adapter (class 03), function 0.
            // Other NVIDIA functions (HDA audio, NICs) are logged but never
            // selected - the old first-fun-0 rule could match those.
            if fun == 0 && class >> 8 == 0x03 {
                candidates.push(FoundGpu {
                    handle: *handle,
                    device_id: device,
                    seg, bus, dev, fun,
                });
            }
            describe_nvidia(&pci, seg, bus, dev, fun, device);
        }
    }
    if candidates.len() > 1 {
        info!("ENUMERATE: {} NVIDIA display adapters present:", candidates.len());
        for c in candidates.iter() {
            info!(
                "ENUMERATE:   candidate {:04x}:{:02x}:{:02x}.{} dev=10de:{:04x}",
                c.seg, c.bus, c.dev, c.fun, c.device_id
            );
        }
    }
    candidates
}

/// TX target selection. Default (no marker): EVERY supported card runs, in
/// enumeration order - one boot unlocks the whole board, and a card that is
/// already unlocked (EALREADY) or unsupported simply yields to the next.
/// A `\tx-select-bdf.txt` marker pins exactly one card instead; a pinned
/// BDF that matches nothing (or an unparsable marker) refuses to guess and
/// returns an empty set - the stick owner asked for a specific card and no
/// other one may run.
#[cfg(feature = "tx")]
pub fn select_tx_targets(all: &[FoundGpu]) -> alloc::vec::Vec<FoundGpu> {
    match crate::tx::read_boot_file("\\tx-select-bdf.txt") {
        Some(raw) => match parse_bdf(&raw) {
            Some(bdf) => {
                for c in all.iter() {
                    if (c.seg, c.bus, c.dev, c.fun) == bdf {
                        info!(
                            "ENUMERATE: tx-select-bdf matched {:04x}:{:02x}:{:02x}.{} - only this card runs",
                            c.seg, c.bus, c.dev, c.fun
                        );
                        return alloc::vec![*c];
                    }
                }
                info!(
                    "ENUMERATE: tx-select-bdf {:04x}:{:02x}:{:02x}.{} matches no candidate - refusing to guess, TX skipped",
                    bdf.0, bdf.1, bdf.2, bdf.3
                );
                alloc::vec::Vec::new()
            }
            None => {
                info!("ENUMERATE: tx-select-bdf.txt unparsable (want bb:dd.f or dddd:bb:dd.f hex) - refusing to guess, TX skipped");
                alloc::vec::Vec::new()
            }
        },
        None => {
            if all.len() > 1 {
                info!(
                    "ENUMERATE: no tx-select-bdf marker - every supported card runs, in enumeration order ({} found)",
                    all.len()
                );
            }
            all.to_vec()
        }
    }
}

/// Parse "bb:dd.f" or "dddd:bb:dd.f" (hex, whitespace/NUL tolerant).
#[cfg(feature = "tx")]
fn parse_bdf(raw: &[u8]) -> Option<(usize, usize, usize, usize)> {
    let clean: alloc::vec::Vec<u8> = raw
        .iter()
        .copied()
        .filter(|b| !b.is_ascii_whitespace() && *b != 0)
        .collect();
    let s = core::str::from_utf8(&clean).ok()?;
    let (seg, rest) = if s.matches(':').count() == 2 {
        let (a, b) = s.split_once(':')?;
        (usize::from_str_radix(a, 16).ok()?, b)
    } else {
        (0, s)
    };
    let (bus_s, devfun) = rest.split_once(':')?;
    let (dev_s, fun_s) = devfun.split_once('.')?;
    let bus = usize::from_str_radix(bus_s, 16).ok()?;
    let dev = usize::from_str_radix(dev_s, 16).ok()?;
    let fun = usize::from_str_radix(fun_s, 16).ok()?;
    Some((seg, bus, dev, fun))
}

/// Resolve the PCI Express capability offset for the FLR path. The walk is
/// authoritative; the historical 0x78 is only a fallback for a firmware/card
/// combination where the capability list itself is unreadable, and it is
/// logged so the evidence record shows which branch ran.
#[cfg(feature = "tx")]
pub fn pcie_cap_or_measured(gpu: &PciIo) -> u8 {
    match gpu.pcie_cap() {
        Some(c) => {
            if c != 0x78 {
                info!("PCIe cap located at {c:#04x} (not the measured 0x78) - FLR uses the walked offset");
            }
            c
        }
        None => {
            info!("PCIe cap walk found nothing (no cap list?) - falling back to the measured 0x78 offset; an FLR may be a no-op here");
            0x78
        }
    }
}

/// Logs the GPU's command, status and device control, then the IDs, command,
/// status, bus numbers and bridge control of every PCI bridge. Read-only.
#[cfg(feature = "tx")]
pub fn log_dma_path(tag: &str, gpu: &PciIo) {
    // PCIe cap from the capability walk: Device Control lives at +0x08.
    let pcie = pcie_cap_or_measured(gpu);
    info!(
        "DMA-PATH[{tag}]: gpu loc={:?} command={:#06x} status={:#06x} devctl={:#06x} (cap {pcie:#04x})",
        gpu.get_location(),
        gpu.config_read_u16(0x04),
        gpu.config_read_u16(0x06),
        gpu.config_read_u16(pcie as u32 + 0x08)
    );
    let Ok(handles) = boot::locate_handle_buffer(SearchType::ByProtocol(&PciIo::GUID)) else {
        info!("DMA-PATH[{tag}]: locate_handle_buffer failed");
        return;
    };
    for handle in handles.iter() {
        // Safety: GetProtocol performs no driver binding (same as enumerate()).
        let Ok(pci) = (unsafe {
            boot::open_protocol::<PciIo>(
                OpenProtocolParams { handle: *handle, agent: boot::image_handle(), controller: None },
                OpenProtocolAttributes::GetProtocol,
            )
        }) else {
            continue;
        };
        if pci.config_read_u16(0x00) == 0xffff || pci.config_read_u8(0x0e) & 0x7f != 0x01 {
            continue;
        }
        let loc = pci.get_location();
        let buses = pci.config_read_u32(0x18);
        info!(
            "DMA-PATH[{tag}]: bridge {:04x}:{:04x} loc={loc:?} command={:#06x} status={:#06x} pri/sec/sub={}/{}/{} bridgectl={:#06x}",
            pci.config_read_u16(0x00),
            pci.config_read_u16(0x02),
            pci.config_read_u16(0x04),
            pci.config_read_u16(0x06),
            buses & 0xff,
            buses >> 8 & 0xff,
            buses >> 16 & 0xff,
            pci.config_read_u16(0x3e)
        );
    }
}

/// Read-only handoff-time PCI snapshot (issue #61 ask #2): GPU config
/// header, capability walk (PCIe DevCtl/LnkCtl + MSI enable), and every
/// bridge's Command / Bridge Control (VGA routing bits). Runs after the
/// FLR/verify phase and before the reconnect, so the log pairs directly
/// with the Windows-side pnputil dump of the same boot: a resource-class
/// delta between the two pinpoints which side changed what.
#[cfg(feature = "tx")]
pub fn diag_handoff_pci(gpu: &PciIo) {
    info!(
        "HANDOFF-PCI gpu cfg00-1f: {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x}",
        gpu.config_read_u32(0x00),
        gpu.config_read_u32(0x04),
        gpu.config_read_u32(0x08),
        gpu.config_read_u32(0x0c),
        gpu.config_read_u32(0x10),
        gpu.config_read_u32(0x14),
        gpu.config_read_u32(0x18),
        gpu.config_read_u32(0x1c),
    );
    info!(
        "HANDOFF-PCI gpu cfg20-3f: {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x}",
        gpu.config_read_u32(0x20),
        gpu.config_read_u32(0x24),
        gpu.config_read_u32(0x28),
        gpu.config_read_u32(0x2c),
        gpu.config_read_u32(0x30),
        gpu.config_read_u32(0x34),
        gpu.config_read_u32(0x38),
        gpu.config_read_u32(0x3c),
    );
    // Capability walk from the 0x34 pointer; PCIe (0x10) and MSI (0x05)
    // get field decodes, anything else is listed by id. Bounded so a
    // poisoned pointer cannot loop.
    let mut ptr = gpu.config_read_u8(0x34) as u32;
    for _ in 0..16 {
        if ptr == 0 || ptr >= 0xff {
            break;
        }
        let id = gpu.config_read_u8(ptr);
        let next = gpu.config_read_u8(ptr + 1) as u32;
        match id {
            0x10 => info!(
                "HANDOFF-PCI cap PCIe@{ptr:#04x}: devcap={:#06x} devctl={:#06x} devsta={:#06x} lnkcap={:#08x} lnkctl={:#06x} lnksta={:#06x}",
                gpu.config_read_u16(ptr + 4),
                gpu.config_read_u16(ptr + 8),
                gpu.config_read_u16(ptr + 0xa),
                gpu.config_read_u32(ptr + 0xc),
                gpu.config_read_u16(ptr + 0x10),
                gpu.config_read_u16(ptr + 0x12),
            ),
            0x05 => info!(
                "HANDOFF-PCI cap MSI@{ptr:#04x}: mc={:#06x}",
                gpu.config_read_u16(ptr + 2)
            ),
            other => info!("HANDOFF-PCI cap id={other:#04x}@{ptr:#04x}"),
        }
        ptr = next;
    }
    let Ok(handles) = boot::locate_handle_buffer(SearchType::ByProtocol(&PciIo::GUID)) else {
        info!("HANDOFF-PCI bridge scan: locate_handle_buffer failed");
        return;
    };
    for handle in handles.iter() {
        // Safety: GetProtocol performs no driver binding (same as log_dma_path).
        let Ok(pci) = (unsafe {
            boot::open_protocol::<PciIo>(
                OpenProtocolParams { handle: *handle, agent: boot::image_handle(), controller: None },
                OpenProtocolAttributes::GetProtocol,
            )
        }) else {
            continue;
        };
        if pci.config_read_u16(0x00) == 0xffff || pci.config_read_u8(0x0e) & 0x7f != 0x01 {
            continue;
        }
        let loc = pci.get_location();
        info!(
            "HANDOFF-PCI bridge {loc:?} cmd={:#06x} bridgectl={:#06x}",
            pci.config_read_u16(0x04),
            pci.config_read_u16(0x3e)
        );
    }
}

/// Bus-master guard for the PCI bridges between the root complex and the GPU.
///
/// `enable` sets COMMAND bit 2 (bus master) on every PCI-to-PCI bridge in the
/// GPU's segment whose secondary..subordinate range contains the GPU's bus,
/// only where the bit is clear, and records the bridges whose readback shows it
/// set. Drop clears bit 2 again on exactly those bridges.
#[cfg(feature = "tx")]
pub struct BridgeMaster {
    /// (bridge handle, location for logs) of every bridge this guard enabled.
    enabled: alloc::vec::Vec<(uefi::Handle, Option<(usize, usize, usize, usize)>)>,
}

#[cfg(feature = "tx")]
impl BridgeMaster {
    pub fn enable(gpu: &PciIo) -> BridgeMaster {
        let mut enabled = alloc::vec::Vec::new();
        let Some((gpu_seg, gpu_bus, _, _)) = gpu.get_location() else {
            info!("BRIDGE-BME: GPU location unavailable - not touching any bridge");
            return BridgeMaster { enabled };
        };
        let Ok(handles) = boot::locate_handle_buffer(SearchType::ByProtocol(&PciIo::GUID)) else {
            info!("BRIDGE-BME: locate_handle_buffer failed");
            return BridgeMaster { enabled };
        };
        for handle in handles.iter() {
            // Safety: GetProtocol performs no driver binding (same as enumerate()).
            let Ok(pci) = (unsafe {
                boot::open_protocol::<PciIo>(
                    OpenProtocolParams { handle: *handle, agent: boot::image_handle(), controller: None },
                    OpenProtocolAttributes::GetProtocol,
                )
            }) else {
                continue;
            };
            if pci.config_read_u16(0x00) == 0xffff || pci.config_read_u8(0x0e) & 0x7f != 0x01 {
                continue;
            }
            let loc = pci.get_location();
            if loc.map(|(seg, _, _, _)| seg) != Some(gpu_seg) {
                continue;
            }
            let buses = pci.config_read_u32(0x18);
            let (sec, sub) = ((buses >> 8 & 0xff) as usize, (buses >> 16 & 0xff) as usize);
            if gpu_bus < sec || gpu_bus > sub {
                continue;
            }
            let before = pci.config_read_u16(0x04);
            if before & 0x4 != 0 {
                info!("BRIDGE-BME: {loc:?} already has bus master (command={before:#06x})");
                continue;
            }
            pci.cfg_write_u16(0x04, before | 0x4);
            let after = pci.config_read_u16(0x04);
            info!("BRIDGE-BME: {loc:?} command {before:#06x} -> {after:#06x}");
            if after & 0x4 != 0 {
                enabled.push((*handle, loc));
            }
        }
        BridgeMaster { enabled }
    }
}

#[cfg(feature = "tx")]
impl Drop for BridgeMaster {
    fn drop(&mut self) {
        for (handle, loc) in self.enabled.drain(..) {
            // Safety: GetProtocol performs no driver binding.
            let Ok(pci) = (unsafe {
                boot::open_protocol::<PciIo>(
                    OpenProtocolParams { handle, agent: boot::image_handle(), controller: None },
                    OpenProtocolAttributes::GetProtocol,
                )
            }) else {
                info!("BRIDGE-BME: restore {loc:?}: protocol gone");
                continue;
            };
            let now = pci.config_read_u16(0x04);
            pci.cfg_write_u16(0x04, now & !0x4);
            info!("BRIDGE-BME: restored {loc:?} command {now:#06x} -> {:#06x}", pci.config_read_u16(0x04));
        }
    }
}

fn describe_nvidia(pci: &PciIo, seg: usize, bus: usize, dev: usize, fun: usize, device: u16) {
    info!(
        "NVIDIA {} {:04x}:{:02x}:{:02x}.{} device {:04x}",
        if fun == 0 { "GPU?" } else { "fn" },
        seg, bus, dev, fun, device
    );
    let header_type = pci.config_read_u8(0x0e) & 0x7f;
    if header_type != 0x00 {
        info!("  header type {:#x}: not type 0, skip caps walk", header_type);
        return;
    }

    // BARs as programmed by firmware (read-only report; no size probing).
    let mut i = 0u32;
    while i < 6 {
        let lo_off = 0x10 + 4 * i;
        let lo = pci.config_read_u32(lo_off);
        if lo == 0 {
            i += 1;
            continue;
        }
        let is_mem = lo & 1 == 0;
        if is_mem && (lo >> 1 & 3) == 2 {
            // 64-bit BAR: consumes this and the next slot
            let hi = pci.config_read_u32(lo_off + 4);
            info!("  BAR{} 64-bit mem base {:#010x}{:08x}", i, hi, lo & !0xf);
            i += 2;
        } else if is_mem {
            info!("  BAR{} 32-bit mem base {:#010x}", i, lo & !0xf);
            i += 1;
        } else {
            info!("  BAR{} io base {:#06x}", i, lo & !3);
            i += 1;
        }
    }

    // Legacy capability list at 0x34.
    let status = pci.config_read_u16(0x06);
    if status & 0x10 == 0 {
        info!("  no capability list");
    } else {
        let mut cap_ptr = pci.config_read_u8(0x34) & 0xfc;
        // Bounded: a malformed or self-referential chain must not hang the app
        // before the transaction even starts (no log line, no chain-load).
        let mut cap_guard = 0u32;
        while cap_ptr != 0 && cap_guard < 48 {
            cap_guard += 1;
            let cap_id = pci.config_read_u8(cap_ptr.into());
            match cap_id {
                0x10 => {
                    let devcap = pci.config_read_u32(cap_ptr as u32 + 0x04);
                    let link_cap = pci.config_read_u32(cap_ptr as u32 + 0x0c);
                    info!(
                        "  PCIe cap @{:#04x}: DEVCAP {:#010x} (FLR {}) LINKCAP {:#010x}",
                        cap_ptr,
                        devcap,
                        if devcap & (1 << 28) != 0 { "yes" } else { "no" },
                        link_cap
                    );
                }
                other => {
                    info!("  cap {:#04x} @{:#04x}", other, cap_ptr);
                }
            }
            cap_ptr = pci.config_read_u8(cap_ptr as u32 + 1);
        }
    }

    // Extended capability list at 0x100 (ReBAR lives here).
    let mut off: u32 = 0x100;
    let mut ext_guard = 0u32;
    loop {
        // Extended capabilities live at 0x100+, but the list is NOT ordered by
        // offset: a field-measured AD106 chain goes 0x100 -> 0x250 -> 0x258 ->
        // 0x128 -> ... (backwards at 0x258 -> 0x128). Only the terminator, a
        // pointer below 0x100, and the iteration cap below bound this walk.
        ext_guard += 1;
        if ext_guard > 256 {
            info!("  ext cap walk stopped after 256 entries (malformed list)");
            break;
        }
        let hdr = pci.config_read_u32(off);
        let id = (hdr & 0xffff) as u16;
        if id == 0x0000 {
            break; // list terminator
        }
        let next = ((hdr >> 20) & 0xfff) as u32;
        match id {
            0x0015 => {
                // Layout per PCIe spec as decoded by Linux and ReBarUEFI
                // (both field-proven): a chain of {CAP, CTRL} 8-byte pairs
                // at +4/+8. The first pair's CTRL[7:5] holds the resizable
                // BAR count; per pair, CTRL[2:0] = BAR index, CTRL[12:8] =
                // current size (1 MiB << n), CAP = supported-sizes bitmap
                // (bit n raw = 2^n MB, mask 0x00FFFFF0). The M0-era decode
                // read the count from CAP bits[4:0] and the size from the
                // next pair's CAP - both wrong; corrected against
                // ReBarUEFI (reference/ReBarUEFI, ReBar.c).
                let first_ctrl = pci.config_read_u32(off + 8);
                let n_bars = (first_ctrl >> 5) & 0x7;
                info!("  ReBAR cap @{:#06x}: {} resizable BAR(s)", off, n_bars);
                for b in 0..n_bars as u32 {
                    let cap = pci.config_read_u32(off + 4 + 8 * b);
                    let ctrl = pci.config_read_u32(off + 8 + 8 * b);
                    let size_idx = (ctrl >> 8) & 0x1f;
                    let mut supported = alloc::string::String::new();
                    for bit in (4..=23).rev() {
                        if cap & (1 << bit) != 0 {
                            if !supported.is_empty() {
                                supported.push('/');
                            }
                            // raw bit (n+4) = 2^n MiB (linux bytes_to_size =
                            // log2(bytes) - 20); CTRL[12:8] size_idx likewise
                            let mib = 1u64 << (bit - 4);
                            if mib >= 1024 {
                                supported.push_str(&alloc::format!("{}G", mib / 1024));
                            } else {
                                supported.push_str(&alloc::format!("{}M", mib));
                            }
                        }
                    }
                    info!(
                        "    BAR{} supported [{}] current {} MiB (size_idx {}, ctrl {:#010x} cap {:#010x})",
                        ctrl & 0x7,
                        supported,
                        1u64 << size_idx,
                        size_idx,
                        ctrl,
                        cap
                    );
                }
            }
            other => info!("  ext cap {:#06x} @{:#06x}", other, off),
        }
        // Extended capability chains are NOT ordered by offset - a PCIe list
        // may point backwards. Field-measured on an AD106 the full chain is
        //   0x100 -> 0x250 -> 0x258 -> 0x128 -> 0x420 -> 0x600 -> 0x900 ->
        //   0xbb0 -> 0xc1c -> 0xd00 -> 0xe00 -> 0
        // (the backwards step is 0x258 -> 0x128; an earlier note here showed a
        // truncated chain that appeared to terminate at 0x420). A monotonicity
        // check therefore truncates a valid list, so only the terminator and a
        // malformed below-0x100 pointer end the walk; the iteration cap above
        // is what bounds a cyclic one.
        if next == 0 || next < 0x100 {
            break;
        }
        off = next;
    }
}
