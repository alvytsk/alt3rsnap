//! DPI awareness: set Per-Monitor V2 before any UI work.

#![cfg(target_os = "windows")]

use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};

pub fn init() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        // Ignore failure: manifest also declares PerMonitorV2, so we are at worst double-set.
    }
}
