# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Alt3rSnap is a Windows-only utility (Rust successor to AltSnap) that moves/resizes windows on Alt+drag. Target: Windows 10 1809+ / Windows 11, x86_64.

## Build & test

Primary dev flow is **WSL/Linux with cross-compile** via `cargo-xwin`. The `tests/` integration suite is platform-agnostic (engine-only) and runs on any host.

```bash
# Windows cross-build (WSL)
cargo xwin build --release --target x86_64-pc-windows-msvc
# Native Windows build
cargo build --release

# Lint + test (runs on Linux; mirrors CI's lint-and-test job)
cargo fmt --all -- --check
cargo clippy --lib --all-targets -- -D warnings
cargo test --lib --all-features        # lib unit tests
cargo test --test engine_fsm           # run one integration test file
cargo test idle_transitions_to_armed   # run one test by name
```

CI (`.github/workflows/ci.yml`) runs fmt/clippy/test on Linux and a Windows release build. Keep `cargo clippy --lib --all-targets -- -D warnings` clean — warnings fail CI.

Manual release-gate checklist (must pass on Windows before tagging): `TESTING.md`.

## Architecture

The crate is split into a **pure-Rust engine** (unit-testable anywhere) and a **Windows adapter** (thin Win32 glue). This separation is load-bearing — do not import `windows` crate types into `src/engine/**` or leak `HWND`/Win32 types into engine signatures.

```
src/lib.rs        → exposes `config` + `engine` (pure Rust, platform-independent)
src/main.rs       → Windows-only binary; all other src/*.rs modules are #[cfg(windows)]

src/engine/       # PURE — no `windows` crate imports
  mod.rs          # Engine::handle(Event) -> Vec<Action>  (the FSM driver)
  state.rs        # State / Event / Action / DragTarget / WindowId(u64)
  modifiers.rs    # Modifiers bitset + ModMatcher
  policy.rs       # ActivationPolicy (which mods arm / raise)
  rules.rs        # WindowRule: process/class/title/trait patterns → RuleAction
  geometry.rs     # Rect math, 3×3 sector picking, resize anchor math
  config.rs       # EngineConfig (engine-visible view)

src/hook.rs       # WH_MOUSE_LL + WH_KEYBOARD_LL callbacks; owns thread-local ENGINE
src/adapter.rs    # resolves DragTarget from cursor; applies Action list → Win32 calls
src/win_api.rs    # SetWindowPos / GetWindowRect / WindowFromPoint wrappers
src/fullscreen.rs # WinEvent foreground hook → FullscreenFocused/Unfocused events
src/tray.rs       # Shell_NotifyIcon tray menu (enable, reload, restart elevated, autostart)
src/tool_window.rs# hidden message-only HWND that runs the pump
src/config.rs     # FileConfig (TOML on disk) + FileConfig::to_engine_config()
src/config_ops.rs # load/save config file ops used by tray "Reload config"
src/elevate.rs    # `is_elevated()` + ShellExecute "runas" relaunch
src/autostart.rs  # HKCU\...\Run (normal) or Task Scheduler /RL HIGHEST (elevated)
src/dpi.rs        # SetProcessDpiAwarenessContext (Per-Monitor V2)
src/logging.rs    # tracing + tracing-appender to %APPDATA%\Alt3rSnap\logs\
```

### Control flow

1. `hook.rs` low-level hooks translate raw Win32 input into `engine::Event` values.
2. `adapter::resolve_target()` attaches a `DragTarget` (rect, maximized, excluded) to mouse-down events by inspecting the window under the cursor.
3. `Engine::handle(event)` advances the FSM (`Idle → Armed → Moving/Resizing`) and returns a `Vec<Action>` — a pure description of what to do.
4. `adapter::apply_actions()` executes each `Action` (SetWindowPos, capture mouse, raise, swallow event, …) and returns a `swallow` bool that the hook feeds back to Windows.

When adding behavior, prefer extending `Event`/`Action`/`State` in the engine and testing it via `tests/engine_*.rs`. Only touch `adapter.rs`/`win_api.rs` for the Win32 execution of new Actions.

### Config

`FileConfig` (serde TOML at `%APPDATA%\Alt3rSnap\config.toml`) is the user-facing shape; `EngineConfig` is the internal shape. `FileConfig::to_engine_config()` is the only bridge — validation (e.g. `center_mode` must be `"symmetric"` in v0.1) happens there. Tray "Reload config" hot-swaps via `Engine::set_config()`; do not cache engine config elsewhere.

### Tests

- `tests/engine_fsm.rs` — state transitions and action emission
- `tests/engine_geometry.rs` — rect/resize math (uses `proptest`)
- `tests/engine_rules.rs` — rule matching
- `tests/config_parse.rs` — TOML round-trip and validation

All integration tests are pure-Rust and run on Linux. **Do not add Win32-dependent tests to `tests/`** — they'd break the `cargo test --lib --all-features` CI job on Ubuntu.

## Conventions specific to this repo

- `src/lib.rs` is intentionally tiny (exports `config` + `engine` only). Anything Win32-touching lives in `src/main.rs`'s module tree and is `#[cfg(windows)]`.
- `WindowId(u64)` is the engine's opaque window handle. Convert at the adapter boundary with `win_api::hwnd_to_id` / `id_to_hwnd`; never pass `HWND` into the engine.
- The release profile is tuned for size (`opt-level = "z"`, `lto = "fat"`, `panic = "abort"`, `strip = "symbols"`). Don't add dependencies casually.
- Design spec: `docs/superpowers/specs/2026-04-20-alt3rsnap-design.md` is the authoritative roadmap (§8 has the post-MVP parity list).
