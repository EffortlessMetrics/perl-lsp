//! CI scope classifier — `cargo xtask ci-scope`
//!
//! Computes:
//! 1. Changed files via `git diff --name-only <base>...HEAD`
//! 2. Maps files to crates via cargo metadata
//! 3. Computes reverse-dependency closure from the dep graph
//! 4. Applies architectural wideners (parser → LSP/DAP, etc.)
//! 5. Detects risk tags via path-prefix + keyword scan
//! 6. Emits a schema_version=2 JSON feedback plan with selected lanes
//!
//! Output is deterministic given the same diff + cargo metadata.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use color_eyre::eyre::{Context, Result, eyre};
use duct::cmd;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Public output types (schema_version 2)
// ---------------------------------------------------------------------------

/// A directly-changed crate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DirectCrate {
    pub name: String,
    pub reason: String,
}

/// A crate in the reverse-dependency closure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct RevDepCrate {
    pub name: String,
    pub reason: String,
}

/// A crate pulled in by an architectural widener.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArchWidener {
    pub name: String,
    pub rule: String,
}

/// A selected standard CI lane with its reason and scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneEntry {
    pub lane: String,
    pub scope: Vec<String>,
    pub reason: String,
}

/// A selected heavy CI lane (mutation, fuzz) promoted by risk tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeavyLaneEntry {
    pub lane: String,
    pub reason: String,
}

/// Platform override flags.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlatformOverrides {
    pub windows_runner: bool,
}

/// The full scope classifier output (schema_version 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeOutput {
    pub schema_version: u32,
    pub base: String,
    pub head_sha: String,
    pub changed_files: Vec<String>,
    pub diff_class: String,
    pub direct_crates: Vec<DirectCrate>,
    pub reverse_dep_closure: Vec<RevDepCrate>,
    pub architecture_wideners: Vec<ArchWidener>,
    pub risk_tags: Vec<String>,
    pub platform_overrides: PlatformOverrides,
    pub selected_lanes: Vec<LaneEntry>,
    pub selected_heavy_lanes: Vec<HeavyLaneEntry>,
    pub explanations: BTreeMap<String, String>,
}

// ---------------------------------------------------------------------------
// Diff classification
// ---------------------------------------------------------------------------

/// Prose-only extensions (never trigger CI lanes).
const PROSE_EXTENSIONS: &[&str] = &[".md", ".txt", ".rst", ".adoc", ".org"];

/// Docs-as-code extensions (may trigger validate-title / doc-build only).
const DOCS_AS_CODE_EXTENSIONS: &[&str] =
    &[".toml", ".yaml", ".yml", ".json", ".kdl", ".ron", ".jsonc"];

/// CI config paths/prefixes.
const CI_CONFIG_PATHS: &[&str] = &[".github/workflows/", ".ci/", "justfile", "Makefile"];

/// Classify a list of changed file paths into a diff_class string.
///
/// Classes: `prose_only`, `docs_as_code`, `ci_config`, `code`, `mixed`.
pub fn classify_diff(files: &[String]) -> String {
    if files.is_empty() {
        return "prose_only".to_string();
    }

    let mut has_prose = false;
    let mut has_docs_as_code = false;
    let mut has_ci_config = false;
    let mut has_code = false;

    for file in files {
        if is_prose_file(file) {
            has_prose = true;
        } else if is_ci_config_file(file) {
            has_ci_config = true;
        } else if is_docs_as_code_file(file) {
            has_docs_as_code = true;
        } else {
            // Assume Rust source / other code
            has_code = true;
        }
    }

    let class_count =
        [has_prose, has_docs_as_code, has_ci_config, has_code].iter().filter(|&&b| b).count();

    if class_count > 1 {
        return "mixed".to_string();
    }
    if has_ci_config {
        return "ci_config".to_string();
    }
    if has_docs_as_code {
        return "docs_as_code".to_string();
    }
    if has_prose {
        return "prose_only".to_string();
    }
    "code".to_string()
}

