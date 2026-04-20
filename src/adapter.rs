//! Bridges engine `Action`s to Win32 calls via `win_api`, and resolves
//! cursor positions into `DragTarget` records for the engine.

#![cfg(target_os = "windows")]

use alt3rsnap::engine::geometry::Point;
use alt3rsnap::engine::state::{Action, DragTarget};

pub unsafe fn resolve_target(_cursor: Point) -> Option<DragTarget> {
    // Filled in Task 24.
    None
}

pub fn apply_actions(_actions: &[Action]) -> bool {
    // Returns true if any action requested SwallowEvent.
    // Filled in Task 24.
    false
}
