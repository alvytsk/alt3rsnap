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
