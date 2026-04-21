# Alt3rSnap v0.1 Release-Gate Manual Test Checklist

Run through every item on Windows before tagging a release. Each checked item should be reproducible on a clean VM.

## Core move/resize
- [ ] Alt + Left-drag moves a normal window (e.g., Notepad).
- [ ] Alt + Right-drag in each of the 8 outer sectors resizes from the correct anchor.
- [ ] Alt + Right-drag in the center sector resizes symmetrically (both opposite edges move equally).
- [ ] Releasing Alt mid-drag — drag continues until mouse-up.
- [ ] Ctrl held at drag start raises the target window.

## Maximized windows
- [ ] Alt + Left-drag on a maximized browser restores it and the cursor remains over the restored rect.

## Multi-monitor + DPI
- [ ] Mixed-DPI multi-monitor: drag a window between 100% and 200% monitors; geometry stays sane.
- [ ] Negative virtual coordinates: monitor arrangement with primary on the right — drag a window to negative X.
- [ ] Monitor hot-plug mid-session: plug/unplug a display; dragging still works.
- [ ] Resolution change during a drag: drag ends cleanly.

## Elevation
- [ ] Running unelevated, try to drag Task Manager → UIPI balloon appears, drag doesn't move it.
- [ ] Tray → Restart elevated → UAC prompts, app relaunches; Task Manager drag now works.
- [ ] Tray menu switches to "Restart normally" when elevated; selecting it returns to normal IL.
- [ ] Elevated app dragging normal windows — no regressions.

## Hosted / modern windows
- [ ] UWP apps (Calculator, Settings) — drag and resize without glitches.
- [ ] Borderless-fullscreen game — pass-through engages, modifier inputs not stolen.
- [ ] Exclusive-fullscreen game — pass-through engages.
- [ ] Layered/transparent window (OBS preview, Everything search) — drag and resize OK.

## Remote / virtualized
- [ ] Remote Desktop client (`mstsc.exe`) — dragging the RDP window locally; the remote session is unaffected.
- [ ] Virtual desktops — drag on desktop 1, switch to desktop 2, drag there. No stale state.

## Taskbar / edge cases
- [ ] Taskbar auto-hide enabled — drag near the hidden edge works.
- [ ] Rapid repeated drag cycles (15+ in quick succession) — no capture leaks.
- [ ] Alt held for 30+ seconds without a drag — no menu-bar activation leaks.
- [ ] Rapid tray "Reload config" presses while a drag is in progress — no crashes.

## Config & tray
- [ ] Opening the tray's "Open config file" creates the default TOML and opens it in the associated editor.
- [ ] Editing `[activation].modifier = "ctrl"` and clicking "Reload config" makes Ctrl-drag work instead of Alt.
- [ ] `exclude.processes = ["mstsc.exe"]` blocks dragging RDP client (while still allowing other apps).
- [ ] Left-clicking the tray icon toggles enabled (icon visibly changes).

## Autostart
- [ ] Toggle "Autostart on logon" → HKCU\...\Run\Alt3rSnap is created; log out/in → app launches at normal IL.
- [ ] Toggle "Autostart as elevated" while elevated → scheduled task created; log out/in → app launches elevated without UAC.

## Logs & crashes
- [ ] `%APPDATA%\Alt3rSnap\logs\alt3rsnap.log.YYYY-MM-DD` contains startup line and drag events in debug mode.
- [ ] Forcing a panic (e.g., inject bad config) writes `%APPDATA%\Alt3rSnap\logs\crash.log`.

## Middle-click (v0.2 M1)

Requires `[behavior].middle_click_action = "toggle_maximize"` in `config.toml`; the v0.1 default is `"none"`.

- [x] Alt + middle-click on a normal window (Notepad, Explorer) toggles maximize/restore.
- [x] Alt + middle-click on a maximized window restores it; releasing the middle button does not reach the application (no paste, no middle-click autoscroll in terminals).
- [+] Alt + middle-click on an excluded window (per `[exclude].processes` or `[[rules]]`) does nothing and the application receives the middle-click as normal.
- [x] Middle-click with `middle_click_action = "none"` passes through.
- [x] Unknown `middle_click_action` value (e.g., `"rollup"`) loads with a tracing warn and Alt + middle-click is a no-op.
- [x] Middle-click in a browser (tab-close behaviour) still works when Alt is **not** held.
- [x] Start Alt + Left-drag and during the drag press the middle button: no stale latch interferes (the drag's `BeginDrag` clears the latch per spec §3.5).
- [ ] After one Alt + middle-click, wait > 1 second, then press the middle button WITHOUT Alt: the click is NOT swallowed (the 500 ms safety timer cleared the latch).

## Resize modes (v0.2 M2)

Set `[resize].center_mode` in `config.toml`; default is `"symmetric"` (already covered under Core move/resize).

- [ ] `center_mode = "bottom_right"` — center-sector Alt + right-drag keeps the **bottom-right corner fixed**; top-left moves by `(-Δx, -Δy)`.
- [ ] `center_mode = "bottom_right"` — the 8 outer sectors still resize from their correct anchor (behaviour unchanged from symmetric).
- [ ] `center_mode = "move"` — center-sector Alt + right-drag **moves** the window instead of resizing.
- [ ] `center_mode = "move"` — the 8 outer sectors still resize normally (only the center sector changes routing).
- [ ] Unknown `center_mode` value (e.g., `"closest_edge"`) loads with a tracing warn and the center sector falls back to `"symmetric"`.
- [ ] Edit `center_mode` in the config file, tray → "Reload config" → new mode takes effect on the next drag without restart.

## Rules TOML (v0.2 M3)

`[[rules]]` entries with `match_process` / `match_class` / `match_title` (each `exact` / `glob` / `regex`), optional `match_traits`, and `action = "exclude"`.

- [ ] `[[rules]] match_process = { glob = "chrome*.exe" }` + `action = "exclude"` blocks dragging Chrome while other apps still drag normally.
- [ ] `[[rules]] match_class = { regex = "^ConsoleWindowClass$" }` blocks dragging legacy `cmd.exe`.
- [x] `[[rules]] match_class = { glob = "*XamlExplorerHost*" }` blocks dragging the **Windows 11 Alt+Tab task switcher**. *(verified 2026-04-21)*
- [x] `[[rules]] match_class = { glob = "*MultitaskingView*" }` blocks dragging the **Windows 10 classic Alt+Tab/Task View** surface.
- [ ] `[[rules]] match_traits = { require_tool = true }` + `action = "exclude"` blocks dragging tool-style windows (e.g., floating palette in Paint.NET).
- [ ] Case-insensitive process match: `match_process = { exact = "NOTEPAD.EXE" }` (upper-case) matches a running `notepad.exe`.
- [ ] Both `[exclude].processes` and `[[rules]]` defined → exclude entries evaluate **before** `[[rules]]` (first match wins per spec §4.2 step 8).
- [ ] Unknown action value, e.g., `action = "include_only"` → that rule is dropped with a tracing warn; other rules still apply; config still loads.
- [ ] Bad regex, e.g., `match_process = { regex = "[" }` → that rule is dropped with a tracing warn; other rules still apply.
- [ ] Matcher-less rule (`[[rules]] action = "exclude"` with no `match_*` fields) → dropped silently; no window gets excluded by it.
- [ ] `[[rules]]` with `action` field missing entirely → config load **fails** with a TOML parse error (`missing field \`action\``) and the previous config stays active (no silent behavioural flip).
- [ ] Edit `[[rules]]` in the config file, tray → "Reload config" → new rules take effect on the next drag-target resolution without restart.
