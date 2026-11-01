//! Armed-path read-only prep (issue #38 follow-up): between the banner and
//! the auto-run deadline, do everything that needs no GPU write and no
//! exclusivity - PCI enumeration, GOP probe, cold-state sample, tx target
//! selection, and the family asset reads into the tx.rs read-through
//! cache. The post-countdown path then starts the transaction from RAM.
//!
//! Safety shape (constitution unchanged): every step is read-only
//! (config/MMIO reads, USB file reads); any key still aborts with zero GPU
//! writes; the transaction itself (detach/claim first GPU touch) starts
//! only after PROCEED, exactly as before. Key-check latency is bounded by
//! the longest single step (one 29-84 MB asset read, 1-4 s on USB2 sticks).

use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::time::Duration;
use log::info;

use crate::keywait::{self, Confirm};
use crate::pci::{self, FoundGpu};

const TICKS_PER_SEC: u64 = 20; // 50 ms poll cadence, same as keywait

/// Steps, in the same order the post-confirm path runs them, so the log
/// shape is identical whether prep ran or not.
const STEP_NAMES: &[&str] = &["enumerate", "gop-probe", "sample", "select", "assets"];

struct PrepState {
    step: usize,
    gpus: Option<Vec<FoundGpu>>,
    families: Vec<&'static str>,
}

impl PrepState {
    fn new() -> Self {
        PrepState { step: 0, gpus: None, families: Vec::new() }
    }

    fn done(&self) -> bool {
        self.step >= STEP_NAMES.len()
    }

    /// One read-only step; every step logs the same lines its post-confirm
    /// counterpart would.
    fn step(&mut self) {
        match self.step {
            0 => {
                self.gpus = Some(pci::enumerate());
            }
            1 => {
                crate::gop::enumerate_gop();
            }
            2 => {
                if let Some(gpus) = &self.gpus {
                    if let Some(gpu) = gpus.first() {
                        crate::sample::run(gpu);
                    } else {
                        info!("SAMPLE: no NVIDIA GPU present, skipped (negative control path)");
                    }
                }
            }
            3 => {
                if let Some(gpus) = &self.gpus {
                    for t in pci::select_tx_targets(gpus) {
                        if let Some(family) = crate::runner::asset_family(t.device_id) {
                            if !self.families.contains(&family) {
                                self.families.push(family);
                            }
                        }
                    }
                    if self.families.is_empty() {
                        info!("PREP: select found no supported target - asset step will be empty");
                    }
                }
            }
            4 => {
                for family in &self.families {
                    crate::tx::prefetch_family_assets(family);
                }
            }
            _ => {}
        }
        self.step += 1;
    }
}

struct GpusCell(UnsafeCell<Option<Vec<FoundGpu>>>);

// Safety: UEFI applications are single-threaded during boot services.
unsafe impl Sync for GpusCell {}
unsafe impl Send for GpusCell {}

static PREP_GPUS: GpusCell = GpusCell(UnsafeCell::new(None));

/// Hand the prep's enumeration to the post-confirm path (Some = the armed
/// prep ran and main.rs must not re-enumerate / re-probe / re-sample).
/// Consumed once.
pub fn take_gpus() -> Option<Vec<FoundGpu>> {
    // Safety: single-threaded boot services; pointer use confined here.
    unsafe { (*PREP_GPUS.0.get()).take() }
}

/// Armed countdown interleaved with prep steps. Contract matches
/// `keywait::wait_confirm_armed`: PROCEED at the deadline, ANY key aborts
/// (prep is read-only, so aborting mid-prep is trivially safe). The
/// deadline is wall-clock from entry via the calibrated timing clock; if
/// prep outlives it, the steps finish first - the marker already declared
/// intent, and idling would only delay the run. A missing clock falls back
/// to 50 ms tick counting, same as keywait.
pub fn armed_confirm_with_prep(seconds: u64) -> Confirm {
    let mut prep = PrepState::new();
    let t0 = crate::timing::ms_since_entry();
    let deadline_ms = t0.map(|t| t.saturating_add(seconds.saturating_mul(1000)));
    let mut waited_ticks: u64 = 0;
    let budget_ticks = seconds.saturating_mul(TICKS_PER_SEC);
    loop {
        if prep.done() {
            let deadline_hit = match (deadline_ms, crate::timing::ms_since_entry()) {
                (Some(dl), Some(now)) => now >= dl,
                _ => {
                    waited_ticks += 1;
                    waited_ticks >= budget_ticks
                }
            };
            if deadline_hit {
                info!(
                    "CONFIRM: {seconds}s armed countdown elapsed - PROCEED (marker file opted in)"
                );
                // Safety: single-threaded boot services.
                unsafe {
                    *PREP_GPUS.0.get() = prep.gpus.take();
                }
                return Confirm::Proceed;
            }
        } else {
            let name = STEP_NAMES[prep.step];
            info!("PREP: {name} begin");
            prep.step();
        }
        if let Some(key) = keywait::poll_key() {
            info!(
                "CONFIRM: key {key:?} during the {seconds}s armed countdown - ABORT (no GPU writes this boot)"
            );
            return Confirm::Abort;
        }
        uefi::boot::stall(Duration::from_millis(50));
    }
}
