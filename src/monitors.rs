//! Monitor snapshot — Windows adapter module.
//! Enumerates displays and returns an engine-facing `MonitorSnapshot`.

#![cfg(windows)]

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
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
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
