//! Token subsystem health metrics collection.
//!
//! Collects variant counts, metadata coverage, categorization, and performance metrics
//! from perl-token for inclusion in project status reporting.

use std::path::Path;

use serde::Deserialize;

mod source;

use source::{
    count_runtime_dependencies, crate_depends_on_token, read_token_baseline,
    read_token_kind_source, read_token_perf_scorecard, token_category_counts,
    token_display_name_arms, token_kind_variants,
};

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
    let token_lib = read_token_kind_source(root);
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
                ("token_kind_display_name", "display_name"),
                ("token_kind_category_predicates", "category predicates"),
                ("token_clone", "clone"),
                ("token_new_short", "new short"),
                ("token_new_long", "new long"),
                ("lexer_to_parser_token_conversion", "lexer->parser"),
            ]
            .into_iter()
            .filter_map(|(key, label)| {
                scorecard.metrics.get(key).map(|metric| {
                    format!("{label}: p50 {} ns / p95 {} ns", metric.median_ns, metric.p95_ns)
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
        let src = read_token_kind_source(&root);
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
                name.chars().next().is_some_and(|c| c.is_uppercase()),
                "extracted variant {name:?} does not start with uppercase"
            );
        }
        // Spot-check: known TokenCategory variants must NOT appear in the list.
        // These were incorrectly included before the brace-tracking boundary fix.
        // `TokenKind::Identifier` is a legitimate token kind, so it is not a
        // spurious category-name sentinel here even though `TokenCategory` also
        // has an `Identifier` variant.
        for spurious in &["Keyword", "Operator", "Delimiter", "Literal", "Special"] {
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
        let src = read_token_kind_source(&root);
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
        let src = read_token_kind_source(&root);
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

    /// The committed token scorecard uses benchmark names emitted by
    /// `crates/perl-token/benches/support/perf_scorecard.rs`. The status reader
    /// must consume those names directly so parser status does not report token
    /// performance as unverified when the artifact exists.
    #[test]
    fn collect_token_health_metrics_reads_committed_perf_scorecard_keys() {
        let root = project_root().expect("project root");
        let metrics = collect_token_health_metrics(&root);

        assert!(
            metrics.performance_row.starts_with("PASS ("),
            "token performance row must be verified from committed scorecard; got: {}",
            metrics.performance_row
        );
        for label in [
            "category predicates",
            "clone",
            "display_name",
            "lexer->parser",
            "new long",
            "new short",
        ] {
            assert!(
                metrics.performance_row.contains(label),
                "token performance row must include {label:?}; got: {}",
                metrics.performance_row
            );
        }
    }

    #[test]
    fn collect_token_health_metrics_reads_split_kind_module() -> std::io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();
        let token_src = root.join("crates/perl-token/src");
        std::fs::create_dir_all(&token_src)?;
        std::fs::create_dir_all(root.join("crates/perl-lexer"))?;
        std::fs::create_dir_all(root.join("crates/perl-parser-core"))?;

        std::fs::write(
            token_src.join("lib.rs"),
            "mod kind;\npub use kind::{TokenCategory, TokenKind, TokenKindMetadata};\n",
        )?;
        std::fs::write(
            token_src.join("kind.rs"),
            r#"
pub enum TokenKind {
    // ===== Keywords =====
    My,
    // ===== Operators =====
    Plus,
}

#[non_exhaustive]
pub enum TokenCategory {
    Keyword,
    Operator,
}

#[non_exhaustive]
pub struct TokenKindMetadata {
    pub category: TokenCategory,
    pub display_name: &'static str,
}

impl TokenKind {
    pub fn display_name(self) -> &'static str {
        match self {
            TokenKind::My => "'my'",
            TokenKind::Plus => "'+'",
        }
    }
}
"#,
        )?;
        std::fs::write(root.join("crates/perl-lexer/Cargo.toml"), "perl-token = {}\n")?;
        std::fs::write(root.join("crates/perl-parser-core/Cargo.toml"), "perl-token = {}\n")?;
        std::fs::write(root.join("crates/perl-token/Cargo.toml"), "[dependencies]\n")?;

        let metrics = collect_token_health_metrics(root);

        assert_eq!(metrics.variant_count, 2);
        assert_eq!(metrics.metadata_coverage_count, 2);
        assert_eq!(metrics.display_name_coverage_count, 2);
        assert_eq!(metrics.metadata_status, "PASS");
        assert_eq!(
            metrics.category_partition_status,
            "PASS (2 tokens partitioned across canonical groups)"
        );

        Ok(())
    }
}
