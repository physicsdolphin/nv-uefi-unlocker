//! Firmware-resident entry and fail-closed authorization.
//!
//! FV images only register ReadyToBoot; BDS_ARCH presence is not a phase
//! indicator. File-loaded images execute at entry (TPL_APPLICATION).
//! ReadyToBoot runs at TPL_CALLBACK, so it permits ONLY explicitly pinned
//! unattended policy: it never polls ReadKeyStroke or lowers the TPL.
//! Attempts persist until an operator explicitly reauthorizes from the app;
//! returning from firmware is not proof that the OS booted successfully.
use crate::{
    chainload, dxe_policy as policy, keywait, logging, runner, timing, volumes, DXE_BUILD_ID,
    VERSION,
};
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, Ordering};
use log::info;
use uefi::boot::{self, EventType, Tpl};
use uefi::prelude::*;
use uefi::proto::device_path::media::FilePath;
use uefi::proto::loaded_image::LoadedImage;
use uefi::runtime::{self, VariableAttributes, VariableVendor};
use uefi::{guid, CStr16, Event, Guid};

const READY_TO_BOOT: Guid = guid!("7ce88fb3-4bd7-4679-87a8-a8d8dee50d2b");
pub const ARM_VENDOR: Guid = guid!("2e7f7609-a04c-4593-b88a-651a03812f2a");
pub const ARM_VARIABLE: &CStr16 = cstr16!("NvUnlockDxeArm");
const LATCH_VARIABLE: &CStr16 = cstr16!("NvUnlockDxeRan");
const FAIL_VARIABLE: &CStr16 = cstr16!("NvUnlockDxeFail");
pub const ATTRS_NV: VariableAttributes = VariableAttributes::NON_VOLATILE
    .union(VariableAttributes::BOOTSERVICE_ACCESS)
    .union(VariableAttributes::RUNTIME_ACCESS);
const ATTRS_BS: VariableAttributes = VariableAttributes::BOOTSERVICE_ACCESS;
const ARM_MARKER: &CStr16 = cstr16!(r"\tx-dxe-arm.txt");
const NOCONFIRM_MARKER: &CStr16 = cstr16!(r"\tx-dxe-noconfirm.txt");
const QUIET_MARKER: &CStr16 = cstr16!(r"\tx-quiet.txt");
const FLUSH_LINES_MARKER: &CStr16 = cstr16!(r"\tx-flush-lines.txt");
static ENTERED: AtomicBool = AtomicBool::new(false);

pub fn driver_entry() -> Status {
    // A filesystem image has a MEDIA_FILEPATH node. FV/unknown provenance
    // always defers; protocol presence says nothing about DXE completion.
    if file_loaded() {
        run_flow(false);
        Status::SUCCESS
    } else {
        match unsafe {
            boot::create_event_ex(
                EventType::NOTIFY_SIGNAL,
                Tpl::CALLBACK,
                Some(ready_to_boot),
                None,
                Some(&READY_TO_BOOT),
            )
        } {
            Ok(_) => Status::SUCCESS,
            Err(e) => e.status(),
        }
    }
}

fn file_loaded() -> bool {
    let Ok(image) = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle()) else {
        return false;
    };
    let Some(path) = image.file_path() else {
        return false;
    };
    path.node_iter()
        .any(|node| <&FilePath>::try_from(node).is_ok())
}

unsafe extern "efiapi" fn ready_to_boot(event: Event, _ctx: Option<NonNull<core::ffi::c_void>>) {
    // One-shot even when a boot option is retried. Closing our event does
    // not lower TPL or dispatch user-input operations inside notification.
    let _ = boot::close_event(event);
    run_flow(true);
}

// Bounded, stack-only reads: corrupt/oversized NVRAM must not allocate or
// panic before the persistent attempt budget and watchdog are established.
struct VariableData {
    bytes: [u8; 64],
    len: usize,
    attrs: VariableAttributes,
}
impl core::ops::Deref for VariableData {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}
fn var_read(name: &CStr16) -> uefi::Result<Option<VariableData>> {
    let mut bytes = [0u8; 64];
    match runtime::get_variable(name, &VariableVendor(ARM_VENDOR), &mut bytes) {
        Ok((data, attrs)) => {
            let len = data.len();
            Ok(Some(VariableData { bytes, len, attrs }))
        }
        Err(e) if e.status() == Status::NOT_FOUND => Ok(None),
        Err(e) => Err(e.status().into()),
    }
}

fn write_verified(name: &CStr16, attrs: VariableAttributes, bytes: &[u8]) -> uefi::Result {
    runtime::set_variable(name, &VariableVendor(ARM_VENDOR), attrs, bytes)?;
    match var_read(name)? {
        Some(actual) if &*actual == bytes && actual.attrs == attrs => Ok(()),
        _ => Err(Status::DEVICE_ERROR.into()),
    }
}

fn delete_verified(name: &CStr16) -> uefi::Result {
    match runtime::delete_variable(name, &VariableVendor(ARM_VENDOR)) {
        Ok(()) => {}
        Err(e) if e.status() == Status::NOT_FOUND => {}
        Err(e) => return Err(e),
    }
    match var_read(name)? {
        None => Ok(()),
        Some(_) => Err(Status::DEVICE_ERROR.into()),
    }
}

