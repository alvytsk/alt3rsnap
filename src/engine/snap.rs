//! Snap engine: zone baking, proximity + hysteresis, tie-break priority.
//! Pure Rust; no Win32 imports. Called only from `State::Moving`.

use crate::engine::config::{SnapEngineConfig, ZoneToggles};
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapSession {
    pub ctx: SnapContext,
    pub engaged: Option<EngagedZone>,
    pub last_preview_rect: Option<Rect>,
    pub suspended: bool,
    /// Latches true once cursor has moved ≥ 16 px from ctx.grab; stays true.
    pub restore_guard_cleared: bool,
}

/// Materialise the full list of snap zones from the current monitor snapshot and config.
/// Called once per drag at `BeginDrag` time.
pub fn bake_zones(cfg: &SnapEngineConfig, snapshot: &MonitorSnapshot) -> Vec<SnapZone> {
    let mut out = Vec::with_capacity(snapshot.monitors.len() * 11);
    if !cfg.enabled {
        return out;
    }
    for (idx, mon) in snapshot.monitors.iter().enumerate() {
        push_enabled(&mut out, cfg.zones, idx, mon);
    }
    out
}

fn push_enabled(out: &mut Vec<SnapZone>, z: ZoneToggles, idx: usize, mon: &MonitorInfo) {
    let wa = mon.work_area;
    let w = wa.right - wa.left;
    let h = wa.bottom - wa.top;
    let mid_x = wa.left + w / 2;
    let mid_y = wa.top + h / 2;
    let third_x1 = wa.left + w / 3;
    let third_x2 = wa.left + (2 * w) / 3;

    let push = |out: &mut Vec<SnapZone>, id, r| {
        out.push(SnapZone {
            id,
            target_rect: r,
            monitor_index: idx,
        });
    };

    if z.top_left_quarter {
        push(
            out,
            SnapZoneId::TopLeftQuarter,
            Rect {
                left: wa.left,
                top: wa.top,
                right: mid_x,
                bottom: mid_y,
            },
        );
    }
    if z.top_right_quarter {
        push(
            out,
            SnapZoneId::TopRightQuarter,
            Rect {
                left: mid_x,
                top: wa.top,
                right: wa.right,
                bottom: mid_y,
            },
        );
    }
    if z.bottom_left_quarter {
        push(
            out,
            SnapZoneId::BottomLeftQuarter,
            Rect {
                left: wa.left,
                top: mid_y,
                right: mid_x,
                bottom: wa.bottom,
            },
        );
    }
    if z.bottom_right_quarter {
        push(
            out,
            SnapZoneId::BottomRightQuarter,
            Rect {
                left: mid_x,
                top: mid_y,
                right: wa.right,
                bottom: wa.bottom,
            },
        );
    }
    if z.top_maximize {
        push(out, SnapZoneId::TopMaximize, wa);
    }
    if z.bottom_maximize {
        push(out, SnapZoneId::BottomMaximize, wa);
    }
    if z.left_half {
        push(
            out,
            SnapZoneId::LeftHalf,
            Rect {
                left: wa.left,
                top: wa.top,
                right: mid_x,
                bottom: wa.bottom,
            },
        );
    }
    if z.right_half {
        push(
            out,
            SnapZoneId::RightHalf,
            Rect {
                left: mid_x,
                top: wa.top,
                right: wa.right,
                bottom: wa.bottom,
            },
        );
    }
    if z.left_third {
        push(
            out,
            SnapZoneId::LeftThird,
            Rect {
                left: wa.left,
                top: wa.top,
                right: third_x1,
                bottom: wa.bottom,
            },
        );
    }
    if z.middle_third {
        push(
            out,
            SnapZoneId::MiddleThird,
            Rect {
                left: third_x1,
                top: wa.top,
                right: third_x2,
                bottom: wa.bottom,
            },
        );
    }
    if z.right_third {
        push(
            out,
            SnapZoneId::RightThird,
            Rect {
                left: third_x2,
                top: wa.top,
                right: wa.right,
                bottom: wa.bottom,
            },
        );
    }
}
