//! Single-key confirmation with timeout (TOTORONG interaction pattern).

use alloc::format;
use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;
use log::info;
use uefi::proto::console::text::{Key, ScanCode};
use uefi::{boot, system};

pub enum KeyOrTimeout {
    Key(Key),
    Timeout,
}

/// Outcome of the CONFIRM phase.
pub enum Confirm {
    /// Run the transaction (TX builds) / continue the read-only phases.
    Proceed,
    /// Abort into chain-load: no GPU writes this boot. (TX builds only -
    /// read-only builds never abort.)
    #[allow(dead_code)]
    Abort,
}

/// One console-input poll. Public so the armed-prep loop (tx_prep) can
/// interleave read-only work with the abort window.
pub fn poll_key() -> Option<Key> {
    let key = system::with_stdin(|stdin| stdin.read_key());
    let key = key.ok().flatten()?;
    // `\tx-ignore-key.txt`:列出的键值被视为不存在(幽灵键)。Esc 永远不可忽略。
    if is_ignored(key_code(key)) {
        return None;
    }
    Some(key)
}

/// 键值编码:Printable 取 UCS-2 码元,Special 取 0x1_0000 | 扫描码。
fn key_code(key: Key) -> u32 {
    match key {
        Key::Printable(c) => u32::from(u16::from(c)),
        Key::Special(s) => 0x1_0000 | u32::from(s.0),
    }
}

const NO_KEY: u32 = u32::MAX;
const MAX_IGNORED: usize = 8;
static IGNORED: [AtomicU32; MAX_IGNORED] = [const { AtomicU32::new(NO_KEY) }; MAX_IGNORED];

fn is_ignored(code: u32) -> bool {
    IGNORED.iter().any(|s| s.load(Ordering::Relaxed) == code)
}

/// 解析 `tx-ignore-key.txt` 的一行,返回键值编码。接受 ABORT 日志行里打印的写法:
///   `Special(LEFT)`          (Debug 输出,逐字匹配)
///   `Printable('z')`、`'\u{1a}'`
///   `0x1A` / `26`            (Printable 码元)
///   `scan 0x04`              (Special 扫描码)
fn parse_key_line(line: &str) -> Option<u32> {
    let t = line.trim();
    if t.is_empty() {
        return None;
    }
    if t.starts_with("Special(") {
        return (0u16..0x100)
            .find(|&sc| format!("{:?}", Key::Special(ScanCode(sc))) == t)
            .map(|sc| 0x1_0000 | u32::from(sc));
    }
    if let Some(rest) = t.strip_prefix("scan") {
        return parse_num(rest.trim()).filter(|&n| n < 0x100).map(|n| 0x1_0000 | n);
    }
    let inner = t
        .strip_prefix("Printable(")
        .and_then(|r| r.strip_suffix(')'))
        .unwrap_or(t);
    if let Some(q) = inner.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
        if let Some(h) = q.strip_prefix("\\u{").and_then(|r| r.strip_suffix('}')) {
            return u32::from_str_radix(h, 16).ok().filter(|&n| n <= 0xFFFF);
        }
        let mut it = q.chars();
        return match (it.next(), it.next()) {
            (Some(c), None) if (c as u32) <= 0xFFFF => Some(c as u32),
            _ => None,
        };
    }
    parse_num(inner).filter(|&n| n <= 0xFFFF)
}

fn parse_num(t: &str) -> Option<u32> {
    match t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        Some(h) => u32::from_str_radix(h, 16).ok(),
        None => t.parse().ok(),
    }
}

/// 加载 `\tx-ignore-key.txt`(每行一个键,`#` 起注释;坏行记录后跳过)。
pub fn load_ignore_keys(content: &[u8]) {
    let Ok(text) = core::str::from_utf8(content) else {
        info!("KEYS: tx-ignore-key.txt is not UTF-8 - ignored");
        return;
    };
    let esc = 0x1_0000 | u32::from(ScanCode::ESCAPE.0);
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some(code) = parse_key_line(line) else {
            info!("KEYS: tx-ignore-key.txt line '{line}' not understood - skipped");
            continue;
        };
        if code == esc {
            info!("KEYS: tx-ignore-key.txt '{line}' is Esc - never ignorable, skipped");
            continue;
        }
        let added = IGNORED.iter().any(|s| {
            s.compare_exchange(NO_KEY, code, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
        });
        if added {
            info!("KEYS: tx-ignore-key.txt - ignoring '{line}' (treated as not pressed)");
        } else {
            info!("KEYS: tx-ignore-key.txt - more than {MAX_IGNORED} entries, '{line}' dropped");
        }
    }
}

