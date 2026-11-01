//! Firmware-resident DXE driver flow (docs/dxe-driver-ffs-feasibility.md,
//! audited in PR #100 - F1/F2/N1-N7/M1-M4 remediations).
//!
//! Dispatch: the FFS file carries a TRUE depex, so the DXE dispatcher runs
//! this entry in the first dispatch pass - and the entry must return
//! immediately. PciIo handles and the ESP filesystem only appear during the
//! BDS ConnectAll, and blocking here would stall the dispatcher forever (BDS
//! would never start). The work therefore runs in the ReadyToBoot
//! notification, which BDS signals immediately before it starts the boot
//! option - the same window the USB application occupies. Firmware Setup
//! never signals ReadyToBoot, so setup menus are untouched.
//!
//! Loaded after BDS is up (UEFI shell `load` - the zero-flash smoke path),
//! this boot option's ReadyToBoot has already fired, so the flow runs
//! directly in the entry.
//!
//! Authority (audit F1/M1): arming is NOT a filename. The NVRAM variable
//! `NvUnlockDxeArm` (project GUID, NV|BS|RT) is written only by the USB
//! application's installer mode - the operator's deliberate act of booting
//! the stick they hold - and its payload pins the GPT partition GUID of the
//! armed volume. A file dropped on any FAT volume cannot forge it; the
//! marker file remains as a presence flag on that one volume and must be a
//! regular file (audit N5).
//!
//! Failsafe (audit F2): the consecutive-failure counter is an NVRAM
//! variable, not a file - it survives a pulled stick, a read-only volume and
//! volume-identity changes. Read failure = skip this boot; write failure =
//! skip this boot. Both fail closed. A Rust panic does NOT return cleanly:
//! uefi's panic handler resets the platform (SHUTDOWN). The counter is
//! incremented before anything that can panic meaningfully, so panic loops
//! still advance it; per-bin panic handlers need a crate split (follow-up).
//!
//! Watchdog (audit M3): this form never touches the platform watchdog. The
//! BDS-armed 5-minute timer stays armed - a hang self-recovers, and the
//! counter turns the next boot into a skip.
//!
//! Gates: arm variable + regular-file marker on the GUID-matched volume,
//! bounded waits, one run per boot shared across image instances (NVRAM
//! latch, audit N6). The countdown with any-key abort is the safe exit; an
//! explicit `\tx-dxe-noconfirm.txt` opts out of the window for headless
//! operation; absent console input without that marker aborts (audit N1).
//! Aborting or never arming always returns cleanly so BDS continues.

use core::ptr::NonNull;

use log::info;
use uefi::boot::{self, EventType, OpenProtocolAttributes, OpenProtocolParams, SearchType, Tpl};
use uefi::prelude::*;
use uefi::proto::console::text::Input;
use uefi::proto::media::file::{File, FileAttribute, FileMode};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::runtime::{self, VariableAttributes};
use uefi::{guid, Event, CStr16, Guid, Handle, Identify};

use crate::banner;
use crate::chainload;
use crate::keywait;
use crate::logging;
use crate::runner;
use crate::timing;
use crate::volumes;
use crate::{VERSION, DXE_BUILD_ID};

/// ReadyToBoot event group (UEFI spec 2.0 event group; MdeModulePkg
/// gEfiEventReadyToBootGuid).
const READY_TO_BOOT: Guid = guid!("7ce88fb3-4bd7-4679-87a8-a8d8dee50d2b");
/// BDS architectural protocol: installed once the DXE dispatcher handed
/// control to BDS. Present = we were loaded after DXE (shell `load`).
const BDS_ARCH: Guid = guid!("665e3ff6-46cc-11d4-9a38-0090273fc14d");

