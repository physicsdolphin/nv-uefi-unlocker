//! Dual logger: console + `\loader.log` on the boot volume (USB).
//!
//! The UEFI Shell's `> file` redirection proved unreliable on the target
//! firmware (empty capture), so we own the file ourselves. This also
//! pre-implements M1's "write samples to USB FAT" requirement.
//!
//! Constitution note: the log file lives on OUR boot volume and is the only
//! thing this app ever writes; the ESP stays untouched.

use alloc::format;
use core::cell::UnsafeCell;
use log::{Level, Log};
use uefi::boot;
use uefi::proto::media::file::{File, FileAttribute, FileHandle, FileMode, RegularFile};
use uefi::{cstr16, system, CStr16, CString16};

pub const LOG_PATH: &CStr16 = cstr16!(r"\loader.log");

/// Flush every 16th line, and always on stage/verdict lines. FAT only
/// commits size/dirent on Flush, so the markers below are what survives a
/// hang or mid-call reset - exactly what the 2026-10-27 Z790 diagnosis
/// needed (the log ended at "POST-FLR: verified"). A Flush per line is
/// visible on slow USB media (the TU106 run of 2026-10-28 spent noticeable
/// seconds on ~415 flushed lines).
const FLUSH_EVERY_LINES: usize = 16;
// Boundary coverage (P1-6, nv_unlock_win 0.2.14 discipline): every
// transaction verdict/phase line flushes, so a mid-call reset always
// leaves the last boundary on the stick. Mapping to the six transaction
// boundaries: claim verdicts -> "claim:", FWSEC ignition return ->
// "FWSEC:" (colon form: FWSEC-TU*/FWSEC-TRACE diagnostics stay on the
// every-16 cadence), booter START/staging -> "BOOTER:", selectors ->
// "SELECTOR", FLR -> "FLR" + "POST-", StartImage -> "CHAIN" + the
// finish() close. Deliberately NOT added: FWSEC-TU/FWSEC-TRACE/
// BOOTER-AUDIT/TX-ASSET-DIAG/SCRUB progress streams (issue #38 budget:
// per-line flush cost ~5-10 ms each on USB BOT; this set adds <=~50
// flushed lines = ~0.3-0.5 s worst case - the real-machine ablation
// round must confirm no new 1.25 s BOT stalls appear, else revert).
const FLUSH_MARKERS: &[&str] = &[
    "PHASE", "RESULT", "POST-", "FINAL", "RECONNECT", "DETACH", "CHAIN",
    "SELECTOR", "HANDOFF", "WATCHDOG", "AUTH-SCAN end",
    "claim:", "FLR", "BOOTER:", "FWSEC:",
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
            let flush = n % FLUSH_EVERY_LINES == 0
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
        if !quiet || console_visible(&line) {
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
        log::info!("log file {} opened on boot volume", LOG_PATH);
    } else {
        log::info!("log file unavailable (console-only); open failed");
    }
    log::info!("nv-uefi-unloader v{version} (M0 skeleton: read-only)");
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

fn open_fresh_log() -> Option<RegularFile> {
    let mut fs = boot::get_image_file_system(boot::image_handle()).ok()?;
    let mut root: uefi::proto::media::file::Directory = fs.open_volume().ok()?;

    // Remove a stale log from a previous boot so each run starts clean.
    if let Ok(handle) = root.open(LOG_PATH, FileMode::ReadWrite, FileAttribute::empty()) {
        // A regular file handle only; directories cannot be deleted blindly.
        if let Some(reg) = handle.into_regular_file() {
            let _ = reg.delete();
        }
    }

    let handle: FileHandle = root
        .open(LOG_PATH, FileMode::CreateReadWrite, FileAttribute::empty())
        .ok()?;
    handle.into_regular_file()
}
