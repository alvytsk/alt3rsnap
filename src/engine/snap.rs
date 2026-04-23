//! Snap engine: zone baking, proximity + hysteresis, tie-break priority.
//! Pure Rust; no Win32 imports. Called only from `State::Moving`.

use crate::engine::config::{SnapEngineConfig, ZoneToggles};
use crate::engine::geometry::{Point, Rect};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Engage(EngagedZone),
    Hold,
    Disengage,
    None,
}

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

/// Returns the decision for this MouseMove; mutates `engaged` / `last_preview_rect`
/// appropriately. Never returns Engage while `suspended` or while the restore guard
/// is active.
pub fn evaluate(session: &mut SnapSession, cursor: Point) -> Decision {
    if session.suspended {
        if session.engaged.take().is_some() {
            session.last_preview_rect = None;
            return Decision::Disengage;
        }
        return Decision::None;
    }

    if session.ctx.restore_guard_active && !session.restore_guard_cleared {
        let dx = cursor.x - session.ctx.grab.x;
        let dy = cursor.y - session.ctx.grab.y;
        // Per-axis short-circuit first: if either axis delta already >= 16, we're out.
        // Otherwise the squared-distance check is bounded by 16*16 + 16*16 = 512, safe in i32.
        if dx.abs() < 16 && dy.abs() < 16 && (dx * dx + dy * dy) < (16 * 16) {
            return Decision::None;
        }
        session.restore_guard_cleared = true;
    }

    let winner = best_candidate(&session.ctx, cursor, session.ctx.engage_px as i32);

    match (&session.engaged, winner) {
        (None, None) => Decision::None,
        (None, Some(z)) => {
            let ez = EngagedZone {
                id: z.id,
                target_rect: z.target_rect,
                entered_at_cursor: cursor,
            };
            session.engaged = Some(ez.clone());
            session.last_preview_rect = Some(z.target_rect);
            Decision::Engage(ez)
        }
        (Some(cur), Some(z)) if cur.id == z.id => Decision::Hold,
        (Some(_), Some(z)) => {
            let ez = EngagedZone {
                id: z.id,
                target_rect: z.target_rect,
                entered_at_cursor: cursor,
            };
            session.engaged = Some(ez.clone());
            session.last_preview_rect = Some(z.target_rect);
            Decision::Engage(ez)
        }
        (Some(_), None) => {
            let dist_px = disengage_distance_to_current(session, cursor);
            if dist_px >= session.ctx.disengage_px as i32 {
                session.engaged = None;
                session.last_preview_rect = None;
                Decision::Disengage
            } else {
                Decision::Hold
            }
        }
    }
}

fn best_candidate(ctx: &SnapContext, cursor: Point, engage_px: i32) -> Option<&SnapZone> {
    let mut best: Option<(&SnapZone, u8)> = None;
    for z in &ctx.zones {
        if !cursor_eligible(
            z,
            cursor,
            engage_px,
            &ctx.monitors.monitors[z.monitor_index],
        ) {
            continue;
        }
        let pr = priority_rank(z.id);
        // Strict `<` on priority keeps the first zone seen on equal priority — stable
        // ordering matches bake order (quarters before halves before thirds).
        best = match best {
            None => Some((z, pr)),
            Some((_, br)) if pr < br => Some((z, pr)),
            Some(x) => Some(x),
        };
    }
    best.map(|(z, _)| z)
}

/// True iff the cursor is within `engage_px` of the zone's triggering edge/corner
/// (edges use `bounds` so auto-hidden taskbar does not defeat the top edge).
fn cursor_eligible(z: &SnapZone, cursor: Point, engage_px: i32, mon: &MonitorInfo) -> bool {
    use SnapZoneId::*;
    let b = mon.bounds;
    // Cursor must be physically on this monitor (inclusive bounds) — prevents a zone
    // on monitor A from firing when the cursor has crossed onto monitor B's side of
    // a shared display boundary.
    if cursor.x < b.left || cursor.x > b.right || cursor.y < b.top || cursor.y > b.bottom {
        return false;
    }
    match z.id {
        TopLeftQuarter => {
            (cursor.x - b.left).abs() <= engage_px && (cursor.y - b.top).abs() <= engage_px
        }
        TopRightQuarter => {
            (cursor.x - b.right).abs() <= engage_px && (cursor.y - b.top).abs() <= engage_px
        }
        BottomLeftQuarter => {
            (cursor.x - b.left).abs() <= engage_px && (cursor.y - b.bottom).abs() <= engage_px
        }
        BottomRightQuarter => {
            (cursor.x - b.right).abs() <= engage_px && (cursor.y - b.bottom).abs() <= engage_px
        }
        TopMaximize => {
            (cursor.y - b.top).abs() <= engage_px && cursor.x >= b.left && cursor.x <= b.right
        }
        BottomMaximize => {
            (cursor.y - b.bottom).abs() <= engage_px && cursor.x >= b.left && cursor.x <= b.right
        }
        LeftHalf => {
            (cursor.x - b.left).abs() <= engage_px && cursor.y >= b.top && cursor.y <= b.bottom
        }
        RightHalf => {
            (cursor.x - b.right).abs() <= engage_px && cursor.y >= b.top && cursor.y <= b.bottom
        }
        LeftThird => (cursor.x - b.left).abs() <= engage_px,
        // MiddleThird: no edge to probe — eligible whenever cursor is in middle-x band of monitor.
        MiddleThird => {
            cursor.x > b.left + (b.right - b.left) / 3
                && cursor.x < b.left + 2 * (b.right - b.left) / 3
        }
        RightThird => (cursor.x - b.right).abs() <= engage_px,
    }
}

/// Lower number = higher priority.
fn priority_rank(id: SnapZoneId) -> u8 {
    use SnapZoneId::*;
    match id {
        TopLeftQuarter | TopRightQuarter | BottomLeftQuarter | BottomRightQuarter => 0,
        TopMaximize => 1,
        BottomMaximize => 2,
        LeftHalf | RightHalf => 3,
        LeftThird | MiddleThird | RightThird => 4,
    }
}

/// Distance from cursor to the nearest edge of the engaged zone's target rect.
/// When the cursor is outside the rect this is the standard point-to-boundary distance;
/// when it is inside, it is the minimum distance to any of the four edges (always ≥ 0).
fn disengage_distance_to_current(session: &SnapSession, cursor: Point) -> i32 {
    let Some(ez) = &session.engaged else {
        return i32::MAX;
    };
    let r = ez.target_rect;
    let inside_x = cursor.x >= r.left && cursor.x <= r.right;
    let inside_y = cursor.y >= r.top && cursor.y <= r.bottom;
    if inside_x && inside_y {
        // Inside rect: nearest wall distance.
        let dl = cursor.x - r.left;
        let dr = r.right - cursor.x;
        let dt = cursor.y - r.top;
        let db = r.bottom - cursor.y;
        dl.min(dr).min(dt).min(db).max(0)
    } else {
        // Outside rect: Chebyshev distance to boundary.
        let dx = 0.max((r.left - cursor.x).max(cursor.x - r.right));
        let dy = 0.max((r.top - cursor.y).max(cursor.y - r.bottom));
        dx.max(dy)
    }
}
