//! EBS observer (`\tx-ebs-watch.txt`) - the runtime instrument the soft-lock
//! family diagnosis was missing (cross-review 2026-11-22, issues #242/#278/
//! #279): the ExitBootServices handoff was only ever reconstructed statically
//! (module inventories, callback decompiles, edk2 source walks). Nobody ever
//! OBSERVED whether EBS is entered at all, whether the DxeCore fanout
//! returns, and whether the memory map is being dirtied between the OS
//! loader's GetMemoryMap and the real CoreExitBootServices. This module
//! wraps the two boot-services entries the observation needs and arms the
//! two fanout checkpoints the edk2 sequence allows:
//!
//! ```text
//!   ENTER   (wrapper pre-call)   EBS called by the OS loader / GRUB
//!   BEFORE  (BeforeEBS notify)   DxeCore fanout stage 1 entered
//!         (timer kill + TerminateMemoryMap happen between BEFORE and NOTIFY;
//!          a FAILED TerminateMemoryMap = EFI_INVALID_PARAMETER = services
//!          stay alive = the loader retries with a fresh map)
//!   NOTIFY  (EBS-group notify)   mid-fanout checkpoint (map TERMINATED!)
//!   RETURN  (wrapper post-call)  the real CoreExitBootServices returned
//! ```
//!
//! Readbacks per EBS call land in `\ebs-watch.log` (enter / error-return
//! lines with the MapKey and the GetMemoryMap counters), on the framebuffer
//! rows, and on the raw port channels. edk2 facts this leans on
//! (MdeModulePkg Core/Dxe/DxeMain/DxeMain.c `CoreExitBootServices`):
//!
//! - a FAILED EBS keeps boot services alive (early return, table untouched)
//!   -> file writes stay legal on that path;
//! - a SUCCESSFUL EBS zeroes the whole boot-services table and NULLs
//!   ST->ConOut/BootServices -> after RETURN-ok only the raw channels are
//!   legal (port I/O + framebuffer rows), nothing that could allocate;
//! - BeforeEBS fires exactly once per boot (`mExitBootServicesCalled` is
//!   not reset on a retry) -> the BEFORE file line cannot livelock.
//!
//! Write-budget discipline (the self-perturbation guard): every file write
//! in the EBS window may allocate (FAT driver, `format!`-free but the
//! Close/reopen commit chain calls LocateHandleBuffer) and an allocation
//! inside the window stales the caller's MapKey - an unbounded writer would
//! manufacture the very INVALID_PARAMETER retry loop this instrument exists
//! to observe. [`file_write_allowed`] bounds each line class (enter<=1 +
//! before<=1 collapse into at most ONE manufactured retry; error-return
//! lines are safe - the loader re-reads the map after them); the budget is
//! mirrored (textually identical) in sim/src/ebswatch_gate.rs.
//!
//! `\tx-ebs-selftest.txt`: the loader calls GetMemoryMap + ExitBootServices
//! ITSELF at the chain point (instead of StartImage'ing an OS loader) and
//! parks in a raw stall loop. Validates the whole instrument on any machine
//! - including QEMU/OVMF - without booting Windows. Power-cycle afterwards.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use log::info;
use uefi::boot::{self, EventType, Tpl};
use uefi::proto::console::gop::GraphicsOutput;
use uefi::proto::media::file::{File, FileAttribute, FileHandle, FileMode, RegularFile};
use uefi::{cstr16, guid, CStr16, Event, Identify};

use uefi_raw::table::boot::MemoryDescriptor;

/// `\ebs-watch.log` on the boot stick (our volume; the ESP stays untouched).
const WATCH_PATH: &CStr16 = cstr16!(r"\ebs-watch.log");
// The watch/selftest marker consts moved to config::flag(FlagArm::EbsWatch /
// EbsSelftest) - marker file or tx.ini key, double-set refused (issue #290).

const EBS_GROUP: uefi::Guid = guid!("27abf055-b1b8-4c26-8048-748f37baa2df");
const BEFORE_EBS_GROUP: uefi::Guid = guid!("8be0e274-3970-4b44-80c5-1ab9502f3bfc");

