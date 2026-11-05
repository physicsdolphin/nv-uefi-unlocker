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

/// Close-and-reopen every marker line, Flush() on the count cadence.
///
/// Why not Flush() for the markers: `FatIFileAccess` deliberately skips the
/// cache flush on the success path and `FatFlush` is the only path that
/// writes the directory entry (size included). Firmware that treats Flush
/// as a no-op therefore leaves a 0-byte file no matter how often it is
/// called - measured on both 8-series boards of #122/#170: >40 lines,
/// `DETACH` markers among them (marker lines are flushed), file still 0
/// bytes, while the graceful read-only round on the same stick wrote 13 KB
/// at Close. `EFI_FILE_PROTOCOL.Close` is defined to release the handle and
/// the EDK2 FAT driver flushes the dirent on the way out, so marker lines
/// now write -> close -> reopen(ReadWrite, seek END). A hang after the
/// marker then leaves a readable file instead of an empty one.
///
/// Cost: one open + seek + close per marker (~50 markers/run, the same
/// 0.3-0.5 s envelope the old flush budget assumed). The count cadence
/// stays a plain Flush() - it is cheap and now only a nicety.
///
/// 2026-10-27 background (why markers exist at all): FAT only commits
/// size/dirent on Flush, so the markers are what survives a
/// hang or mid-call reset - exactly what the 2026-10-27 Z790 diagnosis
/// needed (the log ended at "POST-FLR: verified"). A Flush per line is
/// visible on slow USB media (the TU106 run of 2026-10-28 spent noticeable
/// seconds on ~415 flushed lines); the 2026-11-01 TU106 run measured four
/// ~1.2 s stalls all aligned to flush events (USB flash GC shape), so the
/// count cadence is a knob: `\tx-flush-lines.txt` on the stick root sets it
/// (0 = marker lines only). Raising it trades forensics resolution for
/// fewer flush stalls - the marker lines below always flush regardless.
///
/// Boundary coverage (P1-6, nv_unlock_win 0.2.14 discipline): every
/// transaction verdict/phase line flushes via the markers below, so a
/// mid-call reset always leaves the last boundary on the stick. Mapping:
/// claim verdicts -> "claim:", FWSEC ignition return -> "FWSEC:" (colon
/// form only - FWSEC-TU*/FWSEC-TRACE diagnostics stay on the count
/// cadence), booter START/staging -> "BOOTER:", selectors -> "SELECTOR",
/// FLR -> "FLR" + "POST-", StartImage -> "CHAIN" + the finish() close.
/// Deliberately NOT added: FWSEC-TU/FWSEC-TRACE/BOOTER-AUDIT/
/// TX-ASSET-DIAG/SCRUB progress streams (issue #38 budget: per-line flush
/// ~5-10 ms on USB BOT; the marker set adds <=~50 flushed lines =
/// ~0.3-0.5 s worst case - the real-machine ablation round must confirm
/// no new 1.25 s BOT stalls appear, else revert).
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
    "claim:", "FLR", "BOOTER:", "FWSEC:",
    // Issue #128 run 11: a freeze inside the native-GOP recovery block
    // left the log blind at the 16-line cadence - run 10's post-block
    // lines only reached the stick because the graceful path ran on to
    // FINAL (marker flush + close). The whole recovery sequence is ~25
    // lines, so per-line flush costs ~0.1-0.25 s worst case on USB BOT
    // and turns any future hang into a between-two-lines answer.
    "NATIVE-GOP", "VGA-RESTORE",
    // Issue #122 (2026-11-05): the reporter's self-built TX binaries hang
    // with a created-but-0-byte log and a screen that never lights up, in
    // the thin strip between the file existing and the first marker line.
    // These entries make the strip itself readable: "log file" is line 1,
    // the version banner line 2, BOOT/DIAG/EARLY cover the probes behind
    // them, and PREP/CONFIRM cover the armed countdown where the first
    // TX-only runtime work (enumeration/GOP probe/sample/asset reads)
    // actually runs.
    "log file", "nv-uefi-unlocker", "BOOT", "DIAG", "EARLY", "PREP", "CONFIRM",
    // Issue #170 diag5: the recovered v2.1.10 logs end at "TX: layout ok",
    // but "layout ok" and the next log call are adjacent statements - the
    // evidence pipeline stopped, not necessarily the execution. "TX:" and
    // "layout" commit every segment boundary and the FWSEC-section step
    // lines (all "FWSEC:"-prefixed) so the ignition stretch is line-resolved.
    "TX:", "layout",
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
    // #122/#170 diag: stop mirroring while the console stack is torn down
    console_enabled: UnsafeCell<bool>,
}

