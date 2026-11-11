//! GOP ordering surgery (issue #228: C621E + 4x GA102 handoff dark-screen
//! class).
//!
//! `LocateHandleBuffer(GOP)` order is the EDK2 handle-database creation
//! order, and every boot-graphics consumer takes the FIRST handle:
//! grub's efi_gop, the Linux efistub framebuffer (screen_info -> simpledrm),
//! bootmgr's boot-graphics walk - all sink-blind. A native-GOP rebuild
//! REPLACES the console card's child with a freshly created handle, so the
//! verified console child sorts LAST; every downstream consumer then draws
//! into whichever headless competitor was bound at POST (C621E: the BMC's
//! ASPEED VGA, bus 1-2), and stays dark until the OS GPU driver re-targets
//! to the connected output - Windows does at login (Dxgkrnl rebuilds the
//! topology from driver-reported connector state), a Linux box without a
//! display manager never does. Evidence: #228 loader.log probe order
//! [card4, ASPEED] stock vs the flipped order after the rebuild; journal
//! ast-fbcon + live nvidia fb1; X99 single-GPU control unaffected.
//!
//! Two surgeries, both refused unless ALL gates pass (read-only):
//!   1. the verified console child is enumerated exactly once,
//!   2. it carries EDID (Discovered or Active, >=128 bytes) - sink proof,
//!   3. the flip is in place (it is not already first),
//!   4. every competitor is EDID-less, on a DIFFERENT PCI controller than
//!      the console child, and (hide only) not matched by refusal rules.
//! Default since the #228 bench (2026-11-12, healthy-machine negative
//! control: console-already-first = Noop, zero writes):
//!   reorder - the competitor's controller drivers are stopped and
//!     restarted (parent-level disconnect + NON-recursive reconnect); the
//!     fresh child appends to the tail, so the verified console child
//!     becomes first while the competitor keeps working;
//!   \tx-gop-hide.txt - hard exclusion: listed BDFs (bb:dd.f / dddd:bb:dd.f)
//!     or vendor:device (vvvv:dddd) get disconnected without recreate;
//!   \tx-gop-no-reorder.txt - opts the reorder out (dump stays).
//! Readback after every write; ANY gate failure = loud log + no write
//! ("a wrong reconnect is worse than none", ordering edition). Scope note:
//! this fixes the in-session chain (unlock -> handoff -> OS, the replay
//! workflow); a warm reboot rebuilds the handle order firmware-side and is
//! out of scope here.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::ffi::c_void;
use core::ptr::null_mut;
use log::info;
use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams, SearchType};
use uefi::proto::console::gop::GraphicsOutput;
use uefi::{guid, Handle, Identify};

use crate::pci::PciIo;
use crate::tx_detach::{device_path_bytes, path_belongs_to};

const EDID_DISCOVERED_GUID: uefi::Guid = guid!("1c0c34f6-d380-41fa-a049-8ad06c1a66aa");
const EDID_ACTIVE_GUID: uefi::Guid = guid!("bd8b1056-9f36-4ec2-b72e-3adc17766705");

/// EFI_EDID_DISCOVERED_PROTOCOL / EFI_EDID_ACTIVE_PROTOCOL (MdePkg
/// EdidDiscovered.h): UINT32 SizeOfEdid + UINT8* Edid. Nothing else.
#[repr(C)]
struct EdidProto {
    size_of_edid: u32,
    edid: *mut u8,
}
const _: () = assert!(core::mem::offset_of!(EdidProto, edid) == 8);
const _: () = assert!(core::mem::size_of::<EdidProto>() == 16);

/// Raw HandleProtocol against an arbitrary GUID - EDID protocols are not
/// modeled in uefi-rs.
fn handle_protocol_edid(handle: Handle, guid: &uefi::Guid) -> Option<u32> {
    let st = uefi::table::system_table_raw()?;
    // Safety: boot services are live; HandleProtocol is read-only.
    unsafe {
        let bt = st.as_ref().boot_services;
        if bt.is_null() {
            return None;
        }
        let mut out: *mut c_void = null_mut();
        let status = ((*bt).handle_protocol)(handle.as_ptr(), guid, &mut out);
        if status != uefi::Status::SUCCESS || out.is_null() {
            return None;
        }
        let e = out as *mut EdidProto;
        let size = (*e).size_of_edid;
        if (*e).edid.is_null() || size < 128 {
            None
        } else {
            Some(size)
        }
    }
}

/// EDID bytes on this GOP child? Discovered first (driver-read over DDC),
/// Active as the firmware-processed fallback. Some(size) = sink proof.
pub fn edid_size(handle: Handle) -> Option<u32> {
    handle_protocol_edid(handle, &EDID_DISCOVERED_GUID)
        .or_else(|| handle_protocol_edid(handle, &EDID_ACTIVE_GUID))
}

