//! HIR lowering coverage inventory.
//!
//! This metric tracks the current bridge from parser AST nodes to crate-local
//! HIR shells. It is intentionally descriptive: it does not score provider
//! behavior and it does not imply that not-yet-modeled constructs are failures.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, Result, eyre};
use perl_parser::NodeKind;
use perl_parser_core::hir::HirKind;
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
    status: HirCoverageStatus,
    hir_kinds: Vec<&'static str>,
    note: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum HirCoverageStatus {
    Lowered,
    DynamicBoundary,
    IntentionallySkipped,
    NotYetModeled,
}

impl HirCoverageStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Lowered => "lowered",
            Self::DynamicBoundary => "dynamic_boundary",
            Self::IntentionallySkipped => "intentionally_skipped",
            Self::NotYetModeled => "not_yet_modeled",
        }
    }

    fn meaning(self) -> &'static str {
        match self {
            Self::Lowered => "Emits one or more HIR items today.",
            Self::DynamicBoundary => {
                "Emits an explicit dynamic-boundary HIR item for unsupported static truth."
            }
            Self::IntentionallySkipped => {
                "Traversal, metadata, or recovery placeholder; no standalone HIR item expected."
            }
            Self::NotYetModeled => "Parser AST construct exists, but HIR has no shell yet.",
        }
    }
}

