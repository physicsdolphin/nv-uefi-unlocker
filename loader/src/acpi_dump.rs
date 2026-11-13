//! `\tx-acpi-dump.txt` knob: dump the firmware ACPI tables (DSDT + every
//! SSDT) to `\acpi\` on the stick, plus a keyword index.
//!
//! Why this exists: the relock family ends in arguments about who owns the
//! dGPU's power. The concrete answer lives in the platform AML - does the
//! root port's device have a `_PR3` power resource, are `_ON`/`_OFF`
//! idempotent, which GPIO carries PERST# (FAQ chapter `#relock`, section
//! "who actually cuts the power"). Reading AML needs a decompiler, which a
//! UEFI app has no business embedding: this dumps raw bytes and marks the
//! offsets where the interesting names occur, so `iasl -d` on a dev box
//! gives the answer.
//!
//! Purely read-only: no GPU, no writes outside `\acpi\`, no effect on the
//! transaction. Absent marker = one log line and return.

use alloc::vec::Vec;
use log::info;
use uefi::proto::media::file::{Directory, File, FileAttribute, FileMode};
use uefi::table::cfg::ConfigTableEntry;
use uefi::{cstr16, CStr16};

const MARKER: &CStr16 = cstr16!(r"\tx-acpi-dump.txt");
const OUT_DIR: &CStr16 = cstr16!(r"\acpi");

/// Per-table and total caps: AML tables are a few hundred KiB at most;
/// anything bigger is a broken header, and the stick is FAT32.
const MAX_TABLE: usize = 1024 * 1024;
const MAX_TOTAL: usize = 8 * 1024 * 1024;
const MAX_TABLES: usize = 64;

/// Names that decide the power-ownership question. AML pads short names
/// with `_`, so `_ON` really is on disk as `_ON_`.
const NEEDLES: [&[u8]; 14] = [
    b"_PR3", b"_ON_", b"_OFF", b"PRST", b"_ADR", b"_HID", b"_CID", b"GPIO", b"_DSM", b"_STA",
    b"_PS0", b"_PS3", b"_PXM", b"_DOD",
];

pub fn run_if_marker() {
    if !crate::chainload::boot_volume_has(MARKER) {
        return;
    }
    info!("ACPI-DUMP: \\tx-acpi-dump.txt present - dumping firmware tables to \\acpi\\ (read-only)");

    let Some(rsdp) = find_rsdp() else {
        info!("ACPI-DUMP: no RSDP in the config table - nothing to dump");
        return;
    };
    // RSDP: sig(8) checksum(1) oemid(6) revision(1) rsdt(4) [length(4)
    // xsdt(8) ext checksum(1) reserved(3)] - ACPI 6.x, 6.2.2.
    let sig = read::<8>(rsdp);
    if &sig[..] != b"RSD PTR " {
        info!("ACPI-DUMP: RSDP signature mismatch - aborting");
        return;
    }
    let rev = unsafe { *rsdp.add(15) };
    let rsdt = read_u32(unsafe { rsdp.add(16) });
    let (xsdt, len) = if rev >= 2 {
        (read_u64(unsafe { rsdp.add(24) }), read_u32(unsafe { rsdp.add(20) }) as usize)
    } else {
        (0, 20)
    };
    let len = len.clamp(20, 64);
    if checksum(rsdp, len) != 0 {
        info!("ACPI-DUMP: RSDP checksum bad - dumping anyway (firmware tables keep their own checksums)");
    }
    info!("ACPI-DUMP: RSDP rev={rev} rsdt={rsdt:#x} xsdt={xsdt:#x}");
    if xsdt != 0 {
        if let Err(e) = dump_tree(xsdt as *const u8, 8) {
            info!("ACPI-DUMP: XSDT walk stopped: {e}");
        }
    } else if rsdt != 0 {
        if let Err(e) = dump_tree(rsdt as *const u8, 4) {
            info!("ACPI-DUMP: RSDT walk stopped: {e}");
        }
    } else {
        info!("ACPI-DUMP: both RSDT and XSDT are null");
    }
}

