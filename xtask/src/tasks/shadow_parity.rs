//! Shadow-parity measurement — pre-push shell selector vs `ci_scope` Rust
//! classifier (#3985 Slice 3B).
//!
//! **MEASUREMENT ONLY.** This module does not select, run, skip, or route
//! any check. It reproduces the `hooks/pre-push` doc-only/single-crate
//! taxonomy as a pure Rust function (`shell_verdict`) purely so it can be
//! compared, scenario by scenario, against the real
//! `ci_scope::classify_files` output (`rust_verdict`) — the shared Rust
//! classifier that #3985 Slices 1-3A already repointed `ci_scope`, `gates`,
//! `targeted_checks`, and the pre-push new-branch base resolution onto.
//! Nothing in the repository consumes this module's output for routing;
//! `hooks/pre-push` and `ci_scope.rs` are both untouched by this slice.
//!
//! The open question this measures evidence for (recorded on #3985, not
//! decided here): should the pre-push shell fast-path be repointed onto
//! `ci_scope`'s taxonomy? The two selectors disagree on real inputs
//! (workflow files, `justfile`, crate manifests) — see
//! `docs/reference/CHANGE_SET_SHADOW_PARITY.md` for the generated report
//! and per-scenario agreement matrix.
//!
//! A second, independent measurement fell out of building this harness:
//! `ci_scope::classify_files`'s own production call sites already disagree
//! on `cargo metadata`'s `--no-deps` flag (`ci_scope.rs`'s own `run()`
//! requests the full resolve graph; `gates::compute_scope_output` and
//! `ci_pr_summary::run` pass `--no-deps`), which changes whether the
//! reverse-dependency closure is populated at all. `run()` below measures
//! both modes and the report shows both — this is a real, pre-existing
//! wrinkle inside the Rust classifier, orthogonal to the shell-vs-Rust
//! taxonomy question, and is reported (not fixed) here.

use std::collections::BTreeSet;

use color_eyre::eyre::{Context, Result};
use serde::Serialize;

use crate::tasks::ci_scope;
use crate::utils;

// ---------------------------------------------------------------------------
// Scenario corpus
// ---------------------------------------------------------------------------

/// One representative changed-path scenario. `changed_paths` is a synthetic
/// list of paths — no real git history is needed since both classifiers
/// under comparison are pure functions of a changed-path list (plus, for
/// `ci_scope`, `cargo metadata`).
#[derive(Debug, Clone)]
pub struct Scenario {
    pub name: &'static str,
    pub description: &'static str,
    pub changed_paths: &'static [&'static str],
}

/// The 11 representative scenarios named in #3985 Slice 3B.
pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "docs-only",
            description: "Prose-only edit (README + a docs/reference page).",
            changed_paths: &["docs/reference/STABILITY.md", "README.md"],
        },
        Scenario {
            name: "rust-leaf-crate",
            description: "Single source file in a leaf crate with no architectural widener.",
            changed_paths: &["crates/perl-pod/src/lib.rs"],
        },
        Scenario {
            name: "test-only",
            description: "Single test file in the same leaf crate (no widener).",
            changed_paths: &["crates/perl-pod/tests/pod_parsing_test.rs"],
        },
        Scenario {
            name: "several-crates",
            description: "Three unrelated facade crates changed in one push.",
            changed_paths: &[
                "crates/perl-parser/src/lib.rs",
                "crates/perl-lsp-rs/src/lib.rs",
                "crates/perl-dap/src/lib.rs",
            ],
        },
        Scenario {
            name: "workflow-only",
            description: "A GitHub Actions workflow file, no crate touched.",
            changed_paths: &[".github/workflows/ci.yml"],
        },
        Scenario {
            name: "extension-only",
            description: "VS Code extension TypeScript source, no Cargo crate touched.",
            changed_paths: &["vscode-extension/src/extension.ts"],
        },
        Scenario {
            name: "deletion",
            description: "A deleted file inside a leaf crate (git diff --name-only reports \
                the path with no status letter — same as an add/modify at this layer).",
            changed_paths: &["crates/perl-pod/src/deprecated_helper.rs"],
        },
        Scenario {
            name: "rename-cross-crate",
            description: "A rename/type-change that moves a file from one crate to another \
                (git diff --name-only without -M reports both the old and new path).",
            changed_paths: &[
                "crates/perl-pod/src/legacy.rs",
                "crates/perl-parser/src/pod_bridge.rs",
            ],
        },
        Scenario {
            name: "shared-foundation-crate",
            description: "Single source file in perl-parser — a shared foundation crate with \
                an architectural widener rule.",
            changed_paths: &["crates/perl-parser/src/statement.rs"],
        },
        Scenario {
            name: "new-branch-mixed-diff",
            description: "A broad, mixed new-branch diff: a foundation crate, a leaf crate, \
                docs, and a workflow file all in one push.",
            changed_paths: &[
                "crates/perl-parser/src/lib.rs",
                "crates/perl-pod/src/lib.rs",
                "docs/reference/STABILITY.md",
                ".github/workflows/ci.yml",
            ],
        },
        Scenario {
            name: "existing-pr-update",
            description: "A small incremental follow-up commit on an already-open PR: one \
                crate's source file plus its test.",
            changed_paths: &[
                "crates/perl-lsp-rs/src/providers/hover.rs",
                "crates/perl-lsp-rs/tests/hover_test.rs",
            ],
        },
    ]
}

