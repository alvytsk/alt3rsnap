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
