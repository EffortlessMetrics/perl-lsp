//! Token subsystem health metrics collection.
//!
//! Collects variant counts, metadata coverage, categorization, and performance metrics
//! from perl-token for inclusion in project status reporting.

use std::fs;
use std::path::Path;

use regex::Regex;
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct TokenBaseline {
    floor_metrics: TokenFloorMetrics,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenFloorMetrics {
    metadata_coverage_pct: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenPerfScorecard {
    metrics: std::collections::BTreeMap<String, TokenPerfMetric>,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenPerfMetric {
    median_ns: u128,
    p95_ns: u128,
}

#[derive(Debug, Clone)]
pub struct TokenHealthMetrics {
    /// Total number of `TokenKind` enum variants.
    pub variant_count: usize,
    /// Number of variants that have a `display_name()` mapping.
    ///
    /// Currently identical to `display_name_coverage_count`.  Kept as a
    /// separate field so the metadata-coverage concept can expand to include
    /// additional per-variant metadata (e.g., `is_keyword()`, precedence)
    /// without a breaking struct change.
    pub metadata_coverage_count: usize,
    /// Number of variants covered by `display_name()` match arms.
    pub display_name_coverage_count: usize,
    /// `"PASS"`, `"WARN"`, or `"FAIL"` based on coverage vs. the baseline.
    pub metadata_status: &'static str,
    /// Human-readable summary of category partition health.
    pub category_partition_status: String,
    /// Human-readable summary of lexer + parser-core token dependency check.
    pub lexer_parser_conformance_status: String,
    /// Count of non-dev, non-comment lines under `[dependencies]` in
    /// `crates/perl-token/Cargo.toml`.
    pub runtime_dependency_count: usize,
    /// Human-readable performance summary row, or `"UNVERIFIED …"` when the
    /// scorecard JSON is missing.
    pub performance_row: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn collect_token_health_metrics(root: &Path) -> TokenHealthMetrics {
    let token_src = root.join("crates/perl-token/src/lib.rs");
    let token_lib = fs::read_to_string(token_src).unwrap_or_default();
    let variants = token_kind_variants(&token_lib);
    let display_name_arms = token_display_name_arms(&token_lib);
    let category_counts = token_category_counts(&token_lib);
    let category_total = category_counts.values().sum::<usize>();
    let uncategorized = variants.len().saturating_sub(category_total);
    let category_partition_status = if uncategorized == 0 && category_total == variants.len() {
        format!("PASS ({category_total} tokens partitioned across canonical groups)")
    } else {
        format!("WARN ({} partitioned, {} uncategorized)", category_total, uncategorized)
    };

    let metadata_coverage_count = display_name_arms.len();
    let display_name_coverage_count = display_name_arms.len();
    let metadata_coverage_pct = metadata_coverage_count as f64 / variants.len().max(1) as f64;
    let baseline = read_token_baseline(root);
    let metadata_status = baseline.map_or(
        if metadata_coverage_count == variants.len() { "PASS" } else { "WARN" },
        |b| {
            if metadata_coverage_pct + f64::EPSILON < b.floor_metrics.metadata_coverage_pct {
                "FAIL"
            } else if metadata_coverage_count == variants.len() {
                "PASS"
            } else {
                "WARN"
            }
        },
    );

    let lexer_dep = crate_depends_on_token(root, "crates/perl-lexer/Cargo.toml");
    let parser_dep = crate_depends_on_token(root, "crates/perl-parser-core/Cargo.toml");
    let lexer_parser_conformance_status = if lexer_dep && parser_dep {
        "PASS (lexer + parser-core both consume shared `perl-token`)".to_string()
    } else {
        format!("WARN (lexer dependency: {lexer_dep}, parser-core dependency: {parser_dep})")
    };

    let runtime_dependency_count = count_runtime_dependencies(root);

    let performance_row = read_token_perf_scorecard(root).map_or_else(
        || "UNVERIFIED (token scorecard missing)".to_string(),
        |scorecard| {
            let mut keys = [
                ("display_name_lookup", "display_name"),
                ("token_kind_clone", "kind copy"),
                ("token_allocation", "token alloc"),
            ]
            .into_iter()
            .filter_map(|(key, label)| {
                scorecard.metrics.get(key).map(|metric| {
                    format!(
                        "{label}: p50 {:.3} ms / p95 {:.3} ms",
                        ns_to_ms(metric.median_ns),
                        ns_to_ms(metric.p95_ns)
                    )
                })
            })
            .collect::<Vec<_>>();
            if keys.is_empty() {
                "UNVERIFIED (token scorecard missing key metrics)".to_string()
            } else {
                keys.sort();
                format!("PASS ({})", keys.join("; "))
            }
        },
    );

    TokenHealthMetrics {
        variant_count: variants.len(),
        metadata_coverage_count,
        display_name_coverage_count,
        metadata_status,
        category_partition_status,
        lexer_parser_conformance_status,
        runtime_dependency_count,
        performance_row,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn crate_depends_on_token(root: &Path, cargo_toml: &str) -> bool {
    fs::read_to_string(root.join(cargo_toml)).ok().is_some_and(|content| {
        content.lines().any(|line| line.trim_start().starts_with("perl-token"))
    })
}

fn count_runtime_dependencies(root: &Path) -> usize {
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

fn read_token_baseline(root: &Path) -> Option<TokenBaseline> {
    let raw = fs::read_to_string(root.join(".ci/metrics/baselines/token.json")).ok()?;
    serde_json::from_str(&raw).ok()
}

fn read_token_perf_scorecard(root: &Path) -> Option<TokenPerfScorecard> {
    let raw = fs::read_to_string(root.join("docs/project/status/token_performance_scorecard.json"))
        .ok()?;
    serde_json::from_str(&raw).ok()
}

fn token_kind_variants(source: &str) -> Vec<String> {
    let Some(enum_start) = source.find("pub enum TokenKind") else {
        return Vec::new();
    };
    // Find the closing `}` of the `pub enum TokenKind` block by tracking
    // brace depth from the opening `{`.  This avoids including variants from
    // adjacent types (`TokenCategory`, `TokenKindMetadata`, …) that appear
    // between the enum's closing brace and `impl TokenKind`.
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

fn token_display_name_arms(source: &str) -> Vec<String> {
    // Locate the `display_name` method inside `impl TokenKind` (not
    // `impl Token` or `impl TokenRef`, which also have a `display_name`
    // delegating method).  We anchor on `impl TokenKind` first so we always
    // scan the canonical coverage match, not a delegation wrapper.
    let Some(impl_start) = source.find("impl TokenKind") else {
        return Vec::new();
    };
    let impl_tail = &source[impl_start..];

    // Find `fn display_name` within the impl block.
    let Some(fn_rel) = impl_tail.find("fn display_name(self)") else {
        return Vec::new();
    };
    let fn_start = impl_start + fn_rel;

    // Advance to the opening `{` of the function body.
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

fn token_category_counts(source: &str) -> std::collections::BTreeMap<&'static str, usize> {
    let Some(enum_start) = source.find("pub enum TokenKind") else {
        return std::collections::BTreeMap::new();
    };
    // Mirror the same brace-tracking boundary used in `token_kind_variants`
    // so the two functions stay in sync as the source file evolves.
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

fn ns_to_ms(ns: u128) -> f64 {
    ns as f64 / 1_000_000.0
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

#[cfg(test)]
pub fn token_metrics_fixture() -> TokenHealthMetrics {
    TokenHealthMetrics {
        variant_count: 132,
        metadata_coverage_count: 132,
        display_name_coverage_count: 132,
        metadata_status: "PASS",
        category_partition_status: "PASS (132 tokens partitioned across canonical groups)"
            .to_string(),
        lexer_parser_conformance_status:
            "PASS (lexer + parser-core both consume shared `perl-token`)".to_string(),
        runtime_dependency_count: 0,
        performance_row: "UNVERIFIED (token scorecard missing)".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::project_root;

    /// `token_kind_variants` must extract exactly the correct variant count from
    /// the real `perl-token` source.  The fixture hardcodes 132 — this test
    /// ensures the parser and the fixture stay in sync as the enum grows.
    #[test]
    fn token_kind_variants_matches_actual_enum() {
        let root = project_root().expect("project root");
        let src = std::fs::read_to_string(root.join("crates/perl-token/src/lib.rs"))
            .expect("perl-token/src/lib.rs must exist");
        let variants = token_kind_variants(&src);
        assert!(
            !variants.is_empty(),
            "token_kind_variants must find at least one variant — check the regex or enum structure"
        );
        // The fixture constant is 132.  If the enum grows or shrinks, update the
        // fixture too.  This assertion catches the boundary-overcount bug (including
        // TokenCategory variants) as well as genuine enum changes.
        assert_eq!(
            variants.len(),
            132,
            "token_kind_variants returned {} variants but expected 132; \
             check that the enum boundary is computed correctly (only TokenKind variants, \
             not TokenCategory or other adjacent types)",
            variants.len()
        );
        // Every extracted name must start with an uppercase letter (the regex
        // guarantees this, but a double-check costs nothing).
        for name in &variants {
            assert!(
                name.chars().next().map_or(false, |c| c.is_uppercase()),
                "extracted variant {name:?} does not start with uppercase"
            );
        }
        // Spot-check: known TokenCategory variants must NOT appear in the list.
        // These were incorrectly included before the brace-tracking boundary fix.
        for spurious in &["Keyword", "Operator", "Delimiter", "Literal", "Identifier", "Special"] {
            assert!(
                !variants.iter().any(|v| v == spurious),
                "TokenCategory variant {spurious:?} must not appear in TokenKind variant list"
            );
        }
    }

    /// `token_display_name_arms` count must equal `token_kind_variants` count:
    /// every variant must have a display-name arm and no arm must be orphaned.
    #[test]
    fn display_name_arms_match_variant_count() {
        let root = project_root().expect("project root");
        let src = std::fs::read_to_string(root.join("crates/perl-token/src/lib.rs"))
            .expect("perl-token/src/lib.rs must exist");
        let variants = token_kind_variants(&src);
        let arms = token_display_name_arms(&src);
        assert_eq!(
            arms.len(),
            variants.len(),
            "display_name() arms ({}) must cover all TokenKind variants ({}); \
             missing or extra arms indicate coverage drift",
            arms.len(),
            variants.len()
        );
    }

    /// `token_category_counts` totals must equal the full variant count:
    /// no variant may be uncategorised.
    #[test]
    fn all_variants_are_categorised() {
        let root = project_root().expect("project root");
        let src = std::fs::read_to_string(root.join("crates/perl-token/src/lib.rs"))
            .expect("perl-token/src/lib.rs must exist");
        let variants = token_kind_variants(&src);
        let counts = token_category_counts(&src);
        let total: usize = counts.values().sum();
        assert_eq!(
            total,
            variants.len(),
            "category totals ({total}) must cover every variant ({}); \
             uncategorised tokens indicate a missing section header in the enum",
            variants.len()
        );
    }

    /// `collect_token_health_metrics` on the real project root must return PASS
    /// for all status fields (no coverage gaps, lexer+parser deps present).
    /// This test would have caught the fixture drift in CI if run against master.
    #[test]
    fn collect_token_health_metrics_returns_pass_on_live_repo() {
        let root = project_root().expect("project root");
        let metrics = collect_token_health_metrics(&root);
        assert_eq!(
            metrics.metadata_status, "PASS",
            "token metadata_status must be PASS — display_name() coverage has drifted"
        );
        assert!(
            metrics.category_partition_status.starts_with("PASS"),
            "token category_partition_status must be PASS — uncategorised variants found: {}",
            metrics.category_partition_status
        );
        assert!(
            metrics.lexer_parser_conformance_status.starts_with("PASS"),
            "lexer/parser must both depend on perl-token: {}",
            metrics.lexer_parser_conformance_status
        );
        // Variant count must match the fixture constant — if the enum grows, the
        // fixture must be updated too.
        assert_eq!(
            metrics.variant_count, 132,
            "variant_count is {} but fixture expects 132; update token_metrics_fixture()",
            metrics.variant_count
        );
    }
}
