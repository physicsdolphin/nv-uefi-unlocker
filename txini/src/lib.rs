//! `\tx.ini` - the optional consolidated stick configuration (issue #290).
//!
//! Why this crate exists: the marker-file UI grew to ~60 zero-byte/content
//! switches and issue #290 asked for one file. The parser and the arm
//! schema live here once (the romcore pattern: no_std + alloc,
//! host-testable, consumed by the loader), so the host tests exercise the
//! exact code the stick runs.
//!
//! Design contract (docs/tx-ini.md carries the operator-facing copy):
//! - markers stay authoritative when no `\tx.ini` exists - zero behavior
//!   change for every fielded stick;
//! - an arm set BOTH in the ini and by its marker file is a refusal - no
//!   silent precedence, the operator removes one of the two;
//! - parse and value errors are refusals; unknown keys are warnings (the
//!   bench knobs deliberately stay marker-file only);
//! - bare `key` lines mean true - a switch must never depend on file
//!   size, only on presence and syntax (the issue #106 zero-byte lesson);
//! - UTF-8 only: a UTF-16 BOM is refused with a re-save instruction
//!   (Notepad's other default); CRLF, LF and CR line endings all work.
//!
//! What the ini deliberately does NOT model: the four early-exit arms
//! (dry-run / agitate-only / fwsec-only / exit-fw) were merged into a
//! `mode` key in the first sketch and un-merged again - in the marker
//! world they are independent booleans and field combinations
//! (dry-run + exit-fw) are legal, so collapsing them would change
//! behavior. The staging axis is not a key at all: the automatic retry
//! ladders (STAGING-FALLBACK / FWSEC-FALLBACK) own band selection
//! end-to-end and the manual VRAM preset was retired 2026-11-25 - its
//! only saving (one failed attempt + one function-level FLR) happens
//! behind the already-dark detached screen, imperceptible to the
//! operator.

#![no_std]

#[cfg(test)]
extern crate std;

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// The consolidated config file (boot-volume root), parsed by [`parse`].
pub const INI_PATH: &str = "\\tx.ini";

/// Keys that may legitimately appear on more than one line: each line is
/// one list entry (the loader joins them with `\n` before the existing
/// per-site content parsers see them, so those parsers stay untouched).
/// Every other duplicate key is a refusal.
pub const REPEATABLE_KEYS: &[&str] = &["ignore_key", "input_reset", "gop_hide"];

/// Upper bound the loader enforces on the raw file before parsing: a real
/// config is under 1 KiB, so anything near this is the wrong file.
pub const MAX_RAW_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: String,
    pub value: Option<String>,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// FF FE / FE FF prefix: Notepad's UTF-16 default. Silently ignoring
    /// the whole file would turn "configured" into "defaults", so refuse.
    Utf16Bom,
    InvalidUtf8 { line: u32 },
    BadSyntax { line: u32, why: String },
    DuplicateKey { line: u32, key: String },
}

impl ParseError {
    /// Runtime output (loader log), therefore English-only.
    pub fn message(&self) -> String {
        match self {
            ParseError::Utf16Bom => format!(
                "{INI_PATH} starts with a UTF-16 BOM - re-save it as UTF-8 (BOM is fine) or ANSI"
            ),
            ParseError::InvalidUtf8 { line } => {
                format!("{INI_PATH} line {line}: not valid UTF-8")
            }
            ParseError::BadSyntax { line, why } => format!("{INI_PATH} line {line}: {why}"),
            ParseError::DuplicateKey { line, key } => format!(
                "{INI_PATH} line {line}: key '{key}' set twice - only {REPEATABLE_KEYS:?} may repeat"
            ),
        }
    }
}

fn trim_ascii_ws(s: &str) -> &str {
    s.trim_matches(|c| c == ' ' || c == '\t')
}

