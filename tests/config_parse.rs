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