// ---------------------------------------------------------------------------
// Shell selector — reproduced (not executed) from hooks/pre-push
// ---------------------------------------------------------------------------

/// The shell pre-push selector's routing verdict, reproduced in Rust from
/// `hooks/pre-push` lines 77-206 (doc-only glob, then single-crate scope,
/// else full `pr-fast`). This is a measurement-only reimplementation —
/// `hooks/pre-push` itself is not invoked and not modified by this module.
///
/// **Provenance note**: This enum models the shell's *taxonomy* logic
/// (`hooks/pre-push:77-206` glob rules and branch detection), but NOT the
/// `resolve-package-name` resolution step the shell applies at line 182
/// (converting directory basename → Cargo package name, e.g.
/// `perl-lsp` → `perl-lsp-rs` per issue #4512). For the current corpus
/// scenarios in this module, all crate directory basenames match their
/// Cargo package names, so the verdicts are correct despite this
/// simplification. When adding scenarios with basename ≠ package-name,
/// the `SingleCrate` variant and matrix data must be updated accordingly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ShellVerdict {
    /// Every changed path matched the doc-only glob (`hooks/pre-push:130`)
    /// — code gates are skipped entirely.
    DocOnlySkip,
    /// Every non-doc-only changed path lives under exactly one
    /// `crates/<name>/` — the targeted `cargo fmt/clippy/test -p <name>`
    /// gate runs (`hooks/pre-push:172-206`). Note: `<name>` here is the
    /// directory basename, not the resolved Cargo package name (see
    /// provenance note above).
    SingleCrate(String),
    /// Ambiguous, multi-crate, or non-crate change — falls back to the full
    /// `nix develop -c just pr-fast` gate (`hooks/pre-push:208+`).
    FullGate,
}

impl ShellVerdict {
    pub fn label(&self) -> String {
        match self {
            ShellVerdict::DocOnlySkip => "doc-only-skip".to_string(),
            ShellVerdict::SingleCrate(name) => format!("single-crate({name})"),
            ShellVerdict::FullGate => "full-gate".to_string(),
        }
    }
}

/// Mirrors the shell `case` glob at `hooks/pre-push:130`:
/// `*.md|*.txt|LICENSE*|CHANGELOG*|docs/*|.github/ISSUE_TEMPLATE/*|*/LICENSE*`.
fn is_doc_only_path(path: &str) -> bool {
    path.ends_with(".md")
        || path.ends_with(".txt")
        || path.starts_with("LICENSE")
        || path.starts_with("CHANGELOG")
        || path.starts_with("docs/")
        || path.starts_with(".github/ISSUE_TEMPLATE/")
        || path.contains("/LICENSE")
}

