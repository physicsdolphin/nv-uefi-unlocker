//! TOTORONG-lineage staged driver detach + exclusive claim (msi_release.c
//! semantics): the transaction only works after ALL console and vendor
//! drivers are off the GPU and the PciIo is EXCLUSIVELY owned. Three stages
//! (console agents on GOP, then the vendor driver on PCI), each verified by
//! an open-protocol count delta; then exclusive claim + IO|MEM|BME enable +
//! config snapshot baseline. Reconnect is NOT performed: after FLR the
//! firmware BDS re-discovers the device (TOTORONG never reconnects either).
#![allow(dead_code)]

use alloc::vec;
use log::info;
use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams};
use uefi::proto::console::gop::GraphicsOutput;
use uefi::proto::device_path::DevicePath;
use uefi::proto::loaded_image::LoadedImage;
use uefi::{Handle, Identify};

use crate::pci::PciIo;

fn open_count(handle: Handle) -> usize {
    match boot::protocols_per_handle(handle) {
        Ok(per) => per.iter().count(),
        Err(_) => 0,
    }
}

/// Raw device-path bytes of a handle (same technique as the chainload
/// self-volume fingerprint).
pub(crate) fn device_path_bytes(handle: Handle) -> Option<alloc::vec::Vec<u8>> {
    let dp = unsafe {
        boot::open_protocol::<DevicePath>(
            OpenProtocolParams { handle, agent: boot::image_handle(), controller: None },
            OpenProtocolAttributes::GetProtocol,
        )
    }
    .ok()?;
    Some(dp.as_bytes().to_vec())
}

/// Node-aligned device-path ownership: `child` belongs to `parent` when the
/// parent's node sequence (its END node excluded) appears byte-exact at the
/// same offsets in `child`.
///
/// The raw byte-prefix compare this replaces failed on every NVIDIA GOP so
/// far seen: the child path replaces the parent's trailing END node with
/// extra nodes (an AcpiAdr display node) before its own END, so the compare
/// tripped over the parent's END bytes and the target's own GOP read as
/// "foreign". Instances: the issue #14 flip board, the ASUS Z790 2803
/// report (stage 1 never ran there), and the TU106 1f07 regression log of
/// 2026-10-28 (GOP-TOPOLOGY: 36 B present, gone after the GPU PCI
/// disconnect, back after reconnect - the dGPU's own GOP, judged foreign).
/// Reference implementation + shape tests: sim/src/devpath.rs.
pub fn path_belongs_to(parent: &[u8], child: &[u8]) -> bool {
    // Byte offset where the parent's END node (type 0x7f) begins; a
    // malformed node (length < 4 or past the buffer) ends the walk.
    let mut body = 0usize;
    let mut nodes = 0usize;
    let mut i = 0usize;
    while i + 4 <= parent.len() {
        let len = u16::from_le_bytes([parent[i + 2], parent[i + 3]]) as usize;
        if len < 4 || i + len > parent.len() || parent[i] == 0x7f {
            break;
        }
        body = i + len;
        nodes += 1;
        i += len;
    }
    if nodes == 0 {
        return false;
    }
    child.len() >= body && child[..body] == parent[..body]
}

/// Does this GOP handle belong to the target GPU? The vendor GOP child
/// handle's device path extends the PCI device path, so a node-aligned
/// prefix match identifies ownership. A false "no" is safe: the stage-3
/// recursive disconnect tears the target's own stack regardless - the
/// check only decides whether we ALSO tear down other cards' GOPs.
fn gop_belongs_to_target(gop: Handle, pci_handle: Handle) -> bool {
    match (device_path_bytes(pci_handle), device_path_bytes(gop)) {
        (Some(p), Some(g)) => path_belongs_to(&p, &g),
        _ => false,
    }
}

/// EFI_OPEN_PROTOCOL_INFORMATION_ENTRY (UEFI spec 7.3).
#[repr(C)]
struct OpenInfo {
    agent: *mut core::ffi::c_void,
    controller: *mut core::ffi::c_void,
    attributes: u32,
    open_count: u32,
}