/// Parse the raw file into ordered [`Entry`] lines. Comments (`#`, `;`),
/// blank lines and all three line-ending styles are accepted; a UTF-8 BOM
/// is stripped. Everything syntactically dubious is a [`ParseError`] -
/// the loader refuses to run on it instead of guessing.
pub fn parse(raw: &[u8]) -> Result<Vec<Entry>, ParseError> {
    if raw.starts_with(&[0xff, 0xfe]) || raw.starts_with(&[0xfe, 0xff]) {
        return Err(ParseError::Utf16Bom);
    }
    let body = raw.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(raw);
    let mut out: Vec<Entry> = Vec::new();
    for (idx, raw_line) in body
        .split(|&b| b == b'\n' || b == b'\r')
        .enumerate()
    {
        let line = idx as u32 + 1;
        let Ok(text) = core::str::from_utf8(raw_line) else {
            return Err(ParseError::InvalidUtf8 { line });
        };
        let t = trim_ascii_ws(text);
        if t.is_empty() || t.starts_with('#') || t.starts_with(';') {
            continue;
        }
        let (key, value) = match t.split_once('=') {
            Some((k, v)) => (
                trim_ascii_ws(k),
                Some(trim_ascii_ws(v).to_string()),
            ),
            None => (t, None),
        };
        if key.is_empty() {
            return Err(ParseError::BadSyntax {
                line,
                why: "empty key".to_string(),
            });
        }
        if !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(ParseError::BadSyntax {
                line,
                why: format!(
                    "bad key syntax '{key}' (allowed: A-Z a-z 0-9 _ -)"
                ),
            });
        }
        if !REPEATABLE_KEYS.contains(&key) && out.iter().any(|e| e.key == key) {
            return Err(ParseError::DuplicateKey {
                line,
                key: key.to_string(),
            });
        }
        out.push(Entry {
            key: key.to_string(),
            value,
            line,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Resolved arm schema
// ---------------------------------------------------------------------------

/// Output profile family (replaces `\tx-quiet.txt` + `\tx-ultrafast.txt`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    Normal,
    Quiet,
    Ultrafast,
}

/// Log-commit family - the marker pair was already a three-state ladder
/// in the loader (`\tx-log-commit.txt` = 2, `\tx-log-commit-lines.txt` = 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogCommit {
    Full,
    Lines,
    Off,
}

/// Reconnect family: `On` = native-GOP rebuild, `Off` = dark handoff.
/// Default is `On` EXCEPT when a rebar arm is active (the pair + any GOP
/// reconnect is the proven Code 12 family - the loader then defaults to
/// the dark handoff without asking; rebar_gop remains the explicit
/// opt-in for that ablation shape). The legacy recursive-reconnect path
/// and its marker trio were retired 2026-11-25.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reconnect {
    /// Native-GOP rebuild on the gates the loader already applies.
    On,
    /// No reconnect at all - dark handoff, bootmgfw self-establishes.
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebarRoute {
    Mmio,
    Ecam,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebarSetting {
    Read,
    Scan,
    Pair { select: u32, advertise: u32 },
}

/// Everything the ini said, in typed form. `None`/`false`/empty means
/// "key not set" - the loader then falls back to the marker file, and the
/// conflict check has already guaranteed the two never overlap.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resolved {
    // families
    pub output: Option<Output>,
    pub log_commit: Option<LogCommit>,
    pub reconnect: Option<Reconnect>,
    // early-exit arms (independent booleans by design - see module docs)
    pub dry_run: bool,
    pub agitate_only: bool,
    pub fwsec_only: bool,
    pub exit_fw: bool,
    // booleans
    pub no_log: bool,
    pub no_console: bool,
    pub relock: bool,
    pub force_continue: bool,
    pub preload: bool,
    pub ebs_watch: bool,
    pub ebs_selftest: bool,
    pub host_probe: bool,
    pub acpi_dump: bool,
    pub gop_no_reorder: bool,
    pub sbr_gpu: bool,
    pub rebar_gop: bool,
    pub rebar_then_tx: bool,
    pub no_asset_cache: bool,
    // single-value arms
    /// `Some` = armed; the value is the countdown in seconds (0..=30).
    pub i_know: Option<u64>,
    /// Count-based log-flush cadence, 0..=4096 lines (0 = marker lines
    /// only).
    pub flush_lines: Option<u64>,
    /// FB-size override in GiB, 1..=64 (`\tu10x-fb-gb.txt` semantics).
    pub fb_gb: Option<u64>,
    pub chain_target: Option<String>,
    pub prom_sha256: Option<String>,
    pub select_bdf: Option<String>,
    pub native_gop: Option<String>,
    pub rebar: Option<RebarSetting>,
    pub rebar_route: Option<RebarRoute>,
    // list arms (one entry per repeated key line)
    pub ignore_key: Vec<String>,
    pub input_reset: Vec<String>,
    pub gop_hide: Vec<String>,
}

/// Marker files an ini key fights with. Empty for unknown keys (they get
/// a warning instead - the bench knobs stay marker-file only on purpose).
fn markers_for(key: &str) -> &'static [&'static str] {
    match key {
        "i_know_what_i_am_doing" => &["\\i_know_what_i_am_doing.txt"],
        "chain_target" => &["\\chain-target.txt"],
        "prom_sha256" => &["\\tx-prom-sha256.txt"],
        "select_bdf" => &["\\tx-select-bdf.txt"],
        "output" => &["\\tx-quiet.txt", "\\tx-ultrafast.txt"],
        "log_commit" => &["\\tx-log-commit.txt", "\\tx-log-commit-lines.txt"],
        "reconnect" => &["\\tx-no-reconnect.txt"],
        "dry_run" => &["\\tx-dry-run.txt"],
        "agitate_only" => &["\\tx-agitate-only.txt"],
        "fwsec_only" => &["\\tx-fwsec-only.txt"],
        "exit_fw" => &["\\tx-exit-fw.txt"],
        "no_log" => &["\\tx-no-log.txt"],
        "no_console" => &["\\tx-no-console.txt"],
        "relock" => &["\\tx-relock.txt"],
        "force_continue" => &["\\tx-force-continue.txt"],
        "preload" => &["\\tx-preload.txt"],
        "ebs_watch" => &["\\tx-ebs-watch.txt"],
        "ebs_selftest" => &["\\tx-ebs-selftest.txt"],
        "host_probe" => &["\\tx-host-probe.txt"],
        "acpi_dump" => &["\\tx-acpi-dump.txt"],
        "gop_no_reorder" => &["\\tx-gop-no-reorder.txt"],
        "sbr_gpu" => &["\\tx-sbr-gpu.txt"],
        "rebar" => &["\\tx-rebar.txt", "\\tx-rebar-read.txt", "\\tx-rebar-scan.txt"],
        "rebar_route" => &["\\tx-rebar-ecam.txt"],
        "rebar_gop" => &["\\tx-rebar-gop.txt"],
        "rebar_then_tx" => &["\\tx-rebar-then-tx.txt"],
        "no_asset_cache" => &["\\tx-no-asset-cache.txt"],
        "ignore_key" => &["\\tx-ignore-key.txt"],
        "input_reset" => &["\\input-reset.txt"],
        "gop_hide" => &["\\tx-gop-hide.txt"],
        "native_gop" => &["\\tx-native-gop.txt"],
        "fb_gb" => &["\\tu10x-fb-gb.txt"],
        "flush_lines" => &["\\tx-flush-lines.txt"],
        _ => &[],
    }
}

