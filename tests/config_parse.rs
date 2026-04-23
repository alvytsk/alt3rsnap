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

// ── M3: [[rules]] serde + bridge tests ───────────────────────────────────────

use alt3rsnap::config::PatternFile;

#[test]
fn rules_array_empty_by_default() {
    let cfg = load_from_str("").expect("should parse empty");
    assert!(cfg.rules.is_empty());
}

#[test]
fn rules_array_round_trips_exact_process() {
    let cfg = load_from_str(
        r#"
        [[rules]]
        match_process = { exact = "mstsc.exe" }
        action = "exclude"
    "#,
    )
    .unwrap();
    assert_eq!(cfg.rules.len(), 1);
    assert_eq!(
        cfg.rules[0].match_process,
        Some(PatternFile::Exact("mstsc.exe".into()))
    );
    assert_eq!(cfg.rules[0].action, "exclude");
}

#[test]
fn rules_array_round_trips_glob_process() {
    let cfg = load_from_str(
        r#"
        [[rules]]
        match_process = { glob = "game_*.exe" }
        action = "exclude"
    "#,
    )
    .unwrap();
    assert_eq!(
        cfg.rules[0].match_process,
        Some(PatternFile::Glob("game_*.exe".into()))
    );
}

#[test]
fn rules_array_round_trips_regex_process() {
    let cfg = load_from_str(
        r#"
        [[rules]]
        match_process = { regex = "chrome.*\\.exe" }
        action = "exclude"
    "#,
    )
    .unwrap();
    assert_eq!(
        cfg.rules[0].match_process,
        Some(PatternFile::Regex("chrome.*\\.exe".into()))
    );
}

#[test]
fn rules_array_round_trips_class_and_title() {
    let cfg = load_from_str(
        r#"
        [[rules]]
        match_class = { exact = "ConsoleWindowClass" }
        match_title = { glob = "My App*" }
        action = "exclude"
    "#,
    )
    .unwrap();
    assert_eq!(
        cfg.rules[0].match_class,
        Some(PatternFile::Exact("ConsoleWindowClass".into()))
    );
    assert_eq!(
        cfg.rules[0].match_title,
        Some(PatternFile::Glob("My App*".into()))
    );
}

#[test]
fn rules_array_round_trips_trait_mask() {
    let cfg = load_from_str(
        r#"
        [[rules]]
        match_process = { exact = "foo.exe" }
        action = "exclude"
        match_traits = { require_topmost = true, require_tool = false }
    "#,
    )
    .unwrap();
    assert_eq!(cfg.rules[0].match_traits.require_topmost, Some(true));
    assert_eq!(cfg.rules[0].match_traits.require_tool, Some(false));
    assert_eq!(cfg.rules[0].match_traits.require_cloaked, None);
}

#[test]
fn multiple_rules_preserve_order() {
    let cfg = load_from_str(
        r#"
        [[rules]]
        match_process = { exact = "first.exe" }
        action = "exclude"

        [[rules]]
        match_process = { exact = "second.exe" }
        action = "exclude"
    "#,
    )
    .unwrap();
    assert_eq!(cfg.rules.len(), 2);
    assert_eq!(
        cfg.rules[0].match_process,
        Some(PatternFile::Exact("first.exe".into()))
    );
    assert_eq!(
        cfg.rules[1].match_process,
        Some(PatternFile::Exact("second.exe".into()))
    );
}

// ── M3: bridge validation tests ──────────────────────────────────────────────

use alt3rsnap::engine::rules::{evaluate, Pattern, RuleAction, WindowInfo, WindowTraits};

fn win_info(process: &str, class: &str, title: &str) -> WindowInfo {
    WindowInfo {
        process_basename: process.to_lowercase(),
        class_name: class.to_string(),
        title: title.to_string(),
        traits: WindowTraits::default(),
    }
}

#[test]
fn bridge_exclude_processes_and_rules_merge_exclude_first() {
    let cfg = load_from_str(
        r#"
        [exclude]
        processes = ["mstsc.exe"]

        [[rules]]
        match_process = { exact = "notepad.exe" }
        action = "exclude"
    "#,
    )
    .unwrap();
    let ec = cfg.to_engine_config().unwrap();
    // exclude.processes entries come first
    assert_eq!(ec.rules.len(), 2);
    // First rule from [exclude].processes
    assert_eq!(
        evaluate(&ec.rules, &win_info("mstsc.exe", "", "")),
        Some(&RuleAction::Exclude)
    );
    // Second rule from [[rules]]
    assert_eq!(
        evaluate(&ec.rules, &win_info("notepad.exe", "", "")),
        Some(&RuleAction::Exclude)
    );
}