/// Read-only: log every open of `guid` on `handle` (agent, controller,
/// attributes) via OpenProtocolInformation. Attributes: 0x8 BY_CHILD,
/// 0x10 BY_DRIVER, 0x20 EXCLUSIVE. Maps which drivers sit on which display
/// handle before anything is disconnected.
fn log_openers(label: &str, handle: Handle, guid: &uefi::Guid) {
    let Some(st) = uefi::table::system_table_raw() else {
        return;
    };
    // Safety: boot services are live; the table pointer comes from the
    // firmware and the returned buffer is freed with FreePool below.
    unsafe {
        let bt = st.as_ref().boot_services;
        if bt.is_null() {
            return;
        }
        let mut buf: *mut OpenInfo = core::ptr::null_mut();
        let mut n: usize = 0;
        let s = ((*bt).open_protocol_information)(
            handle.as_ptr(),
            guid,
            (&mut buf as *mut *mut OpenInfo).cast(),
            &mut n,
        );
        if s != uefi::Status::SUCCESS || buf.is_null() {
            info!("DIAG {label}: {:?} OpenProtocolInformation {:?}", handle, s);
            return;
        }
        for i in 0..n {
            let e = &*buf.add(i);
            // Issue #107 logging ask: identify the DRIVING IMAGE behind each
            // opener (BiosVideo vs the vendor GOP driver build), not just the
            // opaque agent pointer. Image base+size is a stable per-build
            // fingerprint; a recursive ConnectController hang log then shows
            // exactly which display driver was being (re)selected.
            let image_note = match boot::open_protocol::<LoadedImage>(
                OpenProtocolParams {
                    handle: Handle::from_ptr(e.agent.cast()).unwrap_or(handle),
                    agent: boot::image_handle(),
                    controller: None,
                },
                OpenProtocolAttributes::GetProtocol,
            ) {
                Ok(li) => alloc::format!(" image={:p}+{:#x}", (li.info().0).cast::<u8>(), li.info().1),
                Err(_) => alloc::string::String::new(),
            };
            info!(
                "DIAG {label}: {:?} agent={:p} controller={:p} attr={:#x} count={}{}",
                handle, e.agent, e.controller, e.attributes, e.open_count, image_note
            );
        }
        if let Some(p) = core::ptr::NonNull::new(buf.cast::<u8>()) {
            let _ = boot::free_pool(p);
        }
    }
}

/// Read-only display topology: every GOP handle with its device-path
/// length, target ownership and GOP openers, plus the PciIo openers on the
/// GPU. Pathless GOPs (firmware splitters / ConSplitter) are skipped by the
/// detach below; this records what stays alive on top of the target.
fn log_display_topology(pci_handle: Handle) {
    log_openers("PCIIO-OPENERS", pci_handle, &PciIo::GUID);
    let Ok(h) = boot::locate_handle_buffer(boot::SearchType::ByProtocol(&GraphicsOutput::GUID)) else {
        info!("DIAG GOP-TOPOLOGY: no GOP handles");
        return;
    };
    for i in 0..h.len() {
        let gop = h[i];
        info!(
            "DIAG GOP-TOPOLOGY: {:?} devpath_len={:?} on_target={} protocols={}",
            gop,
            device_path_bytes(gop).map(|b| b.len()),
            gop_belongs_to_target(gop, pci_handle),
            open_count(gop)
        );
        log_openers("GOP-OPENERS", gop, &GraphicsOutput::GUID);
    }
}

fn config_snapshot(pci: &PciIo) -> alloc::vec::Vec<u32> {
    let mut c = vec![0u32; 16];
    for (i, slot) in c.iter_mut().enumerate() {
        *slot = pci.config_read_u32((i * 4) as u32);
    }
    c
}

