//! Marker-content number parsing (issue #38 follow-up): reference
//! implementation + shape tests.
//!
//! `loader/src/chainload.rs`'s `parse_marker_number`, `parse_armed_seconds`,
//! `parse_dxe_arm_seconds` and `parse_flush_lines` must stay textually
//! identical to the copies here (loader is UEFI-target-only and cannot run
//! host tests; sim is the test bench). These parse the stick-root knobs:
//! the armed countdown seconds (`\i_know_what_i_am_doing.txt`, 1..=30,
//! default 5), the DXE driver's arm countdown (`\tx-dxe-arm.txt`, same
//! floor - audit N1) and the log flush cadence (`\tx-flush-lines.txt`,
//! 0..=4096, default 16, 0 = marker-lines only).

fn parse_marker_number(content: &[u8], default: u64, min: u64, max: u64) -> u64 {
    let line = content
        .split(|&b| b == b'\r' || b == b'\n')
        .find(|l| !l.is_empty());
    let Some(line) = line else { return default };
    let digits: &[u8] = {
        let trimmed: &[u8] = {
            let start = line.iter().position(|b| !b.is_ascii_whitespace()).unwrap_or(line.len());
            let end = line.iter().rposition(|b| !b.is_ascii_whitespace()).map_or(0, |i| i + 1);
            &line[start..end.max(start)]
        };
        let stop = trimmed.iter().position(|b| !b.is_ascii_digit()).unwrap_or(trimmed.len());
        &trimmed[..stop]
    };
    if digits.is_empty() {
        return default;
    }
    let value = digits
        .iter()
        .fold(0u64, |acc, b| acc.saturating_mul(10).saturating_add((b - b'0') as u64));
    value.clamp(min, max)
}

pub fn parse_armed_seconds(content: &[u8]) -> u64 {
    parse_marker_number(content, 5, 1, 30)
}

/// DXE-driver arm countdown (`\tx-dxe-arm.txt`): same floor as the
/// application form (audit N1/M1 - 0 is unreachable by marker content;
/// headless operation opts out via the separate `\tx-dxe-noconfirm.txt`
/// marker, which has no number to parse).
pub fn parse_dxe_arm_seconds(content: &[u8]) -> u64 {
    parse_marker_number(content, 5, 1, 30)
}

pub fn parse_flush_lines(content: &[u8]) -> u64 {
    parse_marker_number(content, 16, 0, 4096)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armed_seconds_clamped_to_1_30() {
        assert_eq!(parse_armed_seconds(b"2\r\n"), 2);
        assert_eq!(parse_armed_seconds(b" 2s\n"), 2); // leading digits only
        assert_eq!(parse_armed_seconds(b"02"), 2);
        assert_eq!(parse_armed_seconds(b"0"), 1); // an abort window must remain
        assert_eq!(parse_armed_seconds(b"999"), 30);
        assert_eq!(parse_armed_seconds(b"184467440737095516160"), 30); // saturate+clamp
    }

    #[test]
    fn armed_seconds_defaults() {
        assert_eq!(parse_armed_seconds(b""), 5); // zero-byte marker
        assert_eq!(parse_armed_seconds(b"\r\n\r\n"), 5); // blank lines only
        assert_eq!(parse_armed_seconds(b"   \n"), 5);
        assert_eq!(parse_armed_seconds(b"go now"), 5); // no leading digits
        assert_eq!(parse_armed_seconds(b"# comment"), 5);
    }

    #[test]
    fn flush_lines_clamped_to_0_4096() {
        assert_eq!(parse_flush_lines(b"64"), 64);
        assert_eq!(parse_flush_lines(b"0"), 0); // marker-lines-only mode
        assert_eq!(parse_flush_lines(b"1"), 1); // per-line (legacy slow shape)
        assert_eq!(parse_flush_lines(b"999999"), 4096);
        assert_eq!(parse_flush_lines(b"junk"), 16);
        assert_eq!(parse_flush_lines(b""), 16);
    }

    #[test]
    fn first_nonempty_line_wins() {
        assert_eq!(parse_armed_seconds(b"\n\n8\n2\n"), 8);
        assert_eq!(parse_flush_lines(b"\n\n32\n16\n"), 32);
        // A leading non-digit line does not scan forward - defaults apply.
        assert_eq!(parse_armed_seconds(b"# notes\n3\n"), 5);
    }

    #[test]
    fn dxe_arm_seconds_share_the_app_floor() {
        // Audit N1: the DXE form's abort window must remain, exactly like
        // the application's - "0" in the marker cannot remove it.
        assert_eq!(parse_dxe_arm_seconds(b"0"), 1);
        assert_eq!(parse_dxe_arm_seconds(b"7"), 7);
        assert_eq!(parse_dxe_arm_seconds(b""), 5);
        assert_eq!(parse_dxe_arm_seconds(b"999"), 30);
    }
}