#[test]
fn bridge_exclude_processes_prepend_order() {
    let cfg = load_from_str(
        r#"
        [exclude]
        processes = ["excluded.exe"]

        [[rules]]
        match_process = { exact = "custom.exe" }
        action = "exclude"
    "#,
    )
    .unwrap();
    let ec = cfg.to_engine_config().unwrap();
    // exclude.processes rule is index 0, [[rules]] entry is index 1
    if let Some(Pattern::Exact(ref s)) = ec.rules[0].match_process {
        assert_eq!(s.as_str(), "excluded.exe");
    } else {
        panic!("expected Exact pattern at index 0");
    }
}

#[test]
fn bridge_matcher_less_rule_is_dropped() {
    let cfg = load_from_str(
        r#"
        [[rules]]
        action = "exclude"
    "#,
    )
    .unwrap();
    let ec = cfg.to_engine_config().unwrap();
    assert_eq!(ec.rules.len(), 0);
}

#[test]
fn bridge_include_only_action_is_dropped() {
    let cfg = load_from_str(
        r#"
        [[rules]]
        match_process = { exact = "foo.exe" }
        action = "include_only"
    "#,
    )
    .unwrap();
    let ec = cfg.to_engine_config().unwrap();
    assert_eq!(ec.rules.len(), 0);
}

#[test]
fn bridge_override_action_is_dropped() {
    let cfg = load_from_str(
        r#"
        [[rules]]
        match_process = { exact = "foo.exe" }
        action = "override"
    "#,
    )
    .unwrap();
    let ec = cfg.to_engine_config().unwrap();
    assert_eq!(ec.rules.len(), 0);
}

#[test]
fn bridge_unknown_action_is_dropped() {
    let cfg = load_from_str(
        r#"
        [[rules]]
        match_process = { exact = "foo.exe" }
        action = "vaporize"
    "#,
    )
    .unwrap();
    let ec = cfg.to_engine_config().unwrap();
    assert_eq!(ec.rules.len(), 0);
}

#[test]
fn bridge_regex_compile_error_drops_that_rule_only() {
    let cfg = load_from_str(
        r#"
        [[rules]]
        match_process = { regex = "[invalid(regex" }
        action = "exclude"

        [[rules]]
        match_process = { exact = "good.exe" }
        action = "exclude"
    "#,
    )
    .unwrap();
    let ec = cfg.to_engine_config().unwrap();
    // Only the good rule survives
    assert_eq!(ec.rules.len(), 1);
    assert_eq!(
        evaluate(&ec.rules, &win_info("good.exe", "", "")),
        Some(&RuleAction::Exclude)
    );
}

#[test]
fn bridge_process_regex_is_case_insensitive() {
    let cfg = load_from_str(
        r#"
        [[rules]]
        match_process = { regex = "Chrome\\.exe" }
        action = "exclude"
    "#,
    )
    .unwrap();
    let ec = cfg.to_engine_config().unwrap();
    // "chrome.exe" (lowercase) should match the "Chrome.exe" regex because
    // the bridge compiles match_process regexes with case_insensitive(true).
    assert_eq!(
        evaluate(&ec.rules, &win_info("chrome.exe", "", "")),
        Some(&RuleAction::Exclude)
    );
    assert_eq!(
        evaluate(&ec.rules, &win_info("CHROME.EXE", "", "")),
        Some(&RuleAction::Exclude)
    );
}

#[test]
fn bridge_class_regex_is_case_sensitive_by_default() {
    let cfg = load_from_str(
        r#"
        [[rules]]
        match_class = { regex = "^ConsoleWindowClass$" }
        action = "exclude"
    "#,
    )
    .unwrap();
    let ec = cfg.to_engine_config().unwrap();
    assert_eq!(
        evaluate(&ec.rules, &win_info("cmd.exe", "ConsoleWindowClass", "")),
        Some(&RuleAction::Exclude)
    );
    // Different case does not match (no (?i) flag)
    assert_eq!(
        evaluate(&ec.rules, &win_info("cmd.exe", "consolewindowclass", "")),
        None
    );
}

#[test]
fn bridge_class_regex_case_insensitive_via_inline_flag() {
    let cfg = load_from_str(
        r#"
        [[rules]]
        match_class = { regex = "(?i)^ConsoleWindowClass$" }
        action = "exclude"
    "#,
    )
    .unwrap();
    let ec = cfg.to_engine_config().unwrap();
    assert_eq!(
        evaluate(&ec.rules, &win_info("cmd.exe", "consolewindowclass", "")),
        Some(&RuleAction::Exclude)
    );
}

#[test]
fn bridge_process_exact_is_case_insensitive_via_lowercasing() {
    // [exclude].processes entries are desugared as Exact(lowercase).
    let cfg = load_from_str(
        r#"
        [exclude]
        processes = ["MSTSC.EXE"]
    "#,
    )
    .unwrap();
    let ec = cfg.to_engine_config().unwrap();
    assert_eq!(
        evaluate(&ec.rules, &win_info("mstsc.exe", "", "")),
        Some(&RuleAction::Exclude)
    );
}

