//! Verify transitive normal-dep closure of published crates.
//!
//! A published crate must not have any transitive normal dependency on a
//! workspace member that has `publish = false`.  If it does, `cargo publish`
//! will fail for downstream users who try to add it as a dependency.
//!
//! Algorithm
//! ---------
//! 1. Run `cargo metadata --format-version 1` (without `--no-deps`) so the
//!    response includes the full `resolve` graph.
//! 2. Read `[workspace.metadata.publish.allow]` from the root `Cargo.toml` to
//!    get the set of crates that are published to crates.io.
//! 3. Collect workspace members with `publish = []` (i.e. `publish = false`).
//! 4. For every published crate (or just the one supplied via `--crate-name`),
//!    BFS-walk the normal-dep edges in the resolve graph.  Report any visit to
//!    a `publish = false` workspace member as a violation.
//! 5. Exit non-zero if any violations were found.

use crate::utils::run_cargo_metadata;
use color_eyre::eyre::{Result, bail, eyre};
use serde::Deserialize;
use std::collections::{HashMap, HashSet, VecDeque};

// ---------------------------------------------------------------------------
// Cargo metadata types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct FullMetadata {
    packages: Vec<FullPackage>,
    workspace_members: Vec<String>,
    #[serde(rename = "metadata")]
    workspace_metadata: Option<WorkspacePublishMeta>,
    resolve: Option<ResolveGraph>,
}