fn is_prose_file(file: &str) -> bool {
    PROSE_EXTENSIONS.iter().any(|ext| file.ends_with(ext))
        || file.starts_with("docs/")
        || file.starts_with(".github/ISSUE_TEMPLATE/")
        || file == "LICENSE"
        || file == "CHANGELOG"
}

fn is_ci_config_file(file: &str) -> bool {
    CI_CONFIG_PATHS.iter().any(|prefix| file.starts_with(prefix) || file == *prefix)
        || file.ends_with(".sh")
        || file.starts_with("scripts/")
        || file.starts_with("hooks/")
}

fn is_docs_as_code_file(file: &str) -> bool {
    DOCS_AS_CODE_EXTENSIONS.iter().any(|ext| file.ends_with(ext))
        && !is_prose_file(file)
        && !is_ci_config_file(file)
}

// ---------------------------------------------------------------------------
// Risk tag detection
// ---------------------------------------------------------------------------

/// Risk tag constants.
pub const RISK_TAG_CONCURRENCY: &str = "concurrency";
pub const RISK_TAG_PARSER_RECOVERY: &str = "parser_recovery";
pub const RISK_TAG_OFFSET_MATH: &str = "offset_math";
pub const RISK_TAG_PATH_NORMALIZATION: &str = "path_normalization";
pub const RISK_TAG_PERF_HOT_PATH: &str = "perf_hot_path";
pub const RISK_TAG_PUBLIC_API: &str = "public_api";
pub const RISK_TAG_DEP_CHANGE: &str = "dep_change";
pub const RISK_TAG_SECURITY_SURFACE: &str = "security_surface";

/// Public API facade crates (changing these → public_api risk tag).
const PUBLIC_API_CRATES: &[&str] =
    &["perl-parser", "perl-lsp-rs", "perl-dap", "perl-uri", "perl-lsp"];

/// Benchmarks directory (files referenced by benchmarks → perf_hot_path).
const BENCH_PATH_PREFIXES: &[&str] = &["benchmarks/", "benches/", "criterion/"];

/// Detect risk tags from a list of changed file paths and optionally their content.
///
/// This uses path-prefix heuristics only (no file reading), which is fast and
/// deterministic. Content-based keyword scanning can be added later.
pub fn detect_risk_tags(files: &[String], direct_crate_names: &[&str]) -> Vec<String> {
    let mut tags: BTreeSet<String> = BTreeSet::new();

    for file in files {
        // dep_change — Cargo manifests / lock file
        if file == "Cargo.toml"
            || file == "Cargo.lock"
            || file.ends_with("/Cargo.toml")
            || file.ends_with("/Cargo.lock")
        {
            tags.insert(RISK_TAG_DEP_CHANGE.to_string());
        }

        // parser_recovery — recovery/ dir or expressions/ dir under parser
        if file.contains("/recovery/")
            || file.contains("/expressions/")
            || (file.contains("perl-parser") && file.contains("recovery"))
        {
            tags.insert(RISK_TAG_PARSER_RECOVERY.to_string());
        }

        // offset_math — UTF-8 / column / line conversion files
        if file.contains("position")
            || file.contains("offset")
            || file.contains("utf")
            || file.contains("column")
        {
            tags.insert(RISK_TAG_OFFSET_MATH.to_string());
        }

        // path_normalization — URI / workspace-folder / file-URI parsing
        if file.contains("uri") || file.contains("workspace-folder") || file.contains("file_uri") {
            tags.insert(RISK_TAG_PATH_NORMALIZATION.to_string());
        }

        // perf_hot_path — files in benchmark directories
        if BENCH_PATH_PREFIXES.iter().any(|prefix| file.starts_with(prefix)) {
            tags.insert(RISK_TAG_PERF_HOT_PATH.to_string());
        }

        // security_surface — auth, eval, shell exec, deserialization
        if file.contains("auth")
            || file.contains("eval")
            || file.contains("exec")
            || file.contains("deserializ")
            || file.contains("shell")
        {
            tags.insert(RISK_TAG_SECURITY_SURFACE.to_string());
        }

        // concurrency — files with async/Arc/Mutex/RwLock (heuristic: just check path tokens)
        if file.contains("async")
            || file.contains("concurrent")
            || file.contains("thread")
            || file.contains("mutex")
            || file.contains("rwlock")
            || file.contains("arc")
        {
            tags.insert(RISK_TAG_CONCURRENCY.to_string());
        }
    }

    // public_api — direct changes to facade crates
    for crate_name in direct_crate_names {
        if PUBLIC_API_CRATES.iter().any(|facade| crate_name == facade) {
            tags.insert(RISK_TAG_PUBLIC_API.to_string());
        }
    }

    tags.into_iter().collect()
}

