use alt3rsnap::engine::geometry::{apply_resize, ResizeAnchor};
use alt3rsnap::engine::geometry::{pick_sector, Sector};
use alt3rsnap::engine::geometry::{Point, Rect};

#[test]
fn rect_translate_shifts_all_four_sides() {
    let r = Rect {
        left: 10,
        top: 20,
        right: 110,
        bottom: 220,
    };
    let shifted = r.translate_by(Point { x: 5, y: -3 });
    assert_eq!(
        shifted,
        Rect {
            left: 15,
            top: 17,
            right: 115,
            bottom: 217
        }
    );
}

#[test]
fn rect_width_and_height() {
    let r = Rect {
        left: 10,
        top: 20,
        right: 110,
        bottom: 220,
    };
    assert_eq!(r.width(), 100);
    assert_eq!(r.height(), 200);
}

#[test]
fn rect_contains_point_uses_inclusive_left_top_exclusive_right_bottom() {
    let r = Rect {
        left: 0,
        top: 0,
        right: 10,
        bottom: 10,
    };
    assert!(r.contains(Point { x: 0, y: 0 }));
    assert!(r.contains(Point { x: 9, y: 9 }));
    assert!(!r.contains(Point { x: 10, y: 5 }));
    assert!(!r.contains(Point { x: 5, y: 10 }));
    assert!(!r.contains(Point { x: -1, y: 0 }));
}

#[test]
fn sector_picks_top_left_corner() {
    let r = Rect {
        left: 0,
        top: 0,
        right: 300,
        bottom: 300,
    };
    let fraction = 0.333_f32;
    assert_eq!(
        pick_sector(r, Point { x: 10, y: 10 }, fraction),
        Sector::TopLeft
    );
}

#[test]
fn sector_picks_center() {
    let r = Rect {
        left: 0,
        top: 0,
        right: 300,
        bottom: 300,
    };
    let fraction = 0.333_f32;
    assert_eq!(
        pick_sector(r, Point { x: 150, y: 150 }, fraction),
        Sector::Center
    );
}

#[test]
fn sector_picks_bottom_right() {
    let r = Rect {
        left: 0,
        top: 0,
        right: 300,
        bottom: 300,
    };
    let fraction = 0.333_f32;
    assert_eq!(
        pick_sector(r, Point { x: 290, y: 290 }, fraction),
        Sector::BottomRight
    );
}

#[test]
fn sector_on_the_top_edge_row_returns_top() {
    let r = Rect {
        left: 0,
        top: 0,
        right: 300,
        bottom: 300,
    };
    let fraction = 0.333_f32;
    assert_eq!(
        pick_sector(r, Point { x: 150, y: 10 }, fraction),
        Sector::Top
    );
}

#[test]
fn sector_clamps_out_of_rect_cursors_to_nearest_sector() {
    let r = Rect {
        left: 0,
        top: 0,
        right: 300,
        bottom: 300,
    };
    let fraction = 0.333_f32;
    // Outside top-left — should clamp to TopLeft.
    assert_eq!(
        pick_sector(r, Point { x: -20, y: -20 }, fraction),
        Sector::TopLeft
    );
}

#[test]
fn resize_from_top_left_moves_left_and_top_only() {
    let r = Rect {
        left: 100,
        top: 100,
        right: 200,
        bottom: 200,
    };
    let out = apply_resize(r, ResizeAnchor::TopLeft, Point { x: -10, y: -5 });
    assert_eq!(
        out,
        Rect {
            left: 90,
            top: 95,
            right: 200,
            bottom: 200
        }
    );
}

#[test]
fn resize_from_bottom_right_moves_right_and_bottom() {
    let r = Rect {
        left: 100,
        top: 100,
        right: 200,
        bottom: 200,
    };
    let out = apply_resize(r, ResizeAnchor::BottomRight, Point { x: 10, y: 15 });
    assert_eq!(
        out,
        Rect {
            left: 100,
            top: 100,
            right: 210,
            bottom: 215
        }
    );
}

#[test]
fn resize_from_left_edge_only_moves_left_side() {
    let r = Rect {
        left: 100,
        top: 100,
        right: 200,
        bottom: 200,
    };
    let out = apply_resize(r, ResizeAnchor::Left, Point { x: -20, y: 50 });
    // Only left changes; vertical delta ignored for Left anchor.
    assert_eq!(
        out,
        Rect {
            left: 80,
            top: 100,
            right: 200,
            bottom: 200
        }
    );
}

