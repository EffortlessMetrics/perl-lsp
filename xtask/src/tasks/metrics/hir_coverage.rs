//! HIR lowering coverage inventory.
//!
//! This metric tracks the current bridge from parser AST nodes to crate-local
//! HIR shells. It is intentionally descriptive: it does not score provider
//! behavior and it does not imply that not-yet-modeled constructs are failures.
//!
//! ## Single source of truth
//!
//! This module no longer contains its own AST-kind classification table.
//! All lowering dispositions are read from
//! [`perl_parser_core::hir::disposition::disposition_for`] — the shared
//! registry that is also consumed by the `hir_lowering_completeness_tests`
//! integration tests.  Changes to how any AST kind is lowered must be made in
//! `disposition.rs`; the coverage check and the completeness gate will both
//! reflect the update automatically.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, Result, eyre};
use perl_parser::NodeKind;
use perl_parser_core::hir::{HirKind, disposition};
use serde::Serialize;

use crate::utils::project_root;

const STATUS_PATH: &str = "docs/project/status/hir_lowering.md";
const GENERATED_BY: &str = "cargo xtask metrics hir-coverage";

/// Run `cargo xtask metrics hir-coverage`.
pub fn run(json: bool, output: Option<PathBuf>, write_status: bool, check: bool) -> Result<()> {
    let root = project_root()?;
    let artifact = build_artifact()?;
    let markdown = render_markdown(&artifact);

    if check {
        let status_path = root.join(STATUS_PATH);
        let existing = fs::read_to_string(&status_path)
            .with_context(|| format!("reading {}", status_path.display()))?;
        if existing != markdown {
            return Err(eyre!(
                "{STATUS_PATH} is out of date; run `cargo xtask metrics hir-coverage --write-status`"
            ));
        }
        println!("hir-coverage: status doc is current");
        return Ok(());
    }

    if write_status {
        let status_path = root.join(STATUS_PATH);
        write_file(&status_path, &markdown)?;
        println!("hir coverage status written: {}", status_path.display());
    }

    if json {
        let output_path = output.unwrap_or_else(|| root.join("target/metrics/hir_coverage.json"));
        let json = serde_json::to_string_pretty(&artifact).context("serializing HIR coverage")?;
        write_file(&output_path, &(json + "\n"))?;
        println!("hir coverage receipt written: {}", output_path.display());
    }

    if !json && !write_status {
        print!("{markdown}");
    }

    Ok(())
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    let parent = path.parent().ok_or_else(|| eyre!("output path has no parent"))?;
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    fs::write(path, content).with_context(|| format!("writing {}", path.display()))
}

#[derive(Debug, Clone, Serialize)]
struct HirCoverageArtifact {
    schema_version: u32,
    subsystem: &'static str,
    generated_by: &'static str,
    total_ast_kinds: usize,
    total_hir_kinds: usize,
    counts: BTreeMap<&'static str, usize>,
    rows: Vec<HirCoverageRow>,
}

#[derive(Debug, Clone, Serialize)]
struct HirCoverageRow {
    ast_kind: &'static str,
    status: &'static str,
    hir_kinds: Vec<&'static str>,
    note: &'static str,
}

fn build_artifact() -> Result<HirCoverageArtifact> {
    let rows = coverage_rows()?;

    let mut counts = BTreeMap::new();
    for row in &rows {
        *counts.entry(row.status).or_insert(0) += 1;
    }

    Ok(HirCoverageArtifact {
        schema_version: 1,
        subsystem: "hir_coverage",
        generated_by: GENERATED_BY,
        total_ast_kinds: NodeKind::ALL_KIND_NAMES.len(),
        total_hir_kinds: HirKind::ALL_KIND_NAMES.len(),
        counts,
        rows,
    })
}

/// Validate that the disposition registry covers every AST kind.
///
/// Returns `Err` if any names in `missing` are non-empty, describing which
/// AST kinds lack a registered disposition.  Extracted so that tests can
/// exercise the error path with synthetic missing-name lists without modifying
/// the global registry.
fn validate_registry_completeness(missing: &[&str]) -> Result<()> {
    if !missing.is_empty() {
        return Err(eyre!(
            "HIR disposition registry is incomplete; missing entries for AST kinds: {}\n\
             Add them to `disposition_for()` in \
             `crates/perl-parser-core/src/hir/disposition.rs`.",
            missing.join(", ")
        ));
    }
    Ok(())
}

