//! Single-key confirmation with timeout (TOTORONG interaction pattern).

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

/// Check the actual system-table consoles, not an unrelated protocol handle.
pub fn stdin_available() -> bool {
    uefi::table::system_table_raw().is_some_and(|st| unsafe { !st.as_ref().stdin.is_null() })
}

pub fn stdout_available() -> bool {
    uefi::table::system_table_raw().is_some_and(|st| unsafe { !st.as_ref().stdout.is_null() })
}

/// One console-input poll. Public so the armed-prep loop (tx_prep) can
/// interleave read-only work with the abort window.
/// 本次启动被判定为"幽灵键"的键值被视为不存在(见 settle_input_once)。
pub fn poll_key() -> Option<Key> {
    if !stdin_available() { return None; }
    let key = system::with_stdin(|stdin| stdin.read_key());
    let key = key.ok().flatten()?;
    let code = key_code(key);
    // `	x-ignore-key.txt` zuerst (Benutzerkonfiguration), dann die
    // boot-lokale Unterdrückung; ein Wert in beiden Tabellen zählt als ignoriert.
    if is_ignored(code) {
        return None;
    }
    if is_suppressed(code) {
        SWALLOWED.fetch_add(1, Ordering::Relaxed);
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
const MAX_SUPPRESSED: usize = 4;
/// 本次启动内被抑制的键值(最多 MAX_SUPPRESSED 个)。
static SUPPRESSED: [AtomicU32; MAX_SUPPRESSED] = [const { AtomicU32::new(NO_KEY) }; MAX_SUPPRESSED];
/// 被抑制键值被吞掉的次数,只用于日志汇总。
static SWALLOWED: AtomicU32 = AtomicU32::new(0);

fn is_suppressed(code: u32) -> bool {
    SUPPRESSED.iter().any(|s| s.load(Ordering::Relaxed) == code)
}

const MAX_IGNORED: usize = 8;
/// `\tx-ignore-key.txt` 用户配置的忽略键值(最多 MAX_IGNORED 个);
/// 与上面逐次启动检测出的 SUPPRESSED 表相互独立。
static IGNORED: [AtomicU32; MAX_IGNORED] = [const { AtomicU32::new(NO_KEY) }; MAX_IGNORED];

fn is_ignored(code: u32) -> bool {
    IGNORED.iter().any(|s| s.load(Ordering::Relaxed) == code)
}

/// uefi 0.41 `ScanCode` 的全部具名变体 `(Debug 名, 扫描码)`。用常量构造,
/// 因此按名称匹配时无需构造任何枚举值(也就不会产生无名变体)。
/// sim/src/keygate.rs 镜像此表并固定数值。
const SCAN_NAMES: &[(&str, u16)] = &[
    ("UP", ScanCode::UP.0),
    ("DOWN", ScanCode::DOWN.0),
    ("RIGHT", ScanCode::RIGHT.0),
    ("LEFT", ScanCode::LEFT.0),
    ("HOME", ScanCode::HOME.0),
    ("END", ScanCode::END.0),
    ("INSERT", ScanCode::INSERT.0),
    ("DELETE", ScanCode::DELETE.0),
    ("PAGE_UP", ScanCode::PAGE_UP.0),
    ("PAGE_DOWN", ScanCode::PAGE_DOWN.0),
    ("FUNCTION_1", ScanCode::FUNCTION_1.0),
    ("FUNCTION_2", ScanCode::FUNCTION_2.0),
    ("FUNCTION_3", ScanCode::FUNCTION_3.0),
    ("FUNCTION_4", ScanCode::FUNCTION_4.0),
    ("FUNCTION_5", ScanCode::FUNCTION_5.0),
    ("FUNCTION_6", ScanCode::FUNCTION_6.0),
    ("FUNCTION_7", ScanCode::FUNCTION_7.0),
    ("FUNCTION_8", ScanCode::FUNCTION_8.0),
    ("FUNCTION_9", ScanCode::FUNCTION_9.0),
    ("FUNCTION_10", ScanCode::FUNCTION_10.0),
    ("FUNCTION_11", ScanCode::FUNCTION_11.0),
    ("FUNCTION_12", ScanCode::FUNCTION_12.0),
    ("ESCAPE", ScanCode::ESCAPE.0),
    ("FUNCTION_13", ScanCode::FUNCTION_13.0),
    ("FUNCTION_14", ScanCode::FUNCTION_14.0),
    ("FUNCTION_15", ScanCode::FUNCTION_15.0),
    ("FUNCTION_16", ScanCode::FUNCTION_16.0),
    ("FUNCTION_17", ScanCode::FUNCTION_17.0),
    ("FUNCTION_18", ScanCode::FUNCTION_18.0),
    ("FUNCTION_19", ScanCode::FUNCTION_19.0),
    ("FUNCTION_20", ScanCode::FUNCTION_20.0),
    ("FUNCTION_21", ScanCode::FUNCTION_21.0),
    ("FUNCTION_22", ScanCode::FUNCTION_22.0),
    ("FUNCTION_23", ScanCode::FUNCTION_23.0),
    ("FUNCTION_24", ScanCode::FUNCTION_24.0),
    ("MUTE", ScanCode::MUTE.0),
    ("VOLUME_UP", ScanCode::VOLUME_UP.0),
    ("VOLUME_DOWN", ScanCode::VOLUME_DOWN.0),
    ("BRIGHTNESS_UP", ScanCode::BRIGHTNESS_UP.0),
    ("BRIGHTNESS_DOWN", ScanCode::BRIGHTNESS_DOWN.0),
    ("SUSPEND", ScanCode::SUSPEND.0),
    ("HIBERNATE", ScanCode::HIBERNATE.0),
    ("TOGGLE_DISPLAY", ScanCode::TOGGLE_DISPLAY.0),
    ("RECOVERY", ScanCode::RECOVERY.0),
    ("EJECT", ScanCode::EJECT.0),
];

/// 解析 `tx-ignore-key.txt` 的一行,返回键值编码。接受 ABORT 日志行里打印的写法:
///   `Special(LEFT)`          (按名称查 SCAN_NAMES,不构造枚举值)
///   `Special(ScanCode(n))`   (无名扫描码,n 为十进制或 0x 十六进制)
///   `Printable('z')`、`'\u{1a}'`
///   `0x1A` / `26`            (Printable 码元)
///   `scan 0x104`             (Special 扫描码,任意 u16)
fn parse_key_line(line: &str) -> Option<u32> {
    let t = line.trim();
    if t.is_empty() {
        return None;
    }
    if let Some(name) = t.strip_prefix("Special(").and_then(|r| r.strip_suffix(')')) {
        if let Some(n) = name
            .strip_prefix("ScanCode(")
            .and_then(|r| r.strip_suffix(')'))
        {
            return parse_num(n.trim())
                .filter(|&n| n <= 0xFFFF)
                .map(|n| 0x1_0000 | n);
        }
        return SCAN_NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|&(_, sc)| 0x1_0000 | u32::from(sc));
    }
    if let Some(rest) = t.strip_prefix("scan") {
        return parse_num(rest.trim())
            .filter(|&n| n <= 0xFFFF)
            .map(|n| 0x1_0000 | n);
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

/// 抑制一个键值。Esc 永远不可被抑制:被按住的真人按键与幽灵键
/// 表现相同,保留 Esc 才能让人工中止始终可用。
fn suppress(key: Key) -> bool {
    let code = key_code(key);
    if matches!(key, Key::Special(ScanCode::ESCAPE)) {
        return false;
    }
    SUPPRESSED
        .iter()
        .any(|s| s.compare_exchange(NO_KEY, code, Ordering::Relaxed, Ordering::Relaxed).is_ok())
}

/// 连续轮询中同一键值的重复计数(中间没有安静间隙)。
struct Repeat {
    last: u32,
    run: u32,
}

impl Repeat {
    const MIN: u32 = 3;
    const fn new() -> Self {
        Repeat { last: NO_KEY, run: 0 }
    }
    fn quiet(&mut self) {
        self.last = NO_KEY;
        self.run = 0;
    }
    /// 记录一次按键;同一键值连续出现达到 MIN 次时返回 true。
    fn feed(&mut self, key: Key) -> bool {
        let code = key_code(key);
        if code == self.last {
            self.run += 1;
        } else {
            self.last = code;
            self.run = 1;
        }
        self.run == Self::MIN
    }
}

/// 倒计时开始前输入只运行一次(此时还没有任何中止窗口):
/// 1. 复位 ConIn(不做扩展自检;很多固件返回 UNSUPPORTED,仅记录);
/// 2. 丢弃已缓冲的按键(上限 64 个,不会被持续上报的设备卡死);
/// 3. 调用 `between`(USB 设备在第一次读取控制台之后才出现,见 usbreset);
/// 4. 安静窗口:排空为 0 时只需连续 FAST_QUIET_MS(300 ms)安静即结束,看到过按键
///    后需要连续 QUIET_MS(1 s);最长 MAX_SETTLE_MS。不能只看第一次轮询:实测
///    被持续上报的键在第一次轮询之后约 50 ms 才开始(见 PR 讨论)。
///    窗口内同一键值连续出现 >= 3 次(无安静间隙)=> 判定为幽灵键,本次启动
///    抑制该键值(Esc 除外),并在控制台提示。其余键的含义不变:
///    Space/Enter 仍然确认,其他键仍然中止,超时仍然中止。
/// 日志只记录前 3 个被丢弃的键,避免被持续上报的键刷满日志。
pub fn settle_input_once(between: impl FnOnce()) {
    const MAX_DRAIN: u32 = 64;
    const POLL_MS: u64 = 50;
    const QUIET_MS: u64 = 1000;
    const FAST_QUIET_MS: u64 = 300;
    const MAX_SETTLE_MS: u64 = 4000;

    match system::with_stdin(|stdin| stdin.reset(false)) {
        Ok(()) => info!("INPUT-SETTLE: ConIn reset ok"),
        Err(e) => info!("INPUT-SETTLE: ConIn reset failed: {:?} (continuing)", e.status()),
    }
    let mut rep = Repeat::new();
    let mut drained = 0u32;
    while drained < MAX_DRAIN {
        match poll_key() {
            Some(key) => {
                drained += 1;
                if drained <= 3 {
                    info!("INPUT-SETTLE: discarded buffered key {key:?}");
                }
                if rep.feed(key) {
                    announce_suppression(key, Repeat::MIN);
                    suppress(key);
                }
            }
            None => break,
        }
    }
    info!("INPUT-SETTLE: drained {drained} buffered key(s)");

    between();

    // 健康路径只付出 FAST_QUIET_MS;一旦看到按键就改用完整的 QUIET_MS。
    let mut need = if drained == 0 { FAST_QUIET_MS } else { QUIET_MS };
    let (mut waited, mut quiet, mut keys) = (0u64, 0u64, 0u32);
    rep.quiet();
    while waited < MAX_SETTLE_MS && quiet < need {
        match poll_key() {
            Some(key) => {
                keys += 1;
                quiet = 0;
                need = QUIET_MS;
                if keys <= 3 {
                    info!("INPUT-SETTLE: settle: discarded key {key:?} at +{waited} ms");
                }
                if rep.feed(key) {
                    announce_suppression(key, Repeat::MIN);
                    suppress(key);
                }
            }
            None => {
                quiet += POLL_MS;
                rep.quiet();
            }
        }
        boot::stall(Duration::from_millis(POLL_MS));
        waited += POLL_MS;
    }
    if quiet < need {
        info!("INPUT-SETTLE: input still busy after {waited} ms but no key value repeated - key semantics unchanged");
    }
    info!(
        "INPUT-SETTLE: settle done after {waited} ms, discarded {keys} key(s), quiet {quiet} ms, {} swallowed after suppression",
        SWALLOWED.load(Ordering::Relaxed)
    );
}

/// 控制台提示(日志与屏幕同时可见,见 logging.rs 的 CONSOLE_QUIET_MARKERS)。
fn announce_suppression(key: Key, n: u32) {
    if matches!(key, Key::Special(ScanCode::ESCAPE)) {
        info!("INPUT-SETTLE: {key:?} repeating - held Esc is never suppressed");
        return;
    }
    info!("INPUT-SETTLE: {key:?} repeated {n}x - phantom key, suppressed for this boot");
    info!("INPUT-SETTLE: add it to \\tx-ignore-key.txt to silence permanently");
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

/// DXE file-load path only (TPL_APPLICATION), for BOTH build classes.
pub fn wait_dxe_countdown(seconds: u64) -> Confirm {
    for _ in 0..seconds * 20 {
        if !stdin_available() { return Confirm::Abort; }
        match system::with_stdin(|stdin| stdin.read_key()) {
            Ok(Some(_)) | Err(_) => return Confirm::Abort,
            Ok(None) => {},
        }
        boot::stall(Duration::from_millis(50));
    }
    Confirm::Proceed
}

pub fn explicit_authorize(seconds: u64) -> bool {
    if !stdin_available() { return false; }
    for _ in 0..seconds * 20 {
        match system::with_stdin(|stdin| stdin.read_key()) {
            Ok(Some(Key::Printable(c))) => return matches!(u16::from(c), 0x20 | 0x0d),
            Ok(Some(_)) | Err(_) => return false,
            Ok(None) => {},
        }
        boot::stall(Duration::from_millis(50));
    }
    false
}