static LOGGER: DualLogger = DualLogger {
    file: UnsafeCell::new(None),
    file_error_reported: UnsafeCell::new(false),
    lines: UnsafeCell::new(0),
    quiet: UnsafeCell::new(false),
    console_enabled: UnsafeCell::new(true),
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
        // line before it is mirrored. Durability policy: see the marker
        // comment above (close+reopen on markers, Flush on the cadence).
        // Safety: single-threaded boot services; pointer use is confined here.
        unsafe {
            let n = *self.lines.get() + 1;
            *self.lines.get() = n;
            let fe = FLUSH_EVERY_LINES.load(core::sync::atomic::Ordering::Relaxed);
            let count_flush = fe > 0 && n % fe == 0;
            let durable = FLUSH_MARKERS.iter().any(|m| line.contains(m));
            if (*self.file.get()).is_some() {
                let mut ok;
                {
                    let f = (*self.file.get()).as_mut().unwrap();
                    ok = f.write(line.as_bytes()).is_ok();
                    if ok && count_flush {
                        ok = f.flush().is_ok();
                    }
                }
                if ok && durable {
                    // Close (drops the handle: that is what commits the
                    // dirent on firmware where Flush is a no-op), then
                    // reopen at EOF so logging continues.
                    let _ = (*self.file.get()).take();
                    match reopen_log() {
                        Some(f) => *self.file.get() = Some(f),
                        None => {
                            *self.file.get() = None;
                            ok = false;
                        }
                    }
                }
                if !ok {
                    // Media pulled / reopen refused: console keeps the run
                    // diagnosable, the file log stops here.
                    *self.file.get() = None;
                }
            }
        }

        // Console mirror. Quiet mode (issue #38) keeps verdict-class lines
        // only; the console-detached gate (#170 diag) suppresses the mirror
        // entirely between detach and reconnect. The file mirror ran first.
        let quiet = unsafe { *self.quiet.get() };
        let console_enabled = unsafe { *self.console_enabled.get() };
        if console_enabled && (!quiet || console_visible(&line)) {
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

/// Stop mirroring to the console (#122/#170 diag, issue #170).
///
/// The GOP handle we disconnect carries the console's TextOut instance, so
/// every call after `DisconnectController` runs against a console stack the
/// spec never promised would stay usable. This gate makes the next log call
/// touch nothing but the file, which splits the two candidate hang sites:
/// a run that survives the mute dies in a Stop() (H2), a run that still
/// hangs was killed by writing to the detached console (H1).
///
/// Read-only builds carry it unused.
#[cfg_attr(not(feature = "tx"), allow(dead_code))]
pub fn mute_console() {
    // Safety: single-threaded boot services.
    unsafe {
        *LOGGER.console_enabled.get() = false;
    }
}

/// Re-enable the console mirror. Idempotent and silent (a log line here is
/// the first post-detach console write, so callers decide when to spend it).
#[cfg_attr(not(feature = "tx"), allow(dead_code))]
pub fn unmute_console() {
    // Safety: single-threaded boot services.
    unsafe {
        *LOGGER.console_enabled.get() = true;
    }
}

#[cfg_attr(not(feature = "tx"), allow(dead_code))]
pub fn console_muted() -> bool {
    // Safety: single-threaded boot services.
    unsafe { !*LOGGER.console_enabled.get() }
}

/// Reopen `\loader.log` for writing with the position at EOF; used by the
/// marker-line durability path after the close that commits the dirent.
/// `FileMode::ReadWrite` on an existing file does not truncate (only
/// CreateReadWrite does), and `END_OF_FILE` seeks to the current end.
fn reopen_log() -> Option<RegularFile> {
    let mut fs = boot::get_image_file_system(boot::image_handle()).ok()?;
    let mut root: uefi::proto::media::file::Directory = fs.open_volume().ok()?;
    let handle: FileHandle = root
        .open(LOG_PATH, FileMode::ReadWrite, FileAttribute::empty())
        .ok()?;
    let mut file = handle.into_regular_file()?;
    file.set_position(RegularFile::END_OF_FILE).ok()?;
    Some(file)
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
    let file = handle.into_regular_file()?;
    // Commit the fresh (0-byte) dirent right away by closing and reopening
    // (issue #122): the reporter's hangs leave a created-but-empty file, and
    // on firmware where Flush is a no-op the creation itself is otherwise
    // uncommitted. With the empty dirent on disk, "file created, still 0
    // bytes" now means the run died between this point and the first log
    // line - a three-call strip (set_logger, first format, first write)
    // instead of an unknown one.
    drop(file);
    reopen_log()
}
