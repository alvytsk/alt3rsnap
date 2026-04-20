# Alt3rSnap — Design Spec

**Date:** 2026-04-20
**Revision:** 2 (incorporates author review of Rev 1)
**Status:** Draft — awaiting author review
**Scope:** A modern, fast Windows utility for moving and resizing windows with a modifier key (Alt by default), written in Rust. Spiritual successor to [AltDrag](https://github.com/stefansundin/altdrag) / [AltSnap](https://github.com/RamonUnch/AltSnap). This spec covers **v0.1 (MVP)** and sketches a roadmap to full AltSnap feature parity.

---

## 1. Goals & non-goals

### 1.1 MVP goals (v0.1)

- Modifier + Left-drag moves any window under the cursor, from any point inside the window. Default modifier is `alt`.
- Modifier + Right-drag resizes the window. Resize anchor is chosen by the cursor's sector within a 3×3 grid (see §4.4); the **center sector resizes symmetrically from the window's center** (not a dead zone).
- Modifier + Left-drag on a **maximized window** restores it and immediately continues the drag, positioning the restored rect under the cursor.
- **Runs non-elevated by default** (manifest: `asInvoker`). A tray action "Restart elevated" re-launches at high IL when the user encounters a window that requires it (admin apps, elevated Task Manager, etc.). This is a deliberate design choice: elevation is situational, not baseline — matching AltSnap's pattern.
- Auto-bypasses fullscreen applications (games, kiosks) with a hardened detection heuristic (not just rect==monitor). Manual override via tray toggle when detection misses.
- Auto-starts on user logon. Default autostart uses the user's `Run` registry key (no elevation prompt). An opt-in "Autostart as elevated" tray checkbox registers a Task Scheduler task with `/RL HIGHEST` instead.
- Richer input model from day one: a keyboard-state bitset + an `ActivationPolicy`, so modifier combos (Ctrl-to-raise, Space-to-suspend-snap, etc.) can be layered on without rewriting the FSM.
- Rule-based exclusions (process / class / title / traits) — internally rich, externally simple in v0.1 config.
- Explicit **focus policy**: do **not** raise the dragged window by default (matches AltSnap). Ctrl during drag raises for that drag.
- Minimal tray UI: enable/disable, autostart toggles, restart elevated, open/reload config, about, exit.
- TOML config at `%APPDATA%\Alt3rSnap\config.toml`.
- Single portable `.exe`, ~1.5–2 MB release build, no DLLs, no installer in MVP.

### 1.2 Non-goals for MVP

- Snap-to-edge / Aero-snap behavior.
- Snap to other windows (magnetic).
- Middle-click maximize/restore.
- Transparency scroll, roll-up, always-on-top toggle.
- Scroll-inactive-window, titlebar actions, keyboard-only drag.
- Window grid / custom snap zones.
- Settings GUI, MSI installer, auto-update, code signing, telemetry.
- Multi-process architecture with a privileged helper (see §8; this is the **post-MVP path** for admin-window support without full app elevation, not "not planned" as in Rev 1).

These are on the parity roadmap (§8) and slot in additively without architectural rework.

### 1.3 Target platform

- **Windows 10 1809+** and **Windows 11**.
- x86_64 only (ARM64 later; not an MVP goal).
- Per-Monitor DPI Aware V2.

---

## 2. Development environment

- **Primary:** WSL2 + `cargo-xwin` targeting `x86_64-pc-windows-msvc`. Built artifact is run/tested under Windows.
- **Fallback:** native Windows build (same source tree) when cross-compile hits an edge case.
- **Toolchain pin:** `rust-toolchain.toml` pins the stable Rust version used in CI.

---

## 3. Architecture overview

Single Rust crate, strict module boundaries. The core (`engine`) is pure Rust with no `windows` crate types in its public signatures; it is unit-testable on any platform. All Win32 types are confined to adapter modules.

### 3.1 File layout

```
alt3rsnap/
├── Cargo.toml
├── rust-toolchain.toml
├── build.rs                 # embeds app.manifest + icon via `embed-resource`
├── app.manifest             # asInvoker, Per-Monitor V2, Win10 1809+ compat GUIDs
├── src/
│   ├── main.rs              # wires modules, installs hooks, runs message pump
│   ├── engine/              # PURE RUST — no `windows` crate imports
│   │   ├── mod.rs           # FSM driver: Engine::handle(Event) -> Vec<Action>
│   │   ├── state.rs         # State enum + transitions
│   │   ├── modifiers.rs     # Modifiers bitset + ModMatcher
│   │   ├── policy.rs        # ActivationPolicy evaluation
│   │   ├── rules.rs         # WindowRule matching (process/class/title/traits)
│   │   ├── geometry.rs      # rect math, 3×3 sector picking, resize math (incl. center-symmetric)
│   │   └── config.rs        # engine-visible config shape
│   ├── hook.rs              # WH_MOUSE_LL + WH_KEYBOARD_LL + WinEvent foreground hook
│   ├── win_api.rs           # SetWindowPos / GetWindowRect / WindowFromPoint / ShowWindow wrappers
│   ├── fullscreen.rs        # hardened fullscreen-focus detection
│   ├── dpi.rs               # Per-Monitor V2 awareness init
│   ├── config.rs            # TOML load/save for %APPDATA%\Alt3rSnap\config.toml
│   ├── tray.rs              # Shell_NotifyIconW + context menu (incl. Restart elevated)
│   ├── elevate.rs           # ShellExecuteW("runas", ...) relaunch
│   └── autostart.rs         # Run-key (normal) and Task Scheduler (elevated) registration
└── tests/
    ├── engine_tests.rs      # FSM tests, runnable on Linux/macOS/Windows
    ├── rules_tests.rs       # WindowRule matching
    └── geometry_tests.rs    # rect/sector/center-symmetric math
```

### 3.2 Data flow

```
OS event (mouse/key/foreground change)
    │
    ▼
[hook.rs]  Win32 callback — translates raw struct into engine::Event
    │
    ▼
[engine]   pure Rust: (state × event × modifiers × policy × rules) → (new_state, Vec<Action>)
    │                                                                          │
    ▼                                                                          ▼
state updated                                               [adapters] execute actions
                                                            (SetWindowPos, ShowWindow, SetCapture, ...)
```

Single-threaded, synchronous. No channels, no async runtime.

### 3.3 Threading model

- Main thread: DPI init → create hidden tool window → install hooks → `GetMessage` pump.
- Low-level hooks run callbacks on the installing thread, so everything executes on the main thread.
- `ENGINE` lives in `thread_local! { RefCell<Engine> }`. Single-threaded access; `RefCell` catches reentrance at runtime.

---

## 4. The engine (pure-Rust core)

### 4.1 States

```
                   +----------+
                   | Disabled |  ← tray toggle
                   +----------+
                        ▲
                        │ ToggleEnable / ToggleDisable
                        ▼
+------+  ArmMatch  +-------+  LeftDown   +--------+
| Idle | ─────────▶ | Armed | ──────────▶ | Moving |
|      | ◀───────── |       | ◀────────── |        |
+------+  UnArm     +-------+  LeftUp     +--------+
   ▲                    │
   │ UnArm              │ RightDown
   │                    ▼
   │               +----------+
   │               | Resizing |
   │               +----------+
   │   RightUp         │
   └───────────────────┘

+-------------+
| PassThrough |  ← entered only from Idle/Armed when a fullscreen app gains focus.
+-------------+    Drags-in-progress are NEVER aborted by fullscreen detection;
                   a `pending_passthrough` flag is set and the transition happens
                   after natural drag end.
```

`ArmMatch` and `UnArm` are derived transitions: the engine recomputes whether the current `Modifiers` match `ActivationPolicy::arm` on every `KeyChange` event. When the result flips, the engine transitions `Idle ↔ Armed` and emits any tray/icon-update actions.

### 4.2 Event and action types

```rust
pub struct WindowId(pub u64);  // opaque wrapper over HWND numeric value
pub struct Point { pub x: i32, pub y: i32 }
pub struct Rect  { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 }

pub struct Modifiers { pub bits: u16 }  // alt, ctrl, shift, win, space, ...

pub enum VirtualKey {
    Alt, Ctrl, Shift, Win, Space,
    Other(u16),     // scancode for future extension
}

pub enum Event {
    KeyChange { vk: VirtualKey, down: bool },
    LeftDown  { cursor: Point },
    LeftUp,
    RightDown { cursor: Point },
    RightUp,
    MouseMove { cursor: Point },
    FullscreenFocused,
    FullscreenUnfocused,
    ToggleEnable,
    ReloadConfig(ConfigView),   // hot reload from tray
}

pub enum Action {
    BeginDrag {
        hwnd: WindowId,
        initial_rect: Rect,
        grab: Point,
        mode: DragMode,
    },
    UpdateDrag { hwnd: WindowId, new_rect: Rect },
    EndDrag { hwnd: WindowId },
    RestoreIfMaximized { hwnd: WindowId, cursor: Point },
        // Adapter restores the window and translates the initial_rect so the
        // cursor stays inside it at the same proportional offset. Emitted
        // before BeginDrag when a drag starts on a maximized window.
    RaiseWindow { hwnd: WindowId },
        // Emitted when ActivationPolicy::raise matches current modifiers
        // (default: Ctrl held at drag start). Default focus policy is NOT to raise.
    CancelMenuActivation,
        // Synthesize 0xFF key to kill Alt-triggered menu-bar activation.
    SwallowEvent,
        // Hook callback returns 1 (suppress click from reaching target app).
    UpdateTrayIcon { enabled: bool },
}

pub enum DragMode {
    Move,
    Resize { anchor: ResizeAnchor },
}

pub enum ResizeAnchor {
    TopLeft, Top, TopRight,
    Left,           Right,
    BottomLeft, Bottom, BottomRight,
    CenterSymmetric,   // see §4.4: opposite edges move symmetrically
}
```

### 4.3 Modifiers and activation policy

`Modifiers` is maintained by the engine as a live bitset updated on every `KeyChange`. `ActivationPolicy` (loaded from config) contains:

```rust
pub struct ModMatcher {
    pub required: Modifiers,   // bits that must be set
    pub forbidden: Modifiers,  // bits that must NOT be set
    pub exact: bool,           // if true, modifiers must equal `required` exactly
}

pub struct ActivationPolicy {
    pub arm: ModMatcher,        // default: { required: ALT, forbidden: WIN, exact: false }
    pub raise: ModMatcher,      // default: { required: CTRL, ..., exact: false }
    pub no_snap: ModMatcher,    // default: { required: SPACE, ..., exact: false }
    // future: snap_to_other, fine_move, etc.
}
```

On every `KeyChange` the engine recomputes `arm.matches(modifiers)`:
- `Idle` + now-matches → transition to `Armed`, emit `UpdateTrayIcon` (optional state indicator).
- `Armed` + no-longer-matches → transition to `Idle`.
- `Moving`/`Resizing` + no-longer-matches → **stay in drag state** (drag completes on mouse-up; modifier state only gates *starting* a drag, not continuing).

Modifier state at `LeftDown`/`RightDown` is captured into the `Moving`/`Resizing` state so policies like `raise` are evaluated against the state at drag start, not during drag.

### 4.4 Resize anchor selection (3×3 grid)

The window is divided into a 3×3 grid with **configurable center fraction** (default: center cell is 1/3 of width and height). Cursor's sector determines the anchor:

```
┌─────────┬─────────┬─────────┐
│  TL     │  T      │   TR    │   (anchors in the OPPOSITE corner/edge stay fixed;
│ anchor= │ anchor= │ anchor= │    labeled by which corner/edge the USER grabs)
│ BR corn │ B edge  │ BL corn │
├─────────┼─────────┼─────────┤
│  L      │  C      │   R     │   Center (C): "CenterSymmetric"
│ anchor= │ anchor= │ anchor= │     → opposite edges move symmetrically from
│ R edge  │ center  │ L edge  │       the window's center (center stays fixed)
├─────────┼─────────┼─────────┤
│  BL     │  B      │   BR    │
│ anchor= │ anchor= │ anchor= │
│ TR corn │ T edge  │ TL corn │
└─────────┴─────────┴─────────┘
```

"Center symmetric" math: if cursor moves by `(Δx, Δy)` from grab point, each opposite edge pair moves by `±Δ` — the left edge moves by `-Δx`, the right edge by `+Δx`; top by `-Δy`, bottom by `+Δy`. The window's center stays fixed. This matches AltSnap's default "resize all directions" behavior.

Additional center modes (configurable later, hard-coded default in MVP):
- `"symmetric"` — the MVP default, as above.
- `"bottom_right"` — center acts as the BR-corner anchor (AltSnap compat option).
- `"closest_edge"` — engine picks the nearest edge and resizes that one.
- `"move"` — center cell falls through to `Moving` instead of `Resizing` (AltSnap's move-in-center option).

Only `"symmetric"` is implemented in MVP; the `CenterMode` enum exists in config so the matrix is future-ready.

### 4.5 Window rules (matching)

All target-window filtering goes through the rule engine:

```rust
pub enum Pattern {
    Exact(String),            // case-insensitive exact match
    Glob(String),             // *, ? wildcards
    Regex(regex::Regex),      // full regex (compiled at config load)
}

pub struct WindowTraitMask {
    pub require_topmost:  Option<bool>,
    pub require_cloaked:  Option<bool>,
    pub require_tool:     Option<bool>,   // WS_EX_TOOLWINDOW
    pub require_owned:    Option<bool>,
    // ... extensible
}

pub struct WindowInfo {
    pub hwnd: WindowId,
    pub process_basename: String,   // lowercase
    pub class_name: String,
    pub title: String,
    pub traits: WindowTraits,
}

pub struct WindowRule {
    pub match_process: Option<Pattern>,
    pub match_class:   Option<Pattern>,
    pub match_title:   Option<Pattern>,
    pub match_traits:  WindowTraitMask,
    pub action:        RuleAction,
}

pub enum RuleAction {
    Exclude,                    // this window is invisible to alt-drag
    IncludeOnly,                // whitelist semantics (future)
    Override(PerWindowConfig),  // per-window config overrides (future)
}
```

Rules are evaluated in config order at `Armed + LeftDown`/`RightDown`. The first rule that matches wins. `WindowInfo` is populated by the adapter (`win_api.rs`) before the engine sees the `LeftDown`/`RightDown` event.

### 4.6 Key transitions (the non-obvious ones)

- **Armed + LeftDown** — adapter resolves target window via `WindowFromPoint → GetAncestor(GA_ROOT)`, populates `WindowInfo`, runs rule engine. If excluded, engine stays in `Armed` and emits no actions. Otherwise:
  - If target is maximized (`IsZoomed`), emit `RestoreIfMaximized` first. Engine then transitions to `Moving` with the restored rect as `initial_rect` (adapter re-queries after restore).
  - Emit `BeginDrag` + optional `RaiseWindow` (if policy matches) + `SwallowEvent`.
- **Moving + MouseMove** — emit `UpdateDrag { new_rect: initial_rect.translated_by(cursor - grab) }`. State unchanged.
- **Moving + LeftUp** — emit `EndDrag` + `CancelMenuActivation`, return to `Armed` or `Idle` (based on current `ActivationPolicy::arm` match).
- **Resizing + MouseMove** — emit `UpdateDrag` with the resize math applied per `ResizeAnchor`.
- **Modifier released mid-drag** — drag continues (see §4.3).
- **PassThrough transitions** — see §4.7.

### 4.7 Fullscreen pass-through (safety-hardened)

`FullscreenFocused` semantics depend on current state:

| Current state | Action on `FullscreenFocused` |
|---|---|
| `Idle`, `Armed` | Transition to `PassThrough`. Emit `UpdateTrayIcon` (optional). |
| `Moving`, `Resizing` | Set `pending_passthrough = true`. **Do not transition.** Continue drag. On natural `EndDrag`, transition to `PassThrough`. |
| `PassThrough` | No-op. |
| `Disabled` | No-op. |

On `FullscreenUnfocused`:
- `PassThrough` → `Idle` (re-evaluate `arm` match against current modifiers; may immediately transition to `Armed`).
- Other states → clear `pending_passthrough` if set (the fullscreen app lost focus before drag ended).

This eliminates the Rev 1 failure mode where a mid-drag foreground flip could strand `SetCapture`.

### 4.8 Testing surface

The engine exposes a deterministic `handle(event) -> Vec<Action>` with fully observable state. Tests are table-driven:

```rust
#[test]
fn alt_right_drag_in_center_sector_resizes_symmetrically() {
    let mut e = Engine::with_config(default_config());
    e.feed(&[
        KeyChange { vk: Alt, down: true },               // → Armed
        RightDown { cursor: center_of(INITIAL_RECT) },    // → Resizing { CenterSymmetric }
    ]);
    let actions = e.handle(MouseMove { cursor: Point { x: 10, y: 0 } });
    assert_eq!(actions, &[UpdateDrag {
        hwnd: HWND,
        new_rect: Rect {
            left:  INITIAL_RECT.left  - 10,
            right: INITIAL_RECT.right + 10,
            top:   INITIAL_RECT.top,
            bottom: INITIAL_RECT.bottom,
        }
    }]);
}
```

---

## 5. Win32 adapter layers

### 5.1 `hook.rs`

Three hooks:

- `WH_MOUSE_LL` — mouse buttons + moves, systemwide.
- `WH_KEYBOARD_LL` — key down/up. Translates to `Event::KeyChange { vk, down }` using a lookup table of VK codes; unknown keys become `VirtualKey::Other(scancode)`.
- `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` — foreground changes; runs the hardened fullscreen classifier in `fullscreen.rs`.

All callbacks run on the main thread. Budget discipline: translate, borrow `ENGINE`, `handle`, apply actions, return. No allocations in the hot path beyond a pre-sized `Vec<Action>` (capacity 4 — the observed maximum is 3: `RestoreIfMaximized` + `BeginDrag` + `SwallowEvent`).

```rust
unsafe extern "system" fn mouse_hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code < 0 { return CallNextHookEx(None, code, w, l); }
    let info = &*(l.0 as *const MSLLHOOKSTRUCT);
    let event = translate_mouse(w, info);
    let actions = ENGINE.with(|e| e.borrow_mut().handle(event));
    let swallow = apply_actions(&actions);
    if swallow { LRESULT(1) } else { CallNextHookEx(None, code, w, l) }
}
```

### 5.2 `win_api.rs`

Thin stateless wrappers:

- `window_under_cursor(pt) -> Option<WindowInfo>` — `WindowFromPoint` + `GetAncestor(GA_ROOT)` + populate process basename, class, title, traits. Skips desktop (`Progman`, `WorkerW`), shell tray, our own tool window, zero-sized windows.
- `get_window_rect(hwnd) -> Rect` — `GetWindowRect` (virtual-screen coords).
- `set_window_rect(hwnd, rect)` — `SetWindowPos(..., SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOSENDCHANGING)`.
- `is_zoomed(hwnd) -> bool` — `IsZoomed`.
- `restore_window(hwnd)` — `ShowWindow(SW_RESTORE)`.
- `raise_window(hwnd)` — `SetForegroundWindow` + `BringWindowToTop`.
- `capture_mouse(tool_hwnd)` / `release_mouse()` — capture on our hidden tool window, not the target.
- `cancel_menu_activation()` — posts a `VK_F18` / 0xFF key to swallow Alt-induced menu-bar activation.
- `process_name_of(hwnd) -> Option<String>` — `GetWindowThreadProcessId` + `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `QueryFullProcessImageNameW` → lowercase basename.

### 5.3 `fullscreen.rs` (hardened heuristic)

Classify the new foreground window as fullscreen iff **all** of:

1. Not cloaked: `DwmGetWindowAttribute(DWMWA_CLOAKED)` returns 0.
2. Window rect equals its monitor's full rect (from `MonitorFromWindow` + `GetMonitorInfoW`, including any taskbar area).
3. Class name is **not** in a known-non-fullscreen set: `Progman`, `WorkerW`, `Shell_TrayWnd`, `Windows.UI.Core.CoreWindow`, `ApplicationFrameWindow`, `XamlExplorerHostIslandWindow`, `Shell_SecondaryTrayWnd`.
4. One of: `WS_EX_TOPMOST` set, **or** no `WS_CAPTION` style, **or** `WS_POPUP` without `WS_OVERLAPPED`.

If classified fullscreen → emit `Event::FullscreenFocused`. On next foreground change to a non-fullscreen window → emit `Event::FullscreenUnfocused`.

Users can manually toggle bypass via tray menu when detection misses (hidden behind an "Advanced" submenu to keep the main menu tight).

### 5.4 `dpi.rs`

One call at startup, before the first `windows`-crate-using function runs:

```rust
SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
```

Manifest also declares Per-Monitor V2 for belt-and-suspenders. All coords thereafter are physical pixels, consistent across monitors.

### 5.5 Admin / UIPI (non-elevated default)

The app is manifested `asInvoker`. Under UIPI, lower-IL processes cannot send window messages (including `SetWindowPos`) to higher-IL windows. When the user alt-drags an elevated window while Alt3rSnap runs unelevated, `SetWindowPos` will silently fail (or return 0 with `GetLastError` = `ERROR_ACCESS_DENIED`).

**Response:**
- Log the failure at `warn` with the target window's process name.
- Rate-limit a tray balloon: "Alt3rSnap can't move elevated windows while running unelevated. Right-click tray → Restart elevated."
- Balloon shows at most once per 60 seconds per session.

This matches AltSnap's situational-elevation pattern: users only pay the elevation cost when they hit its benefit.

### 5.6 `elevate.rs`

`Restart elevated` flow:
1. Build command line from `std::env::current_exe()` + pass-through args + a `--relaunched-elevated` sentinel (so the new process doesn't re-prompt).
2. `ShellExecuteExW` with `lpVerb = "runas"`, `SEE_MASK_NOCLOSEPROCESS`.
3. If user accepts UAC, `ExitProcess(0)` after spawning. If user denies, log and stay running.

The new elevated process:
- Detects `--relaunched-elevated`, skips first-run prompts.
- Replaces tray "Restart elevated" with "Restart normally" (reverse flow, but without UAC — `CreateProcessW` of self is fine since lower-IL-from-higher-IL is trivially allowed via the `explorer.exe` shell-launch trick, or simply via dropping tokens with `CreateProcessAsUserW`; MVP uses the shell trick).

Integrity level is detected at startup via `GetTokenInformation(TokenIntegrityLevel)`.

---

## 6. Config, tray, autostart

### 6.1 `config.rs`

Location: `%APPDATA%\Alt3rSnap\config.toml` (per-user, survives reinstall). Resolved via `SHGetKnownFolderPath(FOLDERID_RoamingAppData)`.

Load at startup. Missing file → write defaults. Parse errors → log, fall back to defaults, one-shot tray balloon. Reload via tray menu "Reload config" re-reads and fires `Engine::handle(ReloadConfig(new_view))`.

MVP schema:

```toml
# Alt3rSnap config — reload via tray menu or restart the app

[activation]
# modifier string is sugar for ActivationPolicy::arm
modifier = "alt"               # "alt" | "ctrl" | "shift" | "win" | combos like "alt+ctrl"
# NOTE: the full ModMatcher form (required/forbidden/exact) will open up in v0.2

[behavior]
enable_move = true
enable_resize = true
raise_on_drag = false          # AltSnap default
restore_maximized_on_move = true

[resize]
center_mode = "symmetric"      # only "symmetric" implemented in v0.1
center_fraction = 0.333        # size of the center sector (0.0..=1.0)

[exclude]
# v0.1 sugar: basenames expand to WindowRule { match_process: Exact(...), action: Exclude }
processes = []                 # e.g. ["mstsc.exe", "vmware-vmx.exe"]
# v0.2 will open up the full WindowRule array form:
# [[rules]]
# match_process = { glob = "game_*.exe" }
# match_class   = "UnityWndClass"
# action = "exclude"
```

Unknown keys are ignored (forward compat).

### 6.2 `tray.rs`

Registered via `Shell_NotifyIconW`. Callback delivered as `WM_APP + 1`.

Right-click menu:

```
✔ Enabled
  ─────────
  ☐ Autostart on logon           ← toggles Run-key entry
  ☐ Autostart as elevated        ← toggles Task Scheduler /RL HIGHEST task
  ─────────
  Restart elevated               ← only shown when running non-elevated
  Restart normally               ← only shown when running elevated
  ─────────
  Open config file
  Reload config
  ─────────
  Advanced ▸                     ← submenu
     ☐ Force fullscreen bypass
     Open log directory
  ─────────
  About…
  Exit
```

- Left-click on icon → toggle Enabled.
- Tray icon has three visual states: enabled (color), disabled (grayscale), passthrough (color with a small overlay badge).
- "Autostart on logon" and "Autostart as elevated" are mutually exclusive — toggling one off the other if needed.
- "About…" shows version, running integrity level, and GitHub URL.

### 6.3 `autostart.rs`

Two back-ends behind a single trait:

```rust
pub trait AutostartBackend {
    fn is_enabled(&self) -> Result<bool>;
    fn enable(&self, exe_path: &Path) -> Result<()>;
    fn disable(&self) -> Result<()>;
}
```

- **`RunKeyAutostart`** — writes `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Alt3rSnap` to the exe path. No elevation required. Runs at user logon with the user's default IL. This is the default.
- **`TaskSchedulerAutostart`** — shells out to `schtasks.exe`:
  ```
  schtasks /Create /TN "Alt3rSnap" /SC ONLOGON /RL HIGHEST /TR "\"<exe>\"" /F
  ```
  Creating this task requires the current process to be elevated; if the user toggles "Autostart as elevated" while non-elevated, the tray first prompts for a one-shot elevated restart to perform the registration.

First-run: no automatic autostart prompt in MVP; user opts in via tray menu explicitly. Sentinel file at `%APPDATA%\Alt3rSnap\first_run_done` marks that the app has started successfully at least once (used for log/crash-report heuristics, not for prompting).

No uninstaller in MVP. Manual cleanup: remove Run-key / scheduled task / `%APPDATA%\Alt3rSnap\` / the exe.

---

## 7. Error handling, testing, build

### 7.1 Error handling

- **Inside hooks:** never panic, never block, never allocate beyond the pre-sized action vec. Win32 API failures logged at `warn`, swallowed.
- **Startup failures (hook install returns null):** `MessageBoxW` + exit 1.
- **Config parse errors:** log, defaults, one-shot balloon.
- **UIPI denial on `SetWindowPos`:** rate-limited balloon suggesting elevated restart (§5.5).
- **Panics:** `std::panic::set_hook` writes thread, payload, backtrace, and last ~50 engine events to `%APPDATA%\Alt3rSnap\crash.log`, then `std::process::abort`. Next launch detects the log, offers to open it.

### 7.2 Logging

`tracing` + `tracing-subscriber` + `tracing-appender` rolling file at `%APPDATA%\Alt3rSnap\alt3rsnap.log` (daily rotation, 3-day retention). Default `info`; `--debug` flag → `debug`. `#![windows_subsystem = "windows"]` — no console window.

### 7.3 Testing strategy

#### 7.3.1 Automated (runs in CI)

| Layer | What's tested | How | Runs on |
|---|---|---|---|
| `engine` FSM | All state transitions incl. fullscreen pass-through safety, pending_passthrough, disable toggle, activation-policy changes | Table-driven `cargo test` | Linux / macOS / Windows |
| `engine::modifiers` / `policy` | ModMatcher evaluation (required / forbidden / exact), policy-change recompute | `cargo test` | Any |
| `engine::rules` | Pattern matching (Exact / Glob / Regex), rule ordering, first-match-wins, trait masks | `cargo test` | Any |
| `engine::geometry` | Rect math, 3×3 sector picking (configurable center fraction), center-symmetric resize, other anchors | `proptest` property tests | Any |
| `config` | Parse fixtures: valid, missing keys, bad types, Unicode paths, unknown keys tolerated | `cargo test` + `include_str!` | Any |
| `fullscreen` classifier | Synthetic `WindowInfo` inputs against the 4-rule heuristic (class names, traits, rect-vs-monitor, cloaked) | `cargo test` | Any |

#### 7.3.2 Manual release gate (Windows, human-executed)

**Required matrix — all must pass before tagging a release:**

- Mixed-DPI multi-monitor: drag a window from a 100% monitor to a 200% monitor and back.
- Negative virtual-screen coordinates: monitor arrangement with primary on the right; drag a window to negative-X space.
- Elevated target window while Alt3rSnap non-elevated: confirm balloon appears, tray "Restart elevated" works, drag succeeds after elevation.
- Elevated Alt3rSnap dragging non-elevated target: verify no regressions.
- UWP window hosted by `ApplicationFrameHost.exe` (Calculator, Settings): move and resize.
- Borderless fullscreen game (any modern title): confirm pass-through engages, modifier inputs not stolen.
- Exclusive fullscreen game: same.
- Maximized window + alt+LeftDrag: confirm restore-then-move with correct cursor-relative positioning.
- Remote Desktop client (`mstsc.exe`): drag RDP window locally; confirm inner RDP session is not affected.
- Taskbar auto-hide enabled: drag near hidden taskbar edge.
- Virtual desktops: drag on desktop 1, switch to desktop 2, drag a window there.
- Layered / transparent windows (e.g., OBS, certain utilities): drag and resize.
- Monitor hot-plug mid-session: plug/unplug a display, verify no stale monitor state.
- Resolution change mid-drag: reduce primary resolution while a drag is active; drag ends cleanly.
- Modifier combos: Ctrl held at drag start raises the window; Space held during drag (future no-snap) is acknowledged but no-op in MVP.

This matrix is tracked in `TESTING.md` at the repo root. Each release tag references a completed checklist (git-committed).

Additional fuzz-style manual checks (not release-blocking, but run before each minor release):
- Rapid repeated drag cycles to stress `SetCapture` lifecycle.
- Alt held for long periods without drag to verify no menu-bar-activation leaks.
- Rapid config reloads via tray while drags are in progress.

### 7.4 Build & CI

- Target: `x86_64-pc-windows-msvc`.
- Primary: `cargo xwin build --release --target x86_64-pc-windows-msvc` from WSL.
- Release profile: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = "symbols"`.
- Size target: ≤2 MB.
- Manifest & icon embedded via `build.rs` + `embed-resource`.
- Key deps: `windows`, `serde`, `toml`, `tracing` + `tracing-subscriber` + `tracing-appender`, `embed-resource`, `regex` (for rule patterns), `proptest` (dev).
- CI:
  - `ubuntu-latest`: all automated tests.
  - `windows-latest`: `cargo build --release`, artifact upload, smoke-launches the exe for 5 seconds to ensure manifest/DPI init succeeds.

### 7.5 Distribution (MVP)

GitHub release with `alt3rsnap.exe` + README + `TESTING.md` release checklist. Unsigned; SmartScreen warning is expected. Code signing deferred.

### 7.6 Security considerations

- No network, no update check, no telemetry in MVP.
- Non-elevated default reduces attack surface vs. Rev 1.
- Config path is a fixed `%APPDATA%` subdirectory — no traversal.
- Rule patterns compile at config load; regex compilation is bounded and errors fall back to defaults.
- Logs and crash reports written under `%APPDATA%` with user ACLs.
- `Restart elevated` is the only IL-changing path and it always goes through the OS's UAC prompt — the user sees every elevation.

---

## 8. Future-parity roadmap

### 8.1 Feature-to-module mapping

| AltSnap feature | Where it slots |
|---|---|
| Middle-click maximize/restore | new `MiddleDown` event, `ToggleMaximize` action |
| Snap to screen edges (half/quarter) | new `engine::snap`; `UpdateDrag` post-processes `new_rect` (with `ActivationPolicy::no_snap` / Space respected) |
| Snap to other windows (magnetic) | `win_api::enumerate_visible_windows` + candidate edges consumed by `engine::snap` |
| Transparency via modifier+scroll | `MouseWheel` event → `SetWindowLayeredAttributes` action |
| Roll-up / always-on-top / minimize / close | new actions; FSM unchanged |
| Per-window rules (beyond exclude) | TOML opens up full `[[rules]]` schema; engine already supports `IncludeOnly`, `Override` |
| Scroll inactive window | hook injects `WM_MOUSEWHEEL` into the window under cursor |
| Keyboard-only drag | hotkey event starts `Moving` without mouse button |
| Titlebar double-click actions | new event + action |
| Window grid / snap zones | geometry extension of screen-edge snap |
| Other center resize modes (`bottom_right`, `closest_edge`, `move`) | already plumbed in config + `ResizeAnchor`; add geometry for each |
| Settings GUI | new adapter module; emits `Event::ReloadConfig` |

### 8.2 Phase plan (indicative)

- **v0.1** — MVP as specified.
- **v0.2** — middle-click maximize/restore, edge snap, richer per-window rules (full TOML schema), additional center resize modes.
- **v0.3** — transparency scroll, roll-up, always-on-top, modifier-combo drag enhancements (Ctrl/Shift/Space live during drag).
- **v0.4** — scroll-inactive-window, titlebar actions, keyboard-only drag.
- **v0.5** — window grid / custom snap zones, magnetic snap.
- **v0.6 (possible)** — **medium-IL main + elevated helper IPC**. Main app stays `asInvoker`; an on-demand elevated helper service (spawned via UAC once, persists for session) handles `SetWindowPos` on elevated targets via a named pipe. Eliminates the "Restart elevated" friction for heavy admin-window users. Requires careful IPC security (ACL on the pipe, message validation, no arbitrary HWND operations beyond a defined action set).
- **v1.0** — settings GUI (`egui` vs native Win32 vs WinUI3 — decision deferred), MSI installer, code signing.

### 8.3 Deferred architectural move: privileged helper

Rev 1 ruled out multi-process IPC entirely. Rev 2 walks that back: **it is the right long-term answer for "admin windows out of the box without always-elevated app"**, and the current engine/adapter shape is a clean base for it. The adapter's window-manipulation surface (`set_window_rect`, `restore_window`, `raise_window`) is exactly the set of calls the elevated helper would need to proxy.

MVP deliberately does **not** scaffold IPC — the engine/adapter shape does not need to anticipate it. When the helper ships in v0.6, the changes are:

1. New crate `alt3rsnap-helper` (tiny, ~300 LOC).
2. Adapter grows a `WinApi` trait with two impls: `LocalWinApi` (direct Win32 calls) and `PipeWinApi` (serializes actions to the helper).
3. Main app detects UIPI failures and, if helper is available, retries via the pipe. If not, it offers to install the helper (one-time UAC).
4. Helper registers itself as a scheduled task at install time and is launched on logon if previously installed.

Security model for the helper: the pipe ACL restricts to the specific user SID; the helper accepts only a small enumerated set of `Action`s (no arbitrary HWND injection); every action is logged; the helper drops its token to the lowest privileges that still allow cross-IL `SetWindowPos`.

### 8.4 What would force further architectural change

- Plugin system for third-party rules/actions.
- Remote/centralized config distribution.

Neither is on the roadmap.

---

## 9. Risks & open questions

- **`cargo-xwin` first-time setup.** Downloads MSVC headers/CRT from Microsoft — reliable but slow; may need troubleshooting on a fresh machine. Fallback is native Windows build.
- **SmartScreen friction.** Unsigned binary → users click through "more info → run anyway." Acceptable for MVP; code signing is v1.0.
- **UWP window quirks.** `ApplicationFrameHost.exe` windows can misbehave under `SetWindowPos` (XAML islands, resize snapping). Tested as part of the release-gate matrix; bugs handled as reported.
- **Menu-activation suppression.** Synthesized key is a known-working AltSnap-inherited hack. Stable in practice; no replacement known.
- **Fullscreen detection false negatives.** The hardened heuristic is best-effort; some custom-chromed full-screen tools may slip through. Advanced tray menu offers a manual bypass toggle.
- **Modifier combos at non-physical layouts.** Users with remapped keyboards (AutoHotkey, kanata, etc.) may see synthetic `KeyChange` events with slightly different timing. Engine's stateless handling should cope; verify in testing.

---

## 10. Glossary

- **`WH_MOUSE_LL` / `WH_KEYBOARD_LL`** — low-level input hooks. Systemwide; installed from any thread with a message pump.
- **UIPI** — User Interface Privilege Isolation. Blocks lower-IL processes from sending window messages to higher-IL ones.
- **Per-Monitor V2 DPI awareness** — modern Windows DPI model; each window has its own per-monitor DPI context.
- **`asInvoker`** — manifest requested execution level matching the parent process token. Default for non-elevated apps.
- **`/RL HIGHEST` (Task Scheduler)** — runs a task at the highest privileges available to the account, bypassing UAC on login for an elevated exe.
- **`DWMWA_CLOAKED`** — DWM attribute indicating a window is hidden from the compositor (e.g., minimized UWP on another virtual desktop). Used in fullscreen classifier to avoid false positives.
- **`IsZoomed`** — Win32 predicate returning true for maximized windows.