/// Poll the console input roughly every 50 ms.
///
/// Fallback-picker semantics (chainload.rs): any key proceeds early,
/// timeout proceeds anyway.
pub fn wait_any_key_or_timeout(seconds: u64) -> KeyOrTimeout {
    let ticks = seconds * 20; // 50 ms each
    for _ in 0..ticks {
        if let Some(key) = poll_key() {
            return KeyOrTimeout::Key(key);
        }
        boot::stall(Duration::from_millis(50));
    }
    KeyOrTimeout::Timeout
}

/// CONFIRM-phase wait (issue #4.1). TX builds: ONLY Space or Enter proceeds
/// (Enter after Space-muscle-memory from shell/BIOS menus; before it aborted
/// and the run looked "finished" while nothing happened - easy to misread
/// as a failed attempt); any other key or the 30s timeout aborts into
/// chain-load with zero GPU writes and says so explicitly, so a stick left
/// in a machine that happens to boot from it can never run the transaction
/// unattended. Read-only builds keep the M0 semantics (any key / timeout
/// proceeds; everything is zero-write by design).
pub fn wait_confirm(seconds: u64) -> Confirm {
    let ticks = seconds * 20;
    for _ in 0..ticks {
        if let Some(key) = poll_key() {
            #[cfg(feature = "tx")]
            match key {
                Key::Printable(c) if u16::from(c) == 0x20 => {
                    info!("CONFIRM: Space pressed, proceeding");
                    return Confirm::Proceed;
                }
                Key::Printable(c) if u16::from(c) == 0x0d => {
                    info!("CONFIRM: Enter pressed, proceeding");
                    return Confirm::Proceed;
                }
                other => {
                    info!(
                        "CONFIRM: {other:?} - ABORT to Windows, NOTHING was unlocked this boot (zero GPU writes)"
                    );
                    return Confirm::Abort;
                }
            }
            #[cfg(not(feature = "tx"))]
            {
                info!("CONFIRM: key {key:?} pressed, proceeding (read-only build)");
                return Confirm::Proceed;
            }
        }
        boot::stall(Duration::from_millis(50));
    }
    #[cfg(feature = "tx")]
    {
        info!("CONFIRM: {seconds}s timeout - ABORT to Windows, NOTHING was unlocked this boot (zero GPU writes)");
        Confirm::Abort
    }
    #[cfg(not(feature = "tx"))]
    {
        info!("CONFIRM: timeout, proceeding (read-only default)");
        Confirm::Proceed
    }
}

/// Armed CONFIRM (`\i_know_what_i_am_doing.txt` on the stick root): the
/// stick itself carries the operator's declaration of intent, so the wait
/// collapses to a short countdown that PROCEEDS on timeout; any key during
/// the window is a human saying stop - TX builds abort into chain-load
/// with zero GPU writes, read-only builds have nothing to abort (they
/// proceed; everything is zero-write by design). TX builds route the armed
/// wait through tx_prep (prep interleaved) and only reach this fn in
/// non-tx builds.
#[cfg_attr(feature = "tx", allow(dead_code))]
pub fn wait_confirm_armed(seconds: u64) -> Confirm {
    let ticks = seconds * 20;
    for _ in 0..ticks {
        if let Some(key) = poll_key() {
            #[cfg(feature = "tx")]
            {
                info!(
                    "CONFIRM: key {key:?} during the {seconds}s armed countdown - ABORT (no GPU writes this boot)"
                );
                return Confirm::Abort;
            }
            #[cfg(not(feature = "tx"))]
            {
                info!("CONFIRM: key {key:?} during the armed countdown, proceeding (read-only build)");
                return Confirm::Proceed;
            }
        }
        boot::stall(Duration::from_millis(50));
    }
    info!("CONFIRM: {seconds}s armed countdown elapsed - PROCEED (marker file opted in)");
    Confirm::Proceed
}