/// Validate that every HIR kind referenced by `hir_refs` exists in `valid_hir_kinds`.
///
/// `ast_kind` is the registry key being validated (used only in the error message).
/// Extracted so that tests can exercise the error path with a synthetic unknown
/// HIR kind name without modifying any global table.
fn validate_hir_kind_ref(
    ast_kind: &str,
    hir_kind: &str,
    valid_hir_kinds: &BTreeSet<&str>,
) -> Result<()> {
    if !valid_hir_kinds.contains(hir_kind) {
        return Err(eyre!(
            "disposition registry for `{ast_kind}` references unknown HIR kind \
             `{hir_kind}`; update `hir_kinds_for()` in \
             `crates/perl-parser-core/src/hir/disposition.rs`."
        ));
    }
    Ok(())
}

fn coverage_rows() -> Result<Vec<HirCoverageRow>> {
    // Guard: the shared registry must cover all AST kinds before we build rows.
    let missing = disposition::missing_dispositions();
    validate_registry_completeness(&missing)?;

    // Validate that the HIR kinds referenced in the registry actually exist.
    let valid_hir_kinds: BTreeSet<&str> = HirKind::ALL_KIND_NAMES.iter().copied().collect();
    for &ast_kind in NodeKind::ALL_KIND_NAMES {
        for &hir_kind in disposition::hir_kinds_for(ast_kind) {
            validate_hir_kind_ref(ast_kind, hir_kind, &valid_hir_kinds)?;
        }
    }

    let rows = NodeKind::ALL_KIND_NAMES
        .iter()
        .map(|&ast_kind| {
            let d = disposition::disposition_for(ast_kind)
                .unwrap_or_else(|| unreachable!("missing_dispositions() guard above ensures this"));
            let status = d.legacy_category().as_str();
            let hir_kinds = disposition::hir_kinds_for(ast_kind).to_vec();
            HirCoverageRow { ast_kind, status, hir_kinds, note: d.note }
        })
        .collect();

    Ok(rows)
}

fn render_markdown(artifact: &HirCoverageArtifact) -> String {
    let status_order = [
        disposition::LegacyCategory::Lowered.as_str(),
        disposition::LegacyCategory::DynamicBoundary.as_str(),
        disposition::LegacyCategory::IntentionallySkipped.as_str(),
        disposition::LegacyCategory::NotYetModeled.as_str(),
    ];

    let mut out = String::new();
    out.push_str("# HIR Lowering Coverage\n\n");
    out.push_str("> Generated by `cargo xtask metrics hir-coverage --write-status`.\n");
    out.push_str("> Check with `cargo xtask metrics hir-coverage --check`.\n\n");
    out.push_str("This status tracks parser AST construct coverage for the crate-local HIR baseline. It is a compiler-substrate proof surface only; no LSP provider consumes these facts yet.\n\n");
    out.push_str("## Summary\n\n");
    out.push_str("| Status | Count | Meaning |\n");
    out.push_str("| --- | ---: | --- |\n");
    for &status_str in &status_order {
        let category = legacy_category_from_str(status_str);
        let count = artifact.counts.get(status_str).copied().unwrap_or_default();
        out.push_str(&format!("| `{}` | {} | {} |\n", status_str, count, category.meaning()));
    }
    out.push('\n');
    out.push_str(&format!(
        "AST kinds tracked: `{}`. HIR construct kinds tracked: `{}`.\n\n",
        artifact.total_ast_kinds, artifact.total_hir_kinds
    ));
    out.push_str("## Inventory\n\n");
    out.push_str("| AST NodeKind | Status | HIR kinds | Note |\n");
    out.push_str("| --- | --- | --- | --- |\n");
    for row in &artifact.rows {
        let hir_kinds = if row.hir_kinds.is_empty() {
            "-".to_string()
        } else {
            row.hir_kinds.iter().map(|kind| format!("`{kind}`")).collect::<Vec<_>>().join(", ")
        };
        out.push_str(&format!(
            "| `{}` | `{}` | {} | {} |\n",
            row.ast_kind, row.status, hir_kinds, row.note
        ));
    }
    out
}

