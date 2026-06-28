//! Semantic snapshot schema and stability rail for HIR lowering.
//!
//! # Purpose — SNAPSHOT, not gold
//!
//! This module records **deterministic HIR snapshots** produced by running
//! `lower_ast()` over a small corpus slice. The snapshots prove *stability*:
//! the same source produces the same HIR item sequence across runs and commits.
//!
//! **This is NOT correctness / curated-gold.** Gold corpus entries (independent
//! human labeling of expected semantic facts) are a separate, future schema.
//! See `docs/specs/PLSP-SPEC-0033-semantic-snapshot-rail.md` for the full
//! claim boundary.
//!
//! # KPI
//!
//! The KPI name is `semantic_snapshot_stability_rate` (fraction of snapshots
//! that match the recorded reference). It is NOT named `semantic_gold_pass_rate`.
//!
//! # Design
//!
//! Each snapshot entry records:
//! - `fixture_id` — stable identifier for the source fixture
//! - `source_hash` — within-build-deterministic hash of the source text (hex, lowercase)
//! - `hir_schema_version` — monotonic HIR schema model version string
//! - `hir_summary` — deterministic structural summary of the lowered `HirFile`
//!
//! The summary avoids raw source offsets (which change on whitespace edits) and
//! instead records item-kind sequences, scope counts, and stash metrics that
//! are stable across semantics-preserving reformatting.

use serde::{Deserialize, Serialize};

/// Monotonic HIR schema version string, bumped whenever the lowering model
/// changes in a way that alters snapshot structure.
///
/// Snapshots recorded under a different version are considered stale and must
/// be regenerated before comparison.
pub const HIR_SCHEMA_VERSION: &str = "hir.v1";

/// A deterministic structural summary of one lowered `HirFile`.
///
/// This summary is intentionally coarse-grained: it records item-kind names
/// and graph sizes rather than raw source offsets so that semantics-preserving
/// whitespace changes do not cause drift.
///
/// NOTE: proves stability, not correctness. Curated-gold assertions belong in
/// a separate schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HirSummary {
    /// Total number of HIR items lowered from the file.
    pub item_count: usize,
    /// Sequence of HIR item kind names in stable depth-first source order.
    pub item_kind_sequence: Vec<String>,
    /// Number of scope frames in the scope graph.
    pub scope_count: usize,
    /// Number of bindings recorded in the scope graph.
    pub binding_count: usize,
    /// Number of package stashes in the stash graph.
    pub package_count: usize,
    /// Number of stash glob slots across all packages.
    pub slot_count: usize,
    /// Number of compile-time directive effects recorded.
    pub directive_count: usize,
    /// Number of module requests recorded in the compile environment.
    pub module_request_count: usize,
    /// Number of dynamic-boundary markers.
    pub dynamic_boundary_count: usize,
}

/// One recorded snapshot entry for a single source fixture.
///
/// The `source_hash` + `hir_schema_version` key identifies the snapshot
/// uniquely. A check-mode comparison fails when the recorded summary differs
/// from the freshly computed one, indicating HIR drift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotEntry {
    /// Stable fixture identifier (matches the fixture filename stem).
    pub fixture_id: String,
    /// Within-build-deterministic hash of the fixture source text (lowercase hex, no prefix).
    ///
    /// Computed via [`source_hash`]. **Not** SHA-256, **not** stable across Rust
    /// versions or platforms — suitable only for within-environment drift detection.
    /// FIXME: if cross-version or cross-machine snapshot persistence is ever required,
    /// migrate to a named stable digest (e.g. the `sha2` crate).
    pub source_hash: String,
    /// HIR schema version at snapshot generation time.
    pub hir_schema_version: String,
    /// Deterministic structural summary of the lowered HIR.
    ///
    /// Proves stability — not correctness. See module doc.
    pub hir_summary: HirSummary,
}