// ---------------------------------------------------------------------------
// Architectural widener rules (const, for testability)
// ---------------------------------------------------------------------------

/// A single widener rule: when any of the `trigger_prefixes` crates change,
/// add `targets` to the widened set with the given `rule` description.
pub struct WidenerRule {
    /// Crate name prefixes that trigger this rule (exact match or prefix match).
    pub trigger_prefixes: &'static [&'static str],
    /// Crates to add to the widened set.
    pub targets: &'static [&'static str],
    /// Human-readable rule description (appears in architecture_wideners[].rule).
    pub rule: &'static str,
    /// Lane to select when this rule fires.
    pub lanes: &'static [&'static str],
    /// Lane reason tag.
    pub lane_reason: &'static str,
}

/// All architectural widener rules. The order matters for lane generation
/// (earlier rules fire first), but deduplication ensures idempotent output.
pub static WIDENER_RULES: &[WidenerRule] = &[
    // Rule 1: parser / lexer / parser-core → semantic, workspace-index, LSP, DAP
    WidenerRule {
        trigger_prefixes: &["perl-parser", "perl-lexer", "perl-parser-core"],
        targets: &["perl-semantic-analyzer", "perl-workspace-index", "perl-lsp-rs", "perl-dap"],
        rule: "parser → DAP downstream smoke",
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
        rule: "semantic → LSP definition/references/rename",
        lanes: &["lsp_providers"],
        lane_reason: "architectural_widener",
    },
    // Rule 3: LSP/DAP crates + features.toml → UX regression
    WidenerRule {
        trigger_prefixes: &["perl-lsp-", "perl-dap"],
        targets: &["perl-lsp-rs"],
        rule: "lsp/dap change → UX regression",
        lanes: &["ux_regression"],
        lane_reason: "architectural_widener",
    },
];

// ---------------------------------------------------------------------------
// Heavy-lane promotion from risk tags
// ---------------------------------------------------------------------------

/// Returns heavy lanes promoted by the given set of risk tags.
pub fn heavy_lanes_from_risk_tags(
    risk_tags: &[String],
    direct_crates: &[DirectCrate],
) -> Vec<HeavyLaneEntry> {
    let mut heavy: Vec<HeavyLaneEntry> = Vec::new();

    if risk_tags.contains(&RISK_TAG_PARSER_RECOVERY.to_string()) {
        heavy.push(HeavyLaneEntry {
            lane: "bounded_parser_fuzz".to_string(),
            reason: format!("risk_tag: {RISK_TAG_PARSER_RECOVERY}"),
        });
    }
    if risk_tags.contains(&RISK_TAG_CONCURRENCY.to_string()) {
        heavy.push(HeavyLaneEntry {
            lane: "thread_sanitizer".to_string(),
            reason: format!("risk_tag: {RISK_TAG_CONCURRENCY}"),
        });
    }
    if risk_tags.contains(&RISK_TAG_PERF_HOT_PATH.to_string()) {
        heavy.push(HeavyLaneEntry {
            lane: "perf_regression".to_string(),
            reason: format!("risk_tag: {RISK_TAG_PERF_HOT_PATH}"),
        });
    }
    if risk_tags.contains(&RISK_TAG_SECURITY_SURFACE.to_string()) {
        heavy.push(HeavyLaneEntry {
            lane: "security_audit".to_string(),
            reason: format!("risk_tag: {RISK_TAG_SECURITY_SURFACE}"),
        });
    }

    // mutation_diff: default lane for any code diff (direct crate changes)
    if !direct_crates.is_empty() {
        let scope: Vec<String> = direct_crates.iter().map(|c| c.name.clone()).collect();
        heavy.push(HeavyLaneEntry {
            lane: "mutation_diff".to_string(),
            reason: format!("code_diff_default (crates: {})", scope.join(", ")),
        });
    }

    heavy
}

