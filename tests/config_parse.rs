use alt3rsnap::config::{load_from_str, FileConfig};

#[test]
fn empty_string_yields_defaults() {
    let cfg = load_from_str("").expect("should parse empty");
    assert_eq!(cfg, FileConfig::default());
}

#[test]
fn modifier_string_is_parsed() {
    let cfg = load_from_str(r#"
        [activation]
        modifier = "ctrl"
    "#).unwrap();
    assert_eq!(cfg.activation.modifier, "ctrl");
}

#[test]
fn exclude_processes_list_round_trips() {
    let cfg = load_from_str(r#"
        [exclude]
        processes = ["mstsc.exe", "vmware-vmx.exe"]
    "#).unwrap();
    assert_eq!(cfg.exclude.processes, vec!["mstsc.exe", "vmware-vmx.exe"]);
}

#[test]
fn unknown_keys_are_silently_tolerated() {
    let cfg = load_from_str(r#"
        [behavior]
        enable_move = true
        mystery_key = 42
    "#);
    assert!(cfg.is_ok());
}

#[test]
fn bad_type_errors() {
    let err = load_from_str(r#"
        [behavior]
        enable_move = "not a bool"
    "#);
    assert!(err.is_err());
}
