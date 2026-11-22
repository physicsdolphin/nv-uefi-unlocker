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
    // DLLLA = LnkSta bit 14 (0x4000); bit 13 is Link Bandwidth Management
    // Status. Field-caught on the TU106 bench (00:01.0, lnksta=0xD101, link
    // verifiably up) - see the loader copy for the full note.
    let link_active = lnksta.map(|v| v & 0x4000 != 0).unwrap_or(true);
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

/// Parse one ACPI address-space descriptor from a GetBarAttributes Resources
/// buffer. Packed EFI_ACPI_ADDRESS_SPACE_DESCRIPTOR (EDK2 Acpi10.h, 46 B):
/// `Desc`@0 (large-item tag 0x8A QWord / 0x87 DWORD), `Len`@1 u16, `ResType`@3
/// (0 = memory, 1 = I/O), `AddrSpaceGranularity`@6 u64, `AddrRangeMin`@14 u64
/// (the firmware-assigned base, translation-adjusted), `AddrLen`@38 u64.
/// Returns (res_type, base, len, granularity).
///
/// sim/src/port_diag.rs mirrors this body verbatim - keep them identical
/// to loader/src/pci.rs `parse_bar_descriptor`.
pub fn parse_bar_descriptor(buf: &[u8]) -> Option<(u8, u64, u64, u64)> {
    if buf.len() < 46 {
        return None;
    }
    let tag = buf[0];
    if tag != 0x8A && tag != 0x87 {
        return None;
    }
    let res_type = buf[3];
    let rd = |o: usize| -> Option<u64> {
        let s = buf.get(o..o + 8)?;
        Some(u64::from_le_bytes(s.try_into().ok()?))
    };
    let gran = rd(6)?;
    let min = rd(14)?;
    let len = rd(38)?;
    Some((res_type, min, len, gran))
}

#[cfg(test)]
mod tests {
    use super::*;

    const D0: Option<u16> = Some(0x0000);
    const D3HOT: Option<u16> = Some(0x0003);
    const LNKCTL_CLEAN: Option<u16> = Some(0x0040); // common clock, no disable
    const LNKCTL_DISABLED: Option<u16> = Some(0x0050); // + Link Disable (bit 4)
    // Gen3 x16, link active: DLLLA bit14 + width[9:4]=16 + speed 3.
    const LNKSTA_UP_X16: Option<u16> = Some(0x4103);
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

    // --- parse_bar_descriptor (GetBarAttributes Resources buffer) ---

    /// Build a 46-byte packed EFI_ACPI_ADDRESS_SPACE_DESCRIPTOR + trailing
    /// end tag, per EDK2 Acpi10.h: Desc@0, Len@1 u16, ResType@3, GenFlag@4,
    /// SpecificFlag@5, Granularity@6 u64, AddrRangeMin@14, AddrRangeMax@22,
    /// AddrTranslationOffset@30, AddrLen@38; then EndTag (0x79, 0x00).
    fn make_desc(tag: u8, res_type: u8, gran: u64, base: u64, len: u64) -> Vec<u8> {
        let mut b = vec![0u8; 48];
        b[0] = tag;
        b[1..3].copy_from_slice(&43u16.to_le_bytes()); // body length = 46-3
        b[3] = res_type;
        b[6..14].copy_from_slice(&gran.to_le_bytes());
        b[14..22].copy_from_slice(&base.to_le_bytes());
        b[22..30].copy_from_slice(&len.to_le_bytes());
        b[38..46].copy_from_slice(&len.to_le_bytes());
        b[46] = 0x79; // ACPI_END_TAG_DESCRIPTOR
        b[47] = 0x00;
        b
    }

    #[test]
    fn qword_memory_descriptor_parses() {
        // TU106 BAR0: 16 MiB MEM32 at 0x3a0c0000.
        let buf = make_desc(0x8A, 0x00, 32, 0x0000_0000_3a0c_0000, 0x0100_0000);
        let (rt, base, len, gran) = parse_bar_descriptor(&buf).expect("parses");
        assert_eq!(rt, 0);
        assert_eq!(base, 0x3a0c_0000);
        assert_eq!(len, 0x0100_0000);
        assert_eq!(gran, 32);
    }

    #[test]
    fn dword_io_descriptor_parses() {
        // IO BAR: DWORD tag 0x87, granularity 32.
        let buf = make_desc(0x87, 0x01, 32, 0x0000_3000, 0x10);
        let (rt, base, len, gran) = parse_bar_descriptor(&buf).expect("parses");
        assert_eq!(rt, 1);
        assert_eq!(base, 0x3000);
        assert_eq!(len, 0x10);
        assert_eq!(gran, 32);
    }

    #[test]
    fn wrong_tag_and_short_buffer_reject() {
        // End-tag-only or truncated buffers must not parse.
        assert!(parse_bar_descriptor(&[0x79, 0x00, 0x00]).is_none());
        assert!(parse_bar_descriptor(&vec![0u8; 45]).is_none());
        let mut buf = make_desc(0x8A, 0x00, 64, 0x1_0000_0000, 0x1000);
        buf[0] = 0x79; // wrong tag
        assert!(parse_bar_descriptor(&buf).is_none());
    }

    #[test]
    fn high_bar_base_survives_u64_parse() {
        // A 64-bit aperture above 4 GiB (the TU106's HIGH staging band lives
        // at 4 GiB+) - the u64 fields must not truncate.
        let buf = make_desc(0x8A, 0x00, 64, 0x0000_0100_0000_0000, 0x1000);
        let (_, base, _, gran) = parse_bar_descriptor(&buf).expect("parses");
        assert_eq!(base, 0x0000_0100_0000_0000);
        assert_eq!(gran, 64);
    }

    #[test]
    fn bench_capture_d101_is_link_active() {
        // Field-pinned regression (TU106 bench, SLOT-REVIVE build round
        // 2026-11-22): the TU106's own PEG port 00:01.0 read lnksta=0xD101
        // with the link verifiably up (the transaction ran green on it).
        // DLLLA is bit 14 - bit 13 is Link Bandwidth Management Status and
        // reads 0 here, which the first draft misused and reported LA=0.
        let st = decode_port_state(Some(0x0008), Some(0x0043), Some(0xD101), 0x01, None);
        assert!(st.link_active, "0xD101 bit 14 is set: the link is active");
        assert_eq!(st.d_state, 0);
        assert!(!st.link_disable);
        assert_eq!(st.speed, 1); // idle down-train, normal on Intel PEG
        assert_eq!(st.width, 16);
    }

    #[test]
    fn empty_port_fingerprint_is_la0_width0() {
        // The natural negative control from the same bench round: 00:1b.0
        // (no device downstream) read lnksta=0x1001 - LA clear, width x0.
        let st = decode_port_state(D0, LNKCTL_CLEAN, Some(0x1001), 0x02, None);
        assert!(!st.link_active);
        assert_eq!(st.width, 0);
        assert_eq!(st.lever, ReviveLever::SecondaryBusReset);
    }
}
