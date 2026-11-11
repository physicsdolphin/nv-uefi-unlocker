//! GOP ordering decision (issue #228 C621E 4x-GA102 handoff dark-screen
//! class): reference matcher + shape tests.
//!
//! `loader/src/gop_order.rs`'s `decide`, `HideKey`, `parse_hide_line` and
//! `parse_hide` must stay textually identical to the copies here (loader is
//! UEFI-target-only and cannot run host tests; sim is the test bench).
//!
//! Background: LocateHandleBuffer(GOP) order is creation order and every
//! boot-graphics consumer (grub gfxterm, Linux efistub, bootmgr BGA) takes
//! the first handle, sink-blind. A native-GOP rebuild appends the verified
//! console child at the tail, so first-pickers land on headless competitors
//! (the #228 flip). The decision only ever demotes EDID-less competitors of
//! a sink-proven console child; anything ambiguous refuses.

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

#[cfg(test)]
mod tests {
    use super::*;

    // The #228 flip shape: console child (EDID-bearing) rebuilt last, one
    // EDID-less competitor ahead of it.
    #[test]
    fn flip_demotes_the_headless_competitor() {
        let order = [(false, false), (true, true)];
        assert_eq!(decide(&order), OrderDecision::Demote(vec![0]));
    }

    #[test]
    fn multiple_competitors_all_demoted() {
        let order = [(false, false), (false, false), (true, true)];
        assert_eq!(decide(&order), OrderDecision::Demote(vec![0, 1]));
    }

    #[test]
    fn console_first_is_a_noop() {
        let order = [(true, true), (false, false)];
        assert_eq!(decide(&order), OrderDecision::Noop);
    }

    #[test]
    fn competitor_with_edid_refuses_the_whole_operation() {
        // Both heads carry sinks - ambiguous topology, operator must decide
        // via \tx-gop-hide.txt instead of the automatic path.
        let order = [(true, false), (true, true)];
        assert_eq!(
            decide(&order),
            OrderDecision::Refuse("competitor carries EDID - ambiguous topology, use \\tx-gop-hide.txt")
        );
    }

    #[test]
    fn console_without_edid_refuses() {
        let order = [(false, false), (false, true)];
        assert_eq!(
            decide(&order),
            OrderDecision::Refuse("console child carries no EDID - sink proof missing")
        );
    }

    #[test]
    fn console_not_enumerated_refuses() {
        let order = [(false, false), (true, false)];
        assert_eq!(
            decide(&order),
            OrderDecision::Refuse("verified console child not enumerated")
        );
    }

    #[test]
    fn single_gop_console_first_never_writes() {
        let order = [(true, true)];
        assert_eq!(decide(&order), OrderDecision::Noop);
    }

    #[test]
    fn hide_lines_parse_both_forms() {
        assert_eq!(parse_hide_line("02:00.0"), Some(HideKey::Bdf(0, 2, 0, 0)));
        assert_eq!(parse_hide_line("0000:af:00.0"), Some(HideKey::Bdf(0, 0xaf, 0, 0)));
        assert_eq!(parse_hide_line("1a03:2000"), Some(HideKey::DevId(0x1a03, 0x2000)));
        assert_eq!(parse_hide_line("10de:2204"), Some(HideKey::DevId(0x10de, 0x2204)));
        assert_eq!(parse_hide_line("  3b:00.0  "), Some(HideKey::Bdf(0, 0x3b, 0, 0)));
    }

    #[test]
    fn hide_lines_skip_comments_blanks_and_garbage() {
        assert_eq!(parse_hide_line("# ASPEED BMC"), None);
        assert_eq!(parse_hide_line(""), None);
        assert_eq!(parse_hide_line("   "), None);
        assert_eq!(parse_hide_line("not-an-id"), None);
        assert_eq!(parse_hide_line("zz:00.0"), None);
    }

    #[test]
    fn hide_marker_parses_lines_and_comments() {
        let raw = "# board BMC VGA\n02:00.0\n\n1a03:2000\n";
        assert_eq!(
            parse_hide(raw),
            vec![HideKey::Bdf(0, 2, 0, 0), HideKey::DevId(0x1a03, 0x2000)]
        );
    }

    #[test]
    fn empty_hide_marker_is_a_refusal_shape_not_a_noop() {
        // The loader refuses an empty usable list loudly ("refusing to
        // guess") - the parse layer just reports zero keys.
        assert!(parse_hide("\n# only comments\n").is_empty());
    }
}
