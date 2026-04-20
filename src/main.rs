#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod dpi;
#[cfg(windows)]
mod tool_window;
#[cfg(windows)]
mod win_api;

#[cfg(windows)]
fn main() {
    dpi::init();
    if let Err(e) = tool_window::init_and_run() {
        eprintln!("tool window error: {e}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("alt3rsnap is Windows-only; build with `cargo xwin build --target x86_64-pc-windows-msvc`.");
    std::process::exit(1);
}
