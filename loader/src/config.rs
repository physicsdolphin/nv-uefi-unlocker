//! `\tx.ini` consolidated stick configuration (issue #290): an OPTIONAL
//! aggregation layer over the marker-file arms. Marker files remain the
//! authority when no ini exists - every fielded stick behaves exactly as
//! before - and an arm set in BOTH places is a boot refusal (no silent
//! precedence). Parse/value errors refuse; unknown keys warn (the bench
//! knobs stay marker-file only on purpose). The grammar and the arm
//! schema live in the `txini` crate, host-tested; this module owns the
//! UEFI side: tri-state file read (absent vs present-but-broken stays
//! distinct - a damaged ini must not silently downgrade to defaults),
//! marker conflict probing, and the fallback accessors every call site
//! switches to.
//!
//! `init()` runs at the very top of `main`, before the logger exists:
//! several arms (log-commit, no-log, no-console, ultrafast) must be known
//! before `logging::init`. Nothing in here logs; refusals and warnings
//! are collected as strings and replayed by `log_diagnostics()` once the
//! logger is up. A refused config stops the boot BEFORE any GPU touch:
//! log, hand the machine back to the firmware boot manager.

use core::cell::UnsafeCell;
use log::info;
use txini::{LogCommit, Output, Resolved};
#[cfg(feature = "diag")]
use txini::{AgitateStage, RebarRoute, RebarSetting, Reconnect};
use uefi::proto::media::file::{File, FileAttribute, FileMode};
use uefi::{boot, cstr16, CStr16, CString16, Status};

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

const INI: &CStr16 = cstr16!(r"\tx.ini");

struct State {
    resolved: Option<Resolved>,
    refusals: Vec<String>,
    warnings: Vec<String>,
    present: bool,
}

// Safety: UEFI applications are single-threaded during boot services (the
// ASSET_CACHE pattern in tx.rs); the cell is written once by init() before
// any reader exists.
struct StateCell(UnsafeCell<Option<State>>);
unsafe impl Sync for StateCell {}
unsafe impl Send for StateCell {}

static STATE: StateCell = StateCell(UnsafeCell::new(None));

fn state() -> &'static Option<State> {
    unsafe { &*STATE.0.get() }
}

fn ini() -> Option<&'static Resolved> {
    state().as_ref().and_then(|s| s.resolved.as_ref())
}

// ---------------------------------------------------------------------------
// Tri-state ini read: Absent / Failed(why) / Content(bytes)
// ---------------------------------------------------------------------------

enum IniRead {
    Absent,
    Failed(String),
    Content(Vec<u8>),
}

fn read_ini() -> IniRead {
    let Ok(mut fs) = boot::get_image_file_system(boot::image_handle()) else {
        // No filesystem surface at all: every marker path is equally dead,
        // and the loader has its own no-stick story. Not an ini problem.
        return IniRead::Absent;
    };
    let Ok(mut dir) = fs.open_volume() else {
        return IniRead::Absent;
    };
    let file = match dir.open(INI, FileMode::Read, FileAttribute::empty()) {
        Ok(f) => f,
        Err(e) if e.status() == Status::NOT_FOUND => return IniRead::Absent,
        Err(e) => {
            return IniRead::Failed(format!("\\tx.ini open failed: {:?}", e.status()));
        }
    };
    let Some(mut file) = file.into_regular_file() else {
        return IniRead::Failed("\\tx.ini is a directory, not a file".to_string());
    };
    let mut out = Vec::new();
    let mut chunk = [0u8; 512];
    loop {
        match file.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                out.extend_from_slice(&chunk[..n]);
                if out.len() > txini::MAX_RAW_BYTES {
                    return IniRead::Failed(format!(
                        "\\tx.ini is over {} bytes - not a config file",
                        txini::MAX_RAW_BYTES
                    ));
                }
            }
            Err(e) => {
                return IniRead::Failed(format!("\\tx.ini read failed: {:?}", e.status()));
            }
        }
    }
    IniRead::Content(out)
}

/// Open-only presence probe (same shape as `tx::probe_file`, kept local so
/// this module stays ungated like its callers in read-only builds).
fn probe(path: &str) -> bool {
    let Ok(path_c) = CString16::try_from(path) else {
        return false;
    };
    let Ok(mut fs) = boot::get_image_file_system(boot::image_handle()) else {
        return false;
    };
    let Ok(mut dir) = fs.open_volume() else {
        return false;
    };
    dir.open(&path_c, FileMode::Read, FileAttribute::empty())
        .is_ok()
}

/// Marker content read for the legacy fallback paths (same reader semantics
/// as the pre-ini call sites: `chainload::boot_volume_read`).
fn read_marker(path: &str) -> Option<Vec<u8>> {
    let Ok(p) = CString16::try_from(path) else {
        return None;
    };
    crate::chainload::boot_volume_read(&p)
}

