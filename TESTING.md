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