fn need_value<'a>(e: &'a Entry, errs: &mut Vec<String>) -> Option<&'a str> {
    match e.value.as_deref() {
        Some(v) if !v.is_empty() => Some(v),
        _ => {
            errs.push(format!(
                "{INI_PATH} line {}: key '{}' needs a value",
                e.line, e.key
            ));
            None
        }
    }
}

fn enum_value<'a>(
    e: &'a Entry,
    allowed: &[&str],
    errs: &mut Vec<String>,
) -> Option<&'a str> {
    let v = need_value(e, errs)?;
    if allowed.contains(&v) {
        Some(v)
    } else {
        errs.push(format!(
            "{INI_PATH} line {}: key '{}' wants one of {allowed:?}, got '{v}'",
            e.line, e.key
        ));
        None
    }
}

fn flag_value(e: &Entry, errs: &mut Vec<String>) -> Option<bool> {
    match e.value.as_deref() {
        None => Some(true),
        Some(v) if v.eq_ignore_ascii_case("true") => Some(true),
        Some(v) if v.eq_ignore_ascii_case("false") => Some(false),
        Some(v) => {
            errs.push(format!(
                "{INI_PATH} line {}: key '{}' wants true/false or nothing, got '{v}'",
                e.line, e.key
            ));
            None
        }
    }
}

