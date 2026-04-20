use alt3rsnap::engine::geometry::{Point, Rect};

#[test]
fn rect_translate_shifts_all_four_sides() {
    let r = Rect { left: 10, top: 20, right: 110, bottom: 220 };
    let shifted = r.translate_by(Point { x: 5, y: -3 });
    assert_eq!(shifted, Rect { left: 15, top: 17, right: 115, bottom: 217 });
}

#[test]
fn rect_width_and_height() {
    let r = Rect { left: 10, top: 20, right: 110, bottom: 220 };
    assert_eq!(r.width(), 100);
    assert_eq!(r.height(), 200);
}

#[test]
fn rect_contains_point_uses_inclusive_left_top_exclusive_right_bottom() {
    let r = Rect { left: 0, top: 0, right: 10, bottom: 10 };
    assert!(r.contains(Point { x: 0, y: 0 }));
    assert!(r.contains(Point { x: 9, y: 9 }));
    assert!(!r.contains(Point { x: 10, y: 5 }));
    assert!(!r.contains(Point { x: 5, y: 10 }));
    assert!(!r.contains(Point { x: -1, y: 0 }));
}