fn find_rsdp() -> Option<*const u8> {
    let mut r: *const u8 = core::ptr::null();
    // Two passes so the ACPI 2 RSDP (with XSDT) always wins.
    uefi::system::with_config_table(|entries| {
        for guid in [ConfigTableEntry::ACPI2_GUID, ConfigTableEntry::ACPI_GUID] {
            if !r.is_null() {
                break;
            }
            for e in entries {
                if e.guid == guid && !e.address.is_null() {
                    r = e.address as *const u8;
                    break;
                }
            }
        }
    });
    if r.is_null() { None } else { Some(r) }
}

/// Walk one table-of-tables. `width` is 8 for XSDT, 4 for RSDT.
fn dump_tree(tree: *const u8, width: usize) -> Result<(), &'static str> {
    let len = read_u32(unsafe { tree.add(4) }) as usize;
    if !(36..=MAX_TABLE).contains(&len) {
        return Err("tree header length out of range");
    }
    if checksum(tree, len) != 0 {
        info!("ACPI-DUMP: table-of-tables checksum bad (proceeding)");
    }
    let n = ((len - 36) / width).min(MAX_TABLES);
    info!("ACPI-DUMP: tree len={len} entries={n}");

    let mut fs = uefi::boot::get_image_file_system(uefi::boot::image_handle())
        .map_err(|_| "image filesystem unavailable")?;
    let mut root = fs.open_volume().map_err(|_| "open_volume failed")?;
    // CreateReadWrite + DIRECTORY on open == mkdir; an existing directory
    // opens the same way, so the marker can be run repeatedly.
    let mut dir = root
        .open(OUT_DIR, FileMode::CreateReadWrite, FileAttribute::DIRECTORY)
        .map_err(|_| "acpi directory unavailable")?
        .into_directory()
        .ok_or("acpi path is not a directory")?;
    let mut out = Vec::new();
    let mut total = 0usize;

    let mut facp: Option<*const u8> = None;
    let mut saw_dsdt = false;

    for i in 0..n {
        let entry = unsafe {
            let p = tree.add(36 + i * width);
            if width == 8 { read_u64(p) } else { read_u32(p) as u64 }
        };
        if entry == 0 {
            continue;
        }
        let t = entry as *const u8;
        let sig = read::<4>(t);
        info!("ACPI-DUMP: entry[{i}]={entry:#x} sig={}", core::str::from_utf8(&sig).unwrap_or("????"));
        if &sig[..] == b"FACP" {
            facp = Some(t);
        }
        if &sig[..] != b"DSDT" && &sig[..] != b"SSDT" {
            continue;
        }
        if &sig[..] == b"DSDT" {
            saw_dsdt = true;
        }
        dump_table(&mut dir, t, sig, &mut out, &mut total);
    }

    // The DSDT is normally NOT an XSDT entry: the FACP carries its address
    // in the DSDT (offset 140, 32-bit) / X_DSDT (148, 64-bit) fields. Follow
    // it when the tree did not hand us one.
    if !saw_dsdt {
        match facp {
            None => info!("ACPI-DUMP: no FACP in the tree - DSDT address unknown"),
            Some(f) => {
                let flen = read_u32(unsafe { f.add(4) }) as usize;
                // FACP: DSDT (32-bit) is at +40; X_DSDT (64-bit) at +140
                // (ACPI 6.x, table 5.11). The X_ variant is only present
                // from revision 2 / length 148 on.
                let d32 = read_u32(unsafe { f.add(40) });
                let xd = if flen >= 148 { read_u64(unsafe { f.add(140) }) } else { 0 };
                let d = if xd != 0 { xd } else { d32 as u64 };
                info!("ACPI-DUMP: FACP len={flen} dsdt32={d32:#x} x_dsdt={xd:#x}");
                if d == 0 {
                    info!("ACPI-DUMP: FACP has no DSDT pointer (firmware may build it later)");
                } else {
                    let t = d as *const u8;
                    let sig = read::<4>(t);
                    if &sig[..] == b"DSDT" {
                        dump_table(&mut dir, t, sig, &mut out, &mut total);
                    } else {
                        info!("ACPI-DUMP: FACP DSDT pointer {d:#x} holds {:?}, not DSDT", sig);
                    }
                }
            }
        }
    }

    let mut index = Vec::new();
    index.extend_from_slice(
        b"# ACPI table dump (tx-acpi-dump). Raw AML; decompile on a dev box:\n\
          #   iasl -d dsdt.aml ssdt-*.aml\n\
          # Index lines: <SIG> len=.. checksum=ok|BAD <file> creator=.. then\n\
          # NAME=hits@offset for every keyword found. _PR3 on the root port's\n\
          # device means a power resource exists; check whether _ON/_OFF are idempotent.\n",
    );
    index.extend_from_slice(&out);
    write_file(&mut dir, cstr16!("index.txt"), &index).map_err(|_| "index write failed")?;
    info!("ACPI-DUMP: done, {} bytes of AML written to \\acpi\\", total);
    Ok(())
}

