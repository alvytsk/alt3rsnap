use alt3rsnap::engine::config::EngineConfig;
use alt3rsnap::engine::state::{Event, State, VirtualKey};
use alt3rsnap::engine::Engine;

#[test]
fn idle_transitions_to_armed_on_alt_down() {
    let mut e = Engine::new(EngineConfig::default());
    assert!(matches!(e.state(), State::Idle));
    let _ = e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    assert!(matches!(e.state(), State::Armed));
}

#[test]
fn armed_transitions_back_to_idle_on_alt_up() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: false,
    });
    assert!(matches!(e.state(), State::Idle));
}

#[test]
fn alt_plus_win_does_not_arm_due_to_forbidden_modifier() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::KeyChange {
        vk: VirtualKey::Win,
        down: true,
    });
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    assert!(matches!(e.state(), State::Idle));
}

use alt3rsnap::engine::geometry::{Point, Rect};
use alt3rsnap::engine::state::{Action, DragMode, DragTarget, WindowId};

fn default_target() -> DragTarget {
    DragTarget {
        hwnd: WindowId(1),
        initial_rect: Rect {
            left: 100,
            top: 100,
            right: 300,
            bottom: 300,
        },
        is_maximized: false,
        exclude: false,
    }
}

#[test]
fn armed_plus_left_down_begins_move_drag() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    let actions = e.handle(Event::LeftDown {
        cursor: Point { x: 150, y: 150 },
        target: Some(default_target()),
    });

    assert!(actions.iter().any(|a| matches!(
        a,
        Action::BeginDrag {
            mode: DragMode::Move,
            ..
        }
    )));
    assert!(actions.contains(&Action::SwallowEvent));
    assert!(matches!(e.state(), State::Moving { .. }));
}

#[test]
fn idle_plus_left_down_emits_no_actions() {
    // User left-clicked without the modifier; engine ignores.
    let mut e = Engine::new(EngineConfig::default());
    let actions = e.handle(Event::LeftDown {
        cursor: Point { x: 150, y: 150 },
        target: Some(default_target()),
    });
    assert!(actions.is_empty());
    assert!(matches!(e.state(), State::Idle));
}

#[test]
fn armed_plus_left_down_on_excluded_window_does_not_begin_drag() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    let mut t = default_target();
    t.exclude = true;
    let actions = e.handle(Event::LeftDown {
        cursor: Point { x: 150, y: 150 },
        target: Some(t),
    });
    assert!(actions.is_empty());
    assert!(matches!(e.state(), State::Armed));
}

#[test]
fn mouse_move_during_moving_emits_update_drag() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    e.handle(Event::LeftDown {
        cursor: Point { x: 150, y: 150 },
        target: Some(default_target()),
    });
    let actions = e.handle(Event::MouseMove {
        cursor: Point { x: 170, y: 145 },
    });
    assert_eq!(
        actions,
        &[Action::UpdateDrag {
            hwnd: WindowId(1),
            new_rect: Rect {
                left: 120,
                top: 95,
                right: 320,
                bottom: 295
            },
        }]
    );
}

#[test]
fn left_up_ends_drag_and_returns_to_armed() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    e.handle(Event::LeftDown {
        cursor: Point { x: 150, y: 150 },
        target: Some(default_target()),
    });
    let actions = e.handle(Event::LeftUp);
    assert!(actions.contains(&Action::EndDrag { hwnd: WindowId(1) }));
    assert!(actions.contains(&Action::CancelMenuActivation));
    assert!(matches!(e.state(), State::Armed));
}

#[test]
fn left_up_returns_to_idle_if_modifier_released_during_drag() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    e.handle(Event::LeftDown {
        cursor: Point { x: 150, y: 150 },
        target: Some(default_target()),
    });
    // Release Alt mid-drag — drag continues; state stays Moving.
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: false,
    });
    assert!(matches!(e.state(), State::Moving { .. }));
    // LeftUp — now arm policy no longer matches, so we go Idle.
    e.handle(Event::LeftUp);
    assert!(matches!(e.state(), State::Idle));
}

#[test]
fn armed_plus_right_down_in_top_left_picks_top_left_anchor() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    let actions = e.handle(Event::RightDown {
        cursor: Point { x: 110, y: 110 }, // top-left of [100..300]
        target: Some(default_target()),
    });
    assert!(actions.iter().any(|a| matches!(
        a,
        Action::BeginDrag {
            mode: DragMode::Resize {
                anchor: alt3rsnap::engine::geometry::ResizeAnchor::TopLeft,
            },
            ..
        }
    )));
    assert!(matches!(e.state(), State::Resizing { .. }));
}

#[test]
fn armed_plus_right_down_in_center_picks_center_symmetric_anchor() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    let actions = e.handle(Event::RightDown {
        cursor: Point { x: 200, y: 200 }, // center of [100..300]
        target: Some(default_target()),
    });
    assert!(actions.iter().any(|a| matches!(
        a,
        Action::BeginDrag {
            mode: DragMode::Resize {
                anchor: alt3rsnap::engine::geometry::ResizeAnchor::CenterSymmetric,
            },
            ..
        }
    )));
}

