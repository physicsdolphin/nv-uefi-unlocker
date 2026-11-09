//! Single-key confirmation with timeout (TOTORONG interaction pattern).

use core::sync::atomic::{AtomicBool, Ordering};
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
    if ESC_ONLY.load(Ordering::Relaxed) && !matches!(key, Key::Special(ScanCode::ESCAPE)) {
        return None;
    }
    Some(key)
}

/// 倒计时开始前输入一直没有安静下来(某个键被持续上报,例如带键盘接口的
/// 鼠标/接收器)时置位。此后只有 Esc 算按键,其余键被忽略,否则武装倒计时
/// 会被这个"卡住的键"立刻中止。
static ESC_ONLY: AtomicBool = AtomicBool::new(false);

/// CONFIRM 阶段之前只运行一次(此时还没有任何中止窗口):
/// 1. 复位 ConIn(不做扩展自检;很多固件返回 UNSUPPORTED,仅记录);
/// 2. 丢弃已缓冲的按键(上限 64 个,不会被持续上报的设备卡死);
/// 3. 调用 `between`(USB 设备在第一次读取控制台之后才出现,见 usbreset);
/// 4. 安静窗口:连续 QUIET_MS 没有按键即结束,最长 MAX_SETTLE_MS。窗口内的键
///    全部丢弃。窗口结束时输入仍未安静 => 进入 ESC-ONLY。
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
    let mut drained = 0u32;
    while drained < MAX_DRAIN {
        match poll_key() {
            Some(key) => {
                drained += 1;
                if drained <= 3 {
                    info!("INPUT-SETTLE: discarded buffered key {key:?}");
                }
            }
            None => break,
        }
    }
    info!("INPUT-SETTLE: drained {drained} buffered key(s)");

    between();

    let (mut waited, mut quiet, mut keys) = (0u64, 0u64, 0u32);
    while waited < MAX_SETTLE_MS && quiet < QUIET_MS {
        if let Some(key) = poll_key() {
            keys += 1;
            quiet = 0;
            if keys <= 3 {
                info!("INPUT-SETTLE: settle: discarded key {key:?} at +{waited} ms");
            }
        } else {
            quiet += POLL_MS;
        }
        boot::stall(Duration::from_millis(POLL_MS));
        waited += POLL_MS;
    }
    if quiet < QUIET_MS {
        ESC_ONLY.store(true, Ordering::Relaxed);
        info!("INPUT-SETTLE: input never went quiet (stuck/streaming key) - countdown abort is now ESC-ONLY, other keys are ignored");
    }
    info!("INPUT-SETTLE: settle done after {waited} ms, discarded {keys} key(s), quiet {quiet} ms");
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
