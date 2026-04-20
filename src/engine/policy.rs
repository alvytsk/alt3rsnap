//! ActivationPolicy — named predicates over Modifiers that drive the engine.

use crate::engine::modifiers::{ModMatcher, Modifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivationPolicy {
    /// When this matches current modifiers, the engine is "armed" and ready to start a drag.
    pub arm: ModMatcher,
    /// When this matches at drag start, the adapter raises the target window.
    pub raise: ModMatcher,
    /// When this matches, snap behavior is suspended (future — MVP no-op).
    pub no_snap: ModMatcher,
}

impl Default for ActivationPolicy {
    /// Default: Alt arms, Ctrl raises, Space suspends snap.
    fn default() -> ActivationPolicy {
        ActivationPolicy {
            arm: ModMatcher {
                required: Modifiers::ALT,
                forbidden: Modifiers::WIN, // avoid conflict with Win+drag snap
                exact: false,
            },
            raise: ModMatcher {
                required: Modifiers::CTRL,
                forbidden: Modifiers::NONE,
                exact: false,
            },
            no_snap: ModMatcher {
                required: Modifiers::SPACE,
                forbidden: Modifiers::NONE,
                exact: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_arms_on_alt_alone() {
        let p = ActivationPolicy::default();
        assert!(p.arm.matches(Modifiers::ALT));
    }

    #[test]
    fn default_policy_does_not_arm_on_alt_plus_win() {
        let p = ActivationPolicy::default();
        assert!(!p.arm.matches(Modifiers::ALT.with(Modifiers::WIN)));
    }

    #[test]
    fn default_policy_raises_on_ctrl() {
        let p = ActivationPolicy::default();
        assert!(p.raise.matches(Modifiers::CTRL));
        assert!(p.raise.matches(Modifiers::CTRL.with(Modifiers::ALT)));
    }
}