#[test]
fn right_up_ends_resize_drag() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    e.handle(Event::RightDown {
        cursor: Point { x: 110, y: 110 },
        target: Some(default_target()),
    });
    e.handle(Event::RightUp);
    assert!(matches!(e.state(), State::Armed));
}

#[test]
fn toggle_enable_from_idle_goes_to_disabled() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::ToggleEnable);
    assert!(matches!(e.state(), State::Disabled));
    e.handle(Event::ToggleEnable);
    // After re-enable, re-evaluate modifier state; no modifiers held → Idle.
    assert!(matches!(e.state(), State::Idle));
}

#[test]
fn fullscreen_focused_from_idle_enters_passthrough() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::FullscreenFocused);
    assert!(matches!(e.state(), State::PassThrough));
    e.handle(Event::FullscreenUnfocused);
    assert!(matches!(e.state(), State::Idle));
}

#[test]
fn fullscreen_focused_during_move_does_not_abort_drag() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    e.handle(Event::LeftDown {
        cursor: Point { x: 150, y: 150 },
        target: Some(default_target()),
    });
    e.handle(Event::FullscreenFocused);
    assert!(matches!(
        e.state(),
        State::Moving {
            pending_passthrough: true,
            ..
        }
    ));
    // Drag still works:
    let actions = e.handle(Event::MouseMove {
        cursor: Point { x: 160, y: 150 },
    });
    assert!(matches!(actions.first(), Some(Action::UpdateDrag { .. })));
    // On LeftUp, transition to PassThrough (pending_passthrough was set).
    e.handle(Event::LeftUp);
    assert!(matches!(e.state(), State::PassThrough));
}

#[test]
fn passthrough_ignores_mouse_events() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::FullscreenFocused);
    let actions = e.handle(Event::LeftDown {
        cursor: Point { x: 1, y: 1 },
        target: Some(default_target()),
    });
    assert!(actions.is_empty());
}

#[test]
fn set_config_disabling_stops_engine() {
    let mut e = Engine::new(EngineConfig::default());
    let cfg = EngineConfig {
        enabled: false,
        ..Default::default()
    };
    let acts = e.set_config(cfg);
    assert!(matches!(e.state(), State::Disabled));
    assert!(acts.contains(&Action::UpdateTrayIcon { enabled: false }));
}

#[test]
fn toggle_maximize_action_exists_and_carries_window_id() {
    let a = Action::ToggleMaximize { hwnd: WindowId(42) };
    if let Action::ToggleMaximize { hwnd } = a {
        assert_eq!(hwnd, WindowId(42));
    } else {
        panic!("expected ToggleMaximize");
    }
}

#[test]
fn middle_down_event_exists_and_carries_cursor_and_target() {
    let ev = Event::MiddleDown {
        cursor: Point { x: 10, y: 20 },
        target: Some(default_target()),
    };
    match ev {
        Event::MiddleDown { cursor, target } => {
            assert_eq!(cursor.x, 10);
            assert_eq!(cursor.y, 20);
            assert!(target.is_some());
        }
        _ => panic!("expected MiddleDown"),
    }
}

use alt3rsnap::engine::config::MiddleClickAction;

#[test]
fn engine_config_default_middle_click_action_is_none() {
    let cfg = EngineConfig::default();
    assert_eq!(cfg.middle_click_action, MiddleClickAction::None);
}

#[test]
fn middle_click_action_variants() {
    let _ = MiddleClickAction::None;
    let _ = MiddleClickAction::ToggleMaximize;
}

fn armed_engine_with_toggle_maximize() -> Engine {
    let cfg = EngineConfig {
        middle_click_action: MiddleClickAction::ToggleMaximize,
        ..Default::default()
    };
    let mut e = Engine::new(cfg);
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    assert!(matches!(e.state(), State::Armed));
    e
}

#[test]
fn armed_plus_middle_down_on_normal_window_emits_toggle_and_swallow() {
    let mut e = armed_engine_with_toggle_maximize();
    let actions = e.handle(Event::MiddleDown {
        cursor: Point { x: 150, y: 150 },
        target: Some(default_target()),
    });
    assert_eq!(
        actions,
        vec![
            Action::ToggleMaximize { hwnd: WindowId(1) },
            Action::SwallowEvent,
        ]
    );
    assert!(matches!(e.state(), State::Armed));
}

#[test]
fn armed_plus_middle_down_on_excluded_window_is_noop() {
    let mut e = armed_engine_with_toggle_maximize();
    let mut target = default_target();
    target.exclude = true;
    let actions = e.handle(Event::MiddleDown {
        cursor: Point { x: 150, y: 150 },
        target: Some(target),
    });
    assert!(actions.is_empty());
    assert!(matches!(e.state(), State::Armed));
}

