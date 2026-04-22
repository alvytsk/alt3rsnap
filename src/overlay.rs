//! Snap preview overlay — hidden layered top-level popup.
//!
//! Lazy-initialised on first `show()`; lives for process lifetime afterwards.
//! Whole-window alpha via `SetLayeredWindowAttributes(LWA_ALPHA)` — the plan commits
//! to this path over `UpdateLayeredWindow` for v0.2 simplicity (spec §3.1).

#![cfg(target_os = "windows")]

use alt3rsnap::engine::geometry::Rect;
use std::sync::{Mutex, OnceLock};
use windows::core::w;
use windows::Win32::Foundation::{BOOL, COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Dwm::DwmGetColorizationColor;
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, FrameRect, PAINTSTRUCT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, RegisterClassExW, SetLayeredWindowAttributes, SetWindowPos,
    ShowWindow, LWA_ALPHA, SWP_NOACTIVATE, SWP_NOREDRAW, SWP_NOZORDER, SW_HIDE, SW_SHOWNA,
    WM_PAINT, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};

/// The overlay's HWND after lazy-create. Wrapped because `HWND` isn't `Send` by default.
static HWND_CACHE: OnceLock<Mutex<Option<HwndBox>>> = OnceLock::new();
static OPACITY: OnceLock<Mutex<u8>> = OnceLock::new();

struct HwndBox(pub HWND);
// SAFETY: HWND is Send in practice — the handle is just a pointer and we guard access via Mutex.
unsafe impl Send for HwndBox {}

fn hwnd_slot() -> &'static Mutex<Option<HwndBox>> {
    HWND_CACHE.get_or_init(|| Mutex::new(None))
}

fn opacity_slot() -> &'static Mutex<u8> {
    OPACITY.get_or_init(|| Mutex::new(0x99))
}

/// Set the overlay's whole-window alpha (0x00..=0xFF). Applied on the next `show()`.
pub fn set_opacity(a: u8) {
    *opacity_slot().lock().unwrap() = a;
}

/// Register the overlay window class. Call once at startup after getting the HINSTANCE.
/// Idempotent — `RegisterClassExW` returns 0 on duplicate, which we ignore.
pub fn register_class(hinstance: HINSTANCE) {
    unsafe {
        let cls = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance,
            lpszClassName: w!("Alt3rSnapOverlay"),
            ..Default::default()
        };
        RegisterClassExW(&cls);
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let fill = CreateSolidBrush(accent_color());
            FillRect(hdc, &ps.rcPaint, fill);
            let _ = DeleteObject(fill);
            let frame = CreateSolidBrush(accent_color());
            FrameRect(hdc, &ps.rcPaint, frame);
            let _ = DeleteObject(frame);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

/// Best-effort DWM-derived accent color; fallback to RGB(0, 120, 215).
fn accent_color() -> COLORREF {
    let mut cr: u32 = 0;
    let mut opaque: BOOL = BOOL(0);
    unsafe {
        if DwmGetColorizationColor(&mut cr, &mut opaque).is_ok() {
            // cr is 0xAARRGGBB; convert to COLORREF (0x00BBGGRR).
            let r = ((cr >> 16) & 0xFF) as u8;
            let g = ((cr >> 8) & 0xFF) as u8;
            let b = (cr & 0xFF) as u8;
            return COLORREF(((b as u32) << 16) | ((g as u32) << 8) | (r as u32));
        }
    }
    COLORREF(0x00D77800) // RGB(0, 120, 215) in COLORREF byte order
}

fn ensure_created(hinstance: HINSTANCE) -> HWND {
    let mut slot = hwnd_slot().lock().unwrap();
    if let Some(h) = slot.as_ref() {
        return h.0;
    }
    unsafe {
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW
                | WS_EX_TOPMOST,
            w!("Alt3rSnapOverlay"),
            w!("Alt3rSnap"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            hinstance,
            None,
        )
        .unwrap_or_default();
        let a = *opacity_slot().lock().unwrap();
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), a, LWA_ALPHA);
        *slot = Some(HwndBox(hwnd));
        hwnd
    }
}

/// Show the preview at the given screen rect. Lazy-creates on first call.
pub fn show(hinstance: HINSTANCE, rect: Rect) {
    let hwnd = ensure_created(hinstance);
    if hwnd.0.is_null() {
        return;
    }
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            None,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOREDRAW,
        );
        let _ = ShowWindow(hwnd, SW_SHOWNA);
    }
}

/// Hide the preview. No-op if already hidden or never created.
pub fn hide() {
    let slot = hwnd_slot().lock().unwrap();
    if let Some(h) = slot.as_ref() {
        unsafe {
            let _ = ShowWindow(h.0, SW_HIDE);
        }
    }
}
