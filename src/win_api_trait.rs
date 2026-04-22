//! Adapter-facing Win32 boundary.
//!
//! The real impl (`Win32WinApi`, Windows only) wraps `src/win_api.rs`.
//! The mock impl (`RecordingWinApi`, always compiled) records every call for
//! Linux-runnable ordering and idempotence tests.

use crate::engine::geometry::Rect;
use crate::engine::state::WindowId;

/// Adapter's single Win32 surface. All side-effects go through here so tests
/// can mock them without linking to Win32.
pub trait WinApi {
    /// Returns `Err(())` if the Win32 call fails; the unit error is intentional
    /// (the only relevant signal is success/failure, not a detailed error type).
    #[allow(clippy::result_unit_err)]
    fn set_window_rect(&mut self, hwnd: WindowId, rect: Rect) -> Result<(), ()>;
    fn is_zoomed(&mut self, hwnd: WindowId) -> bool;
    fn show_maximize(&mut self, hwnd: WindowId);
    fn show_restore(&mut self, hwnd: WindowId);
    fn capture_mouse(&mut self, hwnd: WindowId);
    fn release_mouse(&mut self);
    fn overlay_show(&mut self, rect: Rect);
    fn overlay_hide(&mut self);
}

/// Recording mock; used by `tests/adapter_ordering.rs`. Compiles on all platforms.
pub struct RecordingWinApi {
    pub calls: Vec<Call>,
    pub overlay_visible: bool,
    /// If true, the NEXT `set_window_rect` call returns `Err(())` (then this flag clears).
    pub fail_next_set_window_rect: bool,
    /// If true, `is_zoomed` returns true instead of false.
    pub zoomed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    SetWindowRect(WindowId, Rect),
    IsZoomed(WindowId),
    ShowMaximize(WindowId),
    ShowRestore(WindowId),
    CaptureMouse(WindowId),
    ReleaseMouse,
    OverlayShow(Rect),
    OverlayHide,
}

impl Default for RecordingWinApi {
    fn default() -> Self {
        Self::new()
    }
}

impl RecordingWinApi {
    pub fn new() -> Self {
        Self {
            calls: Vec::new(),
            overlay_visible: false,
            fail_next_set_window_rect: false,
            zoomed: false,
        }
    }
}

impl WinApi for RecordingWinApi {
    fn set_window_rect(&mut self, hwnd: WindowId, rect: Rect) -> Result<(), ()> {
        self.calls.push(Call::SetWindowRect(hwnd, rect));
        if self.fail_next_set_window_rect {
            self.fail_next_set_window_rect = false;
            Err(())
        } else {
            Ok(())
        }
    }
    fn is_zoomed(&mut self, hwnd: WindowId) -> bool {
        self.calls.push(Call::IsZoomed(hwnd));
        self.zoomed
    }
    fn show_maximize(&mut self, hwnd: WindowId) {
        self.calls.push(Call::ShowMaximize(hwnd));
    }
    fn show_restore(&mut self, hwnd: WindowId) {
        self.calls.push(Call::ShowRestore(hwnd));
    }
    fn capture_mouse(&mut self, hwnd: WindowId) {
        self.calls.push(Call::CaptureMouse(hwnd));
    }
    fn release_mouse(&mut self) {
        self.calls.push(Call::ReleaseMouse);
    }
    fn overlay_show(&mut self, rect: Rect) {
        // Idempotent: don't record duplicate shows at the same rect.
        if self.overlay_visible {
            if let Some(Call::OverlayShow(last)) = self.calls.iter().rev().find(|c| {
                matches!(c, Call::OverlayShow(_) | Call::OverlayHide)
            }) {
                if *last == rect {
                    return;
                }
            }
        }
        self.overlay_visible = true;
        self.calls.push(Call::OverlayShow(rect));
    }
    fn overlay_hide(&mut self) {
        if !self.overlay_visible {
            return;
        }
        self.overlay_visible = false;
        self.calls.push(Call::OverlayHide);
    }
}

// ---- Win32 real impl (Windows only) ----
//
// `Win32WinApi` lives in the *binary* crate (src/adapter.rs or similar) rather
// than here, because `win_api` and `overlay` are declared in `main.rs`'s
// module tree and are therefore unreachable from the library crate.  The lib
// exports the `WinApi` trait and `RecordingWinApi` mock; the binary provides
// the concrete `Win32WinApi` that implements the trait.  See `src/adapter.rs`.
