//! Engine snap module integration tests.

use alt3rsnap::engine::config::SnapEngineConfig;
use alt3rsnap::engine::geometry::{Point, Rect};
use alt3rsnap::engine::snap::{
    bake_zones, evaluate, Decision, MonitorInfo, MonitorSnapshot, SnapContext, SnapSession,
    SnapZoneId,
};

fn mon_1080p(bottom_taskbar_height: i32) -> MonitorInfo {
    MonitorInfo {
        bounds: Rect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        },
        work_area: Rect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080 - bottom_taskbar_height,
        },
        scale: 100,
    }
}

#[test]
fn bake_zones_default_toggles_produces_aero_like_set() {
    let snap = MonitorSnapshot {
        monitors: vec![mon_1080p(40)],
    };
    let cfg = SnapEngineConfig::default();
    let zones = bake_zones(&cfg, &snap);

    let ids: Vec<SnapZoneId> = zones.iter().map(|z| z.id).collect();
    assert!(ids.contains(&SnapZoneId::LeftHalf));
    assert!(ids.contains(&SnapZoneId::RightHalf));
    assert!(ids.contains(&SnapZoneId::TopMaximize));
    assert!(ids.contains(&SnapZoneId::TopLeftQuarter));
    assert!(ids.contains(&SnapZoneId::TopRightQuarter));
    assert!(ids.contains(&SnapZoneId::BottomLeftQuarter));
    assert!(ids.contains(&SnapZoneId::BottomRightQuarter));
    assert!(!ids.contains(&SnapZoneId::BottomMaximize));
    assert!(!ids.contains(&SnapZoneId::LeftThird));
}

#[test]
fn zone_rects_never_escape_work_area() {
    let snap = MonitorSnapshot {
        monitors: vec![mon_1080p(40)],
    };
    let cfg = SnapEngineConfig::default();
    let zones = bake_zones(&cfg, &snap);
    let wa = snap.monitors[0].work_area;
    for z in &zones {
        assert!(z.target_rect.left >= wa.left);
        assert!(z.target_rect.top >= wa.top);
        assert!(z.target_rect.right <= wa.right);
        assert!(z.target_rect.bottom <= wa.bottom);
    }
}

fn mk_session(mons: MonitorSnapshot, cfg: SnapEngineConfig) -> SnapSession {
    let zones = alt3rsnap::engine::snap::bake_zones(&cfg, &mons);
    SnapSession {
        ctx: SnapContext {
            monitors: mons,
            zones,
            engage_px: cfg.engage_px,
            disengage_px: cfg.disengage_px,
            grab: Point { x: 960, y: 540 },
            restore_guard_active: false,
        },
        engaged: None,
        last_preview_rect: None,
        suspended: false,
        restore_guard_cleared: true,
    }
}

#[test]
fn evaluate_engages_on_left_edge_within_24px() {
    let mons = MonitorSnapshot {
        monitors: vec![mon_1080p(40)],
    };
    let mut s = mk_session(mons, SnapEngineConfig::default());
    let d = evaluate(&mut s, Point { x: 23, y: 500 });
    match d {
        Decision::Engage(z) => assert_eq!(z.id, SnapZoneId::LeftHalf),
        other => panic!("expected Engage(LeftHalf), got {:?}", other),
    }
    assert!(s.engaged.is_some());
}

#[test]
fn evaluate_holds_while_cursor_stays_near_edge() {
    let mons = MonitorSnapshot {
        monitors: vec![mon_1080p(40)],
    };
    let mut s = mk_session(mons, SnapEngineConfig::default());
    let _ = evaluate(&mut s, Point { x: 23, y: 500 });
    let d = evaluate(&mut s, Point { x: 25, y: 500 });
    assert_eq!(d, Decision::Hold);
}

#[test]
fn evaluate_disengages_only_after_32px_away() {
    let mons = MonitorSnapshot {
        monitors: vec![mon_1080p(40)],
    };
    let mut s = mk_session(mons, SnapEngineConfig::default());
    let _ = evaluate(&mut s, Point { x: 10, y: 500 });
    // Still inside disengage radius.
    assert_eq!(evaluate(&mut s, Point { x: 31, y: 500 }), Decision::Hold);
    // Cross disengage threshold.
    assert_eq!(
        evaluate(&mut s, Point { x: 33, y: 500 }),
        Decision::Disengage
    );
    assert!(s.engaged.is_none());
}