// ---------------------------------------------------------------------------
// Pure decision (sim/src/gop_order.rs must stay textually identical from
// here through parse_hide; loader is UEFI-target-only and cannot run host
// tests - sim is the test bench).
// ---------------------------------------------------------------------------

/// One GOP handle in LocateHandleBuffer order: (carries EDID, is the
/// verified console child).
pub type GopEntry = (bool, bool);

#[derive(Debug, PartialEq)]
pub enum OrderDecision {
    /// Already first - nothing to do.
    Noop,
    /// Demote these indices (children to disconnect [+ recreate]) so the
    /// console child sorts first.
    Demote(Vec<usize>),
    /// Gate failed - loud log, no write.
    Refuse(&'static str),
}

/// The four gates as one pure pass over the enumerated order. All writes
/// happen only on `Demote`.
pub fn decide(order: &[GopEntry]) -> OrderDecision {
    let Some(console) = order.iter().position(|&e| e.1) else {
        return OrderDecision::Refuse("verified console child not enumerated");
    };
    if !order[console].0 {
        return OrderDecision::Refuse("console child carries no EDID - sink proof missing");
    }
    if console == 0 {
        return OrderDecision::Noop;
    }
    let mut demote = Vec::new();
    for (i, &(edid, _)) in order.iter().enumerate() {
        if i == console {
            continue;
        }
        if edid {
            return OrderDecision::Refuse("competitor carries EDID - ambiguous topology, use \\tx-gop-hide.txt");
        }
        demote.push(i);
    }
    OrderDecision::Demote(demote)
}

/// One `\tx-gop-hide.txt` entry: a PCI location (seg, bus, dev, fun) or a
/// vendor:device pair.
#[derive(Debug, PartialEq)]
pub enum HideKey {
    Bdf(usize, usize, usize, usize),
    DevId(u16, u16),
}

/// Parse one marker line. BDF lines contain a '.' ("bb:dd.f" or
/// "dddd:bb:dd.f", hex); vendor:device lines are exactly one ':' and two
/// 4-hex fields ("vvvv:dddd"). Returns None for anything unrecognized.
pub fn parse_hide_line(line: &str) -> Option<HideKey> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }
    if s.contains('.') {
        let (seg, rest) = if s.matches(':').count() == 2 {
            let (a, b) = s.split_once(':')?;
            (usize::from_str_radix(a, 16).ok()?, b)
        } else {
            (0, s)
        };
        let (bus_s, devfun) = rest.split_once(':')?;
        let (dev_s, fun_s) = devfun.split_once('.')?;
        let bus = usize::from_str_radix(bus_s, 16).ok()?;
        let dev = usize::from_str_radix(dev_s, 16).ok()?;
        let fun = usize::from_str_radix(fun_s, 16).ok()?;
        Some(HideKey::Bdf(seg, bus, dev, fun))
    } else {
        let (v, d) = s.split_once(':')?;
        if !v.contains(':') && v.len() <= 4 && d.len() <= 4 {
            let vendor = u16::from_str_radix(v, 16).ok()?;
            let device = u16::from_str_radix(d, 16).ok()?;
            Some(HideKey::DevId(vendor, device))
        } else {
            None
        }
    }
}

/// Parse the whole hide marker: lines, `#` comments, unknown lines skipped
/// (counted by the caller from the returned length).
pub fn parse_hide(raw: &str) -> Vec<HideKey> {
    raw.lines().filter_map(parse_hide_line).collect()
}

// ---------------------------------------------------------------------------
// EFI side
// ---------------------------------------------------------------------------

fn gop_handles() -> Option<Vec<Handle>> {
    let handles = boot::locate_handle_buffer(SearchType::ByProtocol(&GraphicsOutput::GUID)).ok()?;
    Some((0..handles.len()).map(|i| handles[i]).collect())
}

/// Open the PciIo of `handle` (GetProtocol only) and describe it as
/// "bb:dd.f vvvv:dddd" for hide-list matching and logging.
fn pci_id_str(handle: Handle) -> Option<String> {
    // Safety: GetProtocol performs no driver binding.
    let pci = unsafe {
        boot::open_protocol::<PciIo>(
            OpenProtocolParams { handle, agent: boot::image_handle(), controller: None },
            OpenProtocolAttributes::GetProtocol,
        )
    }
    .ok()?;
    let (seg, bus, dev, fun) = pci.get_location()?;
    let vendor = pci.config_read_u16(0x00);
    let device = pci.config_read_u16(0x02);
    Some(format!("{seg:02x}:{bus:02x}:{dev:02x}.{fun:x} {vendor:04x}:{device:04x}"))
}

