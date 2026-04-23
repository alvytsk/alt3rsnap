//! Detect when a fullscreen application takes foreground, so the engine can pass-through.

#![cfg(target_os = "windows")]

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetWindowLongPtrW, GetWindowRect, EVENT_SYSTEM_FOREGROUND, GWL_EXSTYLE,
    GWL_STYLE, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WS_CAPTION, WS_EX_TOPMOST,
    WS_OVERLAPPED, WS_POPUP,
};

use crate::hook::ENGINE;
use alt3rsnap::engine::state::Event;

static mut WIN_EVENT_HOOK: HWINEVENTHOOK = HWINEVENTHOOK(std::ptr::null_mut());

pub fn install() {
    unsafe {
        WIN_EVENT_HOOK = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(win_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
    }
}

pub fn uninstall() {
    unsafe {
        if !WIN_EVENT_HOOK.0.is_null() {
            let _ = UnhookWinEvent(WIN_EVENT_HOOK);
            WIN_EVENT_HOOK = HWINEVENTHOOK(std::ptr::null_mut());
        }
    }
}

unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _obj: i32,
    _child: i32,
    _thread: u32,
    _time: u32,
) {
    if hwnd.0.is_null() {
        return;
    }
    let is_fs = is_fullscreen_window(hwnd);
    let ev = if is_fs {
        Event::FullscreenFocused
    } else {
        Event::FullscreenUnfocused
    };
    let _ = ENGINE.with(|e| {
        let actions = e.borrow_mut().handle(ev);
        let _ = crate::adapter::apply_actions(&actions);
    });
}

unsafe fn is_fullscreen_window(hwnd: HWND) -> bool {
    // 1. Not cloaked.
    let mut cloaked: u32 = 0;
    let _ = DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED,
        &mut cloaked as *mut _ as *mut _,
        std::mem::size_of::<u32>() as u32,
    );
    if cloaked != 0 {
        return false;
    }

    // 2. Class not in known-non-fullscreen set.
    let mut buf = [0u16; 128];
    let n = GetClassNameW(hwnd, &mut buf);
    if n > 0 {
        let class = String::from_utf16_lossy(&buf[..n as usize]);
        const SKIP: &[&str] = &[
            "Progman",
            "WorkerW",
            "Shell_TrayWnd",
            "Shell_SecondaryTrayWnd",
            "Windows.UI.Core.CoreWindow",
            "ApplicationFrameWindow",
            "XamlExplorerHostIslandWindow",
        ];
        if SKIP.iter().any(|s| *s == class) {
            return false;
        }
    }

    // 3. Window rect == monitor rect.
    let mut wr = RECT::default();
    if GetWindowRect(hwnd, &mut wr).is_err() {
        return false;
    }
    let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !GetMonitorInfoW(mon, &mut mi).as_bool() {
        return false;
    }
    let same = wr.left == mi.rcMonitor.left
        && wr.top == mi.rcMonitor.top
        && wr.right == mi.rcMonitor.right
        && wr.bottom == mi.rcMonitor.bottom;
    if !same {
        return false;
    }

    // 4. Topmost OR no caption OR popup-without-overlapped.
    let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
    let topmost = (ex_style & WS_EX_TOPMOST.0) != 0;
    let no_caption = (style & WS_CAPTION.0) == 0;
    let popup_no_overlap = (style & WS_POPUP.0) != 0 && (style & WS_OVERLAPPED.0) == 0;
    topmost || no_caption || popup_no_overlap
}
