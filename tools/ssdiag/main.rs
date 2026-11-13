//! ssdiag v0.7.9 - in-OS NVIDIA register diagnosis + separate-lock re-apply.
//! v0.7.9 lands the issue #237 disassembly round (JeusChrit, 595.44.15 GSP +
//! VBIOS Falcon): GPC-indexed arrays extended to 12 slots (AD102-class),
//! per-GPC ROP mask 0x822880+4g and the Ada TPC RECONFIG RAM 0x822800+4g
//! (our old 0x820788 was the GA100 layout), ClockManager VF/Speedo trims
//! 0x8213a0-3ac (10-bit s6-temp+s4-volt words - the #147 UV-relevant knobs),
//! the VBIOS app08 override-pair regions 0x822148-1fc/0x822300-444, the
//! PLM write-enable bits (0x8200f8 bit4 = PL0 enable per the TU102/116
//! OBJBSP writer; 0x823b24/28 bit6 = PL2, the app08 gate - which also
//! settles stock EN_SW_OVERRIDE=1 as routine VBIOS behavior), and the GB20x
//! HAL 67-71 PES/OFA relocation 0x822a78/0x822a68 (why 0x820dbc reads
//! all-badf on GB206).
//! v0.7.8 fixes two more inventory bugs, both field-caught on a ROG Strix
//! G614JV (i7-13650HX + 2x32G DDR5 SODIMM): hybrid CPUs broke the
//! logical/tpc core division (E-cores are single-threaded - a 6P+8E 14-core
//! chip printed "10 cores"; now RelationProcessorCore entries from
//! GetLogicalProcessorInformationEx are counted across all groups, with a
//! self-validating variable-length walk and the division kept as fallback),
//! and DDR5 >=32GiB SODIMMs overflow the 15-bit MiB form of the SMBIOS
//! Type 17 size field, so the extended size @0x1D is now preferred whenever
//! it carries a real value ("128 MiB" slots -> 32768).
//! v0.7.7 adds the GB206 TPC CTRL row (0x820838+4g, 12 GPCs, read-only) to
//! the blackwell branch - the writable-override sibling of the 0x820c38
//! STATUS pair already captured as GA100_SM_STAT; requested on discussion
//! #216 (desktop 5060 Ti vs 460 laptop floorsweep comparison).
//! v0.7.6 fixes two inventory bugs field-caught on a Huananzhi X99-F8D dual
//! E5-2699v3: the CPU line counted only the calling thread's processor group
//! (GetNativeSystemInfo is group-scoped - 8 threads shown on a 72-thread
//! box; now GetActiveProcessorCount(ALL_PROCESSOR_GROUPS)), and the SMBIOS
//! Type 17 ConfiguredMemoryClockSpeed was read at 0x21 - one past the spec's
//! 0x20 - stitching 0x08 + a string byte into "29704 MT/s" on DDR4-2133
//! (most boards' short Type 17 records never reached the misaligned read;
//! offset fixed, plausibility gate kept).
//! v0.7.5 moves the GR topology anchor (0x409604) out of the turing gate to
//! every family - three-point calibrated (TU104 KO 0x06030005, TU106 2070
//! 0x08040003, TU106 1e93 0x08040006: byte3 = die GPC slots, byte0 = active
//! GPC count) - making it the open-questions #19 polarity discriminator on
//! Ampere (GA106 byte0 4 = masks bit=off, 7 = bit=on).
//! v0.7.4 adds the GA100 VFI dictionary round (reading-notes 2026-11-11),
//! three read-only sections in one build: (1) the kate-land/ga100vfi
//! dictionary - fuse-controller core, the 17 row0/1 security aliases, the
//! fused SM_SPEED_SELECT family, STATUS_OPT_* effective words - probed on
//! every die at the GA100 offsets; (2) the Turing mirror probe (16 words x
//! {0x820xxx, -0x800000, -0x7FF000}) that settled where the option VALUES
//! live on TU106: -0x7FF000 is the real bank (#187/TU104), 0x820xxx decodes
//! a control-agreeing surface with empty option slots; (3) for turing dies
//! the whole dictionary printed a second time at the shifted base, plus the
//! GR-topology 0x409604 cross-check anchor. The alias table carries
//! FEATURE_FUSES_OVR_DIS (FFDO, 0x8203f0 primary / 0x213f0 shifted).
//! Coverage completion (2026-11-12, same version): the repo's option map
//! holds 672 words and the first pass swept 29 - the fuse-side
//! _DISABLE/_DEFECTIVE pairs, the _CP shadow family (volatile
//! floorsweep-reversal candidates), the TPC per-GPC disable/reconfig/
//! defective words and the PCIE devid/magic words are now swept too
//! (~120/672; the ~550 GA100-specific blocks stay excluded by design).
//! v0.7.1 widens the inventory block: physical RAM total + per-module
//! the kate-land/ga100vfi fuse-controller map is cross-family shared IP
//! (0x8203f0 identical on GA100 and TU106), so the controller core, the 17
//! row0/1 security aliases behind the goldilocks glitch state, the fused
//! SM_SPEED_SELECT family and the STATUS_OPT_* effective words (scalars +
//! PES/ROP_L2/TPC arrays) are probed read-only on every die. badf5040 =
//! word exists behind a PLM; ffffffff/badf1100 = not decoded here - the
//! per-die existence answer IS the experiment (open question #SBFE).
//! v0.7.1 widens the inventory block: physical RAM total + per-module
//! SMBIOS Type 17 rows (slot / capacity / MT/s, no serials), disk models
//! with volumes and partition offsets (unelevated volume extents), and the
//! other PCI devices on the bus. The Secure Boot note now says "may not
//! load" - field reports show the probe running on some SB-ON hosts. The
//! per-card driver / GSP read anchors on the driver's own bound-instance list
//! (Services\nvlddmkm\Enum, the maker's ghost-free source) instead of the
//! devnode property walk, which a card-swap ghost can mislead. The #99
//! PF-FLR driver advisory is withdrawn - not verified beyond one issue.
//! v0.7.0 adds the unelevated `--inventory` block (the issue-report hardware
//! snapshot): OS/CPU/board/BIOS/chassis from the registry + raw SMBIOS,
//! per-card driver / VBIOS / EnableGpuFirmware from the class key + nvapi,
//! and a device-id family verdict with pre-flight advisories (Turing GSP
//! legacy mode, the #99 PF-FLR driver family, Secure Boot, laptop chassis).
//! Every mode prints the block first, so a full dump carries its own platform
//! context; `--inventory` prints it alone - no admin, no driver, nothing
//! written.
//! v0.6.14 adds a reference-map line to the TPC block: a STATUS vector
//! byte-identical to the two independent RTX 3070 samples (#169/#54) prints
//! the match, so future reports self-classify as standard binning.
//! v0.6.13 adds the read-only `USB4:` block (issue #54): both Microsoft
//! connection-manager service start types plus the USB4 devnode IDs, and a
//! verdict on whether the device-router takeover reset is suppressed. The
//! per-GPC TPC block, the POWER block and every probe value are unchanged.
//! v0.6.12 redesigns the 0.6.11 arm to a REGISTRY-ONLY lifecycle: --safeboot-arm
//! writes just the two SafeBoot whitelist subkeys (the check is name-based);
//! no binary and no service are staged, and the safe-mode run extracts to
//! %TEMP% and builds the service exactly as a normal-mode run does, so the
//! driver copy exists only for the seconds of each probe run (the 0.6.11
//! persistent %ProgramData% copy + demand-start service was a ready-made
//! BYOVD slot). The proof round uses that same temp layout.
//! v0.6.11 adds --safeboot-arm / --safeboot-disarm (issue #37): stage and
//! remove the pmxdrv vehicle for Safe Mode - the SafeBoot\Minimal/Network
//! whitelist that StartService checks (win32 1084), with an in-session
//! self-heal attempt on 1084 and boot-frozen fallback instructions.
//! v0.6.10 adds the read-only per-GPC TPC floorsweep block for Ampere/Ada
//! (issue #169): STATUS/CTRL pair, GPC-level disable/defect masks, raw
//! OPT/DEFECT banks, and a defect-vs-override-candidate verdict.
//! v0.6.9 maps the Blackwell (GB20x) families onto the FEAT bank and dumps
//! their fuse/selector words read-only (issue #168): a gapped READOUT_1
//! decoder, a PL3-only PLM verdict, and a family note saying the re-apply
//! cannot open the gate because no GB20x payload exists. Also prints the
//! "closed (PL3-only writes)" PLM state.
//! v0.6.8 adds the read-only POWER section: the card's and its upstream root
//! port's PMCSR / D-state, the register-side witness of the platform's
//! `_PR3`/`_OFF` rail path (issue #164/#165).
//! v0.6.3 adds the HVCI runtime probe (SystemCodeIntegrityInformation) to the
//! platform gates and a denied-map precondition breakdown (issue #118).
//! v0.6.2 adds cumulative per-stage [+ms] timing lines (the loader's issue#38
//! convention) so a slow run shows where its time went, and tightens the
//! exit-side temp-file retry grain.
//! v0.6.1 replaces the powershell.exe GPU discovery (~5 s paid every run)
//! with a native CfgMgr32 walk - same OS sources, no spawned process, works
//! unelevated.
//! v0.5.0 adds a read-only config-space ReBAR probe (ECAM via MCFG; the
//! BAR1-resize answer board A4G/ReBAR settings can and cannot provide).
//!
//! Reads the registers the EFI unlock stage touches, from inside the running
//! OS, and - where the EFI chain left the FEAT PLM gate open (Ampere/Ada AON
//! behavior, issue #6) - re-applies the SS override values without a reboot.
//! On Blackwell everything is read-only: the 0x823xxx FEAT words ARE readable
//! there, but the gate admits only PL3 writes (signed firmware) and there is
//! no booter to run, so --reapply refuses instead of pretending.
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
        pub fn GetSystemFirmwareTable(
            provider: u32,
            table_id: u32,
            buffer: *mut u8,
            buffer_size: u32,
        ) -> u32;
        pub fn LoadLibraryW(name: *const u16) -> Handle;
        pub fn GetProcAddress(h: Handle, name: *const u8) -> *const c_void;
        pub fn GetStdHandle(nStdHandle: u32) -> Handle;
        pub fn GetConsoleMode(hConsole: Handle, mode: *mut u32) -> i32;
        pub fn SetConsoleMode(hConsole: Handle, mode: u32) -> i32;
        pub fn SetConsoleCtrlHandler(
            handler: Option<unsafe extern "system" fn(u32) -> i32>,
            add: i32,
        ) -> i32;
        pub fn GlobalMemoryStatusEx(buf: *mut c_void) -> i32;
        pub fn FindFirstVolumeW(buf: *mut u16, len: u32) -> Handle;
        pub fn FindNextVolumeW(h: Handle, buf: *mut u16, len: u32) -> i32;
        pub fn FindVolumeClose(h: Handle) -> i32;
        pub fn GetVolumePathNamesForVolumeNameW(
            name: *const u16,
            buf: *mut u16,
            len: u32,
            ret: *mut u32,
        ) -> i32;
        #[link_name = "GetVolumeInformationW"]
        pub fn GetVolumeInformationW(
            root: *const u16,
            vol_buf: *mut u16,
            vol_len: u32,
            serial: *mut u32,
            maxlen: *mut u32,
            flags: *mut u32,
            fs_buf: *mut u16,
            fs_len: u32,
        ) -> i32;
        pub fn GetDiskFreeSpaceExW(
            dir: *const u16,
            free_avail: *mut u64,
            total: *mut u64,
            total_free: *mut u64,
        ) -> i32;
        pub fn GetNativeSystemInfo(info: *mut SystemInfo) -> i32;
        pub fn GetActiveProcessorCount(GroupNumber: u16) -> u32;
        pub fn GetLogicalProcessorInformationEx(
            relationship: u32,
            buffer: *mut u8,
            return_length: *mut u32,
        ) -> i32;
    }

    /// Field layout matches winnt.h SYSTEM_INFO for x64 (ULONG_PTR fields
    /// are 8 bytes here - active_mask as u32 silently shifted everything
    /// after it and num_processors read as 0).
    #[repr(C)]
    pub struct SystemInfo {
        pub arch: u16,
        pub reserved: u16,
        pub page_size: u32,
        pub min_addr: *mut c_void,
        pub max_addr: *mut c_void,
        pub active_mask: usize,
        pub num_processors: u32,
        pub processor_type: u32,
        pub alloc_granularity: u32,
        pub processor_level: u16,
        pub processor_revision: u16,
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
    use std::ffi::c_void;
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
        // SafeMode whitelist staging (--safeboot-arm / self-heal on 1084)
        pub fn RegCreateKeyExW(
            hkey: isize,
            subkey: *const u16,
            reserved: u32,
            class: *const u16,
            options: u32,
            sam: u32,
            sec: *const c_void,
            out: *mut isize,
            disp: *mut u32,
        ) -> i32;
        pub fn RegSetValueExW(
            hkey: isize,
            name: *const u16,
            reserved: u32,
            typ: u32,
            data: *const u16,
            cb: u32,
        ) -> i32;
        pub fn RegDeleteTreeW(hkey: isize, subkey: *const u16) -> i32;
        pub fn RegCloseKey(hkey: isize) -> i32;
    }
}

// ---------------------------------------------------------------- code integrity (HVCI runtime state)

// HVCI runtime probe: NtQuerySystemInformation(SystemCodeIntegrityInformation = 103) -
// the authoritative *runtime* HVCI state. The registry value under
// HVCI\Scenario reads back as the CONFIGURED policy (often absent on
// machines where Windows 11 enabled memory integrity by default, which
// `gate_word` prints as "unknown"); the CodeIntegrity options are what
// the kernel actually enforces for MmMapIoSpace on caller-chosen
// physical ranges. HVCI running = bit 0x10 (KMCI strict mode); bit 0x2
// = test signing. Issue #118: this is the gate whose presence explains
// a MAP_PHYS `win32 error 5` on an otherwise healthy, elevated run.
#[link(name = "ntdll")]
extern "system" {
    fn NtQuerySystemInformation(
        class: u32, // SYSTEM_INFORMATION_CLASS
        buffer: *mut core::ffi::c_void,
        length: u32,
        returned: *mut u32,
    ) -> i32;
}

const SYSTEM_CODEINTEGRITY_INFORMATION: u32 = 103;
const CI_OPTION_HVCI_KMCI_ENABLED: u32 = 0x10;

// Some(true) = HVCI running; Some(false) = clear; None = query refused
// (treat as unknown, never as a verdict).
fn hvci_runtime() -> Option<bool> {
    #[repr(C)]
    struct CiInfo {
        // SYSTEM_CODEINTEGRITY_INFORMATION opens with the struct Length -
        // the kernel validates it and returns STATUS_INVALID_PARAMETER
        // otherwise (the original probe left it out and read as "refused"
        // even elevated).
        length: u32,
        options: u32,
    }
    let mut info = CiInfo {
        length: core::mem::size_of::<CiInfo>() as u32,
        options: 0,
    };
    let mut returned: u32 = 0;
    // Safety: buffer/length describe the struct; class and sizes are the
    // documented pair for SystemCodeIntegrityInformation.
    let rc = unsafe {
        NtQuerySystemInformation(
            SYSTEM_CODEINTEGRITY_INFORMATION,
            (&mut info as *mut CiInfo).cast(),
            core::mem::size_of::<CiInfo>() as u32,
            &mut returned,
        )
    };
    if rc != 0 || (returned as usize) < core::mem::size_of::<CiInfo>() {
        return None;
    }
    Some(info.options & CI_OPTION_HVCI_KMCI_ENABLED != 0)
}

static MAP_DENIED_SHOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Issue #118 (Catnip's point, adopted): a raw `win32 error 5` plus
/// "card skipped" reads like a verdict on the card, while the identical
/// output is produced by at least four preconditions. Printed once, on
/// the first denied map, so the user knows which gate to suspect.
fn map_denied_guidance() {
    if MAP_DENIED_SHOWN.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    println!("{}", yellow("MAP denied (win32 error 5) - candidate preconditions, most likely first:"));
    let hvci = hvci_runtime();
    match hvci {
        Some(true) => println!(
            "  1. HVCI (memory integrity) is RUNNING on this kernel - MmMapIoSpace on
     caller-chosen physical ranges is refused by design. Disable Core
     isolation (Windows Security > Device security) to use ssdiag here."
        ),
        Some(false) => println!("  1. HVCI is not running - mapping itself is not policy-blocked;"),
        None => println!("  1. HVCI runtime state could not be queried (treat as unknown);"),
    }
    println!("  2. a kernel-mode AV/EDR filter driver vetoing the map (user-mode AV processes");
    println!("     being closed does NOT unload it - temporarily remove the AV kernel");
    println!("     component instead of pausing protection);");
    println!("  3. the driver never actually loaded (SCM event 7045 says installed, but");
    println!("     CodeIntegrity events 3077/3089 say refused - check the System log).");
}

// ---------------------------------------------------------------- cfgmgr32 (GPU discovery)

/// CfgMgr32 - the layer Win32_PnPAllocatedResource / Get-PnpDevice wrap.
/// Used directly for GPU discovery so the run does not pay for a powershell.exe
/// spawn (~5 s measured) on every startup. Read-only, works unelevated.
#[allow(non_snake_case)]
mod cm {
    use std::ffi::c_void;
    pub type ConfigRet = u32; // CONFIGRET = RETURN_TYPE
    pub type DevInst = u32; // DEVINST = DWORD
    // DWORD_PTR: on x64 these handles are pointer-sized - declaring them u32
    // truncates the handle and CM_Get_Next_Res_Des fails CR_INVALID_RES_DES
    pub type LogConf = usize; // LOG_CONF = DWORD_PTR
    pub type ResDes = usize; // RES_DES = DWORD_PTR

    pub const CR_SUCCESS: u32 = 0x0000_0000;
    pub const CM_GETIDLIST_FILTER_CLASS: u32 = 0x0000_0200;
    pub const CM_GETIDLIST_FILTER_ENUMERATOR: u32 = 0x0000_0001;
    pub const ALLOC_LOG_CONF: u32 = 0x0000_0002; // the allocated (boot-assigned) config
    // Remaining CM_Get_First_Log_Conf priorities. An unstarted device (safe
    // mode blocks nvlddmkm, so PnP leaves it Stopped) can lack the ALLOC
    // config while a lower priority still carries the firmware-assigned
    // ranges - issue #37 field report.
    pub const BOOT_LOG_CONF: u32 = 0x0000_0003;
    pub const FORCED_LOG_CONF: u32 = 0x0000_0004;
    pub const OVERRIDE_LOG_CONF: u32 = 0x0000_0005;
    pub const BASIC_LOG_CONF: u32 = 0x0000_0006;
    pub const RES_TYPE_MEM: u32 = 0x0000_0001;
    pub const RES_TYPE_MEM_LARGE: u32 = 0x0000_0007; // >= 4 GiB descriptors
    // CM_DRP numbering field-verified with a live devnode probe (values ARE
    // 1-based here, not the 0-based layout some headers suggest):
    // 1=DeviceDesc, 10=Driver class slot, 14=LocationInformation, 22=BusNumber
    pub const CM_DRP_DEVICEDESC: u32 = 0x0000_0001;
    pub const CM_DRP_DRIVER: u32 = 0x0000_000A;
    pub const CM_DRP_LOCATION_INFORMATION: u32 = 0x0000_000E;
    pub const CM_DRP_BUSNUMBER: u32 = 0x0000_0016;
    pub const CM_DRP_ADDRESS: u32 = 0x0000_001D;

