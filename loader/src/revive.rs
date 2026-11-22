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
//! continues normally with the revived card in the target list.
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
    if !crate::tx::probe_file("\\tx-revive-dead-slot.txt") {
        return gpus.to_vec();
    }
    info!("SLOT-REVIVE: \\tx-revive-dead-slot.txt present - scanning for dead downstream slots");
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
            info!("SLOT-REVIVE: port looks healthy and the recorded bus matches - anomalous capture, no lever");
            continue;
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
                    sbr_slots.push(d.loc);
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