// ---- live state (raw-channel safe: no locks, no allocation) ----
static INSTALLED: AtomicBool = AtomicBool::new(false);
static ENTERED: AtomicUsize = AtomicUsize::new(0);
static RETURNED_OK: AtomicUsize = AtomicUsize::new(0);
static RETURNED_ERR: AtomicUsize = AtomicUsize::new(0);
static NOTIFY_BEFORE: AtomicUsize = AtomicUsize::new(0);
static NOTIFY_EBS: AtomicUsize = AtomicUsize::new(0);
static GMM_CALLS: AtomicUsize = AtomicUsize::new(0);
static GMM_ERR: AtomicUsize = AtomicUsize::new(0);
static GMM_LAST_KEY: AtomicUsize = AtomicUsize::new(0);
static GMM_RING_N: AtomicUsize = AtomicUsize::new(0);
static GMM_RING: [AtomicUsize; 16] = [const { AtomicUsize::new(0) }; 16];
static SERVICES_DEAD: AtomicBool = AtomicBool::new(false);
static REAL_EBS: AtomicUsize = AtomicUsize::new(0);
static REAL_GMM: AtomicUsize = AtomicUsize::new(0);
static FILE_ERROR: AtomicBool = AtomicBool::new(false);
// Framebuffer descriptor captured at install (0 = unusable).
static FB_BASE: AtomicUsize = AtomicUsize::new(0);
static FB_STRIDE: AtomicUsize = AtomicUsize::new(0);
static FB_ROWS: AtomicUsize = AtomicUsize::new(0);
/// 0 = BGR byte order, 1 = RGB, 0xff = unusable (Bitmask/BltOnly).
static FB_FMT: AtomicUsize = AtomicUsize::new(0xff);

// The committed log handle. Close/reopen per write (see commit_line).
// Safety: UEFI boot-services phases are single-threaded; post-EBS the
// handle is never touched again (SERVICES_DEAD gates every file write).
struct WatchFileSync(UnsafeCell<Option<RegularFile>>);
unsafe impl Sync for WatchFileSync {}
static WATCH_FILE: WatchFileSync = WatchFileSync(UnsafeCell::new(None));

// ---- write-budget policy (pure; sim/src/ebswatch_gate.rs mirrors this
// textually - do not reformat one side without the other) ----

/// Line classes with independent budgets.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WatchLine {
    /// Wrapper pre-call line (n = ENTERED count, 1-based).
    Enter,
    /// BeforeEBS notify line (fires once per boot on edk2).
    Before,
    /// Error-return line (services still alive after a failed EBS).
    ErrReturn,
}

/// Budget check for one line class. Every file write between the caller's
/// GetMemoryMap and the real CoreExitBootServices may allocate and stale
/// the caller's MapKey; these budgets keep our own perturbation finite.
/// Worst case is ONE manufactured INVALID_PARAMETER retry: the enter-line
/// and the BeforeEBS-notify line fire within the same first call, and on
/// the retry (n>=2) the enter line is budgeted out while the BeforeEBS
/// group does not re-fire (edk2 keeps `mExitBootServicesCalled` set). The
/// error-return line is safe by construction - the loader re-reads the map
/// AFTER our write on that path.
/// Mirrored (textually identical) in sim/src/ebswatch_gate.rs.
pub fn file_write_allowed(kind: WatchLine, count: usize) -> bool {
    match kind {
        WatchLine::Enter => count <= 1,
        WatchLine::Before => count <= 1,
        WatchLine::ErrReturn => count <= 2,
    }
}

/// CRC-32 (reflected 0xEDB88320, init/xorout 0xffffffff) - the UEFI table
/// header checksum algorithm (edk2 `CalculateCrc32`, MdePkg BaseLib
/// CheckSum.c). Textually identical mirror in sim/src/ebswatch_gate.rs.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    crc ^ 0xffff_ffff
}

// ---- raw channels (no boot services; legal before AND after EBS) ----

#[inline]
fn out8(port: u16, v: u8) {
    // Safety: raw port write; every port used here is harmless if absent.
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") v, options(nomem, nostack));
    }
}

#[inline]
fn in8(port: u16) -> u8 {
    let v: u8;
    // Safety: raw port read of a diagnostic-only status port.
    unsafe {
        core::arch::asm!("in al, dx", out("al") v, in("dx") port, options(nomem, nostack));
    }
    v
}

/// POST diagnostic port. Codes: 0xC0 install, 0xC1 BEFORE, 0xC2 NOTIFY,
/// 0xD0 RETURN-ok, 0xD1 RETURN-err, 0xE0|n ENTER n (n clamped 1..=15).
fn post(code: u8) {
    out8(0x80, code);
}

fn debugcon(bytes: &[u8]) {
    for &b in bytes {
        out8(0xE9, b);
    }
}

