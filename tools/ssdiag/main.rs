//! ssdiag v0.2 - in-OS NVIDIA register diagnosis + separate-lock re-apply.
//!
//! Reads the registers the EFI unlock stage touches, from inside the running
//! OS, and - where the EFI chain left the FEAT PLM gate open (Ampere/Ada AON
//! behavior, issue #6) - re-applies the SS override values without a reboot.
//!
//! Driver lifecycle (strict, cmp50hx "remove-when-done" model): the
//! Intel-signed pmxdrv.sys is EMBEDDED, extracted to %TEMP% on demand,
//! loaded under its own service name (SSDIAG_PMXDRV), and STOPPED + DELETED
//! on exit - nothing stays in the user's service list. If a PMXDRV-class
//! driver is already running (e.g. deployed by nvoc), it is borrowed and
//! left untouched. Requires elevation for the deploy path.
//!
//! Usage:
//!   ssdiag --bar0 0x50000000             # debug dump + FEAT verdict
//!   ssdiag --bar0 0x50000000 --reapply   # + SS0=0x88888888 SS1=0x8 when open
//!
//! Read-back of written values is the only success criterion (const. 1.1).

#![windows_subsystem = "console"]

use sha2::{Digest, Sha256};
use std::ptr;

// ---------------------------------------------------------------- embedded driver

/// In-tree copy of nvoc `core/src/kmd/pmxdrv.sys` (Intel 2019 Authenticode
/// build). Pinned hash must equal nvoc's `PMXDRV_SYS_SHA256`; startup
/// verifies and fails closed on any mismatch.
const PMXDRV_SYS: &[u8] = include_bytes!("pmxdrv.sys");
const PMXDRV_SYS_SHA256: &str =
    "b1a8ee1222eea5f199028d90b9b77c2acf46d6d84a9e125403b2888c6f681c72";

const SERVICE_NAME: &str = "SSDIAG_PMXDRV";
const TEMP_SYS: &str = "ssdiag-pmxdrv.sys";

// ---------------------------------------------------------------- win32

#[allow(non_snake_case)]
mod win {
    use std::ffi::c_void;
    pub type Handle = *mut c_void;
    #[link(name = "advapi32")]
    extern "system" {
        pub fn OpenSCManagerW(
            machine: *const u16,
            db: *const u16,
            access: u32,
        ) -> Handle;
        pub fn OpenServiceW(h: Handle, name: *const u16, access: u32) -> Handle;
        pub fn CreateServiceW(
            h: Handle,
            name: *const u16,
            display: *const u16,
            access: u32,
            svc_type: u32,
            start_type: u32,
            err_type: u32,
            path: *const u16,
            order: *const u16,
            tag: *const u32,
            deps: *const u16,
            account: *const u16,
            password: *const u16,
        ) -> Handle;
        pub fn StartServiceW(h: Handle, argc: u32, argv: *const *const u16) -> i32;
        pub fn ControlService(
            h: Handle,
            code: u32,
            status: *mut ServiceStatus,
        ) -> i32;
        pub fn QueryServiceStatus(h: Handle, status: *mut ServiceStatus) -> i32;
        pub fn DeleteService(h: Handle) -> i32;
        pub fn CloseServiceHandle(h: Handle) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        pub fn CreateFileW(
            name: *const u16,
            access: u32,
            share: u32,
            sa: *const c_void,
            disposition: u32,
            flags: u32,
            template: *const c_void,
        ) -> Handle;
        pub fn DeviceIoControl(
            h: Handle,
            ioctl: u32,
            inbuf: *const c_void,
            inlen: u32,
            outbuf: *mut c_void,
            outlen: u32,
            returned: *mut u32,
            overlapped: *const c_void,
        ) -> i32;
        pub fn CloseHandle(h: Handle) -> i32;
        pub fn GetLastError() -> u32;
        pub fn LoadLibraryW(name: *const u16) -> Handle;
        pub fn GetProcAddress(h: Handle, name: *const u8) -> *const c_void;
        pub fn GetStdHandle(nStdHandle: u32) -> Handle;
        pub fn GetConsoleMode(hConsole: Handle, mode: *mut u32) -> i32;
        pub fn SetConsoleMode(hConsole: Handle, mode: u32) -> i32;
        pub fn SetConsoleCtrlHandler(
            handler: Option<unsafe extern "system" fn(u32) -> i32>,
            add: i32,
        ) -> i32;
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct ServiceStatus {
        pub service_type: u32,
        pub current_state: u32,
        pub controls_accepted: u32,
        pub win32_exit: u32,
        pub specific_exit: u32,
        pub checkpoint: u32,
        pub wait_hint: u32,
    }
}

/// One HKEY_LOCAL_MACHINE DWORD, read for the platform pre-flight. Kept as a
/// separate tiny module so the main `win` block stays kernel32-only.
mod reg {
    #[link(name = "advapi32")]
    extern "system" {
        pub fn RegGetValueW(
            hkey: isize,
            subkey: *const u16,
            value: *const u16,
            flags: u32,
            typ: *mut u32,
            data: *mut core::ffi::c_void,
            cbdata: *mut u32,
        ) -> i32;
    }
}

const HKEY_LOCAL_MACHINE: isize = 0x8000_0002;
const RRF_RT_REG_DWORD: u32 = 0x0000_0010;

/// None = the value is absent (or unreadable), which the caller must print as
/// "unknown" rather than as a pass.
fn hklm_dword(subkey: &str, value: &str) -> Option<u32> {
    let sub = wide(subkey);
    let val = wide(value);
    let mut data: u32 = 0;
    let mut cb: u32 = 4;
    let rc = unsafe {
        reg::RegGetValueW(
            HKEY_LOCAL_MACHINE,
            sub.as_ptr(),
            val.as_ptr(),
            RRF_RT_REG_DWORD,
            core::ptr::null_mut(),
            (&mut data as *mut u32).cast(),
            &mut cb,
        )
    };
    if rc == 0 && cb == 4 {
        Some(data)
    } else {
        None
    }
}

const CI_CONFIG: &str = "SYSTEM\\CurrentControlSet\\Control\\CI\\Config";
const HVCI_SCENARIO: &str =
    "SYSTEM\\CurrentControlSet\\Control\\DeviceGuard\\Scenarios\\HypervisorEnforcedCodeIntegrity";
/// `UEFISecureBootEnabled`: 1 = Secure Boot on, 0 = off; readable unelevated.
const SECUREBOOT_STATE: &str = r"SYSTEM\CurrentControlSet\Control\SecureBoot\State";

fn gate_word(v: Option<u32>) -> &'static str {
    match v {
        Some(0) => "OFF",
        Some(_) => "ON",
        None => "unknown (value absent - the Windows default applies)",
    }
}

