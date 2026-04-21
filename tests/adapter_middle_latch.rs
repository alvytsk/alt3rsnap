//! Tests for the adapter's middle-up swallow latch (spec §5.7).
//!
//! This file lives outside `tests/engine_*` because the latch is adapter-owned
//! state (AtomicBool + monotonic clock), not engine FSM behavior.

use alt3rsnap::swallow_latch::SwallowLatch;

#[test]
fn set_then_try_swallow_returns_true_once_then_false() {
    let latch = SwallowLatch::new();
    assert!(!latch.is_set());
    latch.set(0);
    assert!(latch.is_set());
    assert!(latch.try_swallow(10));
    assert!(!latch.is_set());
    assert!(!latch.try_swallow(10));
}

#[test]
fn on_timer_clears_latch_after_safety_window() {
    let latch = SwallowLatch::new();
    latch.set(1_000);
    // Same frame: don't clear.
    latch.on_timer(1_000);
    assert!(latch.is_set());
    // Just inside safety window: don't clear.
    latch.on_timer(1_000 + 499);
    assert!(latch.is_set());
    // At safety window boundary: clear.
    latch.on_timer(1_000 + alt3rsnap::swallow_latch::SAFETY_CLEAR_MS);
    assert!(!latch.is_set());
}

#[test]
fn on_timer_without_arm_is_noop() {
    let latch = SwallowLatch::new();
    latch.on_timer(10_000);
    assert!(!latch.is_set());
}

#[test]
fn on_timer_preserves_latch_if_safety_not_elapsed() {
    let latch = SwallowLatch::new();
    latch.set(5_000);
    latch.on_timer(5_100);
    assert!(latch.is_set());
    // And a subsequent try_swallow still succeeds.
    assert!(latch.try_swallow(5_101));
}