/// Entry power state, read before anything else touches the device.
pub struct PmEntry {
    pub cap: Option<u8>,
    /// PMCSR as found (before the D0 request).
    pub pmcsr: u16,
    /// Entry was NOT D0 - a sleep/resume or fast-startup handoff, not a POST.
    pub warm: bool,
}

/// Resume-entry preflight. A sleep/resume hands the dGPU over in D3hot: the
/// firmware did not POST it, the previous OS session's device context is
/// still in the function, and everything the transaction relies on (BAR
/// decode, claim guards, FLR) is only defined in D0. Read PMCSR first, say
/// so loudly, then bring the function to D0 with a verified readback.
/// `\tx-require-d0.txt` on the boot volume turns the warning into an abort
/// (nothing has been written yet at that point, so the run is a no-op).
pub fn pm_preflight(pci: &PciIo) -> Result<PmEntry, &'static str> {
    let Some(cap) = pci.pm_cap() else {
        info!("PM: no power-management capability - D-state not evaluable");
        return Ok(PmEntry { cap: None, pmcsr: 0, warm: false });
    };
    let before = pci.pmcsr(cap);
    let warm = before & 0x3 != 0;
    info!("PM: entry PMCSR={before:#06x} state=D{} (cap@{cap:#04x})", before & 0x3);
    if warm {
        info!("PM: WARM ENTRY (resume/fast-startup handoff, not a POST): the previous session's device context is still in this function; the unlock may not survive the Windows handoff");
        if crate::tx::probe_file("\\tx-require-d0.txt") {
            info!("PM: \\tx-require-d0.txt present - aborting before any GPU write (use Restart, or cold boot, and re-run)");
            return Err("warm entry with require-D0 switch set");
        }
    }
    let after_cur = pci.pmcsr(cap);
    if !pci.pmcsr_force_d0(cap) {
        let rb = pci.pmcsr(cap);
        info!("PM: D0 request did NOT stick (PMCSR {after_cur:#06x} -> {rb:#06x}, state=D{})", rb & 0x3);
        return Err("device refuses D0");
    }
    info!("PM: D0 verified (PMCSR={:#06x})", pci.pmcsr(cap));
    Ok(PmEntry { cap: Some(cap), pmcsr: before, warm })
}

/// What `detach_and_claim` hands to the transaction: the config-space
/// baseline (drift reference for the post-transaction check) and the
/// firmware-tracked PciIo attributes captured BEFORE the claim rewrites
/// them - the native-GOP handoff restores the VGA forwarding subset from
/// here (issue #107).
pub struct Claim {
    pub baseline: alloc::vec::Vec<u32>,
    pub initial_attrs: Option<u64>,
}