/// Mirrors the shell `case` glob at `hooks/pre-push:136`: `crates/*/*` —
/// the path must be nested at least one level under `crates/<name>/`.
fn shell_single_crate_name(path: &str) -> Option<&str> {
    let rest = path.strip_prefix("crates/")?;
    let (name, _remainder) = rest.split_once('/')?;
    (!name.is_empty()).then_some(name)
}

/// Compute the shell selector's verdict for a changed-path set.
pub fn shell_verdict(paths: &[&str]) -> ShellVerdict {
    if paths.iter().all(|p| is_doc_only_path(p)) {
        return ShellVerdict::DocOnlySkip;
    }

    let mut crate_names: Vec<String> = Vec::new();
    let mut all_under_crates = true;
    for path in paths {
        if is_doc_only_path(path) {
            continue;
        }
        match shell_single_crate_name(path) {
            Some(name) => {
                if !crate_names.iter().any(|n| n == name) {
                    crate_names.push(name.to_string());
                }
            }
            None => all_under_crates = false,
        }
    }

    if all_under_crates && crate_names.len() == 1 {
        ShellVerdict::SingleCrate(crate_names.remove(0))
    } else {
        ShellVerdict::FullGate
    }
}

// ---------------------------------------------------------------------------
// Rust classifier — thin extraction over ci_scope::classify_files
// ---------------------------------------------------------------------------

/// A digest of `ci_scope::classify_files`'s output relevant to the
/// shell-vs-Rust comparison. `touched_crates` is the union of every
/// selected lane's `scope` field (test/clippy-scoped crates plus
/// architectural-widener targets) — the full set of crates `ci_scope`
/// would direct *some* proof at for this diff.
#[derive(Debug, Clone, Serialize)]
pub struct RustVerdict {
    pub diff_class: String,
    pub direct_crates: Vec<String>,
    pub reverse_dep_crates: Vec<String>,
    pub selected_lane_names: Vec<String>,
    pub touched_crates: Vec<String>,
}

/// Run the real `ci_scope::classify_files` classifier against a changed-path
/// set and reduce its output to the fields relevant to shadow-parity
/// comparison. `ci_scope::classify_files` itself is untouched by this
/// module — this is a read-only consumer, same as `gates.rs`/`ci_pr_summary.rs`.
pub fn rust_verdict(
    paths: &[&str],
    metadata: &serde_json::Value,
    workspace_root: &str,
) -> Result<RustVerdict> {
    let owned: Vec<String> = paths.iter().map(|&p| p.to_string()).collect();
    let output = ci_scope::classify_files(&owned, metadata, workspace_root)
        .context("ci_scope::classify_files failed")?;

    let mut touched_crates: BTreeSet<String> = BTreeSet::new();
    for lane in &output.selected_lanes {
        for c in &lane.scope {
            touched_crates.insert(c.clone());
        }
    }

    Ok(RustVerdict {
        diff_class: output.diff_class.clone(),
        direct_crates: output.direct_crates.iter().map(|c| c.name.clone()).collect(),
        reverse_dep_crates: output.reverse_dep_closure.iter().map(|c| c.name.clone()).collect(),
        selected_lane_names: output.selected_lanes.iter().map(|l| l.lane.clone()).collect(),
        touched_crates: touched_crates.into_iter().collect(),
    })
}

// ---------------------------------------------------------------------------
// Agreement / direction
// ---------------------------------------------------------------------------

/// Whether the two selectors agree on a scenario, and — if not — which
/// selector routes *more* work. This is a comparison of the crate-level
/// proof surface each selector directs *some* check at: `DocOnlySkip`/an
/// empty `touched_crates` set means "no crate-scoped Rust proof runs";
/// `FullGate` means "the whole workspace runs", which is always a superset
/// of any bounded `touched_crates` set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Direction {
    Agree,
    RustBroader,
    RustNarrower,
    Ambiguous,
}

