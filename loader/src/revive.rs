//! SLOT-REVIVE (marker-gated): revive a downstream slot that reads vendor
//! 0xffff at enumeration while the card itself is demonstrably alive -
//! issue #276: its own GOP drives the console, no ACPI resource gates the
//! rail, and Windows later trains it at a different bus number. The
//! host-reversible candidates all live on the ROOT PORT (a D3hot park, a
//! Link Control "Link Disable", a stuck L2/L3-Ready - a PME_Turn_Off
//! handshake whose power-removal step never came), so the ladder acts on
//! the port's own config space, never on the GPU: PMCSR -> D0, clear Link
//! Disable, then the Secondary Bus Reset hammer (which also clears Link
//! Disable and kicks the LTSSM out of a stuck L2/L3-Ready).
//!
//! Constitution mapping: zero GPU writes happen here (the first GPU touch
//! remains behind the confirm/transaction machinery); every lever is a
//! port-side config write verified by readback; a claimed recovery is only
//! believed after the dead function's vendor ID reads back clean; one
//! ladder pass per port, no loops (the "STOP/异常后同会话绝不重试" clause
//! does not bite - nothing failed ON the GPU yet - but the one-pass rule
//! still holds as deliberate design).
//!
//! An SBR resets the device: its config space goes to defaults, BARs are
//! unassigned, and this loader has no BAR re-assignment capability - so an
//! SBR revival deliberately SKIPS the transaction this boot (Windows
//! re-assigns and re-trains; the stick is re-run afterwards). A no-reset
//! revival (PMCSR/Link Disable) leaves the device untouched and the boot
//! continues normally with the revived card in the target list. Note the
//! asymmetry: on a live device (GPU-SBR bench experiment) the BARs can be
//! SAVED before the reset and restored after - on a slot that already
//! reads 0xffff there is no state to save, and the only future source is
//! the firmware's stale PciIo bookkeeping (GetBarAttributes).
//!
//! The console caveat: on a #276-shaped machine the console IS the card's
//! GOP - a lever that disturbs the display pipeline (the SBR) takes the
//! screen down mid-run; loader.log is the only readback from that point on,
//! and the warning line prints BEFORE the SBR while the console still lives.

use crate::pci::{self, FoundGpu, PciIo, ReviveLever};
use core::time::Duration;
use log::info;
use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams, SearchType};
use uefi::{Handle, Identify};