// ---------------------------------------------------------------------------
// init + diagnostics
// ---------------------------------------------------------------------------

/// Parse `\tx.ini` and conflict-check it against the marker files. Must be
/// the first config call of the boot; silent by design (pre-logger).
pub fn init() {
    let (resolved, refusals, warnings, present) = match read_ini() {
        IniRead::Absent => (None, Vec::new(), Vec::new(), false),
        IniRead::Failed(why) => (None, alloc::vec![why], Vec::new(), true),
        IniRead::Content(raw) => match txini::parse(&raw) {
            Err(e) => (None, alloc::vec![e.message()], Vec::new(), true),
            Ok(entries) => {
                let mut warnings = Vec::new();
                match txini::resolve(&entries, &mut |p| probe(p), &mut warnings) {
                    Ok(r) => (Some(r), Vec::new(), warnings, true),
                    Err(errs) => (None, errs, warnings, true),
                }
            }
        },
    };
    // Safety: single-threaded boot services; init runs before all readers.
    unsafe {
        *STATE.0.get() = Some(State {
            resolved,
            refusals,
            warnings,
            present,
        });
    }
}

pub fn refused() -> bool {
    state().as_ref().is_some_and(|s| !s.refusals.is_empty())
}

/// Replay the collected ini lines through the logger. Call after
/// `logging::init`; the refused branch of `main` force-enables the console
/// mirror first so a refusal can never be silenced by no-console.
pub fn log_diagnostics() {
    let Some(s) = state().as_ref() else { return };
    if s.present && s.resolved.is_some() {
        info!(
            "INI: \\tx.ini present and honored; marker files remain valid for every arm not set here (docs/tx-ini.md)"
        );
    }
    for w in &s.warnings {
        info!("INI-WARNING: {w}");
    }
    for r in &s.refusals {
        info!("INI-REFUSED: {r}");
    }
}

// ---------------------------------------------------------------------------
// Family accessors (ini value, else the preserved marker ladder)
// ---------------------------------------------------------------------------

/// Log-commit ladder: 2 = create-commit, 1 = commit-once-per-N-lines,
/// 0 = default. Marker ladder order preserved.
pub fn log_commit() -> u8 {
    if let Some(c) = ini().and_then(|r| r.log_commit) {
        return match c {
            LogCommit::Full => 2,
            LogCommit::Lines => 1,
            LogCommit::Off => 0,
        };
    }
    if probe("\\tx-log-commit.txt") {
        2
    } else if probe("\\tx-log-commit-lines.txt") {
        1
    } else {
        0
    }
}

pub fn ultrafast() -> bool {
    ini().is_some_and(|r| r.output == Some(Output::Ultrafast)) || probe("\\tx-ultrafast.txt")
}

/// Quiet console profile: explicit quiet, ultrafast (which folds quiet),
/// or the marker.
pub fn quiet() -> bool {
    ini().is_some_and(|r| matches!(r.output, Some(Output::Quiet) | Some(Output::Ultrafast)))
        || probe("\\tx-quiet.txt")
        || ultrafast()
}

/// Reconnect family, reduced to the one question the sites ask: skip the
/// native-GOP rebuild (dark handoff - bootmgfw self-establishes its
/// console) or run it. Precedence: explicit ini value, then the
/// tx-no-reconnect marker, then the rebar-aware default - an ACTIVE rebar
/// arm defaults to off (the rebar pair + any GOP reconnect is the proven
/// Code 12 family: pci rebalances over the freshly enlarged BAR),
/// everything else defaults to on. The legacy recursive-reconnect path
/// and its marker trio were retired 2026-11-25 - see the removal note in
/// tx_detach.rs.
#[cfg(feature = "diag")]
pub fn reconnect_off() -> bool {
    if let Some(v) = ini().and_then(|r| r.reconnect) {
        return v == Reconnect::Off;
    }
    if probe("\\tx-no-reconnect.txt") {
        return true;
    }
    rebar().is_some()
}

// ---------------------------------------------------------------------------
// Early-exit arms - deliberately NOT a merged `mode` family: in the marker
// world they are independent booleans and combinations (dry-run +
// exit-fw) are legal field shapes, so collapsing them would change
// behavior (see the txini module docs).
// ---------------------------------------------------------------------------

#[cfg(feature = "diag")]
pub fn dry_run() -> bool {
    ini().is_some_and(|r| r.dry_run) || probe("\\tx-dry-run.txt")
}

#[cfg(feature = "diag")]
pub fn agitate_only() -> bool {
    ini().is_some_and(|r| r.agitate_only) || probe("\\tx-agitate-only.txt")
}

