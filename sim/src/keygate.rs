//! Mirror of the ghost-key state machine in loader/src/keywait.rs
//! (`Repeat` + `key_code`), same hand-kept lockstep discipline as
//! log_gate.rs. Only the pure parts mirror here - the uefi `Key` type
//! becomes an abstract `Option<u16>` (Printable Char16 code unit) /
//! `u16` (Special scancode) pair, encoded identically to the loader's
//! `key_code`.
//!
//! Ghost criterion (PR #220): the SAME encoded key value appearing MIN
//! times consecutively with no quiet gap is a phantom key; any other
//! value resets the run; a quiet poll resets both.

/// Mirror of keywait::key_code: Printable -> Char16 code unit,
/// Special -> 0x1_0000 | scancode.
pub fn key_code(printable: Option<u16>, scan: u16) -> u32 {
    match printable {
        Some(c) => u32::from(c),
        None => 0x1_0000 | u32::from(scan),
    }
}

/// Mirror of ScanCode::ESCAPE's encoding (0x17 per uefi 0.41).
pub const ESC_CODE: u32 = 0x1_0000 | 0x17;

/// Mirror of keywait::Repeat::MIN.
pub const GHOST_MIN: u32 = 3;

#[derive(Debug, Clone, PartialEq)]
pub struct Repeat {
    last: u32,
    run: u32,
}

pub const NO_KEY: u32 = u32::MAX;

impl Repeat {
    pub const fn new() -> Self {
        Repeat { last: NO_KEY, run: 0 }
    }

    pub fn quiet(&mut self) {
        self.last = NO_KEY;
        self.run = 0;
    }

    /// Record one key; returns true exactly once when the same encoded
    /// value has appeared GHOST_MIN times in a row.
    pub fn feed(&mut self, code: u32) -> bool {
        if code == self.last {
            self.run += 1;
        } else {
            self.last = code;
            self.run = 1;
        }
        self.run == GHOST_MIN
    }
}

/// Mirror of keywait::SCAN_NAMES: every named uefi 0.41 `ScanCode`
/// variant with its pinned value. The loader builds its table from the
/// `ScanCode::X.0` constants, so this copy is what pins the numbers.
pub const SCAN_NAMES: &[(&str, u16)] = &[
    ("UP", 0x01),
    ("DOWN", 0x02),
    ("RIGHT", 0x03),
    ("LEFT", 0x04),
    ("HOME", 0x05),
    ("END", 0x06),
    ("INSERT", 0x07),
    ("DELETE", 0x08),
    ("PAGE_UP", 0x09),
    ("PAGE_DOWN", 0x0A),
    ("FUNCTION_1", 0x0B),
    ("FUNCTION_2", 0x0C),
    ("FUNCTION_3", 0x0D),
    ("FUNCTION_4", 0x0E),
    ("FUNCTION_5", 0x0F),
    ("FUNCTION_6", 0x10),
    ("FUNCTION_7", 0x11),
    ("FUNCTION_8", 0x12),
    ("FUNCTION_9", 0x13),
    ("FUNCTION_10", 0x14),
    ("FUNCTION_11", 0x15),
    ("FUNCTION_12", 0x16),
    ("ESCAPE", 0x17),
    ("FUNCTION_13", 0x68),
    ("FUNCTION_14", 0x69),
    ("FUNCTION_15", 0x6A),
    ("FUNCTION_16", 0x6B),
    ("FUNCTION_17", 0x6C),
    ("FUNCTION_18", 0x6D),
    ("FUNCTION_19", 0x6E),
    ("FUNCTION_20", 0x6F),
    ("FUNCTION_21", 0x70),
    ("FUNCTION_22", 0x71),
    ("FUNCTION_23", 0x72),
    ("FUNCTION_24", 0x73),
    ("MUTE", 0x7F),
    ("VOLUME_UP", 0x80),
    ("VOLUME_DOWN", 0x81),
    ("BRIGHTNESS_UP", 0x100),
    ("BRIGHTNESS_DOWN", 0x101),
    ("SUSPEND", 0x102),
    ("HIBERNATE", 0x103),
    ("TOGGLE_DISPLAY", 0x104),
    ("RECOVERY", 0x105),
    ("EJECT", 0x106),
];

/// Mirror of keywait::parse_key_line (`\tx-ignore-key.txt` line -> key code).
pub fn parse_key_line(line: &str) -> Option<u32> {
    let t = line.trim();
    if t.is_empty() {
        return None;
    }
    if let Some(name) = t.strip_prefix("Special(").and_then(|r| r.strip_suffix(')')) {
        if let Some(n) = name
            .strip_prefix("ScanCode(")
            .and_then(|r| r.strip_suffix(')'))
        {
            return parse_num(n.trim())
                .filter(|&n| n <= 0xFFFF)
                .map(|n| 0x1_0000 | n);
        }
        return SCAN_NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|&(_, sc)| 0x1_0000 | u32::from(sc));
    }
    if let Some(rest) = t.strip_prefix("scan") {
        return parse_num(rest.trim())
            .filter(|&n| n <= 0xFFFF)
            .map(|n| 0x1_0000 | n);
    }
    let inner = t
        .strip_prefix("Printable(")
        .and_then(|r| r.strip_suffix(')'))
        .unwrap_or(t);
    if let Some(q) = inner.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
        if let Some(h) = q.strip_prefix("\\u{").and_then(|r| r.strip_suffix('}')) {
            return u32::from_str_radix(h, 16).ok().filter(|&n| n <= 0xFFFF);
        }
        let mut it = q.chars();
        return match (it.next(), it.next()) {
            (Some(c), None) if (c as u32) <= 0xFFFF => Some(c as u32),
            _ => None,
        };
    }
    parse_num(inner).filter(|&n| n <= 0xFFFF)
}