#[test]
fn top_left_corner_prefers_quarter_over_half_and_maximize() {
    let mons = MonitorSnapshot {
        monitors: vec![mon_1080p(40)],
    };
    let mut s = mk_session(mons, SnapEngineConfig::default());
    let d = evaluate(&mut s, Point { x: 5, y: 5 });
    match d {
        Decision::Engage(z) => assert_eq!(z.id, SnapZoneId::TopLeftQuarter),
        other => panic!("expected TopLeftQuarter, got {:?}", other),
    }
}

#[test]
fn top_middle_prefers_top_maximize_over_halves() {
    let mons = MonitorSnapshot {
        monitors: vec![mon_1080p(40)],
    };
    let mut s = mk_session(mons, SnapEngineConfig::default());
    let d = evaluate(&mut s, Point { x: 960, y: 5 });
    match d {
        Decision::Engage(z) => assert_eq!(z.id, SnapZoneId::TopMaximize),
        other => panic!("expected TopMaximize, got {:?}", other),
    }
}

#[test]
fn zone_switch_from_left_half_to_top_left_quarter_in_single_evaluation() {
    let mons = MonitorSnapshot {
        monitors: vec![mon_1080p(40)],
    };
    let mut s = mk_session(mons, SnapEngineConfig::default());
    // First engage LeftHalf mid-screen-left.
    let d1 = evaluate(&mut s, Point { x: 5, y: 500 });
    assert!(matches!(d1, Decision::Engage(ref z) if z.id == SnapZoneId::LeftHalf));
    // Move cursor up toward top-left corner — must switch directly to quarter.
    let d2 = evaluate(&mut s, Point { x: 5, y: 5 });
    match d2 {
        Decision::Engage(z) => assert_eq!(z.id, SnapZoneId::TopLeftQuarter),
        other => panic!("expected Engage(TopLeftQuarter), got {:?}", other),
    }
    // Session must agree: the new engagement must be visible on the session too.
    assert!(matches!(&s.engaged, Some(e) if e.id == SnapZoneId::TopLeftQuarter));
}

#[test]
fn disabling_quarters_falls_back_to_top_maximize_at_corner() {
    let mons = MonitorSnapshot {
        monitors: vec![mon_1080p(40)],
    };
    let mut cfg = SnapEngineConfig::default();
    cfg.zones.top_left_quarter = false;
    let mut s = mk_session(mons, cfg);
    let d = evaluate(&mut s, Point { x: 5, y: 5 });
    // With top_left_quarter disabled, next priority in the candidate set is TopMaximize (top edge).
    match d {
        Decision::Engage(z) => assert_eq!(z.id, SnapZoneId::TopMaximize),
        other => panic!("expected TopMaximize fallback, got {:?}", other),
    }
}

#[test]
fn restore_guard_suppresses_engage_until_16px_movement() {
    let mons = MonitorSnapshot {
        monitors: vec![mon_1080p(40)],
    };
    let mut s = mk_session(mons, SnapEngineConfig::default());
    s.ctx.restore_guard_active = true;
    s.restore_guard_cleared = false;
    s.ctx.grab = Point { x: 100, y: 100 };

    // Within 16 px of grab — no engagement even near edge.
    let d1 = evaluate(&mut s, Point { x: 110, y: 100 });
    assert_eq!(d1, Decision::None);
    assert!(s.engaged.is_none());

    // Beyond 16 px — evaluation resumes. Still not near edge → None.
    let d2 = evaluate(&mut s, Point { x: 117, y: 100 });
    assert_eq!(d2, Decision::None);
    assert!(s.restore_guard_cleared);

    // Guard latched open — moving to edge engages.
    let d3 = evaluate(&mut s, Point { x: 5, y: 500 });
    assert!(matches!(d3, Decision::Engage(ref z) if z.id == SnapZoneId::LeftHalf));
}

#[test]
fn space_suspend_hides_preview_and_suppresses_future_engage() {
    let mons = MonitorSnapshot {
        monitors: vec![mon_1080p(40)],
    };
    let mut s = mk_session(mons, SnapEngineConfig::default());
    let _ = evaluate(&mut s, Point { x: 5, y: 500 });
    assert!(s.engaged.is_some());

    s.suspended = true;
    let d = evaluate(&mut s, Point { x: 6, y: 500 });
    assert_eq!(d, Decision::Disengage);
    assert!(s.engaged.is_none());
    assert_eq!(s.last_preview_rect, None);

    // While suspended, no engagement.
    let d2 = evaluate(&mut s, Point { x: 10, y: 500 });
    assert_eq!(d2, Decision::None);

    // Release Space → next evaluation engages.
    s.suspended = false;
    let d3 = evaluate(&mut s, Point { x: 10, y: 500 });
    assert!(matches!(d3, Decision::Engage(ref z) if z.id == SnapZoneId::LeftHalf));
}

