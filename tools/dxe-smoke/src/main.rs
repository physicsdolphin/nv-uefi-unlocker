//! QEMU-only negative control. Injects an FFS into a RAM FV, invokes the
//! real DXE dispatcher, then signals ReadyToBoot. Never use on real hardware.
#![no_std]
#![no_main]
extern crate alloc;
use loader::{dxe, dxe_policy, volumes};
use uefi::boot::{AllocateType, EventType, MemoryType, Tpl};
use uefi::proto::loaded_image::LoadedImage;
use uefi::runtime::VariableVendor;
use uefi::{boot, guid, prelude::*, runtime, system, table::cfg::ConfigTableEntry, CStr16};
const FAIL: &CStr16 = cstr16!("NvUnlockDxeFail");
const LATCH: &CStr16 = cstr16!("NvUnlockDxeRan");
const READY: uefi::Guid = guid!("7ce88fb3-4bd7-4679-87a8-a8d8dee50d2b");

// PI DXE services layout, from MdePkg/Include/Pi/PiDxeCis.h.
#[repr(C)]
struct DxeServices {
    header: [u8; 24],
    gcd: [usize; 13],
    dispatch: unsafe extern "efiapi" fn() -> Status,
    schedule: usize,
    trust: usize,
    process_fv: unsafe extern "efiapi" fn(*const u8, usize, *mut *mut core::ffi::c_void) -> Status,
}

fn readvar(name: &CStr16) -> Option<alloc::boxed::Box<[u8]>> {
    runtime::get_variable_boxed(name, &VariableVendor(dxe::ARM_VENDOR))
        .ok()
        .map(|x| x.0)
}
fn putvar(name: &CStr16, data: &[u8]) {
    runtime::set_variable(name, &VariableVendor(dxe::ARM_VENDOR), dxe::ATTRS_NV, data).unwrap();
}
fn check(condition: bool, text: &str) {
    uefi::println!(
        "SMOKE {}: {}",
        if condition { "PASS" } else { "FAIL" },
        text
    );
    assert!(condition, "{text}");
}

unsafe extern "efiapi" fn noop(_: uefi::Event, _: Option<core::ptr::NonNull<core::ffi::c_void>>) {}