/// Vendor GUID for every variable this driver reads or writes: the project
/// FFS GUID (ffspack::NVUNLOCK_DXE_GUID), so operators can enumerate all of
/// them with `dmpstore -g`.
pub const ARM_VENDOR: Guid = guid!("2e7f7609-a04c-4593-b88a-651a03812f2a");
/// The arm variable: payload = b"NVUX1" + the armed volume's GPT partition
/// GUID. Written only by the application's installer mode.
pub const ARM_VARIABLE: &CStr16 = cstr16!("NvUnlockDxeArm");
/// Boot-scoped one-run latch (audit N6): BS-only attribute = memory-backed,
/// gone at reset, shared by every image instance in this boot.
const LATCH_VARIABLE: &CStr16 = cstr16!("NvUnlockDxeRan");
/// Consecutive-failure counter (audit F2): NVRAM, decimal ASCII.
const FAIL_VARIABLE: &CStr16 = cstr16!("NvUnlockDxeFail");
pub const ARM_MAGIC: &[u8; 5] = b"NVUX1";
/// Non-volatile variable attributes for operator-persistent state.
pub const ATTRS_NV: VariableAttributes = VariableAttributes::NON_VOLATILE
    .union(VariableAttributes::BOOTSERVICE_ACCESS)
    .union(VariableAttributes::RUNTIME_ACCESS);
/// Boot-service-only (volatile) attributes for the per-boot latch.
const ATTRS_BS: VariableAttributes = VariableAttributes::BOOTSERVICE_ACCESS;

/// The armed volume must still carry this as a REGULAR file (audit N5) - a
/// presence flag on the operator-authorized volume, never the authority.
const ARM_MARKER: &CStr16 = cstr16!(r"\tx-dxe-arm.txt");
/// Explicit unattended mode: skips the countdown window (audit N1 - the
/// window itself can no longer be shortened to 0 by marker content; the
/// floor is 1..=30 like the application form).
const NOCONFIRM_MARKER: &CStr16 = cstr16!(r"\tx-dxe-noconfirm.txt");
/// Console-suppression knob, same file and semantics as the app.
const QUIET_MARKER: &CStr16 = cstr16!(r"\tx-quiet.txt");
/// Log-flush cadence knob, same file and semantics as the app.
const FLUSH_LINES_MARKER: &CStr16 = cstr16!(r"\tx-flush-lines.txt");
/// Two consecutive firmware-side failures (hang/panic -> reset) skip the
/// armed run so the machine still boots Windows (audit F2: NVRAM-backed,
/// fail-closed on read AND write).
const FAIL_LIMIT: u64 = 2;

/// Entry: dispatch must return immediately (see module docs). No console
/// output here - at DXE dispatch time ConOut is not guaranteed to exist.
pub fn driver_entry() -> Status {
    if bds_is_up() {
        run_flow();
    } else {
        // The event is intentionally kept for the image's lifetime: the
        // driver stays resident and fires at every ReadyToBoot signal
        // (the boot latch inside `run_flow` makes retries inert).
        let _ = unsafe {
            boot::create_event_ex(
                EventType::NOTIFY_SIGNAL,
                Tpl::CALLBACK,
                Some(ready_to_boot),
                None,
                Some(&READY_TO_BOOT),
            )
        };
    }
    Status::SUCCESS
}

unsafe extern "efiapi" fn ready_to_boot(_event: Event, _ctx: Option<NonNull<core::ffi::c_void>>) {
    run_flow();
}

/// Was BDS started? (DXE dispatch order: FALSE; shell `load`: TRUE.)
fn bds_is_up() -> bool {
    boot::locate_handle_buffer(SearchType::ByProtocol(&BDS_ARCH))
        .map(|b| !b.is_empty())
        .unwrap_or(false)
}

/// Is there a console input protocol to poll? (audit N1: `with_stdin`
/// asserts - panics - when ConIn is absent; the driver must ask first.)
fn conin_available() -> bool {
    boot::locate_handle_buffer(SearchType::ByProtocol(&Input::GUID))
        .map(|b| !b.is_empty())
        .unwrap_or(false)
}

/// Read one of our variables. `Ok(None)` = not set; `Err` = anything else
/// (hardware error, security violation) - callers fail closed.
fn var_read(name: &CStr16) -> Result<Option<alloc::boxed::Box<[u8]>,>, uefi::Error> {
    match runtime::get_variable_boxed(name, &uefi::runtime::VariableVendor(ARM_VENDOR)) {
        Ok((data, _attr)) => Ok(Some(data)),
        Err(e) if e.status() == uefi::Status::NOT_FOUND => Ok(None),
        Err(e) => Err(e),
    }
}

