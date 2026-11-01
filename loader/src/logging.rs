//! Dual logger: console + `\loader.log` on the marker volume (USB stick for
//! the application form; the NVRAM-armed volume for the DXE driver form -
//! see `volumes` and the PR #100 audit).
//!
//! The UEFI Shell's `> file` redirection proved unreliable on the target
//! firmware (empty capture), so we own the file ourselves. This also
//! pre-implements M1's "write samples to USB FAT" requirement.
//!
//! Constitution note: the log file lives on the operator-designated volume
//! and is the only thing the application form ever writes. The DXE driver
//! form additionally writes its failsafe state to NVRAM variables (not
//! files) and rotates the previous log to `\loader-prev.log` instead of
//! destroying it (audit F3); markers, assets and logs on an ESP are placed
//! there BY the operator, never by an installer of ours.

use alloc::format;
use core::cell::UnsafeCell;
use log::{Level, Log};
use uefi::proto::media::file::{File, FileAttribute, FileHandle, FileMode, RegularFile};
use uefi::{cstr16, system, CStr16, CString16};

pub const LOG_PATH: &CStr16 = cstr16!(r"\loader.log");

/// Flush every 16th line, and always on stage/verdict lines. FAT only
/// commits size/dirent on Flush, so the markers below are what survives a
/// hang or mid-call reset - exactly what the 2026-10-27 Z790 diagnosis
/// needed (the log ended at "POST-FLR: verified"). A Flush per line is
/// visible on slow USB media (the TU106 run of 2026-10-28 spent noticeable
/// seconds on ~415 flushed lines); the 2026-11-01 TU106 run measured four
/// ~1.2 s stalls all aligned to flush events (USB flash GC shape), so the
/// count cadence is a knob: `\tx-flush-lines.txt` on the stick root sets it
/// (0 = marker lines only). Raising it trades forensics resolution for
/// fewer flush stalls - the marker lines below always flush regardless.
const FLUSH_EVERY_LINES_DEFAULT: usize = 16;
static FLUSH_EVERY_LINES: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(FLUSH_EVERY_LINES_DEFAULT);

/// Set the count-based flush cadence (caller clamps; 0 disables
/// count-based flushes, marker lines still flush). Default 16.
pub fn set_flush_every_lines(n: usize) {
    FLUSH_EVERY_LINES.store(n, core::sync::atomic::Ordering::Relaxed);
}

const FLUSH_MARKERS: &[&str] = &[
    "PHASE", "RESULT", "POST-", "FINAL", "RECONNECT", "DETACH", "CHAIN",
    "SELECTOR", "HANDOFF", "WATCHDOG", "AUTH-SCAN end",
    // DXE-form evidence anchors (PR #100 audit F3/N4): provenance, arming
    // and guard-override visibility survive a hang with the run.
    "FORM", "ARM:", "FAILSAFE", "OVERRIDES",
];

/// Console whitelist for quiet mode (issue #38): with `\tx-quiet.txt` on
/// the stick root, only lines carrying one of these substrings are mirrored
/// to the screen; every line still goes to the file first. Deliberately
/// tighter than FLUSH_MARKERS - the file keeps its full evidence cadence,
/// the screen only keeps operator-facing verdicts (on an iGPU GOP that
/// renders ~1 line/s the full mirror cost minutes per run; the TU106 stick
/// logged 415+ flushed lines). Textually mirrored in sim/src/log_gate.rs
/// for shape tests.
const CONSOLE_QUIET_MARKERS: &[&str] = &[
    "nv-uefi-unlocker", "TX BUILD", // version + build-class banner
    "PROMPT", "CONFIRM", // interaction
    "PHASE", "SELECTOR", "HANDOFF", // stage boundaries
    "RESULT", "FINAL", "TX:", "CHAIN", "QUIET", // verdicts
    "FORM", "ARM:", "FAILSAFE", "OVERRIDES", // DXE-form provenance (audit F3/N4)
];

/// Pure matcher for [`CONSOLE_QUIET_MARKERS`]; the sim mirror
/// (sim/src/log_gate.rs) must stay textually identical.
fn console_visible(line: &str) -> bool {
    CONSOLE_QUIET_MARKERS.iter().any(|m| line.contains(m))
}

/// Quiet only takes effect while the file log is alive: when the file
/// could not be opened the console is the only record and must stay
/// complete. Pure decision, mirrored in sim/src/log_gate.rs.
fn quiet_effective(request: bool, file_open: bool) -> bool {
    request && file_open
}

