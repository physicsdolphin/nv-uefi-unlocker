//! Firmware-resident DXE driver flow (docs/dxe-driver-ffs-feasibility.md).
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
//! Gates (feasibility doc §5): arm marker on a scanned SFS volume, a
//! consecutive-failure counter file, bounded waits, one run per boot. The
//! countdown with any-key abort is the safe exit; aborting or never arming
//! always returns cleanly so BDS continues to the boot option.

use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, Ordering};

use log::info;
use uefi::boot::{self, EventType, OpenProtocolAttributes, OpenProtocolParams, SearchType, Tpl};
use uefi::prelude::*;
use uefi::proto::media::file::{File, FileAttribute, FileMode};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::{guid, Event, CStr16, Guid, Handle, Identify};

use crate::chainload;
use crate::keywait;
use crate::logging;
use crate::runner;
use crate::timing;
use crate::volumes;
use crate::VERSION;

/// ReadyToBoot event group (UEFI spec 2.0 event group; MdeModulePkg
/// gEfiEventReadyToBootGuid).
const READY_TO_BOOT: Guid = guid!("7ce88fb3-4bd7-4679-87a8-a8d8dee50d2b");
/// BDS architectural protocol: installed once the dispatcher hands control
/// to BDS. Present = we were loaded after DXE (shell `load`).
const BDS_ARCH: Guid = guid!("665e3ff6-46cc-11d4-9a38-0090273fc14d");

/// The DXE form's arm marker - the operator's intent declaration on the
/// volume that also carries markers, assets and the log. Deliberately NOT
/// the USB stick's `\i_know_what_i_am_doing.txt`: a stick left armed must
/// never arm a firmware-resident driver.
const ARM_MARKER: &CStr16 = cstr16!(r"\tx-dxe-arm.txt");
/// Console-suppression knob, same file and semantics as the app.
const QUIET_MARKER: &CStr16 = cstr16!(r"\tx-quiet.txt");
/// Log-flush cadence knob, same file and semantics as the app.
const FLUSH_LINES_MARKER: &CStr16 = cstr16!(r"\tx-flush-lines.txt");
/// Consecutive-failure counter (see `run_flow`). Plain file on the armed
/// volume: survives reboots, user-deletable from Windows, no runtime
/// variable services involved.
const FAIL_MARKER: &CStr16 = cstr16!(r"\nvunlock-dxe-fail.txt");
/// Two consecutive firmware-side failures (hang/panic -> power cycle) skip
/// the armed run so the machine still reaches Windows. OS-side failures are
/// out of its reach - the operator disarms by deleting the marker files.
const FAIL_LIMIT: u64 = 2;