/// A snapshot manifest collecting all snapshot entries for a corpus slice.
///
/// Written by the `generate-semantic-snapshot` xtask subcommand in generate
/// mode, and read in check mode to detect HIR drift.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifest {
    /// Schema discriminator — always `"semantic_snapshot.v1"`.
    pub schema: String,
    /// KPI name for this stability rail.
    ///
    /// Always `"semantic_snapshot_stability_rate"`. NOT `"semantic_gold_pass_rate"`.
    pub kpi: String,
    /// Claim boundary: this proves stability, not correctness.
    pub claim_boundary: String,
    /// HIR schema version used for all entries in this manifest.
    pub hir_schema_version: String,
    /// Date of last generation (ISO 8601 `YYYY-MM-DD`).
    pub generated_on: String,
    /// Snapshot entries, one per fixture in the corpus slice.
    pub entries: Vec<SnapshotEntry>,
}

impl SnapshotManifest {
    /// Create a new manifest with the correct schema discriminators.
    pub fn new(generated_on: String) -> Self {
        Self {
            schema: "semantic_snapshot.v1".to_string(),
            kpi: "semantic_snapshot_stability_rate".to_string(),
            claim_boundary: concat!(
                "Snapshot proves deterministic HIR stability only. ",
                "This is NOT curated-gold correctness. ",
                "Curated gold (independent human labeling) is a separate, future schema.",
            )
            .to_string(),
            hir_schema_version: HIR_SCHEMA_VERSION.to_string(),
            generated_on,
            entries: Vec::new(),
        }
    }

    /// Compute the `semantic_snapshot_stability_rate` KPI.
    ///
    /// Returns `(stable_count, total_count, rate)`. A stable snapshot is one
    /// where the freshly computed summary matches the recorded summary.
    pub fn stability_rate(&self, fresh_entries: &[SnapshotEntry]) -> (usize, usize, f64) {
        let total = self.entries.len();
        if total == 0 {
            return (0, 0, 1.0);
        }
        let stable = self
            .entries
            .iter()
            .filter(|recorded| {
                fresh_entries.iter().any(|fresh| {
                    fresh.fixture_id == recorded.fixture_id
                        && fresh.source_hash == recorded.source_hash
                        && fresh.hir_schema_version == recorded.hir_schema_version
                        && fresh.hir_summary == recorded.hir_summary
                })
            })
            .count();
        let rate = stable as f64 / total as f64;
        (stable, total, rate)
    }
}

