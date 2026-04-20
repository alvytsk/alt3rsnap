//! WindowRule pattern matching.
//!
//! Rules are evaluated in order; the first matching rule's `action` is returned.

use regex::Regex;

#[derive(Debug, Clone)]
pub enum Pattern {
    Exact(String), // case-insensitive exact
    Glob(String),  // '*' and '?' wildcards, case-insensitive
    Regex(Regex),  // full regex
}

impl Pattern {
    pub fn exact<S: Into<String>>(s: S) -> Pattern {
        Pattern::Exact(s.into().to_lowercase())
    }
    pub fn glob<S: Into<String>>(s: S) -> Pattern {
        Pattern::Glob(s.into().to_lowercase())
    }
    pub fn regex(r: Regex) -> Pattern {
        Pattern::Regex(r)
    }

    pub fn matches(&self, haystack: &str) -> bool {
        match self {
            Pattern::Exact(p) => haystack.eq_ignore_ascii_case(p),
            Pattern::Glob(p) => glob_match(p, &haystack.to_lowercase()),
            Pattern::Regex(r) => r.is_match(haystack),
        }
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    // Simple two-pointer glob matching with '*' and '?'.
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti, mut star, mut match_i) = (0usize, 0usize, usize::MAX, 0usize);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == t[ti] || p[pi] == '?') {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = pi;
            match_i = ti;
            pi += 1;
        } else if star != usize::MAX {
            pi = star + 1;
            match_i += 1;
            ti = match_i;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowTraits {
    pub is_topmost: bool,
    pub is_cloaked: bool,
    pub is_tool: bool,
    pub is_owned: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowTraitMask {
    pub require_topmost: Option<bool>,
    pub require_cloaked: Option<bool>,
    pub require_tool: Option<bool>,
    pub require_owned: Option<bool>,
}

impl WindowTraitMask {
    pub fn matches(self, t: WindowTraits) -> bool {
        let check = |req: Option<bool>, actual: bool| req.map_or(true, |r| r == actual);
        check(self.require_topmost, t.is_topmost)
            && check(self.require_cloaked, t.is_cloaked)
            && check(self.require_tool, t.is_tool)
            && check(self.require_owned, t.is_owned)
    }
}

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub process_basename: String, // lowercase
    pub class_name: String,
    pub title: String,
    pub traits: WindowTraits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleAction {
    Exclude,
    IncludeOnly,
    // Override(PerWindowConfig) — v0.2+
}

#[derive(Debug, Clone)]
pub struct WindowRule {
    pub match_process: Option<Pattern>,
    pub match_class: Option<Pattern>,
    pub match_title: Option<Pattern>,
    pub match_traits: WindowTraitMask,
    pub action: RuleAction,
}

impl WindowRule {
    fn matches(&self, w: &WindowInfo) -> bool {
        let ok = |p: &Option<Pattern>, h: &str| p.as_ref().map_or(true, |p| p.matches(h));
        ok(&self.match_process, &w.process_basename)
            && ok(&self.match_class, &w.class_name)
            && ok(&self.match_title, &w.title)
            && self.match_traits.matches(w.traits)
    }
}

pub fn evaluate<'a>(rules: &'a [WindowRule], w: &WindowInfo) -> Option<&'a RuleAction> {
    rules.iter().find(|r| r.matches(w)).map(|r| &r.action)
}

#[cfg(test)]
mod internal_tests {
    use super::*;

    #[test]
    fn glob_matches_suffix() {
        assert!(glob_match("*foo", "barfoo"));
        assert!(!glob_match("*foo", "foobar"));
    }

    #[test]
    fn glob_matches_single_char() {
        assert!(glob_match("fo?", "foo"));
        assert!(!glob_match("fo?", "fooo"));
    }
}
