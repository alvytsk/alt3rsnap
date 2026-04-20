//! Rect/Point math, 3×3 sector picking, resize math.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const ZERO: Point = Point { x: 0, y: 0 };

    pub fn delta(self, other: Point) -> Point {
        Point { x: self.x - other.x, y: self.y - other.y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(self) -> i32 { self.right - self.left }
    pub fn height(self) -> i32 { self.bottom - self.top }

    pub fn translate_by(self, d: Point) -> Rect {
        Rect {
            left: self.left + d.x,
            top: self.top + d.y,
            right: self.right + d.x,
            bottom: self.bottom + d.y,
        }
    }

    pub fn contains(self, p: Point) -> bool {
        p.x >= self.left && p.x < self.right && p.y >= self.top && p.y < self.bottom
    }

    pub fn center(self) -> Point {
        Point {
            x: (self.left + self.right) / 2,
            y: (self.top + self.bottom) / 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sector {
    TopLeft, Top, TopRight,
    Left, Center, Right,
    BottomLeft, Bottom, BottomRight,
}

/// Classify `cursor` into one of nine sectors of `rect`. `center_fraction` is the
/// size of the center sector as a fraction of width/height (0.0..=1.0).
/// Clamps out-of-rect cursors to the nearest sector.
pub fn pick_sector(rect: Rect, cursor: Point, center_fraction: f32) -> Sector {
    let cf = center_fraction.clamp(0.0, 1.0);
    let side_width = (rect.width() as f32 * (1.0 - cf) / 2.0) as i32;
    let side_height = (rect.height() as f32 * (1.0 - cf) / 2.0) as i32;

    let left_edge = rect.left + side_width;
    let right_edge = rect.right - side_width;
    let top_edge = rect.top + side_height;
    let bottom_edge = rect.bottom - side_height;

    let col = if cursor.x < left_edge { 0 }
              else if cursor.x >= right_edge { 2 }
              else { 1 };
    let row = if cursor.y < top_edge { 0 }
              else if cursor.y >= bottom_edge { 2 }
              else { 1 };

    match (row, col) {
        (0, 0) => Sector::TopLeft,
        (0, 1) => Sector::Top,
        (0, 2) => Sector::TopRight,
        (1, 0) => Sector::Left,
        (1, 1) => Sector::Center,
        (1, 2) => Sector::Right,
        (2, 0) => Sector::BottomLeft,
        (2, 1) => Sector::Bottom,
        (2, 2) => Sector::BottomRight,
        _ => unreachable!(),
    }
}