    #[link(name = "cfgmgr32")]
    extern "system" {
        pub fn CM_Get_Device_ID_List_SizeW(len: *mut u32, filter: *const u16, flags: u32) -> ConfigRet;
        pub fn CM_Get_Device_ID_ListW(filter: *const u16, buf: *mut u16, buflen: u32, flags: u32) -> ConfigRet;
        pub fn CM_Locate_DevNodeW(devinst: *mut DevInst, id: *mut u16, flags: u32) -> ConfigRet;
        // upstream bridge of a devnode - the _PR3/_OFF owner (POWER section)
        pub fn CM_Get_Parent(parent: *mut DevInst, devinst: DevInst, flags: u32) -> ConfigRet;
        pub fn CM_Get_Device_IDW(devinst: DevInst, buf: *mut u16, buflen: u32, flags: u32)
            -> ConfigRet;
        // the log-conf family has no W export: the plain name IS the wide entry
        pub fn CM_Get_First_Log_Conf(logconf: *mut LogConf, devinst: DevInst, priority: u32) -> ConfigRet;
        pub fn CM_Get_Next_Res_Des(
            rd: *mut ResDes,
            cur: ResDes,
            restype: u32,
            out_type: *mut u32,
            flags: u32,
        ) -> ConfigRet;
        pub fn CM_Get_Res_Des_Data(rd: ResDes, buf: *mut c_void, buflen: u32, flags: u32) -> ConfigRet;
        pub fn CM_Free_Res_Des_Handle(rd: ResDes) -> ConfigRet;
        pub fn CM_Free_Log_Conf_Handle(lc: LogConf) -> ConfigRet;
        pub fn CM_Get_DevNode_Registry_PropertyW(
            devinst: DevInst,
            prop: u32,
            regtype: *mut u32,
            buf: *mut c_void,
            len: *mut u32,
            flags: u32,
        ) -> ConfigRet;
        // device -> its disk interface paths (\\?\scsi#...), the documented
        // bridge from a PnP devnode to a PhysicalDrive number
        pub fn CM_Get_Device_Interface_ListW(
            guid: *const u8,
            device_id: *const u16,
            buffer: *mut u16,
            buffer_len: u32,
            flags: u32,
        ) -> ConfigRet;
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

/// gate_word, colored: OFF = green (nothing gates the probe here), ON = red,
/// unknown = yellow. Platform-block polish (2026-11-12 user feedback).
fn gate_word_c(v: Option<u32>) -> String {
    match v {
        Some(0) => green("OFF"),
        Some(_) => red("ON"),
        None => yellow(gate_word(v)),
    }
}

/// The platform gates that decide whether the pinned pmxdrv can load at all,
/// plus what installing it looks like from the outside. All values are
/// readable without the driver, so this runs before anything is deployed -
/// and `--dry-run` can print it on a machine where the real run would only
/// fail with a raw win32 code.
///
/// Gate semantics (issue #40 analysis, softened 2026-11-08 per field data):
/// - Secure Boot ON was expected to refuse THIS driver (the pinned pmxdrv.sys
///   is Intel/COMODO cross-signed, no Microsoft root, post-2015 leaf cert -
///   none of the DSE carve-outs), but field reports show the probe loading
///   and running on some SB-ON hosts, so the tool now says "may not load"
///   instead of a verdict and asks the user to simply try.
/// - The Microsoft vulnerable-driver blocklist does NOT list this hash (file
///   SHA256, authentihashes and imphash all absent from both the OS-delivered
///   and the downloadable policies, with in-list control hashes verified), so
///   blocklist/HVCI being ON refuses nothing until Microsoft adds the hash.
fn preflight() {
    let blocklist = hklm_dword(CI_CONFIG, "VulnerableDriverBlocklistEnable");
    let hvci = hklm_dword(HVCI_SCENARIO, "Enabled");
    let secure_boot = hklm_dword(SECUREBOOT_STATE, "UEFISecureBootEnabled");
    println!("PLATFORM: vulnerable-driver blocklist = {}", gate_word_c(blocklist));
    println!("PLATFORM: core isolation (HVCI)       = {}", gate_word_c(hvci));
    match hvci_runtime() {
        Some(true) => println!(
            "PLATFORM: HVCI runtime (CodeIntegrity) = {} - MmMapIoSpace on arbitrary{} physical ranges is refused by the kernel",
            red("RUNNING"),
            ""
        ),
        Some(false) => println!("PLATFORM: HVCI runtime (CodeIntegrity) = {}", green("clear")),
        None => println!("PLATFORM: HVCI runtime (CodeIntegrity) = {}", yellow("unknown (query refused)")),
    }
    println!("PLATFORM: Secure Boot                 = {}", gate_word_c(secure_boot));
    if secure_boot == Some(1) {
        println!(
            "{}",
            yellow("PLATFORM: Secure Boot is ON - the pinned pmxdrv.sys may not load here.")
        );
        println!("  It is cross-signed (Intel/COMODO, no Microsoft root, signing cert issued");
        println!("  after the 2015-07-29 cutoff), which fits none of the DSE carve-outs - but");
        println!("  field reports show the probe running on some Secure Boot ON hosts, so this");
        println!("  is a caution, not a verdict: just try. If the load fails, the driver-free");
        println!("  path is the EFI read-only sampler.");
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
    usb4_config();
}

/// Issue #54: the USB4/Thunderbolt connection manager resets the tunnel domain
/// when its device-router driver binds, which closes the FEAT gate; disabling
/// only `Usb4DeviceRouter` (the tunnel side) while leaving `Usb4HostRouter`
/// running suppresses that reset and was reported working on two hosts. This
/// block is read-only and exists so every report carries its own configuration
/// instead of the reporter having to describe it - `Start` 4/3 is the working
/// shape, and the two words are also the first thing to blame when a TB report
/// shows a cleared canary.
fn usb4_config() {
    const SVC: &str = "SYSTEM\\CurrentControlSet\\Services\\";
    let host = hklm_dword(&format!("{SVC}Usb4HostRouter"), "Start");
    let dev = hklm_dword(&format!("{SVC}Usb4DeviceRouter"), "Start");
    // a service that never existed (pure TB3 controller, non-Microsoft CM)
    if host.is_none() && dev.is_none() {
        println!("USB4: neither Usb4HostRouter nor Usb4DeviceRouter is installed here");
        return;
    }
    let word = |v: Option<u32>| match v {
        Some(0) => "0 (Boot)".to_string(),
        Some(1) => "1 (System)".to_string(),
        Some(2) => "2 (Automatic)".to_string(),
        Some(3) => "3 (Manual)".to_string(),
        Some(4) => red("4 (Disabled)").to_string(),
        Some(n) => format!("{n} (unknown)"),
        None => "absent".to_string(),
    };
    println!("USB4: Usb4HostRouter Start = {}", word(host));
    println!("USB4: Usb4DeviceRouter Start = {}", word(dev));
    // instance IDs make the router silicon visible (ASMedia/Intel; #54 had both)
    if let Ok(ids) = enum_filtered("USB4") {
        if ids.is_empty() {
            println!("USB4: no USB4 devnodes enumerated");
        }
        for id in ids.iter().take(6) {
            println!("USB4: node {id}");
        }
    }
    match (host, dev) {
        (_, Some(4)) if host != Some(4) => println!(
            "USB4: device-router driver DISABLED - the takeover reset is suppressed; expect the unlock to survive the handoff"
        ),
        (Some(4), Some(4)) => println!(
            "USB4: both routers disabled - the takeover is suppressed, but the tunnel may not come up at all"
        ),
        (Some(4), _) => println!(
            "USB4: host router disabled only - the device router cannot bind; the tunnel may not come up"
        ),
        (_, Some(2) | Some(3) | Some(0) | Some(1)) => println!(
            "USB4: device-router driver ENABLED - the takeover will reset the tunnel at bind; a cleared canary is expected"
        ),
        _ => println!("USB4: device-router Start unreadable - treat as unknown"),
    }
}

// ---------------------------------------------------------------- inventory (v0.7.0)

/// The unelevated machine/GPU snapshot every mode prints first, and that
/// `--inventory` prints alone: the hardware block the issue templates ask
/// for, collected from the registry / raw SMBIOS / nvapi instead of the
/// reporter's memory. Nothing here needs the probe driver or elevation, and
/// nothing is written anywhere. No hostname, username or serials are read.
const OS_CURVER: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
const SMBIOS_BIOS_KEY: &str = r"HARDWARE\DESCRIPTION\System\BIOS";
const CPU0_KEY: &str = r"HARDWARE\DESCRIPTION\System\CentralProcessor\0";
const ENUM_ROOT: &str = r"SYSTEM\CurrentControlSet\Enum";
const NV_ENUM_KEY: &str = r"SYSTEM\CurrentControlSet\Services\nvlddmkm\Enum";
const CLASS_ROOT: &str = r"SYSTEM\CurrentControlSet\Control\Class";

/// One named HKLM string value (reg_value_string takes the wide value name).
fn hklm_string(subkey: &str, value: &str) -> Option<String> {
    let v = wide(value);
    reg_value_string(HKEY_LOCAL_MACHINE, subkey, v.as_ptr())
}

/// Chassis class from the raw SMBIOS table (GetSystemFirmwareTable 'RSMB'):
/// walk the DMTF structures ({u8 type, u8 len, u16 handle} + formatted area +
/// double-NUL string set); System Enclosure (type 3) carries the chassis type
/// byte at struct offset 5. The registry has no chassis type, and the CIM
/// route would cost a spawn (the thing v0.6.1 removed).
fn chassis_class() -> Option<(&'static str, u8)> {
    const RSMB: u32 = 0x5253_4D42; // 'RSMB'
    let need = unsafe { win::GetSystemFirmwareTable(RSMB, 0, core::ptr::null_mut(), 0) };
    if need == 0 {
        return None;
    }
    let mut buf = vec![0u8; need as usize];
    let got = unsafe { win::GetSystemFirmwareTable(RSMB, 0, buf.as_mut_ptr(), need) };
    if got == 0 {
        return None;
    }
    // RAW_SMBIOS_DATA: u8 calling, u8 major, u8 minor, u8 dmi_rev, u32 len
    if buf.len() < 8 {
        return None;
    }
    let data_len = u32::from_le_bytes(buf[4..8].try_into().ok()?) as usize;
    if data_len == 0 || buf.len() < 8 + data_len {
        return None;
    }
    let data = &buf[8..8 + data_len];
    let mut off = 0usize;
    while off + 4 <= data.len() {
        let typ = data[off];
        let len = data[off + 1] as usize;
        if len < 4 || off + len > data.len() {
            break;
        }
        // Win32_SystemEnclosure.ChassisTypes encoding (portable set only)
        if typ == 3 && len >= 8 {
            let chassis = data[off + 5];
            let word = match chassis {
                8 | 9 | 10 | 11 | 14 | 30 | 31 | 32 => "laptop/portable",
                3 | 4 | 5 | 6 | 7 | 13 | 15 | 16 | 17 | 23 | 24 | 28 | 35 => "desktop/tower",
                _ => "other/unknown",
            };
            return Some((word, chassis));
        }
        // skip the formatted area, then the double-NUL string set
        let mut p = off + len;
        let mut nul = 0usize;
        while p < data.len() && nul < 2 {
            if data[p] == 0 {
                nul += 1;
            } else {
                nul = 0;
            }
            p += 1;
        }
        off = p;
    }
    None
}

/// WDDM DriverVersion "32.0.15.6094" -> the marketing number "560.94"
/// (part[2] drops its leading digit, part[3] splits 2+2; anchors: 15.3742 ->
/// 537.42, 16.2101 -> 621.01). None when the shape does not fit - callers
/// then print the raw version only.
fn marketing_driver_version(raw: &str) -> Option<String> {
    let p: Vec<&str> = raw.split('.').collect();
    if p.len() != 4 || p[2].len() != 2 || p[3].len() != 4 {
        return None;
    }
    let d2 = *p[2].as_bytes().get(1)? as char;
    let (maj, min) = p[3].split_at(2);
    if !d2.is_ascii_digit() || !maj.bytes().chain(min.bytes()).all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(format!("{d2}{maj}.{min}"))
}

/// Instance paths the NVIDIA driver is actually bound to THIS boot - the
/// numbered values of Services\nvlddmkm\Enum, the maker's authoritative
/// ghost-free source (make-usbroot.ps1 Get-BoundNvidiaInstancePaths): PnP
/// maintains this list for started devnodes only, so card-swap residue
/// disappears the boot after the card is pulled.
fn nvlddmkm_bound() -> Vec<String> {
    let mut out = Vec::new();
    if let Some(count) = hklm_dword(NV_ENUM_KEY, "Count") {
        for i in 0..count.min(16) {
            if let Some(p) = hklm_string(NV_ENUM_KEY, &i.to_string()) {
                out.push(p);
            }
        }
    }
    out
}

/// Per-adapter driver facts from the class key: DriverVersion/DriverDate and
/// the EnableGpuFirmware GSP switch. The class slot is resolved the way the
/// maker resolves it (Get-InstanceRegistryProps): when this instance is on
/// the driver's bound list, the live slot is Enum\<instance>\Driver - the
/// devnode CM_DRP_DRIVER walk can land on a card-swap ghost when two
/// devnodes share one location string, which read the wrong slot in the
/// field (LEGACY shown for a GSP-mode card). None = no bound driver.
struct CardDriver {
    version: String,
    date: Option<String>,
    gsp: Option<u32>,
}

fn card_driver(instance: &str, devinst: u32) -> Option<CardDriver> {
    let slot = if nvlddmkm_bound().iter().any(|b| b.eq_ignore_ascii_case(instance)) {
        hklm_string(&format!(r"{ENUM_ROOT}\{instance}"), "Driver")
    } else {
        cm_string(devinst, cm::CM_DRP_DRIVER)
    }?;
    let key = format!(r"{CLASS_ROOT}\{slot}");
    let version = hklm_string(&key, "DriverVersion")?;
    Some(CardDriver {
        version,
        date: hklm_string(&key, "DriverDate"),
        gsp: hklm_dword(&key, "EnableGpuFirmware"),
    })
}

/// "PCI\VEN_10DE&DEV_1F07&SUBSYS_8671043..." -> "8671043" (Windows writes
/// subsystem device first, subsystem vendor second).
fn subsys_from(instance: &str) -> Option<String> {
    let up = instance.to_uppercase();
    let i = up.find("SUBSYS_")? + 7;
    let s = up.get(i..i + 8)?;
    if s.bytes().all(|b| b.is_ascii_hexdigit()) {
        Some(s.to_string())
    } else {
        None
    }
}

/// Transaction-family admission by PCI device id - the same ranges the
/// loader's admission table and tools/collect_compat.py carry. The inventory
/// verdict is a pre-answer, not the unlock verdict: the register-level FEAT
/// read still needs the elevated run.
fn family_verdict_by_devid(devid: u16) -> (&'static str, bool) {
    match devid {
        0x2600..=0x28FF => ("ad10x (Ada)", true),
        0x2400..=0x25FF => ("ga10x (Ampere GA103/104/106/107)", true),
        0x2200..=0x223F => ("ga102 (Ampere)", true),
        0x1E02..=0x1FFF => ("tu10x (Turing)", true),
        0x2182..=0x21D1 => ("tu11x (Turing TU116/117)", true),
        _ => ("outside the tu10x/ga10x/ad10x families", false),
    }
}

// ---- v0.7.1 inventory additions: RAM / disks / partitions / other PCI ----

/// nth (1-based) NUL-terminated string of a structure's string block
/// (starts right after the formatted area, ends at the double NUL).
fn smbios_string(data: &[u8], mut p: usize, idx: u8) -> String {
    if idx == 0 {
        return String::new();
    }
    let mut cur = 1u8;
    while p < data.len() && data[p] != 0 {
        let start = p;
        while p < data.len() && data[p] != 0 {
            p += 1;
        }
        let s = String::from_utf8_lossy(&data[start..p]).to_string();
        p += 1; // its NUL
        if cur == idx {
            return s;
        }
        cur += 1;
    }
    String::new()
}

/// Physical RAM total via GlobalMemoryStatusEx.
fn physical_total_mib() -> Option<u64> {    #[repr(C)]
    struct MsEx {
        length: u32,
        load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page: u64,
        avail_page: u64,
        total_virt: u64,
        avail_virt: u64,
        avail_ext: u64,
    }
    let mut ms = MsEx {
        length: core::mem::size_of::<MsEx>() as u32,
        load: 0,
        total_phys: 0,
        avail_phys: 0,
        total_page: 0,
        avail_page: 0,
        total_virt: 0,
        avail_virt: 0,
        avail_ext: 0,
    };
    let ok = unsafe { win::GlobalMemoryStatusEx((&mut ms as *mut MsEx).cast()) };
    if ok != 0 && ms.total_phys > 0 {
        Some(ms.total_phys >> 20)
    } else {
        None
    }
}

/// CPUID topology: "family X model Y stepping Z, N cores / M threads" - the
/// brand string alone is useless on ES/QS CPUs ("Genuine Intel(R) CPU 0000").
/// The logical count comes from the OS (all sockets), threads-per-core from
/// CPUID leaf 0x0B sub-leaf 0, cores = logical / tpc; F/M/S per the standard
/// display-family/model decode.
#[cfg(target_arch = "x86_64")]
fn cpu_topology() -> Option<String> {
    use core::arch::x86_64::__cpuid_count;
    let mut si = win::SystemInfo {
        arch: 0,
        reserved: 0,
        page_size: 0,
        min_addr: ptr::null_mut(),
        max_addr: ptr::null_mut(),
        active_mask: 0,
        num_processors: 0,
        processor_type: 0,
        alloc_granularity: 0,
        processor_level: 0,
        processor_revision: 0,
    };
    unsafe { win::GetNativeSystemInfo((&mut si as *mut win::SystemInfo).cast()) };
    // GetNativeSystemInfo.num_processors is scoped to the CALLING thread's
    // processor group (Win7+ splits >64-thread sockets into groups: a dual
    // E5-2699v3 is 36+36, and one group is not the machine). The inventory
    // line must be the whole box - GetActiveProcessorCount(ALL_PROCESSOR_
    // GROUPS) sums every group. Field-observed (Huananzhi X99-F8D, 2026-11-12):
    // the group-scoped count printed 8 threads on a 72-thread box.
    const ALL_PROCESSOR_GROUPS: u16 = 0xFFFF;
    let all: u32 = unsafe { win::GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) };
    let logical = if all > si.num_processors { all } else { si.num_processors };
    if logical == 0 {
        return None;
    }
    let tpc = __cpuid_count(0x0B, 0).ebx & 0xFFFF;
    let tpc = if tpc == 0 || tpc > logical { 1 } else { tpc };
    // Hybrid (Intel 12th-gen+) breaks the logical/tpc division: E-cores are
    // single-threaded, so 20 threads / tpc=2 printed "10 cores" on a 14-core
    // i7-13650HX (6P+8E, field-caught 2026-11-12). Count physical cores
    // directly from the OS instead - RelationProcessorCore entries cover P
    // and E alike, across all groups. Self-validating walk; the division
    // stays as the fallback and SSDIAG_WALK_DEBUG exposes which path ran.
    let cores = match count_cores_slpiex() {
        Some(n) => {
            debug_walk(&format!("slpiex walk cores={n}"));
            n
        }
        None => {
            let c = logical / tpc;
            debug_walk(&format!("walk failed, division fallback cores={c}"));
            c
        }
    };
    let leaf1 = __cpuid_count(1, 0);
    let family = (leaf1.eax >> 8) & 0xF;
    let ext_family = (leaf1.eax >> 20) & 0xFF;
    let ext_model = (leaf1.eax >> 16) & 0xF;
    let model_low = (leaf1.eax >> 4) & 0xF;
    let display_family = if family == 0xF {
        family + ext_family
    } else {
        family
    };
    let display_model = if family == 0xF || family == 0x6 {
        (ext_model << 4) | model_low
    } else {
        model_low
    };
    let stepping = leaf1.eax & 0xF;
    Some(format!(
        "family {display_family} model {display_model:#04x} stepping {stepping}, {cores} cores / {logical} threads"
    ))
}

/// Physical-core count from GetLogicalProcessorInformationEx (Relation-
/// ProcessorCore=0). Layout settled EMPIRICALLY (ctypes dump on Win11
/// 26200, 2026-11-13): every EX record carries its own byte length at
/// offset +4 - core records are 48 bytes and 8 of them tile the 384-byte
/// buffer of the 8C/16T bench exactly (Numa/Package=48, Cache=56, Group=80,
/// all self-consistent). Walk by that field; no struct guessing. A guessed
/// GroupCount-based walk (v0.7.8 first cut) overran on every machine and
/// silently fell back to the division - which is indistinguishable from
/// success on uniform-SMT boxes and wrong only on hybrid, i.e. the worst
/// possible failure mode; the SSDIAG_WALK_DEBUG env hook exists so the
/// taken path is always observable. Builds whose records lack the size
/// field fail the walk and fall back to the logical/tpc division.
#[cfg(target_arch = "x86_64")]
fn count_cores_slpiex() -> Option<u32> {
    unsafe {
        let mut len: u32 = 0;
        // pass 1: size query (fails with ERROR_INSUFFICIENT_BUFFER by design)
        if win::GetLogicalProcessorInformationEx(0, core::ptr::null_mut(), &mut len) != 0
            || len == 0
            || len > 1 << 20
        {
            return None;
        }
        let mut buf = vec![0u8; len as usize];
        if win::GetLogicalProcessorInformationEx(0, buf.as_mut_ptr(), &mut len) == 0 {
            return None;
        }
        let len = len as usize;
        let mut off = 0usize;
        let mut cores = 0u32;
        while off + 8 <= len {
            let rel = u32::from_le_bytes(buf[off..off + 4].try_into().unwrap());
            let size = u32::from_le_bytes(buf[off + 4..off + 8].try_into().unwrap()) as usize;
            if size < 8 || off + size > len {
                return None;
            }
            if rel == 0 {
                cores += 1;
            }
            off += size;
        }
        if off == len && cores > 0 {
            Some(cores)
        } else {
            None
        }
    }
}

/// Opt-in observability for the core-count path (SSDIAG_WALK_DEBUG=1):
/// without it a walk failure is invisible whenever the division fallback
/// agrees on uniform-SMT machines - the exact trap the first 0.7.8 cut
/// fell into.
#[cfg(target_arch = "x86_64")]
fn debug_walk(msg: &str) {
    if std::env::var_os("SSDIAG_WALK_DEBUG").is_some() {
        eprintln!("WALK: {msg}");
    }
}

/// Decode the SMBIOS Type 17 Extended Size dword into MiB, three forms:
/// spec MiB (bit30 set), spec KiB (else, /1024), and the bare-MiB
/// convention the G614JV firmware uses (0x00008000 = 32GiB written as plain
/// 32768, no flag). Plausibility: real DIMMs are >=512 MiB, so junk like
/// the Z170 table's extended word of 1 decodes to None and callers keep
/// their legacy value.
fn ext_size_mib(ext: u32) -> Option<u32> {
    let v = ext & 0x3FFF_FFFF;
    if v == 0 {
        return None;
    }
    if ext & 0x4000_0000 != 0 {
        Some(v) // spec MiB form
    } else if v >= 512 {
        Some(v) // bare-MiB convention (ASUS DDR5 firmware)
    } else if v / 1024 >= 512 {
        Some(v / 1024) // spec KiB form
    } else {
        None // junk
    }
}

/// Slot capacity from the raw Size field (+ optional Extended Size), MiB.
/// The pure part of ram_summary's decode, kept standalone so the real
/// field bytes are unit-testable.
fn slot_size_mib(raw: u16, ext: Option<u32>) -> Option<u32> {
    match raw {
        0 => None, // slot empty
        0x7FFF => ext.and_then(ext_size_mib), // sentinel: value in the extended field
        0x8000..=0xFFFF => Some((raw & 0x7FFF) as u32), // already MiB
        _ => {
            // Field-observed AMI quirk (ASUS Z170, SMBIOS 3.0): 16 GiB DIMMs
            // written as 0x4000 - plain MiB WITHOUT the bit15 flag - so a
            // spec-literal KiB read yields "16 MiB". No module sold this
            // millennium is smaller than 128 MiB, so a KiB reading that
            // small is really MiB. When the extended slot decodes bigger,
            // the firmware under-reported the legacy field (a DDR5 gap
            // form) - take it.
            let kib = raw as u32;
            let legacy = if kib < 131_072 { kib } else { kib / 1024 };
            match ext.and_then(ext_size_mib) {
                Some(v) if v > legacy => Some(v),
                _ => Some(legacy),
            }
        }
    }
}

/// Populated SMBIOS Type 17 modules: slot locator, capacity, speed (the
/// configured speed when the firmware records one), manufacturer + part
/// number. Serials are deliberately NOT read. Offsets are the spec's
/// formatted area; string indexes point into the trailing string block.
fn ram_summary() -> (Option<u32>, Vec<String>) {
    let rsmb: u32 = 0x5253_4D42; // 'RSMB'
    let mut out = Vec::new();
    let need = unsafe { win::GetSystemFirmwareTable(rsmb, 0, core::ptr::null_mut(), 0) };
    if need == 0 {
        return (None, out);
    }
    let mut buf = vec![0u8; need as usize];
    if unsafe { win::GetSystemFirmwareTable(rsmb, 0, buf.as_mut_ptr(), need) } == 0 {
        return (None, out);
    }
    if buf.len() < 8 {
        return (None, out);
    }
    let data_len = u32::from_le_bytes(buf[4..8].try_into().unwrap_or([0; 4])) as usize;
    if data_len == 0 || buf.len() < 8 + data_len {
        return (None, out);
    }
    let data = &buf[8..8 + data_len];
    let mut slots = None;
    let mut off = 0usize;
    while off + 4 <= data.len() {
        let typ = data[off];
        let len = data[off + 1] as usize;
        if len < 4 || off + len > data.len() {
            break;
        }
        if typ == 16 && len >= 0x11 {
            // physical memory array: populated device count @0x0D
            slots = Some(u16::from_le_bytes(data[off + 0x0D..off + 0x0F].try_into().unwrap()) as u32);
        }
        if typ == 17 && len >= 0x17 {
            // AMI-shifted Type 17 layout, settled by raw byte dumps (Z170
            // 2026-11-12, G614JV 2026-11-13): spec puts Device Locator @0x0F
            // right after Form Factor @0x0E; this board family (ASUS Z170
            // AMI, Huananzhi X99, ASUS laptops - every table we have sampled)
            // pads 0x00 at 0x0F and shifts everything from the locator onward
            // by +1: Speed @0x15, Manufacturer @0x17, Part @0x1A, Extended
            // Size @0x1C, Configured Speed @0x20. Detect via the pad byte and
            // address both layouts - the shift applies to the extended-size
            // read too, which is what the G614JV "128 MiB" (2x32G DDR5)
            // tracing finally hinged on: its Size field is the 0x7FFF
            // sentinel, the real value sits in the shifted extended slot as
            // bare 0x8000 = 32768 MiB (32GiB, no bit30 flag), and the old
            // unshifted sentinel read at 0x1D stitched 0xC0000080 out of it
            // - whose bit30 happens to decode as the MiB form of exactly 128.
            let shift: usize = (data[off + 0x0F] == 0) as usize;
            let ext_at = 0x1B + shift;
            let ext_raw = if len >= ext_at + 4 {
                Some(u32::from_le_bytes(
                    data[off + ext_at..off + ext_at + 4].try_into().unwrap(),
                ))
            } else {
                None
            };
            let raw = u16::from_le_bytes(data[off + 0x0C..off + 0x0E].try_into().unwrap());
            let mib: Option<u32> = slot_size_mib(raw, ext_raw);
            if let Some(mib) = mib {
                let sp =
                    u16::from_le_bytes(data[off + 0x14 + shift..off + 0x16 + shift].try_into().unwrap());
                // Trust the configured value only when it is physically
                // plausible; everything real is >= 200 MT/s.
                let cs = if len >= 0x21 + shift {
                    u16::from_le_bytes(data[off + 0x1F + shift..off + 0x21 + shift].try_into().unwrap())
                } else {
                    0
                };
                let mt = if cs >= 200 { cs } else { sp };
                let locator = smbios_string(data, off + len, data[off + 0x0F + shift]);
                let man = smbios_string(data, off + len, data[off + 0x16 + shift]);
                let part = smbios_string(data, off + len, data[off + 0x19 + shift]);
                let mut line = format!(
                    "RAM SLOT: {} {} MiB",
                    if locator.is_empty() { "?" } else { &locator },
                    mib
                );
                if mt != 0 {
                    line.push_str(&format!(" @ {mt} MT/s"));
                }
                let tag = format!("{man} {part}").trim().to_string();
                if !tag.is_empty() {
                    line.push_str(&format!(" ({tag})"));
                }
                out.push(line);
            }
        }
        // skip the formatted area, then the double-NUL string set
        let mut p = off + len;
        let mut nul = 0usize;
        while p < data.len() && nul < 2 {
            if data[p] == 0 {
                nul += 1;
            } else {
                nul = 0;
            }
            p += 1;
        }
        off = p;
    }
    (slots, out)
}

const GUID_DEVINTERFACE_DISK: [u8; 16] = [
    0x07, 0x63, 0xf5, 0x53, 0xbf, 0xb6, 0xd0, 0x11, 0x94, 0xf2, 0x00, 0xa0, 0xc9, 0x1e, 0xfb, 0x8b,
];
const DISK_CLASS: &str = "{4d36e967-e325-11ce-bfc1-08002be10318}";
const IOCTL_DISK_GET_DRIVE_GEOMETRY: u32 = 0x0007_0000;
// CTL_CODE(0x2d, 0x420, METHOD_BUFFERED, FILE_ANY_ACCESS) - the misremembered
// 0x2d0048 is an unassigned function and fails with win32 1
const IOCTL_STORAGE_GET_DEVICE_NUMBER: u32 = 0x002D_1080;
const IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS: u32 = 0x0056_0000;

/// PnP devnodes of one setup class (CM_GETIDLIST_FILTER_CLASS).
fn enum_class(class: &str) -> Vec<String> {
    unsafe {
        let filter = wide(class);
        let mut len = 0u32;
        let cr = cm::CM_Get_Device_ID_List_SizeW(&mut len, filter.as_ptr(), cm::CM_GETIDLIST_FILTER_CLASS);
        if cr != cm::CR_SUCCESS || len == 0 {
            return Vec::new();
        }
        let mut buf = vec![0u16; len as usize];
        let cr = cm::CM_Get_Device_ID_ListW(filter.as_ptr(), buf.as_mut_ptr(), len, cm::CM_GETIDLIST_FILTER_CLASS);
        if cr != cm::CR_SUCCESS {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut rest: &[u16] = &buf;
        while let Some(nul) = rest.iter().position(|&c| c == 0) {
            let id_w = &rest[..nul];
            rest = &rest[nul + 1..];
            if id_w.is_empty() {
                break;
            }
            out.push(String::from_utf16_lossy(id_w));
        }
        out
    }
}

/// \\?\scsi#... interface paths of one devnode's disk interface.
fn disk_interface_paths(instance: &str) -> Vec<String> {
    let verbose = std::env::var_os("SSDIAG_VERBOSE_DISCOVERY").is_some();
    let mut id = wide(instance);
    id.push(0);
    let mut buf = [0u16; 2048];
    let cr = unsafe {
        cm::CM_Get_Device_Interface_ListW(
            GUID_DEVINTERFACE_DISK.as_ptr(),
            id.as_ptr(),
            buf.as_mut_ptr(),
            buf.len() as u32,
            0, // CM_GET_DEVICE_INTERFACE_LIST_PRESENT
        )
    };
    if cr != cm::CR_SUCCESS {
        if verbose {
            eprintln!("[discover] disk interface list CR_{cr:04x} for {instance}");
        }
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut rest: &[u16] = &buf;
    while let Some(nul) = rest.iter().position(|&c| c == 0) {
        let p = &rest[..nul];
        rest = &rest[nul + 1..];
        if p.is_empty() {
            break;
        }
        out.push(String::from_utf16_lossy(p));
    }
    if verbose {
        let first = out.first().map(|s| s.chars().take(48).collect::<String>()).unwrap_or_default();
        eprintln!("[discover] disk interface list for {instance}: {cr:04x} paths={} first={first:?}", out.len());
    }
    out
}

/// Open a device with zero access (works unelevated) and run one ioctl.
fn dev_ioctl(path: &str, ioctl: u32, out: &mut [u8]) -> bool {
    let verbose = std::env::var_os("SSDIAG_VERBOSE_DISCOVERY").is_some();
    let w = wide(path);
    let h = unsafe {
        win::CreateFileW(w.as_ptr(), 0, 3, ptr::null(), OPEN_EXISTING, 0, ptr::null())
    };
    if h as isize == -1 {
        if verbose {
            eprintln!("[discover] ioctl open failed ({path}): win32 {}", last_err());
        }
        return false;
    }
    let mut returned = 0u32;
    let ok = unsafe {
        win::DeviceIoControl(h, ioctl, ptr::null(), 0, out.as_mut_ptr().cast(), out.len() as u32, &mut returned, ptr::null())
    };
    if ok == 0 && verbose {
        eprintln!("[discover] ioctl {ioctl:#x} on {path}: win32 {}", last_err());
    }
    unsafe { win::CloseHandle(h) };
    ok != 0
}

fn fmt_size(bytes: u64) -> String {
    let mib = bytes >> 20;
    if mib >= 1024 {
        let gib_x10 = (mib * 10 / 1024) as u64;
        format!("{}.{} GiB", gib_x10 / 10, gib_x10 % 10)
    } else {
        format!("{mib} MiB")
    }
}

/// Disks with model + size, volumes mapped onto them via disk extents
/// (partition offset/size), filesystem/label/free. All queries work
/// unelevated (zero-access opens + buffered ioctls); no serials read.
fn disk_report() -> Vec<String> {
    let mut disks: Vec<(u32, String, u64)> = Vec::new(); // (number, name, bytes)
    for id in enum_class(DISK_CLASS) {
        let mut idw = wide(&id);
        idw.push(0);
        let mut devinst = 0u32;
        if unsafe { cm::CM_Locate_DevNodeW(&mut devinst, idw.as_mut_ptr(), 0) } != cm::CR_SUCCESS {
            continue; // phantom (card-swap style residue)
        }
        let name = hklm_string(&format!(r"{ENUM_ROOT}\{id}"), "FriendlyName") // the model line
            .or_else(|| cm_string(devinst, 0x0B)) // CM_DRP_FRIENDLYNAME
            .or_else(|| cm_string(devinst, cm::CM_DRP_DEVICEDESC))
            .unwrap_or_else(|| id.clone());
        let number = disk_interface_paths(&id).iter().find_map(|p| {
            let mut out = [0u8; 12];
            if dev_ioctl(p, IOCTL_STORAGE_GET_DEVICE_NUMBER, &mut out) {
                let n = u32::from_le_bytes(out[4..8].try_into().unwrap());
                if n != 0xFFFF_FFFF {
                    return Some(n);
                }
            }
            None
        });
        let Some(number) = number else {
            if std::env::var_os("SSDIAG_VERBOSE_DISCOVERY").is_some() {
                eprintln!("[discover] disk {id}: no storage device number resolved");
            }
            continue;
        };
        let mut geo = [0u8; 24];
        let bytes = if dev_ioctl(&format!(r"\\.\PhysicalDrive{number}"), IOCTL_DISK_GET_DRIVE_GEOMETRY, &mut geo) {
            // DISK_GEOMETRY: {LI cylinders @0, MEDIA_TYPE @8, tpc @12,
            // spt @16, bps @20} - the DWORDs sit one slot later than a
            // naive layout guess (field-verified: [8..12] reads the
            // MediaType, FixedMedia=12)
            let cyl = u64::from_le_bytes(geo[0..8].try_into().unwrap());
            let tpc = u32::from_le_bytes(geo[12..16].try_into().unwrap()) as u64;
            let spt = u32::from_le_bytes(geo[16..20].try_into().unwrap()) as u64;
            let bps = u32::from_le_bytes(geo[20..24].try_into().unwrap()) as u64;
            if std::env::var_os("SSDIAG_VERBOSE_DISCOVERY").is_some() {
                eprintln!("[discover] disk {id} -> #{number} cyl={cyl} tpc={tpc} spt={spt} bps={bps}");
            }
            cyl * tpc * spt * bps
        } else {
            if std::env::var_os("SSDIAG_VERBOSE_DISCOVERY").is_some() {
                eprintln!("[discover] disk {id} -> #{number}: geometry refused");
            }
            0
        };
        disks.push((number, name, bytes));
    }
    disks.sort_by_key(|(n, _, _)| *n);

    // volumes: extents give (disk, offset, length); letters/fs/free from the
    // volume APIs - every step unelevated
    let mut vols: Vec<(u32, i64, u64, String, String, String, Option<(u64, u64)>)> = Vec::new();
    let mut buf = [0u16; 512];
    let h = unsafe { win::FindFirstVolumeW(buf.as_mut_ptr(), buf.len() as u32) };
    if h as isize != -1 {
        let handle = h;
        loop {
            let nul = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            let vol = String::from_utf16_lossy(&buf[..nul]); // \\?\Volume{guid}\
            if !vol.is_empty() {
                let root_w = wide(&vol); // with trailing backslash: volume APIs
                let mut letters = [0u16; 512];
                let letters = if unsafe {
                    win::GetVolumePathNamesForVolumeNameW(root_w.as_ptr(), letters.as_mut_ptr(), letters.len() as u32, ptr::null_mut())
                } != 0 {
                    let n = letters.iter().position(|&c| c == 0).unwrap_or(0);
                    String::from_utf16_lossy(&letters[..n])
                        .split('\0')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.trim_end_matches('\\').to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                } else {
                    String::new()
                };
                let mut fs = [0u16; 64];
                let mut label = [0u16; 128];
                let (fs_s, label_s) = if unsafe {
                    win::GetVolumeInformationW(root_w.as_ptr(), label.as_mut_ptr(), label.len() as u32, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), fs.as_mut_ptr(), fs.len() as u32)
                } != 0 {
                    let f = {
                        let n = fs.iter().position(|&c| c == 0).unwrap_or(0);
                        String::from_utf16_lossy(&fs[..n])
                    };
                    let l = {
                        let n = label.iter().position(|&c| c == 0).unwrap_or(0);
                        String::from_utf16_lossy(&label[..n])
                    };
                    (f, l)
                } else {
                    (String::new(), String::new())
                };
                let mut free = 0u64;
                let mut total = 0u64;
                let mut tfree = 0u64;
                let sizes = if unsafe {
                    win::GetDiskFreeSpaceExW(root_w.as_ptr(), &mut free, &mut total, &mut tfree)
                } != 0 {
                    Some((free, total))
                } else {
                    None
                };
                // extents via the volume device (no trailing backslash)
                let dev_w = wide(vol.trim_end_matches('\\'));
                let vh = unsafe { win::CreateFileW(dev_w.as_ptr(), 0, 3, ptr::null(), OPEN_EXISTING, 0, ptr::null()) };
                if vh as isize == -1 {
                    if std::env::var_os("SSDIAG_VERBOSE_DISCOVERY").is_some() {
                        eprintln!("[discover] volume open failed ({vol}): win32 {}", last_err());
                    }
                }
                if vh as isize != -1 {
                    let mut out = [0u8; 1024];
                    let mut returned = 0u32;
                    let ok = unsafe {
                        win::DeviceIoControl(vh, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, ptr::null(), 0, out.as_mut_ptr().cast(), out.len() as u32, &mut returned, ptr::null())
                    };
                    unsafe { win::CloseHandle(vh) };
                    if ok != 0 && out.len() >= 8 {
                        // VOLUME_DISK_EXTENTS: ULONG count, then DISK_EXTENTS -
                        // which carry LARGE_INTEGERs, so extent[0] sits at
                        // offset 8 (4-byte count + 4-byte alignment pad), 24
                        // bytes each: {disk u32, offset i64, length i64}
                        let n = u32::from_le_bytes(out[0..4].try_into().unwrap()) as usize;
                        let n = n.min((out.len() - 8) / 24);
                        for e in 0..n {
                            let base = 8 + e * 24;
                            let dnum = u32::from_le_bytes(out[base..base + 4].try_into().unwrap());
                            let soff = i64::from_le_bytes(out[base + 8..base + 16].try_into().unwrap());
                            let slen = i64::from_le_bytes(out[base + 16..base + 24].try_into().unwrap());
                            if slen > 0 {
                                vols.push((dnum, soff, slen as u64, letters.clone(), fs_s.clone(), label_s.clone(), sizes));
                                break; // first extent = partition start
                            }
                        }
                    }
                }
            }
            if unsafe { win::FindNextVolumeW(handle, buf.as_mut_ptr(), buf.len() as u32) } == 0 {
                break;
            }
        }
        unsafe { win::FindVolumeClose(handle) };
    }

    let mut out = Vec::new();
    for (n, name, bytes) in &disks {
        out.push(format!(
            "DISK: #{n} {name} - {}",
            if *bytes > 0 { fmt_size(*bytes) } else { "size unknown".into() }
        ));
    }
    vols.sort_by_key(|(d, o, _, _, _, _, _)| (*d, *o));
    for (d, soff, slen, letters, fs, label, sizes) in &vols {
        let letter = if letters.is_empty() { "(no letter)" } else { &letters };
        let fs_s = if fs.is_empty() { "?" } else { fs };
        let mut line = format!(
            "PART: disk {d} @ {} size {} {letter} {fs_s}",
            fmt_size(*soff as u64),
            fmt_size(*slen)
        );
        if !label.is_empty() {
            line.push_str(&format!(" \"{label}\""));
        }
        if let Some((free, total)) = sizes {
            line.push_str(&format!(" free {} / {}", fmt_size(*free), fmt_size(*total)));
        }
        out.push(line);
    }
    out
}

/// Every PCI function on the bus (desc + ven:dev), sorted; capped so a
/// dense bus does not flood the report. Includes the NVIDIA companions
/// (audio/USB-C functions) already excluded from the GPU rows.
fn pci_lines() -> Vec<String> {
    let mut rows = Vec::new();
    if let Ok(ids) = enum_filtered("PCI") {
        for id in &ids {
            let mut idw = wide(id);
            idw.push(0);
            let mut devinst = 0u32;
            if unsafe { cm::CM_Locate_DevNodeW(&mut devinst, idw.as_mut_ptr(), 0) } != cm::CR_SUCCESS {
                continue;
            }
            let desc = cm_string(devinst, cm::CM_DRP_DEVICEDESC).unwrap_or_else(|| id.clone());
            let up = id.to_uppercase();
            let part = |tag: &str| -> String {
                up.find(tag)
                    .and_then(|i| up.get(i + 4..i + 8))
                    .filter(|s| s.bytes().all(|b| b.is_ascii_hexdigit()))
                    .map(|s| s.to_lowercase())
                    .unwrap_or_else(|| "????".into())
            };
            rows.push(format!("{} ({}:{})", desc, part("VEN_"), part("DEV_")));
        }
    }
    rows.sort();
    rows.dedup();
    let cap = 48;
    let mut out: Vec<String> = rows.iter().take(cap).map(|r| format!("PCI DEV: {r}")).collect();
    if rows.len() > cap {
        out.push(format!("PCI DEV: ... +{} more", rows.len() - cap));
    }
    out
}

/// Facts the advisories need once the header is printed.
struct InventorySummary {
    secure_boot: Option<u32>,
    chassis_laptop: bool,
}

/// Print the inventory block (stable `KEY:` line prefixes - the block is
/// what tools/collect_compat.py ingests) and return the advisory inputs.
fn inventory_header(gpus: &[Gpu]) -> InventorySummary {
    println!(
        "===== ssdiag inventory v{} (read-only; no admin, no driver) =====",
        env!("CARGO_PKG_VERSION")
    );
    // ProductName stays "Windows 10 ..." on many Win11 installs, so the major
    // is derived from the build number (>= 22000 = 11) and shown with
    // DisplayVersion + build.UBR.
    let build = hklm_string(OS_CURVER, "CurrentBuildNumber");
    let disp = hklm_string(OS_CURVER, "DisplayVersion")
        .or_else(|| hklm_string(OS_CURVER, "ReleaseId"));
    let ubr = hklm_dword(OS_CURVER, "UBR");
    let os_line = match &build {
        Some(b) => {
            let major: u32 = b.parse().unwrap_or(0);
            let win = if major >= 22000 {
                "Windows 11"
            } else if major > 0 {
                "Windows 10"
            } else {
                "Windows"
            };
            let dv = disp.as_deref().map(|v| format!(" {v}")).unwrap_or_default();
            match ubr {
                Some(u) => format!("{win}{dv} build {b}.{u}"),
                None => format!("{win}{dv} build {b}"),
            }
        }
        None => "unknown".into(),
    };
    println!("OS:      {os_line}");
    let cpu = hklm_string(CPU0_KEY, "ProcessorNameString").unwrap_or_else(|| "unknown".into());
    match cpu_topology() {
        Some(t) => println!("CPU:     {cpu} ({t})"),
        None => println!("CPU:     {cpu}"),
    }
    let sys_man = hklm_string(SMBIOS_BIOS_KEY, "SystemManufacturer").unwrap_or_default();
    let sys_prod = hklm_string(SMBIOS_BIOS_KEY, "SystemProductName").unwrap_or_default();
    let bb_man = hklm_string(SMBIOS_BIOS_KEY, "BaseBoardManufacturer").unwrap_or_default();
    let bb_prod = hklm_string(SMBIOS_BIOS_KEY, "BaseBoardProduct").unwrap_or_default();
    let board = if !sys_prod.is_empty() {
        format!("{sys_man} {sys_prod} (board: {bb_man} {bb_prod})")
    } else if !bb_prod.is_empty() {
        format!("{bb_man} {bb_prod}")
    } else {
        "unknown".to_string()
    };
    println!("BOARD:   {}", board.trim());
    let bios_v = hklm_string(SMBIOS_BIOS_KEY, "BIOSVersion").unwrap_or_else(|| "unknown".into());
    match hklm_string(SMBIOS_BIOS_KEY, "BIOSReleaseDate") {
        Some(d) => println!("BIOS:    {bios_v} ({d})"),
        None => println!("BIOS:    {bios_v}"),
    }
    let (chassis, ctype) = chassis_class().unwrap_or(("unknown", 0));
    println!("CHASSIS: {chassis} (SMBIOS chassis type {ctype})");

    // v0.7.1: memory, disks/partitions, other PCI devices
    let total = physical_total_mib();
    let (slots, slot_lines) = ram_summary();
    let slots_s = slots.map(|s| format!(", {s} SMBIOS slots")).unwrap_or_default();
    match total {
        Some(mib) => println!("RAM: {mib} MiB physical{slots_s}"),
        None => println!("RAM: unknown (GlobalMemoryStatusEx refused){slots_s}"),
    }
    for l in &slot_lines {
        println!("{l}");
    }
    if slot_lines.is_empty() {
        println!("RAM SLOT: none decoded (SMBIOS Type 17 absent or unreadable)");
    }
    // Under-report honesty note (DDR5 gap): when every populated slot's
    // decoded capacity totals far below the OS-visible physical total, the
    // firmware's Type 17 numbers cannot be trusted for capacity (field case:
    // 2x32G DDR5 SODIMMs reported as "128 MiB" each, G614JV 2026-11-12).
    // Say so instead of letting the bad numbers stand unmarked.
    if let Some(mib) = total {
        let slot_total: u32 = slot_lines
            .iter()
            .filter_map(|l| l.split_whitespace().nth(3).and_then(|w| w.parse::<u32>().ok()))
            .sum();
        if slot_total > 0 && slot_total as u64 * 4 < mib as u64 {
            println!(
                "RAM NOTE: SMBIOS slots total {slot_total} MiB but OS reports {mib} MiB - \
                 firmware under-reports slot capacity; trust the OS total"
            );
        }
    }
    for l in disk_report() {
        println!("{l}");
    }
    for l in pci_lines() {
        println!("{l}");
    }

    // nvapi VBIOS strings keyed by PCI bus number (handles are opaque)
    let nvapi_cards = nvapi_wake::vbios_by_bus().ok();

    for (i, g) in gpus.iter().enumerate() {
        let n = i + 1;
        let desc = cm_string(g.devinst, cm::CM_DRP_DEVICEDESC).unwrap_or_else(|| "unknown".into());
        println!("GPU {n}: {desc}");
        let devid = dev_id_from(&g.id);
        let mut pci_line = match devid {
            Some(d) => format!("10de:{d:04x}"),
            None => "10de:????".to_string(),
        };
        if let Some(s) = subsys_from(&g.id) {
            pci_line.push_str(&format!(" SUBSYS {s}"));
        }
        if let Some((b, d, f)) = g.bdf {
            pci_line.push_str(&format!(" BDF {b:02x}:{d:02x}.{f}"));
        }
        println!("GPU {n} PCI: {pci_line}");
        if let Some(d) = devid {
            let (fam, in_scope) = family_verdict_by_devid(d);
            let verdict = if in_scope {
                green("IN-SCOPE (unlock transaction family)")
            } else {
                red("OUT-OF-SCOPE")
            };
            println!("GPU {n} family: {fam} - {verdict}");
        }
        // VBIOS via nvapi bus match; unavailable without the NVIDIA driver
        let vbios = g
            .bdf
            .and_then(|(b, _, _)| {
                nvapi_cards
                    .as_ref()?
                    .iter()
                    .find(|c| c.bus == b)
                    .map(|c| c.vbios.clone())
            });
        match vbios {
            Some(v) => println!("GPU {n} VBIOS: {v} (nvapi)"),
            None => println!("GPU {n} VBIOS: unknown (nvapi unavailable or no bus match)"),
        }
        if let Some(cd) = card_driver(&g.id, g.devinst) {
            let mkt = marketing_driver_version(&cd.version)
                .map(|m| format!(" ({m})"))
                .unwrap_or_default();
            let date = cd.date.map(|d| format!(", dated {d}")).unwrap_or_default();
            let gsp = match cd.gsp {
                Some(1) => "EnableGpuFirmware = 1 (GSP firmware mode)".to_string(),
                Some(_) => red("EnableGpuFirmware = 0 (LEGACY mode)"),
                None => "EnableGpuFirmware unset (LEGACY mode)".to_string(),
            };
            println!("GPU {n} driver: {}{mkt}{date}; {gsp}", cd.version);
        } else {
            println!(
                "GPU {n} driver: no bound NVIDIA class key (driver not installed / never started this boot?)"
            );
        }
    }
    println!("===== end inventory =====");
    InventorySummary {
        secure_boot: hklm_dword(SECUREBOOT_STATE, "UEFISecureBootEnabled"),
        chassis_laptop: chassis.starts_with("laptop"),
    }
}

/// Pre-flight cautions derived from the inventory facts. Short by design -
/// each names its fix and where it is explained; none of these block
/// anything (this is a report, not a gate).
fn inventory_advisories(gpus: &[Gpu], inv: &InventorySummary) {
    let mut any = false;
    if inv.secure_boot == Some(1) {
        any = true;
        println!(
            "{}",
            yellow(
                "ADVISORY: Secure Boot ON - the EFI stick needs the FFS/driver form or a db-signed EFI; the in-OS ssdiag probe may not load here (field reports show it running on some SB-ON machines - just try it) (FAQ: Secure Boot)"
            )
        );
    }
    for g in gpus {
        let Some(d) = dev_id_from(&g.id) else { continue };
        let (fam, in_scope) = family_verdict_by_devid(d);
        if !in_scope {
            continue;
        }
        let Some(cd) = card_driver(&g.id, g.devinst) else { continue };
        if fam.starts_with("tu") && cd.gsp != Some(1) {
            any = true;
            println!(
                "{}",
                yellow(
                    "ADVISORY: Turing adapter in LEGACY driver mode - unlocking it WILL TDR (Code 43 / watchdog storm); run tools/windows/enable-turing-unlock.ps1 and fully RESTART first (a hibernate resume does not apply it)"
                )
            );
        }
    }
    if inv.chassis_laptop {
        any = true;
        println!(
            "{}",
            yellow(
                "ADVISORY: laptop chassis - the volatile unlock can drop on sleep/RTD3/MUX events; note the display mode (hybrid vs dGPU-direct) when reporting (FAQ: losing the unlock)"
            )
        );
    }
    if !any {
        println!("ADVISORY: none of the known pre-flight cautions apply");
    }
}

/// NUL-separated device instance IDs for one enumerator (`CM_Get_Device_ID_ListW`).
fn enum_filtered(enumerator: &str) -> Result<Vec<String>, String> {
    let filter = wide(enumerator);
    unsafe {
        let mut len = 0u32;
        let cr = cm::CM_Get_Device_ID_List_SizeW(
            &mut len,
            filter.as_ptr(),
            cm::CM_GETIDLIST_FILTER_ENUMERATOR,
        );
        if cr != cm::CR_SUCCESS || len == 0 {
            return Err(format!("list size CR_{cr:04x}"));
        }
        let mut buf = vec![0u16; len as usize];
        let cr = cm::CM_Get_Device_ID_ListW(
            filter.as_ptr(),
            buf.as_mut_ptr(),
            len,
            cm::CM_GETIDLIST_FILTER_ENUMERATOR,
        );
        if cr != cm::CR_SUCCESS {
            return Err(format!("list CR_{cr:04x}"));
        }
        let mut out = Vec::new();
        let mut rest: &[u16] = &buf;
        while let Some(nul) = rest.iter().position(|&c| c == 0) {
            if nul == 0 {
                break;
            }
            out.push(String::from_utf16_lossy(&rest[..nul]));
            rest = &rest[nul + 1..];
        }
        Ok(out)
    }
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
const ERROR_NOT_SAFEBOOT_SERVICE: u32 = 1084;
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_PATH_NOT_FOUND: u32 = 3;
const KEY_WRITE: u32 = 0x0002_0006;
const REG_SZ: u32 = 1;
const REG_OPTION_NON_VOLATILE: u32 = 0;
const RRF_RT_REG_SZ: u32 = 0x0000_0002;
const RRF_RT_REG_EXPAND_SZ: u32 = 0x0000_0004;

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

/// Multi-column cell value: badf reads colored red when ANSI is on, and
/// '!' replaces the '0x' prefix when it is not - a redirected/piped run or
/// a console without VT still shows the bucket at a glance, and the cell
/// stays exactly 10 characters either way. 0xffffffff (NO_DECODE) keeps the
/// plain form.

/// Offset column for `name = value` rows, house style: yellow 0x + cyan
/// digits (same as the PLM survey gutter). Always 10 visible chars.
fn off_h(off: u32) -> String {
    format!("{}{}", paint("96", "0x"), paint("93", &format!("{off:08x}")))
}

fn cell_val(v: u32) -> String {
    if badf_bucket(v).is_some() {
        if COLOR_ON.load(std::sync::atomic::Ordering::Relaxed) {
            red(&format!("{v:#010x}"))
        } else {
            format!("!{v:09x}") // '!' + 9 hex digits = 10 chars = 0x + 8
        }
    } else {
        format!("{v:#010x}")
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
        fn encode16(name: &str) -> Vec<u16> {
            name.encode_utf16().chain([0]).collect()
        }
        unsafe {
            // x64 NVIDIA installs ship the export library as
            // System32\nvapi64.dll; there is no System32\nvapi.dll (that
            // name only exists in SysWOW64 for 32-bit callers), so loading
            // by the bare name failed on every x64 machine (both #118 and
            // the second field report hit "nvapi.dll not loadable").
            // nvapi64.dll first, bare name as a legacy fallback.
            let mut lib = win::LoadLibraryW(encode16("nvapi64.dll").as_ptr());
            if lib.is_null() {
                lib = win::LoadLibraryW(encode16("nvapi.dll").as_ptr());
            }
            if lib.is_null() {
                return Err("nvapi64.dll not loadable - NVIDIA driver installed?".into());
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

    /// One read-only VBIOS fact per nvapi physical GPU.
    pub struct NvCardFact {
        pub bus: u32,
        pub vbios: String,
    }

    /// VBIOS strings keyed by PCI bus number (NvAPI handles are opaque; the
    /// bus id correlates them with the CfgMgr32 devnodes). Read-only; needs
    /// the NVIDIA driver present - callers degrade gracefully.
    /// Query ids from the nvapi-rs reverse table (the same source as the GC6
    /// control id above): GetBusId 0x1BE0B8E5, GetVbiosVersionString
    /// 0xA561FD7D (out param is an NvAPI_ShortString, 64 bytes).
    pub fn vbios_by_bus() -> Result<Vec<NvCardFact>, String> {
        fn encode16(name: &str) -> Vec<u16> {
            name.encode_utf16().chain([0]).collect()
        }
        type BusIdFn = unsafe extern "system" fn(u32, *mut u32) -> i32;
        type VbiosFn = unsafe extern "system" fn(u32, *mut i8) -> i32;
        unsafe {
            let mut lib = win::LoadLibraryW(encode16("nvapi64.dll").as_ptr());
            if lib.is_null() {
                lib = win::LoadLibraryW(encode16("nvapi.dll").as_ptr());
            }
            if lib.is_null() {
                return Err("nvapi64.dll not loadable - NVIDIA driver installed?".into());
            }
            let qip = win::GetProcAddress(lib, b"nvapi_QueryInterface\0".as_ptr());
            if qip.is_null() {
                return Err("nvapi_QueryInterface missing".into());
            }
            let qi: QueryInterfaceFn = std::mem::transmute(qip);
            let init: InitializeFn = std::mem::transmute(qi(0x0150_E828));
            if init() != NVAPI_OK {
                return Err("NvAPI_Initialize failed".into());
            }
            let enumf: EnumFn = std::mem::transmute(qi(0xE5AC_921F));
            let mut handles = [0u32; 64];
            let mut count = 0u32;
            if enumf(handles.as_mut_ptr(), &mut count) != NVAPI_OK {
                return Err("NvAPI_EnumPhysicalGPUs failed".into());
            }
            let bus_p = qi(0x1BE0_B8E5);
            let vbios_p = qi(0xA561_FD7D);
            if bus_p.is_null() || vbios_p.is_null() {
                return Err("VBIOS query ids not resolved by this driver version".into());
            }
            let busf: BusIdFn = std::mem::transmute(bus_p);
            let vbiosf: VbiosFn = std::mem::transmute(vbios_p);
            let mut out = Vec::new();
            for &h in &handles[..count.min(64) as usize] {
                let mut bus = 0u32;
                if busf(h, &mut bus) != NVAPI_OK {
                    continue;
                }
                let mut s = [0i8; 64];
                if vbiosf(h, s.as_mut_ptr()) != NVAPI_OK {
                    continue;
                }
                let bytes: Vec<u8> = s
                    .iter()
                    .take_while(|&&c| c != 0)
                    .map(|&c| c as u8)
                    .collect();
                let vbios = String::from_utf8_lossy(&bytes).to_string();
                if !vbios.is_empty() {
                    out.push(NvCardFact { bus, vbios });
                }
            }
            Ok(out)
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
            .map_err(|e| {
                if e == 5 {
                    map_denied_guidance();
                }
                format!("MAP_PHYS ioctl failed (win32 error {e})")
            })?;
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

/// The embedded payload's pinned-hash gate, shared by the deploy path and the
/// safe-mode staging (both refuse to touch anything on a mismatch).
fn verify_embedded_hash() -> Result<(), String> {
    let mut h = Sha256::new();
    h.update(PMXDRV_SYS);
    let got = format!("{:x}", h.finalize());
    if got != PMXDRV_SYS_SHA256 {
        return Err(format!("embedded pmxdrv.sys hash mismatch ({got}) - refusing to load"));
    }
    Ok(())
}

/// Extract + service-create + start. Service name is OURS (SSDIAG_PMXDRV) so
/// cleanup never touches a driver another tool deployed.
fn deploy_driver() -> Result<(), String> {
    verify_embedded_hash()?;

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
            if err == ERROR_NOT_SAFEBOOT_SERVICE {
                // Safe mode: the whitelist gate (SCM layer; the kernel's own
                // loader enforces the same list one floor down - ZwLoadDriver
                // documented to silently fail, modern builds 0xC000035F).
                // The check is NAME-based (the SafeBoot subkey's default
                // value is a label; no path data anywhere), so an in-session
                // self-heal is just: write the two whitelist keys and start
                // this very service again - the temp-path deploy is exactly
                // what a staged safe-mode run does. If the gate is
                // boot-snapshotted on this build the retry stays refused and
                // the message tells the user how to stage for real. The keys
                // persist beyond this session - only --safeboot-disarm
                // removes them; service + temp copy still die at exit.
                println!("StartService refused: win32 1084 (ERROR_NOT_SAFEBOOT_SERVICE) - safe-mode whitelist gate");
                println!("attempting in-session self-heal: writing the SafeBoot whitelist keys ...");
                let healed = stage_safeboot_keys().and_then(|()| {
                    if win::StartServiceW(svc, 0, ptr::null()) == 0 {
                        Err(format!("retry StartService failed: win32 {}", last_err()))
                    } else {
                        Ok(())
                    }
                });
                return match healed {
                    Ok(()) => {
                        println!("self-heal accepted - whitelist keys live, probe may proceed.");
                        println!("the service and its temp copy are removed on exit as usual; the two");
                        println!("registry keys persist: run 'ssdiag --safeboot-disarm' from a NORMAL-mode");
                        println!("elevated prompt when done (leaving them is a known EDR-flagged state).");
                        win::CloseServiceHandle(svc);
                        win::CloseServiceHandle(mgr);
                        Ok(())
                    }
                    Err(e) => {
                        win::DeleteService(svc);
                        win::CloseServiceHandle(svc);
                        win::CloseServiceHandle(mgr);
                        println!("self-heal refused: {e}");
                        println!("the whitelist is consumed at boot on this build - stage it for real:");
                        println!("  1. boot NORMAL mode, elevated: ssdiag --safeboot-arm");
                        println!("  2. reboot into Safe Mode, run ssdiag elevated");
                        println!("  3. back in NORMAL mode: ssdiag --safeboot-disarm");
                        Err("StartService failed: win32 1084 (safe-mode whitelist; see staging steps above)".into())
                    }
                };
            }
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
    // 50 ms grain over the same 2 s worst-case budget - the kernel releases
    // the image mapping with a small delay after the stop, and the coarse
    // 200 ms steps used to pay that delay in multiples
    for _ in 0..40 {
        if std::fs::remove_file(&sys_path).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
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
    // the kernel can take a beat between StartService and the device symlink;
    // 50 ms grain keeps the same 2 s worst-case budget but stops paying
    // multiples of 200 ms for a symlink that shows up in tens of ms
    for _ in 0..40 {
        if let Some(d) = PmxDrv::try_open() {
            return Ok(d);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    PmxDrv::try_open().ok_or_else(|| "service started but \\\\.\\PMXDRV never appeared".into())
}

// ---------------------------------------------------------------- safe mode staging

/// Safe mode starts only what is whitelisted under
/// `HKLM\SYSTEM\CurrentControlSet\Control\SafeBoot\<Minimal|Network>` - the
/// gate behind win32 1084 (ERROR_NOT_SAFEBOOT_SERVICE). Enforcement is
/// layered: SMSS/kernel for boot drivers at boot, SCM at StartService, and
/// the kernel's own loader for runtime loads (ZwLoadDriver documented to
/// return STATUS_SUCCESS when it silently declines in safe mode; modern
/// builds answer 0xC000035F STATUS_NOT_SAFE_MODE_DRIVER). All layers read
/// the same registry data, so staging the two subkeys is the one fix that
/// covers every door. See docs/diag-2.1.11-diag1-protocol.md §1b.
const SAFEBOOT_BASE: &str = r"SYSTEM\CurrentControlSet\Control\SafeBoot";
const SAFEBOOT_MODES: [&str; 2] = ["Minimal", "Network"];

/// Legacy (0.6.11) staged copy home - arm no longer writes there; only the
/// disarm path (and a cleanup sweep in arm) still look, so a state armed by
/// an older build gets fully removed instead of outliving its purpose.
fn armed_sys_path() -> std::path::PathBuf {
    let base = std::env::var_os("ProgramData")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("nv-uefi-unlocker").join("pmxdrv.sys")
}

fn safeboot_key_path(mode: &str) -> String {
    format!(r"{SAFEBOOT_BASE}\{mode}\{SERVICE_NAME}")
}

/// True when the Minimal whitelist entry reads back as "Service".
fn safeboot_armed() -> bool {
    reg_value_string(HKEY_LOCAL_MACHINE, &safeboot_key_path("Minimal"), ptr::null())
        .map(|v| v == "Service")
        .unwrap_or(false)
}

/// One string value via RegGetValueW; value name NULL reads the default.
fn reg_value_string(hkey: isize, subkey: &str, value: *const u16) -> Option<String> {
    let sub_w = wide(subkey);
    let mut buf = [0u16; 512];
    let mut cb = (buf.len() * 2) as u32;
    let cr = unsafe {
        reg::RegGetValueW(
            hkey,
            sub_w.as_ptr(),
            value,
            RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ,
            ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &mut cb,
        )
    };
    if cr != 0 {
        return None;
    }
    let chars = (cb as usize / 2).min(buf.len());
    let s = String::from_utf16_lossy(&buf[..chars]);
    let s = s.trim_end_matches('\0').to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// Create `SafeBoot\<mode>\<SERVICE_NAME>` with default value "Service" and
/// read it back - the read-back IS the verdict.
fn stage_safeboot_key(mode: &str) -> Result<(), String> {
    let path = safeboot_key_path(mode);
    let sub_w = wide(&path);
    let val_w = wide("Service");
    unsafe {
        let mut hk: isize = 0;
        let cr = reg::RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            sub_w.as_ptr(),
            0,
            ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            ptr::null(),
            &mut hk,
            ptr::null_mut(),
        );
        if cr != 0 {
            return Err(format!("RegCreateKey {path} failed: win32 {cr}"));
        }
        let cr = reg::RegSetValueExW(hk, ptr::null(), 0, REG_SZ, val_w.as_ptr(), (val_w.len() * 2) as u32);
        reg::RegCloseKey(hk);
        if cr != 0 {
            return Err(format!("RegSetValue {path} failed: win32 {cr}"));
        }
    }
    match reg_value_string(HKEY_LOCAL_MACHINE, &path, ptr::null()) {
        Some(v) if v == "Service" => Ok(()),
        other => Err(format!("{path} readback got {other:?}, want \"Service\"")),
    }
}

fn remove_safeboot_keys() -> Result<(), String> {
    for mode in SAFEBOOT_MODES {
        let path = safeboot_key_path(mode);
        let sub_w = wide(&path);
        let cr = unsafe { reg::RegDeleteTreeW(HKEY_LOCAL_MACHINE, sub_w.as_ptr()) };
        if cr != 0 && cr != ERROR_FILE_NOT_FOUND as i32 && cr != ERROR_PATH_NOT_FOUND as i32 {
            return Err(format!("RegDeleteTree {path} failed: win32 {cr}"));
        }
        if reg_value_string(HKEY_LOCAL_MACHINE, &path, ptr::null()).is_some() {
            return Err(format!("{path} still readable after delete"));
        }
    }
    Ok(())
}

/// Write BOTH whitelist subkeys with their read-backs. This is the entire
/// arm: the check is name-based, so no binary and no service are staged - the
/// safe-mode run extracts to %TEMP% and builds the service exactly as a
/// normal-mode run does, and both die at exit (remove-when-done). The only
/// persistent artifact is the pair of registry keys.
fn stage_safeboot_keys() -> Result<(), String> {
    for mode in SAFEBOOT_MODES {
        stage_safeboot_key(mode)?;
    }
    Ok(())
}

/// `--safeboot-arm`: registry-only staging - write the two whitelist keys,
/// sweep any legacy (0.6.11) staged copy, then prove the vehicle with a real
/// deploy/start/verify round in NORMAL mode using the SAME temp layout a
/// safe-mode run will use. The proof's service and temp copy are removed by
/// the normal exit cleanup; the only thing left behind is the key pair.
fn safeboot_arm() -> i32 {
    println!("safeboot-arm: staging the probe driver for Safe Mode (v0.6.12)");
    println!("lifecycle: registry-only - no binary, no service; the driver copy exists only for the");
    println!("seconds of each probe run (safe mode included), deleted on exit as in normal mode");
    let mgr = unsafe { win::OpenSCManagerW(ptr::null(), ptr::null(), SC_MANAGER_CREATE_SERVICE) };
    if mgr.is_null() {
        eprintln!("OpenSCManager failed (admin required): win32 {} - arm needs an elevated prompt", last_err());
        return 2;
    }
    unsafe { win::CloseServiceHandle(mgr) }; // only used as the admin probe
    if safeboot_armed() {
        println!("state: already armed - restaging (idempotent; verifies every piece)");
    }
    if let Err(e) = stage_safeboot_keys() {
        eprintln!("arm failed: {e}");
        return 1;
    }
    for mode in SAFEBOOT_MODES {
        println!("whitelist: {} = \"Service\" (readback ok)", safeboot_key_path(mode));
    }
    // legacy sweep: a 0.6.11 arm left a persistent copy + service behind;
    // neither is needed anymore, remove them on sight
    let legacy = armed_sys_path();
    if legacy.exists() {
        match std::fs::remove_file(&legacy) {
            Ok(()) => {
                if let Some(dir) = legacy.parent() {
                    let _ = std::fs::remove_dir(dir);
                }
                println!("legacy sweep: removed stale staged copy {}", legacy.display());
            }
            Err(e) => println!("legacy sweep: could not remove {} ({e}) - --safeboot-disarm will retry", legacy.display()),
        }
    }

    // proof round: the exact path a safe-mode run takes (temp extract ->
    // create service -> start -> device), so the user does not spend a
    // safe-mode boot on a vehicle this machine would refuse anyway
    println!("proof round: deploy -> start -> \\\\.\\PMXDRV (temp layout; removed on exit)");
    let mut start_ok = true;
    match ensure_driver() {
        Ok(_) => println!("proof: driver started and the device opened (vehicle works on this machine)"),
        Err(e) => {
            println!("proof failed: {e}");
            println!("the whitelist keys stay staged (inert without the vehicle) - fix the cause or disarm");
            start_ok = false;
        }
    }

    println!();
    println!("ARMED. Safe Mode procedure:");
    println!("  1. reboot into Safe Mode (bcdedit /set {{current}} safeboot minimal; BitLocker may ask for the key)");
    println!("  2. elevated: ssdiag (no flags) - the whitelist lets the vehicle start; discovery +");
    println!("     CANARY/POWER readback run as usual, and the service/temp copy are removed on exit");
    println!("  3. back in NORMAL mode, elevated: ssdiag --safeboot-disarm (removes the two keys)");
    println!("NOTE: SafeBoot whitelist entries are a known persistence technique (T1112); managed/EDR");
    println!("      machines may alert on this registry write - same class as the event-7045 note.");
    if !start_ok {
        return 1;
    }
    0
}

/// `--safeboot-disarm`: remove everything the arm (or self-heal) staged, with
/// a read-back proving each piece is gone.
fn safeboot_disarm() -> i32 {
    println!("safeboot-disarm: removing the Safe Mode staging");
    if !safeboot_armed() {
        println!("state: no Minimal whitelist entry found - nothing (or already) disarmed");
    }
    let mut rc = 0;
    match remove_safeboot_keys() {
        Ok(()) => println!("whitelist: SafeBoot\\Minimal+Network entries removed (readback: absent)"),
        Err(e) => {
            eprintln!("{e}");
            rc = 1;
        }
    }
    unsafe {
        let mgr = win::OpenSCManagerW(ptr::null(), ptr::null(), SC_MANAGER_CREATE_SERVICE);
        if mgr.is_null() {
            eprintln!("OpenSCManager failed (admin required): win32 {}", last_err());
            return 2;
        }
        let name_w = wide(SERVICE_NAME);
        let svc = win::OpenServiceW(mgr, name_w.as_ptr(), SERVICE_ALL_ACCESS);
        if !svc.is_null() {
            let _ = service_stop_and_wait(svc);
            win::DeleteService(svc);
            win::CloseServiceHandle(svc);
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        win::CloseServiceHandle(mgr);
        let gone = win::OpenServiceW(
            win::OpenSCManagerW(ptr::null(), ptr::null(), SC_MANAGER_CREATE_SERVICE),
            name_w.as_ptr(),
            SERVICE_ALL_ACCESS,
        );
        // read back through a fresh handle: the service must now not open
        if !gone.is_null() {
            eprintln!("service still opens after DeleteService - disarm incomplete");
            win::CloseServiceHandle(gone);
            rc = 1;
        } else {
            println!("service: SSDIAG_PMXDRV removed (readback: no longer opens)");
        }
    }
    let sys_path = armed_sys_path();
    match std::fs::remove_file(&sys_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            eprintln!("remove {} failed: {e}", sys_path.display());
            rc = 1;
        }
    }
    if let Some(dir) = sys_path.parent() {
        let _ = std::fs::remove_dir(dir); // best effort; only complains below if the file lingered
    }
    if sys_path.exists() {
        eprintln!("{} still present - disarm incomplete", sys_path.display());
        rc = 1;
    } else {
        println!("file: {} removed (readback: absent)", sys_path.display());
    }
    if rc == 0 {
        println!("disarm complete - zero residue");
    }
    rc
}


// ---------------------------------------------------------------- BAR0 discovery

/// One NVIDIA PCI function and its MMIO ranges this boot.
struct Gpu {
    /// Win32_PnPEntity DeviceID (identifies the function; safe for --select).
    id: String,
    /// sorted (start, end) memory ranges
    ranges: Vec<(u64, u64)>,
    /// PCI (bus, device, function) from the PnP bus/address properties, with
    /// the LocationInfo string as fallback - None if neither parsed.
    bdf: Option<(u32, u32, u32)>,
    /// devnode the discovery walk landed on; the POWER section walks up from
    /// it to the root port (cfgmgr32 rather than an ECAM bridge scan).
    devinst: u32,
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
fn discover_gpus() -> Result<(Vec<Gpu>, Vec<String>), String> {
    // Native CfgMgr32 walk (v0.6.1; was a powershell.exe spawn costing ~5 s
    // per run). Display-class filter keeps companion functions (audio/USB-C)
    // out; phantom (not-present) IDs fail CM_Locate_DevNodeW and drop out,
    // which is the same net filter the old "allocated resources" join had.
    // BusNumber/Address come from the devnode registry properties (the PCI
    // Address packs (device << 16) | function) with the LocationInformation
    // string as fallback - LocationInformation is NOT guaranteed to exist
    // (field-verified absent on one TU106 machine, where BusNumber was also
    // empty and the LocationInfo string saved the BDF), so every layer being
    // missing just means bdf stays None.
    const DISPLAY_CLASS: &str = "{4d36e968-e325-11ce-bfc1-08002be10318}";
    const VENDOR: &str = "VEN_10DE";
    unsafe {
        let filter = wide(DISPLAY_CLASS);
        let mut len = 0u32;
        let cr = cm::CM_Get_Device_ID_List_SizeW(
            &mut len,
            filter.as_ptr(),
            cm::CM_GETIDLIST_FILTER_CLASS,
        );
        if cr != cm::CR_SUCCESS {
            return Err(format!("device ID list size failed: CR_{cr:04x}"));
        }
        let mut buf = vec![0u16; len as usize];
        let cr = cm::CM_Get_Device_ID_ListW(
            filter.as_ptr(),
            buf.as_mut_ptr(),
            len,
            cm::CM_GETIDLIST_FILTER_CLASS,
        );
        if cr != cm::CR_SUCCESS {
            return Err(format!("device ID list failed: CR_{cr:04x}"));
        }

        let mut gpus: Vec<Gpu> = Vec::new();
        // Every dropped candidate plus why: an empty result is otherwise
        // indistinguishable between "no card", "not started" and "our query
        // is wrong", which is exactly the ambiguity issue #37 hit.
        let mut skipped: Vec<String> = Vec::new();
        // one NUL-terminated ID per entry, double NUL ends the list
        let mut rest: &[u16] = &buf;
        while let Some(nul) = rest.iter().position(|&c| c == 0) {
            let id_w = &rest[..nul];
            rest = &rest[nul + 1..];
            if id_w.is_empty() {
                break;
            }
            let id = String::from_utf16_lossy(id_w);
            if !id.to_uppercase().contains(VENDOR) {
                continue;
            }
            let mut id_mut = id_w.to_vec();
            id_mut.push(0);
            let mut devinst = 0u32;
            let cr = cm::CM_Locate_DevNodeW(&mut devinst, id_mut.as_mut_ptr(), 0);
            if cr != cm::CR_SUCCESS {
                skipped.push(format!("{id}: CM_Locate_DevNode CR_{cr:04x}"));
                continue;
            }

            // allocated (boot-assigned) hardware resources: walk every
            // descriptor, keep the memory ones. MEM_DES and MEM_LARGE_DES
            // share one layout {count u32, type u32, base u64, end u64, ...},
            // so a single parse covers 32-bit and >= 4 GiB descriptors alike.
            //
            // Priorities are tried in order because an unstarted device can
            // have no ALLOC config at all (safe mode); the first priority
            // that yields ranges wins.
            let mut ranges: Vec<(u64, u64)> = Vec::new();
            let mut conf_ret = cm::CR_SUCCESS;
            for prio in [
                cm::ALLOC_LOG_CONF,
                cm::BOOT_LOG_CONF,
                cm::FORCED_LOG_CONF,
                cm::OVERRIDE_LOG_CONF,
                cm::BASIC_LOG_CONF,
            ] {
                let mut logconf = 0usize;
                let cr = cm::CM_Get_First_Log_Conf(&mut logconf, devinst, prio);
                if cr != cm::CR_SUCCESS {
                    conf_ret = cr;
                    continue;
                }
                // one walk per memory descriptor type - CM_Get_Next_Res_Des takes
                // a concrete type on the first call (ResType_All is not accepted
                // there: CR_INVALID_RES_DES, field-verified) and the same type on
                // continuation; cur is only borrowed, never the logconf handle
                for want in [cm::RES_TYPE_MEM, cm::RES_TYPE_MEM_LARGE] {
                    let mut cur = logconf;
                    loop {
                        let mut rd = 0usize;
                        // the pResourceID out-param comes back 0 regardless of the
                        // requested type - and is unneeded: the walk is type-scoped
                        // by construction (first call + continuation both pass
                        // `want`)
                        let cr = cm::CM_Get_Next_Res_Des(
                            &mut rd,
                            cur,
                            want,
                            core::ptr::null_mut(),
                            0,
                        );
                        if cur != logconf {
                            cm::CM_Free_Res_Des_Handle(cur);
                        }
                        if cr != cm::CR_SUCCESS {
                            break; // NO_MORE_RES_DES ends the walk; anything else ends it too
                        }
                        cur = rd;
                        // one MEM_LARGE descriptor is 32 bytes; 256 is generous (the
                        // buffer is zero-filled, and a successful Mem descriptor read
                        // is >= 32 bytes by struct definition)
                        let mut data = [0u8; 256];
                        if cm::CM_Get_Res_Des_Data(rd, data.as_mut_ptr().cast(), 256, 0)
                            != cm::CR_SUCCESS
                        {
                            continue;
                        }
                        let base = u64::from_le_bytes(data[8..16].try_into().unwrap());
                        let end = u64::from_le_bytes(data[16..24].try_into().unwrap());
                        if base != 0 && end >= base {
                            ranges.push((base, end));
                        }
                    }
                }
                cm::CM_Free_Log_Conf_Handle(logconf);
                if !ranges.is_empty() {
                    break;
                }
            }
            // keep the node even without ranges: config-space recovery needs
            // the bdf, and a stopped device still has one
            if ranges.is_empty() {
                skipped.push(format!("{id}: no memory ranges (log conf CR_{conf_ret:04x})"));
            }
            ranges.sort();
            ranges.dedup();

            let bus = cm_dword(devinst, cm::CM_DRP_BUSNUMBER);
            let addr = cm_dword(devinst, cm::CM_DRP_ADDRESS);
            let loc = cm_string(devinst, cm::CM_DRP_LOCATION_INFORMATION);
            let bdf = match (bus, addr) {
                (Some(b), Some(a)) if b <= 0xFF => {
                    let d = (a >> 16) & 0x1F;
                    let fn_ = a & 0xF;
                    if fn_ <= 7 { Some((b, d, fn_)) } else { None }
                }
                _ => None,
            };
            let bdf = bdf.or_else(|| loc.as_deref().and_then(bdf_from_location));
            gpus.push(Gpu { id, ranges, bdf, devinst });
        }
        // deterministic order (the old CIM join had no ordering guarantee)
        gpus.sort_by(|x, y| x.id.cmp(&y.id));
        Ok((gpus, skipped))
    }
}

/// One DWORD devnode registry property; None when absent or another type.
fn cm_dword(devinst: u32, prop: u32) -> Option<u32> {
    let mut v = 0u32;
    let mut len = 4u32;
    let cr = unsafe {
        cm::CM_Get_DevNode_Registry_PropertyW(
            devinst,
            prop,
            core::ptr::null_mut(),
            (&mut v as *mut u32).cast(),
            &mut len,
            0,
        )
    };
    if cr == cm::CR_SUCCESS && len == 4 {
        Some(v)
    } else {
        None
    }
}

/// One string devnode registry property; None when absent or empty.
fn cm_string(devinst: u32, prop: u32) -> Option<String> {
    let mut buf = [0u16; 512];
    let mut len = (buf.len() * 2) as u32;
    let cr = unsafe {
        cm::CM_Get_DevNode_Registry_PropertyW(
            devinst,
            prop,
            core::ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &mut len,
            0,
        )
    };
    if cr != cm::CR_SUCCESS || len < 2 {
        return None;
    }
    let chars = (len as usize / 2).min(buf.len());
    let s = String::from_utf16_lossy(&buf[..chars]);
    let s = s.trim_end_matches('\0').trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

// ---------------------------------------------------------------- config-space ReBAR (ECAM)

const ACPI_PROVIDER: u32 = 0x4143_5049; // 'ACPI'
const MCFG_TABLE: u32 = 0x4746_434D; // 'MCFG'

/// Parse the device ID (DID) hex out of a PCI DeviceID string
/// ("PCI\VEN_10DE&DEV_2504&..." -> 0x2504).
fn dev_id_from(instance: &str) -> Option<u16> {
    let i = instance.to_uppercase().find("&DEV_")? + 5;
    u16::from_str_radix(instance.get(i..i + 4)?, 16).ok()
}

/// bus/device/function out of the registry LocationInformation
/// ("PCI bus 1, device 0, function 0" - the format string keeps the
/// bus/device/function argument order across UI languages, so the first
/// three integer runs are taken positionally).
fn bdf_from_location(loc: &str) -> Option<(u32, u32, u32)> {
    let mut nums: Vec<u32> = Vec::new();
    let mut cur: Option<u32> = None;
    for ch in loc.chars() {
        if ch.is_ascii_digit() {
            let v = cur.unwrap_or(0);
            cur = if v > 10_000 { None } else { Some(v * 10 + (ch as u32 - '0' as u32)) };
        } else if let Some(v) = cur.take() {
            nums.push(v);
        }
    }
    if let Some(v) = cur.take() {
        nums.push(v);
    }
    if nums.len() < 3 {
        return None;
    }
    let (bus, dev, func) = (nums[0], nums[1], nums[2]);
    if bus > 0xFF || dev > 0x1F || func > 0x7 {
        return None;
    }
    Some((bus, dev, func))
}

/// MCFG base address for segment 0 covering `bus`, or None (no MMCONFIG).
fn mcfg_base_for(bus: u32) -> Option<u64> {
    let need = unsafe {
        win::GetSystemFirmwareTable(ACPI_PROVIDER, MCFG_TABLE, core::ptr::null_mut(), 0)
    };
    if need == 0 {
        return None;
    }
    let mut buf = vec![0u8; need as usize];
    let got = unsafe {
        win::GetSystemFirmwareTable(ACPI_PROVIDER, MCFG_TABLE, buf.as_mut_ptr(), need)
    };
    if got == 0 || got as usize > buf.len() {
        return None;
    }
    let len = (u32::from_le_bytes(buf[4..8].try_into().ok()?) as usize).min(buf.len());
    if len < 44 {
        return None; // 36-byte ACPI header + 8 reserved, then 16-byte entries
    }
    let mut off = 44;
    while off + 16 <= len {
        let base = u64::from_le_bytes(buf[off..off + 8].try_into().ok()?);
        let seg = u16::from_le_bytes(buf[off + 8..off + 10].try_into().ok()?);
        let (sb, eb) = (buf[off + 10], buf[off + 11]);
        if seg == 0 && bus >= sb as u32 && bus <= eb as u32 {
            return Some(base);
        }
        off += 16;
    }
    None
}

/// Read-only ReBAR extended-capability probe through ECAM. The driver only
/// offers physical map/unmap, and config space on every modern x64 machine
/// IS memory-mapped (MCFG/ECAM), so the probe needs no new driver surface:
/// it maps exactly one 4 KiB page - the target function's own config space
/// (bdf resolved at discovery time from the PnP bus/address properties) and
/// validates the DeviceID's VID/DID before anything is decoded. Nothing
/// outside that page is ever touched (no bus scanning - unclaimed ECAM reads
/// are the one platform quirk this avoids). Register layout identical to the
/// loader's corrected decode (eb2bff5): {CAP, CTRL} pairs at +4/+8, count in
/// first CTRL[7:5], per-pair CTRL[2:0] index / CTRL[12:8] size (1 MiB << n),
/// CAP = supported-sizes bitmap (raw bit n = 2^n MB).
fn print_config_rebar(drv: &PmxDrv, instance: &str, bdf: Option<(u32, u32, u32)>) {
    let Some(did) = dev_id_from(instance) else {
        println!("config-space ReBAR: no DEV_ in DeviceID - skipped");
        return;
    };
    let Some((bus, dev, func)) = bdf else {
        println!(
            "config-space ReBAR: bdf not derivable from PnP bus/address properties - skipped"
        );
        return;
    };
    let Some(mcfg) = mcfg_base_for(bus) else {
        println!("config-space ReBAR: no MCFG/MMCONFIG for bus {bus} - unavailable on this firmware");
        return;
    };
    let ecam = mcfg + ((bus as u64) << 20) + ((dev as u64) << 15) + ((func as u64) << 12);
    let va = match drv.map_physical(ecam, 1) {
        Ok(va) => va,
        Err(e) => {
            println!("config-space ReBAR: ECAM map failed ({e}) - skipped");
            return;
        }
    };
    let base = va as *const u8;
    let rd = |off: u32| unsafe { (base.add(off as usize) as *const u32).read_volatile() };
    let idword = rd(0);
    let (vid, cdid) = (idword & 0xffff, (idword >> 16) & 0xffff);
    if vid != 0x10de || cdid != did as u32 {
        println!(
            "config-space ReBAR: ECAM page reads {vid:04x}:{cdid:04x}, expected 10de:{did:04X} - bdf mismatch, skipped"
        );
        drv.unmap_physical(va);
        return;
    }
    let mut off: u32 = 0x100;
    let mut found = false;
    let mut ff_stop: Option<u32> = None;
    let mut pcie_off: Option<u32> = None;
    for _ in 0..48 {
        // An SR-IOV VF's extended space (vfio-pci guest, e.g. geforce-sriov)
        // reads all-FF: cid=0xffff carries next=0xfff, which would step off to
        // 0xfff and read a u32 straddling the one mapped page - silent access
        // violation, nothing printed past the ranges block. Bare metal never
        // hits it (real chains end with next < 0x100 or cid == 0). Treat all-FF
        // as a terminator and keep every read inside [0x100, 0xffc].
        let hdr = rd(off);
        let cid = (hdr & 0xffff) as u16;
        if cid == 0 {
            break;
        }
        if cid == 0xffff {
            ff_stop = Some(off);
            break;
        }
        let next = (hdr >> 20) & 0xfff;
        if cid == 0x0010 && pcie_off.is_none() {
            pcie_off = Some(off);
        }
        if cid == 0x0015 {
            let first_ctrl = rd(off + 8);
            let n_bars = ((first_ctrl >> 5) & 0x7) as u32;
            println!(
                "config-space ReBAR (bus {bus} dev {dev} fn {func}): cap @{off:#06x} - {n_bars} resizable BAR(s)"
            );
            for b in 0..n_bars {
                let cap = rd(off + 4 + 8 * b);
                let ctrl = rd(off + 8 + 8 * b);
                let size_idx = (ctrl >> 8) & 0x1f;
                let mut supported = String::new();
                for bit in (4..=23).rev() {
                    if cap & (1 << bit) != 0 {
                        if !supported.is_empty() {
                            supported.push('/');
                        }
                        // raw bit (n+4) = 2^n MiB (linux bytes_to_size =
                        // log2(bytes) - 20); CTRL[12:8] size_idx likewise -
                        // field round 2: 1f07 reads size_idx 4/8/5 with live
                        // windows 16/256/32 MiB, confirming 2^n MiB
                        let mib = 1u64 << (bit - 4);
                        if mib >= 1024 {
                            supported.push_str(&format!("{}G", mib / 1024));
                        } else {
                            supported.push_str(&format!("{}M", mib));
                        }
                    }
                }
                println!(
                    "  BAR{} supported [{}] current {} MiB (size_idx {})",
                    ctrl & 0x7,
                    supported,
                    1u64 << size_idx,
                    size_idx
                );
            }
            found = true;
            break;
        }
        if next < 0x100 {
            break;
        }
        if next > 0xffc {
            ff_stop = Some(next);
            break;
        }
        off = next;
    }
    if let Some(w) = ff_stop {
        println!(
            "config-space ReBAR (bus {bus} dev {dev} fn {func}): extended config reads all-FF at {w:#06x} - SR-IOV VF / virtualized function, extended caps unreadable here"
        );
    } else if !found {
        println!(
            "config-space ReBAR (bus {bus} dev {dev} fn {func}): no 0x15 capability - card does not advertise resizable BAR (A4G/ReBAR board settings cannot resize this card's BAR1)"
        );
    }
    // Link capability line (field round 3): the OS-side Gen2 verification
    // pair. LnkSta is THE criterion (cmp170hx rule #1: LnkCap can read Gen2
    // while the link still trains Gen1; sysfs max_link_speed lies too).
    // Config offsets: LnkCap +0x0C, LnkCtl|LnkSta +0x10 (Sta in high word),
    // LnkCap2 +0x2C, LnkCtl2|LnkSta2 +0x30 (Sta2 in high word).
    // The PCIe capability usually lives in STANDARD space on NVIDIA (0x78,
    // not 0x60 - cmp170hx pcie-subsystem) - the extended walk above never
    // sees it. Walk the standard chain from the 0x34 pointer as fallback;
    // relative offsets (LnkCap +0x0C .. LnkCtl2 +0x30) are identical.
    if pcie_off.is_none() {
        let mut p = (rd(0x34) & 0xFF) as u32;
        for _ in 0..24 {
            if p == 0 || p > 0xFC {
                break;
            }
            let hdr = rd(p);
            let id = hdr & 0xFF;
            let next = (hdr >> 8) & 0xFF;
            if id == 0x10 {
                pcie_off = Some(p);
                break;
            }
            p = next;
        }
    }
    if let Some(poff) = pcie_off {
        let lnkcap = rd(poff + 0x0C);
        let ctl_sta = rd(poff + 0x10);
        let cap2 = rd(poff + 0x2C);
        let ctl2_sta2 = rd(poff + 0x30);
        let mls = lnkcap & 0xF;
        let mlw = (lnkcap >> 4) & 0x3F;
        let dlllarc = lnkcap & (1 << 20) != 0;
        let sta = (ctl_sta >> 16) & 0xFFFF;
        let cls = sta & 0xF;
        let nlw = (sta >> 4) & 0x3F;
        let tls = ctl2_sta2 & 0xF;
        let mut speeds = String::new();
        for bit in 1..=7u32 {
            if cap2 & (1 << bit) != 0 {
                if !speeds.is_empty() {
                    speeds.push(',');
                }
                speeds.push_str(&gen_name(bit));
            }
        }
        if speeds.is_empty() {
            speeds = "none".into();
        }
        let downgrade = if nlw != 0 && nlw != mlw { " (downgraded)" } else { "" };
        println!(
            "config-space link caps (bus {bus} dev {dev} fn {func}): \
LnkCap max {} x{mlw}{} | LnkSta {} x{nlw}{downgrade} | LnkCap2 [{speeds}] | LnkCtl2 TLS {}",
            gen_name(mls),
            if dlllarc { " DLLLARC+" } else { "" },
            gen_name(cls),
            gen_name(tls),
        );
    } else {
        println!("config-space link caps: no PCIe capability (0x10) found in the chain - skipped");
    }
    drv.unmap_physical(va);
}

/// PCIe speed nibble -> human name (LnkCap[3:0] / LnkSta[3:0] / LnkCtl2[3:0]
/// share the encoding; LnkCap2 uses bit n = Gen n).
fn gen_name(g: u32) -> String {
    match g {
        1 => "Gen1".into(),
        2 => "Gen2".into(),
        3 => "Gen3".into(),
        4 => "Gen4".into(),
        5 => "Gen5".into(),
        0 => "Gen1?0".into(),
        _ => format!("Gen?{g}"),
    }
}

// ---------------------------------------------------------------- POWER (D-state witness)

/// Device instance ID of a devnode (the POWER section prints the root port).
fn cm_device_id(devinst: u32) -> Option<String> {
    let mut buf = [0u16; 256];
    let cr = unsafe { cm::CM_Get_Device_IDW(devinst, buf.as_mut_ptr(), buf.len() as u32, 0) };
    if cr != cm::CR_SUCCESS {
        return None;
    }
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let s = String::from_utf16_lossy(&buf[..n]);
    if s.is_empty() { None } else { Some(s) }
}

/// PMCSR of one function as (raw, PowerState) - None when the config page is
/// unreadable or the chain carries no PM capability. `did` of None skips the
/// DeviceID check (bridges: the vendor ID alone identifies a real function).
fn read_pmcsr(drv: &PmxDrv, (bus, dev, func): (u32, u32, u32), did: Option<u16>) -> Option<(u32, u32)> {
    let mcfg = mcfg_base_for(bus)?;
    let ecam = mcfg + ((bus as u64) << 20) + ((dev as u64) << 15) + ((func as u64) << 12);
    let va = drv.map_physical(ecam, 1).ok()?;
    let base = va as *const u8;
    let rd = |off: u32| unsafe { (base.add(off as usize) as *const u32).read_volatile() };
    let idw = rd(0);
    let real = match did {
        Some(d) => idw & 0xffff == 0x10de && (idw >> 16) as u16 == d,
        None => (idw & 0xffff) != 0xffff && (idw & 0xffff) != 0,
    };
    if !real {
        drv.unmap_physical(va);
        return None;
    }
    let mut p = (rd(0x34) & 0xfc) as u32;
    let mut out = None;
    for _ in 0..24 {
        if p == 0 || p > 0xfc || p + 0x10 > 0x100 {
            break;
        }
        let hdr = rd(p);
        if hdr & 0xff == 0x01 {
            // PMCSR needs a version >= 2 capability (word PMCAP+2 bits [2:0]);
            // a non-zero power state is only meaningful once PME-capable
            let ver = rd(p + 2) & 0x7;
            if ver >= 2 {
                let raw = rd(p + 4);
                out = Some((raw, raw & 0x3));
            }
            break;
        }
        p = (hdr >> 8) & 0xfc;
    }
    drv.unmap_physical(va);
    out
}

fn dstate(s: u32) -> &'static str {
    match s {
        0 => "D0",
        1 => "D1",
        2 => "D2",
        _ => "D3",
    }
}

/// POWER section (v0.6.8, read-only): the card's PMCSR and the PMCSR of its
/// upstream root port. The platform SSDT puts `_PR3` on the PEG ports and
/// `_OFF` is PERST# + slot-rail-off (docs/evidence/tu106-1f07-acpi-dump/
/// ANALYSIS-ssdt02-power.md), so a port parked in D3 names a mechanism that
/// costs the unlock with no register write at all - and the same path is a
/// candidate for a card "vanishing" (#165). The PowerState bits [1:0] are
/// R/W, but nothing here writes them.
fn print_power_state(drv: &PmxDrv, devinst: u32, instance: &str, bdf: Option<(u32, u32, u32)>) {
    let card = match (dev_id_from(instance), bdf) {
        (Some(did), Some(bdf)) => read_pmcsr(drv, bdf, Some(did)),
        _ => None,
    };
    let card_msg = match card {
        Some((raw, s)) if s == 3 => format!("POWER: card PMCSR = {} ({raw:#06x})", red(dstate(s))),
        Some((raw, s)) => format!("POWER: card PMCSR = {} ({raw:#06x})", dstate(s)),
        None => format!(
            "POWER: card config space unreadable ({})",
            if bdf.is_none() { "no bdf" } else { "no answer / no PM capability" }
        ),
    };
    println!("{card_msg}");

    let mut parent = 0u32;
    if unsafe { cm::CM_Get_Parent(&mut parent, devinst, 0) } != cm::CR_SUCCESS {
        println!("POWER: root port: devnode has no parent - skipped");
        return;
    }
    let pid = cm_device_id(parent).unwrap_or_default();
    let (pbus, paddr) = (
        cm_dword(parent, cm::CM_DRP_BUSNUMBER),
        cm_dword(parent, cm::CM_DRP_ADDRESS),
    );
    let Some((b, a)) = pbus.zip(paddr) else {
        println!("POWER: root port {pid}: bus/address properties missing - skipped");
        return;
    };
    let pbdf = (b, (a >> 16) & 0x1F, a & 0xF);
    let tag = pid.strip_prefix("PCI\\").unwrap_or(&pid)
        .split("&SUBSYS").next().unwrap_or("")
        .to_string();
    match read_pmcsr(drv, pbdf, None) {
        Some((raw, 3)) => println!(
            "POWER: root port {:02x}:{:02x}.{} {tag} PMCSR = {} ({raw:#06x}) -> the platform _PR3/_OFF path is ENGAGED: PERST# asserted, slot rail off - the unlock cannot survive this state",
            pbdf.0, pbdf.1, pbdf.2, red(dstate(3))
        ),
        Some((raw, s)) => println!(
            "POWER: root port {:02x}:{:02x}.{} {tag} PMCSR = {} ({raw:#06x}) -> rail intact (only D3cold of the port runs _OFF)",
            pbdf.0, pbdf.1, pbdf.2, dstate(s)
        ),
        None => println!(
            "POWER: root port {:02x}:{:02x}.{} {tag} PMCSR unreadable - skipped",
            pbdf.0, pbdf.1, pbdf.2
        ),
    }
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
/// Blackwell GB20x uses the *same* 0x823xxx bank as Ampere/Ada (issue #168,
/// GB206 `10de:2d04`: PLM `0xfffffb8f`, SS0 `0x06277073`, SS1 `0x00025005`,
/// READOUT_1 `0x00c36180`). Kept as its own table because the readout decode
/// and the rate law differ (§1 of docs/blackwell-gb20x-status.md) and the
/// transaction can never run on this die (no GB20x SEC2 booter).
const FEAT_BLACKWELL: [(&str, u32); 4] = [
    ("PLM", 0x0082_3804),
    ("SS0", 0x0082_381C),
    ("SS1", 0x0082_3820),
    ("READOUT_1", 0x0082_3818),
];
/// Blackwell-only read-only surface: the fuse selectors behind the bank, the
/// override master switch, the Quadro secure-write word, the inert issue-rate
/// modifier and the empty SEC2 PLM. On Ampere/Ada these read different (or are
/// absent) values, so they are printed only for a blackwell die.
const BLACKWELL_ROWS: [(&str, u32); 12] = [
    ("FUSE_EN_SW_OVERRIDE", 0x0082_0040),
    ("FUSE_SS_PLM", 0x0082_00FC),
    ("SECURE_FEAT_OVR_QUADRO_WR", 0x0082_038C),
    ("FUSE_SS_DP", 0x0082_0224),
    ("FUSE_SS_FFMA", 0x0082_059C),
    ("FUSE_SS_FMLA16", 0x0082_07D4),
    ("FUSE_SS_FMLA32", 0x0082_07D8),
    ("FUSE_SS_IMLA0", 0x0082_07DC),
    ("FUSE_SS_IMLA1", 0x0082_07E0),
    ("FUSE_SS_IMLA2", 0x0082_07E4),
    ("FUSE_SS_IMLA3", 0x0082_07E8),
    ("FUSE_SS_IMLA4", 0x0082_07EC),
];
/// `FEAT_READOUT_1` is NOT nine contiguous 3-bit fields: the fields sit at
/// these bit offsets, with gaps at bit 6 and bits 27:25 (measured on GB202,
/// reproduced byte-identically on GB206/GB203M). The contiguous decode yields
/// out-of-range 6s. Field order imla0 fmla16 fmla32 ffma imla1 imla2 imla3
/// imla4 DP.
const READOUT1_SHIFT: [u32; 9] = [0, 3, 7, 10, 13, 16, 19, 22, 28];
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

/// Reset canary (issues #164/#165). The loader writes these two PBUS
/// software-scratch words AFTER its FLR; any FLR, SBR or rail-off clears
/// them, and the open driver never writes this pair (only index 0x1E, in the
/// drain path). Read together with the FEAT values: canary intact + SS gone
/// = a plain write with no reset; canary cleared + SS intact = a reset the
/// AON island survived (FLR/SBR class); both gone = rail-off / full-chip
/// reset, or a write that merely looks like one.
///
/// Offsets follow `NV_PBUS_SW_SCRATCH(i) = 0x1400 + i*4` (dev_bus.h; same
/// formula on Turing/Ampere/Ada), i.e. 0x1480/0x1484 = indices 0x20/0x21.
/// The first build used 0x140020/0x140024 - a mix-up with the register
/// *block* base 0x140000 - and read 0xbadf5040 even from a healthy chip.
const CANARY_A: u32 = 0x0000_1480;
const CANARY_B: u32 = 0x0000_1484;
const CANARY_MAGIC_A: u32 = 0x0164_C0DE;
const CANARY_MAGIC_B: u32 = 0x0165_CAFE;

/// `intact` / `cleared` / `partial` / `other` / `unreadable`. `partial` is a
/// half-surviving pair - a narrow-domain reset, or a devinit that rewrote
/// one of the two words.
fn canary_state(rd: &dyn Fn(u32) -> u32) -> &'static str {
    let (a, b) = (rd(CANARY_A), rd(CANARY_B));
    let poison = |v: u32| v == 0xFFFF_FFFF || v >> 16 == 0xBADF;
    if poison(a) && poison(b) {
        return "unreadable";
    }
    match (a == CANARY_MAGIC_A, b == CANARY_MAGIC_B) {
        (true, true) => "intact",
        (false, false) if a == 0 && b == 0 => "cleared",
        (false, false) => "other",
        _ => "partial",
    }
}
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
/// Ada fuse-controller / TPC-disable block (KohakUnlock issue #16 map, 2026-11-02:
/// 12 fuse PLMs live at 0x8200d0..fc on AD10x — unlike GA100's 0x820050/54;
/// DEBUGCTRL bit1 switches FUSECTRL to the RIR macro per ga100_rir.c). Read-only
/// probe for the soft-disable question: cut-down TPCs with DEFECT=0 are
/// candidates for the _CP shadow path (GA100 precedent) — see
/// docs/open-questions.md. badf on non-Ada dies is expected and is data.
/// The per-GPC TPC words themselves are in the `tpc` section (dump_tpc).
const ADA_FUSE_ROWS: [(&str, u32); 9] = [
    ("FUSE_CTRL", 0x0082_0000),
    ("FUSE_ADDR", 0x0082_0004),
    ("FUSE_REG_50", 0x0082_0050),
    ("FUSE_REG_54", 0x0082_0054),
    ("FUSE_DEBUGCTRL", 0x0082_0060),
    ("DISABLE_FUSE_PROG", 0x0082_007C),
    ("OPT_GPC_DISABLE", 0x0082_0350),
    ("OPT_GPC_DEFECTIVE", 0x0082_05C4),
    ("CTRL_OPT_9C8", 0x0082_09C8),
];
/// Per-GPC TPC floorsweep surface. STATUS at 0x820c38+4g is the effective
/// fused mask (bit n = TPC n of GPC g), CTRL at 0x820838+4g the writable
/// override — exactly the pair nvgpu drives in tpc_gv11b.c (write CTRL, poll
/// STATUS until equal), so the semantics are driver-sourced, not inferred.
/// The OPT/DEFECT banks are printed raw: 0x820350/0x8205c4 are documented
/// GPC-level masks (gpcprobe/Consensus-Protocol, 15-card validation), while
/// a per-GPC assignment inside the banks is still probe-derived
/// (KohakUnlock#16) — see docs/open-questions.md #13.
const TPC_CTRL_BASE: u32 = 0x0082_0838;
const TPC_STATUS_BASE: u32 = 0x0082_0C38;
const TPC_BANK_A: u32 = 0x0082_03A0;
const TPC_BANK_B: u32 = 0x0082_05C0;
const OPT_GPC_DISABLE: u32 = 0x0082_0350;
const OPT_GPC_DEFECTIVE: u32 = 0x0082_05C4;
const TPC_MAX_GPC: u32 = 8;
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
/// PCIe link block, BAR0 mirrors of XVE config space (field round 3 union:
/// cmp50hx patch04 + cmpunlocker 0007 + gpu-unlock-bar1 + AD102 L2 research).
/// XVE shadow base 0x88000, so config CAP_EXP+0x0C (LnkCap) maps to 0x88084
/// and so on; the policy registers are the endpoint-side half of the CMP
/// mining-card Gen2 unlock (docs/cmp-gen2-sm-unlock-assessment.md §1). On a
/// consumer die these read stock/benign - that IS the family answer.
const PCIE_LINK_ROWS: [(&str, u32); 16] = [
    ("LNKCAP", 0x0008_8084),
    ("LNKCTL_LNKSTA", 0x0008_8088),
    ("LNKCAP2", 0x0008_80A4),
    ("LNKCTL2_LNKSTA2", 0x0008_80A8),
    ("PRIV_MISC_1", 0x0008_841C),
    ("VSEC_DEVICE", 0x0008_860C),
    ("VSEC_HIERARCHY", 0x0008_8610),
    ("XVE_OVR_LTSSM", 0x0008_872C),
    ("XVE_B_PLM", 0x0008_8AB4),
    ("XVE_D0_PLM", 0x0008_8FE8),
    ("XVE_D4_PLM", 0x0008_8FEC),
    ("XVE_D8_PLM", 0x0008_8FF0),
    ("XVE_C_PLM", 0x0008_8FF8),
    ("LINK_CONFIG_0", 0x0008_C040),
    ("PL_LINK_RATE", 0x0008_C1C0),
    ("CYA_0_DISG2", 0x0008_C2C0),
];
/// XP3G fuse-override block: status base 0x8e100, override-enable 0x8e110,
/// value 0x8e120 (slot n at base+4n), PLMs 0x8e1b0..bc. The Gen2 capability
/// layer - XP3G_PLM0 must read OPEN before any policy write can land.
const XP3G_ROWS: [(&str, u32); 10] = [
    ("XP3G_STATUS0", 0x0008_E100),
    ("XP3G_STATUS3", 0x0008_E10C),
    ("XP3G_OVR0", 0x0008_E110),
    ("XP3G_OVR3", 0x0008_E11C),
    ("XP3G_VAL0", 0x0008_E120),
    ("XP3G_VAL3", 0x0008_E12C),
    ("XP3G_PLM0", 0x0008_E1B0),
    ("XP3G_PLM4", 0x0008_E1B4),
    ("XP3G_PLM8", 0x0008_E1B8),
    ("XP3G_PLMC", 0x0008_E1BC),
];
/// Fuse/OPT region union: the Gen2 fuse wall (OPT_GEN23 = pure OTP sense,
/// no write port; OPT_GEN3/MAGIC logged-never-written by 0007), the
/// GPC-disable shadow, the AD102 L2 disable-mask layer (aajax.top research:
/// effective = fuse layer | SW mask), plus where the consumer TPC/GPC
/// disable would live if a PR#55-style shadow override exists at all.
const OPT_FUSE_ROWS: [(&str, u32); 7] = [
    ("OPT_GPC_DISABLE", 0x0082_0350),
    ("OPT_MAGIC", 0x0082_0520),
    ("OPT_GEN23", 0x0082_057C),
    ("OPT_GEN3", 0x0082_0580),
    ("L2_CANDIDATE", 0x0082_1200),
    ("L2_SW_MASK", 0x0082_2080),
    ("L2_EFF_MASK", 0x0082_2900),
];
/// cmpunlocker PR#55's GA100 per-GPC SM override/status banks (0xff to
/// 0x820a40+4*gpc, status at 0x820c38+4*gpc). On 20-40 series expect badf -
/// the existence/nonexistence answer per die, read-only.
const GA100_SM_OVR_BASE: u32 = 0x0082_0A40;
const GA100_SM_STATUS_BASE: u32 = 0x0082_0C38;

/// GA100 VFI dictionary round (docs/reading-notes.md "GA100 电压故障注入
/// (VFI) 考据", 2026-11-11). kate-land/ga100vfi published the GA100
/// fuse-controller map and the block is cross-family shared IP - the
/// published `NV_FUSE_OPT_FEATURE_FUSES_OVERRIDE_DISABLE` sits at 0x8203f0
/// exactly like our TU106 PRIV_ROWS word, and the SM_SPEED_SELECT offsets
/// match BLACKWELL_ROWS bit-for-bit - so the whole dictionary is a Rosetta
/// probe for consumer dies. badf5040 = word exists behind a PLM; a plain
/// value = read it with GA100 semantics; 0xffffffff/badf1100 = not decoded
/// here (the AD10x fuse-PLM relocation precedent says absence != relocated
/// never). `NV_FUSE_OPT_` prefix dropped throughout.
const GA100_FUSECTL_ROWS: [(&str, u32); 13] = [
    ("FUSECTRL", 0x0082_0000),
    ("FUSEADDR", 0x0082_0004),
    ("EN_SW_OVERRIDE", 0x0082_0040),
    ("FUSEDEBUG", 0x0082_0060),
    ("CRC_STATUS", 0x0082_0070),
    ("ENABLE_FUSE_PROG_STAT", 0x0082_0078),
    ("DISABLE_FUSE_PROG_STAT", 0x0082_007C),
    ("DISABLE_SW_OVR_STAT", 0x0082_0084),
    ("FUSEWDATA_PLM", 0x0082_00F4),
    ("FUSECTRL_PLM", 0x0082_00F8),
    ("FUSEOPT_PLM", 0x0082_00FC),
    ("RING_GLOBAL_CTL", 0x0012_0060),
    ("RING_RESET_PLM", 0x0012_00A0),
];
/// The 17 row0/1-derived security aliases behind the "goldilocks" glitch
/// state (glitch-loop/EXAMPLES.md) plus the two words the write route needs
/// (SECURE_FUSE_WDATA_WR_SECURE is the last PLM the GA100VFI chain clears;
/// DEVID_SW_OVERRIDE_DIS gates the PCI ID override). FEATURE_FUSES_OVERRIDE
/// _DISABLE is already probed as PRIV_ROWS FUSE_OPT_FEATURE_OVR (0x8203f0,
/// the identity that started this round).
const GA100_SEC_ALIAS_ROWS: [(&str, u32); 20] = [
    ("DEBUG_FEAT_SW_OVR_EN", 0x0082_0148),
    ("IST_DEBUG_FA", 0x0082_0158),
    ("PRIV_SEC_EN", 0x0082_0434),
    ("SECURE_PRI_SRC_ISO_EN", 0x0082_0410),
    ("PRIVKEY_ACC_DIS", 0x0082_0324),
    ("FEATURE_FUSES_OVR_DIS", 0x0082_03F0),
    ("SFK_ACC_DIS", 0x0082_05A8),
    ("SECURE_PMU_DEBUG_DIS", 0x0082_0640),
    ("SECURE_SECENG_DEBUG_DIS", 0x0082_063C),
    ("SECURE_NVDEC_DEBUG_DIS", 0x0082_0644),
    ("SECURE_MINION_DEBUG_DIS", 0x0082_06FC),
    ("SECURE_GSP_DEBUG_DIS", 0x0082_074C),
    ("HDCP_RAM_ACCESS_DIS", 0x0082_07F4),
    ("DEBUG_SW_OVR_EN", 0x0082_0E20),
    ("XV2CLK_RETIME_DIS", 0x0082_1064),
    ("SECURE_FUSE_RESENSE_DIS", 0x0082_1058),
    ("SECURED_BURN_FEAT_EN", 0x0082_1108),
    ("SECURE_XUSB_DEBUG_DIS", 0x0082_0B98),
    ("SECURE_FUSE_WDATA_WR_SEC", 0x0082_048C),
    ("DEVID_SW_OVR_DIS", 0x0082_0584),
];
/// Fused per-op-class SM speed selects - the fuse-level siblings of our
/// runtime SS0/SS1 (`FEAT_OVR_SM_SPD`): same segmentation semantics, two
/// implementation layers. Offsets identical to BLACKWELL_ROWS FUSE_SS_*;
/// this table prints for every die so Turing/Ampere/Ada get the answer too.
const GA100_SPEED_SEL_ROWS: [(&str, u32); 9] = [
    ("SM_SPEED_SELECT_DP", 0x0082_0224),
    ("SM_SPEED_SELECT_FFMA", 0x0082_059C),
    ("SM_SPEED_SELECT_FMLA16", 0x0082_07D4),
    ("SM_SPEED_SELECT_FMLA32", 0x0082_07D8),
    ("SM_SPEED_SELECT_IMLA0", 0x0082_07DC),
    ("SM_SPEED_SELECT_IMLA1", 0x0082_07E0),
    ("SM_SPEED_SELECT_IMLA2", 0x0082_07E4),
    ("SM_SPEED_SELECT_IMLA3", 0x0082_07E8),
    ("SM_SPEED_SELECT_IMLA4", 0x0082_07EC),
];
/// STATUS_OPT_* effective-value scalars (the words the GA100VFI loop
/// monitors for glitch-state classification). Arrays print as bases below.
const GA100_STATUS_OPT_ROWS: [(&str, u32); 20] = [
    ("STAT_HALF_FBPA", 0x0082_0C00),
    ("STAT_DISPLAY", 0x0082_0C04),
    ("STAT_DISP_HEAD", 0x0082_0C08),
    ("STAT_NVJPG", 0x0082_0C0C),
    ("STAT_HALF_LTC", 0x0082_0C10),
    ("STAT_FBIO", 0x0082_0C14),
    ("STAT_FBPA", 0x0082_0C18),
    ("STAT_GPC", 0x0082_0C1C),
    ("STAT_PERLINK", 0x0082_0C20),
    ("STAT_NVDEC", 0x0082_0C24),
    ("STAT_SEC", 0x0082_0C28),
    ("STAT_PCIE_LANE", 0x0082_0C2C),
    ("STAT_SPARE_FS", 0x0082_0C30),
    ("STAT_FB_CONFIG", 0x0082_0C34),
    ("STAT_FBP", 0x0082_0D38),
    ("STAT_NVENC", 0x0082_0D68),
    ("STAT_GC6_ISLAND", 0x0082_0D6C),
    ("STAT_NVLINK", 0x0082_0DB8),
    ("STAT_SYS_PIPE", 0x0082_0E30),
    ("STAT_OFA", 0x0082_0E34),
];
/// STATUS_OPT_* arrays (GA100 16 PES words, 16 ROP/L2 words, 32 per-GPC TPC
/// mask words). TPC words 0..7 sit on dump_tpc's STATUS base - that section
/// only runs for ampere/ada, this one covers every die, which is the point.
const GA100_PES_STATUS_BASE: u32 = 0x0082_0DBC;
const GA100_ROP_L2_STATUS_BASE: u32 = 0x0082_0D70;
const GA100_TPC_STATUS_BASE: u32 = 0x0082_0C38;

/// Issue #237 round (JeusChrit, 595.44.15 GSP + VBIOS disassembly, AD103
/// 6-page capture; docs/evidence/ad103-2704/): GPC-indexed arrays run
/// gpc = 0..11 on AD102-class dies (12 GPC slots) - all GPC arrays here now
/// sweep 12. New faces, disassembly-sourced: per-GPC ROP mask (Graphics-
/// Manager +0x138, query type 10), TPC RECONFIG RAM (RPR v2; note our old
/// GA100-table guess 0x820788+4g was the GA100 layout - the Ada/595 layout
/// is 0x822800+4g, per-FBP words already covered), the VF/Speedo trims
/// (ClockManager +0x2f0, 10-bit words = signed 6-bit + 4-bit fields - the
/// UV-relevant knobs, issue #147), the VBIOS app08 override-pair region
/// (0x6ae4 writes 64 (Data, Control|=1) pairs at PL2 after checking
/// 0x823b24/28 bit6 - which also settles the stock EN_SW_OVERRIDE=1
/// mystery: app08 writes 0x820040=1 itself on every boot), and the PLM
/// write-enable bits (0x8200f8 bit4 = PL0 write enable, checked by the
/// TU102/TU116 OBJBSP writer 0x215ca0; 0x823b24/28 bit6 = PL2).
const AD10X_ROP_PER_GPC_BASE: u32 = 0x0082_2880;
const AD10X_TPC_RECONFIG_BASE: u32 = 0x0082_2800;
const CLK_VF_SPEEDO_TRIMS: [u32; 4] = [0x0082_13A0, 0x0082_13A4, 0x0082_13A8, 0x0082_13AC];
const VBIOS_OVR_PAIR_REGIONS: [(u32, u32); 2] = [(0x0082_2148, 23), (0x0082_2300, 41)];
const PLM_PL0_WE: u32 = 0x0082_00F8; // bit4 = PL0 write enable
const PLM_PL2_WE: [u32; 2] = [0x0082_3B24, 0x0082_3B28]; // bit6 = PL2 write enable
/// GB20x PES/OFA moved house (HAL 67-71): 0x820dbc is GA100/Ampere/Ada
/// only - which is exactly why our GB206 sweeps read all-badf there.
const GB20X_PES_STATUS_BASE: u32 = 0x0082_2A78;
const GB20X_OFA_DISABLE: u32 = 0x0082_2A68;

/// Turing mirror probe (open-questions #13 Turing arm + reading-notes §10).
/// TU104 field data (#187) put the option-status bank at 0x021xxx
/// (-0x7FF000 from the GA10x/GA100 layout) and derived the controller words
/// at 0x020xxx (-0x800000); the v0.7.4 dictionary round then decoded the
/// same words AT 0x820xxx on TU106 (13 plain + 112 badf5040, zero absent).
/// Alias (PRI decode ignoring bit 23), per-die layout, or two live banks -
/// one run, three addresses per word, settles it. On GA10x/Ada the mirror
/// columns should read badf (the control). 16 words x 3 addresses, ordered
/// by primary offset.
const GA100_MIRROR_WORDS: [(&str, u32); 16] = [
    ("FUSECTRL", 0x0082_0000),
    ("EN_SW_OVERRIDE", 0x0082_0040),
    ("FUSEDEBUG", 0x0082_0060),
    ("CRC_STATUS", 0x0082_0070),
    ("FUSEWDATA_PLM", 0x0082_00F4),
    ("FUSECTRL_PLM", 0x0082_00F8),
    ("FUSEOPT_PLM", 0x0082_00FC),
    ("DEBUG_FEAT_SW_OVR_EN", 0x0082_0148),
    ("SM_SPEED_SELECT_DP", 0x0082_0224),
    ("PRIVKEY_ACC_DIS", 0x0082_0324),
    ("OPT_GPC_DISABLE", 0x0082_0350),
    ("WDATA_WR_SECURE", 0x0082_048C),
    ("OPT_GPC_DEFECTIVE", 0x0082_05C4),
    ("STAT_GPC", 0x0082_0C1C),
    ("STAT_TPC_GPC0", 0x0082_0C38),
    ("SBFE", 0x0082_1108),
];

/// Coverage-completion round (2026-11-12, same pinned v0.7.4): the first
/// dictionary pass swept 29 of the 672 published option words - the status
/// and core maps were 100% covered, the option map was not. These tables add
/// the segmentation carriers: the fuse-side _DISABLE/_DEFECTIVE pairs (the
/// words the STATUS_OPT_* values project from and the FF-burn targets of the
/// GA100VFI chain), the _CP shadow layer (the soft-disable copies - the GA100
/// 170hx-rir-tool precedent has them volatile-writable after EN_SW_OVERRIDE;
/// open-questions #13 Ada arm), and the PCIe/devid words. Excluded on
/// purpose: ~550 GA100-specific blocks (XUSB/AZA audio/ADCS/clock trees/
/// Tegra legacy/FPF spares at 0x8242xx) - low cross-die value. HALF_FBPA/
/// HALF_LTC are _ENABLE polarity (1=on). GPC pair skipped (already probed
/// as OPT_GPC_DISABLE/DEFECTIVE in the mirror and OPT_FUSE tables).
const GA100_OPT_DIS_ROWS: [(&str, u32); 34] = [
    ("FBIO_DISABLE", 0x0082_036C),
    ("FBIO_DEFECTIVE", 0x0082_05D4),
    ("FBPA_DISABLE", 0x0082_0368),
    ("FBPA_DEFECTIVE", 0x0082_05D0),
    ("FBP_DISABLE", 0x0082_0364),
    ("FBP_DEFECTIVE", 0x0082_05CC),
    ("PES_DISABLE", 0x0082_0690),
    ("PES_DEFECTIVE", 0x0082_0698),
    ("ROP_L2_DISABLE", 0x0082_02C4),
    ("ROP_L2_DEFECTIVE", 0x0082_05E8),
    ("NVDEC_DISABLE", 0x0082_0378),
    ("NVDEC_DEFECTIVE", 0x0082_05E0),
    ("NVENC_DISABLE", 0x0082_0414),
    ("NVENC_DEFECTIVE", 0x0082_05DC),
    ("NVJPG_DISABLE", 0x0082_0370),
    ("NVJPG_DEFECTIVE", 0x0082_05BC),
    ("OFA_DISABLE", 0x0082_02D8),
    ("OFA_DEFECTIVE", 0x0082_02F0),
    ("NVLINK_DISABLE", 0x0082_0684),
    ("NVLINK_DEFECTIVE", 0x0082_068C),
    ("SYS_PIPE_DISABLE", 0x0082_03A4),
    ("SYS_PIPE_DEFECTIVE", 0x0082_03AC),
    ("DISPLAY_DISABLE", 0x0082_037C),
    ("DISPLAY_DEFECTIVE", 0x0082_05D8),
    ("DISP_HEAD_DISABLE", 0x0082_0418),
    ("PCIE_LANE_DISABLE", 0x0082_0394),
    ("PCIE_LANE_DEFECTIVE", 0x0082_05E4),
    ("PERLINK_DISABLE", 0x0082_07F0),
    ("SEC_DISABLE", 0x0082_04FC),
    ("GC6_ISLAND_DISABLE", 0x0082_0568),
    ("HALF_FBPA_ENABLE", 0x0082_049C),
    ("HALF_LTC_ENABLE", 0x0082_060C),
    ("FB_CONFIG", 0x0082_0328),
    ("GEN3_SPEED_DISABLE", 0x0082_0250),
];
const GA100_DEVID_ROWS: [(&str, u32); 6] = [
    ("PCIE_DEVIDA", 0x0082_04D8),
    ("PCIE_DEVIDB", 0x0082_056C),
    ("PCIE_MAGIC_A", 0x0082_0558),
    ("PCIE_MAGIC_B", 0x0082_0518),
    ("PCIE_MAGIC_C", 0x0082_051C),
    ("PCIE_MAGIC_D", 0x0082_0520),
];
/// Per-GPC TPC fuse words: DISABLE contiguous 0x820760+4g, RECONFIG
/// contiguous 0x820788+4g, DEFECTIVE scattered (printed one by one).
const GA100_TPC_DISABLE_BASE: u32 = 0x0082_0760;
const GA100_TPC_RECONFIG_BASE: u32 = 0x0082_0788;
const GA100_TPC_DEF_ROWS: [(&str, u32); 8] = [
    ("TPC_GPC0_DEFECTIVE", 0x0082_05C8),
    ("TPC_GPC1_DEFECTIVE", 0x0082_0508),
    ("TPC_GPC2_DEFECTIVE", 0x0082_050C),
    ("TPC_GPC3_DEFECTIVE", 0x0082_0514),
    ("TPC_GPC4_DEFECTIVE", 0x0082_0564),
    ("TPC_GPC5_DEFECTIVE", 0x0082_05AC),
    ("TPC_GPC6_DEFECTIVE", 0x0082_0B84),
    ("TPC_GPC7_DEFECTIVE", 0x0082_0B80),
];
/// The _CP shadow family (24 on GA100): soft-disable copies behind
/// EN_SW_OVERRIDE - the volatile floorsweep-reversal candidates (OQ#13).
/// FB_CONFIG appears twice across the tables (pair table + CP anchor) - the
/// second read is the consistency check.
const GA100_CP_ROWS: [(&str, u32); 24] = [
    ("FBP_DISABLE_CP", 0x0082_0234),
    ("GPC_DISABLE_CP", 0x0082_022C),
    ("TPC_GPC0_DISABLE_CP", 0x0082_0230),
    ("TPC_GPC1_DISABLE_CP", 0x0082_0504),
    ("TPC_GPC2_DISABLE_CP", 0x0082_05F4),
    ("TPC_GPC3_DISABLE_CP", 0x0082_05A0),
    ("TPC_GPC4_DISABLE_CP", 0x0082_04C4),
    ("TPC_GPC5_DISABLE_CP", 0x0082_04CC),
    ("TPC_GPC6_DISABLE_CP", 0x0082_0B8C),
    ("TPC_GPC7_DISABLE_CP", 0x0082_0B88),
    ("FBIO_DISABLE_CP", 0x0082_0460),
    ("FBPA_DISABLE_CP", 0x0082_0454),
    ("NVDEC_DISABLE_CP", 0x0082_0458),
    ("NVENC_DISABLE_CP", 0x0082_045C),
    ("NVJPG_DISABLE_CP", 0x0082_0488),
    ("ROP_L2_DISABLE_CP", 0x0082_02E4),
    ("OFA_DISABLE_CP", 0x0082_02EC),
    ("PCIE_LANE_DISABLE_CP", 0x0082_0248),
    ("DISPLAY_DISABLE_CP", 0x0082_024C),
    ("DISP_HEAD_DISABLE_CP", 0x0082_0450),
    ("SYS_PIPE_DISABLE_CP", 0x0082_03A8),
    ("PES_DISABLE_CP", 0x0082_0694),
    ("NVLINK_DISABLE_CP", 0x0082_0688),
    ("FB_CONFIG_CP_ANCHOR", 0x0082_0328),
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
        // GB100=0x1A0 GB102=0x1A2 - same FEAT bank as GB20x, untested sample
        Some(("blackwell-gb1", &FEAT_BLACKWELL))
    } else if (0x1B0..=0x1BF).contains(&die) {
        // GB202-class (50-series); GB206=0x1B6 field-read (issue #168)
        Some(("blackwell-gb2", &FEAT_BLACKWELL))
    } else if (0x210..=0x21F).contains(&die) {
        // GB202 also surfaces as 0x21120000 in an RM debug dump (issue #12);
        // the nv_arch.h scheme puts it at 0x1B2, so accept both forms
        Some(("blackwell-gb2", &FEAT_BLACKWELL))
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
        0xFFFF_FB8F => "closed (PL3-only writes)",
        0xFFFF_FFEF => "closed (relock+drift)",
        _ => "closed (non-standard)",
    }
}

/// decode_plm, colored: OPEN green, stock-closed plain (the shipped state,
/// nothing to alarm about), every other closed form red. Platform polish
/// (2026-11-12 user feedback: the fuse-dictionary PLM notes read flat).
fn paint_plm(v: u32) -> String {
    match decode_plm(v) {
        "OPEN" => format!(" ({})", green("OPEN")),
        "closed (stock)" => format!(" ({})", decode_plm(v)),
        other => format!(" ({})", red(other)),
    }
}

/// Readout fields (not rates) for a Blackwell `FEAT_READOUT_1` word.
fn decode_readout1(v: u32) -> String {
    READOUT1_SHIFT
        .iter()
        .map(|s| ((v >> s) & 7).to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Bit names for a per-GPC TPC mask (bit n = TPC n).
fn decode_tpc_bits(v: u32) -> String {
    (0..8)
        .filter(|b| v & (1 << b) != 0)
        .map(|b| format!("TPC{b}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Read-only per-GPC TPC floorsweep survey (open question #13): which TPCs
/// are fused off per GPC, and whether the affected GPC carries a defect bit.
/// Nothing is written; the CTRL override path is out of scope.
fn dump_tpc(rd: &dyn Fn(u32) -> u32) {
    println!("--- tpc (per-GPC floorsweep; STATUS bit n = TPC n of that GPC) ---");
    let mut off_total = 0u32;
    let mut off_mask = 0u32;
    let mut statuses = [0u32; TPC_MAX_GPC as usize];
    for gpc in 0..TPC_MAX_GPC {
        let ctrl = rd(TPC_CTRL_BASE + gpc * 4);
        let status = rd(TPC_STATUS_BASE + gpc * 4);
        statuses[gpc as usize] = status;
        let b = badf_bucket(status);
        let (note, n) = match b {
            Some(b) => (format!(" [{}]", red(b)), 0),
            None => (format!(" [{}]", decode_tpc_bits(status)), status.count_ones()),
        };
        off_total += n;
        if n > 0 {
            off_mask |= 1 << gpc;
        }
        println!(
            "  GPC{gpc} CTRL={} STATUS={}{}",
            hx32(ctrl),
            hx32(status),
            note
        );
    }
    let dis = rd(OPT_GPC_DISABLE);
    let def = rd(OPT_GPC_DEFECTIVE);
    println!(
        "  OPT_GPC_DISABLE={} (bit per GPC)  OPT_GPC_DEFECTIVE={} (bit per GPC)",
        hx32(dis),
        hx32(def)
    );
    print!("  bank A {:#010x}..:", TPC_BANK_A);
    for gpc in 0..TPC_MAX_GPC {
        print!(" {}", hx32(rd(TPC_BANK_A + gpc * 4)));
    }
    println!();
    print!("  bank B {:#010x}.. (+4 = OPT_GPC_DEFECTIVE above):", TPC_BANK_B);
    for gpc in 0..TPC_MAX_GPC {
        print!(" {}", hx32(rd(TPC_BANK_B + gpc * 4)));
    }
    println!();
    if off_total == 0 {
        println!("  verdict: no TPC fused off in the STATUS masks (full for this GPC count)");
    } else if def & off_mask == 0 {
        println!(
            "  verdict: {off_total} TPC(s) fused off in {} GPC(s), no defect bit on them - override candidates (write path out of scope)",
            off_mask.count_ones()
        );
    } else {
        println!(
            "  verdict: {off_total} TPC(s) fused off; affected GPCs also carry defect bits (mask {:#x})",
            def & off_mask
        );
    }
    // Reference maps from the field (issue #169): byte-identical STATUS
    // vectors seen on independent machines. Matching is on the vector alone
    // so a future report self-classifies without a verdict re-derivation.
    const RTX3070_MAP: [u32; TPC_MAX_GPC as usize] =
        [0x30, 0x30, 0x30, 0x30, 0x31, 0x30, 0x3f, 0x00];
    if statuses == RTX3070_MAP {
        println!(
            "  ref: STATUS vector matches the standard RTX 3070 (2484) binning map - byte-identical on two independent samples (#169 aston89, #54 GWASDW123); 19 TPCs off, 23 active = the 46-SM SKU"
        );
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

fn dump_debug(rd: &dyn Fn(u32) -> u32, fam: &str) {
    println!("--- core/debug block ---");
    for (name, off) in CORE_ROWS {
        let v = rd(off);
        match badf_bucket(v) {
            Some(b) => println!("{name:<22} = {} [{}]", hx32(v), red(b)),
            None => println!("{name:<22} = {}", hx32(v)),
        }
    }
    println!(
        "CANARY_A/B (loader-written)  = {}/{} -> {}",
        hx32(rd(CANARY_A)),
        hx32(rd(CANARY_B)),
        canary_state(rd)
    );
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
    println!("--- FEAT cluster (all mapped families, cross-visibility) ---");
    let blackwell = fam.starts_with("blackwell");
    for (label, regs) in [
        ("turing 0x4096xx", &FEAT_TURING[..]),
        ("0x823xxx amp/ada/gb", &FEAT_AMPADA[..]),
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
                    None if blackwell => {
                        format!(" [fields {} - not rates]", decode_readout1(v))
                    }
                    None => String::new(),
                },
                _ => String::new(),
            };
            println!("  {label:<20} {name:<9} = {}{note}", hx32(v));
        }
    }
    if blackwell {
        println!("--- blackwell fuse/aux block (read-only; selectors are fuse mirrors) ---");
        // GB206 TPC CTRL overrides (0x820838+4g, the writable pair sibling of
        // the 0x820c38 STATUS row below = GA100_SM_STAT; discussion #216
        // asks for exactly these three from a desktop 5060 Ti - read-only).
        print!("  TPC_CTRL (0x820838+4g, 12 GPCs):");
        for gpc in 0..12u32 {
            print!(" {}", cell_val(rd(TPC_CTRL_BASE + gpc * 4)));
        }
        println!();
        for (name, off) in BLACKWELL_ROWS {
            let v = rd(off);
            let note = badf_bucket(v).map(|b| format!(" [{}]", red(b))).unwrap_or_default();
            println!("  {name:<26} = {}{note}", hx32(v));
        }
        println!(
            "  READOUT_1 fields (imla0 fmla16 fmla32 ffma imla1 imla2 imla3 imla4 DP) = {}",
            decode_readout1(rd(0x0082_3818))
        );
        println!("  note: on Blackwell selector 3 measures ~HALF rate, not the Ada 1/8 - trust the raw word, not the Ada rate law");
        println!("  note: FUSE_EN_SW_OVERRIDE=0 gates CTRL_OPT floorsweep only; FUSE_FEAT_OVR_DIS=0 leaves the override path open; FUSE_SS_PLM is NOT the effective gate");
    }
    println!("--- family-specific lines (safe reads; badf on the wrong die is expected) ---");
    for (label, regs) in [
        ("ada 4-write", &ADA_ROWS[..]),
        ("ad1x 64g-bar1", &AD1X_ROWS[..]),
        ("ada fuse/tpc", &ADA_FUSE_ROWS[..]),
    ] {
        for (name, off) in regs {
            let v = rd(*off);
            let note = match *name {
                n if n.ends_with("PLM") => format!("{}", paint_plm(v)),
                _ => match badf_bucket(v) {
                    Some(b) => format!(" [{}]", red(b)),
                    None => String::new(),
                },
            };
            println!("  {label:<15} {name:<22} {}= {}{note}", off_h(*off), hx32(v));
        }
    }
    if fam == "ampere" || fam == "ada" {
        dump_tpc(rd);
    }
    println!("--- pcie/gen2 probe (field round 3 union: cmp50hx patch04 + cmpunlocker 0007 + bar1 + L2) ---");
    for (name, off) in PCIE_LINK_ROWS
        .iter()
        .chain(XP3G_ROWS.iter())
        .chain(OPT_FUSE_ROWS.iter())
    {
        let v = rd(*off);
        let note = match *name {
            n if n.ends_with("_PLM") || n.ends_with("PLM0") => format!("{}", paint_plm(v)),
            _ => match badf_bucket(v) {
                Some(b) => format!(" [{}]", red(b)),
                None => String::new(),
            },
        };
        println!("  {name:<18} = {}{note}", hx32(v));
    }
    // GA100 PR#55 SM banks: 8 dwords per row, badf values colored red (no
    // per-cell bucket text - columns stay aligned). On consumer dies the
    // whole row is the per-die existence answer.
    print!("{: >18}", "  GA100_SM_OVR");
    for gpc in 0..8u32 {
        print!(" {}", cell_val(rd(GA100_SM_OVR_BASE + gpc * 4)));
    }
    println!();
    print!("{: >18}", "  GA100_SM_STAT");
    for gpc in 0..8u32 {
        print!(" {}", cell_val(rd(GA100_SM_STATUS_BASE + gpc * 4)));
    }
    println!();
    println!("--- ampere/ada FEAT sweep (M1 cluster) ---");
    print!("{: >8}", "");
    for off in AMPADA_SWEEP {
        print!(" {}{}", paint("96", "0x"), paint("93", &format!("{off:08x}")));
    }
    println!();
    print!("{: >8}", "value");
    for off in AMPADA_SWEEP {
        print!(" {}", hx32(rd(off)));
    }
    println!();
    println!("--- PLM survey (cmp170hx 26-register union) ---");
    // Two-column layout: same multi-column rule as the arrays - no bucket
    // text per cell; badf values marked red (ANSI on) / '!' prefix (plain,
    // width-stable so the columns stay aligned either way). OPEN keeps its
    // tag - it is rare and single-word.
    for (i, off) in PLM_SURVEY.iter().enumerate() {
        let v = rd(*off);
        let off_s = format!("{}{}", paint("96", "0x"), paint("93", &format!("{off:08x}")));
        let val_s = cell_val(v);
        let note = if v == 0xFFFF_FFFF {
            format!(" [{}]", green("OPEN"))
        } else {
            String::new()
        };
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
        println!("{name:<22} {}= {}{note}", off_h(off), hx32(v));
    }
    println!("--- ga100 vfi dictionary (kate-land/ga100vfi; read-only, cross-die probe) ---");
    for (label, regs) in [
        ("fusectl", &GA100_FUSECTL_ROWS[..]),
        ("sec-alias", &GA100_SEC_ALIAS_ROWS[..]),
        ("speed-sel", &GA100_SPEED_SEL_ROWS[..]),
        ("status-opt", &GA100_STATUS_OPT_ROWS[..]),
        ("opt-dis", &GA100_OPT_DIS_ROWS[..]),
        ("devid", &GA100_DEVID_ROWS[..]),
        ("tpc-def", &GA100_TPC_DEF_ROWS[..]),
        ("cp-shadow", &GA100_CP_ROWS[..]),
    ] {
        for (name, off) in regs {
            let v = rd(*off);
            let note = if name.ends_with("PLM") {
                format!("{}", paint_plm(v))
            } else {
                match badf_bucket(v) {
                    Some(b) => format!(" [{}]", red(b)),
                    None => String::new(),
                }
            };
            println!("  {label:<10} {name:<26} {}= {}{note}", off_h(*off), hx32(v));
        }
    }
    // STATUS_OPT arrays, 8 dwords per line, badf-bucketed inline (GA100_SM
    // bank idiom). The whole block is the per-die existence answer.
    for (label, base, count) in [
        ("PES_GPC", GA100_PES_STATUS_BASE, 12u32),
        ("ROP_L2_FBP", GA100_ROP_L2_STATUS_BASE, 16u32),
        ("TPC_GPC", GA100_TPC_STATUS_BASE, 32u32),
        ("TPC_DIS", GA100_TPC_DISABLE_BASE, 12u32),
        ("TPC_RECFG", GA100_TPC_RECONFIG_BASE, 12u32),
        // #237: the Ada/595 TPC RECONFIG RAM base (0x822800+4g) - our old
        // table carried the GA100-layout guess at 0x820788.
        ("TPC_RECFG_ADA", AD10X_TPC_RECONFIG_BASE, 12u32),
        // #237: per-GPC ROP mask - a different face from the per-FBP group
        // mask (0x820d70+4f, ROP_L2_FBP above).
        ("ROP_GPC_ADA", AD10X_ROP_PER_GPC_BASE, 12u32),
    ] {
        for chunk in (0..count).step_by(8) {
            print!("  {label:<10} [{chunk:>2}]");
            for i in chunk..(chunk + 8).min(count) {
                print!(" {}", cell_val(rd(base + i * 4)));
            }
            println!();
        }
    }
    println!("  --- i237 disassembly round (595.44.15-sourced; read-only) ---");
    {
        // ClockManager +0x2f0 VF/Speedo trims: 10-bit words = signed 6-bit
        // temperature trim + 4-bit voltage trim (issue #147 UV relevance).
        print!("  CLK_TRIMS   ");
        for t in CLK_VF_SPEEDO_TRIMS {
            print!(" {:#010x}", rd(t));
        }
        println!(" (10-bit: s6 temp + s4 volt fields)");
        // VBIOS app08 override-pair regions: 0x6ae4 writes (Data, Control|=1)
        // pairs here at PL2. Word pairs, 4 per line, non-zero ones only.
        for (base, pairs) in VBIOS_OVR_PAIR_REGIONS {
            let mut printed = 0;
            for chunk in 0..pairs {
                let off = base + chunk * 8;
                let (d, c) = (rd(off), rd(off + 4));
                if d == 0 && c == 0 {
                    continue;
                }
                if printed % 4 == 0 {
                    if printed > 0 {
                        println!();
                    }
                    print!("  OVR_PAIRS  {base:#010x}");
                }
                print!(" d={d:#010x} c={c:#010x}");
                printed += 1;
            }
            if printed > 0 {
                println!();
            }
        }
        // PLM write-enable bits (disassembly-proven semantics): FUSECTRL_PLM
        // bit4 = PL0 write enable (checked by the TU102/TU116 OBJBSP writer),
        // 0x823b24/28 bit6 = PL2 (checked by VBIOS app08 before its 64 pairs).
        let pl0we = rd(PLM_PL0_WE);
        println!(
            "  PLM_PL0_WE bit4={} (FUSECTRL_PLM bit4; 1=host-level writes enabled)",
            (pl0we >> 4) & 1
        );
        for p in PLM_PL2_WE {
            let v = rd(p);
            println!("  PLM_PL2_WE {p:#010x} = {v:#010x} bit6={} (VBIOS app08 gate)", (v >> 6) & 1);
        }
    }
    if fam.starts_with("blackwell") {
        // GB20x HAL 67-71 PES/OFA relocation: 0x820dbc is GA100/Ampere/Ada.
        println!("  GB20X_PES   (HAL 67-71 relocated face)");
        print!("   PES_STATUS ");
        for gpc in 0..12u32 {
            print!(" {}", cell_val(rd(GB20X_PES_STATUS_BASE + gpc * 4)));
        }
        println!();
        let ofa = rd(GB20X_OFA_DISABLE);
        let b = badf_bucket(ofa).map(|b| format!(" [{}]", red(b))).unwrap_or_default();
        println!("   OFA_DISABLE {}{b}", hx32(ofa));
    }
    println!("  note: ga100 offsets, NV_FUSE_OPT_ prefix dropped; badf5040 = word exists behind a PLM,");
    println!("  plain value = read it with GA100 semantics, ffffffff/badf1100 = not decoded here (AD10x fuse-PLM relocation precedent)");
    if fam == "turing" {
        // Mirror-round verdict: on TU106 the GA100-layout option VALUES live at
        // -0x7FF000 (the #187/TU104 bank), while 0x820xxx decodes a surface whose
        // control/PLM words agree but option words read zero or badf. Print the
        // whole dictionary once more at the shifted base - that is where the
        // real bits are on this family. GR topology 0x409604 rides along as the
        // independent cross-check anchor (TU104 KO: 5 active GPCs read 0x06030005).
        println!("--- turing fuse bank (ga100 dictionary at -0x7FF000; #13 / cold-values TU104) ---");
        // The shift is defined only inside the 0x82xxxx option bank. Rows
        // outside it - the two PRI-ring words at 0x1200xx - must NOT be
        // shifted: 0x120060 - 0x7FF000 underflows u32, a release build wraps
        // silently (no overflow checks), and the volatile load lands ~4 GiB
        // past the 16 MiB mapping into unassigned PCI space - which froze
        // the Z170 bench on 2026-11-11. They are already printed flat in
        // the dictionary section above.
        let t = |off: u32| rd(off - 0x7FF_000);
        for (label, regs) in [
            ("fusectl", &GA100_FUSECTL_ROWS[..]),
            ("sec-alias", &GA100_SEC_ALIAS_ROWS[..]),
            ("speed-sel", &GA100_SPEED_SEL_ROWS[..]),
            ("status-opt", &GA100_STATUS_OPT_ROWS[..]),
            ("opt-dis", &GA100_OPT_DIS_ROWS[..]),
            ("devid", &GA100_DEVID_ROWS[..]),
            ("tpc-def", &GA100_TPC_DEF_ROWS[..]),
            ("cp-shadow", &GA100_CP_ROWS[..]),
        ] {
            for (name, off) in regs {
                if *off < 0x800_000 {
                    continue; // not an option-bank row (the two RING words)
                }
                let v = t(*off);
                let note = if name.ends_with("PLM") {
                    format!("{}", paint_plm(v))
                } else {
                    match badf_bucket(v) {
                        Some(b) => format!(" [{}]", red(b)),
                        None => String::new(),
                    }
                };
                println!("  {label:<10} {name:<26} {}= {}{note}", off_h(*off - 0x7FF_000), hx32(v));
            }
        }
        for (label, base, count) in [
            ("PES_GPC", GA100_PES_STATUS_BASE, 12u32),
            ("ROP_L2_FBP", GA100_ROP_L2_STATUS_BASE, 16u32),
            ("TPC_GPC", GA100_TPC_STATUS_BASE, 32u32),
        ] {
            for chunk in (0..count).step_by(8) {
                print!("  {label:<10} [{chunk:>2}]");
                for i in chunk..(chunk + 8).min(count) {
                    print!(" {}", cell_val(t(base + i * 4)));
                }
                println!();
            }
        }
    }
    println!("--- turing mirror probe (0x820xxx vs -0x800000 vs -0x7FF000; open-questions #13) ---");
    println!("  {:<24} {:>13} {:>13} {:>13}", "word", "0x820xxx", "-0x800000", "-0x7FF000");
    for (name, off) in GA100_MIRROR_WORDS {
        print!("  {name:<24}");
        for base in [0u32, 0x800_000, 0x7FF_000] {
            let v = if base == 0 { rd(off) } else { rd(off - base) };
            let cell = cell_val(v);
            let cell = if badf_bucket(v).is_some() && !COLOR_ON.load(std::sync::atomic::Ordering::Relaxed) {
                format!("{cell:>13}")
            } else if badf_bucket(v).is_some() {
                format!("{:>13}", cell)
            } else {
                format!("{cell:>13}")
            };
            print!(" {cell}");
        }
        println!();
    }
    println!("  note: mirror == primary means a bit-23 alias (both addresses valid); mirror badf while");
    println!("  primary is plain means 0x820xxx exclusive; different plain values in two columns = two live banks");
    // GR topology anchor, every family (open-questions #19): byte3 = die GPC
    // slots, byte0 = active GPC count - three-point calibrated (TU104 KO
    // 0x06030005 = 5 active/3 ROP; TU106 2070 0x08040003 = 3 active, loader
    // m3 attempt13; TU106 1e93 0x08040006 = 6 active, issue191 loader log).
    // On Ampere it is the OQ#19 polarity discriminator: GA106 byte0 4 ->
    // the TPC masks read bit=off, byte0 7 -> bit=on.
    let gr = rd(0x0040_9604);
    let b3 = (gr >> 24) & 0xff;
    let b0 = gr & 0xff;
    println!("  GR_TOPO_409604         = {} (byte3 GPC-slots {b3} / byte0 active-GPC {b0}; anchors 06030005/08040003/08040006)", hx32(gr));
}

// ---------------------------------------------------------------- stage timing

// Cumulative stage timing, the loader's issue#38 `[+ms]` convention: each
// stage prints one cumulative-elapsed line, so a slow run shows exactly
// where its time went without a second instrumented build. Opt-in since
// v0.6.3: the release default output stays clean, `--timing` (or the
// SSDIAG_TIMING env var) turns the lines on for slow-run diagnosis.
static T0: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
static TIMING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn t0_set() {
    let _ = T0.set(std::time::Instant::now());
}

fn stage(label: &str) {
    if !TIMING.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let ms = T0.get().map(|t| t.elapsed().as_millis()).unwrap_or(0);
    println!("[+{ms}ms] {label}");
}

fn main() {
    t0_set();
    color_init();
    install_crash_cleanup();
    let code = run();
    // strict remove-when-done: our own service and temp file never outlive
    // this process - cleanup runs on every path that got past arg parsing.
    // Gated on DRIVER_TOUCHED: a --dry-run (or any exit before deploy) has
    // nothing to remove and must not burn the retry loop on a temp .sys
    // that never existed.
    if DRIVER_TOUCHED.load(std::sync::atomic::Ordering::Relaxed) {
        cleanup_driver();
        println!("driver service removed (remove-when-done)");
        stage("driver removed");
    }
    stage("total");
    std::process::exit(code);
}

fn run() -> i32 {
    let mut reapply = false;
    let mut dry = false;
    let mut inv_only = false;
    let mut sb_arm = false;
    let mut sb_disarm = false;
    let mut select: Option<String> = None;
    let mut args = std::env::args().skip(1);
    if std::env::var_os("SSDIAG_TIMING").is_some() {
        TIMING.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    while let Some(a) = args.next() {
        match a.as_str() {
            "--reapply" => reapply = true,
            "--safeboot-arm" => sb_arm = true,
            "--safeboot-disarm" => sb_disarm = true,
            "--timing" => TIMING.store(true, std::sync::atomic::Ordering::Relaxed),
            "--help" | "-h" => {
                println!(
                    "ssdiag {ver} - in-OS NVIDIA register diagnosis + separate-lock re-apply

USAGE:
  ssdiag [--reapply] [--select <substr>] [--safeboot-arm|--safeboot-disarm]

OPTIONS:
  --reapply        per card with an OPEN FEAT gate: write SS0=0x88888888
                   SS1=0x8 and verify by read-back (separate-lock method -
                   restores an unlock the driver re-derived, no reboot)
  --select <s>     only cards whose DeviceID contains <s> (multi-GPU)
  --safeboot-arm   stage the probe driver for Safe Mode: hash-checked copy
                   to %ProgramData%, persistent service, and SafeBoot
                   Minimal+Network whitelist entries (elevated, NORMAL mode;
                   includes a start/stop proof round). Undo: --safeboot-disarm
  --safeboot-disarm  remove that staging completely (SafeBoot keys, service,
                   persistent copy) - run elevated in NORMAL mode
  --dry-run        print the platform gates + discovered cards, load nothing
  --inventory      print the unelevated read-only hardware inventory block
                   (OS, CPU, board, BIOS, chassis, RAM slots/speeds, disks
                   and partitions, other PCI devices, per-GPU driver /
                   VBIOS / family verdict + advisories) and exit - the
                   block the issue templates ask for; no admin, no driver,
                   no writes
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
       - FEAT clusters of all mapped families (Turing 0x4096xx incl.
         READOUT, Ampere/Ada/Blackwell 0x823xxx). On Blackwell the block
         reads fine but the transaction cannot run: the fuse selectors,
         override master switch and the non-contiguous READOUT_1 fields
         are printed with the Blackwell rate-law caveat (issue #168)
       - Ada four-write targets (WPR/WPR_CFG/FBPA PLMs), AD10x 64G-BAR1
         line (CYA/CFG/XVE/BAR1 cap), 26-register PLM survey, Ampere FEAT
         sweep
       - pcie/gen2 probe union (field round 3): XVE link block + policy
         registers (0x8808x/0x8841c/0x886xx/0x8872c/0x8c040/0x8c1c0/
         0x8c2c0), XVE PLMs, XP3G override block + PLMs, fuse/OPT wall
         (OPT_GEN23/GEN3/MAGIC, OPT_GPC_DISABLE), GA100 PR#55 SM banks,
         AD102 L2 mask layer - read-only existence evidence per die
       - config-space link caps: LnkCap/LnkSta/LnkCap2/LnkCtl2 decoded
         (LnkSta is the only trustworthy link-speed criterion)
       - POWER: card and upstream root-port PMCSR / D-state - the
         register-side witness of the platform _PR3/_OFF rail path
  4. unlock verdict: FEAT SS0/SS1 == 0x88888888/0x8 => UNLOCKED
     (field-proven to persist in Windows on the Turing relock chain)

FAMILIES: maxwell..rubin recognized (incl. the GA10x 0xb7xx00a1 BOOT_0
quirk); FEAT verdict where the cluster is mapped (Turing/Ampere/Ada/
Blackwell). Blackwell GB20x maps to the same 0x823xxx bank, but only for
reads: there is no GB20x SEC2 booter, so no transaction runs on it.

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
            "--inventory" => {
                inv_only = true;
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

    // safe-mode staging commands run before every other path - they touch
    // only the SCM and the SafeBoot whitelist, never the probe itself
    if sb_arm {
        return safeboot_arm();
    }
    if sb_disarm {
        return safeboot_disarm();
    }

    // v0.7.0: discovery sits ABOVE the admin gate. It is unelevated CfgMgr32
    // (fast since 0.6.1) and both the inventory header and --inventory need
    // it first; an unelevated full run now prints its inventory block before
    // reporting "admin required".
    let (mut gpus, skipped) = match discover_gpus() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    if gpus.is_empty() {
        eprintln!("no NVIDIA display adapter memory ranges found");
        eprintln!(
            "display-class devnodes carrying VEN_10DE: {}",
            gpus.len() + skipped.len()
        );
        for line in &skipped {
            eprintln!("  skipped {line}");
        }
        return 2;
    }
    if dry || std::env::var_os("SSDIAG_VERBOSE_DISCOVERY").is_some() {
        for line in &skipped {
            eprintln!("[discover] skipped {line}");
        }
    }
    if let Some(sel) = &select {
        gpus.retain(|g| g.id.to_uppercase().contains(&sel.to_uppercase()));
        if gpus.is_empty() {
            eprintln!("--select {sel:?}: no NVIDIA function matches");
            return 2;
        }
    }

    let inv = inventory_header(&gpus);
    stage("inventory");
    if inv_only {
        preflight();
        inventory_advisories(&gpus, &inv);
        println!("inventory complete - read-only, nothing was loaded or written");
        println!("posting this block? review it and redact anything you consider personal first");
        return 0;
    }

    // Fast capability gate: deploying the driver needs SC_MANAGER_CREATE_SERVICE
    // access, and this probe is the very call deploy_driver() makes - failing
    // here predicts that failure exactly (same token, same access mask).
    // The borrow path (a PMXDRV-class driver already running) needs no SCM
    // access and stays open to a non-elevated run; --dry-run loads nothing.
    if !dry && PmxDrv::try_open().is_none() {
        let mgr = unsafe { win::OpenSCManagerW(ptr::null(), ptr::null(), SC_MANAGER_CREATE_SERVICE) };
        if mgr.is_null() {
            eprintln!(
                "OpenSCManager failed (admin required): win32 {} - re-run from an elevated prompt",
                last_err()
            );
            return 2;
        }
        unsafe {
            win::CloseServiceHandle(mgr);
        }
    }

    // only functions with a BAR0-class window (companion audio/USB-C are small)
    gpus.retain(|g| !g.bar0_candidates().is_empty());
    if gpus.is_empty() {
        eprintln!("no NVIDIA function with a BAR0-class (>=16 MiB) window found");
        return 2;
    }
    stage(&format!("discovery: {} card(s)", gpus.len()));

    if dry {
        for g in &gpus {
            println!("DRY-RUN card: {} ({} range(s), bdf {:?})", g.id, g.ranges.len(), g.bdf);
        }
        preflight();
        inventory_advisories(&gpus, &inv);
        println!("DRY-RUN: {} NVIDIA range set(s) discovered, nothing was loaded or written", gpus.len());
        return 0;
    }

    preflight();
    inventory_advisories(&gpus, &inv);
    stage("platform gates");
    let drv = match ensure_driver() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    stage("driver ready");

    println!("{ } NVIDIA GPU function(s) discovered", gpus.len());
    let mut rc = 0;
    let total = gpus.len();
    for (i, g) in gpus.iter_mut().enumerate() {
        if i > 0 {
            println!();
        }
        println!("===== GPU {}/{}: {} =====", i + 1, total, g.id);
        // An unstarted device can have no PnP log config at all (safe mode,
        // issue #37): the decode is still programmed, so read BAR0 straight
        // from config space instead of giving up on the card.
        if g.ranges.is_empty() {
            match bar0_from_config(&drv, &g.id, g.bdf) {
                Some(r) => {
                    println!("ranges: none in PnP - BAR0 recovered from config space");
                    g.ranges.push(r);
                }
                None => println!("ranges: none in PnP and config-space recovery failed"),
            }
        }
        println!("ranges:");
        for (a, b) in &g.ranges {
            println!("  {}-{} ({} MiB)", hxaddr(*a), hxaddr(*b), (b - a + 1) >> 20);
        }
        // works even for a parked card (config space answers regardless of
        // D0/MEM decode) - hence before the BAR0 walk, not after
        print_config_rebar(&drv, &g.id, g.bdf);
        print_power_state(&drv, g.devinst, &g.id, g.bdf);
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
        // Host-side last-resort net: every table offset lives inside the
        // 16 MiB BAR0 aperture (PAGES=4096 in walk_candidates), so anything
        // at or past that bound is an arithmetic bug upstream - e.g. a
        // shifted-base subtraction underflowing u32, which release builds
        // wrap silently and then volatile-load ~4 GiB past the mapping
        // (froze the Z170 bench on 2026-11-11 reading unassigned PCI space).
        // Answer all-ones instead of touching the bus: a data point, not a
        // stalled host. The same net guards wr - an out-of-aperture write
        // would corrupt foreign physical pages.
        let rd = |off: u32| unsafe {
            if off >= 0x0100_0000 {
                0xFFFF_FFFF
            } else {
                (base.add(off as usize) as *const u32).read_volatile()
            }
        };
        let wr = |off: u32, v: u32| unsafe {
            if off < 0x0100_0000 {
                (base.add(off as usize) as *mut u32).write_volatile(v)
            }
        };

        // SR-IOV VF shape (geforce-sriov / vfio-pci guests): the window serves
        // a synthetic chipset id on BOOT0 and hard-wired zeros on every other
        // page. Reads succeed but carry no state - the badf encoder lives on
        // the PF's PRI and the VF window has none - so "(closed (non-standard))"
        // below would be a fill artifact and a known-family VERDICT void.
        // First seen on a geforce-sriov RTXA6000-8Q guest over a host 3090
        // (BOOT0 0x1fa52a02, die 0x1fa - no known die segment; docs/evidence/
        // sriov-vf-guest-2230/). Probes span four banks to keep the bare-metal
        // false-positive risk at zero (a bound driver leaves WPR2 HI != 0 and
        // FUSE_VER != 0 on real silicon).
        let zero_fill = [0x00118128u32, 0x001fa828, 0x00840100, 0x00823804, 0x00824140]
            .iter()
            .all(|&off| rd(off) == 0);
        if zero_fill {
            println!(
                "NOTE: register window reads flat-zero behind a non-zero BOOT0 ({}) - virtualized / SR-IOV VF window: values below are fill, not silicon state",
                hx32(boot0)
            );
        }

        let Some((fam_name, regs)) = family_of(boot0) else {
            println!("PMC_BOOT_0={} - NVIDIA-class but unknown die; diagnosis-only pass (FEAT addresses unmapped for this family)", hx32(boot0));
            dump_debug(&rd, "unknown");
            drv.unmap_physical(va);
            rc = 1;
            continue;
        };
        dump_debug(&rd, fam_name);

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
        if fam_name.starts_with("blackwell") {
            println!(
                "{}: Blackwell has no SEC2 stage and no GB20x booter - the transaction path cannot run on this die; the FEAT block is read-only here (docs/blackwell-gb20x-status.md)",
                red("FAMILY NOTE")
            );
            println!("FAMILY NOTE: nonzero SS0/SS1 are pre-driver shadow state, not an unlock - enable bits clear means the fused selectors are in force");
        }
        let can_msg = match (canary_state(&rd), unlocked) {
            ("intact", true) => "CANARY: intact + SS live - no reset, nothing rewrote the state after handoff",
            ("intact", false) => "CANARY: intact + SS gone - no reset; something WROTE the FEAT block (driver init / devinit replay, not a power event)",
            ("cleared", true) => "CANARY: cleared + SS live - a reset happened but the AON island survived it (FLR/SBR class, not a rail-off)",
            ("cleared", false) => "CANARY: cleared + SS gone - a reset hit this boot after the EFI handoff (expected anonymous relock)",
            ("partial", _) => "CANARY: partial - narrow-domain reset, or a devinit rewrote one of the two words",
            ("other", _) => "CANARY: rewritten to third values - a devinit/BIOS pass ran over the pair",
            _ => "CANARY: unreadable - BAR0 reads poisoned here (parked function / no MEM decode?)",
        };
        println!("{can_msg}");

        if reapply {
            if plm != 0xFFFF_FFFF {
                if fam_name.starts_with("blackwell") {
                    println!(
                        "{}: the PLM gate only opens from PL3 firmware and no GB20x payload exists - nothing to re-apply, the USB chain cannot open it either",
                        red("reapply REFUSED")
                    );
                } else {
                    println!("{}: FEAT PLM gate is closed - re-run the USB unlock instead", red("reapply REFUSED"));
                }
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
    stage("probe complete");
    rc
}

/// Walk the card's BAR0 candidates smallest-size first; the PMC sanity read
/// arbitrates. A non-degenerate BOOT_0 on an unknown die still wins as a
/// fallback (diagnosis-only) - the die table must not veto a card. A
/// degenerate pick (all-0/all-1) is remembered only as a last resort.
/// BAR0 for a device PnP handed us no log config for: read it from config
/// space via ECAM and size it with the standard all-ones probe (restored
/// immediately). Only reached for a known bdf and never on the normal path.
/// Capped at 16 MiB - that is all `walk_candidates` maps anyway.
fn bar0_from_config(drv: &PmxDrv, instance: &str, bdf: Option<(u32, u32, u32)>) -> Option<(u64, u64)> {
    let did = dev_id_from(instance)?;
    let (bus, dev, func) = bdf?;
    let mcfg = mcfg_base_for(bus)?;
    let ecam = mcfg + ((bus as u64) << 20) + ((dev as u64) << 15) + ((func as u64) << 12);
    let va = drv.map_physical(ecam, 1).ok()?;
    let p = va as *const u8;
    let rd = |off: u32| unsafe { (p.add(off as usize) as *const u32).read_volatile() };
    let wr = |off: u32, v: u32| unsafe { (p.add(off as usize) as *mut u32).write_volatile(v) };
    let idw = rd(0);
    if idw & 0xffff != 0x10de || (idw >> 16) as u16 != did {
        drv.unmap_physical(va);
        return None;
    }
    let lo = rd(0x10);
    let is64 = (lo >> 1) & 0x3 == 0x2;
    let hi = if is64 { rd(0x14) } else { 0 };
    if lo == 0 || lo == 0xFFFF_FFFF || lo & 0x1 != 0 {
        drv.unmap_physical(va);
        return None; // unassigned or not a memory BAR
    }
    wr(0x10, 0xFFFF_FFFF);
    let mask = rd(0x10);
    wr(0x10, lo);
    drv.unmap_physical(va);
    if mask == 0 || mask == 0xFFFF_FFFF {
        return None;
    }
    // size mask: the low 4 bits are flags and always read back 0
    let size = ((!(mask & !0xf)).wrapping_add(1) as u64).clamp(0x1_0000, 0x100_0000);
    let base = (lo & !0xf) as u64 | ((hi as u64) << 32);
    if base == 0 {
        return None;
    }
    Some((base, base + size - 1))
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readout1_uses_the_gapped_offsets() {
        // GB206/GB202/GB203M stock word: imla0 fmla16 fmla32 ffma imla1
        // imla2 imla3 imla4 DP = 0 0 3 0 3 3 0 3 0. A contiguous 3-bit
        // decode prints four out-of-range 6s here.
        assert_eq!(decode_readout1(0x00C3_6180), "0 0 3 0 3 3 0 3 0");
        assert!(decode_readout1(0x00C3_6180)
            .split(' ')
            .all(|f| f.parse::<u32>().unwrap() <= 7));
    }

    #[test]
    fn blackwell_plm_is_recognised() {
        assert_eq!(decode_plm(0xFFFF_FB8F), "closed (PL3-only writes)");
        assert_eq!(decode_plm(0xFFFF_FF8F), "closed (stock)");
        assert_eq!(decode_plm(0xFFFF_FFFF), "OPEN");
    }

    #[test]
    fn blackwell_dump_renders_without_panicking() {
        let rd = |off: u32| match off {
            0x0082_3800 | 0x0082_3804 => 0xFFFF_FB8F,
            0x0082_3818 => 0x00C3_6180,
            0x0082_381C => 0x0627_7073,
            0x0082_3820 => 0x0002_5005,
            _ => 0,
        };
        dump_debug(&rd, "blackwell-gb2");
    }

    #[test]
    fn tpc_bits_decode() {
        assert_eq!(decode_tpc_bits(0x30), "TPC4 TPC5");
        assert_eq!(decode_tpc_bits(0), "");
        assert_eq!(decode_tpc_bits(0x81), "TPC0 TPC7");
    }

    #[test]
    fn tpc_dump_renders_without_panicking() {
        // 4080-shaped: GPC0/GPC1 one TPC off, GPC3 two, no defect bits.
        let rd = |off: u32| match off {
            0x0082_0C38 => 1,
            0x0082_0C3C => 1,
            0x0082_0C44 => 0x30,
            0x0082_0C54 => 0x3F,
            _ => 0,
        };
        dump_tpc(&rd);
    }

    #[test]
    fn turing_dump_renders_without_panicking() {
        // Walks every turing-gated section: the FEAT cluster, the GA100
        // dictionary, the -0x7FF000 shifted bank and the mirror probe. The
        // 2026-11-11 Z170 freeze lived exactly here - shifting the two
        // PRI-ring rows (0x1200xx, not option-bank rows) underflowed u32 and
        // a release build volatile-loaded ~4 GiB past the BAR0 mapping.
        // Debug builds panic on that overflow; this test is the net.
        let rd = |off: u32| match off {
            0x0040_9650 => 0xFFFF_FFFF,
            0x0040_9664 => 0x8888_8888,
            0x0040_966C => 8,
            _ => 0,
        };
        dump_debug(&rd, "turing");
    }

    #[test]
    fn cell_val_width_stable_across_buckets() {
        // Multi-column rule (2026-11-13 user directive): plain mode marks
        // badf with a width-stable '!' prefix so columns never shift. The
        // assertion is on VISIBLE width - ANSI escapes add bytes, not
        // columns - so strip them before measuring, whatever COLOR_ON
        // happens to be during the test run.
        let visible = |s: &str| s.replace('\x1b', "[").chars().filter(|_| true).count();
        let strip = |s: String| s.replace(['\x1b', '[', ']', ';'], "").split('m').next().unwrap_or("").to_string();
        let normal = cell_val(0x1234_5678);
        let badf = cell_val(0xBADF_5040);
        let nv = strip(normal.clone());
        let bv = strip(badf.clone());
        assert_eq!(nv.chars().count(), bv.chars().count(), "{normal} vs {badf}");
        let _ = visible("");
        assert!(badf.contains('!') || badf.contains('\x1b')); // marked either way
        assert_eq!(cell_val(0xFFFF_FFFF), format!("{:#010x}", 0xFFFF_FFFFu32)); // NO_DECODE stays plain
    }

    #[test]
    fn slot_sizes_decode_from_real_field_bytes() {
        // Z170 (raw dump 2026-11-12): raw MiB form, extended word is junk 1.
        assert_eq!(slot_size_mib(0x4000, Some(1)), Some(16384));
        // G614JV (raw dump 2026-11-13): Size is the 0x7FFF sentinel, the
        // shifted extended slot carries bare 0x00008000 = 32768 MiB (32GiB,
        // no bit30 flag). The pre-0.7.8 unshifted sentinel read at 0x1D
        // stitched 0xC0000080 out of these bytes - bit30 set - and printed
        // exactly "128 MiB". 14C/20T hybrid CPU line and this one come from
        // the same machine.
        assert_eq!(slot_size_mib(0x7FFF, Some(0x0000_8000)), Some(32768));
        assert_eq!(slot_size_mib(0x7FFF, None), None);
        // Legacy small-KiB quirk (spec KiB form reads absurdly small -> MiB).
        assert_eq!(slot_size_mib(0x0080, None), Some(128));
        // Spec MiB form in the extended slot overrides only when bigger.
        assert_eq!(slot_size_mib(0x4000, Some(0x4000_0000 | 32768)), Some(32768));
        // Slot empty.
        assert_eq!(slot_size_mib(0, Some(0x4000_0000 | 16384)), None);
    }

    #[test]
    fn blackwell_families_map_to_the_823_bank() {
        for boot0 in [0x1B6_000A1u32, 0x1A0_000A1, 0x2112_0000, 0x1B2_000A1] {
            let (_, regs) = family_of(boot0).unwrap();
            assert_eq!(regs.len(), 4);
            assert_eq!(regs[0].1, 0x0082_3804);
        }
        // Turing keeps its own block.
        let (fam, regs) = family_of(0x166_000A1).unwrap();
        assert_eq!(fam, "turing");
        assert_eq!(regs[0].1, 0x0040_9650);
    }
}
