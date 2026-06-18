use std::fs;
use std::path::Path;

use regex::Regex;

use super::{TokenBaseline, TokenPerfScorecard};

const TOKEN_KIND_SOURCE: &str = "crates/perl-token/src/kind.rs";
const TOKEN_KIND_FALLBACK_SOURCE: &str = "crates/perl-token/src/lib.rs";

pub(super) fn read_token_kind_source(root: &Path) -> String {
    fs::read_to_string(root.join(TOKEN_KIND_SOURCE))
        .or_else(|_| fs::read_to_string(root.join(TOKEN_KIND_FALLBACK_SOURCE)))
        .unwrap_or_default()
}

pub(super) fn crate_depends_on_token(root: &Path, cargo_toml: &str) -> bool {
    fs::read_to_string(root.join(cargo_toml)).ok().is_some_and(|content| {
        content.lines().any(|line| line.trim_start().starts_with("perl-token"))
    })
}

pub(super) fn count_runtime_dependencies(root: &Path) -> usize {
    let Ok(cargo) = fs::read_to_string(root.join("crates/perl-token/Cargo.toml")) else {
        return 0;
    };
    let mut in_dependencies = false;
    let mut count = 0;
    for line in cargo.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_dependencies = trimmed == "[dependencies]";
            continue;
        }
        if in_dependencies && !trimmed.is_empty() && !trimmed.starts_with('#') {
            count += 1;
        }
    }
    count
}

pub(super) fn read_token_baseline(root: &Path) -> Option<TokenBaseline> {
    let raw = fs::read_to_string(root.join(".ci/metrics/baselines/token.json")).ok()?;
    serde_json::from_str(&raw).ok()
}

pub(super) fn read_token_perf_scorecard(root: &Path) -> Option<TokenPerfScorecard> {
    let raw = fs::read_to_string(root.join("docs/project/status/token_performance_scorecard.json"))
        .ok()?;
    serde_json::from_str(&raw).ok()
}

pub(super) fn token_kind_variants(source: &str) -> Vec<String> {
    let Some(enum_start) = source.find("pub enum TokenKind") else {
        return Vec::new();
    };
    let enum_header_end = source[enum_start..].find('{').map(|i| enum_start + i + 1);
    let Some(body_start) = enum_header_end else {
        return Vec::new();
    };
    let mut depth = 1usize;
    let mut enum_end = body_start;
    for (i, ch) in source[body_start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    enum_end = body_start + i;
                    break;
                }
            }
            _ => {}
        }
    }
    let enum_body = &source[body_start..enum_end];
    let Ok(re) = Regex::new(r"^\s*([A-Z][A-Za-z0-9]*)\s*,\s*$") else {
        return Vec::new();
    };
    enum_body
        .lines()
        .filter_map(|line| re.captures(line))
        .filter_map(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .collect()
}

pub(super) fn token_display_name_arms(source: &str) -> Vec<String> {
    let Some(impl_start) = source.find("impl TokenKind") else {
        return Vec::new();
    };
    let impl_tail = &source[impl_start..];

    let Some(fn_rel) = impl_tail.find("fn display_name(self)") else {
        return Vec::new();
    };
    let fn_start = impl_start + fn_rel;

    let body_start_offset = source[fn_start..].find('{').map(|i| fn_start + i + 1);
    let Some(body_start) = body_start_offset else {
        return Vec::new();
    };
    let mut depth = 1usize;
    let mut body_end = body_start;
    for (i, ch) in source[body_start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    body_end = body_start + i;
                    break;
                }
            }
            _ => {}
        }
    }

    let fn_body = &source[body_start..body_end];
    let Ok(re) = Regex::new(r"TokenKind::([A-Z][A-Za-z0-9]*)\s*=>") else {
        return Vec::new();
    };
    re.captures_iter(fn_body)
        .filter_map(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .collect()
}

pub(super) fn token_category_counts(
    source: &str,
) -> std::collections::BTreeMap<&'static str, usize> {
    let Some(enum_start) = source.find("pub enum TokenKind") else {
        return std::collections::BTreeMap::new();
    };
    let enum_header_end = source[enum_start..].find('{').map(|i| enum_start + i + 1);
    let Some(body_start) = enum_header_end else {
        return std::collections::BTreeMap::new();
    };
    let mut depth = 1usize;
    let mut enum_end = body_start;
    for (i, ch) in source[body_start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    enum_end = body_start + i;
                    break;
                }
            }
            _ => {}
        }
    }
    let enum_body = &source[body_start..enum_end];
    let mut current = "";
    let mut counts = std::collections::BTreeMap::new();
    let Ok(variant_re) = Regex::new(r"^\s*([A-Z][A-Za-z0-9]*)\s*,\s*$") else {
        return counts;
    };
    for line in enum_body.lines() {
        let trimmed = line.trim();
        current = match trimmed {
            "// ===== Keywords =====" => "keywords",
            "// ===== Operators =====" => "operators",
            "// ===== Delimiters =====" => "delimiters",
            "// ===== Literals =====" => "literals",
            "// ===== Identifiers and Variables =====" => "identifiers/sigils",
            "// ===== Special =====" => "special",
            _ => current,
        };
        if !current.is_empty() && variant_re.is_match(trimmed) {
            *counts.entry(current).or_insert(0) += 1;
        }
    }
    counts
}