/// Scan the SFS handle database for the volume whose GPT partition GUID
/// matches the arm variable, and require the arm marker to exist there as a
/// REGULAR file (audit N5). First (and only) match wins.
fn select_armed_volume(guid: [u8; 16]) -> Option<Handle> {
    let handles = boot::locate_handle_buffer(SearchType::ByProtocol(&SimpleFileSystem::GUID)).ok()?;
    for handle in handles.iter() {
        if volumes::partition_guid(*handle) != Some(guid) {
            continue;
        }
        // Safety: GetProtocol open on a handle from this session.
        let Ok(mut fs) = (unsafe {
            boot::open_protocol::<SimpleFileSystem>(
                OpenProtocolParams {
                    handle: *handle,
                    agent: boot::image_handle(),
                    controller: None,
                },
                OpenProtocolAttributes::GetProtocol,
            )
        }) else {
            continue;
        };
        let Ok(mut dir) = fs.open_volume() else {
            continue;
        };
        let regular = dir
            .open(ARM_MARKER, FileMode::Read, FileAttribute::empty())
            .ok()
            .and_then(|f| f.into_regular_file());
        if regular.is_some() {
            return Some(*handle);
        }
    }
    None
}

/// The armed run. Every early exit before logging exists stays SILENT (an
/// unarmed or malformed arm must produce zero console noise); everything
/// after `logging::init` logs and returns cleanly.
fn run_flow() {
    // Per-boot latch, shared by all image instances (audit N6): a second
    // `load` (or FV instance + shell load) must not re-run the transaction
    // in the same power cycle. Latch read failure is fail-open here: the
    // device-state guards remain, and the latch is belt-and-suspenders.
    match var_read(LATCH_VARIABLE) {
        Ok(Some(_)) => return,
        _ => {}
    }

    // Authority (audit F1): the NVRAM arm variable, payload = magic + the
    // armed volume's GPT partition GUID. Anything else (absent, unreadable,
    // malformed) = not armed, silently.
    let guid = match var_read(ARM_VARIABLE) {
        Ok(Some(data)) if data.len() == 21 && &data[..5] == ARM_MAGIC => {
            let mut g = [0u8; 16];
            g.copy_from_slice(&data[5..21]);
            g
        }
        _ => return,
    };

    // Volume identity (audit F1/F2-B/N2): GUID match against the live handle
    // database + regular-file marker on that volume. Unmatched = not armed.
    let Some(_volume) = select_armed_volume(guid) else { return };
    volumes::set_armed_volume(guid);

    // Boot latch: set after the arm decision, before any work.
    let _ = runtime::set_variable(
        LATCH_VARIABLE,
        &uefi::runtime::VariableVendor(ARM_VENDOR),
        ATTRS_BS,
        b"1",
    );

    // Everything below can log: timestamps are run-relative (audit r2 note:
    // they used to count from DXE dispatch, a different meaning than the
    // app's evidence files).
    timing::init();
    logging::init(VERSION);
    banner::print(VERSION);
    info!("FORM: firmware-resident DXE driver, build id '{DXE_BUILD_ID}', tx={}", cfg!(feature = "tx"));
    info!("ARM: NVRAM variable NvUnlockDxeArm -> volume partition GUID {}", guid_hex(&guid));

    // Quiet + flush knobs: same files, same semantics as the app, resolved
    // against the armed volume.
    if chainload::boot_volume_has(QUIET_MARKER) {
        if logging::set_quiet(true) {
            info!(
                "QUIET: \\tx-quiet.txt present - console shows verdict lines only; every line still lands in \\loader.log"
            );
        } else {
            info!(
                "QUIET: \\tx-quiet.txt present but the log file is unavailable - console stays complete (it is the only record)"
            );
        }
    }
    if let Some(content) = chainload::boot_volume_read(FLUSH_LINES_MARKER) {
        let n = chainload::parse_flush_lines(&content);
        logging::set_flush_every_lines(n as usize);
        if n == 0 {
            info!("FLUSH: \\tx-flush-lines.txt - count-based flush off (stage-marker lines only)");
        } else {
            info!("FLUSH: \\tx-flush-lines.txt - every {n} lines");
        }
    }

    // Failsafe (audit F2): NVRAM counter, read fail-closed, write fail-closed.
    let fails = match read_fail_count() {
        Ok(n) => n,
        Err(e) => {
            info!("FAILSAFE: counter unreadable ({e:?}) - skipping this boot (fail-closed)");
            logging::finish();
            return;
        }
    };
    if fails >= FAIL_LIMIT {
        info!(
            "FAILSAFE: {fails} consecutive failures - skipping this boot; disarm by deleting the NvUnlockDxeArm variable (UEFI shell dmpstore, or the stick's \\tx-dxe-arm-remove.txt)"
        );
        logging::finish();
        return;
    }
    if let Err(e) = write_fail_count(fails + 1) {
        info!("FAILSAFE: counter write failed ({e:?}) - skipping this boot (fail-closed)");
        logging::finish();
        return;
    }

    // CONFIRM (audit N1): the arm variable is the intent declaration; the
    // countdown (first marker line, 1..=30, default 5 - the floor matches
    // the app, 0 is unreachable by content) is the safe exit - ANY key
    // aborts into a clean return. `\tx-dxe-noconfirm.txt` is the explicit
    // headless opt-out; absent console input without it aborts (fail-
    // closed), because an unreachable abort window is not a window.
    let marker = chainload::boot_volume_read(ARM_MARKER);
    let secs = marker.as_deref().map(chainload::parse_dxe_arm_seconds);
    let Some(secs) = secs else {
        info!("CONFIRM: \\tx-dxe-arm.txt unreadable on the armed volume - ABORT (fail-closed), no GPU writes this boot");
        clear_fail_count();
        logging::finish();
        return;
    };
    let confirmed = if chainload::boot_volume_has(NOCONFIRM_MARKER) {
        info!("CONFIRM: \\tx-dxe-noconfirm.txt present - unattended mode, no abort window");
        keywait::Confirm::Proceed
    } else if !conin_available() {
        info!("CONFIRM: no console input protocol - aborting the armed run (fail-closed)");
        keywait::Confirm::Abort
    } else {
        info!(
            "CONFIRM: \\tx-dxe-arm.txt present - ARMED: {secs}s countdown auto-runs, ANY key aborts (firmware-resident form)"
        );
        #[cfg(feature = "tx")]
        {
            crate::tx_prep::armed_confirm_with_prep(secs)
        }
        #[cfg(not(feature = "tx"))]
        {
            keywait::wait_confirm_armed(secs)
        }
    };

    if let keywait::Confirm::Proceed = confirmed {
        runner::phases(confirmed);
        // The transaction machinery always returns cleanly (per-card
        // failures are contained inside tx_one_card); firmware side done.
        clear_fail_count();
        info!("FINAL: driver flow complete; BDS continues to the boot option");
    } else {
        info!("CONFIRM: aborted - zero GPU writes, BDS continues to the boot option");
        clear_fail_count();
    }
    logging::finish();
}

pub fn guid_hex(guid: &[u8; 16]) -> alloc::string::String {
    let mut s = alloc::string::String::new();
    for b in guid {
        s.push_str(&alloc::format!("{b:02x}"));
    }
    s
}

/// Consecutive-failure counter, NVRAM-backed (audit F2). Read: absent = 0;
/// any other error = fail-closed.
fn read_fail_count() -> Result<u64, uefi::Error> {
    match var_read(FAIL_VARIABLE)? {
        None => Ok(0),
        Some(data) => {
            let mut n: u64 = 0;
            for &b in data.iter().take(20) {
                if b.is_ascii_digit() {
                    n = n.saturating_mul(10).saturating_add(u64::from(b - b'0'));
                } else {
                    break;
                }
            }
            Ok(n)
        }
    }
}

fn write_fail_count(n: u64) -> Result<(), uefi::Error> {
    runtime::set_variable(
        FAIL_VARIABLE,
        &uefi::runtime::VariableVendor(ARM_VENDOR),
        ATTRS_NV,
        alloc::format!("{n}").as_bytes(),
    )
}

fn clear_fail_count() {
    let _ = runtime::delete_variable(FAIL_VARIABLE, &uefi::runtime::VariableVendor(ARM_VENDOR));
}
