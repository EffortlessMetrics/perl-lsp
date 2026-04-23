//! CI scope classifier — `cargo xtask ci-scope`
//!
//! Computes:
//! 1. Changed files via `git diff --name-only <base>...HEAD`
//! 2. Maps files to crates via cargo metadata
//! 3. Computes reverse-dependency closure from the dep graph
//! 4. Applies architectural wideners (parser → LSP/DAP, etc.)
//! 5. Emits JSON or text describing selected lanes
//!
//! Output is deterministic given the same diff + cargo metadata.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use color_eyre::eyre::{Context, Result, eyre};
use duct::cmd;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Public output types
// ---------------------------------------------------------------------------

/// A directly-changed crate (reason = "direct").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrateEntry {
    pub name: String,
    pub reason: String,
}

/// A crate pulled in by an architectural widener.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct WidenedCrateEntry {
    pub name: String,
    pub reason: String,
}

/// A selected CI lane with its reason and scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneEntry {
    pub lane: String,
    pub reason: String,
    pub scope: Vec<String>,
}

/// The full scope classifier output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeOutput {
    pub schema_version: u32,
    pub base: String,
    pub head_sha: String,
    pub changed_files: Vec<String>,
    pub changed_crates: Vec<CrateEntry>,
    pub widened_crates: Vec<WidenedCrateEntry>,
    pub selected_lanes: Vec<LaneEntry>,
}

// ---------------------------------------------------------------------------
// Architectural widener rules (const, for testability)
// ---------------------------------------------------------------------------

/// A single widener rule: when any of the `trigger_prefixes` crates change,
/// add `targets` to the widened set with the given `reason`.
struct WidenerRule {
    /// Crate name prefixes that trigger this rule (exact match or prefix match).
    trigger_prefixes: &'static [&'static str],
    /// Crates to add to the widened set.
    targets: &'static [&'static str],
    /// Human-readable reason explaining the widening.
    reason: &'static str,
    /// Lane to select when this rule fires.
    lanes: &'static [&'static str],
    /// Lane reason tag.
    lane_reason: &'static str,
}

/// All architectural widener rules.  The order matters for lane generation
/// (earlier rules fire first), but deduplication ensures idempotent output.
static WIDENER_RULES: &[WidenerRule] = &[
    // Rule 1: parser / lexer / parser-core → semantic, workspace-index, LSP, DAP
    WidenerRule {
        trigger_prefixes: &["perl-parser", "perl-lexer", "perl-parser-core"],
        targets: &["perl-semantic-analyzer", "perl-workspace-index", "perl-lsp-rs", "perl-dap"],
        reason: "architectural: parser → LSP/DAP downstream",
        lanes: &["lsp_smoke"],
        lane_reason: "architectural_widener",
    },
    // Rule 2: semantic-analyzer / workspace-index → LSP providers
    WidenerRule {
        trigger_prefixes: &["perl-semantic-analyzer", "perl-workspace-index"],
        targets: &[
            "perl-lsp-definition",
            "perl-lsp-references",
            "perl-lsp-rename",
            "perl-lsp-workspace",
            "perl-lsp-rs",
        ],
        reason: "architectural: semantic → LSP provider downstream",
        lanes: &["lsp_providers"],
        lane_reason: "architectural_widener",
    },
    // Rule 3: LSP/DAP crates + features.toml → UX regression
    WidenerRule {
        trigger_prefixes: &["perl-lsp-", "perl-dap"],
        targets: &["perl-lsp-rs"],
        reason: "architectural: lsp/dap change → UX regression",
        lanes: &["ux_regression"],
        lane_reason: "architectural_widener",
    },
];

// ---------------------------------------------------------------------------
// File classification helpers
// ---------------------------------------------------------------------------

/// Returns true if all changed files are documentation-only (docs/, *.md,
/// .github/ISSUE_TEMPLATE/) and therefore skip heavy CI lanes.
fn is_docs_only(files: &[String]) -> bool {
    if files.is_empty() {
        return false;
    }
    files.iter().all(|f| {
        f.starts_with("docs/") || f.ends_with(".md") || f.starts_with(".github/ISSUE_TEMPLATE/")
    })
}