/// The agitate ablation level: `Some` = the agitate control runs with
/// this surgery-element stage (ini-only; the ladder is a diag-profile
/// instrument, so it deliberately has no marker twin). `None` = key not
/// set - the loader then falls back to the plain `agitate_only` arm.
#[cfg(feature = "diag")]
pub fn agitate_stage() -> Option<AgitateStage> {
    ini().and_then(|r| r.agitate_stage)
}

#[cfg(feature = "diag")]
pub fn fwsec_only() -> bool {
    ini().is_some_and(|r| r.fwsec_only) || probe("\\tx-fwsec-only.txt")
}

pub fn exit_fw() -> bool {
    ini().is_some_and(|r| r.exit_fw) || probe("\\tx-exit-fw.txt")
}

// ---------------------------------------------------------------------------
// Boolean arms (ini flag, else marker presence)
// ---------------------------------------------------------------------------

/// Several variants are constructed only by diag/tx-gated call sites, so a
/// read-only build sees them as dead - same treatment as keywait.rs:408.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum FlagArm {
    NoLog,
    NoConsole,
    Relock,
    ForceContinue,
    Preload,
    EbsWatch,
    EbsSelftest,
    HostProbe,
    AcpiDump,
    GopNoReorder,
    RebarGop,
    RebarThenTx,
    NoAssetCache,
}

impl FlagArm {
    fn marker(self) -> &'static str {
        match self {
            Self::NoLog => "\\tx-no-log.txt",
            Self::NoConsole => "\\tx-no-console.txt",
            Self::Relock => "\\tx-relock.txt",
            Self::ForceContinue => "\\tx-force-continue.txt",
            Self::Preload => "\\tx-preload.txt",
            Self::EbsWatch => "\\tx-ebs-watch.txt",
            Self::EbsSelftest => "\\tx-ebs-selftest.txt",
            Self::HostProbe => "\\tx-host-probe.txt",
            Self::AcpiDump => "\\tx-acpi-dump.txt",
            Self::GopNoReorder => "\\tx-gop-no-reorder.txt",
            Self::RebarGop => "\\tx-rebar-gop.txt",
            Self::RebarThenTx => "\\tx-rebar-then-tx.txt",
            Self::NoAssetCache => "\\tx-no-asset-cache.txt",
        }
    }

    fn from_ini(self, r: &Resolved) -> bool {
        match self {
            Self::NoLog => r.no_log,
            Self::NoConsole => r.no_console,
            Self::Relock => r.relock,
            Self::ForceContinue => r.force_continue,
            Self::Preload => r.preload,
            Self::EbsWatch => r.ebs_watch,
            Self::EbsSelftest => r.ebs_selftest,
            Self::HostProbe => r.host_probe,
            Self::AcpiDump => r.acpi_dump,
            Self::GopNoReorder => r.gop_no_reorder,
            Self::RebarGop => r.rebar_gop,
            Self::RebarThenTx => r.rebar_then_tx,
            Self::NoAssetCache => r.no_asset_cache,
        }
    }
}

/// Boolean arm: ini flag or marker presence. An explicit `key = false`
/// plus its marker is already a refused boot (conflict check), so the
/// fall-through probe can never contradict an ini value.
pub fn flag(arm: FlagArm) -> bool {
    ini().is_some_and(|r| arm.from_ini(r)) || probe(arm.marker())
}

// ---------------------------------------------------------------------------
// Content arms (ini value(s) as content bytes, else the legacy marker read)
// ---------------------------------------------------------------------------

/// FbGb is constructed only by diag-gated sites (select_bdf has its own
/// bespoke accessors keeping the read_asset/boot_volume_read reader split,
/// so it is not a variant here at all; sec_base went back to marker-only).
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum ContentArm {
    ChainTarget,
    PromSha256,
    FbGb,
    IgnoreKey,
    InputReset,
}

impl ContentArm {
    fn marker(self) -> &'static str {
        match self {
            Self::ChainTarget => "\\chain-target.txt",
            Self::PromSha256 => "\\tx-prom-sha256.txt",
            Self::FbGb => "\\tu10x-fb-gb.txt",
            Self::IgnoreKey => "\\tx-ignore-key.txt",
            Self::InputReset => "\\input-reset.txt",
        }
    }

    fn from_ini(self, r: &Resolved) -> Option<Vec<u8>> {
        // List arms join their lines with '\n' so the per-line site parsers
        // (usbreset::parse, keywait ignore-key loader) stay untouched.
        let joined = |l: &Vec<String>| l.join("\n").into_bytes();
        match self {
            Self::ChainTarget => r.chain_target.clone().map(String::into_bytes),
            Self::PromSha256 => r.prom_sha256.clone().map(String::into_bytes),
            Self::FbGb => r.fb_gb.map(|g| g.to_string().into_bytes()),
            Self::IgnoreKey => (!r.ignore_key.is_empty()).then(|| joined(&r.ignore_key)),
            Self::InputReset => (!r.input_reset.is_empty()).then(|| joined(&r.input_reset)),
        }
    }
}

