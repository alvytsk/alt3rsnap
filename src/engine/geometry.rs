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
