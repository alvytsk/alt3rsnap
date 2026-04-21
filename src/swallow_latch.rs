//! Adapter-owned latch that swallows the matching WM_MBUTTONUP after the
//! engine has acted on a WM_MBUTTONDOWN (spec §2.9).
//!
//! The latch is Win32-free: it takes a monotonic-millisecond timestamp as an
//! explicit argument on every operation. The real adapter passes
//! `GetTickCount64`; tests pass a synthetic clock.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// 500 ms safety clear window (spec §2.9).
pub const SAFETY_CLEAR_MS: u64 = 500;

pub struct SwallowLatch {
    armed: AtomicBool,
    armed_at_ms: AtomicU64,
}

impl SwallowLatch {
    pub const fn new() -> Self {
        SwallowLatch {
            armed: AtomicBool::new(false),
            armed_at_ms: AtomicU64::new(0),
        }
    }

    /// Arm the latch. `now_ms` is a monotonic millisecond timestamp.
    pub fn set(&self, now_ms: u64) {
        self.armed_at_ms.store(now_ms, Ordering::Relaxed);
        self.armed.store(true, Ordering::Release);
    }

    /// True iff armed.
    pub fn is_set(&self) -> bool {
        self.armed.load(Ordering::Acquire)
    }

    /// Attempt to swallow a WM_MBUTTONUP. Returns true if the event should be
    /// dropped (the latch was armed); clears the latch in that case.
    ///
    /// `now_ms` is currently unused by this method but is taken so the
    /// signature matches the other latch operations and so a future safety
    /// check could be added here without a breaking change.
    pub fn try_swallow(&self, _now_ms: u64) -> bool {
        self.armed.swap(false, Ordering::AcqRel)
    }

    /// Safety clear: if the latch was armed at least `SAFETY_CLEAR_MS` ago,
    /// clear it. Called from the adapter's `SetTimer` callback (spec §3.4).
    pub fn on_timer(&self, now_ms: u64) {
        if !self.is_set() {
            return;
        }
        let armed_at = self.armed_at_ms.load(Ordering::Acquire);
        if now_ms.saturating_sub(armed_at) >= SAFETY_CLEAR_MS {
            self.armed.store(false, Ordering::Release);
        }
    }
}

impl Default for SwallowLatch {
    fn default() -> Self {
        Self::new()
    }
}
