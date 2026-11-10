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
}
