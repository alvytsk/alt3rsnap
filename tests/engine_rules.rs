use alt3rsnap::engine::rules::{
    evaluate, Pattern, RuleAction, WindowInfo, WindowRule, WindowTraitMask, WindowTraits,
};

fn info(process: &str, class: &str, title: &str) -> WindowInfo {
    WindowInfo {
        process_basename: process.to_lowercase(),
        class_name: class.to_string(),
        title: title.to_string(),
        traits: WindowTraits::default(),
    }
}

#[test]
fn process_exact_match_excludes() {
    let rules = vec![WindowRule {
        match_process: Some(Pattern::exact("mstsc.exe")),
        match_class: None,
        match_title: None,
        match_traits: WindowTraitMask::default(),
        action: RuleAction::Exclude,
    }];
    assert_eq!(
        evaluate(&rules, &info("mstsc.exe", "", "")),
        Some(&RuleAction::Exclude)
    );
    assert_eq!(evaluate(&rules, &info("notepad.exe", "", "")), None);
}

#[test]
fn process_exact_is_case_insensitive() {
    let rules = vec![WindowRule {
        match_process: Some(Pattern::exact("MSTSC.exe")),
        match_class: None,
        match_title: None,
        match_traits: WindowTraitMask::default(),
        action: RuleAction::Exclude,
    }];
    assert_eq!(
        evaluate(&rules, &info("mstsc.exe", "", "")),
        Some(&RuleAction::Exclude)
    );
}

#[test]
fn glob_matches_prefix() {
    let rules = vec![WindowRule {
        match_process: Some(Pattern::glob("game_*.exe")),
        match_class: None,
        match_title: None,
        match_traits: WindowTraitMask::default(),
        action: RuleAction::Exclude,
    }];
    assert_eq!(
        evaluate(&rules, &info("game_foo.exe", "", "")),
        Some(&RuleAction::Exclude)
    );
    assert_eq!(evaluate(&rules, &info("foo.exe", "", "")), None);
}

#[test]
fn first_matching_rule_wins() {
    let rules = vec![
        WindowRule {
            match_process: Some(Pattern::exact("notepad.exe")),
            match_class: None,
            match_title: None,
            match_traits: WindowTraitMask::default(),
            action: RuleAction::IncludeOnly,
        },
        WindowRule {
            match_process: Some(Pattern::exact("notepad.exe")),
            match_class: None,
            match_title: None,
            match_traits: WindowTraitMask::default(),
            action: RuleAction::Exclude,
        },
    ];
    assert_eq!(
        evaluate(&rules, &info("notepad.exe", "", "")),
        Some(&RuleAction::IncludeOnly)
    );
}

#[test]
fn class_and_title_both_required_to_match() {
    let rules = vec![WindowRule {
        match_process: None,
        match_class: Some(Pattern::exact("MyClass")),
        match_title: Some(Pattern::exact("MyTitle")),
        match_traits: WindowTraitMask::default(),
        action: RuleAction::Exclude,
    }];
    assert_eq!(
        evaluate(&rules, &info("x.exe", "MyClass", "MyTitle")),
        Some(&RuleAction::Exclude)
    );
    assert_eq!(evaluate(&rules, &info("x.exe", "MyClass", "Other")), None);
    assert_eq!(
        evaluate(&rules, &info("x.exe", "OtherClass", "MyTitle")),
        None
    );
}

#[test]
fn trait_mask_require_topmost_filters() {
    let rules = vec![WindowRule {
        match_process: None,
        match_class: None,
        match_title: None,
        match_traits: WindowTraitMask {
            require_topmost: Some(true),
            ..Default::default()
        },
        action: RuleAction::Exclude,
    }];
    let mut topmost = info("x.exe", "", "");
    topmost.traits.is_topmost = true;
    assert_eq!(evaluate(&rules, &topmost), Some(&RuleAction::Exclude));
    assert_eq!(evaluate(&rules, &info("x.exe", "", "")), None);
}