// ---------------------------------------------------------------------------
// File-to-crate resolution helpers
// ---------------------------------------------------------------------------

/// Returns true if the changed files include workspace root files that trigger
/// the full-workspace scope (Cargo.toml, Cargo.lock, workflow files, hooks, justfile).
fn is_workspace_root_change(files: &[String]) -> bool {
    files.iter().any(|f| {
        matches!(f.as_str(), "Cargo.toml" | "Cargo.lock" | "justfile")
            || f.starts_with(".github/workflows/")
            || f.starts_with("hooks/")
    })
}

/// Returns true if the changed files include `features.toml`.
fn has_features_toml_change(files: &[String]) -> bool {
    files.iter().any(|f| f == "features.toml")
}

/// Extract unique crate names from cargo metadata JSON for crate dirs in the changed files.
fn crates_from_files(
    files: &[String],
    metadata: &serde_json::Value,
    workspace_root: &str,
) -> Result<BTreeSet<String>> {
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
fn build_reverse_dep_map(metadata: &serde_json::Value) -> BTreeMap<String, BTreeSet<String>> {
    let mut rev_deps: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    let nodes = match metadata.pointer("/resolve/nodes").and_then(|n| n.as_array()) {
        Some(n) => n,
        None => return rev_deps,
    };

    // Build id → name map first
    let mut id_to_name: BTreeMap<String, String> = BTreeMap::new();
    if let Some(pkgs) = metadata.get("packages").and_then(|p| p.as_array()) {
        for pkg in pkgs {
            let id = pkg.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let name = pkg.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if !id.is_empty() && !name.is_empty() {
                id_to_name.insert(id.to_string(), name.to_string());
            }
        }
    }

    for node in nodes {
        let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let node_name = match id_to_name.get(node_id) {
            Some(n) => n.clone(),
            None => node_id.split(':').next().unwrap_or(node_id).to_string(),
        };

        if let Some(deps) = node.get("deps").and_then(|d| d.as_array()) {
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
/// Returns a schema_version 2 `ScopeOutput`. `workspace_root` is the absolute
/// path prefix used in manifest_path fields (e.g. `"/path/to/project"`).
/// In tests, pass a fake root like `"/workspace"`.
pub fn classify_files(
    files: &[String],
    metadata: &serde_json::Value,
    workspace_root: &str,
) -> Result<ScopeOutput> {
    let diff_class = classify_diff(files);

    // Empty diff or prose-only → empty output (no lanes)
    if files.is_empty() || diff_class == "prose_only" {
        return Ok(ScopeOutput {
            schema_version: 2,
            base: String::new(),
            head_sha: String::new(),
            changed_files: files.to_vec(),
            diff_class,
            direct_crates: vec![],
            reverse_dep_closure: vec![],
            architecture_wideners: vec![],
            risk_tags: vec![],
            platform_overrides: PlatformOverrides::default(),
            selected_lanes: vec![],
            selected_heavy_lanes: vec![],
            explanations: BTreeMap::new(),
        });
    }

    let mut lanes: Vec<LaneEntry> = vec![];
    let mut explanations: BTreeMap<String, String> = BTreeMap::new();

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

    // Map changed files → direct crate names
    let directly_changed_set = crates_from_files(files, metadata, workspace_root)?;
    let direct_crates: Vec<DirectCrate> = directly_changed_set
        .iter()
        .map(|name| DirectCrate { name: name.clone(), reason: "direct".to_string() })
        .collect();

    // Build reverse-dep map and compute closure
    let rev_deps = build_reverse_dep_map(metadata);
    let rev_dep_set = reverse_dep_closure(&directly_changed_set, &rev_deps, &all_package_names);
    let reverse_dep_closure_vec: Vec<RevDepCrate> = rev_dep_set
        .iter()
        .filter(|name| !directly_changed_set.contains(*name))
        .map(|name| RevDepCrate {
            name: name.clone(),
            reason: format!(
                "reverse-dep of {}",
                directly_changed_set.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
        })
        .collect();

    // Risk tag detection
    let direct_names: Vec<&str> = direct_crates.iter().map(|c| c.name.as_str()).collect();
    let risk_tags = detect_risk_tags(files, &direct_names);

    // Apply architectural wideners
    let (arch_wideners, widener_lanes, widener_explanations) = apply_wideners_v2(&direct_crates)?;
    lanes.extend(widener_lanes);
    explanations.extend(widener_explanations);

    // Workspace-root changes: trigger infra lanes
    if is_workspace_root_change(files) {
        for lane_name in &["publish", "security", "ci_policy"] {
            lanes.push(LaneEntry {
                lane: lane_name.to_string(),
                reason: "workspace_root".to_string(),
                scope: vec![],
            });
        }
        explanations.insert(
            "publish".to_string(),
            "Workspace root files (Cargo.toml/Lock, workflows, hooks) trigger publish + security + CI-policy checks".to_string(),
        );
    }

    // features.toml change: UX regression lane
    if has_features_toml_change(files) {
        let already = lanes.iter().any(|l| l.lane == "ux_regression");
        if !already {
            lanes.push(LaneEntry {
                lane: "ux_regression".to_string(),
                reason: "features_toml".to_string(),
                scope: vec!["perl-lsp-rs".to_string()],
            });
        }
        explanations.insert(
            "ux_regression".to_string(),
            "features.toml change triggers UX regression check (per #4706)".to_string(),
        );
    }

    // Scoped lanes for all changed crates (direct + rev-dep)
    let all_scope: Vec<String> = {
        let mut s: Vec<String> = direct_crates.iter().map(|c| c.name.clone()).collect();
        for c in &reverse_dep_closure_vec {
            if !s.contains(&c.name) {
                s.push(c.name.clone());
            }
        }
        s.sort();
        s
    };

    if !all_scope.is_empty() {
        lanes.push(LaneEntry {
            lane: "clippy_scoped".to_string(),
            scope: all_scope.clone(),
            reason: "direct".to_string(),
        });
        lanes.push(LaneEntry {
            lane: "test_scoped".to_string(),
            scope: all_scope.clone(),
            reason: "direct".to_string(),
        });
    }

    // mutation_diff default lane for code changes
    let heavy_lanes = heavy_lanes_from_risk_tags(&risk_tags, &direct_crates);

    // Platform overrides (currently static — can be extended)
    let platform_overrides = PlatformOverrides { windows_runner: false };

    Ok(ScopeOutput {
        schema_version: 2,
        base: String::new(),
        head_sha: String::new(),
        changed_files: files.to_vec(),
        diff_class,
        direct_crates,
        reverse_dep_closure: reverse_dep_closure_vec,
        architecture_wideners: arch_wideners,
        risk_tags,
        platform_overrides,
        selected_lanes: lanes,
        selected_heavy_lanes: heavy_lanes,
        explanations,
    })
}

/// Apply architectural widening rules to a set of directly-changed crates.
///
/// Returns (wideners, lanes, explanations) for insertion into the output.
pub fn apply_wideners_v2(
    direct_crates: &[DirectCrate],
) -> Result<(Vec<ArchWidener>, Vec<LaneEntry>, BTreeMap<String, String>)> {
    let mut widened: BTreeMap<String, String> = BTreeMap::new();
    let mut lanes: Vec<LaneEntry> = Vec::new();
    let mut explanations: BTreeMap<String, String> = BTreeMap::new();
    let mut seen_lanes: BTreeSet<String> = BTreeSet::new();

    for rule in WIDENER_RULES {
        let triggered = direct_crates.iter().any(|c| {
            rule.trigger_prefixes
                .iter()
                .any(|prefix| c.name == *prefix || c.name.starts_with(prefix))
        });

        if triggered {
            for target in rule.targets {
                widened.entry(target.to_string()).or_insert_with(|| rule.rule.to_string());
            }

            for lane_name in rule.lanes {
                if !seen_lanes.contains(*lane_name) {
                    seen_lanes.insert(lane_name.to_string());
                    let scope: Vec<String> = rule.targets.iter().map(|s| s.to_string()).collect();
                    lanes.push(LaneEntry {
                        lane: lane_name.to_string(),
                        scope,
                        reason: rule.lane_reason.to_string(),
                    });
                    explanations.insert(
                        lane_name.to_string(),
                        format!("Architectural rule: {}", rule.rule),
                    );
                }
            }
        }
    }

    let arch_wideners: Vec<ArchWidener> =
        widened.into_iter().map(|(name, rule)| ArchWidener { name, rule }).collect();

    Ok((arch_wideners, lanes, explanations))
}

// ---------------------------------------------------------------------------
// CLI config + entry point
// ---------------------------------------------------------------------------

/// Configuration for the `ci-scope` subcommand.
pub struct CiScopeConfig {
    /// Base git ref to diff against (e.g. "origin/main" or "auto").
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
    output.base = base_ref.clone();
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
    let mut candidates = Vec::new();
    if base != "auto" {
        candidates.push(base.to_string());
    }
    candidates.extend(
        ["origin/main", "origin/master", "main", "master", "HEAD~1", "HEAD"]
            .into_iter()
            .map(str::to_string),
    );

    for candidate in candidates {
        if git_ref_exists(&candidate, root)? {
            if base != "auto" && candidate != base {
                eprintln!(
                    "Warning: base ref '{}' not found; using fallback '{}'",
                    base, candidate
                );
            }
            return Ok(candidate);
        }
    }

    Err(color_eyre::eyre::eyre!(
        "Could not resolve a valid base ref from '{}', origin/main, origin/master, main, master, HEAD~1, or HEAD",
        base
    ))
}

fn git_ref_exists(candidate: &str, root: &Path) -> Result<bool> {
    let verify = cmd("git", &["rev-parse", "--verify", candidate])
        .dir(root)
        .stdout_null()
        .stderr_null()
        .unchecked()
        .run()
        .context("Failed to run git rev-parse")?;
    Ok(verify.status.success())
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
    println!("=== CI Scope Classifier (schema v{}) ===", output.schema_version);
    println!("Base:       {}", output.base);
    println!("HEAD SHA:   {}", output.head_sha);
    println!("Diff class: {}", output.diff_class);
    println!("Changed files: {}", output.changed_files.len());

    if output.direct_crates.is_empty() {
        println!("Direct crates: (none)");
    } else {
        println!("Direct crates:");
        for c in &output.direct_crates {
            println!("  [{}] {}", c.reason, c.name);
        }
    }

    if !output.reverse_dep_closure.is_empty() {
        println!("Reverse-dep closure:");
        for c in &output.reverse_dep_closure {
            println!("  {}", c.name);
        }
    }

    if !output.architecture_wideners.is_empty() {
        println!("Architecture wideners:");
        for w in &output.architecture_wideners {
            println!("  {} — {}", w.name, w.rule);
        }
    }

    if !output.risk_tags.is_empty() {
        println!("Risk tags: {}", output.risk_tags.join(", "));
    }

    if output.selected_lanes.is_empty() {
        println!("Selected lanes: (none)");
    } else {
        println!("Selected lanes:");
        for l in &output.selected_lanes {
            println!("  [{}] {} — {:?}", l.reason, l.lane, l.scope);
        }
    }

    if !output.selected_heavy_lanes.is_empty() {
        println!("Heavy lanes:");
        for l in &output.selected_heavy_lanes {
            println!("  {} — {}", l.lane, l.reason);
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

    // --- diff_class tests ---

    #[test]
    fn test_classify_diff_prose_only() {
        let files = vec!["docs/reference/STABILITY.md".to_string(), "README.md".to_string()];
        assert_eq!(classify_diff(&files), "prose_only");
    }

    #[test]
    fn test_classify_diff_empty_is_prose_only() {
        assert_eq!(classify_diff(&[]), "prose_only");
    }

    #[test]
    fn test_classify_diff_code() {
        let files = vec!["crates/perl-parser/src/lib.rs".to_string()];
        assert_eq!(classify_diff(&files), "code");
    }

    #[test]
    fn test_classify_diff_ci_config() {
        let files = vec![".github/workflows/ci.yml".to_string()];
        assert_eq!(classify_diff(&files), "ci_config");
    }

    #[test]
    fn test_classify_diff_mixed_code_and_docs() {
        let files = vec![
            "crates/perl-parser/src/lib.rs".to_string(),
            "docs/reference/STABILITY.md".to_string(),
        ];
        assert_eq!(classify_diff(&files), "mixed");
    }

    // --- risk tag tests ---

    #[test]
    fn test_risk_tag_parser_recovery() {
        let files = vec!["crates/perl-parser/src/expressions/recovery.rs".to_string()];
        let tags = detect_risk_tags(&files, &[]);
        assert!(
            tags.contains(&RISK_TAG_PARSER_RECOVERY.to_string()),
            "should detect parser_recovery"
        );
    }

    #[test]
    fn test_risk_tag_dep_change_cargo_toml() {
        let files = vec!["Cargo.toml".to_string()];
        let tags = detect_risk_tags(&files, &[]);
        assert!(tags.contains(&RISK_TAG_DEP_CHANGE.to_string()));
    }

    #[test]
    fn test_risk_tag_public_api() {
        let tags = detect_risk_tags(&[], &["perl-parser"]);
        assert!(tags.contains(&RISK_TAG_PUBLIC_API.to_string()));
    }

    #[test]
    fn test_risk_tag_none_for_unrelated() {
        let files = vec!["crates/perl-parser/src/stmt.rs".to_string()];
        let tags = detect_risk_tags(&files, &["perl-parser"]);
        // public_api will be set, but no other tags
        assert!(tags.contains(&RISK_TAG_PUBLIC_API.to_string()));
        assert!(!tags.contains(&RISK_TAG_PARSER_RECOVERY.to_string()));
        assert!(!tags.contains(&RISK_TAG_DEP_CHANGE.to_string()));
    }

    // --- crates_from_files tests ---

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

    // --- reverse dep tests ---

    #[test]
    fn test_build_reverse_dep_map_basic() {
        let metadata = serde_json::json!({
            "packages": [
                {"id": "perl-parser 0.1.0", "name": "perl-parser", "manifest_path": "/w/crates/perl-parser/Cargo.toml"},
                {"id": "perl-lsp-rs 0.1.0", "name": "perl-lsp-rs", "manifest_path": "/w/crates/perl-lsp-rs/Cargo.toml"}
            ],
            "resolve": {
                "nodes": [
                    {"id": "perl-parser 0.1.0", "deps": []},
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
        assert!(
            dependents.is_some_and(|d| d.contains("perl-lsp-rs")),
            "perl-parser dependents should include perl-lsp-rs"
        );
    }

    #[test]
    fn test_reverse_dep_closure_transitive() {
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

    // --- widener tests ---

    #[test]
    fn test_apply_wideners_v2_no_match() -> Result<()> {
        let changed = vec![DirectCrate {
            name: "some-unrelated-crate".to_string(),
            reason: "direct".to_string(),
        }];
        let (wideners, lanes, _) = apply_wideners_v2(&changed)?;
        assert!(wideners.is_empty());
        assert!(lanes.is_empty());
        Ok(())
    }

    #[test]
    fn test_apply_wideners_v2_dedup() -> Result<()> {
        let changed = vec![
            DirectCrate { name: "perl-parser".to_string(), reason: "direct".to_string() },
            DirectCrate { name: "perl-lexer".to_string(), reason: "direct".to_string() },
        ];
        let (wideners, _, _) = apply_wideners_v2(&changed)?;
        let count = wideners.iter().filter(|w| w.name == "perl-lsp-rs").count();
        assert_eq!(count, 1, "perl-lsp-rs should appear exactly once");
        Ok(())
    }

    #[test]
    fn test_apply_wideners_v2_parser_triggers_lsp_smoke() -> Result<()> {
        let changed =
            vec![DirectCrate { name: "perl-parser".to_string(), reason: "direct".to_string() }];
        let (_, lanes, explanations) = apply_wideners_v2(&changed)?;
        assert!(
            lanes.iter().any(|l| l.lane == "lsp_smoke"),
            "parser change should add lsp_smoke lane"
        );
        assert!(explanations.contains_key("lsp_smoke"), "lsp_smoke should have an explanation");
        Ok(())
    }

    // --- classify_files integration tests ---

    #[test]
    fn test_classify_files_empty_diff() -> Result<()> {
        let metadata = fake_metadata(&[("perl-parser", "crates/perl-parser")]);
        let output = classify_files(&[], &metadata, "/workspace")?;
        assert_eq!(output.schema_version, 2);
        assert_eq!(output.diff_class, "prose_only");
        assert!(output.selected_lanes.is_empty());
        assert!(output.selected_heavy_lanes.is_empty());
        Ok(())
    }

    #[test]
    fn test_classify_files_docs_only() -> Result<()> {
        let metadata = fake_metadata(&[("perl-parser", "crates/perl-parser")]);
        let files = vec!["docs/reference/STABILITY.md".to_string(), "README.md".to_string()];
        let output = classify_files(&files, &metadata, "/workspace")?;
        assert_eq!(output.diff_class, "prose_only");
        assert!(output.selected_lanes.is_empty(), "docs-only should have no lanes");
        Ok(())
    }

    #[test]
    fn test_classify_files_code_diff_has_mutation_diff() -> Result<()> {
        let metadata = fake_metadata(&[("perl-parser", "crates/perl-parser")]);
        let files = vec!["crates/perl-parser/src/lib.rs".to_string()];
        let output = classify_files(&files, &metadata, "/workspace")?;
        assert_eq!(output.diff_class, "code");
        assert!(
            output.selected_heavy_lanes.iter().any(|l| l.lane == "mutation_diff"),
            "code diff should include mutation_diff heavy lane"
        );
        Ok(())
    }

    #[test]
    fn test_classify_files_parser_recovery_file_triggers_fuzz() -> Result<()> {
        let metadata = fake_metadata(&[("perl-parser", "crates/perl-parser")]);
        let files = vec!["crates/perl-parser/src/expressions/recovery.rs".to_string()];
        let output = classify_files(&files, &metadata, "/workspace")?;
        assert!(
            output.risk_tags.contains(&RISK_TAG_PARSER_RECOVERY.to_string()),
            "recovery file should trigger parser_recovery tag"
        );
        assert!(
            output.selected_heavy_lanes.iter().any(|l| l.lane == "bounded_parser_fuzz"),
            "parser_recovery tag should promote bounded_parser_fuzz"
        );
        Ok(())
    }

    #[test]
    fn test_classify_files_cargo_toml_triggers_dep_change() -> Result<()> {
        let metadata = fake_metadata(&[("perl-parser", "crates/perl-parser")]);
        let files = vec!["Cargo.toml".to_string()];
        let output = classify_files(&files, &metadata, "/workspace")?;
        assert!(output.risk_tags.contains(&RISK_TAG_DEP_CHANGE.to_string()));
        assert!(output.selected_lanes.iter().any(|l| l.lane == "publish"));
        assert!(output.selected_lanes.iter().any(|l| l.lane == "security"));
        Ok(())
    }

    // --- heavy_lanes_from_risk_tags tests ---

    #[test]
    fn test_heavy_lanes_mutation_diff_default() {
        let direct =
            vec![DirectCrate { name: "perl-parser".to_string(), reason: "direct".to_string() }];
        let heavy = heavy_lanes_from_risk_tags(&[], &direct);
        assert!(
            heavy.iter().any(|l| l.lane == "mutation_diff"),
            "should include mutation_diff by default"
        );
    }

    #[test]
    fn test_heavy_lanes_empty_when_no_direct_crates() {
        let heavy = heavy_lanes_from_risk_tags(&[], &[]);
        assert!(
            !heavy.iter().any(|l| l.lane == "mutation_diff"),
            "no direct crates = no mutation_diff"
        );
    }
}