/// Staged detach + exclusive claim. Returns the config-space baseline taken
/// while exclusively owned (drift reference for the post-transaction check).
pub fn detach_and_claim(pci_handle: Handle) -> Result<Claim, &'static str> {
    info!("DETACH: staged console/driver detach begin (TOTORONG craft)");
    log_display_topology(pci_handle);
    // Attributes BEFORE the claim's Set(0x0007) wipes the VGA forwarding
    // bits - captured on the still-bound device.
    let initial_attrs = pci_attrs_before_claim(pci_handle);

    // Stage 1+2: disconnect every driver bound to the TARGET GPU's GOP
    // handle (ConSplitter TextOut agent + GraphicsConsole; the UEFI
    // disconnect walks children, TOTORONG splits them only for failure
    // attribution). Multi-GPU (2026-10-27): GOPs belonging to OTHER cards
    // are left untouched - disconnecting them bought nothing, killed the
    // console when it lived on another GPU, and reconnect_all (target
    // handle only) never restored them. The stage-3 recursive disconnect
    // still guarantees the target's own stack comes off.
    let gop_handles =
        boot::locate_handle_buffer(boot::SearchType::ByProtocol(&GraphicsOutput::GUID));
    // A/B isolation (issue #14 flip): the prefix check misclassified the
    // target's own GOP on at least one single-GPU board (the board had no
    // Intel VGA in PCI enumeration, yet the GOP was declared "not on the
    // target GPU"), so v2.1.0-pre..v2.1.2 left the console connected where
    // v0.2.0 disconnected it - the only transaction-path delta on the
    // machine that flipped 0x91 -> unlocked. \tx-gop-disconnect.txt forces
    // the v0.2.0 behavior (disconnect every GOP handle) so the GOP variable
    // can be tested one boot per side on otherwise identical code.
    let force_gop_disconnect =
        crate::tx::probe_file("\\tx-gop-disconnect.txt");
    if force_gop_disconnect {
        info!("DETACH: \\tx-gop-disconnect.txt present - disconnecting every GOP handle (v0.2.0 behavior)");
    }
    if let Ok(h) = gop_handles {
        for i in 0..h.len() {
            let gop = h[i];
            if !force_gop_disconnect && !gop_belongs_to_target(gop, pci_handle) {
                info!("DETACH: GOP handle {:?} is not on the target GPU (multi-GPU) - left untouched, its console stays alive", gop);
                continue;
            }
            let before = open_count(gop);
            info!("DETACH: stage GOP handle {:?} protocols {}", gop, before);
            let st = boot::disconnect_controller(gop, None, None);
            let after = open_count(gop);
            info!(
                "DETACH: GOP disconnect status {:?} protocols {} -> {}",
                st.map(|_| ()),
                before,
                after
            );
        }
    } else {
        info!("DETACH: no GOP handles (headless?) - console stages skipped");
    }

    // Stage 3: the vendor GPU driver (nvgop) on the PCI handle.
    let before = open_count(pci_handle);
    let st = boot::disconnect_controller(pci_handle, None, None);
    let after = open_count(pci_handle);
    info!(
        "DETACH: PCI disconnect status {:?} protocols {} -> {}",
        st.map(|_| ()),
        before,
        after
    );
    log_display_topology(pci_handle);

    // Exclusive claim: re-open EXCLUSIVE (this fails while any other driver
    // still holds the PCI I/O protocol - the TOTORONG acceptance gate).
    let pci = {
        // Safety: exclusive claim; all other drivers detached by now.
        unsafe {
            boot::open_protocol::<PciIo>(
                OpenProtocolParams {
                    handle: pci_handle,
                    agent: boot::image_handle(),
                    controller: None,
                },
                OpenProtocolAttributes::Exclusive,
            )
        }
    };
    let pci = match pci {
        Ok(p) => p,
        Err(e) => {
            info!("DETACH: EXCLUSIVE claim failed {:?} (a driver still holds the device?)", e.status());
            return Err("exclusive claim failed");
        }
    };
    info!("DETACH: PciIo EXCLUSIVE claimed");

    // Restore IO|MEM|BME: try the Attributes API first (ClaimRestore),
    // then fall back to a raw PCI_COMMAND write - the driver's stop path
    // cleared MEMORY/BME, and this firmware's Attributes(Enable) is a
    // no-op while detached (observed: MEM=false after enable).
    pci.attributes_enable(0x0007);
    let cmd = pci.config_read_u16(0x04);
    if cmd & 0x7 != 0x7 {
        info!("DETACH: Attributes(Enable) no-op (cmd={cmd:#06x}) - raw COMMAND write");
        pci.cfg_write_u16(0x04, 0x0007);
        let cmd2 = pci.config_read_u16(0x04);
        info!("DETACH: raw COMMAND write result cmd={cmd2:#06x}");
    }
    let baseline = config_snapshot(&pci);
    info!(
        "DETACH: config baseline (vendor={:#06x} dev={:#06x} cmd={:#06x} - MEM={} BME={})",
        baseline[0] & 0xffff,
        baseline[0] >> 16,
        baseline[1] & 0xffff,
        baseline[1] & 0x2 != 0,
        baseline[1] & 0x4 != 0
    );
    // BAR0 reads return garbage until MEMORY decode is back. Verify - 1.1.
    if baseline[1] & 0x2 == 0 {
        return Err("PCI command MEMORY bit did not stick after both enable paths");
    }
    Ok(Claim { baseline, initial_attrs })
}