/// The IMMEDIATE PCI parent of a GOP child: the LONGEST device path among
/// the PciIo handles that prefixes the child's path. A first-match walk is
/// wrong on multi-hop topologies - the C621E's ASPEED child hangs three PCI
/// hops deep (root port a190 -> AST1150 -> ASPEED GPU), and the first
/// prefix match is the outermost ANCESTOR, whose disconnect/reconnect cycle
/// is a no-op on the GPU driver (the #228 bench run 2 caught it: parent
/// disconnect Ok on the root port, reconnect NOT_FOUND, ASPEED child
/// untouched).
fn parent_pci_of(child: Handle) -> Option<Handle> {
    let child_path = device_path_bytes(child)?;
    let handles = boot::locate_handle_buffer(SearchType::ByProtocol(&PciIo::GUID)).ok()?;
    let mut best: Option<(usize, Handle)> = None;
    for i in 0..handles.len() {
        let p = handles[i];
        if let Some(pp) = device_path_bytes(p) {
            if pp.len() <= child_path.len() && path_belongs_to(&pp, &child_path) {
                match best {
                    Some((len, _)) if len >= pp.len() => {}
                    _ => best = Some((pp.len(), p)),
                }
            }
        }
    }
    best.map(|(_, h)| h)
}

/// Read-only full order dump: handle, EDID presence, console flag. Runs on
/// every console rebound regardless of knobs - the first card's probe logs
/// the stock order and the console card's rebound logs the flipped order,
/// so one unlock run carries before/after evidence.
fn dump_order(console_child: Handle) -> Option<Vec<Handle>> {
    let handles = gop_handles()?;
    for (i, h) in handles.iter().enumerate() {
        let edid = match edid_size(*h) {
            Some(n) => alloc::format!("yes({n}B)"),
            None => "no".to_string(),
        };
        let role = if *h == console_child { " <console" } else { "" };
        // pci_id_str opens PciIo, which lives on the PCI CONTROLLER - the
        // GOP child only extends its path. Resolve the parent first.
        let owner = parent_pci_of(*h)
            .and_then(|p| pci_id_str(p))
            .unwrap_or_else(|| "no-pci-parent".to_string());
        info!("GOP-ORDER: [{i}] {:?} edid={edid}{role} ({owner})", h);
    }
    Some(handles)
}

/// Entry point, called from gop_native::recover after "display path
/// verified" - i.e. ONLY on the path where a verified console child exists.
/// Dump is unconditional (read-only); the surgeries are marker-gated.
pub fn after_console_rebound(console_child: Handle) {
    let Some(handles) = dump_order(console_child) else {
        info!("GOP-ORDER: no GOP handles enumerable - nothing to do");
        return;
    };

    // \tx-gop-hide.txt first (operator intent overrides the auto path).
    if let Some(raw) = crate::tx::read_boot_file("\\tx-gop-hide.txt") {
        let text = String::from_utf8_lossy(&raw);
        run_hide(console_child, &handles, &text);
        return;
    }
    // Default ON since the #228 bench (2026-11-12): the healthy-machine
    // negative control confirmed the zero-write path (console child already
    // first = Noop), and the demote path only ever runs on the actual flip
    // with EDID sink proof on the console child - "start with the display
    // output the machine itself chose" is the boot-native semantics.
    // \tx-gop-no-reorder.txt opts out; the legacy enable marker
    // \tx-gop-reorder.txt is still accepted and is now a no-op.
    if crate::tx::probe_file("\\tx-gop-no-reorder.txt") {
        info!("GOP-REORDER: \\tx-gop-no-reorder.txt present - ordering surgery skipped (opt-out)");
        return;
    }
    run_reorder(console_child, &handles);
}

