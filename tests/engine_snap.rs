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