/// Firmware-tracked PciIo attributes of a handle still owned by its
/// drivers (read through a non-exclusive GetProtocol open). Called before
/// the exclusive claim so the native-GOP handoff knows which VGA bits to
/// put back (issue #107).
fn pci_attrs_before_claim(pci_handle: Handle) -> Option<u64> {
    // Safety: GetProtocol open, read-only call into the protocol.
    let pci = unsafe {
        boot::open_protocol::<PciIo>(
            OpenProtocolParams { handle: pci_handle, agent: boot::image_handle(), controller: None },
            OpenProtocolAttributes::GetProtocol,
        )
    }
    .ok()?;
    let attrs = pci.attributes_get();
    info!("DETACH: pre-claim attributes {attrs:?} (VGA subset {:#x})", attrs.unwrap_or(0) & 0x001C);
    attrs
}

/// Post-transaction GOP/console reconnect (TOTORONG craft): after the FLR
/// returned the card to a cold state, recursively ConnectController so the
/// firmware re-binds the vendor GOP/driver stack. Non-fatal: bootmgfw
/// self-establishes its own console when it starts; this only restores the
/// display earlier for machines where the dGPU is the primary adapter.
pub fn reconnect_all(pci_handle: uefi::Handle) {
    if crate::tx::probe_file("\\no-reconnect.txt") {
        info!("RECONNECT: skipped (\\no-reconnect.txt) - bootmgfw self-establishes its console");
        return;
    }
    // Give the FLR-rebooted function a moment, then reconnect. Recursive by
    // default (verified on single-GOP desktops). If a pathless GOP survived
    // the detach (an AMI splitter child, ASUS Z790 + 2803), connect the
    // vendor GOP driver only: the recursive connect re-binds the console
    // stack over the stale child and the platform resets inside the call.
    // bootmgfw locates the fresh vendor GOP on its own.
    boot::stall(core::time::Duration::from_millis(200));
    // Diagnostic switch (issues #35/#61): on a family of boards the
    // recursive rebind deadlocks inside the firmware right here (detach
    // leaves zero GOP handles -> stale_pathless_gop()=false ->
    // recursive=true). The marker forces the vendor-GOP-only bind PR#13
    // validated on the pathless-survivor shape and leaves the console
    // stack to bootmgfw - the Z170-A control proved bootmgfw's own
    // re-establishment suffices whenever it can bind at all.
    let force_nonrec = crate::tx::read_boot_file("\\tx-reconnect-nonrecursive.txt").is_some();
    let recursive = if force_nonrec {
        info!("RECONNECT: \\tx-reconnect-nonrecursive.txt present - forcing NON-recursive (vendor GOP bind only; console stack left to bootmgfw)");
        false
    } else {
        !stale_pathless_gop()
    };
    info!("RECONNECT: ConnectController (recursive={recursive}) begin");
    match boot::connect_controller(pci_handle, &[], None, recursive) {
        Ok(()) => info!("RECONNECT: ConnectController ok (recursive={recursive})"),
        Err(e) => info!("RECONNECT: ConnectController failed: {:?} (bootmgfw will self-establish)", e.status()),
    }
    log_display_topology(pci_handle);
}

/// True if any GOP handle without a device path is still installed. Called
/// after the detach, when the target's vendor GOP is gone: a pathless GOP
/// left over is a firmware splitter/console child (AMI) that a recursive
/// reconnect would re-bind over.
pub fn stale_pathless_gop() -> bool {
    let Ok(h) = boot::locate_handle_buffer(boot::SearchType::ByProtocol(&GraphicsOutput::GUID)) else {
        return false;
    };
    let stale = h.iter().any(|&gop| device_path_bytes(gop).is_none());
    if stale {
        info!("RECONNECT: pathless GOP survived the detach - vendor GOP only (non-recursive)");
    }
    stale
}
