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
    pub variant_count: usize,
    pub metadata_coverage_count: usize,
    pub display_name_coverage_count: usize,
    pub metadata_status: &'static str,
    pub category_partition_status: String,
    pub lexer_parser_conformance_status: String,
    pub runtime_dependency_count: usize,
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
    let Some(display_start) = source.find("impl TokenKind") else {
        return Vec::new();
    };
    let enum_body = &source[enum_start..display_start];
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
    let Ok(re) = Regex::new(r"TokenKind::([A-Z][A-Za-z0-9]*)\s*=>") else {
        return Vec::new();
    };
    re.captures_iter(source)
        .filter_map(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .collect()
}

fn token_category_counts(source: &str) -> std::collections::BTreeMap<&'static str, usize> {
    let Some(enum_start) = source.find("pub enum TokenKind") else {
        return std::collections::BTreeMap::new();
    };
    let Some(display_start) = source.find("impl TokenKind") else {
        return std::collections::BTreeMap::new();
    };
    let enum_body = &source[enum_start..display_start];
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