fn num_value(e: &Entry, min: u64, max: u64, errs: &mut Vec<String>) -> Option<u64> {
    let v = need_value(e, errs)?;
    match v.parse::<u64>() {
        Ok(n) if n >= min && n <= max => Some(n),
        Ok(n) => {
            errs.push(format!(
                "{INI_PATH} line {}: key '{}' value {n} out of range {min}..={max}",
                e.line, e.key
            ));
            None
        }
        Err(_) => {
            errs.push(format!(
                "{INI_PATH} line {}: key '{}' wants a plain decimal number {min}..={max}, got '{v}'",
                e.line, e.key
            ));
            None
        }
    }
}

/// Validate the parsed entries against the arm schema and detect
/// ini-vs-marker conflicts. `marker` answers "is this marker file present
/// on the stick" (the loader wires `probe_file` in; tests wire a set).
/// Returns the typed config, or every refusal line at once - the loader
/// logs them all so one fix-up round trip is enough.
pub fn resolve(
    entries: &[Entry],
    marker: &mut dyn FnMut(&str) -> bool,
    warnings: &mut Vec<String>,
) -> Result<Resolved, Vec<String>> {
    let mut r = Resolved::default();
    let mut errs: Vec<String> = Vec::new();
    for e in entries {
        // Conflict first: every governed key fights its whole marker set,
        // independently of value validity - a double-set arm is refused
        // even when the ini value itself is also broken.
        for m in markers_for(&e.key) {
            if marker(m) {
                errs.push(format!(
                    "{INI_PATH} key '{}' and marker {m} both set the same arm - remove one of the two",
                    e.key
                ));
            }
        }
        match e.key.as_str() {
            "output" => {
                r.output = enum_value(e, &["normal", "quiet", "ultrafast"], &mut errs).map(
                    |s| match s {
                        "quiet" => Output::Quiet,
                        "ultrafast" => Output::Ultrafast,
                        _ => Output::Normal,
                    },
                );
            }
            "log_commit" => {
                r.log_commit = enum_value(e, &["full", "lines", "off"], &mut errs).map(
                    |s| match s {
                        "lines" => LogCommit::Lines,
                        "off" => LogCommit::Off,
                        _ => LogCommit::Full,
                    },
                );
            }
            "reconnect" => {
                r.reconnect = enum_value(e, &["on", "off"], &mut errs).map(|s| match s {
                    "off" => Reconnect::Off,
                    _ => Reconnect::On,
                });
            }
            "dry_run" => r.dry_run = flag_value(e, &mut errs).unwrap_or(false),
            "agitate_only" => r.agitate_only = flag_value(e, &mut errs).unwrap_or(false),
            "fwsec_only" => r.fwsec_only = flag_value(e, &mut errs).unwrap_or(false),
            "exit_fw" => r.exit_fw = flag_value(e, &mut errs).unwrap_or(false),
            "no_log" => r.no_log = flag_value(e, &mut errs).unwrap_or(false),
            "no_console" => r.no_console = flag_value(e, &mut errs).unwrap_or(false),
            "relock" => r.relock = flag_value(e, &mut errs).unwrap_or(false),
            "force_continue" => r.force_continue = flag_value(e, &mut errs).unwrap_or(false),
            "preload" => r.preload = flag_value(e, &mut errs).unwrap_or(false),
            "ebs_watch" => r.ebs_watch = flag_value(e, &mut errs).unwrap_or(false),
            "ebs_selftest" => r.ebs_selftest = flag_value(e, &mut errs).unwrap_or(false),
            "host_probe" => r.host_probe = flag_value(e, &mut errs).unwrap_or(false),
            "acpi_dump" => r.acpi_dump = flag_value(e, &mut errs).unwrap_or(false),
            "gop_no_reorder" => r.gop_no_reorder = flag_value(e, &mut errs).unwrap_or(false),
            "sbr_gpu" => r.sbr_gpu = flag_value(e, &mut errs).unwrap_or(false),
            "rebar_gop" => r.rebar_gop = flag_value(e, &mut errs).unwrap_or(false),
            "rebar_then_tx" => r.rebar_then_tx = flag_value(e, &mut errs).unwrap_or(false),
            "no_asset_cache" => r.no_asset_cache = flag_value(e, &mut errs).unwrap_or(false),
            "i_know_what_i_am_doing" => {
                // Bare key (or `key =`) = armed with the default countdown;
                // otherwise a strict 0..=30 (the marker parser clamps, the
                // ini refuses - a typo'd countdown must not boot as 5).
                r.i_know = match e.value.as_deref() {
                    None | Some("") => Some(5),
                    Some(v) => match v.parse::<u64>() {
                        Ok(n) if n <= 30 => Some(n),
                        Ok(n) => {
                            errs.push(format!(
                                "{INI_PATH} line {}: i_know_what_i_am_doing value {n} out of range 0..=30",
                                e.line
                            ));
                            None
                        }
                        Err(_) => {
                            errs.push(format!(
                                "{INI_PATH} line {}: i_know_what_i_am_doing wants seconds 0..=30 (or nothing for the default 5), got '{v}'",
                                e.line
                            ));
                            None
                        }
                    },
                };
            }
            "flush_lines" => r.flush_lines = num_value(e, 0, 4096, &mut errs),
            "fb_gb" => r.fb_gb = num_value(e, 1, 64, &mut errs),
            "chain_target" => r.chain_target = need_value(e, &mut errs).map(str::to_string),
            "prom_sha256" => r.prom_sha256 = need_value(e, &mut errs).map(str::to_string),
            "select_bdf" => r.select_bdf = need_value(e, &mut errs).map(str::to_string),
            "native_gop" => r.native_gop = need_value(e, &mut errs).map(str::to_string),
            "rebar_route" => {
                r.rebar_route =
                    enum_value(e, &["mmio", "ecam"], &mut errs).map(|s| match s {
                        "ecam" => RebarRoute::Ecam,
                        _ => RebarRoute::Mmio,
                    });
            }
            "rebar" => {
                let v = need_value(e, &mut errs);
                r.rebar = v.and_then(|v| match v {
                    "read" => Some(RebarSetting::Read),
                    "scan" => Some(RebarSetting::Scan),
                    _ => {
                        // Grammar is ALWAYS "<target>/<advertise>"; 0 on a
                        // side = do not write that side, "0/0" = skip the
                        // whole arm. Bare ("11") and half-slash ("9/", "/9")
                        // forms are refused - with 0 meaning "skip", there is
                        // no reason to keep three spellings for one intent.
                        // target 1..=19, advertise 0..=15.
                        let Some((s, a)) = v.split_once('/') else {
                            errs.push(format!(
                                "{INI_PATH} line {}: rebar wants 'read', 'scan' or '<target>/<advertise>' (0 = don't write that side, e.g. '{v}/0' or '0/0'), got '{v}'",
                                e.line
                            ));
                            return None;
                        };
                        let bad = |what: &str, got: &str, lo: u32, hi: u32| {
                            format!(
                                "{INI_PATH} line {}: rebar {what} '{got}' out of range {lo}..={hi}",
                                e.line
                            )
                        };
                        let (sel, adv) = match (s.parse::<u32>(), a.parse::<u32>()) {
                            (Ok(sel), Ok(adv))
                                if (0..=19).contains(&sel) && (0..=15).contains(&adv) =>
                            {
                                (sel, adv)
                            }
                            (Ok(sel), _) if sel > 19 => {
                                errs.push(bad("target", s, 0, 19));
                                return None;
                            }
                            (_, Ok(adv)) if adv > 15 => {
                                errs.push(bad("advertise", a, 0, 15));
                                return None;
                            }
                            _ => {
                                errs.push(format!(
                                    "{INI_PATH} line {}: rebar wants '<target>/<advertise>' as plain decimals, got '{v}'",
                                    e.line
                                ));
                                return None;
                            }
                        };
                        if sel == 0 && adv == 0 {
                            // "0/0" = the whole rebar arm is skipped: no
                            // claim, no SBR, no write. Represented as None
                            // so the loader treats it exactly like absence.
                            None
                        } else {
                            Some(RebarSetting::Pair {
                                select: sel,
                                advertise: adv,
                            })
                        }
                    }
                });
            }
            "ignore_key" | "input_reset" | "gop_hide" => {
                let list = match e.key.as_str() {
                    "ignore_key" => &mut r.ignore_key,
                    "input_reset" => &mut r.input_reset,
                    _ => &mut r.gop_hide,
                };
                if let Some(v) = need_value(e, &mut errs) {
                    list.push(v.to_string());
                }
            }
            _ => warnings.push(format!(
                "{INI_PATH} line {}: unknown key '{}' ignored - bench knobs stay marker-file only (docs/tx-ini.md)",
                e.line, e.key
            )),
        }
    }
    if errs.is_empty() {
        Ok(r)
    } else {
        Err(errs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec as StdVec;

    fn entries(raw: &str) -> StdVec<Entry> {
        parse(raw.as_bytes()).expect("parse")
    }
    fn no_markers(_: &str) -> bool {
        false
    }
    fn resolve_ok(raw: &str) -> Resolved {
        let mut w = StdVec::new();
        resolve(&entries(raw), &mut no_markers, &mut w).expect("resolve")
    }
    fn resolve_err(raw: &str) -> StdVec<String> {
        let mut w = StdVec::new();
        resolve(&entries(raw), &mut no_markers, &mut w).expect_err("resolve")
    }

    // ---- parser ------------------------------------------------------

    #[test]
    fn utf16_bom_refused() {
        assert_eq!(parse(&[0xff, 0xfe, b'a']), Err(ParseError::Utf16Bom));
        assert_eq!(parse(&[0xfe, 0xff, b'a']), Err(ParseError::Utf16Bom));
        // a lone 0xFF is not a BOM - it surfaces as invalid UTF-8 instead
        assert!(matches!(parse(&[0xff]), Err(ParseError::InvalidUtf8 { line: 1 })));
    }

    #[test]
    fn utf8_bom_stripped() {
        let es = parse(b"\xEF\xBB\xBFrelock\n").expect("parse");
        assert_eq!(es.len(), 1);
        assert_eq!(es[0].key, "relock");
        assert_eq!(es[0].line, 1);
    }

    #[test]
    fn line_endings_and_comments() {
        let es = entries("# c1\r\n; c2\r\n\r\nrelock\n\rno_log\r dry_run = true \r\n");
        let keys: StdVec<&str> = es.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["relock", "no_log", "dry_run"]);
        assert!(es.iter().all(|e| e.value.is_none() || e.value.as_deref() == Some("true")));
    }

    #[test]
    fn values_trim_and_keep_inner_equals() {
        let es = entries("chain_target = \\EFI\\Microsoft\\Boot\\bootmgfw.efi\nx = a=b\n");
        assert_eq!(es[0].value.as_deref(), Some("\\EFI\\Microsoft\\Boot\\bootmgfw.efi"));
        assert_eq!(es[1].value.as_deref(), Some("a=b"));
    }

    #[test]
    fn bad_syntax_refused() {
        assert!(matches!(
            parse(b"= 5\n"),
            Err(ParseError::BadSyntax { why, .. }) if why.contains("empty key")
        ));
        assert!(matches!(
            parse(b"bad key = 1\n"),
            Err(ParseError::BadSyntax { line: 1, .. })
        ));
        assert!(matches!(
            parse(b"ok\n\xff\xfe bad\n"),
            Err(ParseError::InvalidUtf8 { line: 2 })
        ));
    }

    #[test]
    fn duplicate_refused_repeatable_not() {
        assert!(matches!(
            parse(b"relock\nrelock\n"),
            Err(ParseError::DuplicateKey { key, .. }) if key == "relock"
        ));
        assert_eq!(entries("ignore_key = 1\nignore_key = 2\n").len(), 2);
    }

    // ---- resolver: booleans ------------------------------------------

    #[test]
    fn flags_bare_true_false() {
        let r = resolve_ok("relock\nno_log = true\npreload = FALSE\n");
        assert!(r.relock && r.no_log);
        assert!(!r.preload);
        assert!(resolve_err("relock = yes").len() == 1);
    }

    // ---- resolver: families ------------------------------------------

    #[test]
    fn families_happy_and_typo() {
        let r = resolve_ok("output = ultrafast\nlog_commit = lines\nreconnect = off\n");
        assert_eq!(r.output, Some(Output::Ultrafast));
        assert_eq!(r.log_commit, Some(LogCommit::Lines));
        assert_eq!(r.reconnect, Some(Reconnect::Off));
        let errs = resolve_err("output = fast\nlog_commit = always\nreconnect = legacy\n");
        assert_eq!(errs.len(), 3);
        assert!(errs[2].contains("legacy")); // retired value names are refusals now
    }

    #[test]
    fn family_bare_key_needs_value() {
        assert!(resolve_err("reconnect").len() == 1);
    }

    // ---- resolver: numbers -------------------------------------------

    #[test]
    fn i_know_bare_and_strict() {
        assert_eq!(resolve_ok("i_know_what_i_am_doing").i_know, Some(5));
        assert_eq!(resolve_ok("i_know_what_i_am_doing = 0").i_know, Some(0));
        assert_eq!(resolve_ok("i_know_what_i_am_doing = 30").i_know, Some(30));
        assert!(!resolve_err("i_know_what_i_am_doing = 31").is_empty());
        assert!(!resolve_err("i_know_what_i_am_doing = 5s").is_empty());
        assert_eq!(resolve_ok("i_know_what_i_am_doing =").i_know, Some(5));
    }

    #[test]
    fn flush_lines_and_fb_gb_ranges() {
        assert_eq!(resolve_ok("flush_lines = 4096").flush_lines, Some(4096));
        assert_eq!(resolve_ok("flush_lines = 0").flush_lines, Some(0));
        assert!(!resolve_err("flush_lines = 4097").is_empty());
        assert_eq!(resolve_ok("fb_gb = 8").fb_gb, Some(8));
        assert!(!resolve_err("fb_gb = 0").is_empty());
        assert!(!resolve_err("fb_gb = 65").is_empty());
    }

    // ---- resolver: rebar ---------------------------------------------

    #[test]
    fn rebar_shapes() {
        assert_eq!(resolve_ok("rebar = read").rebar, Some(RebarSetting::Read));
        assert_eq!(resolve_ok("rebar = scan").rebar, Some(RebarSetting::Scan));
        assert_eq!(
            resolve_ok("rebar = 11/9").rebar,
            Some(RebarSetting::Pair { select: 11, advertise: 9 })
        );
        assert_eq!(
            resolve_ok("rebar = 10/7").rebar,
            Some(RebarSetting::Pair { select: 10, advertise: 7 })
        );
        // 0 = don't write that side; 0/0 = skip the whole arm (None)
        assert_eq!(
            resolve_ok("rebar = 11/0").rebar,
            Some(RebarSetting::Pair { select: 11, advertise: 0 })
        );
        assert_eq!(
            resolve_ok("rebar = 0/9").rebar,
            Some(RebarSetting::Pair { select: 0, advertise: 9 })
        );
        assert_eq!(resolve_ok("rebar = 0/0").rebar, None);
        assert_eq!(resolve_ok("rebar_route = ecam").rebar_route, Some(RebarRoute::Ecam));
        // bare / half-slash forms are refusals now (spell 0 explicitly)
        assert!(!resolve_err("rebar = 11").is_empty());
        assert!(!resolve_err("rebar = 9/").is_empty());
        assert!(!resolve_err("rebar = /9").is_empty());
        assert!(!resolve_err("rebar = 11/16").is_empty());
        assert!(!resolve_err("rebar = 20/9").is_empty());
        assert!(!resolve_err("rebar = a/b").is_empty());
    }

    // ---- resolver: conflicts -----------------------------------------

    #[test]
    fn conflicts_refuse_with_marker_named() {
        let es = entries("relock\noutput = quiet\n");
        let mut w = StdVec::new();
        let mut set = |p: &str| p == "\\tx-relock.txt" || p == "\\tx-ultrafast.txt";
        let errs = resolve(&es, &mut set, &mut w).expect_err("conflict");
        assert_eq!(errs.len(), 2);
        assert!(errs[0].contains("\\tx-relock.txt"));
        assert!(errs[1].contains("\\tx-ultrafast.txt"));
        assert!(errs.iter().all(|e| e.contains("remove one of the two")));
    }

    #[test]
    fn conflict_beats_bad_value() {
        // double-set arm is reported even when the ini value is also junk
        let es = entries("reconnect = wat\n");
        let mut w = StdVec::new();
        let mut set = |p: &str| p == "\\tx-no-reconnect.txt";
        let errs = resolve(&es, &mut set, &mut w).expect_err("both");
        assert!(errs.iter().any(|e| e.contains("both set the same arm")));
        assert!(errs.iter().any(|e| e.contains("off"))); // enum error lists the allowed set
    }

    // ---- resolver: lists and unknowns ---------------------------------

    #[test]
    fn lists_accumulate_in_order() {
        let r = resolve_ok("ignore_key = 4242\nignore_key = 13\ninput_reset = usb 10de:2684\n");
        assert_eq!(r.ignore_key, ["4242", "13"]);
        assert_eq!(r.input_reset, ["usb 10de:2684"]);
    }

    #[test]
    fn unknown_key_warns_but_resolves() {
        let mut w = StdVec::new();
        let r = resolve(&entries("tx-no-canary\n"), &mut no_markers, &mut w).expect("ok");
        assert!(!r.no_log);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("unknown key 'tx-no-canary'"));
    }

    #[test]
    fn marker_only_knobs_are_unknown_keys() {
        // spot-check the bench set stays out of the schema (sec_base went
        // back to marker-only with the 2026-11-25 schema trim)
        for k in [
            "tx-no-canary",
            "tx-ptimer-seed",
            "tx-no-flr",
            "tu10x-no-ss",
            "pre-flr-on",
            "sec_base",
        ] {
            let mut w = StdVec::new();
            let raw = format!("{k}\n");
            resolve(&entries(&raw), &mut no_markers, &mut w).expect("resolve");
            assert_eq!(w.len(), 1, "{k} should be unknown");
        }
    }
}