fn run_hide(console_child: Handle, handles: &[Handle], raw: &str) {
    let keys = parse_hide(raw);
    if keys.is_empty() {
        info!("GOP-HIDE: marker present but no usable line - refusing to guess, nothing hidden");
        return;
    }
    let Some(console_parent) = parent_pci_of(console_child) else {
        info!("GOP-HIDE: console child has no PCI parent - refusing");
        return;
    };
    // Pass 1 - resolve every candidate BEFORE any write: a hide list that
    // touches the console card's own controller tears the verified console
    // (the sibling-head rule), so the whole operation must refuse up front
    // instead of hiding half the list first.
    let mut actions: Vec<(usize, Handle, String)> = Vec::new();
    for (i, h) in handles.iter().enumerate() {
        if *h == console_child {
            continue;
        }
        let Some(parent) = parent_pci_of(*h) else {
            info!("GOP-HIDE: [{i}] no PCI parent - skipped");
            continue;
        };
        if !keys.iter().any(|k| key_matches(k, parent)) {
            continue;
        }
        if parent == console_parent {
            info!("GOP-HIDE: [{i}] matches the console card itself - whole operation refused");
            return;
        }
        let id = pci_id_str(parent).unwrap_or_else(|| "unknown-controller".to_string());
        actions.push((i, *h, id));
    }
    // Pass 2 - writes (pre-call line per #128: a hang lands BETWEEN lines).
    let mut hid = 0usize;
    for (i, child, id) in &actions {
        info!("GOP-HIDE: [{i}] {id} disconnecting (hang lands here)");
        let st = boot::disconnect_controller(*child, None, None);
        hid += 1;
        info!("GOP-HIDE: [{i}] {id} disconnected ({st:?})");
    }
    // Readback (constitution): the console child must still be enumerated,
    // and the count must have dropped by `hid`.
    let after = gop_handles();
    match after {
        Some(v) if v.contains(&console_child) && v.len() + hid == handles.len() => {
            info!("GOP-HIDE: readback ok - {} GOP(s) hidden, console child still enumerated", v.len());
        }
        _ => {
            info!("GOP-HIDE: READBACK FAILED - GOP count or console child unexpected; state NOT rolled back, inspect before next boot");
        }
    }
}

fn key_matches(key: &HideKey, pci_handle: Handle) -> bool {
    let Some(pci) = (unsafe {
        boot::open_protocol::<PciIo>(
            OpenProtocolParams { handle: pci_handle, agent: boot::image_handle(), controller: None },
            OpenProtocolAttributes::GetProtocol,
        )
    })
    .ok() else { return false };
    match key {
        HideKey::Bdf(seg, bus, dev, fun) => pci.get_location() == Some((*seg, *bus, *dev, *fun)),
        HideKey::DevId(vendor, device) => {
            pci.config_read_u16(0x00) == *vendor && pci.config_read_u16(0x02) == *device
        }
    }
}

fn run_reorder(console_child: Handle, handles: &[Handle]) {
    let order: Vec<GopEntry> = handles.iter().map(|h| (edid_size(*h).is_some(), *h == console_child)).collect();
    match decide(&order) {
        OrderDecision::Noop => {
            info!("GOP-REORDER: console child already first - nothing to do");
        }
        OrderDecision::Refuse(reason) => {
            info!("GOP-REORDER: refused: {reason}");
        }
        OrderDecision::Demote(idxs) => {
            let Some(console_parent) = parent_pci_of(console_child) else {
                info!("GOP-REORDER: console child has no PCI parent - refusing");
                return;
            };
            for i in idxs {
                let child = handles[i];
                let Some(parent) = parent_pci_of(child) else {
                    info!("GOP-REORDER: [{i}] no PCI parent - skipped");
                    continue;
                };
                let owner = pci_id_str(parent).unwrap_or_else(|| "no-pci-parent".into());
                if parent == console_parent {
                    // A sibling head of the console card: demoting it would
                    // tear the console card's own controller.
                    info!("GOP-REORDER: [{i}] {owner} is the console card itself - skipped");
                    continue;
                }
                // Pre-call line (issue #128 discipline): a hang inside the
                // driver cycle must land BETWEEN two flushed lines.
                info!("GOP-REORDER: [{i}] {owner} demoting - parent driver cycle next (hang lands here)");
                let st1 = boot::disconnect_controller(parent, None, None);
                let st2 = boot::connect_controller(parent, &[], None, false);
                info!("GOP-REORDER: [{i}] {owner} demoted (parent disconnect={st1:?} reconnect={st2:?})");
            }
            // Readback (constitution): the console child must now sort first.
            match gop_handles() {
                Some(after) => {
                    let pos = after.iter().position(|h| *h == console_child);
                    let lost = handles.len().saturating_sub(after.len());
                    match pos {
                        Some(0) if lost == 0 => info!(
                            "GOP-REORDER: readback ok - console child is first of {} GOP(s), competitors recreated behind it",
                            after.len()
                        ),
                        Some(0) => info!(
                            "GOP-REORDER: readback ok (degraded) - console child is first of {} GOP(s); {lost} competitor handle(s) did not rebind and were removed - ordering goal met",
                            after.len()
                        ),
                        _ => info!("GOP-REORDER: READBACK FAILED - console child at {pos:?} of {} GOP(s); state NOT rolled back", after.len()),
                    }
                }
                None => info!("GOP-REORDER: READBACK FAILED - no GOP handles enumerable"),
            }
        }
    }
}