/// Validate and write one table. Failures are recorded in the index and do
/// not abort the walk - a broken SSDT must not cost us the DSDT.
fn dump_table(dir: &mut Directory, t: *const u8, sig: [u8; 4], out: &mut Vec<u8>, total: &mut usize) {
    let len = read_u32(unsafe { t.add(4) }) as usize;
    let ok = if checksum(t, len) == 0 { "ok" } else { "BAD" };
    let mut l = line_head(&sig, len, ok);
    if !(36..=MAX_TABLE).contains(&len) {
        l.push_fmt(format_args!(" SKIPPED (length out of range)"));
        push_line(out, &l);
        return;
    }
    let (name, idx) = if &sig[..] == b"DSDT" {
        (cstr16!("dsdt.aml"), usize::MAX)
    } else {
        ssdt_name()
    };
    if idx == usize::MAX {
        l.push_fmt(format_args!(" dsdt.aml"));
    } else {
        l.push_fmt(format_args!(" ssdt-{idx:02x}.aml"));
    }
    // Creator id at +28 ("INTL" and friends) tells us whose AML this is;
    // the body starts at +36.
    let creator = read::<4>(unsafe { t.add(28) });
    if creator.iter().all(|b| b.is_ascii_graphic()) {
        l.push_fmt(format_args!(" creator={}", core::str::from_utf8(&creator).unwrap_or("")));
    }
    if *total + len > MAX_TOTAL {
        l.push_fmt(format_args!(" SKIPPED (total cap reached)"));
        push_line(out, &l);
        return;
    }
    let hay = unsafe { core::slice::from_raw_parts(t, len) };
    // Header is 36 bytes; only the body can carry names.
    for n in NEEDLES {
        let (hits, first) = scan(&hay[36..], n);
        if hits > 0 {
            l.push_fmt(format_args!(
                " {}={hits}@{:#x}",
                core::str::from_utf8(n).unwrap_or("?"),
                first + 36
            ));
        }
    }
    match write_file(dir, name, hay) {
        Ok(()) => {
            *total += len;
            l.push_fmt(format_args!(" written"));
        }
        Err(e) => l.push_fmt(format_args!(" WRITE FAILED ({e})")),
    }
    push_line(out, &l);
}

/// Index lines are newline-terminated; without it every entry ran together
/// on the first field run.
fn push_line(out: &mut Vec<u8>, l: &Line) {
    out.extend_from_slice(l.as_bytes());
    out.push(b'\n');
}