/// Compare a shell verdict against a Rust verdict and return the direction
/// plus a one-line human-readable note.
pub fn compare(shell: &ShellVerdict, rust: &RustVerdict) -> (Direction, String) {
    match shell {
        ShellVerdict::DocOnlySkip => {
            if rust.touched_crates.is_empty() {
                (Direction::Agree, "both select no crate-scoped Rust proof".to_string())
            } else {
                (
                    Direction::RustBroader,
                    format!(
                        "shell skips all code gates; ci_scope still touches {:?}",
                        rust.touched_crates
                    ),
                )
            }
        }
        ShellVerdict::SingleCrate(name) => {
            let rust_set: BTreeSet<&String> = rust.touched_crates.iter().collect();
            if rust_set.is_empty() {
                (
                    Direction::RustNarrower,
                    format!(
                        "shell runs the targeted gate for `{name}`; ci_scope selects no \
                         crate-scoped lane at all"
                    ),
                )
            } else if rust_set.len() == 1 && rust_set.contains(name) {
                (Direction::Agree, format!("both scope to crate `{name}` only"))
            } else if rust_set.contains(name) {
                (
                    Direction::RustBroader,
                    format!(
                        "shell scopes to `{name}` only; ci_scope also touches {:?}",
                        rust.touched_crates
                    ),
                )
            } else {
                (
                    Direction::Ambiguous,
                    format!(
                        "shell scopes to `{name}`; ci_scope's touched set {:?} does not \
                         include it",
                        rust.touched_crates
                    ),
                )
            }
        }
        ShellVerdict::FullGate => (
            Direction::RustNarrower,
            format!(
                "shell runs the full workspace `pr-fast` gate; ci_scope selects a bounded \
                 scope of {} crate(s): {:?}",
                rust.touched_crates.len(),
                rust.touched_crates
            ),
        ),
    }
}

// ---------------------------------------------------------------------------
// Report row + rendering
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ComparisonRow {
    pub scenario: String,
    pub description: String,
    pub changed_paths: Vec<String>,
    pub shell_verdict: String,
    pub rust_diff_class: String,
    pub rust_touched_crates: Vec<String>,
    pub rust_selected_lanes: Vec<String>,
    pub direction: Direction,
    pub note: String,
}

/// Run every scenario through both classifiers and produce the comparison
/// matrix. `metadata` is real `cargo metadata` JSON (see `run()` below);
/// tests pass a small fixture instead so the harness stays deterministic
/// and offline.
pub fn compare_all(
    metadata: &serde_json::Value,
    workspace_root: &str,
) -> Result<Vec<ComparisonRow>> {
    let mut rows = Vec::new();
    for scenario in scenarios() {
        let shell = shell_verdict(scenario.changed_paths);
        let rust = rust_verdict(scenario.changed_paths, metadata, workspace_root)
            .with_context(|| format!("scenario '{}'", scenario.name))?;
        let (direction, note) = compare(&shell, &rust);
        rows.push(ComparisonRow {
            scenario: scenario.name.to_string(),
            description: scenario.description.to_string(),
            changed_paths: scenario.changed_paths.iter().map(|s| (*s).to_string()).collect(),
            shell_verdict: shell.label(),
            rust_diff_class: rust.diff_class,
            rust_touched_crates: rust.touched_crates,
            rust_selected_lanes: rust.selected_lane_names,
            direction,
            note,
        });
    }
    Ok(rows)
}

fn direction_label(direction: Direction) -> &'static str {
    match direction {
        Direction::Agree => "AGREE",
        Direction::RustBroader => "DIFFER (rust broader)",
        Direction::RustNarrower => "DIFFER (rust narrower)",
        Direction::Ambiguous => "DIFFER (ambiguous)",
    }
}

/// Render the comparison matrix as a Markdown table plus per-scenario notes
/// — the shape committed to `docs/reference/CHANGE_SET_SHADOW_PARITY.md`.
pub fn render_markdown(rows: &[ComparisonRow]) -> String {
    let mut out = String::new();
    out.push_str("| Scenario | Changed paths | Shell verdict | ci_scope diff_class | ci_scope touched crates | Agreement | Direction note |\n");
    out.push_str("|---|---|---|---|---|---|---|\n");
    for row in rows {
        out.push_str(&format!(
            "| `{}` | {} | `{}` | `{}` | {} | {} | {} |\n",
            row.scenario,
            row.changed_paths.iter().map(|p| format!("`{p}`")).collect::<Vec<_>>().join("<br>"),
            row.shell_verdict,
            row.rust_diff_class,
            if row.rust_touched_crates.is_empty() {
                "(none)".to_string()
            } else {
                row.rust_touched_crates.join(", ")
            },
            direction_label(row.direction),
            row.note.replace('|', "\\|"),
        ));
    }
    out
}