#[entry]
fn main() -> Status {
    boot::set_watchdog_timer(0, 0x10000, None).unwrap();
    let mode = volumes::read_small_file("\\mode.txt", 64).unwrap().unwrap();
    let mode = core::str::from_utf8(&mode).unwrap().trim();
    uefi::println!("SMOKE mode={mode}");
    let _ = runtime::delete_variable(LATCH, &VariableVendor(dxe::ARM_VENDOR));
    let image = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle()).unwrap();
    let identity = volumes::partition_guid(image.device().unwrap()).unwrap();
    drop(image);
    let snapshot = volumes::read_policy().unwrap();
    let mut grant = dxe_policy::encode(identity, &snapshot);
    if mode == "tamper" {
        grant[52] ^= 1;
    }
    if mode == "oversized-arm" {
        grant.resize(65, 0);
    }
    putvar(dxe::ARM_VARIABLE, &grant);
    putvar(
        FAIL,
        match mode {
            "exhausted" => b"2",
            "malformed-counter" => b"1junk",
            _ => b"0",
        },
    );

    let ffs = volumes::read_small_file("\\driver.ffs", 1 << 20)
        .unwrap()
        .unwrap();
    let size = (72 + ffs.len() + 4095) & !4095;
    let mem = boot::allocate_pages(
        AllocateType::AnyPages,
        MemoryType::BOOT_SERVICES_DATA,
        size / 4096,
    )
    .unwrap();
    // Intentionally retained until QEMU exits: FV protocol owns this memory.
    let fv = unsafe { core::slice::from_raw_parts_mut(mem.as_ptr(), size) };
    fv.fill(0xff);
    fv[..72].fill(0);
    fv[16..32].copy_from_slice(&[
        0x78, 0xe5, 0x8c, 0x8c, 0x3d, 0x8a, 0x1c, 0x4f, 0x99, 0x35, 0x89, 0x61, 0x85, 0xc3, 0x2d,
        0xd3,
    ]);
    fv[32..40].copy_from_slice(&(size as u64).to_le_bytes());
    fv[40..44].copy_from_slice(b"_FVH");
    fv[44..48].copy_from_slice(&0x0004feffu32.to_le_bytes());
    fv[48..50].copy_from_slice(&72u16.to_le_bytes());
    fv[55] = 2;
    fv[56..60].copy_from_slice(&((size / 4096) as u32).to_le_bytes());
    fv[60..64].copy_from_slice(&4096u32.to_le_bytes());
    let sum = fv[..72].chunks_exact(2).fold(0u16, |n, b| {
        n.wrapping_add(u16::from_le_bytes([b[0], b[1]]))
    });
    fv[50..52].copy_from_slice(&sum.wrapping_neg().to_le_bytes());
    fv[72..72 + ffs.len()].copy_from_slice(&ffs);
    let ds = system::with_config_table(|tables| {
        tables
            .iter()
            .find(|t| t.guid == ConfigTableEntry::DXE_SERVICES_GUID)
            .unwrap()
            .address
            .cast::<DxeServices>()
    });
    let mut handle = core::ptr::null_mut();
    let status = unsafe { ((*ds).process_fv)(fv.as_ptr(), size, &mut handle) };
    uefi::println!("SMOKE process_fv={status:?}");
    check(status == Status::SUCCESS, "RAM FV accepted");
    let status = unsafe { ((*ds).dispatch)() };
    uefi::println!("SMOKE dispatch={status:?}");
    check(status == Status::SUCCESS, "FFS dispatched by DxeCore");
    check(
        readvar(LATCH).is_none(),
        "FV entry deferred despite BDS_ARCH presence",
    );

    let event = unsafe {
        boot::create_event_ex(
            EventType::NOTIFY_SIGNAL,
            Tpl::CALLBACK,
            Some(noop),
            None,
            Some(&READY),
        )
    }
    .unwrap();
    let st = uefi::table::system_table_raw().unwrap().as_ptr();
    let consoles = unsafe { ((*st).stdin, (*st).stdout) };
    if mode == "headless" {
        unsafe {
            (*st).stdin = core::ptr::null_mut();
            (*st).stdout = core::ptr::null_mut();
        }
    }
    boot::signal_event(&event).unwrap();
    unsafe {
        (*st).stdin = consoles.0;
        (*st).stdout = consoles.1;
    }
    let count = readvar(FAIL).unwrap();
    uefi::println!("SMOKE count={}", core::str::from_utf8(&count).unwrap());
    let expected: &[u8] = match mode {
        "exhausted" => b"2",
        "oversized-arm" => b"0",
        "malformed-counter" => b"1junk",
        _ => b"1",
    };
    check(count.as_ref() == expected, "attempt budget persisted");
    check(
        readvar(LATCH).as_deref()
            == if mode == "oversized-arm" {
                None
            } else {
                Some(b"1".as_slice())
            },
        "boot latch established or invalid authority refused",
    );
    boot::signal_event(&event).unwrap();
    check(
        readvar(FAIL).as_deref() == Some(count.as_ref()),
        "repeated ReadyToBoot did not rerun",
    );
    // A second image instance has fresh Rust statics. The shared NVRAM
    // latch must still stop it, not merely the first instance's AtomicBool.
    let section_size =
        usize::from(ffs[24]) | (usize::from(ffs[25]) << 8) | (usize::from(ffs[26]) << 16);
    let second = boot::load_image(
        boot::image_handle(),
        boot::LoadImageSource::FromBuffer {
            buffer: &ffs[28..24 + section_size],
            file_path: None,
        },
    )
    .unwrap();
    boot::start_image(second).unwrap();
    boot::signal_event(&event).unwrap();
    check(
        readvar(FAIL).as_deref() == Some(count.as_ref()),
        "second image instance did not rerun",
    );
    uefi::println!("SMOKE COMPLETE mode={mode}");
    // QEMU isa-debug-exit, never a hardware transaction.
    unsafe {
        core::arch::asm!("out dx, eax", in("dx") 0xf4u16, in("eax") 0x10u32);
    }
    Status::SUCCESS
}