/// Compute a within-build-deterministic hash of a source string.
///
/// Returns lowercase hex without any prefix. The output is 32 characters
/// (two concatenated 64-bit `DefaultHasher` values encoded as 16-hex-char each).
///
/// # Stability caveats
///
/// `std::collections::hash_map::DefaultHasher` is **not** SHA-256, **not** FNV,
/// and **not** stable across Rust versions or platforms — its output can change
/// between Rust releases or between machines. This function is intentionally
/// scoped to **within-environment drift detection** (the snapshot rail defined by
/// PLSP-SPEC-0033 needs only within-run consistency).
///
/// FIXME: if cross-version or cross-machine snapshot persistence is ever required,
/// replace this with a named stable digest (e.g. the `sha2` crate).
pub fn source_hash(source: &str) -> String {
    // Uses two independent DefaultHasher seeds to spread 128 bits of hash output
    // while staying dependency-free. This is sufficient for within-build drift
    // detection — it is NOT a cryptographic or cross-platform hash.
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Two independent seeds for 128-bit spread (reduces collisions for small
    // fixtures while staying dependency-free).
    let mut h1 = DefaultHasher::new();
    source.hash(&mut h1);
    let d1 = h1.finish();

    let mut h2 = DefaultHasher::new();
    // Mix in a second constant to differentiate h2 from h1.
    0x9e3779b97f4a7c15u64.hash(&mut h2);
    source.hash(&mut h2);
    let d2 = h2.finish();

    format!("{d1:016x}{d2:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_hash_is_deterministic() {
        let s = "package Foo; sub bar { return 1; } 1;";
        assert_eq!(source_hash(s), source_hash(s));
    }

    #[test]
    fn source_hash_differs_for_different_sources() {
        let a = source_hash("my $x = 1;");
        let b = source_hash("my $y = 2;");
        assert_ne!(a, b, "distinct sources must produce distinct hashes");
    }

    #[test]
    fn source_hash_len_is_32() {
        // Two 64-bit values → 16 hex chars each → 32 total.
        assert_eq!(source_hash("hello").len(), 32);
    }

    #[test]
    fn manifest_schema_discriminators() {
        let m = SnapshotManifest::new("2026-06-21".to_string());
        assert_eq!(m.schema, "semantic_snapshot.v1");
        assert_eq!(m.kpi, "semantic_snapshot_stability_rate");
        assert_eq!(m.hir_schema_version, HIR_SCHEMA_VERSION);
        assert!(m.claim_boundary.contains("stability"), "claim boundary must mention stability");
        assert!(m.claim_boundary.contains("NOT"), "claim boundary must disclaim gold/correctness");
    }

    #[test]
    fn stability_rate_empty_manifest() {
        let m = SnapshotManifest::new("2026-06-21".to_string());
        let (stable, total, rate) = m.stability_rate(&[]);
        assert_eq!(stable, 0);
        assert_eq!(total, 0);
        assert!((rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stability_rate_all_match() {
        let entry = SnapshotEntry {
            fixture_id: "foo".to_string(),
            source_hash: "abc".to_string(),
            hir_schema_version: HIR_SCHEMA_VERSION.to_string(),
            hir_summary: HirSummary {
                item_count: 2,
                item_kind_sequence: vec!["SubDecl".to_string(), "LiteralExpr".to_string()],
                scope_count: 1,
                binding_count: 0,
                package_count: 0,
                slot_count: 0,
                directive_count: 0,
                module_request_count: 0,
                dynamic_boundary_count: 0,
            },
        };
        let mut m = SnapshotManifest::new("2026-06-21".to_string());
        m.entries.push(entry.clone());

        let (stable, total, rate) = m.stability_rate(&[entry]);
        assert_eq!(stable, 1);
        assert_eq!(total, 1);
        assert!((rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stability_rate_mismatch_on_drift() {
        let recorded = SnapshotEntry {
            fixture_id: "foo".to_string(),
            source_hash: "abc".to_string(),
            hir_schema_version: HIR_SCHEMA_VERSION.to_string(),
            hir_summary: HirSummary {
                item_count: 2,
                item_kind_sequence: vec!["SubDecl".to_string()],
                scope_count: 1,
                binding_count: 0,
                package_count: 0,
                slot_count: 0,
                directive_count: 0,
                module_request_count: 0,
                dynamic_boundary_count: 0,
            },
        };
        let drifted = SnapshotEntry {
            hir_summary: HirSummary {
                item_count: 5, // changed
                item_kind_sequence: vec!["SubDecl".to_string(), "PackageDecl".to_string()],
                scope_count: 2,
                ..recorded.hir_summary.clone()
            },
            ..recorded.clone()
        };
        let mut m = SnapshotManifest::new("2026-06-21".to_string());
        m.entries.push(recorded);
        let (stable, total, rate) = m.stability_rate(&[drifted]);
        assert_eq!(stable, 0);
        assert_eq!(total, 1);
        assert!((rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn snapshot_entry_round_trips_json() {
        let entry = SnapshotEntry {
            fixture_id: "sub_basic".to_string(),
            source_hash: source_hash("package Foo; sub bar { 1; } 1;"),
            hir_schema_version: HIR_SCHEMA_VERSION.to_string(),
            hir_summary: HirSummary {
                item_count: 3,
                item_kind_sequence: vec![
                    "PackageDecl".to_string(),
                    "SubDecl".to_string(),
                    "LiteralExpr".to_string(),
                ],
                scope_count: 2,
                binding_count: 0,
                package_count: 1,
                slot_count: 1,
                directive_count: 0,
                module_request_count: 0,
                dynamic_boundary_count: 0,
            },
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: SnapshotEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, entry);
    }
}