/// 16550 init at 115200 8N1. Harmless no-op where the port does not exist.
fn uart_init() {
    out8(0x3F8 + 1, 0x00); // disable interrupts
    out8(0x3F8 + 3, 0x80); // DLAB
    out8(0x3F8, 0x01); // divisor low: 1 = 115200
    out8(0x3F8 + 1, 0x00);
    out8(0x3F8 + 3, 0x03); // 8N1
    out8(0x3F8 + 2, 0xC7); // FIFO
    out8(0x3F8 + 4, 0x03); // DTR|RTS
}

fn uart_puts(bytes: &[u8]) {
    for &b in bytes {
        // THR-empty poll (LSR bit 5); a nonexistent port reads 0xff and the
        // loop falls through - the write goes nowhere, nothing hangs.
        for _ in 0..100_000 {
            if in8(0x3F8 + 5) & 0x20 != 0 {
                break;
            }
        }
        out8(0x3F8, b);
    }
}

/// Raw-text channels used at every checkpoint, including after RETURN-ok.
fn raw_text(bytes: &[u8]) {
    post(0x00); // frame separator on the POST display
    debugcon(bytes);
    uart_puts(bytes);
}

// ---- stack-only line builder (no allocation inside the EBS window) ----

struct Line {
    buf: [u8; 256],
    n: usize,
}

impl Line {
    fn new() -> Self {
        Self { buf: [0; 256], n: 0 }
    }
    fn push(&mut self, s: &[u8]) {
        let room = self.buf.len() - self.n;
        let take = s.len().min(room);
        self.buf[self.n..self.n + take].copy_from_slice(&s[..take]);
        self.n += take;
    }
    fn push_hex(&mut self, v: usize) {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        self.push(b"0x");
        if v == 0 {
            self.push(b"0");
            return;
        }
        let mut shift = usize::BITS - 4;
        while shift > 0 && (v >> shift) & 0xf == 0 {
            shift -= 4;
        }
        loop {
            self.push(&[HEX[((v >> shift) & 0xf) as usize]]);
            if shift == 0 {
                break;
            }
            shift -= 4;
        }
    }
    fn bytes(&self) -> &[u8] {
        &self.buf[..self.n]
    }
}

/// Enter line, e.g. `EBS-ENTER n=1 key=0x12 gmm=37/1 lastkey=0x11 distinct=4\r\n`
fn enter_line(n: usize, map_key: usize) -> Line {
    let mut l = Line::new();
    l.push(b"EBS-ENTER n=");
    push_dec(&mut l, n);
    l.push(b" key=");
    l.push_hex(map_key);
    l.push(b" gmm=");
    push_dec(&mut l, GMM_CALLS.load(Ordering::Relaxed));
    l.push(b"/");
    push_dec(&mut l, GMM_ERR.load(Ordering::Relaxed));
    l.push(b" lastkey=");
    l.push_hex(GMM_LAST_KEY.load(Ordering::Relaxed));
    l.push(b" distinct=");
    push_dec(&mut l, GMM_RING_N.load(Ordering::Relaxed));
    l.push(b"\r\n");
    l
}

/// Error-return line: `EBS-RETURN n=1 status=0x800000000000000e (services still alive)\r\n`
fn err_line(n: usize, status: usize) -> Line {
    let mut l = Line::new();
    l.push(b"EBS-RETURN n=");
    push_dec(&mut l, n);
    l.push(b" status=");
    l.push_hex(status);
    l.push(b" (services still alive - the caller retries with a fresh map)\r\n");
    l
}

