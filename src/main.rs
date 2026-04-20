#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod adapter;
#[cfg(windows)]
mod dpi;
#[cfg(windows)]
mod fullscreen;
#[cfg(windows)]
mod hook;
#[cfg(windows)]
mod tool_window;
#[cfg(windows)]
mod win_api;

#[cfg(windows)]
fn main() {
    dpi::init();
    if let Err(e) = hook::install() {
        eprintln!("hook install failed: {e}");
        std::process::exit(1);
    }
    fullscreen::install();
    let result = tool_window::init_and_run();
    fullscreen::uninstall();
    hook::uninstall();
    if let Err(e) = result {
        eprintln!("tool window error: {e}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("alt3rsnap is Windows-only; build with `cargo xwin build --target x86_64-pc-windows-msvc`.");
    std::process::exit(1);
}
