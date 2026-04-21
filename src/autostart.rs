#![cfg(target_os = "windows")]

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use tracing::{error, info};
use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ,
};
use windows::Win32::System::Threading::CREATE_NO_WINDOW;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_NAME: &str = "Alt3rSnap";
const SCHED_TASK_NAME: &str = "Alt3rSnap";

fn exe_path() -> PathBuf {
    std::env::current_exe().unwrap_or_default()
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ---- Run key backend (normal IL) ----

pub fn is_normal_enabled() -> bool {
    unsafe {
        let subkey = wide(RUN_KEY);
        let name = wide(RUN_NAME);
        let mut hk = HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_READ,
            &mut hk,
        )
        .is_err()
        {
            return false;
        }
        let mut size: u32 = 0;
        let ok =
            RegQueryValueExW(hk, PCWSTR(name.as_ptr()), None, None, None, Some(&mut size)).is_ok();
        let _ = RegCloseKey(hk);
        ok
    }
}

fn run_key_set(enabled: bool) {
    unsafe {
        let subkey = wide(RUN_KEY);
        let name = wide(RUN_NAME);
        let mut hk = HKEY::default();
        if let Err(e) = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_WRITE,
            &mut hk,
        )
        .ok()
        {
            error!("autostart: open HKCU\\{} failed: {:?}", RUN_KEY, e);
            return;
        }
        if enabled {
            let exe = exe_path();
            let val = format!("\"{}\"", exe.display());
            let val_w = wide(&val);
            let bytes = std::slice::from_raw_parts(val_w.as_ptr() as *const u8, val_w.len() * 2);
            match RegSetValueExW(hk, PCWSTR(name.as_ptr()), 0, REG_SZ, Some(bytes)).ok() {
                Ok(()) => info!("autostart: Run key set to {}", val),
                Err(e) => error!("autostart: RegSetValueExW failed: {:?}", e),
            }
        } else {
            match RegDeleteValueW(hk, PCWSTR(name.as_ptr())).ok() {
                Ok(()) => info!("autostart: Run key cleared"),
                Err(e) => error!("autostart: RegDeleteValueW failed: {:?}", e),
            }
        }
        let _ = RegCloseKey(hk);
    }
}

// ---- Task Scheduler backend (elevated IL) ----

/// Filesystem check for the Alt3rSnap task. Much faster than spawning `schtasks /Query`
/// and avoids flashing a console window every time the tray menu opens.
pub fn is_elevated_enabled() -> bool {
    let Some(windir) = std::env::var_os("WINDIR") else {
        return false;
    };
    let path = Path::new(&windir)
        .join("System32")
        .join("Tasks")
        .join(SCHED_TASK_NAME);
    path.exists()
}

fn sched_task_set(enabled: bool) {
    let exe = exe_path();
    let result = if enabled {
        Command::new("schtasks.exe")
            .creation_flags(CREATE_NO_WINDOW.0)
            .args([
                "/Create",
                "/TN",
                SCHED_TASK_NAME,
                "/SC",
                "ONLOGON",
                "/RL",
                "HIGHEST",
                "/TR",
                &format!("\"{}\"", exe.display()),
                "/F",
            ])
            .output()
    } else {
        Command::new("schtasks.exe")
            .creation_flags(CREATE_NO_WINDOW.0)
            .args(["/Delete", "/TN", SCHED_TASK_NAME, "/F"])
            .output()
    };
    match result {
        Ok(o) if o.status.success() => {
            info!(
                "autostart: schtasks {} ok",
                if enabled { "create" } else { "delete" }
            );
        }
        Ok(o) => {
            error!(
                "autostart: schtasks {} exit={:?} stderr={}",
                if enabled { "create" } else { "delete" },
                o.status.code(),
                String::from_utf8_lossy(&o.stderr)
            );
        }
        Err(e) => error!("autostart: schtasks spawn failed: {:?}", e),
    }
}

// ---- Tray-facing helpers ----

pub fn toggle_normal() {
    let now = is_normal_enabled();
    run_key_set(!now);
    // Only clear the elevated backend if it's actually present — otherwise we'd
    // spawn schtasks.exe on every toggle and block the UI for seconds.
    if !now && is_elevated_enabled() {
        sched_task_set(false);
    }
}

pub fn toggle_elevated() {
    if !crate::elevate::is_elevated() {
        crate::elevate::restart_elevated();
        return;
    }
    let now = is_elevated_enabled();
    sched_task_set(!now);
    if !now && is_normal_enabled() {
        run_key_set(false);
    }
}
