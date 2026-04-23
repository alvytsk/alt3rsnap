//! Monitor snapshot — Windows adapter module.
//! Enumerates displays and returns an engine-facing `MonitorSnapshot`.

#![cfg(windows)]

use std::mem::size_of;

use alt3rsnap::engine::geometry::Rect;
use alt3rsnap::engine::snap::{MonitorInfo, MonitorSnapshot};
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

/// Returns a fresh snapshot of the current display layout.
/// Always callable; safe to invoke repeatedly (no caching in this function).
pub fn snapshot() -> MonitorSnapshot {
    let mut monitors: Vec<MonitorInfo> = Vec::with_capacity(4);

    unsafe extern "system" fn enum_proc(
        hmon: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lp: LPARAM,
    ) -> BOOL {
        // SAFETY: lp is a &mut Vec<MonitorInfo> passed by the caller below.
        let out = unsafe { &mut *(lp.0 as *mut Vec<MonitorInfo>) };
        let mut mi = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let ok = unsafe { GetMonitorInfoW(hmon, &mut mi) };
        if ok.as_bool() {
            let bounds = Rect {
                left: mi.rcMonitor.left,
                top: mi.rcMonitor.top,
                right: mi.rcMonitor.right,
                bottom: mi.rcMonitor.bottom,
            };
            let work_area = Rect {
                left: mi.rcWork.left,
                top: mi.rcWork.top,
                right: mi.rcWork.right,
                bottom: mi.rcWork.bottom,
            };
            let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
            let _ = unsafe { GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
            let scale = ((dpi_x as f32 / 96.0) * 100.0).round() as u32;
            out.push(MonitorInfo {
                bounds,
                work_area,
                scale,
            });
        }
        BOOL(1)
    }

    unsafe {
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(enum_proc),
            LPARAM(&mut monitors as *mut _ as isize),
        );
    }

    MonitorSnapshot { monitors }
}

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static CACHE: OnceLock<Mutex<Cached>> = OnceLock::new();

struct Cached {
    snapshot: MonitorSnapshot,
    last_refreshed: Instant,
    /// Set when a broadcast (WM_DISPLAYCHANGE / WM_SETTINGCHANGE) arrives mid-drag.
    /// Consumed at drag end via `on_drag_ended`.
    dirty: bool,
    /// Timestamp of the most recent refresh request; used by `flush_pending` to
    /// apply the 200 ms debounce.
    pending_refresh: Option<Instant>,
}

fn cache() -> &'static Mutex<Cached> {
    CACHE.get_or_init(|| {
        let s = snapshot();
        Mutex::new(Cached {
            snapshot: s,
            last_refreshed: Instant::now(),
            dirty: false,
            pending_refresh: None,
        })
    })
}

/// Debounce window: after a refresh request, `flush_pending` only fires the refresh
/// when this much time has elapsed. Subsequent requests within the window reset the
/// timer (only the last matters).
const DEBOUNCE: Duration = Duration::from_millis(200);

/// Returns a clone of the currently cached `MonitorSnapshot`. Thread-safe;
/// initialises the cache on first call.
pub fn current() -> MonitorSnapshot {
    cache().lock().unwrap().snapshot.clone()
}

/// Called from tool_window's broadcast handlers (`WM_DISPLAYCHANGE`,
/// `WM_SETTINGCHANGE(SPI_SETWORKAREA)`). If a drag is currently active, defer the
/// refresh to drag end (set `dirty`); otherwise schedule a debounced refresh that
/// `flush_pending` must complete after the 200 ms window.
pub fn request_refresh(drag_active: bool) {
    let mut c = cache().lock().unwrap();
    if drag_active {
        c.dirty = true;
        return;
    }
    c.pending_refresh = Some(Instant::now());
}

/// Called from tool_window's `WM_TIMER` when the 200 ms debounce expires. Only
/// fires the actual `snapshot()` call if at least `DEBOUNCE` has elapsed since
/// the last `request_refresh`.
pub fn flush_pending() {
    let mut c = cache().lock().unwrap();
    let Some(t0) = c.pending_refresh else { return };
    if Instant::now().duration_since(t0) < DEBOUNCE {
        return;
    }
    c.pending_refresh = None;
    c.dirty = false;
    c.snapshot = snapshot();
    c.last_refreshed = Instant::now();
}

/// Called from the adapter when a drag ends. If a broadcast arrived mid-drag
/// (setting `dirty`), refresh now to pick up the new layout for the next drag.
pub fn on_drag_ended() {
    let mut c = cache().lock().unwrap();
    if c.dirty {
        c.dirty = false;
        c.snapshot = snapshot();
        c.last_refreshed = Instant::now();
    }
}