/// Returns true if the changed files include workspace root files that trigger
/// the full-workspace scope (Cargo.toml, Cargo.lock, workflow files, hooks, justfile).
fn is_workspace_root_change(files: &[String]) -> bool {
    files.iter().any(|f| {
        matches!(f.as_str(), "Cargo.toml" | "Cargo.lock" | "justfile")
            || f.starts_with(".github/workflows/")
            || f.starts_with("hooks/")
    })
}

/// Extract unique crate names from the cargo metadata JSON for crate dirs
/// seen in the changed files list.
///
/// Returns (crate_name → canonical_crate_name) for all changed crates.
fn crates_from_files(
    files: &[String],
    metadata: &serde_json::Value,
    workspace_root: &str,
) -> Result<BTreeSet<String>> {
    // Collect changed crate directories like "crates/perl-parser"
    let mut crate_dirs = BTreeSet::new();
    for file in files {
        let parts: Vec<&str> = file.splitn(3, '/').collect();
        if parts.len() >= 2 && parts[0] == "crates" && !parts[1].is_empty() {
            crate_dirs.insert(format!("crates/{}", parts[1]));
        }
    }

    if crate_dirs.is_empty() {
        return Ok(BTreeSet::new());
    }

    let packages = metadata
        .get("packages")
        .and_then(|p| p.as_array())
        .ok_or_else(|| eyre!("cargo metadata missing 'packages' array"))?;

    let root_normalized = workspace_root.replace('\\', "/");
    let mut names = BTreeSet::new();

    for package in packages {
        let manifest_path = match package.get("manifest_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => continue,
        };
        let pkg_name = match package.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => continue,
        };

        let manifest_normalized = manifest_path.replace('\\', "/");
        let relative = manifest_normalized
            .strip_prefix(root_normalized.as_str())
            .and_then(|p| p.strip_prefix('/'))
            .and_then(|p| p.strip_suffix("/Cargo.toml"));

        if let Some(rel_dir) = relative
            && crate_dirs.contains(rel_dir)
        {
            names.insert(pkg_name.to_string());
        }
    }

    Ok(names)
}

// ---------------------------------------------------------------------------
// Reverse-dependency closure
// ---------------------------------------------------------------------------

/// Build a reverse-dependency map: package_name → set of packages that depend on it.
///
/// Uses the `resolve.nodes` array from cargo metadata.
fn build_reverse_dep_map(metadata: &serde_json::Value) -> BTreeMap<String, BTreeSet<String>> {
    let mut rev_deps: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    let nodes = metadata.pointer("/resolve/nodes").and_then(|n| n.as_array());

    let nodes = match nodes {
        Some(n) => n,
        None => return rev_deps,
    };

    // Build id → name map first
    let packages = metadata.get("packages").and_then(|p| p.as_array());
    let mut id_to_name: BTreeMap<String, String> = BTreeMap::new();
    if let Some(pkgs) = packages {
        for pkg in pkgs {
            let id = pkg.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let name = pkg.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if !id.is_empty() && !name.is_empty() {
                id_to_name.insert(id.to_string(), name.to_string());
            }
        }
    }

    // For each node, record that its deps have this node as a reverse dep
    for node in nodes {
        let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let node_name = match id_to_name.get(node_id) {
            Some(n) => n.clone(),
            None => {
                // Fall back: try to extract name from id "name version (registry)"
                node_id.split(':').next().unwrap_or(node_id).to_string()
            }
        };

        let deps = node.get("deps").and_then(|d| d.as_array());
        if let Some(deps) = deps {
            for dep in deps {
                let dep_pkg_id = dep.get("pkg").and_then(|v| v.as_str()).unwrap_or("");
                let dep_name = match id_to_name.get(dep_pkg_id) {
                    Some(n) => n.clone(),
                    None => dep_pkg_id.split(':').next().unwrap_or(dep_pkg_id).to_string(),
                };
                if !dep_name.is_empty() {
                    rev_deps.entry(dep_name).or_default().insert(node_name.clone());
                }
            }
        }
    }

    rev_deps
}