#[test]
fn cross_monitor_engages_on_secondary_left_edge() {
    // Two monitors side-by-side, 1920 each; secondary is at x=1920..3840.
    let mons = MonitorSnapshot {
        monitors: vec![
            mon_1080p(40),
            MonitorInfo {
                bounds: Rect {
                    left: 1920,
                    top: 0,
                    right: 3840,
                    bottom: 1080,
                },
                work_area: Rect {
                    left: 1920,
                    top: 0,
                    right: 3840,
                    bottom: 1040,
                },
                scale: 100,
            },
        ],
    };
    let mut s = mk_session(mons, SnapEngineConfig::default());
    // Cursor at x=1925 on monitor 2 → within 24 px of its left edge = monitor-2 LeftHalf.
    let d = evaluate(&mut s, Point { x: 1925, y: 500 });
    match d {
        Decision::Engage(z) => {
            assert_eq!(z.id, SnapZoneId::LeftHalf);
            assert_eq!(z.target_rect.left, 1920);
        }
        other => panic!("expected monitor-2 LeftHalf, got {:?}", other),
    }
}

#[test]
fn cursor_at_shared_display_pixel_picks_first_monitor_zone() {
    // Two monitors side-by-side, sharing the pixel at x=1920.
    // The inclusive-bounds guard means the cursor is "on" BOTH monitors at x=1920;
    // best_candidate's strict-< priority tie-break keeps the first-baked zone, so
    // monitor-0's RightHalf wins. This pins the current behavior — a future change
    // to half-open bounds semantics (matching MonitorFromPoint) would deliberately
    // flip this test too.
    let mons = MonitorSnapshot {
        monitors: vec![
            mon_1080p(40),
            MonitorInfo {
                bounds: Rect {
                    left: 1920,
                    top: 0,
                    right: 3840,
                    bottom: 1080,
                },
                work_area: Rect {
                    left: 1920,
                    top: 0,
                    right: 3840,
                    bottom: 1040,
                },
                scale: 100,
            },
        ],
    };
    let mut s = mk_session(mons, SnapEngineConfig::default());
    let d = evaluate(&mut s, Point { x: 1920, y: 500 });
    match d {
        Decision::Engage(z) => {
            assert_eq!(z.id, SnapZoneId::RightHalf);
            assert_eq!(z.target_rect.right, 1920);
        }
        other => panic!(
            "expected monitor-0 RightHalf at shared pixel, got {:?}",
            other
        ),
    }
}

#[test]
fn zones_baked_at_session_creation_are_immutable_across_reload() {
    let mons = MonitorSnapshot {
        monitors: vec![mon_1080p(40)],
    };
    let cfg_on = SnapEngineConfig::default();
    let mut s = mk_session(mons.clone(), cfg_on);

    // Simulate a reload that would disable snap — not applied to the session directly.
    // The session keeps its own ctx; changing the "outside" config does not re-bake.
    // (Engine-level invariant: SnapContext is immutable for drag lifetime.)
    let cfg_off = SnapEngineConfig {
        enabled: false,
        ..SnapEngineConfig::default()
    };
    let _ = &cfg_off; // never applied to s — that is the point

    let d = evaluate(&mut s, Point { x: 5, y: 500 });
    assert!(
        matches!(d, Decision::Engage(ref z) if z.id == SnapZoneId::LeftHalf),
        "drag-local zones must still evaluate even if outer config disabled snap"
    );
}

use proptest::prelude::*;

proptest! {
    #[test]
    fn proptest_zones_never_escape_work_area(
        taskbar_h in 0i32..200i32,
        w in 640i32..=7680i32,
        h in 480i32..=4320i32,
    ) {
        let mi = MonitorInfo {
            bounds: Rect { left: 0, top: 0, right: w, bottom: h },
            work_area: Rect { left: 0, top: 0, right: w, bottom: (h - taskbar_h).max(1) },
            scale: 100,
        };
        let snap = MonitorSnapshot { monitors: vec![mi.clone()] };
        let zones = alt3rsnap::engine::snap::bake_zones(&SnapEngineConfig::default(), &snap);
        for z in &zones {
            prop_assert!(z.target_rect.left >= mi.work_area.left);
            prop_assert!(z.target_rect.top >= mi.work_area.top);
            prop_assert!(z.target_rect.right <= mi.work_area.right);
            prop_assert!(z.target_rect.bottom <= mi.work_area.bottom);
        }
    }
}
