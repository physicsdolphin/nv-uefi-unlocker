//! Monotonic stage timing (issue #38): every log line gains a
//! `[+  1234ms]` prefix so the slow-GOP echo cost and the real transaction
//! waits (FWSEC/scrub/GFW) separate in the evidence record - the 1-2 min
//! complaint decomposes into echo vs wait only with per-line stamps.
//!
//! Clock: rdtsc, calibrated once against a 100 ms `boot::stall` at the top
//! of `main`. Millisecond display over a minutes-long run; a nonsense
//! calibration rate degrades to prefix-less lines (marker matching never
//! depended on the prefix). No allocation, no boot-services calls after
//! init.

use core::arch::x86_64::_rdtsc;
use core::cell::UnsafeCell;
use core::time::Duration;

struct Clock {
    t0: u64,
    ticks_per_ms: u64,
}

struct ClockCell(UnsafeCell<Option<Clock>>);

// Safety: UEFI applications are single-threaded during boot services.
unsafe impl Sync for ClockCell {}
unsafe impl Send for ClockCell {}

static CLOCK: ClockCell = ClockCell(UnsafeCell::new(None));

/// Calibrate against a 100 ms stall; called once at the top of `main`,
/// before the first log line. ~100 ms of boot time is the price.
pub fn init() {
    let a = unsafe { _rdtsc() };
    uefi::boot::stall(Duration::from_millis(100));
    let b = unsafe { _rdtsc() };
    let per_ms = b.wrapping_sub(a) / 100;
    // Safety: single-threaded boot services; written once, before any reader.
    unsafe {
        *CLOCK.0.get() = (per_ms > 0).then(|| Clock { t0: a, ticks_per_ms: per_ms });
    }
}

/// Milliseconds since [`init`], or `None` before calibration / if the
/// calibration produced a nonsense rate.
pub fn ms_since_entry() -> Option<u64> {
    // Safety: single-threaded boot services; set once by init().
    let c = unsafe { (*CLOCK.0.get()).as_ref() }?;
    Some(unsafe { _rdtsc() }.wrapping_sub(c.t0) / c.ticks_per_ms)
}