fn run_flow(in_notification: bool) {
    if ENTERED.swap(true, Ordering::SeqCst) {
        return;
    }
    // Strict one-run latch across image instances. Any error refuses work.
    if !matches!(var_read(LATCH_VARIABLE), Ok(None)) {
        return;
    }
    let grant = match var_read(ARM_VARIABLE) {
        Ok(Some(bytes)) => policy::decode(&bytes),
        _ => None,
    };
    let Some((identity, expected_policy)) = grant else {
        return;
    };
    if write_verified(LATCH_VARIABLE, ATTRS_BS, b"1").is_err() {
        return;
    }

    // No disk, console, calibration, or other transaction prep precedes the
    // persistent attempt increment. Malformed counters also fail closed.
    let count = match var_read(FAIL_VARIABLE) {
        Ok(bytes) => policy::attempt_count(bytes.as_deref()),
        Err(_) => None,
    };
    let Some(count) = count.filter(|n| *n < policy::ATTEMPT_LIMIT) else {
        return;
    };
    if write_verified(
        FAIL_VARIABLE,
        ATTRS_NV,
        if count == 0 { b"1" } else { b"2" },
    )
    .is_err()
    {
        return;
    }
    // ReadyToBoot occurs BEFORE EDK2 enables the OS-loader watchdog.
    // Own a 300s timer explicitly; unsupported/failed timer means no run.
    if boot::set_watchdog_timer(300, 0x10001, None).is_err() {
        return;
    }

    volumes::set_armed_volume(identity);
    let snapshot = match volumes::read_policy() {
        Ok(s) if policy::fingerprint(&s) == expected_policy => s,
        _ => return,
    };
    volumes::pin_policy(snapshot);
    let Some(marker) = chainload::boot_volume_read(ARM_MARKER) else {
        return;
    };
    let unattended = chainload::boot_volume_has(NOCONFIRM_MARKER);
    // ReadKeyStroke is APPLICATION-only. An event notification cannot
    // provide an interactive abort window; never silently remove it.
    if !unattended && (in_notification || !keywait::stdin_available()) {
        return;
    }

    timing::init();
    logging::init(VERSION);
    info!("FORM: firmware-resident DXE driver, build id '{DXE_BUILD_ID}', tx={}, notification={in_notification}", cfg!(feature = "tx"));
    info!(
        "ARM: NVUX2 volume {} + pinned manifest/control snapshot",
        guid_hex(&identity)
    );
    info!("FAILSAFE: attempt {}/{}, retained until explicit app reauthorization; firmware return is not OS health", count + 1, policy::ATTEMPT_LIMIT);
    info!("WATCHDOG: explicitly armed for 300s; retained on return, boot manager/OS owns subsequent policy");
    if chainload::boot_volume_has(QUIET_MARKER) {
        logging::set_quiet(true);
    }
    if let Some(content) = chainload::boot_volume_read(FLUSH_LINES_MARKER) {
        logging::set_flush_every_lines(chainload::parse_flush_lines(&content) as usize);
    }
    let confirmed = if unattended {
        info!("CONFIRM: pinned unattended policy - no keyboard window");
        keywait::Confirm::Proceed
    } else {
        let secs = chainload::parse_dxe_arm_seconds(&marker);
        info!("CONFIRM: {secs}s countdown, ANY key aborts");
        // No asset preparation here: the abort poll cannot be delayed by
        // multi-megabyte disk reads. Only the file-loaded APPLICATION path.
        keywait::wait_dxe_countdown(secs)
    };
    if matches!(confirmed, keywait::Confirm::Proceed) {
        runner::phases(confirmed);
        info!("FINAL: driver phases returned; OS health unconfirmed, attempt count retained");
    } else {
        info!("CONFIRM: aborted - zero GPU writes; attempt count retained");
    }
    logging::finish();
    // Refresh our timeout so shell use does not inherit a nearly expired
    // transaction timer. BDS sets its own timer before starting the loader.
    let _ = boot::set_watchdog_timer(300, 0x10001, None);
}

/// App-only authorization/disarm. An independent, explicit Space/Enter gate
/// precedes any NVRAM arm write even in a read-only or armed app build.
pub fn manage_arm() {
    const INSTALL: &CStr16 = cstr16!(r"\tx-dxe-arm-install.txt");
    const REMOVE: &CStr16 = cstr16!(r"\tx-dxe-arm-remove.txt");
    if chainload::boot_volume_has(REMOVE) {
        let result = delete_verified(ARM_VARIABLE);
        info!("DXE-ARM: disarm result {result:?}");
        return;
    }
    if !chainload::boot_volume_has(INSTALL) {
        return;
    }
    info!("DXE-ARM: persistent authorization pins ALL controls and the manifest. Check OS/GPU health before rearming. Space/Enter confirms within 30s; any other key or timeout cancels.");
    if !keywait::explicit_authorize(30) {
        info!("DXE-ARM: authorization cancelled");
        return;
    }
    let result = install_arm();
    info!("DXE-ARM: authorization result {result:?}");
}

fn install_arm() -> uefi::Result {
    let image = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle())?;
    let device = image.device().ok_or(Status::NOT_FOUND)?;
    let identity = volumes::partition_guid(device).ok_or(Status::UNSUPPORTED)?;
    drop(image);
    let snapshot = volumes::read_policy().map_err(uefi::Error::from)?;
    if snapshot[0].is_none() {
        return Err(Status::NOT_FOUND.into());
    }
    #[cfg(feature = "tx")]
    if snapshot.last().and_then(|s| s.as_ref()).is_none() {
        return Err(Status::NOT_FOUND.into());
    }
    let payload = policy::encode(identity, &snapshot);
    // Disarm before resetting attempts. Never reset the per-boot latch:
    // even reauthorization cannot permit a second run in this session.
    delete_verified(ARM_VARIABLE)?;
    write_verified(FAIL_VARIABLE, ATTRS_NV, b"0")?;
    write_verified(ARM_VARIABLE, ATTRS_NV, &payload)
}

pub fn guid_hex(identity: &[u8; 16]) -> alloc::string::String {
    identity.iter().map(|b| alloc::format!("{b:02x}")).collect()
}
