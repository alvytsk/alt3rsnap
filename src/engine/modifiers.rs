//! Modifier bitset and ModMatcher.

use std::fmt;

/// Bitset of modifier keys. Bits are arbitrary but stable within a version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub bits: u16,
}

impl Modifiers {
    pub const NONE: Modifiers = Modifiers { bits: 0 };
    pub const ALT: Modifiers = Modifiers { bits: 1 << 0 };
    pub const CTRL: Modifiers = Modifiers { bits: 1 << 1 };
    pub const SHIFT: Modifiers = Modifiers { bits: 1 << 2 };
    pub const WIN: Modifiers = Modifiers { bits: 1 << 3 };
    pub const SPACE: Modifiers = Modifiers { bits: 1 << 4 };

    pub fn with(self, m: Modifiers) -> Modifiers {
        Modifiers {
            bits: self.bits | m.bits,
        }
    }
    pub fn without(self, m: Modifiers) -> Modifiers {
        Modifiers {
            bits: self.bits & !m.bits,
        }
    }
    pub fn contains(self, m: Modifiers) -> bool {
        (self.bits & m.bits) == m.bits
    }
    pub fn intersects(self, m: Modifiers) -> bool {
        (self.bits & m.bits) != 0
    }
    pub fn is_empty(self) -> bool {
        self.bits == 0
    }
}

impl fmt::Display for Modifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.contains(Modifiers::ALT) {
            parts.push("alt");
        }
        if self.contains(Modifiers::CTRL) {
            parts.push("ctrl");
        }
        if self.contains(Modifiers::SHIFT) {
            parts.push("shift");
        }
        if self.contains(Modifiers::WIN) {
            parts.push("win");
        }
        if self.contains(Modifiers::SPACE) {
            parts.push("space");
        }
        if parts.is_empty() {
            f.write_str("none")
        } else {
            f.write_str(&parts.join("+"))
        }
    }
}

/// A predicate over a `Modifiers` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModMatcher {
    pub required: Modifiers,
    pub forbidden: Modifiers,
    pub exact: bool,
}

impl ModMatcher {
    pub const NEVER: ModMatcher = ModMatcher {
        required: Modifiers::NONE,
        forbidden: Modifiers::NONE,
        exact: true, // combined with empty required: only matches NONE
    };

    pub fn matches(self, m: Modifiers) -> bool {
        if !m.contains(self.required) {
            return false;
        }
        if m.intersects(self.forbidden) {
            return false;
        }
        if self.exact && m != self.required {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_display_is_human_readable() {
        let m = Modifiers::ALT.with(Modifiers::CTRL);
        assert_eq!(m.to_string(), "alt+ctrl");
    }

    #[test]
    fn modifiers_contains_respects_bits() {
        let both = Modifiers::ALT.with(Modifiers::SHIFT);
        assert!(both.contains(Modifiers::ALT));
        assert!(both.contains(Modifiers::SHIFT));
        assert!(!both.contains(Modifiers::CTRL));
    }

    #[test]
    fn matcher_with_only_required_accepts_required_plus_others() {
        let m = ModMatcher {
            required: Modifiers::ALT,
            forbidden: Modifiers::NONE,
            exact: false,
        };
        assert!(m.matches(Modifiers::ALT));
        assert!(m.matches(Modifiers::ALT.with(Modifiers::CTRL)));
        assert!(!m.matches(Modifiers::CTRL));
    }

    #[test]
    fn matcher_with_forbidden_rejects_when_forbidden_present() {
        let m = ModMatcher {
            required: Modifiers::ALT,
            forbidden: Modifiers::WIN,
            exact: false,
        };
        assert!(m.matches(Modifiers::ALT));
        assert!(!m.matches(Modifiers::ALT.with(Modifiers::WIN)));
    }

    #[test]
    fn matcher_exact_requires_no_extras() {
        let m = ModMatcher {
            required: Modifiers::ALT,
            forbidden: Modifiers::NONE,
            exact: true,
        };
        assert!(m.matches(Modifiers::ALT));
        assert!(!m.matches(Modifiers::ALT.with(Modifiers::CTRL)));
    }
}