/// Marker-gated dead-slot revival. Returns the candidate list to use for
/// target selection: the input unchanged when the marker is absent or
/// nothing revived, a fresh [`pci::enumerate`] when a no-reset revival
/// brought a card back, and that list minus the SBR-reset slots otherwise
/// (their config space was reset - the transaction is skipped this boot).
pub fn revive_dead_slots(gpus: &[FoundGpu]) -> alloc::vec::Vec<FoundGpu> {
    let force_sbr = crate::tx::probe_file("\\tx-revive-force-sbr.txt");
    if !crate::tx::probe_file("\\tx-revive-dead-slot.txt") && !force_sbr {
        return gpus.to_vec();
    }
    info!(
        "SLOT-REVIVE: armed (\\tx-revive-dead-slot.txt or \\tx-revive-force-sbr.txt{}) - scanning for dead downstream slots",
        if force_sbr { ", FORCE-SBR override on" } else { "" }
    );
    let handles = match boot::locate_handle_buffer(SearchType::ByProtocol(&PciIo::GUID)) {
        Ok(h) => h,
        Err(e) => {
            info!("SLOT-REVIVE: locate_handle_buffer failed: {:?}", e.status());
            return gpus.to_vec();
        }
    };

    struct DeadFn {
        handle: Handle,
        loc: (usize, usize, usize, usize),
    }
    struct Port {
        handle: Handle,
        loc: (usize, usize, usize, usize),
        sec: u8,
        sub: u8,
    }
    let mut dead: alloc::vec::Vec<DeadFn> = alloc::vec::Vec::new();
    let mut ports: alloc::vec::Vec<Port> = alloc::vec::Vec::new();
    for handle in handles.iter() {
        // Safety: GetProtocol performs no driver binding (same as enumerate()).
        let Ok(pci) = (unsafe {
            boot::open_protocol::<PciIo>(
                OpenProtocolParams { handle: *handle, agent: boot::image_handle(), controller: None },
                OpenProtocolAttributes::GetProtocol,
            )
        }) else {
            continue;
        };
        let Some(loc) = pci.get_location() else { continue };
        let vendor = pci.config_read_u16(0x00);
        if vendor == 0xffff || vendor == 0x0000 {
            dead.push(DeadFn { handle: *handle, loc });
        } else if pci.config_read_u8(0x0e) & 0x7f == 0x01 {
            // Alive PCI-to-PCI bridge (header type 1, same filter as
            // log_dma_path/BridgeMaster).
            let buses = pci.config_read_u32(0x18);
            ports.push(Port {
                handle: *handle,
                loc,
                sec: ((buses >> 8) & 0xff) as u8,
                sub: ((buses >> 16) & 0xff) as u8,
            });
        }
    }
    info!("SLOT-REVIVE: {} dead function(s), {} alive bridge(s)", dead.len(), ports.len());

    let mut sbr_slots: alloc::vec::Vec<(usize, usize, usize, usize)> = alloc::vec::Vec::new();
    let mut revived_no_reset = false;

    for d in dead.iter() {
        let Some(p) = ports.iter().find(|p| {
            p.loc.0 == d.loc.0 && d.loc.1 >= p.sec as usize && d.loc.1 <= p.sub as usize
        }) else {
            info!(
                "SLOT-REVIVE: {:04x}:{:02x}:{:02x}.{} dead but no alive parent port - nothing to act on",
                d.loc.0, d.loc.1, d.loc.2, d.loc.3
            );
            continue;
        };
        // Safety: GetProtocol performs no driver binding (same as enumerate()).
        let Ok(port) = (unsafe {
            boot::open_protocol::<PciIo>(
                OpenProtocolParams { handle: p.handle, agent: boot::image_handle(), controller: None },
                OpenProtocolAttributes::GetProtocol,
            )
        }) else {
            continue;
        };
        let diag = pci::read_port_diag(&port);
        let mut st = pci::decode_port_state(
            diag.pmcsr,
            diag.lnkctl,
            diag.lnksta,
            p.sec,
            Some(d.loc.1 as u8),
        );
        info!(
            "SLOT-REVIVE: {:04x}:{:02x}:{:02x}.{} dead under port {:04x}:{:02x}:{:02x}.{} (sec=0x{:02x} sub=0x{:02x}) - pmcsr={:?} lnkctl={:?} lnksta={:?} bcr={:?} | D{} ldis={} LA={} spd=x{} w=x{} lever={:?}",
            d.loc.0, d.loc.1, d.loc.2, d.loc.3,
            p.loc.0, p.loc.1, p.loc.2, p.loc.3, p.sec, p.sub,
            diag.pmcsr, diag.lnkctl, diag.lnksta, diag.bcr,
            st.d_state, st.link_disable as u8, st.link_active as u8, st.speed, st.width, st.lever,
        );
        if st.lever == ReviveLever::NoneHealthy {
            if force_sbr {
                info!(
                    "SLOT-REVIVE: FORCE-SBR: paradoxical port (port reads healthy, child reads 0xffff) - \\tx-revive-force-sbr.txt overrides the decline and forces the SBR lever"
                );
                st.lever = ReviveLever::SecondaryBusReset;
            } else {
                info!("SLOT-REVIVE: port looks healthy and the recorded bus matches - anomalous capture, no lever (\\tx-revive-force-sbr.txt overrides)");
                continue;
            }
        }
        if st.lever == ReviveLever::NoneRelocated {
            info!(
                "SLOT-REVIVE: LINK IS UP and the port's secondary bus (0x{:02x}) no longer matches the recorded location bus (0x{:02x}) - the device was RELOCATED by firmware renumbering, not powered down (memory windows still route, which is why the console updates); no lever applies - relocation-aware discovery is the follow-up",
                p.sec, d.loc.1
            );
            continue;
        }

        // The escalation ladder. Each lever: act on the port, verify the
        // write by readback, wait, reprobe the dead function's vendor ID.
        let mut sbr = false;
        for _round in 0..3u8 {
            match st.lever {
                ReviveLever::PmcsrD0 => {
                    match port.pm_cap() {
                        Some(cap) => {
                            let ok = port.pmcsr_force_d0(cap);
                            info!("SLOT-REVIVE: lever 1 PMCSR->D0 (cap@{cap:#04x}) readback_verified={ok}");
                        }
                        None => info!("SLOT-REVIVE: lever 1 skipped - no PM capability on the port"),
                    }
                    boot::stall(Duration::from_millis(100));
                }
                ReviveLever::ClearLinkDisable => {
                    match port.pcie_cap() {
                        Some(cap) => {
                            let before = port.config_read_u16(cap as u32 + 0x10);
                            port.cfg_write_u16(cap as u32 + 0x10, before & !0x10);
                            let after = port.config_read_u16(cap as u32 + 0x10);
                            info!("SLOT-REVIVE: lever 2 Link Disable clear (lnkctl {before:#06x} -> {after:#06x})");
                        }
                        None => info!("SLOT-REVIVE: lever 2 skipped - no PCIe capability on the port"),
                    }
                    boot::stall(Duration::from_millis(100));
                }
                ReviveLever::SecondaryBusReset => {
                    sbr = true;
                    info!(
                        "SLOT-REVIVE: lever 3 SECONDARY BUS RESET - if the card owns the console, the screen goes down from here (loader.log is the only readback); the transaction is skipped this boot by design"
                    );
                    sbr_cycle(&port);
                }
                ReviveLever::NoneHealthy | ReviveLever::NoneRelocated => break,
            }
            // Reprobe the dead function through its own handle: the vendor ID
            // is the only "it came back" evidence (readback is the only
            // success criterion).
            let vendor = open_dead_vendor(d.handle);
            if vendor != 0xffff && vendor != 0x0000 {
                info!(
                    "SLOT-REVIVE: {:04x}:{:02x}:{:02x}.{} vendor read back {:#06x} - slot is ALIVE{}",
                    d.loc.0,
                    d.loc.1,
                    d.loc.2,
                    d.loc.3,
                    vendor,
                    if sbr {
                        " (via SBR - its config space was reset, BARs unassigned)"
                    } else {
                        " (no reset - device state intact)"
                    },
                );
                if sbr {
                    // Upgrade attempt: the firmware's BAR bookkeeping
                    // (GetBarAttributes on this handle - host-side state
                    // built at POST enumeration, survives the device reset)
                    // may let us re-arm MMIO, which would keep a revived GPU
                    // candidate in this boot's transaction instead of the
                    // skip. Non-candidates (bridges, other functions) gain
                    // nothing but the evidence is free.
                    let bars_ok = restore_bars_from_bookkeeping(d.handle);
                    if bars_ok {
                        info!(
                            "SLOT-REVIVE: BARs restored from firmware bookkeeping - staying in the candidate list (transaction proceeds this boot)"
                        );
                        revived_no_reset = true;
                    } else {
                        sbr_slots.push(d.loc);
                        info!(
                            "SLOT-REVIVE: BAR bookkeeping unavailable or failed - excluding from this boot's transaction (chain-load Windows, re-run the stick afterwards)"
                        );
                    }
                } else {
                    revived_no_reset = true;
                }
                break;
            }
            let diag2 = pci::read_port_diag(&port);
            st = pci::decode_port_state(
                diag2.pmcsr,
                diag2.lnkctl,
                diag2.lnksta,
                p.sec,
                Some(d.loc.1 as u8),
            );
            info!(
                "SLOT-REVIVE: still 0xffff after this lever - next: {:?} (D{} ldis={} LA={})",
                st.lever, st.d_state, st.link_disable as u8, st.link_active as u8
            );
        }
    }

    if revived_no_reset || !sbr_slots.is_empty() {
        let mut out = pci::enumerate();
        if !sbr_slots.is_empty() {
            out.retain(|g| {
                !sbr_slots
                    .iter()
                    .any(|(s, b, dd, f)| g.seg == *s && g.bus == *b && g.dev == *dd && g.fun == *f)
            });
            info!(
                "SLOT-REVIVE: SBR-reset slot(s) excluded from this boot's transaction (their config space was reset and the loader does not re-assign BARs) - chain-loading Windows, which re-assigns and re-trains; re-run the stick afterwards"
            );
        }
        return out;
    }
    gpus.to_vec()
}