fn legacy_category_from_str(s: &str) -> disposition::LegacyCategory {
    match s {
        "lowered" => disposition::LegacyCategory::Lowered,
        "dynamic_boundary" => disposition::LegacyCategory::DynamicBoundary,
        "intentionally_skipped" => disposition::LegacyCategory::IntentionallySkipped,
        _ => disposition::LegacyCategory::NotYetModeled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hir_coverage_inventory_covers_all_ast_kinds_once() -> Result<()> {
        let rows = coverage_rows()?;
        assert_eq!(rows.len(), NodeKind::ALL_KIND_NAMES.len());
        Ok(())
    }

    #[test]
    fn hir_coverage_inventory_has_nonempty_status_counts() -> Result<()> {
        let artifact = build_artifact()?;
        for status_str in [
            disposition::LegacyCategory::Lowered.as_str(),
            disposition::LegacyCategory::DynamicBoundary.as_str(),
            disposition::LegacyCategory::IntentionallySkipped.as_str(),
            disposition::LegacyCategory::NotYetModeled.as_str(),
        ] {
            assert!(
                artifact.counts.get(status_str).copied().unwrap_or_default() > 0,
                "expected at least one `{status_str}` HIR coverage row"
            );
        }
        Ok(())
    }

    #[test]
    fn hir_coverage_status_mentions_no_provider_cutover() -> Result<()> {
        let artifact = build_artifact()?;
        let markdown = render_markdown(&artifact);
        assert!(markdown.contains("no LSP provider consumes these facts yet"));
        assert!(markdown.contains("AST NodeKind"));
        Ok(())
    }

    #[test]
    fn hir_coverage_registry_has_no_missing_dispositions() {
        let missing = disposition::missing_dispositions();
        assert!(
            missing.is_empty(),
            "HIR disposition registry is missing entries for: {:?}",
            missing
        );
    }

    #[test]
    fn hir_coverage_disposition_registry_agrees_with_hir_kinds() -> Result<()> {
        let valid_hir_kinds: BTreeSet<&str> = HirKind::ALL_KIND_NAMES.iter().copied().collect();
        for &ast_kind in NodeKind::ALL_KIND_NAMES {
            for &hir_kind in disposition::hir_kinds_for(ast_kind) {
                assert!(
                    valid_hir_kinds.contains(hir_kind),
                    "disposition registry for `{ast_kind}` references unknown HIR kind `{hir_kind}`"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn legacy_category_from_str_round_trips_all_slugs() {
        // Verify that `legacy_category_from_str` returns the correct variant for
        // each stable slug produced by `LegacyCategory::as_str()`.  This also
        // exercises the `_` fallthrough arm which handles any unknown string.
        use disposition::LegacyCategory;
        assert_eq!(legacy_category_from_str("lowered"), LegacyCategory::Lowered);
        assert_eq!(legacy_category_from_str("dynamic_boundary"), LegacyCategory::DynamicBoundary);
        assert_eq!(
            legacy_category_from_str("intentionally_skipped"),
            LegacyCategory::IntentionallySkipped
        );
        // The `_ => NotYetModeled` arm.
        assert_eq!(
            legacy_category_from_str("not_yet_modeled"),
            LegacyCategory::NotYetModeled,
            "not_yet_modeled slug must fall through to NotYetModeled"
        );
        assert_eq!(
            legacy_category_from_str("anything_else"),
            LegacyCategory::NotYetModeled,
            "unknown slug must default to NotYetModeled via _ arm"
        );
    }

    #[test]
    fn hir_coverage_row_fields_are_populated() -> Result<()> {
        // Verify that every row returned by `coverage_rows()` has a non-empty
        // `ast_kind`, a non-empty `status` slug, and a non-empty `note`.
        // This exercises the inner mapping closure in `coverage_rows()`.
        let rows = coverage_rows()?;
        assert!(!rows.is_empty(), "coverage_rows() must return at least one row");
        for row in &rows {
            assert!(!row.ast_kind.is_empty(), "every row must have a non-empty ast_kind");
            assert!(!row.status.is_empty(), "every row must have a non-empty status slug");
            assert!(!row.note.is_empty(), "every row must have a non-empty note");
        }
        // Spot-check a representative row.
        let pkg_row = rows.iter().find(|r| r.ast_kind == "Package");
        assert!(pkg_row.is_some(), "Package must appear in coverage rows");
        let pkg = pkg_row.unwrap();
        assert_eq!(pkg.status, "lowered", "Package must have lowered status");
        assert!(
            pkg.hir_kinds.contains(&"PackageDecl"),
            "Package row must list PackageDecl HIR kind"
        );
        Ok(())
    }

    #[test]
    fn hir_coverage_markdown_contains_all_status_slugs() -> Result<()> {
        // Verify that the generated markdown contains all four status slugs in the
        // summary table — exercising the `render_markdown()` loop over `status_order`.
        let artifact = build_artifact()?;
        let markdown = render_markdown(&artifact);
        assert!(markdown.contains("lowered"), "markdown must mention 'lowered'");
        assert!(markdown.contains("dynamic_boundary"), "markdown must mention 'dynamic_boundary'");
        assert!(
            markdown.contains("intentionally_skipped"),
            "markdown must mention 'intentionally_skipped'"
        );
        assert!(markdown.contains("not_yet_modeled"), "markdown must mention 'not_yet_modeled'");
        // Verify the inventory table header is present.
        assert!(markdown.contains("| AST NodeKind | Status | HIR kinds | Note |"));
        // Verify the summary header is present.
        assert!(markdown.contains("## Summary"));
        assert!(markdown.contains("## Inventory"));
        Ok(())
    }

    #[test]
    fn hir_coverage_artifact_total_counts_are_consistent() -> Result<()> {
        // Verify that the sum of all status counts equals the total number of AST kinds
        // tracked — the `schema_version`, `total_ast_kinds`, and `total_hir_kinds`
        // fields must all be plausible.
        let artifact = build_artifact()?;
        let count_sum: usize = artifact.counts.values().sum();
        assert_eq!(
            count_sum, artifact.total_ast_kinds,
            "sum of per-status counts must equal total_ast_kinds"
        );
        assert_eq!(
            artifact.total_ast_kinds,
            NodeKind::ALL_KIND_NAMES.len(),
            "total_ast_kinds must equal ALL_KIND_NAMES length"
        );
        assert_eq!(artifact.schema_version, 1, "schema_version must be 1");
        assert_eq!(artifact.subsystem, "hir_coverage", "subsystem slug must be hir_coverage");
        assert!(artifact.total_hir_kinds > 0, "total_hir_kinds must be positive");
        Ok(())
    }

    #[test]
    fn validate_registry_completeness_errors_on_missing_entries() {
        // Drive the extracted error path: when missing is non-empty,
        // validate_registry_completeness() must return Err with a message that
        // names the missing AST kinds.  This test would FAIL if the error path
        // were removed or the message text changed.
        let missing = vec!["FakeKindA", "FakeKindB"];
        let err = validate_registry_completeness(&missing).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("HIR disposition registry is incomplete"),
            "error message must mention 'HIR disposition registry is incomplete'; got: {msg}"
        );
        assert!(msg.contains("FakeKindA"), "error message must name the missing kind; got: {msg}");
        assert!(msg.contains("FakeKindB"), "error message must name all missing kinds; got: {msg}");
        // Confirm the happy path: empty slice is Ok(()).
        assert!(validate_registry_completeness(&[]).is_ok(), "empty missing list must be Ok");
    }

    #[test]
    fn validate_hir_kind_ref_errors_on_unknown_hir_kind() {
        // Drive the extracted error path: when a HIR kind name is not in the
        // valid set, validate_hir_kind_ref() must return Err naming both the
        // ast_kind and the unknown hir_kind.  This test would FAIL if the
        // error path were removed or the message text changed.
        let valid: BTreeSet<&str> = ["PackageDecl", "SubDecl"].iter().copied().collect();
        let err = validate_hir_kind_ref("FakeAstKind", "NonExistentHirKind", &valid).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("FakeAstKind"), "error message must name the ast_kind; got: {msg}");
        assert!(
            msg.contains("NonExistentHirKind"),
            "error message must name the unknown hir_kind; got: {msg}"
        );
        assert!(
            msg.contains("references unknown HIR kind"),
            "error message must include 'references unknown HIR kind'; got: {msg}"
        );
        // Confirm the happy path: a known HIR kind is Ok(()).
        assert!(
            validate_hir_kind_ref("FakeAstKind", "PackageDecl", &valid).is_ok(),
            "known HIR kind must be Ok"
        );
    }
}
