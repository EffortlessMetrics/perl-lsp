//! `cargo xtask ci pr-summary` — PR gate summary (dry-run mode).
//!
//! Computes a human-readable markdown summary of what CI would run for the
//! current PR diff against a base ref, without posting to GitHub.
//!
//! # Output sections (markdown)
//! - `## Changed Crates` — files that differ, mapped to crate names.
//! - `## Widened Crates` — transitive reverse-deps pulled in for re-testing.
//! - `## Gates Run` — CI gates selected by scope classifier.
//! - `## Gates Skipped by Policy` — gates excluded this cycle.
//! - `## Timing Estimate` — estimated vs actual CI duration (from learned estimates when available).
//! - `## Receipts` — artifact links (not live in dry-run; placeholder text).
//!
//! # Claim boundary
//! **DRY-RUN ONLY.** This command emits markdown to stdout and exits 0.
//! GitHub sticky-comment posting is deferred to a follow-up issue.
//! Real-time CI integration (live receipts) is a separate follow-up.
//!
//! # Example
//! ```text
//! cargo xtask ci pr-summary --base origin/main --dry-run
//! ```

use color_eyre::eyre::{Context, Result};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Public config type
// ---------------------------------------------------------------------------

/// Configuration for the `ci pr-summary` subcommand.
pub struct CiPrSummaryConfig {
    /// Base git reference to diff against (e.g. `origin/main`).
    pub base: String,
    /// When true, emit markdown to stdout only; do not post to GitHub.
    /// Currently the only supported mode — `false` is reserved for a future
    /// GitHub-posting follow-up and will be rejected with an error.
    pub dry_run: bool,
}

// ---------------------------------------------------------------------------
// Internal section types
// ---------------------------------------------------------------------------

/// A crate that was directly changed in the PR diff.
struct ChangedCrate {
    name: String,
    file_count: usize,
}

/// A crate pulled in via reverse-dep widening.
struct WidenedCrate {
    name: String,
    reason: String,
}

/// A CI gate that was selected to run.
struct GateRun {
    name: String,
    reason: String,
}