fn render_text(rows: &[ComparisonRow]) -> String {
    let mut out = String::new();
    for row in rows {
        out.push_str(&format!("=== {} ===\n", row.scenario));
        out.push_str(&format!("  description: {}\n", row.description));
        out.push_str(&format!("  changed paths: {:?}\n", row.changed_paths));
        out.push_str(&format!("  shell verdict: {}\n", row.shell_verdict));
        out.push_str(&format!(
            "  ci_scope: diff_class={} touched_crates={:?} lanes={:?}\n",
            row.rust_diff_class, row.rust_touched_crates, row.rust_selected_lanes
        ));
        out.push_str(&format!(
            "  agreement: {} — {}\n\n",
            direction_label(row.direction),
            row.note
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// CLI entry point
// ---------------------------------------------------------------------------

pub struct ShadowParityConfig {
    pub format: String,
}

/// Load `cargo metadata` JSON for the live workspace. `no_deps` mirrors
/// `utils::run_cargo_metadata`'s flag: `false` requests the full resolve
/// graph (what `cargo xtask ci-scope`'s own `run()` requests — populates
/// reverse-dependency closure); `true` passes `--no-deps` (what
/// `gates::compute_scope_output` and `ci_pr_summary::run` request in
/// production today — the resolve graph, and therefore reverse-dependency
/// closure, is always empty on those call sites).
fn load_metadata_value(no_deps: bool) -> Result<serde_json::Value> {
    let metadata_raw = utils::run_cargo_metadata(no_deps)
        .context("Failed to load cargo metadata for shadow-parity report")?;
    serde_json::from_slice(&metadata_raw).context("Failed to parse cargo metadata JSON")
}

/// Entry point for `cargo xtask change-set-parity`. MEASUREMENT ONLY: loads
/// real `cargo metadata` (both with and without the resolve graph, since
/// `ci_scope::classify_files`'s production call sites disagree on which
/// they pass — see `load_metadata_value`), runs both selectors over the
/// fixed scenario corpus under each mode, and prints the comparison.
/// Selects, skips, and routes nothing.
pub fn run(config: ShadowParityConfig) -> Result<()> {
    let root = utils::project_root()?;
    let workspace_root = root.to_string_lossy().replace('\\', "/");

    let full_metadata = load_metadata_value(false)?;
    let no_deps_metadata = load_metadata_value(true)?;

    let full_rows = compare_all(&full_metadata, &workspace_root)?;
    let no_deps_rows = compare_all(&no_deps_metadata, &workspace_root)?;

    match config.format.as_str() {
        "markdown" => {
            println!(
                "## ci_scope with full `cargo metadata` (reverse-dep closure populated — matches `cargo xtask ci-scope`'s own CLI)\n"
            );
            println!("{}", render_markdown(&full_rows));
            println!(
                "\n## ci_scope with `cargo metadata --no-deps` (matches `gates::compute_scope_output` / `ci_pr_summary::run` — reverse-dep closure always empty)\n"
            );
            println!("{}", render_markdown(&no_deps_rows));
        }
        "json" => {
            let json = serde_json::to_string_pretty(&serde_json::json!({
                "full_metadata": full_rows,
                "no_deps_metadata": no_deps_rows,
            }))
            .context("Failed to serialize comparison rows to JSON")?;
            println!("{json}");
        }
        _ => {
            println!(
                "=== ci_scope with full `cargo metadata` (reverse-dep closure populated) ===\n"
            );
            println!("{}", render_text(&full_rows));
            println!(
                "=== ci_scope with `cargo metadata --no-deps` (reverse-dep closure empty) ===\n"
            );
            println!("{}", render_text(&no_deps_rows));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Unit tests (inline, deterministic, offline)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A small fixture `cargo metadata` JSON covering exactly the crate
    /// names referenced by `scenarios()`, including the widener-trigger
    /// crates so `ci_scope`'s architectural-widener rules fire the same
    /// way they would against the real workspace graph. `--no-deps` is not
    /// simulated here — the `resolve.nodes` array is present but empty, so
    /// reverse-dep closure is empty (deterministic, matches
    /// `gates.rs`/`ci_pr_summary.rs`'s production `--no-deps` call sites).
    fn fixture_metadata() -> serde_json::Value {
        let packages = [
            ("perl-parser", "crates/perl-parser"),
            ("perl-pod", "crates/perl-pod"),
            ("perl-lsp-rs", "crates/perl-lsp-rs"),
            ("perl-dap", "crates/perl-dap"),
            ("perl-semantic-analyzer", "crates/perl-semantic-analyzer"),
            ("perl-workspace", "crates/perl-workspace"),
        ];
        let pkg_array: Vec<serde_json::Value> = packages
            .iter()
            .map(|(name, rel_dir)| {
                serde_json::json!({
                    "id": format!("{name} 0.1.0"),
                    "name": name,
                    "manifest_path": format!("/workspace/{rel_dir}/Cargo.toml"),
                    "dependencies": []
                })
            })
            .collect();
        serde_json::json!({
            "packages": pkg_array,
            "resolve": { "nodes": [] },
            "workspace_root": "/workspace"
        })
    }

    // --- shell_verdict ---

    #[test]
    fn shell_verdict_docs_only() {
        let paths = ["docs/reference/STABILITY.md", "README.md"];
        assert_eq!(shell_verdict(&paths), ShellVerdict::DocOnlySkip);
    }

    #[test]
    fn shell_verdict_single_crate() {
        let paths = ["crates/perl-pod/src/lib.rs"];
        assert_eq!(shell_verdict(&paths), ShellVerdict::SingleCrate("perl-pod".to_string()));
    }

    #[test]
    fn shell_verdict_multi_crate_falls_back_to_full_gate() {
        let paths = ["crates/perl-parser/src/lib.rs", "crates/perl-lsp-rs/src/lib.rs"];
        assert_eq!(shell_verdict(&paths), ShellVerdict::FullGate);
    }

    #[test]
    fn shell_verdict_non_crate_path_falls_back_to_full_gate() {
        let paths = [".github/workflows/ci.yml"];
        assert_eq!(shell_verdict(&paths), ShellVerdict::FullGate);
    }

    #[test]
    fn shell_verdict_license_subdir_is_doc_only() {
        let paths = ["crates/perl-parser/LICENSE-APACHE"];
        assert_eq!(shell_verdict(&paths), ShellVerdict::DocOnlySkip);
    }

    #[test]
    fn shell_verdict_bare_crate_root_file_is_not_single_crate() {
        // `crates/perl-pod` (no nested path) does not match the shell's
        // `crates/*/*` case pattern, mirroring hooks/pre-push:136.
        let paths = ["crates/perl-pod"];
        assert_eq!(shell_verdict(&paths), ShellVerdict::FullGate);
    }

    // --- rust_verdict ---

    #[test]
    fn rust_verdict_docs_only_is_prose_only_with_no_touched_crates() -> Result<()> {
        let metadata = fixture_metadata();
        let paths = ["docs/reference/STABILITY.md", "README.md"];
        let verdict = rust_verdict(&paths, &metadata, "/workspace")?;
        assert_eq!(verdict.diff_class, "prose_only");
        assert!(verdict.touched_crates.is_empty());
        Ok(())
    }

    #[test]
    fn rust_verdict_leaf_crate_touches_only_itself() -> Result<()> {
        let metadata = fixture_metadata();
        let paths = ["crates/perl-pod/src/lib.rs"];
        let verdict = rust_verdict(&paths, &metadata, "/workspace")?;
        assert_eq!(verdict.diff_class, "code");
        assert_eq!(verdict.touched_crates, vec!["perl-pod".to_string()]);
        Ok(())
    }

    #[test]
    fn rust_verdict_foundation_crate_widens_beyond_itself() -> Result<()> {
        let metadata = fixture_metadata();
        let paths = ["crates/perl-parser/src/statement.rs"];
        let verdict = rust_verdict(&paths, &metadata, "/workspace")?;
        assert!(verdict.direct_crates.contains(&"perl-parser".to_string()));
        // The lsp_smoke architectural widener adds downstream crates beyond
        // the directly-changed perl-parser.
        assert!(
            verdict.touched_crates.len() > 1,
            "expected widener to add crates beyond perl-parser, got {:?}",
            verdict.touched_crates
        );
        assert!(verdict.selected_lane_names.contains(&"lsp_smoke".to_string()));
        Ok(())
    }

    #[test]
    fn rust_verdict_workflow_only_has_no_touched_crates() -> Result<()> {
        let metadata = fixture_metadata();
        let paths = [".github/workflows/ci.yml"];
        let verdict = rust_verdict(&paths, &metadata, "/workspace")?;
        assert_eq!(verdict.diff_class, "ci_config");
        assert!(verdict.direct_crates.is_empty());
        // publish/security/ci_policy lanes select but carry empty scope,
        // so no crate is "touched" in the clippy/test/widener sense.
        assert!(verdict.touched_crates.is_empty());
        Ok(())
    }

    /// Pins the empirical basis for `run()`'s full-vs-`--no-deps` split:
    /// with a populated `resolve.nodes` graph, `ci_scope`'s
    /// reverse-dependency closure adds crates to `touched_crates` for a
    /// leaf crate with no architectural-widener rule (perl-pod is not a
    /// `WIDENER_RULES` trigger). Against the live workspace this made
    /// `touched_crates` for a single leaf-crate change balloon far beyond
    /// the shell's `single-crate(<name>)` scope — a load-bearing
    /// measurement distinct from the diff_class taxonomy question.
    #[test]
    fn rust_verdict_reverse_dep_closure_grows_touched_crates_when_metadata_has_resolve_graph()
    -> Result<()> {
        // perl-pod <- perl-parser <- perl-lsp-rs (A is depended on by B,
        // which is depended on by C).
        let metadata = serde_json::json!({
            "packages": [
                {"id": "perl-pod 0.1.0", "name": "perl-pod", "manifest_path": "/workspace/crates/perl-pod/Cargo.toml"},
                {"id": "perl-parser 0.1.0", "name": "perl-parser", "manifest_path": "/workspace/crates/perl-parser/Cargo.toml"},
                {"id": "perl-lsp-rs 0.1.0", "name": "perl-lsp-rs", "manifest_path": "/workspace/crates/perl-lsp-rs/Cargo.toml"}
            ],
            "resolve": {
                "nodes": [
                    {"id": "perl-pod 0.1.0", "deps": []},
                    {"id": "perl-parser 0.1.0", "deps": [{"pkg": "perl-pod 0.1.0", "name": "perl_pod", "dep_kinds": []}]},
                    {"id": "perl-lsp-rs 0.1.0", "deps": [{"pkg": "perl-parser 0.1.0", "name": "perl_parser", "dep_kinds": []}]}
                ]
            },
            "workspace_root": "/workspace"
        });
        let paths = ["crates/perl-pod/src/lib.rs"];
        let verdict = rust_verdict(&paths, &metadata, "/workspace")?;
        assert_eq!(verdict.direct_crates, vec!["perl-pod".to_string()]);
        assert!(
            verdict.touched_crates.contains(&"perl-parser".to_string()),
            "expected reverse-dep closure to add perl-parser, got {:?}",
            verdict.touched_crates
        );
        assert!(
            verdict.touched_crates.contains(&"perl-lsp-rs".to_string()),
            "expected transitive reverse-dep closure to add perl-lsp-rs, got {:?}",
            verdict.touched_crates
        );
        Ok(())
    }

    // --- compare / direction ---

    #[test]
    fn compare_agrees_when_both_skip() {
        let shell = ShellVerdict::DocOnlySkip;
        let rust = RustVerdict {
            diff_class: "prose_only".to_string(),
            direct_crates: vec![],
            reverse_dep_crates: vec![],
            selected_lane_names: vec![],
            touched_crates: vec![],
        };
        let (direction, _) = compare(&shell, &rust);
        assert_eq!(direction, Direction::Agree);
    }

    #[test]
    fn compare_agrees_when_single_crate_matches_touched_set() {
        let shell = ShellVerdict::SingleCrate("perl-pod".to_string());
        let rust = RustVerdict {
            diff_class: "code".to_string(),
            direct_crates: vec!["perl-pod".to_string()],
            reverse_dep_crates: vec![],
            selected_lane_names: vec!["clippy_scoped".to_string(), "test_scoped".to_string()],
            touched_crates: vec!["perl-pod".to_string()],
        };
        let (direction, _) = compare(&shell, &rust);
        assert_eq!(direction, Direction::Agree);
    }

    #[test]
    fn compare_rust_broader_when_widener_adds_crates() {
        let shell = ShellVerdict::SingleCrate("perl-parser".to_string());
        let rust = RustVerdict {
            diff_class: "code".to_string(),
            direct_crates: vec!["perl-parser".to_string()],
            reverse_dep_crates: vec![],
            selected_lane_names: vec!["lsp_smoke".to_string(), "test_scoped".to_string()],
            touched_crates: vec!["perl-lsp-rs".to_string(), "perl-parser".to_string()],
        };
        let (direction, _) = compare(&shell, &rust);
        assert_eq!(direction, Direction::RustBroader);
    }

    #[test]
    fn compare_rust_narrower_when_shell_is_full_gate() {
        let shell = ShellVerdict::FullGate;
        let rust = RustVerdict {
            diff_class: "ci_config".to_string(),
            direct_crates: vec![],
            reverse_dep_crates: vec![],
            selected_lane_names: vec!["publish".to_string()],
            touched_crates: vec![],
        };
        let (direction, _) = compare(&shell, &rust);
        assert_eq!(direction, Direction::RustNarrower);
    }

    #[test]
    fn compare_rust_narrower_when_single_crate_has_empty_touched_crates() {
        // Shell identifies a single crate but ci_scope's touched_crates is
        // empty (all selected lanes carry no crate scope, e.g., only
        // ci_config/security/publish lanes fire).
        let shell = ShellVerdict::SingleCrate("perl-pod".to_string());
        let rust = RustVerdict {
            diff_class: "code".to_string(),
            direct_crates: vec!["perl-pod".to_string()],
            reverse_dep_crates: vec![],
            selected_lane_names: vec!["ci_config".to_string()],
            touched_crates: vec![],
        };
        let (direction, _) = compare(&shell, &rust);
        assert_eq!(direction, Direction::RustNarrower);
    }

    #[test]
    fn compare_ambiguous_when_single_crate_not_in_touched_set() {
        // Shell scopes to a single crate, but ci_scope's touched_crates set
        // does not include that crate (contradictory signals — ci_scope
        // selected some lane but excluded the changed crate).
        let shell = ShellVerdict::SingleCrate("perl-pod".to_string());
        let rust = RustVerdict {
            diff_class: "code".to_string(),
            direct_crates: vec!["perl-pod".to_string()],
            reverse_dep_crates: vec![],
            selected_lane_names: vec!["lsp_smoke".to_string()],
            touched_crates: vec!["perl-parser".to_string(), "perl-lsp-rs".to_string()],
        };
        let (direction, _) = compare(&shell, &rust);
        assert_eq!(direction, Direction::Ambiguous);
    }

    // --- full corpus + report rendering ---

    #[test]
    fn compare_all_covers_every_scenario_without_error() -> Result<()> {
        let metadata = fixture_metadata();
        let rows = compare_all(&metadata, "/workspace")?;
        assert_eq!(rows.len(), scenarios().len());
        assert_eq!(rows.len(), 11, "the corpus must have exactly 11 representative scenarios");
        Ok(())
    }

    #[test]
    fn render_markdown_includes_every_scenario_name() -> Result<()> {
        let metadata = fixture_metadata();
        let rows = compare_all(&metadata, "/workspace")?;
        let markdown = render_markdown(&rows);
        for scenario in scenarios() {
            assert!(
                markdown.contains(scenario.name),
                "markdown report missing scenario `{}`",
                scenario.name
            );
        }
        Ok(())
    }
}
