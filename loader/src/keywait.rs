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

/// One console-input poll. Public so the armed-prep loop (tx_prep) can
/// interleave read-only work with the abort window.
/// 本次启动被判定为"幽灵键"的键值被视为不存在(见 settle_input_once)。
pub fn poll_key() -> Option<Key> {
    let key = system::with_stdin(|stdin| stdin.read_key());
    let key = key.ok().flatten()?;
    if is_suppressed(key_code(key)) {
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
/// 4. 健康路径短路:排空为 0 且第一次轮询立即安静 => 立刻返回(约一次 50 ms);
/// 5. 否则进入安静窗口:连续 QUIET_MS 没有按键即结束,最长 MAX_SETTLE_MS。
///    窗口内同一键值连续出现 >= 3 次(无安静间隙)=> 判定为幽灵键,本次启动
///    抑制该键值(Esc 除外),并在控制台提示。其余键的含义不变:
///    Space/Enter 仍然确认,其他键仍然中止,超时仍然中止。
/// 日志只记录前 3 个被丢弃的键,避免被持续上报的键刷满日志。
pub fn settle_input_once(between: impl FnOnce()) {
    const MAX_DRAIN: u32 = 64;
    const POLL_MS: u64 = 50;
    const QUIET_MS: u64 = 1000;
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

    // 健康路径:没有任何缓冲键,且第一次轮询立即安静,不付出安静窗口的 1 s。
    let first = poll_key();
    if drained == 0 && first.is_none() {
        info!("INPUT-SETTLE: input quiet, no settle window needed");
        return;
    }

    let mut pending = first;
    let (mut waited, mut quiet, mut keys) = (0u64, 0u64, 0u32);
    rep.quiet();
    while waited < MAX_SETTLE_MS && quiet < QUIET_MS {
        match pending.take().or_else(poll_key) {
            Some(key) => {
                keys += 1;
                quiet = 0;
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
    if quiet < QUIET_MS {
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