#[test]
fn resize_center_symmetric_moves_opposite_edges_equally() {
    let r = Rect {
        left: 100,
        top: 100,
        right: 200,
        bottom: 200,
    };
    let out = apply_resize(r, ResizeAnchor::CenterSymmetric, Point { x: 10, y: 5 });
    // x+10 → right moves +10, left moves -10
    // y+5 → bottom moves +5, top moves -5
    assert_eq!(
        out,
        Rect {
            left: 90,
            top: 95,
            right: 210,
            bottom: 205
        }
    );
}

// --- center_mode = "bottom_right" geometry ---
// BR-anchor: the TopLeft resize anchor is used, which keeps (right, bottom) fixed
// and moves (left, top) by the delta.

#[test]
fn resize_top_left_anchor_keeps_right_and_bottom_fixed() {
    let r = Rect {
        left: 100,
        top: 100,
        right: 200,
        bottom: 200,
    };
    // Dragging right (+10) and down (+8): only left/top move.
    let out = apply_resize(r, ResizeAnchor::TopLeft, Point { x: 10, y: 8 });
    assert_eq!(
        out,
        Rect {
            left: 110,
            top: 108,
            right: 200,
            bottom: 200,
        }
    );
}

#[test]
fn resize_top_left_anchor_dragging_up_left_expands_window() {
    let r = Rect {
        left: 100,
        top: 100,
        right: 200,
        bottom: 200,
    };
    let out = apply_resize(r, ResizeAnchor::TopLeft, Point { x: -15, y: -20 });
    assert_eq!(
        out,
        Rect {
            left: 85,
            top: 80,
            right: 200,
            bottom: 200,
        }
    );
}

// --- sector boundary tests at extreme center_fraction ---

#[test]
fn sector_with_tiny_center_fraction_puts_most_area_in_edges() {
    // center_fraction = 0.1 → center band is only 10% of width/height.
    // A cursor at 50% of the rect should still land in Center.
    let r = Rect {
        left: 0,
        top: 0,
        right: 100,
        bottom: 100,
    };
    assert_eq!(pick_sector(r, Point { x: 50, y: 50 }, 0.1), Sector::Center);
    // Cursor at 5% from left edge → TopLeft or Left (outside center band).
    assert_ne!(pick_sector(r, Point { x: 5, y: 50 }, 0.1), Sector::Center);
}

#[test]
fn sector_with_large_center_fraction_puts_most_area_in_center() {
    // center_fraction = 0.9 → outer bands are only 5% of width/height each.
    let r = Rect {
        left: 0,
        top: 0,
        right: 100,
        bottom: 100,
    };
    assert_eq!(pick_sector(r, Point { x: 50, y: 50 }, 0.9), Sector::Center);
    // Cursor at 3% from left edge → TopLeft (in the outer band).
    assert_eq!(pick_sector(r, Point { x: 3, y: 3 }, 0.9), Sector::TopLeft);
    // Cursor at 10% still inside center band with fraction 0.9.
    assert_eq!(pick_sector(r, Point { x: 10, y: 10 }, 0.9), Sector::Center);
}

#[test]
fn sector_boundary_exact_edge_of_center_band() {
    // With f32 fraction 1/3 on a 300-wide rect:
    //   side_width = (300.0 * (1.0 - 1/3) / 2.0) as i32 = 99  (f32 truncation)
    //   left_edge = 0 + 99 = 99;  right_edge = 300 - 99 = 201.
    let r = Rect {
        left: 0,
        top: 0,
        right: 300,
        bottom: 300,
    };
    // x=98: Left sector (just outside center band).
    assert_eq!(
        pick_sector(r, Point { x: 98, y: 150 }, 1.0 / 3.0),
        Sector::Left
    );
    // x=99: Center sector (on the left edge of center band, inclusive).
    assert_eq!(
        pick_sector(r, Point { x: 99, y: 150 }, 1.0 / 3.0),
        Sector::Center
    );
    // x=200: still Center sector.
    assert_eq!(
        pick_sector(r, Point { x: 200, y: 150 }, 1.0 / 3.0),
        Sector::Center
    );
    // x=201: Right sector (first pixel outside the center band).
    assert_eq!(
        pick_sector(r, Point { x: 201, y: 150 }, 1.0 / 3.0),
        Sector::Right
    );
}
