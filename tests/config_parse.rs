use alt3rsnap::config::{load_from_str, FileConfig};

#[test]
fn empty_string_yields_defaults() {
    let cfg = load_from_str("").expect("should parse empty");
    assert_eq!(cfg, FileConfig::default());
}

#[test]
fn modifier_string_is_parsed() {
    let cfg = load_from_str(
        r#"
        [activation]
        modifier = "ctrl"
    "#,
    )
    .unwrap();
    assert_eq!(cfg.activation.modifier, "ctrl");
}

#[test]
fn exclude_processes_list_round_trips() {
    let cfg = load_from_str(
        r#"
        [exclude]
        processes = ["mstsc.exe", "vmware-vmx.exe"]
    "#,
    )
    .unwrap();
    assert_eq!(cfg.exclude.processes, vec!["mstsc.exe", "vmware-vmx.exe"]);
}

#[test]
fn unknown_keys_are_silently_tolerated() {
    let cfg = load_from_str(
        r#"
        [behavior]
        enable_move = true
        mystery_key = 42
    "#,
    );
    assert!(cfg.is_ok());
}

#[test]
fn bad_type_errors() {
    let err = load_from_str(
        r#"
        [behavior]
        enable_move = "not a bool"
    "#,
    );
    assert!(err.is_err());
}

use alt3rsnap::engine::modifiers::Modifiers;

#[test]
fn file_config_with_alt_modifier_produces_arm_matcher_requiring_alt() {
    let file = FileConfig::default();
    let ec = file.to_engine_config().unwrap();
    assert!(ec.policy.arm.matches(Modifiers::ALT));
}

#[test]
fn file_config_with_ctrl_modifier_produces_arm_matcher_requiring_ctrl() {
    let mut file = FileConfig::default();
    file.activation.modifier = "ctrl".into();
    let ec = file.to_engine_config().unwrap();
    assert!(ec.policy.arm.matches(Modifiers::CTRL));
    assert!(!ec.policy.arm.matches(Modifiers::ALT));
}

#[test]
fn unknown_modifier_errors() {
    let mut file = FileConfig::default();
    file.activation.modifier = "not-a-key".into();
    assert!(file.to_engine_config().is_err());
}

#[test]
fn exclude_processes_become_rules() {
    let mut file = FileConfig::default();
    file.exclude.processes = vec!["mstsc.exe".into()];
    let ec = file.to_engine_config().unwrap();
    assert_eq!(ec.rules.len(), 1);
}

#[test]
fn behavior_middle_click_action_defaults_to_none_string() {
    let cfg = load_from_str("").expect("should parse empty");
    assert_eq!(cfg.behavior.middle_click_action, "none");
}

#[test]
fn behavior_middle_click_action_round_trip() {
    let cfg = load_from_str(
        r#"
        [behavior]
        middle_click_action = "toggle_maximize"
    "#,
    )
    .unwrap();
    assert_eq!(cfg.behavior.middle_click_action, "toggle_maximize");
}

#[test]
fn behavior_unknown_middle_click_action_preserved_at_file_layer() {
    let cfg = load_from_str(
        r#"
        [behavior]
        middle_click_action = "roll_up_window"
    "#,
    )
    .unwrap();
    // File layer preserves the raw string; bridge is where validation happens.
    assert_eq!(cfg.behavior.middle_click_action, "roll_up_window");
}

use alt3rsnap::engine::config::CenterMode;

#[test]
fn resize_center_mode_defaults_to_symmetric_string() {
    let cfg = load_from_str("").expect("should parse empty");
    assert_eq!(cfg.resize.center_mode, "symmetric");
}

#[test]
fn resize_center_mode_round_trip_bottom_right() {
    let cfg = load_from_str(
        r#"
        [resize]
        center_mode = "bottom_right"
    "#,
    )
    .unwrap();
    assert_eq!(cfg.resize.center_mode, "bottom_right");
}