#[test]
fn bridge_rules_with_trait_mask_produce_correct_window_rule() {
    let cfg = load_from_str(
        r#"
        [[rules]]
        match_class = { exact = "SomeClass" }
        action = "exclude"
        match_traits = { require_topmost = true }
    "#,
    )
    .unwrap();
    let ec = cfg.to_engine_config().unwrap();
    assert_eq!(ec.rules.len(), 1);
    assert_eq!(ec.rules[0].match_traits.require_topmost, Some(true));
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

// ── M4: [snap] file-layer serde tests ────────────────────────────────────────

#[test]
fn snap_section_defaults_when_missing() {
    let toml = r#"
        [activation]
        modifier = "alt"
    "#;
    let cfg: alt3rsnap::config::FileConfig = alt3rsnap::config::load_from_str(toml).unwrap();
    assert!(cfg.snap.enabled);
    assert_eq!(cfg.snap.engage_distance_px, 24);
    assert_eq!(cfg.snap.disengage_distance_px, 32);
    assert_eq!(cfg.snap.preview_opacity, 0x99);
    assert!(cfg.snap.zones.left_half);
    assert!(!cfg.snap.zones.left_third);
    assert!(!cfg.snap.zones.bottom_maximize);
}

#[test]
fn snap_section_full_round_trip() {
    let toml = r#"
        [snap]
        enabled = true
        engage_distance_px = 30
        disengage_distance_px = 40
        preview_opacity = 200

        [snap.zones]
        top_maximize = false
        left_third = true
    "#;
    let cfg: alt3rsnap::config::FileConfig = alt3rsnap::config::load_from_str(toml).unwrap();
    assert_eq!(cfg.snap.engage_distance_px, 30);
    assert_eq!(cfg.snap.disengage_distance_px, 40);
    assert_eq!(cfg.snap.preview_opacity, 200);
    assert!(!cfg.snap.zones.top_maximize);
    assert!(cfg.snap.zones.left_third);
}

// ── M4: [snap] bridge + RuntimeConfig tests ──────────────────────────────────

#[test]
fn snap_bridge_clamps_disengage_up_to_engage_when_less() {
    let toml = r#"
        [snap]
        engage_distance_px = 50
        disengage_distance_px = 10
    "#;
    let cfg: alt3rsnap::config::FileConfig = alt3rsnap::config::load_from_str(toml).unwrap();
    let rt = cfg.to_runtime_config().unwrap();
    assert_eq!(rt.engine.snap.engage_px, 50);
    assert_eq!(rt.engine.snap.disengage_px, 50);
}

#[test]
fn snap_bridge_caps_engage_at_256() {
    let toml = r#"
        [snap]
        engage_distance_px = 9999
        disengage_distance_px = 9999
    "#;
    let cfg: alt3rsnap::config::FileConfig = alt3rsnap::config::load_from_str(toml).unwrap();
    let rt = cfg.to_runtime_config().unwrap();
    assert_eq!(rt.engine.snap.engage_px, 256);
    assert_eq!(rt.engine.snap.disengage_px, 256);
}

#[test]
fn snap_bridge_passes_zones_through() {
    let toml = r#"
        [snap.zones]
        bottom_maximize = true
        left_third = true
    "#;
    let cfg: alt3rsnap::config::FileConfig = alt3rsnap::config::load_from_str(toml).unwrap();
    let rt = cfg.to_runtime_config().unwrap();
    assert!(rt.engine.snap.zones.bottom_maximize);
    assert!(rt.engine.snap.zones.left_third);
}

#[test]
fn runtime_config_adapter_preview_opacity() {
    let toml = r#"[snap]
        preview_opacity = 128
    "#;
    let cfg: alt3rsnap::config::FileConfig = alt3rsnap::config::load_from_str(toml).unwrap();
    let rt = cfg.to_runtime_config().unwrap();
    assert_eq!(rt.adapter.preview_opacity, 128);
}

#[test]
fn v01_config_without_snap_or_rules_still_loads() {
    // A representative v0.1 config — no [snap], no [[rules]], no middle_click_action.
    let toml = r#"
        [activation]
        modifier = "alt"

        [behavior]
        enable_move = true
        enable_resize = true
        raise_on_drag = false
        restore_maximized_on_move = true

        [resize]
        center_mode = "symmetric"
        center_fraction = 0.333

        [exclude]
        processes = []
    "#;
    let cfg: alt3rsnap::config::FileConfig = alt3rsnap::config::load_from_str(toml).unwrap();
    let rt = cfg.to_runtime_config().unwrap();
    assert!(
        rt.engine.snap.enabled,
        "snap enabled by default on upgrade per spec §7.3"
    );
    assert_eq!(rt.adapter.preview_opacity, 0x99);
}
