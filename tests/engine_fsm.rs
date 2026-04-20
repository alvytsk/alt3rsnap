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
