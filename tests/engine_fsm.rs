use alt3rsnap::engine::config::EngineConfig;
use alt3rsnap::engine::state::{Event, State, VirtualKey};
use alt3rsnap::engine::Engine;

#[test]
fn idle_transitions_to_armed_on_alt_down() {
    let mut e = Engine::new(EngineConfig::default());
    assert!(matches!(e.state(), State::Idle));
    let _ = e.handle(Event::KeyChange { vk: VirtualKey::Alt, down: true });
    assert!(matches!(e.state(), State::Armed));
}

#[test]
fn armed_transitions_back_to_idle_on_alt_up() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::KeyChange { vk: VirtualKey::Alt, down: true });
    e.handle(Event::KeyChange { vk: VirtualKey::Alt, down: false });
    assert!(matches!(e.state(), State::Idle));
}

#[test]
fn alt_plus_win_does_not_arm_due_to_forbidden_modifier() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::KeyChange { vk: VirtualKey::Win, down: true });
    e.handle(Event::KeyChange { vk: VirtualKey::Alt, down: true });
    assert!(matches!(e.state(), State::Idle));
}

use alt3rsnap::engine::geometry::{Point, Rect};
use alt3rsnap::engine::state::{Action, DragMode, DragTarget, WindowId};

fn default_target() -> DragTarget {
    DragTarget {
        hwnd: WindowId(1),
        initial_rect: Rect { left: 100, top: 100, right: 300, bottom: 300 },
        is_maximized: false,
        exclude: false,
    }
}

#[test]
fn armed_plus_left_down_begins_move_drag() {
    let mut e = Engine::new(EngineConfig::default());
    e.handle(Event::KeyChange { vk: VirtualKey::Alt, down: true });
    let actions = e.handle(Event::LeftDown {
        cursor: Point { x: 150, y: 150 },
        target: Some(default_target()),
    });

    assert!(actions.iter().any(|a| matches!(a,
        Action::BeginDrag { mode: DragMode::Move, .. })));
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
    e.handle(Event::KeyChange { vk: VirtualKey::Alt, down: true });
    let mut t = default_target();
    t.exclude = true;
    let actions = e.handle(Event::LeftDown {
        cursor: Point { x: 150, y: 150 },
        target: Some(t),
    });
    assert!(actions.is_empty());
    assert!(matches!(e.state(), State::Armed));
}