/// The platform gates that decide whether the pinned pmxdrv can load at all,
/// plus what installing it looks like from the outside. All values are
/// readable without the driver, so this runs before anything is deployed -
/// and `--dry-run` can print it on a machine where the real run would only
/// fail with a raw win32 code.
///
/// Gate semantics (verified 2026-11, issue #40):
/// - Secure Boot ON is the gate that actually refuses THIS driver: the pinned
///   pmxdrv.sys is Intel/COMODO cross-signed (leaf cert issued 2018-10-24, no
///   Microsoft root), so it qualifies for none of the DSE cross-signing
///   carve-outs (only machines upgraded from Windows <= 1607, Secure Boot
///   off, or end-entity certs issued before 2015-07-29 are exempt).
/// - The Microsoft vulnerable-driver blocklist does NOT list this hash (file
///   SHA256, authentihashes and imphash all absent from both the OS-delivered
///   and the downloadable policies, with in-list control hashes verified), so
///   blocklist/HVCI being ON refuses nothing until Microsoft adds the hash.
fn preflight() {
    let blocklist = hklm_dword(CI_CONFIG, "VulnerableDriverBlocklistEnable");
    let hvci = hklm_dword(HVCI_SCENARIO, "Enabled");
    let secure_boot = hklm_dword(SECUREBOOT_STATE, "UEFISecureBootEnabled");
    println!("PLATFORM: vulnerable-driver blocklist = {}", gate_word(blocklist));
    println!("PLATFORM: core isolation (HVCI)       = {}", gate_word(hvci));
    println!("PLATFORM: Secure Boot                 = {}", gate_word(secure_boot));
    if secure_boot == Some(1) {
        println!(
            "{}",
            red("PLATFORM: Secure Boot is ON - the pinned pmxdrv.sys cannot load here.")
        );
        println!("  It is cross-signed (Intel/COMODO, no Microsoft root, signing cert issued");
        println!("  after the 2015-07-29 cutoff), so it fits none of the DSE carve-outs and");
        println!("  StartService is expected to be refused - unless this OS was upgraded from");
        println!("  Windows <= 1607. The driver-free path is the EFI read-only sampler.");
    }
    if blocklist == Some(1) || hvci == Some(1) {
        println!(
            "{}",
            yellow("PLATFORM: blocklist/HVCI gate is ON, but that alone refuses nothing here.")
        );
        println!("  The pinned pmxdrv hash is NOT on the current Microsoft blocklist (checked");
        println!("  2026-11 against the OS-delivered and downloadable policies); it loads or");
        println!("  fails on the signature gate above, not on this one.");
    }
    println!("NOTE: a signed kernel driver is installed for the duration of the probe; SCM logs");
    println!("      a driver install (event 7045) and managed/EDR machines may alert on it.");
}

const GENERIC_RW: u32 = 0xC000_0000;
const OPEN_EXISTING: u32 = 3;
const SC_MANAGER_CREATE_SERVICE: u32 = 0x0002;
const SERVICE_ALL_ACCESS: u32 = 0x000F_01FF;
const SERVICE_KERNEL_DRIVER: u32 = 1;
const SERVICE_DEMAND_START: u32 = 3;
const SERVICE_ERROR_NORMAL: u32 = 0;
const SERVICE_CONTROL_STOP: u32 = 1;
const SERVICE_STOPPED: u32 = 1;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn last_err() -> u32 {
    unsafe { win::GetLastError() }
}

// ---------------------------------------------------------------- color
// Light ANSI colorizer: 0x prefix yellow + digits cyan, UNLOCKED green,
// NOT_UNLOCKED red, badf buckets red, OPEN green. Enabled only on a real
// console after flipping ENABLE_VIRTUAL_TERMINAL_PROCESSING (Win10+); a
// pipe/redirect or NO_COLOR degrades to plain text automatically.

