//! PORT-DIAG / SLOT-REVIVE decode helper (issue #276 dead-slot families).
//!
//! `decode_port_state` is a textual mirror of the loader's copy in
//! `loader/src/pci.rs` (the loader is UEFI-target-only and cannot run host
//! tests; sim is the test bench). Keep the two bodies identical - any
//! ladder-ordering change must land in both files in the same commit.
//!
//! Semantics recap (see the loader doc comments for the full narrative):
//! a downstream slot reads vendor 0xffff while the card itself is alive.
//! Ladder: D-state != 0 -> park back to D0 first (a parked port stops its
//! LTSSM); Link Control bit 4 (Link Disable) set -> clear it; link not
//! active with clean software state -> Secondary Bus Reset (drives a hot
//! reset that clears Link Disable and kicks the LTSSM out of a stuck
//! L2/L3-Ready); link active + the port's secondary bus no longer matches
//! the dead function's recorded bus -> RELOCATED (firmware renumbering -
//! memory windows still route, which is why the console updates; config
//! TLPs route by bus number and master-abort); link active + same bus ->
//! anomalous, no lever.

/// Ladder choice for a bridge whose downstream function(s) read vendor
/// 0xffff. Pure function: mirrors loader/src/pci.rs `decode_port_state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviveLever {
    NoneHealthy,
    NoneRelocated,
    PmcsrD0,
    ClearLinkDisable,
    SecondaryBusReset,
}

/// Mirror of the raw capture plus the decoded fields and the chosen lever.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortState {
    pub d_state: u8,
    pub link_disable: bool,
    pub link_active: bool,
    pub speed: u8,
    pub width: u8,
    pub sec_bus: u8,
    pub relocated: bool,
    pub lever: ReviveLever,
}

/// sim/src/port_diag.rs mirrors this body verbatim - keep them identical
/// to loader/src/pci.rs `decode_port_state`.
pub fn decode_port_state(
    pmcsr: Option<u16>,
    lnkctl: Option<u16>,
    lnksta: Option<u16>,
    sec_bus: u8,
    dead_recorded_bus: Option<u8>,
) -> PortState {
    let d_state = (pmcsr.unwrap_or(0) & 0x3) as u8;
    let link_disable = lnkctl.map(|v| v & 0x10 != 0).unwrap_or(false);
    let link_active = lnksta.map(|v| v & 0x2000 != 0).unwrap_or(true);
    let speed = (lnksta.unwrap_or(0) & 0xf) as u8;
    let width = ((lnksta.unwrap_or(0) >> 4) & 0x3f) as u8;
    let relocated = link_active && dead_recorded_bus.map(|b| b != sec_bus).unwrap_or(false);
    let lever = if d_state != 0 {
        ReviveLever::PmcsrD0
    } else if link_disable {
        ReviveLever::ClearLinkDisable
    } else if !link_active {
        ReviveLever::SecondaryBusReset
    } else if relocated {
        ReviveLever::NoneRelocated
    } else {
        ReviveLever::NoneHealthy
    };
    PortState { d_state, link_disable, link_active, speed, width, sec_bus, relocated, lever }
}

#[cfg(test)]
mod tests {
    use super::*;

    const D0: Option<u16> = Some(0x0000);
    const D3HOT: Option<u16> = Some(0x0003);
    const LNKCTL_CLEAN: Option<u16> = Some(0x0040); // common clock, no disable
    const LNKCTL_DISABLED: Option<u16> = Some(0x0050); // + Link Disable (bit 4)
    // Gen3 x16, link active: LA bit13 + width[9:4]=16 (0b010000 << 4) + speed 3.
    const LNKSTA_UP_X16: Option<u16> = Some(0x2103);
    // LA clear, speed/width stale-looking zero.
    const LNKSTA_DOWN: Option<u16> = Some(0x0000);

    #[test]
    fn d3hot_port_takes_pmcsr_d0_first() {
        let st = decode_port_state(D3HOT, LNKCTL_DISABLED, LNKSTA_DOWN, 0x02, Some(0x02));
        // The D-state outranks the Link Disable: a parked port stops its
        // LTSSM, so parking it back comes first and the ladder re-decodes.
        assert_eq!(st.d_state, 3);
        assert!(st.link_disable);
        assert!(!st.link_active);
        assert_eq!(st.lever, ReviveLever::PmcsrD0);
    }

    #[test]
    fn link_disable_takes_the_clear() {
        let st = decode_port_state(D0, LNKCTL_DISABLED, LNKSTA_DOWN, 0x02, Some(0x02));
        assert!(st.link_disable);
        assert!(!st.link_active);
        assert_eq!(st.lever, ReviveLever::ClearLinkDisable);
    }

    #[test]
    fn inactive_clean_link_takes_the_sbr() {
        // The expected #89/#276 "stuck L2/L3-Ready" shape: D0, no software
        // disable, link simply not active.
        let st = decode_port_state(D0, LNKCTL_CLEAN, LNKSTA_DOWN, 0x02, Some(0x02));
        assert_eq!(st.d_state, 0);
        assert!(!st.link_disable);
        assert!(!st.link_active);
        assert_eq!(st.lever, ReviveLever::SecondaryBusReset);
    }

    #[test]
    fn link_up_with_moved_bus_is_relocated() {
        // Port's secondary bus was renumbered 0x02 -> 0x43 while the dead
        // function's handle still records 0x02: the device is alive on the
        // new bus (memory windows still route); never "revive" it.
        let st = decode_port_state(D0, LNKCTL_CLEAN, LNKSTA_UP_X16, 0x43, Some(0x02));
        assert!(st.link_active);
        assert!(st.relocated);
        assert_eq!(st.speed, 3);
        assert_eq!(st.width, 16);
        assert_eq!(st.lever, ReviveLever::NoneRelocated);
    }

    #[test]
    fn link_up_same_bus_is_anomalous_no_lever() {
        let st = decode_port_state(D0, LNKCTL_CLEAN, LNKSTA_UP_X16, 0x02, Some(0x02));
        assert!(st.link_active);
        assert!(!st.relocated);
        assert_eq!(st.lever, ReviveLever::NoneHealthy);
    }

    #[test]
    fn missing_caps_never_trigger_a_lever() {
        // A cap-less port decodes as healthy defaults (D0 / not disabled /
        // link ASSUMED ACTIVE): the arm must not hammer a port it cannot
        // read state from.
        let st = decode_port_state(None, None, None, 0x02, Some(0x02));
        assert_eq!(st.lever, ReviveLever::NoneHealthy);
        assert!(st.link_active);
        assert_eq!(st.width, 0);
    }

    #[test]
    fn no_recorded_bus_never_reads_as_relocated() {
        // PORT-DIAG (enumerate-time) calls with None - the snapshot alone
        // must not claim relocation.
        let st = decode_port_state(D0, LNKCTL_CLEAN, LNKSTA_UP_X16, 0x43, None);
        assert!(st.link_active);
        assert!(!st.relocated);
        assert_eq!(st.lever, ReviveLever::NoneHealthy);
    }
}