/// Entry: dispatch must return immediately (see module docs). No console
/// output here - at DXE dispatch time ConOut is not guaranteed to exist.
pub fn driver_entry() -> Status {
    timing::init();
    if bds_is_up() {
        run_flow();
    } else {
        // The event is intentionally kept for the image's lifetime: the
        // driver stays resident and fires at every ReadyToBoot signal
        // (the one-shot guard inside `run_flow` makes retries inert).
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

/// Scan every SimpleFileSystem volume for the arm marker; first hit wins.
/// All reads-only and bounded - the not-armed path costs one open per
/// volume and never logs (zero console noise on unarmed boots).
fn select_arm_volume() -> Option<Handle> {
    let handles = boot::locate_handle_buffer(SearchType::ByProtocol(&SimpleFileSystem::GUID)).ok()?;
    for handle in handles.iter() {
        // Safety: GetProtocol open on a handle from this session; the
        // scoped wrapper closes it on drop.
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
        if dir.open(ARM_MARKER, FileMode::Read, FileAttribute::empty()).is_ok() {
            return Some(*handle);
        }
    }
    None
}

static RAN: AtomicBool = AtomicBool::new(false);

/// The armed run: volume selection, logging, failsafe, countdown confirm
/// (the safe exit), then the shared phase sequence. Every path returns
/// cleanly - BDS continues to the boot option.
fn run_flow() {
    // One run per boot: ReadyToBoot fires per boot-option start, so a
    // failed first boot attempt that falls back to another option must not
    // re-run the transaction mid-session (the card guards also refuse).
    if RAN.swap(true, Ordering::SeqCst) {
        return;
    }

    // Volume selection BEFORE logging: the arm marker designates the volume
    // that carries markers, assets and the log.
    let Some(volume) = select_arm_volume() else { return };
    volumes::set_selected_volume(volume);
    logging::init(VERSION);

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

    // Failsafe (feasibility doc §5.3): increment before any work, clear on
    // clean completion. A hang/panic leaves the count; two consecutive
    // failures skip the armed run so the machine still boots Windows.
    let fails = read_fail_count();
    if fails >= FAIL_LIMIT {
        info!(
            "FAILSAFE: {fails} consecutive failures - skipping this boot; disarm by deleting \\nvunlock-dxe-fail.txt and \\tx-dxe-arm.txt on the armed volume"
        );
        logging::finish();
        return;
    }
    write_fail_count(fails + 1);

    // Watchdog parity with the app: the boot manager arms a 5-minute
    // watchdog before StartImage; a ~20s transaction must not race it.
    match boot::set_watchdog_timer(0, 0x10000, None) {
        Ok(()) => info!("WATCHDOG: disabled"),
        Err(e) => info!("WATCHDOG: disable failed: {:?}", e.status()),
    }

    // CONFIRM. The arm marker IS the intent declaration; the countdown
    // (first marker line, 0..=30s, default 5; 0 = no wait) is the safe
    // exit - ANY key aborts into a clean return. TX builds interleave the
    // read-only prep into the window, exactly like the app.
    let secs = chainload::boot_volume_read(ARM_MARKER)
        .map(|c| chainload::parse_dxe_arm_seconds(&c))
        .unwrap_or(5);
    info!(
        "CONFIRM: \\tx-dxe-arm.txt present - ARMED: {secs}s countdown auto-runs, ANY key aborts (firmware-resident form)"
    );
    #[cfg(feature = "tx")]
    let confirmed = crate::tx_prep::armed_confirm_with_prep(secs);
    #[cfg(not(feature = "tx"))]
    let confirmed = keywait::wait_confirm_armed(secs);

    if let keywait::Confirm::Proceed = confirmed {
        runner::phases(confirmed);
        // The transaction machinery always returns cleanly (per-card
        // failures are contained inside tx_one_card); firmware side done.
        clear_fail_count();
        info!("FINAL: driver flow complete; BDS continues to the boot option");
    } else {
        info!("CONFIRM: aborted by key - zero GPU writes, BDS continues to the boot option");
        clear_fail_count();
    }
    logging::finish();
}

fn read_fail_count() -> u64 {
    match chainload::boot_volume_read(FAIL_MARKER) {
        Some(content) => chainload::parse_fail_count(&content),
        None => 0,
    }
}

fn write_fail_count(n: u64) {
    let Ok(mut fs) = volumes::marker_fs() else {
        info!("FAILSAFE: counter write failed (volume unavailable)");
        return;
    };
    let Ok(mut dir) = fs.open_volume() else {
        info!("FAILSAFE: counter write failed (open_volume)");
        return;
    };
    // EFI open never truncates: delete, then recreate (fresh-file idiom of
    // logging::open_fresh_log).
    if let Ok(handle) = dir.open(FAIL_MARKER, FileMode::ReadWrite, FileAttribute::empty()) {
        if let Some(reg) = handle.into_regular_file() {
            let _ = reg.delete();
        }
    }
    let Ok(file) = dir.open(FAIL_MARKER, FileMode::CreateReadWrite, FileAttribute::empty()) else {
        info!("FAILSAFE: counter create failed - failsafe is best-effort, run continues");
        return;
    };
    let Some(mut reg) = file.into_regular_file() else { return };
    let text = alloc::format!("{n}");
    let _ = reg.write(text.as_bytes());
}

fn clear_fail_count() {
    let Ok(mut fs) = volumes::marker_fs() else { return };
    let Ok(mut dir) = fs.open_volume() else { return };
    if let Ok(handle) = dir.open(FAIL_MARKER, FileMode::ReadWrite, FileAttribute::empty()) {
        if let Some(reg) = handle.into_regular_file() {
            let _ = reg.delete();
        }
    }
}