static COLOR_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn color_init() {
    if std::env::var_os("NO_COLOR").is_some() {
        return;
    }
    // SSDIAG_COLOR=force pins colors on (for testing through pipes).
    let forced = std::env::var("SSDIAG_COLOR").is_ok_and(|v| v == "force");
    if !forced {
        const STD_OUTPUT_HANDLE: u32 = u32::MAX - 10; // (DWORD)-11
        const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
        unsafe {
            let h = win::GetStdHandle(STD_OUTPUT_HANDLE);
            let mut mode = 0u32;
            // GetConsoleMode fails on a redirected handle - no console, no color
            if win::GetConsoleMode(h, &mut mode) == 0 {
                return;
            }
            win::SetConsoleMode(h, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }
    COLOR_ON.store(true, std::sync::atomic::Ordering::Relaxed);
}

fn paint(code: &str, s: &str) -> String {
    if COLOR_ON.load(std::sync::atomic::Ordering::Relaxed) {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

/// 0x-prefixed 8-digit u32: prefix yellow, digits cyan.
fn hx32(v: u32) -> String {
    if COLOR_ON.load(std::sync::atomic::Ordering::Relaxed) {
        format!("\x1b[33m0x\x1b[36m{v:08x}\x1b[0m")
    } else {
        format!("{v:#010x}")
    }
}

/// 0x-prefixed u64 address: prefix bright cyan, digits bright yellow.
/// Width = minimal even nibble count (no leading-zero padding: a 32-bit
/// BAR reads 0xc0000000, a >4GiB BAR1 reads 0x6200000000).
fn hxaddr(v: u64) -> String {
    let digits = format!("{v:x}");
    let pad = digits.len() + (digits.len() & 1);
    if COLOR_ON.load(std::sync::atomic::Ordering::Relaxed) {
        format!("\x1b[96m0x\x1b[93m{digits:0>pad$}\x1b[0m")
    } else {
        format!("{digits:0>pad$}")
    }
}

fn green(s: &str) -> String {
    paint("92", s)
}

fn red(s: &str) -> String {
    paint("91", s)
}

fn yellow(s: &str) -> String {
    paint("93", s)
}

// ---------------------------------------------------------------- NVAPI GC6 wake
// Hybrid-laptop idle parks the dGPU in GC6/GCOFF (RTD3): BARs stop decoding
// and every read degenerates. GC6 exit is a DRIVER-mediated firmware sequence
// (link wake, VRAM retrain, context restore) - a raw PMCSR flip cannot do it.
// The lightweight path is the driver's own control: NvAPI_GPU_GC6Control
// (NDA 0xD387D414; nvoc nvapi-rs reverse - cmd 0=query/1=sleep/2=wake, state
// GC6_IDLE=2 / D0_ACTIVE=3, struct v1 12 bytes, version word 0x1000C).
mod nvapi_wake {
    use super::win;
    use std::ffi::c_void;

    const NVAPI_OK: i32 = 0;
    const GC6_VERSION_V1: u32 = 0x0001_000C; // ver 1, 12-byte struct
    const GC6_CMD_QUERY: u32 = 0;
    const GC6_CMD_WAKE: u32 = 2;
    const GC6_STATE_D0_ACTIVE: u32 = 3;

    type QueryInterfaceFn = unsafe extern "system" fn(u32) -> *mut c_void;
    type InitializeFn = unsafe extern "system" fn() -> i32;
    type EnumFn = unsafe extern "system" fn(*mut u32, *mut u32) -> i32;
    type Gc6Fn = unsafe extern "system" fn(u32, *mut c_void) -> i32;

    /// Wake every physical GPU out of GC6. Returns a human summary; errors
    /// mean nvapi.dll is unusable (driver not installed) - callers degrade
    /// gracefully.
    pub fn wake_all() -> Result<String, String> {
        unsafe {
            let name: Vec<u16> = "nvapi.dll".encode_utf16().chain([0]).collect();
            let lib = win::LoadLibraryW(name.as_ptr());
            if lib.is_null() {
                return Err("nvapi.dll not loadable - NVIDIA driver installed?".into());
            }
            let qip = win::GetProcAddress(lib, b"nvapi_QueryInterface\0".as_ptr());
            if qip.is_null() {
                return Err("nvapi_QueryInterface missing".into());
            }
            let qi: QueryInterfaceFn = std::mem::transmute(qip);
            let init: InitializeFn = std::mem::transmute(qi(0x0150_E828));
            let st = init();
            if st != NVAPI_OK {
                return Err(format!("NvAPI_Initialize failed ({st:#x})"));
            }
            let enumf: EnumFn = std::mem::transmute(qi(0xE5AC_921F));
            let mut handles = [0u32; 64];
            let mut count = 0u32;
            if enumf(handles.as_mut_ptr(), &mut count) != NVAPI_OK {
                return Err("NvAPI_EnumPhysicalGPUs failed".into());
            }
            let gc6: Gc6Fn = std::mem::transmute(qi(0xD387_D414));

            let mut report = String::new();
            for &h in &handles[..count.min(64) as usize] {
                let mut d = [0u32; 3];
                d[0] = GC6_VERSION_V1;
                d[1] = GC6_CMD_QUERY;
                if gc6(h, d.as_mut_ptr().cast()) != NVAPI_OK {
                    continue; // desktop GPU / no RTD3 support: nothing to wake
                }
                match d[2] {
                    GC6_STATE_D0_ACTIVE => report.push_str(&format!("gpu{h}: D0; ")),
                    2 => {
                        // parked in GC6 - force-wake and poll the exit
                        d[1] = GC6_CMD_WAKE;
                        if gc6(h, d.as_mut_ptr().cast()) != NVAPI_OK {
                            report.push_str(&format!("gpu{h}: wake-call failed; "));
                            continue;
                        }
                        let mut woken = false;
                        for _ in 0..10 {
                            std::thread::sleep(std::time::Duration::from_millis(300));
                            let mut q = [0u32; 3];
                            q[0] = GC6_VERSION_V1;
                            q[1] = GC6_CMD_QUERY;
                            if gc6(h, q.as_mut_ptr().cast()) == NVAPI_OK
                                && q[2] == GC6_STATE_D0_ACTIVE
                            {
                                woken = true;
                                break;
                            }
                        }
                        report.push_str(&format!(
                            "gpu{h}: GC6 -> {}; ",
                            if woken {
                                super::green("D0")
                            } else {
                                super::red("wake timeout")
                            }
                        ));
                    }
                    s => report.push_str(&format!("gpu{h}: state {s}; ")),
                }
            }
            Ok(report)
        }
    }
}

// ---------------------------------------------------------------- pmxdrv transport
// Byte layout copied verbatim from nvoc core/src/kmd/pmxdrv.rs (reverse-
// engineered + live-verified there; the code table was mis-derived once by
// hand - do NOT re-derive).

const DEVICE_PATH: &str = r"\\.\PMXDRV";
const IOCTL_MAP_PHYS: u32 = 0x222A_B8;
const IOCTL_UNMAP_PHYS: u32 = 0x222A_BC;
const INPUT_LEN: usize = 16; // [u64 user-mode request ptr][u32 aux][u32 pad]
const REQ_LEN: u32 = 24; // request tag = struct length
const MAP_FAIL_MARK: u64 = 0xBEEF;

struct PmxDrv {
    handle: win::Handle,
}

impl PmxDrv {
    fn try_open() -> Option<Self> {
        let wide = wide(DEVICE_PATH);
        let h = unsafe {
            win::CreateFileW(
                wide.as_ptr(),
                GENERIC_RW,
                3,
                ptr::null(),
                OPEN_EXISTING,
                0,
                ptr::null(),
            )
        };
        if h as isize == -1 {
            None
        } else {
            Some(Self { handle: h })
        }
    }

    fn forward(&self, ioctl: u32, request: &mut [u8]) -> Result<(), u32> {
        let mut input = [0u8; INPUT_LEN];
        input[..8].copy_from_slice(&(request.as_ptr() as u64).to_le_bytes());
        let mut returned: u32 = 0;
        let ok = unsafe {
            win::DeviceIoControl(
                self.handle,
                ioctl,
                input.as_ptr().cast(),
                input.len() as u32,
                ptr::null_mut(),
                0,
                &mut returned,
                ptr::null(),
            )
        };
        if ok == 0 {
            return Err(last_err());
        }
        Ok(())
    }

    fn map_physical(&self, pa: u64, pages: u32) -> Result<u64, String> {
        let mut request = [0u8; 24];
        request[0..4].copy_from_slice(&REQ_LEN.to_le_bytes());
        request[4..12].copy_from_slice(&pa.to_le_bytes());
        request[12..16].copy_from_slice(&pages.to_le_bytes());
        self.forward(IOCTL_MAP_PHYS, &mut request)
            .map_err(|e| format!("MAP_PHYS ioctl failed (win32 error {e})"))?;
        let va = u64::from_le_bytes(request[16..24].try_into().unwrap());
        if va == 0 || va == MAP_FAIL_MARK {
            return Err("map rejected by driver".into());
        }
        Ok(va)
    }

    fn unmap_physical(&self, va: u64) {
        let mut request = [0u8; 24];
        request[0..4].copy_from_slice(&REQ_LEN.to_le_bytes());
        request[16..24].copy_from_slice(&va.to_le_bytes());
        let _ = self.forward(IOCTL_UNMAP_PHYS, &mut request);
    }
}

impl Drop for PmxDrv {
    fn drop(&mut self) {
        unsafe {
            win::CloseHandle(self.handle);
        }
    }
}

// ---------------------------------------------------------------- driver lifecycle

fn service_stop_and_wait(svc: win::Handle) -> Result<(), String> {
    let mut st = win::ServiceStatus::default();
    let sent = unsafe { win::ControlService(svc, SERVICE_CONTROL_STOP, &mut st) };
    if sent == 0 && st.current_state != SERVICE_STOPPED {
        return Err(format!("stop refused (win32 {}, state {})", last_err(), st.current_state));
    }
    for _ in 0..25 {
        let mut st = win::ServiceStatus::default();
        if unsafe { win::QueryServiceStatus(svc, &mut st) } != 0 && st.current_state == SERVICE_STOPPED {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    Err("stop still pending after 5s".into())
}

/// Extract + service-create + start. Service name is OURS (SSDIAG_PMXDRV) so
/// cleanup never touches a driver another tool deployed.
fn deploy_driver() -> Result<(), String> {
    // payload integrity, fail closed
    let mut h = Sha256::new();
    h.update(PMXDRV_SYS);
    let got = format!("{:x}", h.finalize());
    if got != PMXDRV_SYS_SHA256 {
        return Err(format!("embedded pmxdrv.sys hash mismatch ({got}) - refusing to load"));
    }

    let sys_path = std::env::temp_dir().join(TEMP_SYS);
    std::fs::write(&sys_path, PMXDRV_SYS)
        .map_err(|e| format!("extract to {} failed: {e}", sys_path.display()))?;
    let readback = std::fs::read(&sys_path).map_err(|e| format!("re-read failed: {e}"))?;
    if readback != PMXDRV_SYS {
        return Err("extract re-read mismatch (partial write?)".into());
    }

    // ImagePath: \??\ + plain absolute path (canonicalize's \\?\ prefix stacks
    // into an illegal name - nvoc lesson).
    let abs = if sys_path.is_absolute() {
        sys_path
    } else {
        std::env::current_dir().unwrap().join(&sys_path)
    };
    let path_w = wide(format!(r"\??\{}", abs.to_string_lossy()).as_str());
    let name_w = wide(SERVICE_NAME);

    unsafe {
        let mgr = win::OpenSCManagerW(ptr::null(), ptr::null(), SC_MANAGER_CREATE_SERVICE);
        if mgr.is_null() {
            return Err(format!(
                "OpenSCManager failed (admin required): win32 {}",
                last_err()
            ));
        }
        // leftover of our own previous run: stop, delete, wait
        let existing = win::OpenServiceW(mgr, name_w.as_ptr(), SERVICE_ALL_ACCESS);
        if !existing.is_null() {
            let _ = service_stop_and_wait(existing);
            win::DeleteService(existing);
            win::CloseServiceHandle(existing);
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        let svc = win::CreateServiceW(
            mgr,
            name_w.as_ptr(),
            name_w.as_ptr(),
            SERVICE_ALL_ACCESS,
            SERVICE_KERNEL_DRIVER,
            SERVICE_DEMAND_START,
            SERVICE_ERROR_NORMAL,
            path_w.as_ptr(),
            ptr::null(),
            ptr::null_mut(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        );
        if svc.is_null() {
            let err = last_err();
            win::CloseServiceHandle(mgr);
            return Err(format!("CreateService failed: win32 {err}"));
        }
        if win::StartServiceW(svc, 0, ptr::null()) == 0 {
            let err = last_err();
            win::DeleteService(svc);
            win::CloseServiceHandle(svc);
            win::CloseServiceHandle(mgr);
            return Err(format!("StartService failed: win32 {err}"));
        }
        win::CloseServiceHandle(svc);
        win::CloseServiceHandle(mgr);
    }
    Ok(())
}

/// Teardown on the paths that never reach `cleanup_driver()` in `main`.
///
/// The release profile is `panic = "abort"`, so a panic runs no destructors and
/// `?`/`unwrap` sites in the probe would leave the kernel service installed and
/// started, with the extracted `pmxdrv.sys` still in %TEMP%. Hooks are the only
/// place that code runs on those paths: the panic hook fires before the abort,
/// and a console control handler fires before the console teardown kills us.
/// True once this process installed or borrowed the driver. The teardown line in
/// main() keys off it so a --dry-run does not claim to have removed something it
/// never loaded.
static DRIVER_TOUCHED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn install_crash_cleanup() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        cleanup_driver();
        default_hook(info);
    }));
    // Safety: the handler only calls cleanup_driver(), which re-opens the SCM
    // itself and is idempotent; it runs on the process's control thread. It can
    // race an in-flight ensure_driver() (Ctrl-C mid-deploy runs cleanup
    // concurrently with StartService/retry-open); the worst case is the old
    // behavior - a stale service - so the race is accepted, not synchronized.
    unsafe {
        win::SetConsoleCtrlHandler(Some(console_ctrl), 1);
    }
}

/// CTRL_C_EVENT / CTRL_BREAK_EVENT / CTRL_CLOSE_EVENT / logoff / shutdown.
/// Returns 0 so the default handler still terminates the process afterwards.
unsafe extern "system" fn console_ctrl(_event: u32) -> i32 {
    cleanup_driver();
    0
}

/// Own-the-service cleanup: stop + delete + remove the temp .sys (retry a
/// short while - the kernel releases the image mapping with a small delay).
fn cleanup_driver() {
    unsafe {
        let mgr = win::OpenSCManagerW(ptr::null(), ptr::null(), SC_MANAGER_CREATE_SERVICE);
        if mgr.is_null() {
            return;
        }
        let name_w = wide(SERVICE_NAME);
        let svc = win::OpenServiceW(mgr, name_w.as_ptr(), SERVICE_ALL_ACCESS);
        if !svc.is_null() {
            let _ = service_stop_and_wait(svc);
            win::DeleteService(svc);
            win::CloseServiceHandle(svc);
        }
        win::CloseServiceHandle(mgr);
    }
    let sys_path = std::env::temp_dir().join(TEMP_SYS);
    for _ in 0..10 {
        if std::fs::remove_file(&sys_path).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

fn ensure_driver() -> Result<PmxDrv, String> {
    if let Some(d) = PmxDrv::try_open() {
        println!("PMXDRV device already running - borrowing it (no service changes)");
        DRIVER_TOUCHED.store(true, std::sync::atomic::Ordering::Relaxed);
        return Ok(d);
    }
    deploy_driver()?;
    DRIVER_TOUCHED.store(true, std::sync::atomic::Ordering::Relaxed);
    // the kernel can take a beat between StartService and the device symlink
    for _ in 0..10 {
        if let Some(d) = PmxDrv::try_open() {
            return Ok(d);
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    PmxDrv::try_open().ok_or_else(|| "service started but \\\\.\\PMXDRV never appeared".into())
}


// ---------------------------------------------------------------- BAR0 discovery

/// One NVIDIA PCI function and its MMIO ranges this boot.
struct Gpu {
    /// Win32_PnPEntity DeviceID (identifies the function; safe for --select).
    id: String,
    /// sorted (start, end) memory ranges
    ranges: Vec<(u64, u64)>,
}

impl Gpu {
    /// BAR0 candidates, SMALLEST-SIZE first (BAR0 is 16 MiB << idx per
    /// straps; a ReBAR BAR1 can start at a lower address and must not win -
    /// 2026-10-28: address-min picked the 256 MiB BAR1 aperture). The
    /// caller walks the list and lets the PMC sanity read arbitrate.
    fn bar0_candidates(&self) -> Vec<u64> {
        let mut c: Vec<(u64, u64)> = self
            .ranges
            .iter()
            .copied()
            .filter(|(a, b)| b - a + 1 >= 0x100_0000)
            .collect();
        c.sort_by_key(|(a, b)| b - a + 1);
        c.into_iter().map(|(a, _)| a).collect()
    }
}

/// Discover the NVIDIA PCI functions' MMIO ranges from the OS itself.
/// Physical assignments change every boot (UEFI assigns one layout, Windows
/// re-assigns another) - a base remembered from any previous session is a
/// hazard, not a shortcut (2026-10-28: a stale UEFI-era address mapped a
/// foreign MMIO window and the first read hard-hung the machine).
fn discover_gpus() -> Result<Vec<Gpu>, String> {
    // Dependent/Antecedent are embedded CimInstance objects - property access,
    // not string regex (the Dependent.ToString path does not round-trip).
    // Display-class filter keeps companion functions (audio/USB-C) out.
    let ps = "$disp = Get-PnpDevice -Class Display | Where-Object InstanceId -match 'VEN_10DE' | Select-Object -ExpandProperty InstanceId
              $r = Get-CimInstance Win32_PnPAllocatedResource | Where-Object { $_.Dependent -match 'VEN_10DE' -and $_.Antecedent -match 'Memory' }
              $r | ForEach-Object { $id = $_.Dependent.DeviceID; if ($disp -ccontains $id) { $m = Get-CimInstance $_.Antecedent; \"{0}|{1}|{2}\" -f $id, $m.StartingAddress, $m.EndingAddress } }";
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", ps])
        .output()
        .map_err(|e| format!("powershell spawn failed: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "powershell query failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let mut gpus: Vec<Gpu> = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let mut it = line.trim().split('|');
        let (Some(id), Some(a), Some(b)) = (it.next(), it.next(), it.next()) else {
            continue;
        };
        let (Ok(a), Ok(b)) = (a.trim().parse::<u64>(), b.trim().parse::<u64>()) else {
            continue;
        };
        if id.is_empty() || a == 0 || b < a {
            continue;
        }
        match gpus.iter_mut().find(|g| g.id == id) {
            Some(g) => g.ranges.push((a, b)),
            None => gpus.push(Gpu { id: id.to_string(), ranges: vec![(a, b)] }),
        }
    }
    for g in &mut gpus {
        g.ranges.sort();
    }
    gpus.retain(|g| !g.ranges.is_empty());
    Ok(gpus)
}

// ---------------------------------------------------------------- register dump

const SS0_UNLOCK: u32 = 0x8888_8888;
const SS1_UNLOCK: u32 = 0x8;

// READOUT at 0x409668 per nlzy turing-unlock (TU_FEAT_READOUT).
const FEAT_TURING: [(&str, u32); 4] = [
    ("PLM", 0x0040_9650),
    ("SS0", 0x0040_9664),
    ("READOUT", 0x0040_9668),
    ("SS1", 0x0040_966C),
];
const FEAT_AMPADA: [(&str, u32); 4] = [
    ("PLM", 0x0082_3804),
    ("SS0", 0x0082_381C),
    ("SS1", 0x0082_3820),
    ("READOUT_1", 0x0082_3818),
];
/// Readable-at-PL0 core/debug block. Data-driven on purpose: add rows here
/// as the debug surface grows. VGA/MMU_PLM/FUSE_VER_DECODE from the loader
/// M1 sample table; the rest cross-checked against the reference unlockers.
const CORE_ROWS: [(&str, u32); 20] = [
    ("PMC_BOOT_0", 0x0000_0000),
    ("PSTRAPS_SET0", 0x0010_1000),
    ("PSTRAPS_SET1", 0x0010_100C),
    ("FB_SIZE_MIB", 0x0011_83A4),
    ("GFW_PLM", 0x0011_8128),
    ("GFW_SCRATCH", 0x0011_8234),
    ("BSI_SCRATCH15", 0x0011_80FC),
    ("VGA_WORKSPACE", 0x0062_5F04),
    ("WPR2_LO", 0x001F_A824),
    ("WPR2_HI", 0x001F_A828),
    ("MMU_PLM", 0x001F_A7C8),
    ("MMU_LOCK_LO", 0x001F_A82C),
    ("MMU_LOCK_HI", 0x001F_A830),
    ("GSP_CPUCTL", 0x0011_0100),
    ("GSP_EXCI", 0x0011_00D0),
    ("SEC2_CPUCTL", 0x0084_0100),
    ("SEC2_MBOX0", 0x0084_0040),
    ("SEC2_MBOX1", 0x0084_0044),
    ("SEC2_EXCI", 0x0084_00D0),
    ("FUSE_VER_DECODE", 0x0082_41C0),
];
/// Privileged block - usually badf at PL0, still worth the record.
const PRIV_ROWS: [(&str, u32); 2] =
    [("FUSE_OPT_FEATURE_OVR", 0x0082_03F0), ("FUSE_VER", 0x0082_4140)];
/// Ada four-write chain targets (WPR_CFG -> FBPA -> WPR -> FEAT).
const ADA_ROWS: [(&str, u32); 3] = [
    ("WPR_PLM", 0x001F_A7C4),
    ("WPR_CFG_PLM", 0x001F_A7CC),
    ("FBPA_PLM", 0x009A_0148),
];
/// AD10x 64G-BAR1 line (CYA/CFG must go through SEC2; host writes dropped).
const AD1X_ROWS: [(&str, u32); 4] = [
    ("AD1X_CYA", 0x0008_8724),
    ("AD1X_CFG", 0x0008_8DCC),
    ("AD1X_XVE_PLM", 0x0008_8FF4),
    ("AD1X_BAR1_CAP_MIRROR", 0x0008_8BBC),
];
/// cmp170hx 26-register PLM survey (union across references; GA100-era
/// addresses - on other dies expect badf or benign values, that IS data).
const PLM_SURVEY: [u32; 26] = [
    0x0082_00D0, 0x0082_00D4, 0x0082_00D8, 0x0082_00DC, 0x0082_00E0, 0x0082_00E4,
    0x0082_00E8, 0x0082_00EC, 0x0082_00F0, 0x0082_00F4, 0x0082_00FC,
    0x0082_3800, 0x0082_3804, 0x0082_3B00,
    0x009A_0008, 0x009A_000C, 0x009A_0148, 0x009A_014C, 0x009A_0168, 0x009A_03F0,
    0x009A_0554, 0x009A_0BFC,
    0x0010_0B10, 0x0010_0B38, 0x0010_0B84, 0x0010_0B9C,
];
/// Ampere/Ada FEAT sweep (loader M1 cluster) minus rows FEAT_AMPADA has.
const AMPADA_SWEEP: [u32; 9] = [
    0x0082_3800, 0x0082_3808, 0x0082_380C, 0x0082_3810, 0x0082_3814, 0x0082_3824,
    0x0082_3828, 0x0082_382C, 0x0082_3B00,
];

fn badf_bucket(v: u32) -> Option<&'static str> {
    if v >> 16 != 0xBADF {
        return None;
    }
    Some(match v {
        0xBADF_5040 => "PRIV_VIOLATION",
        0xBADF_1100 => "UNDECODED",
        _ if (v & 0xFF00) == 0x2000 => "FLOORSWEPT",
        _ => "BADF",
    })
}

/// Die id = PMC_BOOT_0 bits [31:20]; nvchip series per open-gpu-kernel-modules
/// nv_arch.h implementation numbers (GP104=NV124, GA102F=NV17F, AD107=NV197 -
/// the latter two confirmed against 1f07/AD106/AD107 field reads).
/// Die id = PMC_BOOT_0 bits [31:20]; nvchip series per open-gpu-kernel-modules
/// nv_arch.h GPU_ARCHITECTURE table (MAXWELL=0x110, MAXWELL2=0x120, PASCAL=0x130,
/// VOLTA=0x140, VOLTA2=0x150, TURING=0x160, AMPERE=0x170, HOPPER=0x180,
/// ADA=0x190, GB1XX=0x1A0, GB2XX=0x1B0, RUBIN=0x1C0) + implementation numbers
/// (GP104=4, GA102F=F, AD107=7). Field anchors: TU106 0x166, AD106 0x196,
/// AD107 0x197, GA100 0x170, GA106 0xb76 (GA10x quirk).
fn family_of(boot0: u32) -> Option<(&'static str, &'static [(&'static str, u32)])> {
    let die = (boot0 >> 20) & 0xFFF;
    if (0x110..=0x11F).contains(&die) {
        // GM200=0x110 GM204=0x114 GM206=0x116 GM107=0x117 GM108=0x118
        Some(("maxwell", &FEAT_NONE))
    } else if (0x120..=0x12F).contains(&die) {
        Some(("maxwell2", &FEAT_NONE))
    } else if (0x130..=0x13F).contains(&die) {
        // GP100=0x130 GP102=0x132 GP104=0x134 GP106=0x136 GP107=0x137 GP108=0x138
        Some(("pascal", &FEAT_NONE))
    } else if (0x140..=0x14F).contains(&die) {
        // GV100=0x140 GV11B=0x14B
        Some(("volta", &FEAT_NONE))
    } else if (0x150..=0x15F).contains(&die) {
        Some(("volta2", &FEAT_NONE))
    } else if (0x160..=0x16F).contains(&die) {
        Some(("turing", &FEAT_TURING))
    } else if (0x170..=0x17F).contains(&die) {
        // GA100 reads 0x170000a1 plainly (cmp170hx register-reference)
        Some(("ampere", &FEAT_AMPADA))
    } else if (0x180..=0x18F).contains(&die) {
        // GH100=0x180
        Some(("hopper", &FEAT_NONE))
    } else if (0x190..=0x19F).contains(&die) {
        // Ada is NV19x (AD106 field read 0x196000a1) - NOT 0x22x (device ids)
        Some(("ada", &FEAT_AMPADA))
    } else if (0x1A0..=0x1AF).contains(&die) {
        // GB100=0x1A0 GB102=0x1A2
        Some(("blackwell-gb1", &FEAT_NONE))
    } else if (0x1B0..=0x1BF).contains(&die) {
        // GB202-class (50-series)
        Some(("blackwell-gb2", &FEAT_NONE))
    } else if (0x1C0..=0x1CF).contains(&die) {
        Some(("rubin", &FEAT_NONE))
    } else if (0xB70..=0xB7F).contains(&die) {
        // GA10x BOOT_0 quirk: 0xb7xx00a1 (GA102=0xb74, GA106=0xb76 field-
        // confirmed 2026-10-28; the "0xb74000a1" note in cmp170hx
        // register-reference is exactly this). GA100 does NOT do this.
        Some(("ampere", &FEAT_AMPADA))
    } else {
        None
    }
}

/// Families whose FEAT-cluster addresses we have not mapped: diagnosis
/// proceeds, the verdict degrades to a family note.
const FEAT_NONE: [(&str, u32); 0] = [];

fn decode_plm(v: u32) -> &'static str {
    match v {
        0xFFFF_FFFF => "OPEN",
        0xFFFF_FF8F => "closed (stock)",
        0xFFFF_FFEF => "closed (relock+drift)",
        _ => "closed (non-standard)",
    }
}

fn decode_ss(v: u32, width: u32) -> String {
    let names = [".", "1/2", "1/4", "1/8"];
    (0..width)
        .map(|i| {
            let nib = (v >> (4 * i)) & 0xF;
            if nib == 0 {
                ".".to_string()
            } else if nib & 0x8 != 0 {
                // rates 4-7 exist in the field but have no documented meaning
                let rate = match nib & 7 {
                    0 => "0".to_string(), // override + full rate
                    r @ 1..=3 => names[r as usize].to_string(),
                    r => return format!("?{r:x}*"),
                };
                format!("{rate}*")
            } else {
                format!("r{}", nib & 7)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn dump_debug(rd: &dyn Fn(u32) -> u32) {
    println!("--- core/debug block ---");
    for (name, off) in CORE_ROWS {
        let v = rd(off);
        match badf_bucket(v) {
            Some(b) => println!("{name:<22} = {} [{}]", hx32(v), red(b)),
            None => println!("{name:<22} = {}", hx32(v)),
        }
    }
    // PSTRAPS decode (loader sample.rs fields, NvStrapsReBar-verified)
    let s0 = rd(0x0010_1000);
    let s1 = rd(0x0010_100C);
    const BAR1_SIZES: [&str; 11] = [
        "64MiB", "128MiB", "256MiB", "512MiB", "1GiB", "2GiB", "4GiB", "8GiB", "16GiB", "32GiB",
        "64GiB",
    ];
    let bar1_idx = ((s0 >> 14) & 0x3) + ((s1 >> 20) & 0x7);
    let bar0_idx = (s1 >> 17) & 0x7;
    println!(
        "  PSTRAPS: BAR1 index {bar1_idx} = {}   BAR0 = {}MiB   BAR3 = {}xBAR0   override_en = {}",
        BAR1_SIZES.get(bar1_idx as usize).copied().unwrap_or("reserved(>10)"),
        16 << bar0_idx,
        if s1 & 0x0080_0000 != 0 { 1 } else { 2 },
        s0 >> 31
    );
    println!("--- FEAT cluster (both families, cross-visibility) ---");
    for (label, regs) in [
        ("turing 0x4096xx", &FEAT_TURING[..]),
        ("ampere/ada 0x823xxx", &FEAT_AMPADA[..]),
    ] {
        for (name, off) in regs {
            let v = rd(*off);
            let note = match *name {
                "PLM" => {
                    let d = decode_plm(v);
                    let d = if d == "OPEN" { green("OPEN") } else { d.to_string() };
                    format!(" ({d})")
                }
                "SS0" => format!(" [{}]", decode_ss(v, 8)),
                "SS1" => format!(" [{}]", decode_ss(v, 1)),
                "READOUT" | "READOUT_1" => match badf_bucket(v) {
                    Some(b) => format!(" [{}]", red(b)),
                    None if v == 0 => format!(" [{}]", green("full speed")),
                    None => String::new(),
                },
                _ => String::new(),
            };
            println!("  {label:<20} {name:<9} = {}{note}", hx32(v));
        }
    }
    println!("--- family-specific lines (safe reads; badf on the wrong die is expected) ---");
    for (label, regs) in [
        ("ada 4-write", &ADA_ROWS[..]),
        ("ad1x 64g-bar1", &AD1X_ROWS[..]),
    ] {
        for (name, off) in regs {
            let v = rd(*off);
            let note = match *name {
                n if n.ends_with("PLM") => format!(" ({})", decode_plm(v)),
                _ => match badf_bucket(v) {
                    Some(b) => format!(" [{}]", red(b)),
                    None => String::new(),
                },
            };
            println!("  {label:<15} {name:<22} = {}{note}", hx32(v));
        }
    }
    println!("--- ampere/ada FEAT sweep (M1 cluster) ---");
    print!("{: >24}", "");
    for off in AMPADA_SWEEP {
        print!(" {}{}", paint("96", "0x"), paint("93", &format!("{off:08x}")));
    }
    println!();
    print!("{: >24}", "value");
    for off in AMPADA_SWEEP {
        print!(" {}", hx32(rd(off)));
    }
    println!();
    println!("--- PLM survey (cmp170hx 26-register union) ---");
    for (i, off) in PLM_SURVEY.iter().enumerate() {
        let v = rd(*off);
        let note = match badf_bucket(v) {
            Some(b) => format!(" [{}]", red(b)),
            None if v == 0xFFFF_FFFF => format!(" [{}]", green("OPEN")),
            None => String::new(),
        };
        let off_s = format!("{}{}", paint("96", "0x"), paint("93", &format!("{off:08x}")));
        let val_s = hx32(v);
        print!("  {off_s}={val_s}{note}");
        if i % 2 == 1 {
            println!();
        }
    }
    if PLM_SURVEY.len() % 2 == 1 {
        println!();
    }
    for (name, off) in PRIV_ROWS {
        let v = rd(off);
        let note = badf_bucket(v).map(|b| format!(" [{}]", red(b))).unwrap_or_default();
        println!("{name:<22} = {}{note}", hx32(v));
    }
}

fn main() {
    color_init();
    install_crash_cleanup();
    let code = run();
    // strict remove-when-done: our own service and temp file never outlive
    // this process - cleanup runs on every path that got past arg parsing.
    // Gated on DRIVER_TOUCHED: a --dry-run (or any exit before deploy) has
    // nothing to remove and must not burn the 10 x 200 ms retry loop on a
    // temp .sys that never existed.
    if DRIVER_TOUCHED.load(std::sync::atomic::Ordering::Relaxed) {
        cleanup_driver();
        println!("driver service removed (remove-when-done)");
    }
    std::process::exit(code);
}

fn run() -> i32 {
    let mut reapply = false;
    let mut dry = false;
    let mut select: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--reapply" => reapply = true,
            "--help" | "-h" => {
                println!(
                    "ssdiag {ver} - in-OS NVIDIA register diagnosis + separate-lock re-apply

USAGE:
  ssdiag [--reapply] [--select <substr>]

OPTIONS:
  --reapply        per card with an OPEN FEAT gate: write SS0=0x88888888
                   SS1=0x8 and verify by read-back (separate-lock method -
                   restores an unlock the driver re-derived, no reboot)
  --select <s>     only cards whose DeviceID contains <s> (multi-GPU)
  --dry-run        print the platform gates + discovered cards, load nothing
  --help           this text

WHAT IT DOES:
  1. discovers every NVIDIA Display-class GPU and its MMIO ranges from the
     OS itself (assignments change every boot - stale addresses are refused)
  2. wakes GC6/GCOFF-parked GPUs via the driver's own NVAPI GC6Control
     (on demand: only when a card reads degenerate)
  3. per-GPU register dump, badf-bucketed and colored:
       - core/debug block: PMC_BOOT_0, PSTRAPS (decoded BAR1/BAR0/BAR3
         sizes), FB_SIZE_MIB, GFW PLM/scratch, BSI_SCRATCH15, VGA
         workspace, WPR2 lo/hi, MMU PLM + lock window, GSP/SEC2 CPUCTL +
         mailboxes + EXCI, fuse rows
       - FEAT clusters of BOTH families (Turing 0x4096xx incl. READOUT,
         Ampere/Ada 0x823xxx)
       - Ada four-write targets (WPR/WPR_CFG/FBPA PLMs), AD10x 64G-BAR1
         line (CYA/CFG/XVE/BAR1 cap), 26-register PLM survey, Ampere FEAT
         sweep
  4. unlock verdict: FEAT SS0/SS1 == 0x88888888/0x8 => UNLOCKED
     (field-proven to persist in Windows on the Turing relock chain)

FAMILIES: maxwell..rubin recognized (incl. the GA10x 0xb7xx00a1 BOOT_0
quirk); FEAT verdict only where the cluster is mapped (Turing/Ampere/Ada).

DRIVER: pmxdrv.sys embedded (Intel-signed, hash-pinned), deployed as its
own service on demand and stopped+deleted on every exit path.

COLORS: auto on real consoles (VT), pipes/NO_COLOR degrade to plain,
SSDIAG_COLOR=force pins on.",
                    ver = env!("CARGO_PKG_VERSION")
                );
                std::process::exit(0);
            }
            "--dry-run" => {
                dry = true;
            }
            "--select" => {
                select = Some(args.next().unwrap_or_else(|| {
                    eprintln!("--select needs a DeviceID substring (e.g. 1F07 or 4&38d5f52c)");
                    std::process::exit(2);
                }));
            }
            "--bar0" => {
                eprintln!("--bar0 was removed in v0.3: physical assignments change every boot, so                            the tool discovers each GPU's ranges from the OS itself. Use --select                            to narrow multi-GPU systems.");
                std::process::exit(2);
            }
            other => {
                eprintln!("unknown arg {other:?}; usage: ssdiag [--reapply] [--select <substr>]");
                return 2;
            }
        }
    }

    let mut gpus = match discover_gpus() {
        Ok(g) if !g.is_empty() => g,
        Ok(_) => {
            eprintln!("no NVIDIA display adapter memory ranges found");
            return 2;
        }
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    if let Some(sel) = &select {
        gpus.retain(|g| g.id.to_uppercase().contains(&sel.to_uppercase()));
        if gpus.is_empty() {
            eprintln!("--select {sel:?}: no NVIDIA function matches");
            return 2;
        }
    }
    // only functions with a BAR0-class window (companion audio/USB-C are small)
    gpus.retain(|g| !g.bar0_candidates().is_empty());
    if gpus.is_empty() {
        eprintln!("no NVIDIA function with a BAR0-class (>=16 MiB) window found");
        return 2;
    }

    if dry {
        preflight();
        println!("DRY-RUN: {} NVIDIA range set(s) discovered, nothing was loaded or written", gpus.len());
        return 0;
    }

    preflight();
    let drv = match ensure_driver() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };

    println!("{ } NVIDIA GPU function(s) discovered", gpus.len());
    let mut rc = 0;
    for (i, g) in gpus.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("===== GPU {}/{}: {} =====", i + 1, gpus.len(), g.id);
        println!("ranges:");
        for (a, b) in &g.ranges {
            println!("  {}-{} ({} MiB)", hxaddr(*a), hxaddr(*b), (b - a + 1) >> 20);
        }
        let mut picked = walk_candidates(&drv, g);
        // degenerate reads (GC6/GCOFF park, D3, MEM decode off) -> one NVAPI
        // driver-mediated wake, then a single re-walk
        if picked.as_ref().map_or(true, |(_, _, b)| *b == 0 || *b == 0xFFFF_FFFF) {
            match nvapi_wake::wake_all() {
                Ok(r) if !r.is_empty() => println!("NVAPI GC6 wake: {r}"),
                Ok(_) => println!("NVAPI GC6 wake: all GPUs already D0"),
                Err(e) => println!("NVAPI wake unavailable: {e}"),
            }
            picked = walk_candidates(&drv, g);
        }
        let Some((bar0, va, boot0)) = picked else {
            println!("no candidate range decoded as an NVIDIA PMC - card skipped");
            rc = 1;
            continue;
        };
        println!("BAR0: {}", hxaddr(bar0));

        if boot0 == 0xFFFF_FFFF || boot0 == 0 {
            drv.unmap_physical(va);
            println!(
                "BAR0 reads {} - still degenerate after GC6 wake attempt: device D3 / MEM decode off / driver unbound; nothing to diagnose",
                red(if boot0 == 0 { "all zeros" } else { "all ones" })
            );
            rc = 1;
            continue;
        }

        let base = va as *const u8;
        let rd = |off: u32| unsafe { (base.add(off as usize) as *const u32).read_volatile() };
        let wr = |off: u32, v: u32| unsafe { (base.add(off as usize) as *mut u32).write_volatile(v) };

        let Some((fam_name, regs)) = family_of(boot0) else {
            println!("PMC_BOOT_0={} - NVIDIA-class but unknown die; diagnosis-only pass (FEAT addresses unmapped for this family)", hx32(boot0));
            dump_debug(&rd);
            drv.unmap_physical(va);
            rc = 1;
            continue;
        };
        dump_debug(&rd);

        if regs.is_empty() {
            println!(
                "VERDICT [{fam_name}]: family recognized - FEAT addresses not mapped for this family (diagnosis only)"
            );
            drv.unmap_physical(va);
            continue;
        }
        let get = |name: &str| regs.iter().find(|(n, _)| *n == name).unwrap().1;
        let plm = rd(get("PLM"));
        let ss0 = rd(get("SS0"));
        let ss1 = rd(get("SS1"));
        let unlocked = ss0 == SS0_UNLOCK && ss1 == SS1_UNLOCK;
        let verdict = if unlocked {
            format!("{} (override values live)", green("UNLOCKED"))
        } else {
            red("NOT_UNLOCKED")
        };
        println!("VERDICT [{fam_name}]: {verdict}");

        if reapply {
            if plm != 0xFFFF_FFFF {
                println!("{}: FEAT PLM gate is closed - re-run the USB unlock instead", red("reapply REFUSED"));
                rc = 1;
            } else {
                wr(get("SS0"), SS0_UNLOCK);
                wr(get("SS1"), SS1_UNLOCK);
                let rb0 = rd(get("SS0"));
                let rb1 = rd(get("SS1"));
                let ok = rb0 == SS0_UNLOCK && rb1 == SS1_UNLOCK;
                println!(
                    "reapply: SS0={} SS1={} -> {}",
                    hx32(rb0),
                    hx32(rb1),
                    if ok { green("OK") } else { red("READBACK MISMATCH") }
                );
                if !ok {
                    rc = 1;
                }
            }
        }
        drv.unmap_physical(va);
    }
    rc
}

/// Walk the card's BAR0 candidates smallest-size first; the PMC sanity read
/// arbitrates. A non-degenerate BOOT_0 on an unknown die still wins as a
/// fallback (diagnosis-only) - the die table must not veto a card. A
/// degenerate pick (all-0/all-1) is remembered only as a last resort.
fn walk_candidates(drv: &PmxDrv, g: &Gpu) -> Option<(u64, u64, u32)> {
    let mut picked: Option<(u64, u64, u32)> = None; // (bar0, va, boot0)
    for bar0 in g.bar0_candidates() {
        const PAGES: u32 = 4096; // 16 MiB BAR0
        let va = match drv.map_physical(bar0, PAGES) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("map BAR0@{bar0:#x} failed: {e}");
                continue;
            }
        };
        let boot0 = unsafe { ((va as *const u8).add(0) as *const u32).read_volatile() };
        if boot0 == 0xFFFF_FFFF || boot0 == 0 {
            if picked.is_none() {
                picked = Some((bar0, va, boot0));
            } else {
                drv.unmap_physical(va);
            }
            continue;
        }
        match family_of(boot0) {
            Some(_) => {
                if let Some((_, old_va, _)) = picked {
                    drv.unmap_physical(old_va);
                }
                picked = Some((bar0, va, boot0));
                break;
            }
            None => {
                if let Some((_, old_va, old_b)) = picked {
                    if old_b != 0 && old_b != 0xFFFF_FFFF {
                        drv.unmap_physical(va);
                        continue;
                    }
                    drv.unmap_physical(old_va);
                }
                picked = Some((bar0, va, boot0));
            }
        }
    }
    picked
}