#[derive(Deserialize)]
struct FullPackage {
    name: String,
    id: String,
    /// `None` means "publish everywhere"; `Some([])` means `publish = false`.
    publish: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct WorkspacePublishMeta {
    publish: Option<AllowList>,
}

#[derive(Deserialize)]
struct AllowList {
    allow: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct ResolveGraph {
    nodes: Vec<ResolveNode>,
}

#[derive(Deserialize)]
struct ResolveNode {
    id: String,
    deps: Vec<ResolveDep>,
}

#[derive(Deserialize)]
struct ResolveDep {
    /// The package ID of the dependency. Note: this field is called `pkg` in
    /// the cargo metadata JSON, NOT `id`.
    pkg: String,
    dep_kinds: Vec<DepKind>,
}

#[derive(Deserialize)]
struct DepKind {
    /// `null` = normal dep, `"dev"` = dev dep, `"build"` = build dep.
    kind: Option<String>,
    /// Platform filter (e.g. `cfg(windows)`). Present but does not affect
    /// whether the dep is a violation.
    #[allow(dead_code)]
    target: Option<String>,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the publish-closure gate.
///
/// If `crate_filter` is `Some(name)`, only that crate is checked.  Otherwise
/// all crates in the allowlist are checked.
pub fn run(crate_filter: Option<String>) -> Result<()> {
    let metadata = load_metadata()?;

    // Load [workspace.metadata.publish.allow]
    let allowlist = metadata
        .workspace_metadata
        .as_ref()
        .and_then(|m| m.publish.as_ref())
        .and_then(|p| p.allow.as_ref())
        .ok_or_else(|| eyre!("No [workspace.metadata.publish.allow] found in Cargo.toml"))?;

    // Build set of workspace-member package IDs (used to restrict violations
    // to workspace-local crates only — external registry crates are never flagged).
    let workspace_member_ids: HashSet<&str> =
        metadata.workspace_members.iter().map(String::as_str).collect();

    // Collect the names of workspace members that have `publish = false`.
    let no_publish_names: HashSet<String> = metadata
        .packages
        .iter()
        .filter(|pkg| workspace_member_ids.contains(pkg.id.as_str()) && pkg.publish == Some(vec![]))
        .map(|pkg| pkg.name.clone())
        .collect();

    // Determine which crates to check.
    let crates_to_check: Vec<&String> = if let Some(ref filter) = crate_filter {
        if !allowlist.contains(filter) {
            bail!("Crate '{}' not found in publish allowlist", filter);
        }
        vec![filter]
    } else {
        allowlist.iter().collect()
    };

    // Build package_id -> name mapping.
    let id_to_name: HashMap<&str, &str> =
        metadata.packages.iter().map(|pkg| (pkg.id.as_str(), pkg.name.as_str())).collect();

    // Build name -> package_id mapping (for root lookup).
    let name_to_id: HashMap<&str, &str> =
        metadata.packages.iter().map(|pkg| (pkg.name.as_str(), pkg.id.as_str())).collect();

    // Build the normal-dep resolve graph: pkg_id -> [normal dep pkg_ids].
    let resolve_graph: HashMap<&str, Vec<&str>> =
        metadata.resolve.as_ref().map(|r| build_normal_dep_graph(r)).unwrap_or_default();

    // Walk each crate and collect violations.
    let mut violations: Vec<(String, String)> = Vec::new();
    for crate_name in &crates_to_check {
        let Some(&start_id) = name_to_id.get(crate_name.as_str()) else {
            // Crate is in the allowlist but not found in metadata — unlikely
            // but skip gracefully rather than panic.
            continue;
        };
        let bad =
            check_transitive_closure(start_id, &resolve_graph, &no_publish_names, &id_to_name);
        for forbidden in bad {
            violations.push(((*crate_name).clone(), forbidden));
        }
    }

    // Report all violations before deciding exit code.
    if !violations.is_empty() {
        for (published, forbidden) in &violations {
            eprintln!("ERROR: publish-closure violation");
            eprintln!(
                "  Published crate `{}` has transitive normal dep on `{}` (publish = false)",
                published, forbidden
            );
        }
        bail!("publish-closure check failed ({} violation(s))", violations.len());
    }

    let count = crates_to_check.len();
    println!(
        "publish-closure: OK ({} crate{} checked, 0 violations)",
        count,
        if count == 1 { "" } else { "s" }
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn load_metadata() -> Result<FullMetadata> {
    let bytes = run_cargo_metadata(false)?;
    let metadata: FullMetadata = serde_json::from_slice(&bytes)
        .map_err(|e| eyre!("Failed to parse cargo metadata JSON: {}", e))?;
    Ok(metadata)
}

/// Build a map from package ID to the list of *normal* (non-dev, non-build)
/// dependency package IDs.
fn build_normal_dep_graph(resolve: &ResolveGraph) -> HashMap<&str, Vec<&str>> {
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in &resolve.nodes {
        let normal_deps: Vec<&str> =
            node.deps.iter().filter(|dep| is_normal_dep(dep)).map(|dep| dep.pkg.as_str()).collect();
        graph.insert(node.id.as_str(), normal_deps);
    }
    graph
}

/// Returns `true` if this dependency edge is a *normal* (non-dev, non-build)
/// dependency.
///
/// A dep edge can have multiple `dep_kinds` entries when it is used in
/// multiple roles (e.g. both as a normal dep and a dev dep).  We treat it as
/// a normal dep if *any* of its dep_kinds has `kind == null`.
///
/// An empty `dep_kinds` list is treated conservatively as a normal dep.
fn is_normal_dep(dep: &ResolveDep) -> bool {
    dep.dep_kinds.is_empty() || dep.dep_kinds.iter().any(|dk| dk.kind.is_none())
}

/// BFS from `start_id` following only normal-dep edges.  Returns the names of
/// any visited workspace members that have `publish = false`.
fn check_transitive_closure<'a>(
    start_id: &'a str,
    graph: &'a HashMap<&str, Vec<&'a str>>,
    no_publish_names: &HashSet<String>,
    id_to_name: &'a HashMap<&str, &'a str>,
) -> Vec<String> {
    let mut visited: HashSet<&str> = HashSet::new();
    let mut queue: VecDeque<&str> = VecDeque::new();
    let mut bad: Vec<String> = Vec::new();

    // Seed with the starting crate itself (skip it in the violation check).
    visited.insert(start_id);
    queue.push_back(start_id);

    while let Some(current_id) = queue.pop_front() {
        let Some(deps) = graph.get(current_id) else {
            continue;
        };
        for &dep_id in deps {
            if visited.contains(dep_id) {
                continue;
            }
            visited.insert(dep_id);
            if let Some(&dep_name) = id_to_name.get(dep_id)
                && no_publish_names.contains(dep_name)
            {
                bad.push(dep_name.to_string());
            }
            queue.push_back(dep_id);
        }
    }

    bad
}
