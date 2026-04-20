#![cfg(target_os = "windows")]

use std::path::PathBuf;
use std::process::Command;

use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ,
};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_NAME: &str = "Alt3rSnap";
const SCHED_TASK_NAME: &str = "Alt3rSnap";

fn exe_path() -> PathBuf { std::env::current_exe().unwrap_or_default() }

// ---- Run key backend (normal IL) ----

fn run_key_enabled() -> bool {
    unsafe {
        let subkey: Vec<u16> = RUN_KEY.encode_utf16().chain(std::iter::once(0)).collect();
        let name:   Vec<u16> = RUN_NAME.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hk: HKEY = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, windows::core::PCWSTR(subkey.as_ptr()), 0, KEY_READ, &mut hk).is_err() {
            return false;
        }
        let mut size: u32 = 0;
        let ok = RegQueryValueExW(hk, windows::core::PCWSTR(name.as_ptr()), None, None, None, Some(&mut size)).is_ok();
        let _ = RegCloseKey(hk);
        ok
    }
}

fn run_key_set(enabled: bool) {
    unsafe {
        let subkey: Vec<u16> = RUN_KEY.encode_utf16().chain(std::iter::once(0)).collect();
        let name:   Vec<u16> = RUN_NAME.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hk: HKEY = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, windows::core::PCWSTR(subkey.as_ptr()), 0, KEY_WRITE, &mut hk).is_err() {
            return;
        }
        if enabled {
            let exe = exe_path();
            let val = format!("\"{}\"", exe.to_string_lossy());
            let val_w: Vec<u16> = val.encode_utf16().chain(std::iter::once(0)).collect();
            let bytes = std::slice::from_raw_parts(val_w.as_ptr() as *const u8, val_w.len() * 2);
            let _ = RegSetValueExW(hk, windows::core::PCWSTR(name.as_ptr()), 0, REG_SZ, Some(bytes));
        } else {
            let _ = RegDeleteValueW(hk, windows::core::PCWSTR(name.as_ptr()));
        }
        let _ = RegCloseKey(hk);
    }
}

// ---- Task Scheduler backend (elevated IL) ----

fn sched_task_enabled() -> bool {
    Command::new("schtasks.exe")
        .args(["/Query", "/TN", SCHED_TASK_NAME])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn sched_task_set(enabled: bool) {
    let exe = exe_path();
    if enabled {
        let _ = Command::new("schtasks.exe")
            .args([
                "/Create",
                "/TN", SCHED_TASK_NAME,
                "/SC", "ONLOGON",
                "/RL", "HIGHEST",
                "/TR", &format!("\"{}\"", exe.to_string_lossy()),
                "/F",
            ])
            .output();
    } else {
        let _ = Command::new("schtasks.exe")
            .args(["/Delete", "/TN", SCHED_TASK_NAME, "/F"])
            .output();
    }
}

// ---- Tray-facing helpers ----

pub fn toggle_normal() {
    let now = run_key_enabled();
    run_key_set(!now);
    if !now { sched_task_set(false); }
}

pub fn toggle_elevated() {
    if !crate::elevate::is_elevated() {
        crate::elevate::restart_elevated();
        return;
    }
    let now = sched_task_enabled();
    sched_task_set(!now);
    if !now { run_key_set(false); }
}

pub fn is_normal_enabled() -> bool { run_key_enabled() }
pub fn is_elevated_enabled() -> bool { sched_task_enabled() }