#[test]
fn resize_center_mode_round_trip_move() {
    let cfg = load_from_str(
        r#"
        [resize]
        center_mode = "move"
    "#,
    )
    .unwrap();
    assert_eq!(cfg.resize.center_mode, "move");
}

#[test]
fn bridge_center_mode_symmetric() {
    let mut file = FileConfig::default();
    file.resize.center_mode = "symmetric".into();
    let ec = file.to_engine_config().expect("bridge ok");
    assert_eq!(ec.center_mode, CenterMode::Symmetric);
}

#[test]
fn bridge_center_mode_bottom_right() {
    let mut file = FileConfig::default();
    file.resize.center_mode = "bottom_right".into();
    let ec = file.to_engine_config().expect("bridge ok");
    assert_eq!(ec.center_mode, CenterMode::BottomRight);
}

#[test]
fn bridge_center_mode_move() {
    let mut file = FileConfig::default();
    file.resize.center_mode = "move".into();
    let ec = file.to_engine_config().expect("bridge ok");
    assert_eq!(ec.center_mode, CenterMode::Move);
}

#[test]
fn bridge_center_mode_unknown_defaults_to_symmetric() {
    let mut file = FileConfig::default();
    file.resize.center_mode = "closest_edge".into();
    let ec = file
        .to_engine_config()
        .expect("bridge must not error on unknown center_mode");
    assert_eq!(ec.center_mode, CenterMode::Symmetric);
}

#[test]
fn bridge_center_mode_empty_defaults_to_symmetric() {
    let mut file = FileConfig::default();
    file.resize.center_mode = "".into();
    let ec = file.to_engine_config().expect("bridge ok");
    assert_eq!(ec.center_mode, CenterMode::Symmetric);
}

#[test]
fn v01_config_without_center_mode_uses_symmetric_default() {
    // A v0.1 config that never sets center_mode must still load cleanly.
    let cfg = load_from_str(
        r#"
        [activation]
        modifier = "alt"
        [behavior]
        enable_move = true
    "#,
    )
    .unwrap();
    let ec = cfg.to_engine_config().expect("v0.1 config ok on v0.2 code");
    assert_eq!(ec.center_mode, CenterMode::Symmetric);
}

use alt3rsnap::engine::config::MiddleClickAction;

#[test]
fn bridge_middle_click_action_none() {
    let mut file = FileConfig::default();
    file.behavior.middle_click_action = "none".into();
    let engine = file.to_engine_config().expect("bridge ok");
    assert_eq!(engine.middle_click_action, MiddleClickAction::None);
}

#[test]
fn bridge_middle_click_action_toggle_maximize() {
    let mut file = FileConfig::default();
    file.behavior.middle_click_action = "toggle_maximize".into();
    let engine = file.to_engine_config().expect("bridge ok");
    assert_eq!(
        engine.middle_click_action,
        MiddleClickAction::ToggleMaximize
    );
}

#[test]
fn bridge_middle_click_action_unknown_defaults_to_none() {
    let mut file = FileConfig::default();
    file.behavior.middle_click_action = "maximize_or_something".into();
    // Bridge must succeed (warn-and-default, not error) on unknown strings.
    let engine = file
        .to_engine_config()
        .expect("bridge must not error on unknown middle_click_action");
    assert_eq!(engine.middle_click_action, MiddleClickAction::None);
}

#[test]
fn bridge_middle_click_action_empty_defaults_to_none() {
    let mut file = FileConfig::default();
    file.behavior.middle_click_action = "".into();
    let engine = file.to_engine_config().expect("bridge ok");
    assert_eq!(engine.middle_click_action, MiddleClickAction::None);
}

#[test]
fn bridge_middle_click_action_case_insensitive() {
    let mut file = FileConfig::default();
    file.behavior.middle_click_action = "Toggle_Maximize".into();
    let engine = file.to_engine_config().expect("bridge ok");
    assert_eq!(
        engine.middle_click_action,
        MiddleClickAction::ToggleMaximize
    );
}