fn open_dead_vendor(handle: Handle) -> u16 {
    // Safety: GetProtocol performs no driver binding (same as enumerate()).
    match unsafe {
        boot::open_protocol::<PciIo>(
            OpenProtocolParams { handle, agent: boot::image_handle(), controller: None },
            OpenProtocolAttributes::GetProtocol,
        )
    } {
        Ok(pci) => pci.config_read_u16(0x00),
        Err(_) => 0xffff,
    }
}

/// Restore a revived device's BARs from the firmware's own bookkeeping
/// (GetBarAttributes on the handle - host-side state built at POST
/// enumeration, untouched by the device-side reset). Writes each returned
/// base address back into the device's config space with readback, then
/// arms command = IO|MEM|BME (the claim's own value). True only when at
/// least one BAR was restored with verified readbacks AND the command
/// stuck; false means no bookkeeping was available or nothing verified -
/// the caller keeps its skip-transaction behavior in that case.
fn restore_bars_from_bookkeeping(handle: Handle) -> bool {
    // Safety: GetProtocol performs no driver binding (same as enumerate()).
    let Ok(g) = (unsafe {
        boot::open_protocol::<PciIo>(
            OpenProtocolParams { handle, agent: boot::image_handle(), controller: None },
            OpenProtocolAttributes::GetProtocol,
        )
    }) else {
        info!("SLOT-REVIVE: BAR restore: open PciIo failed");
        return false;
    };
    let mut restored = 0usize;
    let mut i = 0u8;
    while i < 6 {
        match g.get_bar_attributes(i) {
            Some((res_type, base, _len, gran)) if base != 0 => {
                let off = 0x10 + (i as u32) * 4;
                g.cfg_write_u32(off, base as u32);
                let rb = g.config_read_u32(off);
                info!(
                    "SLOT-REVIVE: BAR{i} restored base={base:#x} len={_len:#x} type={res_type} gran={gran} (readback {rb:#010x}){}",
                    if rb == base as u32 { "" } else { " MISMATCH" }
                );
                if rb != base as u32 {
                    return false;
                }
                restored += 1;
                // A 64-bit BAR consumes the next config dword as its upper
                // half - write it (the descriptor's granularity says which)
                // and skip the slot in this loop.
                if gran == 64 {
                    let hi = (base >> 32) as u32;
                    g.cfg_write_u32(off + 4, hi);
                    let rb_hi = g.config_read_u32(off + 4);
                    info!(
                        "SLOT-REVIVE: BAR{i} upper half {hi:#010x} (readback {rb_hi:#010x}){}",
                        if rb_hi == hi { "" } else { " MISMATCH" }
                    );
                    if rb_hi != hi {
                        return false;
                    }
                    i += 1; // the consumed upper slot
                }
            }
            _ => info!("SLOT-REVIVE: BAR{i}: no bookkeeping (unassigned or unsupported)"),
        }
        i += 1;
    }
    if restored == 0 {
        info!("SLOT-REVIVE: BAR restore: bookkeeping returned nothing usable");
        return false;
    }
    g.cfg_write_u16(0x04, 0x0007); // IO|MEM|BME - the claim's own value
    let cmd_rb = g.config_read_u16(0x04);
    info!("SLOT-REVIVE: command armed 0x0007 (readback {cmd_rb:#06x})");
    cmd_rb & 0x0007 == 0x0007 && restored > 0
}