/// Compute the full reverse-dependency closure for a set of changed crate names.
/// Returns only workspace-internal crates (those present in packages).
fn reverse_dep_closure(
    changed: &BTreeSet<String>,
    rev_deps: &BTreeMap<String, BTreeSet<String>>,
    all_package_names: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut closure = BTreeSet::new();
    let mut queue: Vec<String> = changed.iter().cloned().collect();

    while let Some(crate_name) = queue.pop() {
        if let Some(dependents) = rev_deps.get(&crate_name) {
            for dep in dependents {
                if all_package_names.contains(dep) && !closure.contains(dep) {
                    closure.insert(dep.clone());
                    queue.push(dep.clone());
                }
            }
        }
    }

    closure
}

// ---------------------------------------------------------------------------
// Main public API (testable without live git/cargo)
// ---------------------------------------------------------------------------

/// Classify a list of changed files against cargo metadata JSON.
///
/// `workspace_root` is the absolute path prefix used in manifest_path fields
/// (e.g. `"/path/to/project"`). In tests, pass a fake root like `"/workspace"`.
pub fn classify_files(
    files: &[String],
    metadata: &serde_json::Value,
    workspace_root: &str,
) -> Result<ScopeOutput> {
    // Empty diff → empty output
    if files.is_empty() {
        return Ok(ScopeOutput {
            schema_version: 1,
            base: String::new(),
            head_sha: String::new(),
            changed_files: vec![],
            changed_crates: vec![],
            widened_crates: vec![],
            selected_lanes: vec![],
        });
    }

    // Docs-only → skip heavy lanes
    if is_docs_only(files) {
        return Ok(ScopeOutput {
            schema_version: 1,
            base: String::new(),
            head_sha: String::new(),
            changed_files: files.to_vec(),
            changed_crates: vec![],
            widened_crates: vec![],
            selected_lanes: vec![],
        });
    }

    let mut lanes: Vec<LaneEntry> = vec![];
    let mut changed_crates: Vec<CrateEntry> = vec![];

    // Workspace-root changes: trigger infra lanes
    if is_workspace_root_change(files) {
        lanes.push(LaneEntry {
            lane: "publish".to_string(),
            reason: "workspace_root".to_string(),
            scope: vec![],
        });
        lanes.push(LaneEntry {
            lane: "security".to_string(),
            reason: "workspace_root".to_string(),
            scope: vec![],
        });
        lanes.push(LaneEntry {
            lane: "ci_policy".to_string(),
            reason: "workspace_root".to_string(),
            scope: vec![],
        });
    }

    // Collect all package names for reverse-dep filtering
    let all_package_names: BTreeSet<String> = metadata
        .get("packages")
        .and_then(|p| p.as_array())
        .map(|pkgs| {
            pkgs.iter()
                .filter_map(|pkg| pkg.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // Map changed files → crate names
    let directly_changed = crates_from_files(files, metadata, workspace_root)?;

    // Build reverse-dep map and compute closure
    let rev_deps = build_reverse_dep_map(metadata);
    let rev_dep_crates = reverse_dep_closure(&directly_changed, &rev_deps, &all_package_names);

    // Populate changed_crates (direct + reverse-dep)
    for name in &directly_changed {
        changed_crates.push(CrateEntry { name: name.clone(), reason: "direct".to_string() });
    }
    for name in &rev_dep_crates {
        if !directly_changed.contains(name) {
            changed_crates
                .push(CrateEntry { name: name.clone(), reason: "reverse-dep".to_string() });
        }
    }
    changed_crates.sort();

    // Add scoped lanes for directly changed + reverse-dep crates
    if !changed_crates.is_empty() {
        let scope: Vec<String> = changed_crates.iter().map(|c| c.name.clone()).collect();
        lanes.push(LaneEntry {
            lane: "clippy_scoped".to_string(),
            reason: "changed_crates".to_string(),
            scope: scope.clone(),
        });
        lanes.push(LaneEntry {
            lane: "test_scoped".to_string(),
            reason: "changed_crates".to_string(),
            scope,
        });
    }

    // Apply architectural wideners
    let widened = apply_wideners(&changed_crates)?;

    // Derive widener lanes from the widener rules that fired
    for rule in WIDENER_RULES {
        let triggered = changed_crates.iter().any(|c| {
            rule.trigger_prefixes
                .iter()
                .any(|prefix| c.name == *prefix || c.name.starts_with(prefix))
        });
        if triggered {
            for lane_name in rule.lanes {
                let scope: Vec<String> = rule.targets.iter().map(|s| s.to_string()).collect();
                // Avoid duplicate lane entries
                let already_present = lanes.iter().any(|l| l.lane == *lane_name);
                if !already_present {
                    lanes.push(LaneEntry {
                        lane: lane_name.to_string(),
                        reason: rule.lane_reason.to_string(),
                        scope,
                    });
                }
            }
        }
    }

    Ok(ScopeOutput {
        schema_version: 1,
        base: String::new(),
        head_sha: String::new(),
        changed_files: files.to_vec(),
        changed_crates,
        widened_crates: widened,
        selected_lanes: lanes,
    })
}

/// Apply architectural widening rules to a set of changed crates.
///
/// Returns the list of widened crates (deduplicated, sorted).
pub fn apply_wideners(changed: &[CrateEntry]) -> Result<Vec<WidenedCrateEntry>> {
    let mut widened: BTreeMap<String, String> = BTreeMap::new();

    for rule in WIDENER_RULES {
        let triggered = changed.iter().any(|c| {
            rule.trigger_prefixes
                .iter()
                .any(|prefix| c.name == *prefix || c.name.starts_with(prefix))
        });

        if triggered {
            for target in rule.targets {
                // First-write wins for the reason; if already present keep original
                widened.entry(target.to_string()).or_insert_with(|| rule.reason.to_string());
            }
        }
    }

    Ok(widened.into_iter().map(|(name, reason)| WidenedCrateEntry { name, reason }).collect())
}

// ---------------------------------------------------------------------------
// CLI config + entry point
// ---------------------------------------------------------------------------

/// Configuration for the `ci-scope` subcommand.
pub struct CiScopeConfig {
    /// Base git ref to diff against (e.g. "origin/master").
    pub base: String,
    /// Output format: "json" or "text".
    pub format: String,
}

/// Entry point called from xtask main.
pub fn run(config: CiScopeConfig) -> Result<()> {
    let root = crate::utils::project_root()?;
    let base_ref = resolve_base_ref(&config.base, &root)?;
    let head_sha = get_head_sha(&root)?;
    let changed_files = get_changed_files(&base_ref, &root)?;
    let metadata = load_metadata(&root)?;
    let workspace_root = root.to_string_lossy().replace('\\', "/");

    let mut output = classify_files(&changed_files, &metadata, &workspace_root)?;
    output.base = config.base.clone();
    output.head_sha = head_sha;
    output.changed_files = changed_files;

    match config.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&output)
                .context("Failed to serialize scope output to JSON")?;
            println!("{json}");
        }
        _ => {
            print_text_summary(&output);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Git + cargo helpers for the live CLI path
// ---------------------------------------------------------------------------

fn resolve_base_ref(base: &str, root: &Path) -> Result<String> {
    let verify = cmd("git", &["rev-parse", "--verify", base])
        .dir(root)
        .stdout_null()
        .stderr_null()
        .unchecked()
        .run()
        .context("Failed to run git rev-parse")?;

    if verify.status.success() {
        return Ok(base.to_string());
    }

    // Fallback: HEAD~1 (useful when origin/master is not available locally)
    eprintln!("Warning: base ref '{}' not found; falling back to HEAD~1", base);
    Ok("HEAD~1".to_string())
}

fn get_head_sha(root: &Path) -> Result<String> {
    let output = cmd("git", &["rev-parse", "HEAD"])
        .dir(root)
        .stdout_capture()
        .stderr_null()
        .run()
        .context("Failed to get HEAD SHA")?;
    Ok(String::from_utf8(output.stdout).context("HEAD SHA was not valid UTF-8")?.trim().to_string())
}

fn get_changed_files(base_ref: &str, root: &Path) -> Result<Vec<String>> {
    let diff_spec = format!("{base_ref}...HEAD");
    let output = cmd("git", &["diff", "--name-only", &diff_spec])
        .dir(root)
        .stdout_capture()
        .stderr_capture()
        .unchecked()
        .run()
        .context("Failed to run git diff")?;

    if !output.status.success() {
        // Two-dot fallback
        let diff_spec_two = format!("{base_ref}..HEAD");
        let output2 = cmd("git", &["diff", "--name-only", &diff_spec_two])
            .dir(root)
            .stdout_capture()
            .stderr_capture()
            .run()
            .context("Failed to run git diff (two-dot fallback)")?;
        let stdout =
            String::from_utf8(output2.stdout).context("git diff output was not valid UTF-8")?;
        return Ok(stdout.lines().map(|l| l.to_string()).collect());
    }

    let stdout = String::from_utf8(output.stdout).context("git diff output was not valid UTF-8")?;
    Ok(stdout.lines().map(|l| l.to_string()).collect())
}

fn load_metadata(root: &Path) -> Result<serde_json::Value> {
    let output = cmd("cargo", &["metadata", "--format-version", "1"])
        .dir(root)
        .stdout_capture()
        .stderr_capture()
        .run()
        .context("Failed to run cargo metadata")?;

    let stdout =
        String::from_utf8(output.stdout).context("cargo metadata output was not valid UTF-8")?;
    serde_json::from_str(&stdout).context("Failed to parse cargo metadata JSON")
}

// ---------------------------------------------------------------------------
// Text output
// ---------------------------------------------------------------------------

fn print_text_summary(output: &ScopeOutput) {
    println!("=== CI Scope Classifier ===");
    println!("Base:     {}", output.base);
    println!("HEAD SHA: {}", output.head_sha);
    println!("Changed files: {}", output.changed_files.len());

    if output.changed_crates.is_empty() {
        println!("Changed crates: (none)");
    } else {
        println!("Changed crates:");
        for c in &output.changed_crates {
            println!("  [{}] {}", c.reason, c.name);
        }
    }

    if !output.widened_crates.is_empty() {
        println!("Widened crates:");
        for w in &output.widened_crates {
            println!("  {} — {}", w.name, w.reason);
        }
    }

    if output.selected_lanes.is_empty() {
        println!("Selected lanes: (none)");
    } else {
        println!("Selected lanes:");
        for l in &output.selected_lanes {
            println!("  [{}] {} — {:?}", l.reason, l.lane, l.scope);
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests (inline)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_metadata(packages: &[(&str, &str)]) -> serde_json::Value {
        let pkg_array: Vec<serde_json::Value> = packages
            .iter()
            .map(|(name, rel_dir)| {
                serde_json::json!({
                    "id": format!("{} 0.1.0", name),
                    "name": name,
                    "manifest_path": format!("/workspace/{}/Cargo.toml", rel_dir),
                    "dependencies": []
                })
            })
            .collect();

        serde_json::json!({
            "packages": pkg_array,
            "resolve": {
                "nodes": packages.iter().map(|(name, _)| {
                    serde_json::json!({
                        "id": format!("{} 0.1.0", name),
                        "deps": []
                    })
                }).collect::<Vec<_>>()
            },
            "workspace_root": "/workspace"
        })
    }

    #[test]
    fn test_is_docs_only_true() {
        assert!(is_docs_only(&[
            "docs/reference/STABILITY.md".to_string(),
            "README.md".to_string(),
        ]));
    }

    #[test]
    fn test_is_docs_only_false_when_code_present() {
        assert!(!is_docs_only(&[
            "crates/perl-parser/src/lib.rs".to_string(),
            "docs/foo.md".to_string(),
        ]));
    }

    #[test]
    fn test_is_docs_only_false_for_empty() {
        assert!(!is_docs_only(&[]));
    }

    #[test]
    fn test_is_workspace_root_change_cargo_toml() {
        assert!(is_workspace_root_change(&["Cargo.toml".to_string()]));
    }

    #[test]
    fn test_is_workspace_root_change_cargo_lock() {
        assert!(is_workspace_root_change(&["Cargo.lock".to_string()]));
    }

    #[test]
    fn test_is_workspace_root_change_workflow() {
        assert!(is_workspace_root_change(&[".github/workflows/ci.yml".to_string()]));
    }

    #[test]
    fn test_is_workspace_root_change_not_triggered_by_crate() {
        assert!(!is_workspace_root_change(&["crates/perl-parser/src/lib.rs".to_string()]));
    }

    #[test]
    fn test_crates_from_files_basic() -> Result<()> {
        let files = vec!["crates/perl-parser/src/lib.rs".to_string()];
        let metadata = fake_metadata(&[("perl-parser", "crates/perl-parser")]);
        let crates = crates_from_files(&files, &metadata, "/workspace")?;
        assert!(crates.contains("perl-parser"));
        assert_eq!(crates.len(), 1);
        Ok(())
    }

    #[test]
    fn test_crates_from_files_empty() -> Result<()> {
        let files: Vec<String> = vec![];
        let metadata = fake_metadata(&[("perl-parser", "crates/perl-parser")]);
        let crates = crates_from_files(&files, &metadata, "/workspace")?;
        assert!(crates.is_empty());
        Ok(())
    }

    #[test]
    fn test_build_reverse_dep_map_basic() {
        // perl-lsp-rs depends on perl-parser
        let metadata = serde_json::json!({
            "packages": [
                {"id": "perl-parser 0.1.0", "name": "perl-parser", "manifest_path": "/w/crates/perl-parser/Cargo.toml"},
                {"id": "perl-lsp-rs 0.1.0", "name": "perl-lsp-rs", "manifest_path": "/w/crates/perl-lsp-rs/Cargo.toml"}
            ],
            "resolve": {
                "nodes": [
                    {
                        "id": "perl-parser 0.1.0",
                        "deps": []
                    },
                    {
                        "id": "perl-lsp-rs 0.1.0",
                        "deps": [{"pkg": "perl-parser 0.1.0", "name": "perl_parser", "dep_kinds": []}]
                    }
                ]
            }
        });
        let rev = build_reverse_dep_map(&metadata);
        let dependents = rev.get("perl-parser");
        assert!(dependents.is_some(), "perl-parser should have reverse deps");
        assert!(dependents.unwrap().contains("perl-lsp-rs"));
    }

    #[test]
    fn test_reverse_dep_closure_transitive() {
        // A → B → C: changing A should close over B and C
        let metadata = serde_json::json!({
            "packages": [
                {"id": "A 0.1.0", "name": "A", "manifest_path": "/w/crates/a/Cargo.toml"},
                {"id": "B 0.1.0", "name": "B", "manifest_path": "/w/crates/b/Cargo.toml"},
                {"id": "C 0.1.0", "name": "C", "manifest_path": "/w/crates/c/Cargo.toml"}
            ],
            "resolve": {
                "nodes": [
                    {"id": "A 0.1.0", "deps": []},
                    {"id": "B 0.1.0", "deps": [{"pkg": "A 0.1.0", "name": "a"}]},
                    {"id": "C 0.1.0", "deps": [{"pkg": "B 0.1.0", "name": "b"}]}
                ]
            }
        });
        let rev = build_reverse_dep_map(&metadata);
        let all_names: BTreeSet<String> = ["A", "B", "C"].iter().map(|s| s.to_string()).collect();
        let changed: BTreeSet<String> = ["A".to_string()].into();
        let closure = reverse_dep_closure(&changed, &rev, &all_names);
        assert!(closure.contains("B"), "B should be in closure");
        assert!(closure.contains("C"), "C should be in closure");
        assert!(!closure.contains("A"), "A itself should not be in the rev-dep closure");
    }

    #[test]
    fn test_apply_wideners_no_match() -> Result<()> {
        let changed = vec![CrateEntry {
            name: "some-unrelated-crate".to_string(),
            reason: "direct".to_string(),
        }];
        let widened = apply_wideners(&changed)?;
        assert!(widened.is_empty());
        Ok(())
    }

    #[test]
    fn test_apply_wideners_dedup() -> Result<()> {
        // Both parser and lexer target perl-lsp-rs; should appear once
        let changed = vec![
            CrateEntry { name: "perl-parser".to_string(), reason: "direct".to_string() },
            CrateEntry { name: "perl-lexer".to_string(), reason: "direct".to_string() },
        ];
        let widened = apply_wideners(&changed)?;
        let count = widened.iter().filter(|w| w.name == "perl-lsp-rs").count();
        assert_eq!(count, 1, "perl-lsp-rs should appear exactly once");
        Ok(())
    }
}
