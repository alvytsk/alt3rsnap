#![cfg(target_os = "windows")]

use alt3rsnap::engine::geometry::Point;
use alt3rsnap::engine::rules::{evaluate, RuleAction};
use alt3rsnap::engine::state::{Action, DragTarget};
use crate::hook::ENGINE;
use crate::win_api;

use windows::Win32::Foundation::HWND;

pub unsafe fn resolve_target(cursor: Point) -> Option<DragTarget> {
    let info = win_api::window_under_cursor(cursor)?;
    let hwnd_raw = win_api::window_under_cursor_hwnd(cursor)?;

    let exclude = ENGINE.with(|e| {
        let eng = e.borrow();
        matches!(evaluate(&eng.config().rules, &info), Some(RuleAction::Exclude))
    });

    let initial_rect = win_api::hwnd_rect(hwnd_raw)?;
    let is_maximized = win_api::is_zoomed(hwnd_raw);

    Some(DragTarget {
        hwnd: win_api::hwnd_to_id(hwnd_raw),
        initial_rect,
        is_maximized,
        exclude,
    })
}

pub fn apply_actions(actions: &[Action]) -> bool {
    let mut swallow = false;
    for a in actions {
        match a {
            Action::BeginDrag { .. } => unsafe {
                win_api::capture_mouse(crate::tool_window::hwnd());
            },
            Action::UpdateDrag { hwnd, new_rect } => unsafe {
                win_api::set_window_rect(win_api::id_to_hwnd(*hwnd), *new_rect);
            },
            Action::EndDrag { .. } => unsafe {
                win_api::release_mouse();
            },
            Action::RestoreIfMaximized { hwnd, .. } => unsafe {
                win_api::restore(win_api::id_to_hwnd(*hwnd));
            },
            Action::RaiseWindow { hwnd } => unsafe {
                win_api::raise(win_api::id_to_hwnd(*hwnd));
            },
            Action::CancelMenuActivation => unsafe {
                win_api::cancel_menu_activation();
            },
            Action::SwallowEvent => { swallow = true; }
            Action::UpdateTrayIcon { .. } => {
                // Wired in Task 27 (tray).
            }
        }
    }
    swallow
}
