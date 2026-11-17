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
    for node in dp.node_iter() {
        // The iterator provides a bounded node, including its 4-byte header.
        let bytes = unsafe { core::slice::from_raw_parts(node.as_ffi_ptr().cast::<u8>(), node.length() as usize) };
        if bytes[0] == DEV_PATH_MEDIA && bytes[1] == DEV_PATH_MEDIA_HARD_DRIVE
            && bytes.len() >= HARD_DRIVE_NODE_MIN_LEN {
            let mut identity = [0u8; 16];
            if bytes[41] == SIG_TYPE_GPT {
                identity.copy_from_slice(&bytes[24..40]);
                return Some(identity);
            }
            if bytes[41] == SIG_TYPE_MBR {
                identity[..4].copy_from_slice(&bytes[24..28]);
                identity[4..12].copy_from_slice(&bytes[8..16]);
                identity[12] = SIG_TYPE_MBR;
                return Some(identity);
            }
        }
    }
    None
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

struct PolicyCell(UnsafeCell<Option<crate::dxe_policy::Snapshot>>);
// Published once before phases; no callbacks mutate this snapshot.
unsafe impl Sync for PolicyCell {}
static POLICY: PolicyCell = PolicyCell(UnsafeCell::new(None));

/// Exact-size reads, including empty files; errors never become absence.
/// Unlike the UI marker reader, oversized policy files are not truncated.
pub fn read_small_file(path: &str, cap: usize) -> Result<Option<alloc::vec::Vec<u8>>, uefi::Status> {
    use uefi::proto::media::file::{File, FileAttribute, FileInfo, FileMode};
    let path = uefi::CString16::try_from(path).map_err(|_| uefi::Status::INVALID_PARAMETER)?;
    let mut fs = marker_fs().map_err(|e| e.status())?;
    let mut root = fs.open_volume().map_err(|e| e.status())?;
    let handle = match root.open(&path, FileMode::Read, FileAttribute::empty()) {
        Ok(f) => f,
        Err(e) if e.status() == uefi::Status::NOT_FOUND => return Ok(None),
        Err(e) => return Err(e.status()),
    };
    let mut file = handle.into_regular_file().ok_or(uefi::Status::INVALID_PARAMETER)?;
    let size = file.get_boxed_info::<FileInfo>().map_err(|e| e.status())?.file_size();
    if size > cap as u64 { return Err(uefi::Status::BAD_BUFFER_SIZE); }
    let mut out = alloc::vec![0; size as usize];
    let mut got = 0;
    while got < out.len() {
        let n = file.read(&mut out[got..]).map_err(|e| e.status())?;
        if n == 0 { return Err(uefi::Status::DEVICE_ERROR); }
        got += n;
    }
    let mut tail = [0];
    if file.read(&mut tail).map_err(|e| e.status())? != 0 {
        return Err(uefi::Status::BAD_BUFFER_SIZE);
    }
    Ok(Some(out))
}

pub fn read_policy() -> Result<crate::dxe_policy::Snapshot, uefi::Status> {
    crate::dxe_policy::CONFIG_PATHS.iter().map(|path| {
        read_small_file(path, if *path == crate::dxe_policy::MANIFEST { 65536 } else { 4096 })
    }).collect()
}

pub fn pin_policy(snapshot: crate::dxe_policy::Snapshot) {
    // Single publisher before any transaction, boot latch acquired first.
    unsafe { *POLICY.0.get() = Some(snapshot); }
}

/// Resident controls are immutable for the run. Unknown paths are absent,
/// so introducing a new control cannot silently bypass reauthorization.
pub fn policy_file(path: &str) -> Option<alloc::vec::Vec<u8>> {
    let policy = unsafe { &*POLICY.0.get() };
    let i = crate::dxe_policy::CONFIG_PATHS.iter().position(|p| p.eq_ignore_ascii_case(path))?;
    policy.as_ref()?.get(i)?.clone()
}