struct DualLogger {
    file: UnsafeCell<Option<RegularFile>>,
    // set after a failed open so we log the reason exactly once
    file_error_reported: UnsafeCell<bool>,
    lines: UnsafeCell<usize>,
    // issue #38: skip the console mirror for non-verdict lines
    quiet: UnsafeCell<bool>,
}

static LOGGER: DualLogger = DualLogger {
    file: UnsafeCell::new(None),
    file_error_reported: UnsafeCell::new(false),
    lines: UnsafeCell::new(0),
    quiet: UnsafeCell::new(false),
};

// Safety: UEFI applications are single-threaded during boot services.
unsafe impl Sync for DualLogger {}
unsafe impl Send for DualLogger {}

impl Log for DualLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        // Stage timing (issue #38): a calibrated stamp per line separates
        // slow-GOP echo cost from real transaction waits in the record.
        let line = match crate::timing::ms_since_entry() {
            Some(ms) => format!(
                "[{ms:>7}ms] {:5}: {}\r\n",
                record.level(),
                format_args!("{}: {}", record.target(), record.args())
            ),
            None => format!(
                "[{:5}] {}: {}\r\n",
                record.level(),
                record.target(),
                record.args()
            ),
        };

        // File mirror FIRST, then console: a dangling console chain is
        // itself a suspected crash site, so the stick must already hold the
        // line before it is mirrored. Flush policy: see FLUSH_MARKERS above.
        // Safety: single-threaded boot services; pointer use is confined here.
        unsafe {
            let n = *self.lines.get() + 1;
            *self.lines.get() = n;
            let fe = FLUSH_EVERY_LINES.load(core::sync::atomic::Ordering::Relaxed);
            let flush = (fe > 0 && n % fe == 0)
                || FLUSH_MARKERS.iter().any(|m| line.contains(m));
            if let Some(f) = (*self.file.get()).as_mut() {
                let ok = f.write(line.as_bytes()).is_ok() && (!flush || f.flush().is_ok());
                if !ok {
                    // Media pulled: stop trying, keep console alive.
                    *self.file.get() = None;
                }
            }
        }

        // Console mirror. Quiet mode (issue #38) keeps verdict-class lines
        // only; the stick already holds every line (file mirror ran first).
        let quiet = unsafe { *self.quiet.get() };
        if crate::keywait::stdout_available() && (!quiet || console_visible(&line)) {
            if let Ok(s) = CString16::try_from(line.as_str()) {
                system::with_stdout(|out| {
                    let _ = out.output_string_lossy(&s);
                });
            }
        }
    }

    fn flush(&self) {}
}

/// Install the dual logger and open a fresh `\loader.log` on the boot volume.
/// Replaces `uefi::helpers::init()` (which only installs uefi's own
/// console-only logger; allocator and panic handler need no init call).
pub fn init(version: &str) {
    let file = open_fresh_log();
    if file.is_none() {
        *unsafe { &mut *LOGGER.file_error_reported.get() } = true;
    }
    // Safety: called once, before any logging.
    unsafe {
        *LOGGER.file.get() = file;
    }
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    if file_open() {
        log::info!("log file {} opened on marker volume", LOG_PATH);
    } else {
        log::info!("log file unavailable (console-only); open failed");
    }
    // Build class in the log line itself (audit M2): LTO can drop the
    // banner's strings from a binary whose banner is never called, so the
    // log - not the banner - is the authoritative build-class record. The
    // typo "unloader" the audit flagged is fixed; the quiet whitelist
    // matches "nv-uefi-unlocker" and keeps this line visible.
    #[cfg(feature = "tx")]
    log::info!("nv-uefi-unlocker v{version} (TX BUILD: CONTAINS GPU-WRITE PATHS)");
    #[cfg(not(feature = "tx"))]
    log::info!("nv-uefi-unlocker v{version} (read-only build; no GPU writes by design)");
}

/// Close the log file (flush) before chain-loading. Idempotent.
/// Call right before handing control away; later lines go to console only.
pub fn finish() {
    // Safety: single-threaded; take() drops (close = flush per spec).
    let taken = unsafe { (*LOGGER.file.get()).take() };
    drop(taken);
}