pub fn content(arm: ContentArm) -> Option<Vec<u8>> {
    if let Some(b) = ini().and_then(|r| arm.from_ini(r)) {
        return Some(b);
    }
    read_marker(arm.marker())
}

/// `tx-native-gop.txt` names a volume file; the ini key carries that name
/// as a string value. Fallback reader is the cached `read_boot_file` the
/// site used (diag-gated module, diag-gated accessor).
#[cfg(feature = "diag")]
pub fn native_gop_name() -> Option<Vec<u8>> {
    ini()
        .and_then(|r| r.native_gop.clone().map(String::into_bytes))
        .or_else(|| crate::tx::read_boot_file("\\tx-native-gop.txt"))
}

/// `tx-gop-hide.txt` is a hide-list (one BDF or VID:DID per line).
#[cfg(feature = "diag")]
pub fn gop_hide_list() -> Option<Vec<u8>> {
    if let Some(b) = ini().and_then(|r| {
        (!r.gop_hide.is_empty()).then(|| r.gop_hide.join("\n").into_bytes())
    }) {
        return Some(b);
    }
    crate::tx::read_boot_file("\\tx-gop-hide.txt")
}

/// TX-side BDF pin keeps its three-state reader semantics (issue #106:
/// present-but-broken must refuse, not fall through to "no pin").
#[cfg(feature = "diag")]
pub fn select_bdf_tx() -> Result<Option<Vec<u8>>, ()> {
    if let Some(b) = ini().and_then(|r| r.select_bdf.clone().map(String::into_bytes)) {
        return Ok(Some(b));
    }
    crate::tx::read_asset("\\tx-select-bdf.txt")
}

/// SAMPLE-side BDF pin (read-only phase, two-state reader by design).
pub fn select_bdf_sample() -> Option<Vec<u8>> {
    if let Some(b) = ini().and_then(|r| r.select_bdf.clone().map(String::into_bytes)) {
        return Some(b);
    }
    read_marker("\\tx-select-bdf.txt")
}

// ---------------------------------------------------------------------------
// Typed arms
// ---------------------------------------------------------------------------

/// Armed mode: `Some(secs)` = auto-run countdown. Ini strictness (0..=30
/// refused outside) lives in txini; the marker path keeps its lenient
/// clamp semantics untouched.
pub fn armed() -> Option<u64> {
    if let Some(secs) = ini().and_then(|r| r.i_know) {
        return Some(secs);
    }
    read_marker("\\i_know_what_i_am_doing.txt")
        .map(|c| crate::chainload::parse_armed_seconds(&c))
}

pub fn flush_lines() -> Option<u64> {
    if let Some(n) = ini().and_then(|r| r.flush_lines) {
        return Some(n);
    }
    read_marker("\\tx-flush-lines.txt")
        .map(|c| crate::chainload::parse_flush_lines(&c))
}

/// ReBAR rung selector: read-probe / field-scan / `<select>/<advertise>`
/// pair. Marker path preserves the lenient legacy parse verbatim
/// (defaults 13/9); the ini path is strict in txini.
#[cfg(feature = "diag")]
pub fn rebar() -> Option<RebarSetting> {
    if let Some(v) = ini().and_then(|r| r.rebar) {
        return Some(v);
    }
    if probe("\\tx-rebar-read.txt") {
        return Some(RebarSetting::Read);
    }
    if probe("\\tx-rebar-scan.txt") {
        return Some(RebarSetting::Scan);
    }
    if probe("\\tx-rebar.txt") {
        let raw = read_marker("\\tx-rebar.txt").unwrap_or_default();
        let s: String = raw.iter().map(|&b| b as char).collect();
        let mut parts = s.trim().split('/');
        let select = parts
            .next()
            .and_then(|p| p.parse::<u32>().ok())
            .filter(|&n| (1..=19).contains(&n))
            .unwrap_or(13);
        let advertise = parts
            .next()
            .and_then(|p| p.parse::<u32>().ok())
            .filter(|&n| n <= 15)
            .unwrap_or(9);
        return Some(RebarSetting::Pair {
            select,
            advertise,
        });
    }
    None
}

#[cfg(feature = "diag")]
pub fn rebar_route() -> RebarRoute {
    if ini().and_then(|r| r.rebar_route) == Some(RebarRoute::Ecam) {
        return RebarRoute::Ecam;
    }
    if probe("\\tx-rebar-ecam.txt") {
        RebarRoute::Ecam
    } else {
        RebarRoute::Mmio
    }
}