/// `ssdt-NN.aml` built at compile time is not possible per index, so render
/// into a static buffer per call. Also returns the index for the index line.
/// The name is RELATIVE to the open `\acpi` directory handle (a full
/// `\acpi\...` path does not resolve there) - that mistake made every SSDT
/// write fail on the first field run.
fn ssdt_name() -> (&'static CStr16, usize) {
    use core::sync::atomic::{AtomicU16, Ordering};
    static SLOT: AtomicU16 = AtomicU16::new(0);
    // One name slot per call site is enough for the sequential walk; the
    // write happens before the next call renders the next name.
    const HEX: &[u8; 16] = b"0123456789abcdef";
    // Safety: single-threaded boot services, and the buffer is only read
    // inside the same call chain before the next render.
    static mut BUF: [u16; 17] = [0; 17];
    let n = SLOT.fetch_add(1, Ordering::Relaxed) as usize;
    // 12 units = 11 chars + the NUL that `[0u16; 12]` leaves at the end.
    let mut b = [0u16; 12];
    let s = r"ssdt-";
    for (k, ch) in s.bytes().enumerate() {
        b[k] = ch as u16;
    }
    b[5] = HEX[(n >> 4) & 0xf] as u16;
    b[6] = HEX[n & 0xf] as u16;
    b[7] = b'.' as u16;
    b[8] = b'a' as u16;
    b[9] = b'm' as u16;
    b[10] = b'l' as u16;
    // Write through a raw pointer: no `static mut` reference is taken, so
    // the borrow cannot overlap with the next render.
    let sp = core::ptr::addr_of_mut!(BUF) as *mut u16;
    let cs = unsafe {
        for (k, v) in b.iter().enumerate() {
            core::ptr::write(sp.add(k), *v);
        }
        CStr16::from_u16_with_nul_unchecked(core::slice::from_raw_parts(sp, b.len()))
    };
    (cs, n)
}

fn write_file(dir: &mut Directory, name: &CStr16, data: &[u8]) -> Result<(), &'static str> {
    // CreateReadWrite never truncates: delete a stale file first.
    if let Ok(h) = dir.open(name, FileMode::ReadWrite, FileAttribute::empty()) {
        if let Some(reg) = h.into_regular_file() {
            let _ = reg.delete();
        }
    }
    let handle = dir
        .open(name, FileMode::CreateReadWrite, FileAttribute::empty())
        .map_err(|_| "create failed")?;
    let Some(mut file) = handle.into_regular_file() else {
        return Err("not a regular file");
    };
    file.write(data).map_err(|_| "write failed")?;
    file.flush().map_err(|_| "flush failed")
}

fn scan(hay: &[u8], needle: &[u8]) -> (usize, usize) {
    let mut hits = 0;
    let mut first = 0;
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        if &hay[i..i + needle.len()] == needle {
            if hits == 0 {
                first = i;
            }
            hits += 1;
        }
        i += 1;
    }
    (hits, first)
}

fn checksum(p: *const u8, len: usize) -> u8 {
    let mut sum = 0u8;
    for i in 0..len {
        sum = sum.wrapping_add(unsafe { *p.add(i) });
    }
    sum
}

fn read<const N: usize>(p: *const u8) -> [u8; N] {
    let mut b = [0u8; N];
    for (i, slot) in b.iter_mut().enumerate() {
        *slot = unsafe { *p.add(i) };
    }
    b
}

fn read_u32(p: *const u8) -> u32 {
    u32::from_le_bytes(read::<4>(p))
}

fn read_u64(p: *const u8) -> u64 {
    let lo = read_u32(p) as u64;
    let hi = read_u32(unsafe { p.add(4) }) as u64;
    lo | (hi << 32)
}

/// Tiny append-only formatter (no `alloc::format!` in no_std without a
/// formatter allocation, and these lines are built in a loop).
struct Line {
    buf: [u8; 320],
    len: usize,
}

impl Line {
    fn push_fmt(&mut self, args: core::fmt::Arguments<'_>) {
        use core::fmt::Write;
        struct W<'a>(&'a mut Line);
        impl core::fmt::Write for W<'_> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                for &b in s.as_bytes() {
                    if self.0.len < self.0.buf.len() {
                        self.0.buf[self.0.len] = b;
                        self.0.len += 1;
                    }
                }
                Ok(())
            }
        }
        let _ = W(self).write_fmt(args);
    }
    fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

fn line_head(sig: &[u8], len: usize, ok: &str) -> Line {
    let mut l = Line { buf: [0; 320], len: 0 };
    l.push_fmt(format_args!(
        "{} len={len} checksum={ok}",
        core::str::from_utf8(sig).unwrap_or("????")
    ));
    l
}