#[test]
fn armed_plus_middle_down_with_action_none_is_noop() {
    // Default EngineConfig has middle_click_action = None.
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    assert!(matches!(e.state(), State::Armed));
    let actions = e.handle(Event::MiddleDown {
        cursor: Point { x: 150, y: 150 },
        target: Some(default_target()),
    });
    assert!(actions.is_empty());
    assert!(matches!(e.state(), State::Armed));
}

#[test]
fn idle_plus_middle_down_is_noop_even_with_toggle_maximize() {
    let cfg = EngineConfig {
        middle_click_action: MiddleClickAction::ToggleMaximize,
        ..Default::default()
    };
    let mut e = Engine::new(cfg);
    // No Alt press → Idle.
    assert!(matches!(e.state(), State::Idle));
    let actions = e.handle(Event::MiddleDown {
        cursor: Point { x: 150, y: 150 },
        target: Some(default_target()),
    });
    assert!(actions.is_empty());
}

#[test]
fn armed_plus_middle_down_with_no_target_is_noop() {
    let mut e = armed_engine_with_toggle_maximize();
    let actions = e.handle(Event::MiddleDown {
        cursor: Point { x: 150, y: 150 },
        target: None,
    });
    assert!(actions.is_empty());
}

use alt3rsnap::engine::config::CenterMode;
use alt3rsnap::engine::state::DragOrigin;

#[test]
fn engine_config_default_center_mode_is_symmetric() {
    assert_eq!(EngineConfig::default().center_mode, CenterMode::Symmetric);
}

#[test]
fn center_mode_variants_exist() {
    let _ = CenterMode::Symmetric;
    let _ = CenterMode::BottomRight;
    let _ = CenterMode::Move;
}

#[test]
fn drag_origin_variants_exist() {
    let _ = DragOrigin::PrimaryButton;
    let _ = DragOrigin::CenterMoveMode;
}

#[test]
fn state_moving_has_drag_origin_primary_button_on_left_down() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    e.handle(Event::LeftDown {
        cursor: Point { x: 150, y: 150 },
        target: Some(default_target()),
    });
    assert!(matches!(
        e.state(),
        State::Moving {
            drag_origin: DragOrigin::PrimaryButton,
            ..
        }
    ));
}

#[test]
fn right_down_center_sector_with_bottom_right_mode_uses_top_left_anchor() {
    let cfg = EngineConfig {
        center_mode: CenterMode::BottomRight,
        ..Default::default()
    };
    let mut e = Engine::new(cfg);
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    let actions = e.handle(Event::RightDown {
        cursor: Point { x: 200, y: 200 }, // center of [100..300]
        target: Some(default_target()),
    });
    assert!(actions.iter().any(|a| matches!(
        a,
        Action::BeginDrag {
            mode: DragMode::Resize {
                anchor: alt3rsnap::engine::geometry::ResizeAnchor::TopLeft,
            },
            ..
        }
    )));
    assert!(matches!(e.state(), State::Resizing { .. }));
}

#[test]
fn right_down_center_sector_with_move_mode_enters_moving() {
    let cfg = EngineConfig {
        center_mode: CenterMode::Move,
        ..Default::default()
    };
    let mut e = Engine::new(cfg);
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    let actions = e.handle(Event::RightDown {
        cursor: Point { x: 200, y: 200 }, // center of [100..300]
        target: Some(default_target()),
    });
    assert!(actions.iter().any(|a| matches!(
        a,
        Action::BeginDrag {
            mode: DragMode::Move,
            ..
        }
    )));
    assert!(matches!(
        e.state(),
        State::Moving {
            drag_origin: DragOrigin::CenterMoveMode,
            ..
        }
    ));
}

#[test]
fn right_up_ends_center_move_mode_drag() {
    let cfg = EngineConfig {
        center_mode: CenterMode::Move,
        ..Default::default()
    };
    let mut e = Engine::new(cfg);
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    e.handle(Event::RightDown {
        cursor: Point { x: 200, y: 200 },
        target: Some(default_target()),
    });
    assert!(matches!(e.state(), State::Moving { .. }));
    let actions = e.handle(Event::RightUp);
    assert!(actions.contains(&Action::EndDrag { hwnd: WindowId(1) }));
    assert!(actions.contains(&Action::CancelMenuActivation));
    assert!(matches!(e.state(), State::Armed));
}

#[test]
fn right_down_non_center_with_move_mode_still_resizes() {
    let cfg = EngineConfig {
        center_mode: CenterMode::Move,
        ..Default::default()
    };
    let mut e = Engine::new(cfg);
    e.handle(Event::KeyChange {
        vk: VirtualKey::Alt,
        down: true,
    });
    let actions = e.handle(Event::RightDown {
        cursor: Point { x: 110, y: 110 }, // top-left sector of [100..300]
        target: Some(default_target()),
    });
    assert!(actions.iter().any(|a| matches!(
        a,
        Action::BeginDrag {
            mode: DragMode::Resize { .. },
            ..
        }
    )));
    assert!(matches!(e.state(), State::Resizing { .. }));
}