fn parse_num(t: &str) -> Option<u32> {
    match t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        Some(h) => u32::from_str_radix(h, 16).ok(),
        None => t.parse().ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(c: u16) -> u32 {
        key_code(Some(c), 0)
    }

    #[test]
    fn kg01_key_code_encoding_matches_loader() {
        assert_eq!(key(0x61), 0x61);
        assert_eq!(key(0x7A), 0x7A);
        assert_eq!(key_code(None, 0x04), 0x1_0004);
        assert_eq!(key_code(None, 0x17), ESC_CODE);
    }

    #[test]
    fn kg02_ghost_fires_once_on_the_third_repeat() {
        let mut rep = Repeat::new();
        assert!(!rep.feed(key(0x61)));
        assert!(!rep.feed(key(0x61)));
        assert!(rep.feed(key(0x61)), "3rd consecutive same value = ghost");
        assert!(!rep.feed(key(0x61)), "4th repeat must NOT re-fire");
    }

    #[test]
    fn kg03_different_value_resets_the_run() {
        let mut rep = Repeat::new();
        assert!(!rep.feed(key(0x61)));
        assert!(!rep.feed(key(0x61)));
        assert!(!rep.feed(key(0x62)), "run broken by a different value");
        assert!(!rep.feed(key(0x62)));
        assert!(rep.feed(key(0x62)), "fresh run of 0x62 counts from 1");
    }

    #[test]
    fn kg04_quiet_resets_everything() {
        let mut rep = Repeat::new();
        assert!(!rep.feed(key(0x61)));
        assert!(!rep.feed(key(0x61)));
        rep.quiet();
        assert!(!rep.feed(key(0x61)), "quiet gap restarts the run");
        assert!(!rep.feed(key(0x61)));
        assert!(rep.feed(key(0x61)));
    }

    #[test]
    fn kg05_special_and_printable_do_not_share_a_run() {
        let mut rep = Repeat::new();
        // Char16 0x61 vs Special scan 0x61 = 0x1_0061 - distinct codes.
        assert!(!rep.feed(key(0x61)));
        assert!(!rep.feed(key_code(None, 0x61)));
        assert!(!rep.feed(key(0x61)));
        assert!(!rep.feed(key_code(None, 0x61)));
        assert!(!rep.feed(key(0x61)), "interleaved codes never reach MIN");
    }

    #[test]
    fn kg06_esc_is_a_distinct_code_that_cannot_collide_with_printable() {
        // The loader's suppress() refuses ESC by matching the encoded
        // value; the mirror pins the encoding so the refusal stays exact.
        assert_eq!(ESC_CODE, key_code(None, 0x17));
        assert_ne!(ESC_CODE, key_code(Some(0x17), 0));
    }

    #[test]
    fn kg07_every_scan_name_round_trips_to_its_code() {
        for &(name, sc) in SCAN_NAMES {
            let line = format!("Special({name})");
            assert_eq!(parse_key_line(&line), Some(key_code(None, sc)), "{name}");
        }
    }

    #[test]
    fn kg08_scan_names_are_unique_and_pinned() {
        for (i, &(n, sc)) in SCAN_NAMES.iter().enumerate() {
            assert!(
                SCAN_NAMES[i + 1..].iter().all(|&(m, c)| m != n && c != sc),
                "{n}"
            );
        }
        assert_eq!(parse_key_line("Special(LEFT)"), Some(0x1_0004));
        assert_eq!(parse_key_line("Special(ESCAPE)"), Some(ESC_CODE));
        // laptop phantom-key class above 0xFF
        assert_eq!(parse_key_line("Special(TOGGLE_DISPLAY)"), Some(0x1_0104));
        assert_eq!(parse_key_line("Special(EJECT)"), Some(0x1_0106));
    }

    #[test]
    fn kg09_unnamed_and_numeric_forms() {
        assert_eq!(parse_key_line("Special(ScanCode(0x18))"), Some(0x1_0018));
        assert_eq!(parse_key_line("Special(ScanCode(300))"), Some(0x1_012C));
        assert_eq!(parse_key_line("scan 0x104"), Some(0x1_0104));
        assert_eq!(parse_key_line("Special(NOPE)"), None);
        assert_eq!(parse_key_line("LEFT"), None);
        assert_eq!(parse_key_line("scan 0x10000"), None);
    }

    #[test]
    fn kg10_printable_forms() {
        assert_eq!(parse_key_line("Printable('z')"), Some(0x7A));
        assert_eq!(parse_key_line("'\\u{1a}'"), Some(0x1A));
        assert_eq!(parse_key_line("0x1A"), Some(0x1A));
        assert_eq!(parse_key_line("26"), Some(26));
        assert_eq!(parse_key_line("Printable('ab')"), None);
    }
}