/// Summary data gathered from the diff + cargo metadata + policy files.
struct PrSummary {
    base_ref: String,
    head_sha: String,
    changed_file_count: usize,
    diff_class: String,
    changed_crates: Vec<ChangedCrate>,
    widened_crates: Vec<WidenedCrate>,
    gates_run: Vec<GateRun>,
    gates_skipped: Vec<String>,
    policy_note: Option<String>,
    timing_estimate_secs: Option<u64>,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run the `ci pr-summary` subcommand.
pub fn run(config: CiPrSummaryConfig) -> Result<()> {
    if !config.dry_run {
        color_eyre::eyre::bail!(
            "--dry-run is required. GitHub-posting mode is not yet implemented \
             (see follow-up issue for #4825)."
        );
    }

    // Use the current working directory as the git/cargo root.
    // This allows test isolation via `current_dir(temp_dir)` while still
    // working correctly when invoked from the real project root.
    let root = std::env::current_dir().context("failed to get current working directory")?;
    let summary = gather_summary(&config.base, &root)?;
    let markdown = render_markdown(&summary);
    print!("{markdown}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Data gathering
// ---------------------------------------------------------------------------

fn gather_summary(base: &str, root: &Path) -> Result<PrSummary> {
    let head_sha = git_output(&["rev-parse", "--short", "HEAD"], root)
        .unwrap_or_else(|| "unknown".to_string());

    let changed_files = get_changed_files(base, root)?;
    let file_count = changed_files.len();
    let diff_class = classify_diff_simple(&changed_files);

    // Attempt cargo metadata for crate mapping; degrade gracefully on error.
    let (changed_crates, widened_crates) = match load_metadata(root) {
        Ok(metadata) => {
            let workspace_root = root.to_string_lossy().replace('\\', "/");
            match crate::tasks::ci_scope::classify_files(&changed_files, &metadata, &workspace_root)
            {
                Ok(scope) => {
                    let changed = scope
                        .direct_crates
                        .iter()
                        .map(|c| ChangedCrate {
                            name: c.name.clone(),
                            file_count: count_files_in_crate(&changed_files, &c.name),
                        })
                        .collect();
                    let widened = scope
                        .architecture_wideners
                        .iter()
                        .map(|w| WidenedCrate { name: w.name.clone(), reason: w.rule.clone() })
                        .chain(scope.reverse_dep_closure.iter().map(|r| WidenedCrate {
                            name: r.name.clone(),
                            reason: r.reason.clone(),
                        }))
                        .collect::<Vec<_>>();
                    // Deduplicate widened by name
                    let mut seen: BTreeSet<String> = BTreeSet::new();
                    let widened_dedup: Vec<WidenedCrate> =
                        widened.into_iter().filter(|w| seen.insert(w.name.clone())).collect();
                    (changed, widened_dedup)
                }
                Err(_) => (vec![], vec![]),
            }
        }
        Err(_) => (vec![], vec![]),
    };

    // Gate selection — use the selected_lanes from ci_scope for gates_run.
    // Gates skipped are everything NOT in selected_lanes (heuristic).
    let (gates_run, gates_skipped, policy_note) = derive_gates(&changed_files, root);

    // Timing estimate — try reading policy/learned-estimates or docs/ci
    let timing_estimate_secs = read_timing_estimate(root);

    Ok(PrSummary {
        base_ref: base.to_string(),
        head_sha,
        changed_file_count: file_count,
        diff_class,
        changed_crates,
        widened_crates,
        gates_run,
        gates_skipped,
        policy_note,
        timing_estimate_secs,
    })
}

/// Count how many changed files belong to a given crate name by prefix match.
fn count_files_in_crate(files: &[String], crate_name: &str) -> usize {
    files
        .iter()
        .filter(|f| {
            f.starts_with(&format!("crates/{crate_name}/"))
                || (crate_name == "xtask" && f.starts_with("xtask/"))
        })
        .count()
}

/// Simple diff classifier (mirrors ci_scope logic without cargo metadata dependency).
fn classify_diff_simple(files: &[String]) -> String {
    if files.is_empty() {
        return "prose_only".to_string();
    }
    let has_rs = files.iter().any(|f| f.ends_with(".rs"));
    let has_toml = files.iter().any(|f| f.ends_with(".toml") || f.ends_with(".lock"));
    let has_md = files.iter().any(|f| f.ends_with(".md"));
    let has_ci = files.iter().any(|f| f.starts_with(".github/workflows/") || f.starts_with(".ci/"));
    if has_rs && (has_md || has_toml) {
        "mixed".to_string()
    } else if has_rs {
        "code".to_string()
    } else if has_ci {
        "ci_config".to_string()
    } else if has_toml {
        "docs_as_code".to_string()
    } else if has_md {
        "prose_only".to_string()
    } else {
        "code".to_string()
    }
}

/// Derive gate lists from the changed files and project structure.
///
/// Returns `(gates_run, gates_skipped, policy_note)`.
fn derive_gates(
    changed_files: &[String],
    root: &Path,
) -> (Vec<GateRun>, Vec<String>, Option<String>) {
    // All known gate names this project runs
    const ALL_GATES: &[&str] = &[
        "fmt",
        "clippy_scoped",
        "test_scoped",
        "lsp_smoke",
        "lsp_providers",
        "ux_regression",
        "publish",
        "security",
        "ci_policy",
        "bounded_parser_fuzz",
        "thread_sanitizer",
        "perf_regression",
        "security_audit",
        "mutation_diff",
        "parser_ratchet",
    ];

    // Try to load scope via cargo metadata for precise gate selection.
    let selected_names: BTreeSet<String> = load_metadata(root)
        .ok()
        .and_then(|metadata| {
            let workspace_root = root.to_string_lossy().replace('\\', "/");
            crate::tasks::ci_scope::classify_files(changed_files, &metadata, &workspace_root).ok()
        })
        .map(|scope| {
            let mut names: BTreeSet<String> =
                scope.selected_lanes.iter().map(|l| l.lane.clone()).collect();
            names.extend(scope.selected_heavy_lanes.iter().map(|l| l.lane.clone()));
            // fmt always runs
            names.insert("fmt".to_string());
            if scope.lanes.parser_ratchet.selected {
                names.insert("parser_ratchet".to_string());
            }
            names
        })
        .unwrap_or_else(|| {
            // Graceful degradation: select fmt + test_scoped for any non-empty diff
            let mut fallback = BTreeSet::new();
            fallback.insert("fmt".to_string());
            if !changed_files.is_empty() {
                fallback.insert("test_scoped".to_string());
                fallback.insert("clippy_scoped".to_string());
            }
            fallback
        });

    // Read policy note from policy/ directory if present
    let policy_note = read_policy_note(root);

    let gates_run: Vec<GateRun> = ALL_GATES
        .iter()
        .filter(|name| selected_names.contains(**name))
        .map(|name| GateRun { name: name.to_string(), reason: gate_reason(name) })
        .collect();

    let gates_skipped: Vec<String> = ALL_GATES
        .iter()
        .filter(|name| !selected_names.contains(**name))
        .map(|s| s.to_string())
        .collect();

    (gates_run, gates_skipped, policy_note)
}

fn gate_reason(name: &str) -> String {
    match name {
        "fmt" => "always runs".to_string(),
        "clippy_scoped" => "code changes detected".to_string(),
        "test_scoped" => "code changes detected".to_string(),
        "lsp_smoke" => "architectural widener: parser → downstream smoke".to_string(),
        "lsp_providers" => {
            "architectural widener: semantic → LSP definition/references".to_string()
        }
        "ux_regression" => "architectural widener: lsp/dap change or features.toml".to_string(),
        "publish" => "workspace root file changed".to_string(),
        "security" => "workspace root file changed".to_string(),
        "ci_policy" => "workspace root file changed".to_string(),
        "bounded_parser_fuzz" => "risk_tag: parser_recovery".to_string(),
        "thread_sanitizer" => "risk_tag: concurrency".to_string(),
        "perf_regression" => "risk_tag: perf_hot_path".to_string(),
        "security_audit" => "risk_tag: security_surface".to_string(),
        "mutation_diff" => "default: any code diff".to_string(),
        "parser_ratchet" => "parser path changed".to_string(),
        _ => "selected by scope classifier".to_string(),
    }
}

/// Read the first available policy file for a summary note.
fn read_policy_note(root: &Path) -> Option<String> {
    let candidates = [
        "policy/ci-budget.toml",
        "policy/ci-lanes.toml",
        ".ci/gate-policy.yaml",
        ".ci/GATE_REGISTRY.toml",
    ];
    for candidate in &candidates {
        let path = root.join(candidate);
        if path.exists() {
            return Some(format!("Policy loaded from `{candidate}`"));
        }
    }
    None
}

/// Attempt to read a timing estimate from docs/ci/ files.
fn read_timing_estimate(root: &Path) -> Option<u64> {
    // Look for learned-estimates or lem-budgeting files
    let candidates =
        ["docs/ci/learned-estimates.md", "docs/ci/lem-budgeting.md", "docs/ci/ci-actuals.md"];
    for candidate in &candidates {
        let path = root.join(candidate);
        if path.exists() {
            // We found a file — emit a note that estimates exist but don't parse them.
            // Return a sentinel value indicating "file present, estimate not parsed".
            return Some(0);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Markdown renderer
// ---------------------------------------------------------------------------

fn render_markdown(s: &PrSummary) -> String {
    let mut out = String::new();

    out.push_str("<!-- perl-lsp-ci-summary-dry-run -->\n");
    out.push_str("## PR CI Gate Summary (dry-run)\n\n");
    out.push_str(&format!("**Base**: `{}`  \n", s.base_ref));
    out.push_str(&format!("**HEAD**: `{}`  \n", s.head_sha));
    out.push_str(&format!(
        "**Diff class**: `{}` ({} file(s) changed)  \n\n",
        s.diff_class, s.changed_file_count
    ));

    // Changed crates
    out.push_str("## Changed Crates\n\n");
    if s.changed_crates.is_empty() {
        out.push_str("_(no crates directly changed — diff may be docs/prose only)_\n\n");
    } else {
        for c in &s.changed_crates {
            out.push_str(&format!("- `{}` ({} file(s))\n", c.name, c.file_count));
        }
        out.push('\n');
    }

    // Widened crates
    out.push_str("## Widened Crates\n\n");
    if s.widened_crates.is_empty() {
        out.push_str("_(no widening — changes are isolated)_\n\n");
    } else {
        for w in &s.widened_crates {
            out.push_str(&format!("- `{}` — {}\n", w.name, w.reason));
        }
        out.push('\n');
    }

    // Gates run
    out.push_str("## Gates Run\n\n");
    if s.gates_run.is_empty() {
        out.push_str("_(no gates selected — prose-only diff)_\n\n");
    } else {
        for g in &s.gates_run {
            out.push_str(&format!("- `{}` — {}\n", g.name, g.reason));
        }
        out.push('\n');
    }

    // Gates skipped
    out.push_str("## Gates Skipped by Policy\n\n");
    if s.gates_skipped.is_empty() {
        out.push_str("_(all known gates selected)_\n\n");
    } else {
        for name in &s.gates_skipped {
            out.push_str(&format!("- `{name}`\n"));
        }
        out.push('\n');
    }

    // Timing
    out.push_str("## Timing Estimate\n\n");
    match s.timing_estimate_secs {
        Some(0) => {
            out.push_str(
                "Learned-estimates file present — detailed per-lane estimates available in \
                 `docs/ci/`. Total estimate not parsed in this dry-run.\n\n",
            );
        }
        Some(secs) => {
            let mins = secs / 60;
            let remaining_secs = secs % 60;
            out.push_str(&format!("Estimated total CI duration: ~{mins}m {remaining_secs}s\n\n"));
        }
        None => {
            out.push_str(
                "No learned-estimates file found. Timing estimates require `docs/ci/` \
                 receipts — run CI once to populate.\n\n",
            );
        }
    }

    if let Some(ref note) = s.policy_note {
        out.push_str("## Policy\n\n");
        out.push_str(&format!("{note}\n\n"));
    }

    // Receipts
    out.push_str("## Receipts\n\n");
    out.push_str(
        "_(dry-run mode: no live CI run. Run `cargo xtask gates` to generate \
         receipt artifacts, or check GitHub Actions for live URLs after push.)_\n",
    );

    out
}

// ---------------------------------------------------------------------------
// Git + cargo helpers
// ---------------------------------------------------------------------------

fn git_output(args: &[&str], cwd: &Path) -> Option<String> {
    let output =
        Command::new("git").args(args).current_dir(cwd).stderr(Stdio::null()).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn get_changed_files(base_ref: &str, root: &Path) -> Result<Vec<String>> {
    // Three-dot diff: commits reachable from HEAD but not from base_ref
    let diff_spec = format!("{base_ref}...HEAD");
    let output = Command::new("git")
        .args(["diff", "--name-only", &diff_spec])
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("failed to run git diff")?;

    if output.status.success() {
        let stdout =
            String::from_utf8(output.stdout).context("git diff output was not valid UTF-8")?;
        let files: Vec<String> =
            stdout.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
        return Ok(files);
    }

    // Two-dot fallback
    let diff_spec_two = format!("{base_ref}..HEAD");
    let output2 = Command::new("git")
        .args(["diff", "--name-only", &diff_spec_two])
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("failed to run git diff (two-dot fallback)")?;

    let stdout =
        String::from_utf8(output2.stdout).context("git diff output was not valid UTF-8")?;
    Ok(stdout.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
}

fn load_metadata(root: &Path) -> Result<serde_json::Value> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("failed to run cargo metadata")?;

    let stdout =
        String::from_utf8(output.stdout).context("cargo metadata output was not valid UTF-8")?;
    serde_json::from_str(&stdout).context("failed to parse cargo metadata JSON")
}

// ---------------------------------------------------------------------------
// Unit tests (inline)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_markdown_has_all_required_sections() {
        let summary = PrSummary {
            base_ref: "origin/master".to_string(),
            head_sha: "abc1234".to_string(),
            changed_file_count: 2,
            diff_class: "code".to_string(),
            changed_crates: vec![ChangedCrate { name: "perl-parser".to_string(), file_count: 2 }],
            widened_crates: vec![],
            gates_run: vec![
                GateRun { name: "fmt".to_string(), reason: "always runs".to_string() },
                GateRun {
                    name: "test_scoped".to_string(),
                    reason: "code changes detected".to_string(),
                },
            ],
            gates_skipped: vec!["bounded_parser_fuzz".to_string()],
            policy_note: None,
            timing_estimate_secs: None,
        };

        let md = render_markdown(&summary);

        assert!(md.contains("## Changed Crates"), "must have Changed Crates section");
        assert!(md.contains("## Widened Crates"), "must have Widened Crates section");
        assert!(md.contains("## Gates Run"), "must have Gates Run section");
        assert!(md.contains("## Gates Skipped by Policy"), "must have Gates Skipped section");
        assert!(md.contains("## Timing Estimate"), "must have Timing Estimate section");
        assert!(md.contains("## Receipts"), "must have Receipts section");
        assert!(md.contains("perl-parser"), "should list changed crate");
        assert!(md.contains("origin/master"), "should show base ref");
        assert!(md.contains("abc1234"), "should show head sha");
        assert!(md.contains("dry-run"), "should mention dry-run");
    }

    #[test]
    fn render_markdown_empty_changeset() {
        let summary = PrSummary {
            base_ref: "origin/master".to_string(),
            head_sha: "000000".to_string(),
            changed_file_count: 0,
            diff_class: "prose_only".to_string(),
            changed_crates: vec![],
            widened_crates: vec![],
            gates_run: vec![],
            gates_skipped: vec!["fmt".to_string(), "test_scoped".to_string()],
            policy_note: None,
            timing_estimate_secs: None,
        };

        let md = render_markdown(&summary);

        assert!(md.contains("no crates directly changed"), "empty changeset note expected");
        assert!(md.contains("no gates selected"), "no gates note expected");
    }

    #[test]
    fn classify_diff_simple_code() {
        let files = vec!["crates/perl-parser/src/lib.rs".to_string()];
        assert_eq!(classify_diff_simple(&files), "code");
    }

    #[test]
    fn classify_diff_simple_empty() {
        assert_eq!(classify_diff_simple(&[]), "prose_only");
    }

    #[test]
    fn classify_diff_simple_mixed() {
        let files = vec![
            "crates/perl-parser/src/lib.rs".to_string(),
            "docs/reference/STABILITY.md".to_string(),
        ];
        assert_eq!(classify_diff_simple(&files), "mixed");
    }

    #[test]
    fn gate_reason_covers_all_known_gates() {
        let known = [
            "fmt",
            "clippy_scoped",
            "test_scoped",
            "lsp_smoke",
            "lsp_providers",
            "ux_regression",
            "publish",
            "security",
            "ci_policy",
            "bounded_parser_fuzz",
            "thread_sanitizer",
            "perf_regression",
            "security_audit",
            "mutation_diff",
            "parser_ratchet",
        ];
        for gate in &known {
            let reason = gate_reason(gate);
            assert!(!reason.is_empty(), "gate_reason({gate}) should not be empty");
        }
    }

    #[test]
    fn count_files_in_crate_correct() {
        let files = vec![
            "crates/perl-parser/src/lib.rs".to_string(),
            "crates/perl-parser/src/stmt.rs".to_string(),
            "crates/perl-lexer/src/lib.rs".to_string(),
        ];
        assert_eq!(count_files_in_crate(&files, "perl-parser"), 2);
        assert_eq!(count_files_in_crate(&files, "perl-lexer"), 1);
        assert_eq!(count_files_in_crate(&files, "xtask"), 0);
    }

    #[test]
    fn count_files_in_crate_xtask() {
        let files = vec!["xtask/src/main.rs".to_string(), "xtask/src/tasks/ci.rs".to_string()];
        assert_eq!(count_files_in_crate(&files, "xtask"), 2);
    }
}