/// One Secondary Bus Reset cycle on `port`: assert Bridge Control bit 6,
/// verify by readback, hold 100 ms, de-assert, wait 2 s for the retrain
/// (PCIe/CEM convention). The LTSSM restarts from Detect - which also
/// clears a Link Disable and kicks a stuck L2/L3-Ready.
fn sbr_cycle(port: &PciIo) {
    let before = port.config_read_u16(0x3e);
    port.cfg_write_u16(0x3e, before | 0x40);
    let mid = port.config_read_u16(0x3e);
    info!("SLOT-REVIVE: SBR asserted (bcr {before:#06x} -> {mid:#06x})");
    boot::stall(Duration::from_millis(100));
    port.cfg_write_u16(0x3e, before & !0x40);
    let after = port.config_read_u16(0x3e);
    info!("SLOT-REVIVE: SBR de-asserted (bcr -> {after:#06x}); waiting 2 s for the link to retrain");
    boot::stall(Duration::from_millis(2000));
}

/// GPU-SBR (marker `\tx-sbr-gpu.txt`): a forced Secondary Bus Reset on the
/// TARGET GPU's own parent port, executed INSIDE the transaction's
/// disciplined window - after detach+claim (no firmware drivers bound, the
/// console mirror is muted, the card is exclusively claimed) and before
/// FWSEC ignition (nothing unlocked, nothing to destroy).
///
/// The first attempt (bench 2026-11-22) ran PRE-transaction - firmware GOP
/// still bound, console still printing through the card - and soft-locked
/// the machine between "TX: BEGIN" and the next flush line (keyboard alive,
/// CAD rebootable, screen black; the transaction's first GPU write was
/// never reached, so the pre-flush forensics point at the firmware layer,
/// not at a GPU write). Pre-transaction position REMOVED: nothing else in
/// the codebase touches a card while firmware drivers are bound to it -
/// the DETACH-first discipline exists precisely because that state is
/// unexplored. The reset + config save/restore itself was proven sound in
/// that round: the card answered config after the reset, the BAR restore
/// read back clean, and BAR0 read the chip identity (MMIO alive) - the
/// wedge was in the firmware-driver layer, which is what this placement
/// avoids.
///
/// BAR restore comes from the claim's config baseline (captured while the
/// device was alive pre-claim) - the asymmetry recorded on the SBR-skip
/// still holds for DEAD slots: a #276-class slot that reads 0xffff has no
/// readable state, and its only future source is GetBarAttributes on the
/// stale handle (implemented in `restore_bars_from_bookkeeping`).
///
/// Returns true when the caller should continue the chain (marker absent =
/// silent no-op, or reset + restore + MMIO sanity all verified); false when
/// the post-reset device could not be verified alive with working MMIO (a
/// reset device without verified life must not be transacted - readback is
/// the only success criterion). On false the transaction aborts with the
/// card freshly reset and nothing unlocked.
pub fn sbr_gpu_in_transaction(gpu: &crate::tx::TxGpu, baseline: &[u32]) -> bool {
    if !crate::tx::probe_file("\\tx-sbr-gpu.txt") {
        return true;
    }
    info!(
        "GPU-SBR: \\tx-sbr-gpu.txt present - forced Secondary Bus Reset on this card's parent port (disciplined window: drivers detached, console muted, nothing unlocked)"
    );
    // GetBarAttributes cross-check, taken while the device is alive: the
    // bookkeeping is host-side POST state, the claim baseline is the live
    // config. Element-wise agreement validates the bookkeeping as a BAR
    // source for the dead-slot restore path (the #276-class upgrade) - the
    // round that mattered, since a 0xffff slot cannot be read directly.
    // Note the bookkeeping is LOGICAL-BAR indexed: a 64-bit MEM pair is one
    // entry (gran=64) consuming two config dwords, and IO descriptors carry
    // no granularity (gran=0) - the config walk below mirrors both.
    {
        let mut cfg_entries: alloc::vec::Vec<(u64, u64)> = alloc::vec::Vec::new();
        let mut d = 4usize; // baseline dword 4 = config offset 0x10 (BAR0)
        while d <= 9 {
            let v = baseline[d];
            if v == 0 {
                d += 1;
                continue;
            }
            if v & 0x1 == 0x1 {
                cfg_entries.push(((v & 0xfffc) as u64, 0)); // IO: gran unset
                d += 1;
            } else if v & 0x2 != 0 && d < 9 {
                let hi = baseline[d + 1];
                cfg_entries.push((((hi as u64) << 32) | (v & 0xffff_fff0) as u64, 64));
                d += 2;
            } else {
                cfg_entries.push(((v & 0xffff_fff0) as u64, 32));
                d += 1;
            }
        }
        let mut bk_entries: alloc::vec::Vec<(u64, u64)> = alloc::vec::Vec::new();
        for i in 0..6u8 {
            if let Some((_, base, _len, gran)) = gpu.pci.get_bar_attributes(i) {
                if base != 0 {
                    bk_entries.push((base, gran));
                }
            }
        }
        info!(
            "GPU-SBR: bookkeeping cross-check - config {} logical BAR(s), bookkeeping {} entry(ies)",
            cfg_entries.len(),
            bk_entries.len()
        );
        for (i, (cb, cg)) in cfg_entries.iter().enumerate() {
            match bk_entries.get(i) {
                Some((bb, bg)) => info!(
                    "GPU-SBR: bookkeeping BAR{i}: base={bb:#x} gran={bg} vs config base={cb:#x} gran={cg}{}",
                    if bb == *cb && bg == *cg { " MATCH" } else { " MISMATCH" }
                ),
                None => info!(
                    "GPU-SBR: bookkeeping BAR{i}: missing (config has base={cb:#x} gran={cg})"
                ),
            }
        }
        for (i, (bb, bg)) in bk_entries.iter().enumerate().skip(cfg_entries.len()) {
            info!("GPU-SBR: bookkeeping BAR{i}: extra entry base={bb:#x} gran={bg}");
        }
    }
    // Location first (the card's PciIo is exclusively claimed here).
    let Some((seg, gpu_bus, _, _)) = gpu.pci.get_location() else {
        info!("GPU-SBR: card location unavailable - skipping the experiment, chain continues");
        return true;
    };
    // Find the parent port by secondary/subordinate containment (same walk
    // as BridgeMaster). All device-side access goes through `gpu`.
    let Ok(handles) = boot::locate_handle_buffer(SearchType::ByProtocol(&PciIo::GUID)) else {
        info!("GPU-SBR: locate_handle_buffer failed - skipping the experiment, chain continues");
        return true;
    };
    let mut parent: Option<Handle> = None;
    for handle in handles.iter() {
        // Safety: GetProtocol, same as enumerate().
        let Ok(p) = (unsafe {
            boot::open_protocol::<PciIo>(
                OpenProtocolParams { handle: *handle, agent: boot::image_handle(), controller: None },
                OpenProtocolAttributes::GetProtocol,
            )
        }) else {
            continue;
        };
        if p.config_read_u16(0x00) == 0xffff || p.config_read_u8(0x0e) & 0x7f != 0x01 {
            continue;
        }
        let Some(loc) = p.get_location() else { continue };
        if loc.0 != seg {
            continue;
        }
        let buses = p.config_read_u32(0x18);
        let (sec, sub) = ((buses >> 8 & 0xff) as usize, (buses >> 16 & 0xff) as usize);
        if gpu_bus < sec || gpu_bus > sub {
            continue;
        }
        parent = Some(*handle);
        info!(
            "GPU-SBR: parent port {:04x}:{:02x}:{:02x}.{} (sec=0x{sec:02x} sub=0x{sub:02x})",
            loc.0, loc.1, loc.2, loc.3
        );
        break;
    }
    let Some(ph) = parent else {
        info!("GPU-SBR: no parent bridge found - skipping the experiment, chain continues");
        return true;
    };
    // Safety: GetProtocol, same as enumerate().
    let Ok(port) = (unsafe {
        boot::open_protocol::<PciIo>(
            OpenProtocolParams { handle: ph, agent: boot::image_handle(), controller: None },
            OpenProtocolAttributes::GetProtocol,
        )
    }) else {
        info!("GPU-SBR: open parent port failed - skipping the experiment, chain continues");
        return true;
    };
    let diag = pci::read_port_diag(&port);
    info!(
        "GPU-SBR: pre-SBR port state - pmcsr={:?} lnkctl={:?} lnksta={:?} (D{} ldis={} LA={} spd=x{} w=x{})",
        diag.pmcsr,
        diag.lnkctl,
        diag.lnksta,
        (diag.pmcsr.unwrap_or(0) & 3) as u8,
        diag.lnkctl.map(|v| v & 0x10 != 0).unwrap_or(false),
        diag.lnksta.map(|v| v & 0x4000 != 0).unwrap_or(false),
        (diag.lnksta.unwrap_or(0) & 0xf) as u8,
        ((diag.lnksta.unwrap_or(0) >> 4) & 0x3f) as u8,
    );
    sbr_cycle(&port);
    // Reprobe through the claimed card: the device must answer config with
    // its own vendor ID.
    let vendor = gpu.pci.config_read_u16(0x00);
    if vendor != crate::pci::NVIDIA_VENDOR_ID {
        info!(
            "GPU-SBR: post-reset vendor read {vendor:#06x} (want 10de) - the device did not come back; aborting the chain (a reset device without verified life must not be transacted)"
        );
        return false;
    }
    info!("GPU-SBR: vendor read back 0x10de - device is alive; restoring the claim-baseline config");
    // Restore BARs (baseline dwords 4..9 = offsets 0x10..0x24) + the ROM BAR
    // (baseline[12] = 0x30) + the command (baseline[1] = the claim's own
    // 0x0007). Every write readback-verified.
    for i in 4..10usize {
        let off = (i as u32) * 4;
        let v = baseline[i];
        gpu.pci.cfg_write_u32(off, v);
        let rb = gpu.pci.config_read_u32(off);
        info!(
            "GPU-SBR: cfg[{off:#04x}] restored {v:#010x} (readback {rb:#010x}){}",
            if rb == v { "" } else { " MISMATCH" }
        );
        if rb != v {
            return false;
        }
    }
    gpu.pci.cfg_write_u32(0x30, baseline[12]);
    gpu.pci.cfg_write_u16(0x04, baseline[1] as u16);
    let cmd_rb = gpu.pci.config_read_u16(0x04);
    info!(
        "GPU-SBR: command restored {:#06x} (readback {cmd_rb:#06x})",
        baseline[1] as u16
    );
    boot::stall(Duration::from_millis(100));
    // The MMIO proof: BAR0 offset 0 must read the chip identity.
    let boot0 = gpu.read32(0);
    let mmio_ok = matches!(boot0, Ok(v) if v != 0xffff_ffff && v != 0x0000_0000);
    info!(
        "GPU-SBR: BAR0[0] read {:#010x} (PMC_BOOT_0 chip identity) - MMIO {}",
        boot0.unwrap_or(0xffff_ffff),
        if mmio_ok { "alive" } else { "NOT DECODING" }
    );
    mmio_ok
}