/// Console-suppression gate (issue #38): `\tx-quiet.txt` opts the run into
/// verdict-lines-only console output. The file always records everything;
/// quiet only gates the mirror. Refused while the log file is closed - the
/// console is then the only record and must stay complete. Returns whether
/// quiet took effect.
pub fn set_quiet(quiet: bool) -> bool {
    let effective = quiet_effective(quiet, file_open());
    // Safety: single-threaded boot services; pointer use confined here.
    unsafe {
        *LOGGER.quiet.get() = effective;
    }
    effective
}

/// Current quiet state (banner slimming reads it; the only caller is
/// tx-build-only, so read-only builds carry it unused).
#[cfg_attr(not(feature = "tx"), allow(dead_code))]
pub fn is_quiet() -> bool {
    // Safety: single-threaded boot services.
    unsafe { *LOGGER.quiet.get() }
}

fn file_open() -> bool {
    unsafe { (*LOGGER.file.get()).is_some() }
}

/// Log history (audit F3 + 2026-11-01 log-persistence round): each armed
/// boot shifts a three-generation chain instead of destroying the previous
/// record — `\loader.log` → `\loader-prev.log` → `\loader-prev2.log` →
/// `\loader-prev3.log` (oldest dropped). EFI has no rename, so every shift
/// is a bounded copy: 1 MiB per generation caps the worst case at ~3 MiB of
/// volume I/O per armed boot.
const LOG_HISTORY: usize = 3;
const PREV_LOG_PATHS: [&CStr16; LOG_HISTORY] = [
    cstr16!(r"\loader-prev.log"),
    cstr16!(r"\loader-prev2.log"),
    cstr16!(r"\loader-prev3.log"),
];
const PREV_LOG_CAP: usize = 1024 * 1024;

fn open_fresh_log() -> Option<RegularFile> {
    let mut fs = crate::volumes::marker_fs().ok()?;
    let mut root: uefi::proto::media::file::Directory = fs.open_volume().ok()?;

    // Rotate (audit F3): archive the previous log instead of destroying it.
    // Best-effort - a failed rotation must not stop the new run.
    rotate_previous_log(&mut root);

    let handle: FileHandle = root
        .open(LOG_PATH, FileMode::CreateReadWrite, FileAttribute::empty())
        .ok()?;
    handle.into_regular_file()
}

/// Shift the generation chain oldest-first (each copy's source still
/// exists), then delete the original so the fresh log starts empty.
fn rotate_previous_log(root: &mut uefi::proto::media::file::Directory) {
    for gen in (1..LOG_HISTORY).rev() {
        copy_capped(root, PREV_LOG_PATHS[gen - 1], PREV_LOG_PATHS[gen]);
    }
    copy_capped(root, LOG_PATH, PREV_LOG_PATHS[0]);
    if let Ok(h) = root.open(LOG_PATH, FileMode::ReadWrite, FileAttribute::empty()) {
        if let Some(reg) = h.into_regular_file() {
            let _ = reg.delete();
        }
    }
}

/// Bounded copy of one log generation: delete the destination (EFI open
/// never truncates), copy up to `PREV_LOG_CAP`, flush via drop-close.
/// Missing sources are the normal early-boot case and stay silent.
fn copy_capped(root: &mut uefi::proto::media::file::Directory, src: &CStr16, dst: &CStr16) {
    let Ok(handle) = root.open(src, FileMode::Read, FileAttribute::empty()) else {
        return;
    };
    let Some(mut old) = handle.into_regular_file() else { return };

    if let Ok(h) = root.open(dst, FileMode::ReadWrite, FileAttribute::empty()) {
        if let Some(reg) = h.into_regular_file() {
            let _ = reg.delete();
        }
    }
    let Ok(new_handle) = root.open(dst, FileMode::CreateReadWrite, FileAttribute::empty()) else {
        return;
    };
    let Some(mut new) = new_handle.into_regular_file() else { return };

    let mut chunk = [0u8; 4096];
    let mut copied = 0usize;
    while copied < PREV_LOG_CAP {
        let want = chunk.len().min(PREV_LOG_CAP - copied);
        match old.read(&mut chunk[..want]) {
            Ok(0) => break,
            Ok(n) => {
                if new.write(&chunk[..n]).is_err() {
                    break;
                }
                copied += n;
            }
            Err(_) => break,
        }
    }
    // Drops close (and flush) both files.
    drop(new);
    drop(old);
}
