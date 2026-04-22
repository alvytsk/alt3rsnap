//! Adapter-ordering + overlay-idempotence contract tests.
//!
//! These run on Linux via the pure-Rust `RecordingWinApi` mock from the
//! `win_api_trait` module. They validate the plan's §2.10 and §2.11 invariants
//! independently of the production adapter's Win32 code path.

use alt3rsnap::engine::geometry::Rect;
use alt3rsnap::engine::state::{Action, WindowId};
use alt3rsnap::win_api_trait::{Call, RecordingWinApi, WinApi};

/// A minimal local `apply` that mirrors the subset of adapter dispatch logic
/// we care about for ordering/idempotence. Production `apply_actions` calls
/// Win32 wrappers directly; the trait is the test seam.
fn apply(api: &mut dyn WinApi, acts: &[Action]) {
    for a in acts {
        match a {
            Action::ShowSnapPreview { rect } => api.overlay_show(*rect),
            Action::HideSnapPreview => api.overlay_hide(),
            Action::ApplySnapRect { hwnd, rect } => {
                // On failure the production path would emit DragAborted; for the
                // ordering test we assume success and let the mock record the call.
                let _ = api.set_window_rect(*hwnd, *rect);
            }
            Action::UpdateDrag { hwnd, new_rect } => {
                let _ = api.set_window_rect(*hwnd, *new_rect);
            }
            _ => {}
        }
    }
}

#[test]
fn apply_snap_rect_is_terminal_for_geometry_in_a_batch() {
    // Per plan §2.11: after `ApplySnapRect`, no later action in the same batch
    // may call `set_window_rect`. Assert the invariant on a representative batch.
    let mut api = RecordingWinApi::new();
    let r = Rect {
        left: 0,
        top: 0,
        right: 960,
        bottom: 540,
    };
    let batch = vec![
        Action::HideSnapPreview,
        Action::ApplySnapRect {
            hwnd: WindowId(1),
            rect: r,
        },
        // EndDrag / CancelMenuActivation would normally follow — those are not
        // geometry-mutating, so the invariant trivially holds.
    ];
    apply(&mut api, &batch);

    // Only ONE set_window_rect, from ApplySnapRect.
    let set_count = api
        .calls
        .iter()
        .filter(|c| matches!(c, Call::SetWindowRect(..)))
        .count();
    assert_eq!(set_count, 1);

    // Assert the LAST SetWindowRect carries the engaged rect.
    let last_set = api
        .calls
        .iter()
        .rev()
        .find(|c| matches!(c, Call::SetWindowRect(..)));
    assert!(matches!(
        last_set,
        Some(Call::SetWindowRect(WindowId(1), rr)) if *rr == r
    ));
}

#[test]
fn overlay_show_is_idempotent_on_same_rect() {
    let mut api = RecordingWinApi::new();
    let r = Rect {
        left: 10,
        top: 20,
        right: 30,
        bottom: 40,
    };
    api.overlay_show(r);
    api.overlay_show(r);
    let shows = api
        .calls
        .iter()
        .filter(|c| matches!(c, Call::OverlayShow(_)))
        .count();
    assert_eq!(
        shows, 1,
        "duplicate show at same rect should dedupe to a single call"
    );
}

#[test]
fn overlay_hide_is_noop_when_already_hidden() {
    let mut api = RecordingWinApi::new();
    api.overlay_hide();
    api.overlay_hide();
    let hides = api
        .calls
        .iter()
        .filter(|c| matches!(c, Call::OverlayHide))
        .count();
    assert_eq!(hides, 0, "hide on already-hidden should not record");
}

#[test]
fn overlay_show_to_new_rect_after_same_rect_still_records() {
    // Not strictly from the plan, but locks the follow-on behavior: once the
    // overlay is visible, a DIFFERENT rect must still be recorded (the overlay
    // needs repositioning for zone-switch per §2.4).
    let mut api = RecordingWinApi::new();
    let r1 = Rect {
        left: 0,
        top: 0,
        right: 100,
        bottom: 100,
    };
    let r2 = Rect {
        left: 200,
        top: 0,
        right: 400,
        bottom: 100,
    };
    api.overlay_show(r1);
    api.overlay_show(r2);
    let shows: Vec<&Call> = api
        .calls
        .iter()
        .filter(|c| matches!(c, Call::OverlayShow(_)))
        .collect();
    assert_eq!(shows.len(), 2);
}

#[test]
fn overlay_hide_then_show_records_both() {
    let mut api = RecordingWinApi::new();
    let r = Rect {
        left: 0,
        top: 0,
        right: 100,
        bottom: 100,
    };
    api.overlay_show(r);
    api.overlay_hide();
    assert_eq!(api.calls, vec![Call::OverlayShow(r), Call::OverlayHide]);
}

#[test]
fn set_window_rect_failure_returns_err() {
    // Verify the mock's fail_next_set_window_rect flag works as documented.
    let mut api = RecordingWinApi::new();
    api.fail_next_set_window_rect = true;
    let r = Rect {
        left: 0,
        top: 0,
        right: 1,
        bottom: 1,
    };
    let res = api.set_window_rect(WindowId(1), r);
    assert!(res.is_err());
    // Next call succeeds again.
    let res2 = api.set_window_rect(WindowId(1), r);
    assert!(res2.is_ok());
}
