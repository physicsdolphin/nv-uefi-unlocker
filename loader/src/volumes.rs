//! Marker/asset/log volume resolution.
//!
//! The USB application reads markers and assets from its own boot volume
//! (`get_image_file_system` of the image handle). The firmware-resident DXE
//! driver has no SFS self-volume - it is loaded from an FV, so that call
//! fails and every marker/asset would read as absent, silently disarming the
//! driver. At run time it therefore scans all SimpleFileSystem handles for
//! the arm marker and pins that volume here (`dxe::select_arm_volume`);
//! every reader resolves through [`marker_fs`]. With nothing pinned the
//! helper falls back to the image volume, so application behavior stays
//! byte-identical.

use core::sync::atomic::{AtomicUsize, Ordering};

use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams, ScopedProtocol};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::Handle;

static SELECTED: AtomicUsize = AtomicUsize::new(0);

/// Pin the volume all marker/asset/log reads resolve against.
pub fn set_selected_volume(h: Handle) {
    SELECTED.store(h.as_ptr() as usize, Ordering::Relaxed);
}

/// The pinned volume, if any.
pub fn selected_volume() -> Option<Handle> {
    let p = SELECTED.load(Ordering::Relaxed);
    if p == 0 {
        return None;
    }
    // Safety: stored from a handle produced by locate_handle_buffer in this
    // boot session; boot-services handles stay valid until ExitBootServices.
    unsafe { Handle::from_ptr(p as *mut _) }
}

/// Open the volume markers, assets and the log resolve against: the
/// driver-selected volume when pinned, else the image's own boot volume.
/// GetProtocol on an SFS handle mirrors `chainload::open_volume_logged`.
pub fn marker_fs() -> uefi::Result<ScopedProtocol<SimpleFileSystem>> {
    if let Some(h) = selected_volume() {
        // Safety: protocol open on a live handle from this session; the
        // scoped wrapper closes the protocol on drop, matching the
        // image-volume path's ownership.
        unsafe {
            return boot::open_protocol::<SimpleFileSystem>(
                OpenProtocolParams { handle: h, agent: boot::image_handle(), controller: None },
                OpenProtocolAttributes::GetProtocol,
            );
        }
    }
    boot::get_image_file_system(boot::image_handle())
}