fn build_artifact() -> Result<HirCoverageArtifact> {
    let rows = coverage_rows();
    validate_rows(&rows)?;

    let mut counts = BTreeMap::new();
    for row in &rows {
        *counts.entry(row.status.as_str()).or_insert(0) += 1;
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

fn coverage_rows() -> Vec<HirCoverageRow> {
    NodeKind::ALL_KIND_NAMES.iter().map(|kind| row_for_ast_kind(kind)).collect()
}

fn row_for_ast_kind(ast_kind: &'static str) -> HirCoverageRow {
    match ast_kind {
        "ArrayLiteral" => {
            lowered(ast_kind, &["LiteralExpr"], "Lowered as aggregate literal shell.")
        }
        "Block" => lowered(
            ast_kind,
            &["BlockShell"],
            "Lowered as block shell and contributes a ScopeGraph block frame.",
        ),
        "Do" => boundary(
            ast_kind,
            "Non-block `do` forms emit `DynamicBoundary`; block bodies traverse.",
        ),
        "Eval" => {
            boundary(ast_kind, "Expression `eval` emits `DynamicBoundary`; block bodies traverse.")
        }
        "Assignment" => boundary(
            ast_kind,
            "Typeglob assignment with a non-static RHS emits `DynamicBoundary`; other assignments traverse.",
        ),
        "FunctionCall" => lowered(
            ast_kind,
            &["CallExpr", "DynamicBoundary", "RequireDecl"],
            "`require` calls lower as `RequireDecl`; coderef calls add a dynamic boundary.",
        ),
        "HashLiteral" => lowered(ast_kind, &["LiteralExpr"], "Lowered as aggregate literal shell."),
        "Identifier" => {
            lowered(ast_kind, &["BarewordExpr"], "Lowered as bareword expression shell.")
        }
        "IndirectCall" => {
            lowered(ast_kind, &["IndirectCallExpr"], "Lowered as indirect-object call shell.")
        }
        "Method" => lowered(
            ast_kind,
            &["MethodDecl"],
            "Lowered as method declaration shell and contributes a method scope frame.",
        ),
        "MethodCall" => lowered(ast_kind, &["MethodCallExpr"], "Lowered as method-call shell."),
        "Number" => lowered(ast_kind, &["LiteralExpr"], "Lowered as numeric literal shell."),
        "Package" => lowered(
            ast_kind,
            &["PackageDecl"],
            "Lowered and updates package context plus package scope.",
        ),
        "String" => lowered(ast_kind, &["LiteralExpr"], "Lowered as string literal shell."),
        "Subroutine" => lowered(
            ast_kind,
            &["SubDecl"],
            "Lowered as sub declaration shell and contributes a subroutine scope frame.",
        ),
        "Undef" => lowered(ast_kind, &["LiteralExpr"], "Lowered as undef literal shell."),
        "Use" => lowered(
            ast_kind,
            &["UseDecl"],
            "Lowered as use declaration shell and records CompileEnvironment directive facts.",
        ),
        "VariableDeclaration" => lowered(
            ast_kind,
            &["VariableDecl"],
            "Lowered as single variable declaration shell and records ScopeGraph bindings.",
        ),
        "VariableListDeclaration" => lowered(
            ast_kind,
            &["VariableDecl"],
            "Lowered as list variable declaration shell and records ScopeGraph bindings.",
        ),
        "Error" => {
            skipped(ast_kind, "Recovered partials are traversed; raw error nodes emit no HIR.")
        }
        "ExpressionStatement" => skipped(ast_kind, "Statement wrapper is traversal-only."),
        "MissingBlock" | "MissingExpression" | "MissingIdentifier" | "MissingStatement"
        | "UnknownRest" => {
            skipped(ast_kind, "Parser recovery placeholder, intentionally no HIR item.")
        }
        "Program" => skipped(ast_kind, "Root wrapper is traversal-only."),
        "Prototype" => skipped(ast_kind, "Captured as declaration metadata."),
        "Signature" | "MandatoryParameter" | "NamedParameter" | "OptionalParameter"
        | "SlurpyParameter" => skipped(
            ast_kind,
            "Captured as ScopeGraph parameter binding metadata; no standalone HIR item.",
        ),
        "Variable" | "VariableWithAttributes" => skipped(
            ast_kind,
            "Consumed by declaration lowering or recorded as ScopeGraph references.",
        ),
        "Format" => not_modeled(
            ast_kind,
            "No HIR shell yet; format declarations contribute a ScopeGraph format frame.",
        ),
        "No" => skipped(
            ast_kind,
            "`no` directives record CompileEnvironment facts; no standalone HIR item yet.",
        ),
        "PhaseBlock" => skipped(
            ast_kind,
            "Phase blocks record CompileEnvironment phase facts and contribute a ScopeGraph phase frame.",
        ),
        "Typeglob" => not_modeled(
            ast_kind,
            "No standalone HIR shell yet; typeglob assignments can contribute StashGraph slots or boundaries.",
        ),
        "Defer" => not_modeled(
            ast_kind,
            "Deferred cleanup needs scope/control-flow modeling before a HIR shell.",
        ),
        _ => not_modeled(ast_kind, "No first-slice HIR shell yet."),
    }
}

fn lowered(
    ast_kind: &'static str,
    hir_kinds: &[&'static str],
    note: &'static str,
) -> HirCoverageRow {
    row(ast_kind, HirCoverageStatus::Lowered, hir_kinds, note)
}

fn boundary(ast_kind: &'static str, note: &'static str) -> HirCoverageRow {
    row(ast_kind, HirCoverageStatus::DynamicBoundary, &["DynamicBoundary"], note)
}

fn skipped(ast_kind: &'static str, note: &'static str) -> HirCoverageRow {
    row(ast_kind, HirCoverageStatus::IntentionallySkipped, &[], note)
}

fn not_modeled(ast_kind: &'static str, note: &'static str) -> HirCoverageRow {
    row(ast_kind, HirCoverageStatus::NotYetModeled, &[], note)
}

fn row(
    ast_kind: &'static str,
    status: HirCoverageStatus,
    hir_kinds: &[&'static str],
    note: &'static str,
) -> HirCoverageRow {
    HirCoverageRow { ast_kind, status, hir_kinds: hir_kinds.to_vec(), note }
}

fn validate_rows(rows: &[HirCoverageRow]) -> Result<()> {
    let ast_kinds = NodeKind::ALL_KIND_NAMES.iter().copied().collect::<BTreeSet<_>>();
    let hir_kinds = HirKind::ALL_KIND_NAMES.iter().copied().collect::<BTreeSet<_>>();

    let mut seen = BTreeSet::new();
    for row in rows {
        if !ast_kinds.contains(row.ast_kind) {
            return Err(eyre!("HIR coverage row references unknown AST kind `{}`", row.ast_kind));
        }
        if !seen.insert(row.ast_kind) {
            return Err(eyre!("duplicate HIR coverage row for AST kind `{}`", row.ast_kind));
        }
        for hir_kind in &row.hir_kinds {
            if !hir_kinds.contains(hir_kind) {
                return Err(eyre!(
                    "HIR coverage row `{}` references unknown HIR kind `{hir_kind}`",
                    row.ast_kind
                ));
            }
        }
    }

    let missing = ast_kinds.difference(&seen).copied().collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(eyre!("missing HIR coverage rows for AST kinds: {}", missing.join(", ")));
    }

    Ok(())
}

fn render_markdown(artifact: &HirCoverageArtifact) -> String {
    let mut out = String::new();
    out.push_str("# HIR Lowering Coverage\n\n");
    out.push_str("> Generated by `cargo xtask metrics hir-coverage --write-status`.\n");
    out.push_str("> Check with `cargo xtask metrics hir-coverage --check`.\n\n");
    out.push_str("This status tracks parser AST construct coverage for the crate-local HIR baseline. It is a compiler-substrate proof surface only; no LSP provider consumes these facts yet.\n\n");
    out.push_str("## Summary\n\n");
    out.push_str("| Status | Count | Meaning |\n");
    out.push_str("| --- | ---: | --- |\n");
    for status in [
        HirCoverageStatus::Lowered,
        HirCoverageStatus::DynamicBoundary,
        HirCoverageStatus::IntentionallySkipped,
        HirCoverageStatus::NotYetModeled,
    ] {
        let count = artifact.counts.get(status.as_str()).copied().unwrap_or_default();
        out.push_str(&format!("| `{}` | {} | {} |\n", status.as_str(), count, status.meaning()));
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
            row.ast_kind,
            row.status.as_str(),
            hir_kinds,
            row.note
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hir_coverage_inventory_covers_all_ast_kinds_once() -> Result<()> {
        let rows = coverage_rows();
        validate_rows(&rows)?;
        assert_eq!(rows.len(), NodeKind::ALL_KIND_NAMES.len());
        Ok(())
    }

    #[test]
    fn hir_coverage_inventory_has_nonempty_status_counts() -> Result<()> {
        let artifact = build_artifact()?;
        for status in [
            HirCoverageStatus::Lowered,
            HirCoverageStatus::DynamicBoundary,
            HirCoverageStatus::IntentionallySkipped,
            HirCoverageStatus::NotYetModeled,
        ] {
            assert!(
                artifact.counts.get(status.as_str()).copied().unwrap_or_default() > 0,
                "expected at least one `{}` HIR coverage row",
                status.as_str()
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
}
