//! Hidden tool window that hosts the tray icon and receives hook-forwarded events.

#![cfg(target_os = "windows")]

use std::mem::size_of;
use std::sync::OnceLock;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, KillTimer, PostQuitMessage,
    RegisterClassExW, SetTimer, TranslateMessage, MSG, WM_COMMAND, WM_DESTROY, WM_DISPLAYCHANGE,
    WM_SETTINGCHANGE, WM_TIMER, WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_OVERLAPPED,
};

pub const TOOL_WND_CLASS: PCWSTR = w!("Alt3rSnapToolWnd");

const TIMER_ID_SWALLOW_LATCH: usize = 0x4153_0001; // "AS0001" — avoids collision with tray ids.
/// Timer ID for the debounced monitor-layout refresh (WM_DISPLAYCHANGE / WM_SETTINGCHANGE).
const TIMER_ID_MONITOR_REFRESH: usize = 0x4153_0002; // "AS0002"

static TOOL_HWND: OnceLock<HwndWrap> = OnceLock::new();

#[derive(Copy, Clone)]
struct HwndWrap(pub HWND);
unsafe impl Send for HwndWrap {}
unsafe impl Sync for HwndWrap {}

pub fn hwnd() -> HWND {
    TOOL_HWND.get().copied().map(|w| w.0).unwrap_or_default()
}

pub fn create() -> windows::core::Result<HWND> {
    unsafe {
        let hinstance: HINSTANCE = GetModuleHandleW(None)?.into();
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance,
            lpszClassName: TOOL_WND_CLASS,
            ..Default::default()
        };
        if RegisterClassExW(&wc) == 0 {
            return Err(windows::core::Error::from_win32());
        }
        // Hidden top-level window (no parent).  WS_EX_TOOLWINDOW keeps it out of
        // the taskbar/Alt-Tab list; WS_EX_NOACTIVATE prevents focus theft.
        // Top-level (non-message-only) windows receive broadcast messages like
        // WM_DISPLAYCHANGE and WM_SETTINGCHANGE that HWND_MESSAGE windows miss.
        // Never shown — it exists only as a message target.
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            TOOL_WND_CLASS,
            w!("Alt3rSnap"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            hinstance,
            None,
        )?;
        let _ = TOOL_HWND.set(HwndWrap(hwnd));
        let _ = SetTimer(hwnd, TIMER_ID_SWALLOW_LATCH, 250, None);
        Ok(hwnd)
    }
}

pub fn run_pump() {
    unsafe {
        let mut msg = MSG::default();
        loop {
            let got = GetMessageW(&mut msg, HWND::default(), 0, 0);
            if got.0 <= 0 {
                break;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        m if m == crate::tray::WM_TRAY_CALLBACK => {
            crate::tray::on_tray_message(hwnd, wparam, lparam);
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 as u32) & 0xFFFF;
            crate::tray::on_command(id);
            LRESULT(0)
        }
        WM_TIMER => {
            let id = wparam.0 as usize;
            if id == TIMER_ID_SWALLOW_LATCH {
                let now = windows::Win32::System::SystemInformation::GetTickCount64();
                crate::adapter::swallow_latch().on_timer(now);
                LRESULT(0)
            } else if id == TIMER_ID_MONITOR_REFRESH {
                crate::monitors::flush_pending();
                let _ = KillTimer(hwnd, TIMER_ID_MONITOR_REFRESH);
                LRESULT(0)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        WM_DISPLAYCHANGE => {
            crate::monitors::request_refresh(crate::adapter::drag_active());
            let _ = SetTimer(hwnd, TIMER_ID_MONITOR_REFRESH, 200, None);
            LRESULT(0)
        }
        WM_SETTINGCHANGE => {
            // wParam carries the SPI_* category; only act on SPI_SETWORKAREA.
            const SPI_SETWORKAREA: u32 = 0x002F;
            if wparam.0 as u32 == SPI_SETWORKAREA {
                crate::monitors::request_refresh(crate::adapter::drag_active());
                let _ = SetTimer(hwnd, TIMER_ID_MONITOR_REFRESH, 200, None);
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
