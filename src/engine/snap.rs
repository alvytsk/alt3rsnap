//! Snap engine: zone baking, proximity + hysteresis, tie-break priority.
//! Pure Rust; no Win32 imports. Called only from `State::Moving`.

use crate::engine::geometry::{Point, Rect};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorInfo {
    pub bounds: Rect,
    pub work_area: Rect,
    pub scale: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorSnapshot {
    pub monitors: Vec<MonitorInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapZoneId {
    TopLeftQuarter,
    TopRightQuarter,
    BottomLeftQuarter,
    BottomRightQuarter,
    TopMaximize,
    BottomMaximize,
    LeftHalf,
    RightHalf,
    LeftThird,
    MiddleThird,
    RightThird,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapZone {
    pub id: SnapZoneId,
    pub target_rect: Rect,
    pub monitor_index: usize,
}

/// One engagement record on an active drag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngagedZone {
    pub id: SnapZoneId,
    pub target_rect: Rect,
    pub entered_at_cursor: Point,
}

/// Immutable-per-drag inputs to snap evaluation.
#[derive(Debug, Clone)]
pub struct SnapContext {
    pub monitors: MonitorSnapshot,
    pub zones: Vec<SnapZone>,
    pub engage_px: u32,
    pub disengage_px: u32,
    /// Cursor point at BeginDrag — used by restore-guard and for debug logs.
    pub grab: Point,
    /// True iff the drag target was maximized at BeginDrag and was restored;
    /// while true, `evaluate` returns `None` until cursor moved ≥ 16 px from grab.
    pub restore_guard_active: bool,
}

/// Mutable-per-drag snap state.
#[derive(Debug, Clone)]
pub struct SnapSession {
    pub ctx: SnapContext,
    pub engaged: Option<EngagedZone>,
    pub last_preview_rect: Option<Rect>,
    pub suspended: bool,
    /// Latches true once cursor has moved ≥ 16 px from ctx.grab; stays true.
    pub restore_guard_cleared: bool,
}
