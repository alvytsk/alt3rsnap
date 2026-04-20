//! Thin, stateless Win32 wrappers used by hook.rs and the tray.

#![cfg(target_os = "windows")]

use alt3rsnap::engine::geometry::{Point as GPoint, Rect as GRect};
use alt3rsnap::engine::rules::{WindowInfo, WindowTraits};
use alt3rsnap::engine::state::WindowId;

use windows::core::PWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_NAME_FORMAT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetAncestor, GetClassNameW, GetWindowLongPtrW, GetWindowRect,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsZoomed, SetForegroundWindow,
    SetWindowPos, ShowWindow, WindowFromPoint, GA_ROOT, GWL_EXSTYLE, GWL_STYLE, HWND_TOP,
    SWP_NOACTIVATE, SWP_NOSENDCHANGING, SWP_NOZORDER, SW_RESTORE, WINDOW_EX_STYLE, WINDOW_STYLE,
    WS_CAPTION, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_OVERLAPPEDWINDOW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::keybd_event;

pub fn to_win_point(p: GPoint) -> POINT { POINT { x: p.x, y: p.y } }
pub fn from_win_rect(r: RECT) -> GRect {
    GRect { left: r.left, top: r.top, right: r.right, bottom: r.bottom }
}

pub unsafe fn window_under_cursor(cursor: GPoint) -> Option<WindowInfo> {
    let mut hwnd = WindowFromPoint(to_win_point(cursor));
    if hwnd.0.is_null() { return None; }
    hwnd = GetAncestor(hwnd, GA_ROOT);
    if hwnd.0.is_null() { return None; }
    collect_window_info(hwnd)
}

pub unsafe fn collect_window_info(hwnd: HWND) -> Option<WindowInfo> {
    let class = read_class_name(hwnd)?;
    if SKIP_CLASSES.iter().any(|c| *c == class) { return None; }
    let title = read_window_text(hwnd);
    let process = read_process_basename(hwnd)?;
    let traits = read_window_traits(hwnd);
    Some(WindowInfo {
        process_basename: process.to_lowercase(),
        class_name: class,
        title,
        traits,
    })
}

const SKIP_CLASSES: &[&str] = &[
    "Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd",
    "Button", // desktop's "Show Desktop" button
];

unsafe fn read_class_name(hwnd: HWND) -> Option<String> {
    let mut buf = [0u16; 256];
    let n = GetClassNameW(hwnd, &mut buf);
    if n <= 0 { return None; }
    Some(String::from_utf16_lossy(&buf[..n as usize]))
}

unsafe fn read_window_text(hwnd: HWND) -> String {
    let n = GetWindowTextLengthW(hwnd);
    if n <= 0 { return String::new(); }
    let mut buf = vec![0u16; (n + 1) as usize];
    let got = GetWindowTextW(hwnd, &mut buf);
    String::from_utf16_lossy(&buf[..got as usize])
}

unsafe fn read_process_basename(hwnd: HWND) -> Option<String> {
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 { return None; }
    let proc_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
    let mut buf = [0u16; 1024];
    let mut size: u32 = buf.len() as u32;
    let ok = QueryFullProcessImageNameW(
        proc_handle,
        PROCESS_NAME_FORMAT(0),
        PWSTR(buf.as_mut_ptr()),
        &mut size,
    ).is_ok();
    let _ = windows::Win32::Foundation::CloseHandle(proc_handle);
    if !ok { return None; }
    let full = String::from_utf16_lossy(&buf[..size as usize]);
    Some(std::path::Path::new(&full)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or(full))
}

unsafe fn read_window_traits(hwnd: HWND) -> WindowTraits {
    let ex_style = WINDOW_EX_STYLE(GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32);
    let cloaked: u32 = {
        let mut v: u32 = 0;
        let _ = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut v as *mut _ as *mut _,
            std::mem::size_of::<u32>() as u32,
        );
        v
    };
    WindowTraits {
        is_topmost: (ex_style.0 & WS_EX_TOPMOST.0) != 0,
        is_cloaked: cloaked != 0,
        is_tool:    (ex_style.0 & WS_EX_TOOLWINDOW.0) != 0,
        is_owned:   false, // populate later if needed
    }
}

pub unsafe fn hwnd_rect(hwnd: HWND) -> Option<GRect> {
    let mut r = RECT::default();
    GetWindowRect(hwnd, &mut r).ok()?;
    Some(from_win_rect(r))
}

pub unsafe fn set_window_rect(hwnd: HWND, rect: GRect) {
    let _ = SetWindowPos(
        hwnd,
        HWND_TOP,
        rect.left,
        rect.top,
        rect.width(),
        rect.height(),
        SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOSENDCHANGING,
    );
}

pub unsafe fn is_zoomed(hwnd: HWND) -> bool { IsZoomed(hwnd).as_bool() }

pub unsafe fn restore(hwnd: HWND) { let _ = ShowWindow(hwnd, SW_RESTORE); }

pub unsafe fn raise(hwnd: HWND) {
    let _ = BringWindowToTop(hwnd);
    let _ = SetForegroundWindow(hwnd);
}

pub unsafe fn cancel_menu_activation() {
    // Send a harmless key (VK_F18 = 0x87) up-down to swallow Alt-triggered menu focus.
    keybd_event(0x87, 0, Default::default(), 0);
    keybd_event(0x87, 0, windows::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_KEYUP, 0);
}

pub fn hwnd_to_id(hwnd: HWND) -> WindowId { WindowId(hwnd.0 as usize as u64) }
pub fn id_to_hwnd(id: WindowId) -> HWND { HWND(id.0 as usize as *mut core::ffi::c_void) }

pub unsafe fn window_under_cursor_hwnd(cursor: GPoint) -> Option<HWND> {
    let mut hwnd = WindowFromPoint(to_win_point(cursor));
    if hwnd.0.is_null() { return None; }
    hwnd = GetAncestor(hwnd, GA_ROOT);
    if hwnd.0.is_null() { return None; }
    Some(hwnd)
}
