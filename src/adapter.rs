#![cfg(target_os = "windows")]

use crate::hook::ENGINE;
use crate::win_api;
use alt3rsnap::engine::geometry::{Point, Rect};
use alt3rsnap::engine::rules::{evaluate, RuleAction};
use alt3rsnap::engine::state::{Action, DragTarget, WindowId};
use alt3rsnap::swallow_latch::SwallowLatch;
use alt3rsnap::win_api_trait::WinApi;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static LAST_BALLOON_EPOCH_SECS: AtomicU64 = AtomicU64::new(0);

static SWALLOW_LATCH: std::sync::OnceLock<SwallowLatch> = std::sync::OnceLock::new();

pub fn swallow_latch() -> &'static SwallowLatch {
    SWALLOW_LATCH.get_or_init(SwallowLatch::new)
}

/// Returns true if a window drag is currently in progress.
/// Stub in Slice G; Slice H2 Task H2.2 replaces the body with a real
/// `AtomicBool` set by `apply_actions` on `BeginDrag`/`EndDrag`.
pub fn drag_active() -> bool {
    false
}

fn maybe_balloon_uipi() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let last = LAST_BALLOON_EPOCH_SECS.load(Ordering::Relaxed);
    if now.saturating_sub(last) < 60 {
        return;
    }
    LAST_BALLOON_EPOCH_SECS.store(now, Ordering::Relaxed);
    crate::tray::show_balloon(
        "Alt3rSnap",
        "Can't move this window while running unelevated. Right-click tray → Restart elevated.",
    );
}

pub unsafe fn resolve_target(cursor: Point) -> Option<DragTarget> {
    let info = win_api::window_under_cursor(cursor)?;
    let hwnd_raw = win_api::window_under_cursor_hwnd(cursor)?;

    let exclude = ENGINE.with(|e| {
        let eng = e.borrow();
        matches!(
            evaluate(&eng.config().rules, &info),
            Some(RuleAction::Exclude)
        )
    });

    let initial_rect = win_api::hwnd_rect(hwnd_raw)?;
    let is_maximized = win_api::is_zoomed(hwnd_raw);

    Some(DragTarget {
        hwnd: win_api::hwnd_to_id(hwnd_raw),
        initial_rect,
        is_maximized,
        exclude,
        monitor_snapshot: None,
    })
}

pub fn apply_actions(actions: &[Action]) -> bool {
    // Spec §3.5: clear the latch defensively before any BeginDrag in this batch.
    if actions
        .iter()
        .any(|a| matches!(a, Action::BeginDrag { .. }))
    {
        swallow_latch().on_begin_drag();
    }

    let mut swallow = false;
    for a in actions {
        match a {
            Action::BeginDrag { .. } => unsafe {
                win_api::capture_mouse(crate::tool_window::hwnd());
            },
            Action::UpdateDrag { hwnd, new_rect } => unsafe {
                let ok = win_api::set_window_rect(win_api::id_to_hwnd(*hwnd), *new_rect);
                if !ok && !crate::elevate::is_elevated() {
                    maybe_balloon_uipi();
                }
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
            Action::SwallowEvent => {
                swallow = true;
            }
            Action::UpdateTrayIcon { enabled } => {
                crate::tray::set_enabled_flag(*enabled);
            }
            Action::ToggleMaximize { hwnd } => unsafe {
                let h = win_api::id_to_hwnd(*hwnd);
                if win_api::is_zoomed(h) {
                    win_api::restore(h);
                } else {
                    win_api::maximize(h);
                }
                swallow_latch().set(now_ms());
            },
            // Slice H2 owns the real behaviour for snap overlay / snap apply.
            Action::ShowSnapPreview { .. } => {}
            Action::HideSnapPreview => {}
            Action::ApplySnapRect { .. } => {}
        }
    }
    swallow
}

fn now_ms() -> u64 {
    // GetTickCount64 is monotonic, unaffected by system clock adjustment.
    unsafe { windows::Win32::System::SystemInformation::GetTickCount64() }
}

// ---- Win32 real WinApi impl ----

/// Concrete `WinApi` implementation that calls the real Win32 wrappers and the
/// overlay.  Lives here (binary crate) rather than in `win_api_trait` because
/// `win_api` and `overlay` are declared in `main.rs`'s module tree.
pub struct Win32WinApi {
    pub hinstance: windows::Win32::Foundation::HINSTANCE,
}

impl WinApi for Win32WinApi {
    #[allow(clippy::result_unit_err)]
    fn set_window_rect(&mut self, hwnd: WindowId, rect: Rect) -> Result<(), ()> {
        let h = win_api::id_to_hwnd(hwnd);
        if unsafe { win_api::set_window_rect(h, rect) } {
            Ok(())
        } else {
            Err(())
        }
    }
    fn is_zoomed(&mut self, hwnd: WindowId) -> bool {
        let h = win_api::id_to_hwnd(hwnd);
        unsafe { win_api::is_zoomed(h) }
    }
    fn show_maximize(&mut self, hwnd: WindowId) {
        let h = win_api::id_to_hwnd(hwnd);
        unsafe { win_api::maximize(h) };
    }
    fn show_restore(&mut self, hwnd: WindowId) {
        let h = win_api::id_to_hwnd(hwnd);
        unsafe { win_api::restore(h) };
    }
    fn capture_mouse(&mut self, hwnd: WindowId) {
        let h = win_api::id_to_hwnd(hwnd);
        unsafe { win_api::capture_mouse(h) };
    }
    fn release_mouse(&mut self) {
        unsafe { win_api::release_mouse() };
    }
    fn overlay_show(&mut self, rect: Rect) {
        crate::overlay::show(self.hinstance, rect);
    }
    fn overlay_hide(&mut self) {
        crate::overlay::hide();
    }
}