fn push_dec(l: &mut Line, mut v: usize) {
    let mut tmp = [0u8; 20];
    let mut i = 0;
    loop {
        tmp[i] = b'0' + (v % 10) as u8;
        i += 1;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    while i > 0 {
        i -= 1;
        l.push(&tmp[i..i + 1]);
    }
}

// ---- framebuffer marks (raw linear writes; survive post-EBS) ----
// Row 1 blue = BEFORE reached; row 2 yellow = NOTIFY reached; row 3 green =
// RETURN-ok. A photo of the hung screen names the fanout stage directly.

fn fb_mark(row: usize, rgb: (u8, u8, u8)) {
    let base = FB_BASE.load(Ordering::Relaxed);
    let stride = FB_STRIDE.load(Ordering::Relaxed);
    let rows = FB_ROWS.load(Ordering::Relaxed);
    let fmt = FB_FMT.load(Ordering::Relaxed);
    if base == 0 || fmt == 0xff || stride == 0 || row >= rows {
        return;
    }
    let (r, g, b) = rgb;
    // 16 squares of 8x8 with 4 px gaps, starting at (16, 8 + row*20).
    for sq in 0..16usize {
        let x0 = 16 + sq * 12;
        let y0 = 8 + row * 20;
        for dy in 0..8usize {
            for dx in 0..8usize {
                let px = (y0 + dy) * stride + (x0 + dx);
                let p = (base + px * 4) as *mut u8;
                // Safety: inside the captured mode's visible area; the base,
                // stride and rows come from the GOP mode info at install.
                unsafe {
                    match fmt {
                        0 => {
                            // BGRX: B, G, R, reserved
                            p.write_volatile(b);
                            p.add(1).write_volatile(g);
                            p.add(2).write_volatile(r);
                        }
                        _ => {
                            // RGBX: R, G, B, reserved
                            p.write_volatile(r);
                            p.add(1).write_volatile(g);
                            p.add(2).write_volatile(b);
                        }
                    }
                }
            }
        }
    }
}

/// Capture the current GOP framebuffer descriptor (best effort). Raw
/// handle_protocol + read-only mode-struct reads: uefi-rs 0.41 keeps the
/// wrapper's raw pointer private, and the framebuffer descriptor must be
/// captured without disturbing the console stack the transaction may have
/// just rebuilt.
fn capture_fb() {
    let gop_guid: uefi_raw::Guid = uefi_raw::guid!("9042a9de-23dc-4a38-96fb-7aded080516a");
    let Ok(h) = boot::locate_handle_buffer(boot::SearchType::ByProtocol(&GraphicsOutput::GUID))
    else {
        info!("EBS-WATCH: no GOP handles - framebuffer rows unavailable");
        return;
    };
    let Some(st) = uefi::table::system_table_raw() else { return };
    // Safety: boot services live; raw protocol read, no state change.
    unsafe {
        let bt = st.as_ref().boot_services;
        if bt.is_null() {
            return;
        }
        for &handle in h.iter() {
            let mut raw: *mut uefi_raw::protocol::console::GraphicsOutputProtocol =
                core::ptr::null_mut();
            let s = ((*bt).handle_protocol)(
                handle.as_ptr(),
                &gop_guid,
                (&mut raw as *mut *mut uefi_raw::protocol::console::GraphicsOutputProtocol)
                    .cast(),
            );
            if s != uefi_raw::Status::SUCCESS || raw.is_null() || (*raw).mode.is_null() {
                continue;
            }
            let mode = &*(*raw).mode;
            if mode.info.is_null() {
                continue;
            }
            let info = &*mode.info;
            let pf = match info.pixel_format {
                uefi_raw::protocol::console::GraphicsPixelFormat::
                    PIXEL_BLUE_GREEN_RED_RESERVED_8_BIT_PER_COLOR => 0usize,
                uefi_raw::protocol::console::GraphicsPixelFormat::
                    PIXEL_RED_GREEN_BLUE_RESERVED_8_BIT_PER_COLOR => 1usize,
                _ => 0xffusize,
            };
            let (base, stride, w, ht) = (
                mode.frame_buffer_base as usize,
                info.pixels_per_scan_line as usize,
                info.horizontal_resolution as usize,
                info.vertical_resolution as usize,
            );
            if base == 0 || pf == 0xff || stride == 0 {
                continue;
            }
            FB_BASE.store(base, Ordering::Relaxed);
            FB_STRIDE.store(stride, Ordering::Relaxed);
            FB_ROWS.store((ht / 20).min(8), Ordering::Relaxed);
            FB_FMT.store(pf, Ordering::Relaxed);
            info!(
                "EBS-WATCH: framebuffer captured {base:#x} {w}x{ht} stride={stride} fmt={} (rows: blue=BEFORE yellow=NOTIFY green=RETURN)",
                if pf == 0 { "bgr" } else { "rgb" }
            );
            return;
        }
    }
    info!("EBS-WATCH: no usable GOP mode - framebuffer rows unavailable");
}

// ---- committed file channel (boot-services legal while SERVICES_DEAD=false) ----

/// Fresh file per boot: delete a stale one first, then create (install only;
/// same shape as logging::open_fresh_log).
fn open_watch_file() -> Option<RegularFile> {
    let mut fs = boot::get_image_file_system(boot::image_handle()).ok()?;
    let mut root: uefi::proto::media::file::Directory = fs.open_volume().ok()?;
    if let Ok(handle) = root.open(WATCH_PATH, FileMode::ReadWrite, FileAttribute::empty()) {
        if let Some(reg) = handle.into_regular_file() {
            let _ = reg.delete();
        }
    }
    let handle: FileHandle =
        root.open(WATCH_PATH, FileMode::CreateReadWrite, FileAttribute::empty()).ok()?;
    handle.into_regular_file()
}

/// Reopen the existing watch file at EOF (commit path - MUST NOT delete:
/// the commit chain close/reopens, and a delete here would destroy the
/// content the Close just committed; that exact bug cost the QEMU round -
/// every commit deleted its own line and the file stayed 0 bytes).
fn reopen_watch_file() -> Option<RegularFile> {
    let mut fs = boot::get_image_file_system(boot::image_handle()).ok()?;
    let mut root: uefi::proto::media::file::Directory = fs.open_volume().ok()?;
    let handle: FileHandle =
        root.open(WATCH_PATH, FileMode::ReadWrite, FileAttribute::empty()).ok()?;
    let mut file = handle.into_regular_file()?;
    file.set_position(RegularFile::END_OF_FILE).ok()?;
    Some(file)
}

/// Append one line and COMMIT it: write + Flush + Close + reopen(END). The
/// Close is the #122 lesson - on firmware where Flush is a no-op the dirent
/// (size) is only committed by Close, and a hang after this line must leave
/// a readable file. Count the Close/reopen's LocateHandleBuffer allocations
/// in the perturbation budget: see file_write_allowed.
fn commit_line(line: &Line) {
    // Safety: single-threaded boot services; the handle is confined here.
    unsafe {
        let slot = WATCH_FILE.0.get();
        if FILE_ERROR.load(Ordering::Relaxed) {
            return;
        }
        // Pre-StartImage close (pre_close_watch_file) leaves the slot None;
        // reopen on demand so a retried EBS (error path, services alive)
        // keeps its budgeted lines instead of dying on the closed handle.
        if (*slot).is_none() && !SERVICES_DEAD.load(Ordering::Relaxed) {
            match reopen_watch_file() {
                Some(f) => *slot = Some(f),
                None => {
                    FILE_ERROR.store(true, Ordering::Relaxed);
                    info!("EBS-WATCH: \\ebs-watch.log reopen failed - file channel closed, port+framebuffer channels stay live");
                    return;
                }
            }
        }
        let mut ok = false;
        if let Some(f) = (*slot).as_mut() {
            let pos0 = f.get_position().unwrap_or(0xffff_ffff);
            let sp = f.set_position(RegularFile::END_OF_FILE);
            let w = f.write(line.bytes());
            let w_ok = sp.is_ok() && w.is_ok();
            let pos1 = f.get_position().unwrap_or(0xffff_ffff);
            let fl = f.flush();
            info!(
                "EBS-WATCH: commit pos0={pos0} write={} pos1={pos1} flush={}",
                if w_ok { "ok" } else { "ERR" },
                if fl.is_ok() { "ok" } else { "ERR" }
            );
            ok = w_ok && fl.is_ok();
        }
        if ok {
            // Close (drops the handle -> commits the dirent), reopen at EOF.
            let _ = (*slot).take();
            match reopen_watch_file() {
                Some(f) => *slot = Some(f),
                None => FILE_ERROR.store(true, Ordering::Relaxed),
            }
        } else {
            FILE_ERROR.store(true, Ordering::Relaxed);
            // One console-only note (services alive on every commit_line
            // call path by construction of the budgets).
            info!("EBS-WATCH: \\ebs-watch.log write failed - file channel closed, port+framebuffer channels stay live");
        }
    }
}

/// Close the watch file before StartImage so the dirent is committed even on
/// SUCCESS runs: on a clean ExitBootServices nothing further is ever written
/// (post-EBS only raw channels are legal), and a handle left open at handoff
/// leaves the dirent uncommitted on FAT drivers that only commit on Close -
/// the #278 unlocked boot's 0-byte `\ebs-watch.log` shape (the same run's
/// data was in the cache; Windows saw a stale 0-byte dirent). Commit-time
/// reopen-on-demand keeps the error-path lines alive after this.
pub fn pre_close_watch_file() {
    if !INSTALLED.load(Ordering::Relaxed) {
        return;
    }
    // Safety: single-threaded boot services; drop = Close per spec.
    unsafe {
        let _ = (*WATCH_FILE.0.get()).take();
    }
}

// ---- wrapped table entries ----

/// ExitBootServices wrapper: log the call, run the real one, log the return.
/// Post-ok: NOTHING that could touch boot services (the table is zeroed and
/// ST->ConOut is NULL at that point) - raw channels only.
unsafe extern "efiapi" fn wrapped_exit_boot_services(
    image_handle: uefi_raw::Handle,
    map_key: usize,
) -> uefi_raw::Status {
    let n = ENTERED.fetch_add(1, Ordering::Relaxed) + 1;
    post(0xE0 | n.min(15) as u8);
    // Pre-call lines: bounded (our own writes may stale the caller's MapKey).
    // After the first successful EBS there is no pre-call ever again.
    if !SERVICES_DEAD.load(Ordering::Relaxed) {
        let line = enter_line(n, map_key);
        if file_write_allowed(WatchLine::Enter, n) {
            commit_line(&line);
        }
        raw_text(line.bytes());
    }
    let real = REAL_EBS.load(Ordering::Relaxed) as usize;
    if real == 0 {
        // Patch failed earlier: behave as if we were not here.
        return uefi_raw::Status::UNSUPPORTED;
    }
    let real: unsafe extern "efiapi" fn(uefi_raw::Handle, usize) -> uefi_raw::Status =
        core::mem::transmute(real);
    let st = real(image_handle, map_key);
    if st.is_success() {
        RETURNED_OK.fetch_add(1, Ordering::Relaxed);
        SERVICES_DEAD.store(true, Ordering::Relaxed);
        post(0xD0);
        fb_mark(2, (0, 255, 0)); // row 3: green
        raw_text(b"EBS-RETURN ok (boot services terminated)\r\n");
    } else {
        RETURNED_ERR.fetch_add(1, Ordering::Relaxed);
        post(0xD1);
        let e = RETURNED_ERR.load(Ordering::Relaxed);
        let line = err_line(n, st.0);
        if file_write_allowed(WatchLine::ErrReturn, e) {
            commit_line(&line);
        }
        raw_text(line.bytes());
    }
    st
}

/// GetMemoryMap wrapper: counters only (calls, errors, last MapKey, distinct
/// MapKeys in a 16-slot ring). No per-call output - this is called often by
/// every driver; the counters ride in the EBS-ENTER lines.
unsafe extern "efiapi" fn wrapped_get_memory_map(
    size: *mut usize,
    map: *mut MemoryDescriptor,
    key: *mut usize,
    desc_size: *mut usize,
    desc_version: *mut u32,
) -> uefi_raw::Status {
    let real = REAL_GMM.load(Ordering::Relaxed) as usize;
    if real == 0 {
        return uefi_raw::Status::UNSUPPORTED;
    }
    let real: unsafe extern "efiapi" fn(
        *mut usize,
        *mut MemoryDescriptor,
        *mut usize,
        *mut usize,
        *mut u32,
    ) -> uefi_raw::Status = core::mem::transmute(real);
    let st = real(size, map, key, desc_size, desc_version);
    GMM_CALLS.fetch_add(1, Ordering::Relaxed);
    if st.is_success() {
        if !key.is_null() {
            let k = *key;
            GMM_LAST_KEY.store(k, Ordering::Relaxed);
            let n = GMM_RING_N.fetch_add(1, Ordering::Relaxed);
            // First-seen keys land in the ring; repeats leave it alone. The
            // ring is 16 slots - a loader with more distinct keys saturates
            // it, so "distinct" is a lower bound. Saturation note: the
            // counter keeps rising even when the ring is full.
            let mut seen = false;
            for slot in GMM_RING.iter() {
                if slot.load(Ordering::Relaxed) == k {
                    seen = true;
                    break;
                }
            }
            if !seen && n < 16 {
                GMM_RING[n].store(k, Ordering::Relaxed);
            } else if !seen {
                // Ring full: keep the "distinct" estimate conservative.
                GMM_RING_N.fetch_sub(1, Ordering::Relaxed);
            }
        }
    } else {
        GMM_ERR.fetch_add(1, Ordering::Relaxed);
    }
    st
}

// ---- fanout checkpoints ----

/// BeforeEBS notify: the fanout's first stage. Boot services are FULLY alive
/// here (the memory map is not terminated yet) - one committed file line is
/// legal, and its allocation cost is the documented possible one-time EBS
/// retry (edk2 fires this group exactly once per boot, so no livelock).
unsafe extern "efiapi" fn before_notify(_event: Event, _ctx: Option<NonNullBare>) {
    let n = NOTIFY_BEFORE.fetch_add(1, Ordering::Relaxed) + 1;
    post(0xC1);
    fb_mark(0, (0, 0, 255)); // row 1: blue
    let mut l = Line::new();
    l.push(b"BEFORE-EBS notify reached (fanout stage 1; services alive; this line's write may cause a one-time EBS retry)\r\n");
    if file_write_allowed(WatchLine::Before, n) {
        commit_line(&l);
    }
    raw_text(l.bytes());
}

/// EBS-group notify: the mid-fanout checkpoint. The memory map is
/// TERMINATED here (edk2: TerminateMemoryMap ran before this fanout) - NO
/// allocation, NO file, NO console, NO logging from this callback. Raw
/// channels only.
unsafe extern "efiapi" fn ebs_notify(_event: Event, _ctx: Option<NonNullBare>) {
    NOTIFY_EBS.fetch_add(1, Ordering::Relaxed);
    post(0xC2);
    fb_mark(1, (255, 255, 0)); // row 2: yellow
    raw_text(b"EBS-NOTIFY reached (fanout mid-point; memory map terminated - nothing but raw channels from here)\r\n");
}

/// `Option<NonNull<c_void>>` spelled out (the EventNotifyFn context type).
type NonNullBare = core::ptr::NonNull<core::ffi::c_void>;

// ---- install ----

/// Recompute the boot-services table header CRC after the pointer surgery
/// (UEFI spec: EFI_TABLE_HEADER.CRC32 over the whole table with the field
/// zeroed). Callers may legitimately verify the table.
///
/// Safety: `bt` must be the live firmware boot-services table.
unsafe fn fix_gbs_crc(bt: *mut uefi_raw::table::boot::BootServices) {
    let hdr = bt.cast::<uefi_raw::table::Header>().as_mut().expect("gBS header");
    let size = hdr.size as usize;
    hdr.crc = 0;
    let bytes = core::slice::from_raw_parts(bt.cast::<u8>(), size);
    hdr.crc = crc32(bytes);
}

/// Marker probe + full install. Called from `main` right before the chain;
/// inert (one marker probe) on every stick that does not opt in.
pub fn install_if_marker() {
    let watch = crate::config::flag(crate::config::FlagArm::EbsWatch);
    let selftest = crate::config::flag(crate::config::FlagArm::EbsSelftest);
    if !watch && !selftest {
        return;
    }
    if INSTALLED.swap(true, Ordering::Relaxed) {
        return;
    }
    uart_init();
    post(0xC0);
    capture_fb();

    // Patch the table.
    let patched = (|| -> bool {
        let Some(st) = uefi::table::system_table_raw() else {
            return false;
        };
        // Safety: boot services are live; the table pointer is the firmware's.
        unsafe {
            let bt = st.as_ref().boot_services;
            if bt.is_null() {
                return false;
            }
            REAL_EBS.store((*bt).exit_boot_services as usize, Ordering::Relaxed);
            REAL_GMM.store((*bt).get_memory_map as usize, Ordering::Relaxed);
            (*bt).exit_boot_services = wrapped_exit_boot_services;
            (*bt).get_memory_map = wrapped_get_memory_map;
            fix_gbs_crc(bt);
        }
        true
    })();

    // Arm the two fanout checkpoints (UEFI 2.0+; older firmware refuses).
    let ev_before = unsafe {
        boot::create_event_ex(
            EventType::NOTIFY_SIGNAL,
            Tpl::CALLBACK,
            Some(before_notify),
            None,
            Some(&BEFORE_EBS_GROUP),
        )
    };
    let ev_ebs = unsafe {
        boot::create_event_ex(
            EventType::NOTIFY_SIGNAL,
            Tpl::CALLBACK,
            Some(ebs_notify),
            None,
            Some(&EBS_GROUP),
        )
    };

    // Fresh committed watch file with an install header.
    FILE_ERROR.store(false, Ordering::Relaxed);
    unsafe {
        *WATCH_FILE.0.get() = open_watch_file();
    }
    let mut l = Line::new();
    l.push(b"EBS-WATCH installed (gBS patched: ExitBootServices + GetMemoryMap wrapped, header CRC recomputed)\r\n");
    commit_line(&l);

    info!(
        "EBS-WATCH: \\tx-ebs-watch.txt{} present - gBS wrapped (ExitBootServices+GetMemoryMap, header CRC recomputed) patch={} checkpoints={}",
        if selftest { " + \\tx-ebs-selftest.txt" } else { "" },
        if patched { "ok" } else { "FAILED" },
        match (ev_before.is_ok(), ev_ebs.is_ok()) {
            (true, true) => "BEFORE+EBS armed",
            (true, false) => "BEFORE armed, EBS group refused (firmware < UEFI 2.0?)",
            (false, true) => "EBS armed, BEFORE group refused",
            (false, false) => "both groups refused (firmware < UEFI 2.0?)",
        }
    );
    info!(
        "EBS-WATCH: readouts - \\ebs-watch.log (committed lines, budgeted) + framebuffer rows (blue=BEFORE yellow=NOTIFY green=RETURN) + POST 0x80 (0xC1/0xC2/0xD0/0xD1/0xE0|n) + 0xE9 + UART 0x3F8; no file/console writes after a successful EBS (table zeroed)"
    );
    if selftest {
        info!("EBS-SELFTEST: \\tx-ebs-selftest.txt present - the loader will call GetMemoryMap+ExitBootServices ITSELF at the chain point, then park (power-cycle afterwards; no OS boot)");
    }
}

/// `\tx-ebs-selftest.txt`: exercise the instrument end to end from the
/// loader itself. Called from `main` at the chain point; when the marker is
/// absent this is a single bool probe. On success the machine PARKS here
/// (boot services are gone; only the raw channels run) - a power cycle ends
/// the session.
pub fn selftest_if_marker() -> bool {
    if !crate::config::flag(crate::config::FlagArm::EbsSelftest) {
        return false;
    }
    info!("EBS-SELFTEST: beginning self-call of the wrapped ExitBootServices (the log/screen show the ENTER/BEFORE/NOTIFY/RETURN sequence live)");
    let Some(st) = uefi::table::system_table_raw() else { return true };
    // Safety: boot services are live on every path below until the EBS call
    // itself succeeds; the parking tail touches nothing but raw channels.
    unsafe {
        let bt = st.as_ref().boot_services;
        if bt.is_null() {
            info!("EBS-SELFTEST: no boot services table - aborting the self-test, chain continues");
            return true;
        }
        let gmm = (*bt).get_memory_map;
        let ebs = (*bt).exit_boot_services;
        // Up to three attempts with a fresh MapKey each - the loader-shaped
        // dance. The FIRST attempt may fail INVALID_PARAMETER by design: the
        // enter line + the BeforeEBS notify line write to the watch file
        // inside the window, and that allocation stales the key (one
        // manufactured retry, budgeted - see file_write_allowed).
        for attempt in 1..=3usize {
            // The descriptor buffer must OUTLIVE the EBS call: a FreePool
            // between GetMemoryMap and ExitBootServices changes the memory
            // map and stales the key. (Caught by this instrument on the
            // first QEMU round - the classic loader footgun, exactly the
            // class the ENTER/RETURN pairs exist to expose.)
            let mut buf = alloc::vec![0u8; 4096];
            let mut size = 4096usize;
            let mut key = None;
            for _ in 0..8 {
                let mut k = 0usize;
                let mut dsz = 0usize;
                let mut dv = 0u32;
                let s = gmm(
                    &mut size,
                    buf.as_mut_ptr().cast::<MemoryDescriptor>(),
                    &mut k,
                    &mut dsz,
                    &mut dv,
                );
                if s.is_success() {
                    key = Some(k);
                    break;
                }
                if s != uefi_raw::Status::BUFFER_TOO_SMALL {
                    info!("EBS-SELFTEST: GetMemoryMap failed (status {:#x}) - aborting the self-test, chain continues", s.0);
                    return true;
                }
                size *= 2;
                buf.resize(size, 0);
            }
            let Some(key) = key else {
                info!("EBS-SELFTEST: GetMemoryMap never succeeded - aborting the self-test, chain continues");
                return true;
            };
            let mut l = Line::new();
            l.push(b"EBS-SELFTEST: calling ExitBootServices (attempt ");
            push_dec(&mut l, attempt);
            l.push(b") now\r\n");
            raw_text(l.bytes());
            let s = ebs(boot::image_handle().as_ptr(), key);
            if s.is_success() {
                // Raw tail only: boot services are terminated (gBS zeroed).
                // Park in a busy loop - uefi::boot::stall IS a boot service
                // and must not be called here.
                raw_text(b"EBS-SELFTEST: RETURN ok - instrument verified end to end (ENTER/BEFORE/NOTIFY/RETURN all visible above); PARKING - power-cycle to end the session\r\n");
                fb_mark(2, (0, 255, 0));
                let mut spin = 0usize;
                loop {
                    // ~1s worth of iterations per POST code step; no services.
                    for _ in 0..30_000_000 {
                        core::hint::spin_loop();
                    }
                    spin += 1;
                    post(0xF0 | (spin & 0x0f) as u8);
                    if spin & 0x0f == 0 {
                        raw_text(b"EBS-SELFTEST: parked (alive)\r\n");
                    }
                }
            }
            info!(
                "EBS-SELFTEST: ExitBootServices attempt {} returned {:#x} (services alive) - see the EBS-RETURN line in \\ebs-watch.log",
                attempt, s.0
            );
            drop(buf); // failed attempt: releasing the buffer is safe now
        }
        info!("EBS-SELFTEST: three attempts failed with services alive - a persistent INVALID_PARAMETER loop is the MapKey-dirtying signature; chain continues");
    }
    true
}
