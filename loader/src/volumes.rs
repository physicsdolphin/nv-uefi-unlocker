//! Marker/asset/log volume resolution.
//!
//! The USB application reads markers and assets from its own boot volume
//! (`get_image_file_system` of the image handle). The firmware-resident DXE
//! driver has no SFS self-volume - it is loaded from an FV, so that call
//! fails and every marker/asset would read as absent, silently disarming the
//! driver.
//!
//! Volume identity (PR #100 audit F1/F2-B/N2/Q2): neither a filename nor a
//! raw handle may act as the identity of the armed volume. The arm authority
//! is the NVRAM variable written by the application's installer mode
//! (`dxe`), and its payload pins the **GPT partition GUID** taken from the
//! firmware's own device path at install time. `marker_fs()` re-resolves
//! that GUID against the live handle database on every call, so a pulled
//! medium (or a recycled DxeCore handle) fails closed with `NOT_FOUND`
//! instead of resolving onto an unrelated volume. Whole-disk/CD volumes have
//! no partition GUID and cannot be armed.
//!
//! With nothing pinned the helper falls back to the image volume - the
//! application's behavior stays byte-identical.

use core::cell::UnsafeCell;

use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams, ScopedProtocol, SearchType};
use uefi::proto::device_path::DevicePath;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::{Handle, Identify};

struct ArmedCell(UnsafeCell<Option<[u8; 16]>>);
// Safety: boot-services execution is single-threaded.
unsafe impl Sync for ArmedCell {}
unsafe impl Send for ArmedCell {}
static ARMED: ArmedCell = ArmedCell(UnsafeCell::new(None));

/// Device-path node constants used by the raw walk (`partition_guid`).
const DEV_PATH_MEDIA: u8 = 0x04;
const DEV_PATH_MEDIA_HARD_DRIVE: u8 = 0x01;
const DEV_PATH_END: u8 = 0x7f;
/// MEDIA_HARD_DRIVE signature types: 0x01 = PC-AT (MBR disk signature),
/// 0x02 = 16-byte GPT GUID.
const SIG_TYPE_MBR: u8 = 0x01;
const SIG_TYPE_GPT: u8 = 0x02;
/// Header(4) + PartitionNumber(4) + Start(8) + Size(8) + Signature(16)
/// + PartitionFormat(1) + SignatureType(1).
const HARD_DRIVE_NODE_MIN_LEN: usize = 42;

/// Pin the armed volume identity as reported by the firmware's own device
/// path. Handle-free by design (audit N2).
pub fn set_armed_volume(identity: [u8; 16]) {
    // Safety: single-threaded boot services; written once before any reader.
    unsafe { *ARMED.0.get() = Some(identity) };
}

/// The pinned volume identity, if any.
pub fn armed() -> Option<[u8; 16]> {
    // Safety: single-threaded boot services.
    unsafe { *ARMED.0.get() }
}

/// The partition identity of a handle's device path, from its hard-drive
/// media node: GPT GUID (16 bytes), or MBR (4-byte disk signature ++ the
/// partition's start LBA, zero-padded). Both are properties of the firmware's
/// own enumeration - a file dropped on a volume cannot change them. Cloning
/// one requires reading the armed volume's partition table (physical access
/// or an OS-level disk tool), which is already above the file-drop threat
/// bar (audit F1). Whole-disk/CD volumes return `None` (cannot be armed).
pub fn partition_guid(handle: Handle) -> Option<[u8; 16]> {
    // Safety: GetProtocol open on a handle from this session; the scoped
    // wrapper closes the protocol on drop.
    let dp = unsafe {
        boot::open_protocol::<DevicePath>(
            OpenProtocolParams { handle, agent: boot::image_handle(), controller: None },
            OpenProtocolAttributes::GetProtocol,
        )
        .ok()?
    };
    let mut p = dp.as_ffi_ptr().cast::<u8>();
    // Safety: node lengths come from the firmware's own well-formed device
    // path; the walk is length-bounded and stops at the END node.
    loop {
        let major = unsafe { *p };
        let sub = unsafe { *p.add(1) };
        let len = u16::from_le_bytes([unsafe { *p.add(2) }, unsafe { *p.add(3) }]) as usize;
        if len < 4 {
            return None;
        }
        if major == DEV_PATH_MEDIA && sub == DEV_PATH_MEDIA_HARD_DRIVE && len >= HARD_DRIVE_NODE_MIN_LEN {
            let sig_type = unsafe { *p.add(41) };
            let mut identity = [0u8; 16];
            if sig_type == SIG_TYPE_GPT {
                unsafe { core::ptr::copy_nonoverlapping(p.add(24), identity.as_mut_ptr(), 16) };
                return Some(identity);
            }
            if sig_type == SIG_TYPE_MBR {
                // Identity = 4-byte disk signature ++ start LBA (LE) ++ pad.
                unsafe { core::ptr::copy_nonoverlapping(p.add(24), identity.as_mut_ptr(), 4) };
                unsafe { core::ptr::copy_nonoverlapping(p.add(8), identity.as_mut_ptr().add(4), 4) };
                return Some(identity);
            }
        }
        if major == DEV_PATH_END {
            return None;
        }
        p = unsafe { p.add(len) };
    }
}

/// Open the volume markers, assets and the log resolve against.
///
/// Driver form: re-resolve the pinned partition GUID against the live SFS
/// handle database on every call - removal of the medium yields
/// `NOT_FOUND`, never a different volume (audit N2). Application form:
/// the image's own boot volume, unchanged.
pub fn marker_fs() -> uefi::Result<ScopedProtocol<SimpleFileSystem>> {
    if let Some(guid) = armed() {
        let handles = boot::locate_handle_buffer(SearchType::ByProtocol(&SimpleFileSystem::GUID))?;
        for handle in handles.iter() {
            if partition_guid(*handle) == Some(guid) {
                // Safety: protocol open on a live handle from this session;
                // the scoped wrapper closes the protocol on drop.
                return unsafe {
                    boot::open_protocol::<SimpleFileSystem>(
                        OpenProtocolParams {
                            handle: *handle,
                            agent: boot::image_handle(),
                            controller: None,
                        },
                        OpenProtocolAttributes::GetProtocol,
                    )
                };
            }
        }
        return Err(uefi::Status::NOT_FOUND.into());
    }
    boot::get_image_file_system(boot::image_handle())
}
