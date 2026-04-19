//! Offline manifest validation for the publish pipeline.
//!
//! Two checks (both use `cargo metadata --no-deps`; no network contact):
//!
//! 1. **Allowlist drift** — every entry in `[workspace.metadata.publish.allow]`
//!    must be a publishable workspace member (`publish` field is `None` or
//!    non-empty), AND every publishable workspace member must appear in the
//!    allowlist.  Replaces the `scripts/publish-topo.py --check-drift`
//!    invocation in `publish-dry-run.yml`.
//!
//! 2. **LICENSE present** — every allowlisted crate must carry a non-empty
//!    `license` or `license_file` field in the cargo metadata output.
//!    Workspace-inherited values (`license.workspace = true`) are already
//!    resolved to the actual string by cargo before this code sees them.
//!
//! Prints all violations before exiting non-zero so the developer can fix
//! everything in one pass.

use crate::utils::{load_publish_allowlist, run_cargo_metadata};
use color_eyre::eyre::{Result, bail, eyre};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Cargo metadata types (no-deps variant)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub(crate) struct NoDepsMetadata {
    packages: Vec<NoDepsPackage>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct NoDepsPackage {
    name: String,
    id: String,
    /// `None` = publish everywhere; `Some([])` = `publish = false`.
    publish: Option<Vec<String>>,
    license: Option<String>,
    license_file: Option<String>,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Entry point for `cargo xtask publish-manifest-check`.
///
/// Runs allowlist-drift and LICENSE-present checks against the current
/// workspace state.  Exits non-zero if any violations are found.
pub fn run() -> Result<()> {
    let allowlist = load_publish_allowlist()?;
    let bytes = run_cargo_metadata(true)?;
    let meta: NoDepsMetadata =
        serde_json::from_slice(&bytes).map_err(|e| eyre!("Failed to parse cargo metadata: {e}"))?;

    let violations = check_metadata(&meta, &allowlist);

    if !violations.is_empty() {
        for v in &violations {
            eprintln!("ERROR: publish-manifest-check: {v}");
        }
        bail!("publish-manifest-check failed ({} violation(s))", violations.len());
    }

    println!("publish-manifest-check: OK ({} crates checked, 0 violations)", allowlist.len());
    Ok(())
}

// ---------------------------------------------------------------------------
// Pure check logic (extracted for unit tests)
// ---------------------------------------------------------------------------

/// Run all manifest checks and return violation messages (empty = pass).
///
/// Extracted as a pure function so unit tests can drive it without spawning
/// a real `cargo metadata` process.
pub(crate) fn check_metadata(meta: &NoDepsMetadata, allowlist: &[String]) -> Vec<String> {
    let allowlist_set: HashSet<&str> = allowlist.iter().map(String::as_str).collect();
    let ws_ids: HashSet<&str> = meta.workspace_members.iter().map(String::as_str).collect();

    // Map name -> package for workspace members only.
    let pkg_map: HashMap<&str, &NoDepsPackage> = meta
        .packages
        .iter()
        .filter(|p| ws_ids.contains(p.id.as_str()))
        .map(|p| (p.name.as_str(), p))
        .collect();

    // Publishable = workspace member whose `publish` field is `None` or non-empty.
    let publishable: HashSet<&str> = meta
        .packages
        .iter()
        .filter(|p| ws_ids.contains(p.id.as_str()))
        .filter(|p| p.publish.as_ref().is_none_or(|v| !v.is_empty()))
        .map(|p| p.name.as_str())
        .collect();

    let mut violations: Vec<String> = Vec::new();

    // Drift A: allowlist entry is not a publishable workspace member.
    for name in allowlist {
        if !publishable.contains(name.as_str()) {
            violations.push(format!(
                "drift: '{name}' is in [workspace.metadata.publish.allow] but \
                 has publish=false in Cargo.toml (or is not a workspace member)"
            ));
        }
    }

    // Drift B: publishable workspace member absent from allowlist.
    for name in &publishable {
        if !allowlist_set.contains(*name) {
            violations.push(format!(
                "drift: '{name}' is publishable (no publish=false) but is \
                 absent from [workspace.metadata.publish.allow]"
            ));
        }
    }

    // LICENSE check for every allowlisted crate.
    for name in allowlist {
        let Some(pkg) = pkg_map.get(name.as_str()) else {
            // Already flagged by drift check A.
            continue;
        };
        let has_license = pkg.license.as_deref().is_some_and(|l| !l.is_empty())
            || pkg.license_file.as_deref().is_some_and(|f| !f.is_empty());
        if !has_license {
            violations.push(format!(
                "license: '{name}' has no `license` or `license-file` — \
                 crates.io will reject it at upload"
            ));
        }
    }

    violations
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pkg(name: &str, publish_false: bool, license: Option<&str>) -> NoDepsPackage {
        NoDepsPackage {
            name: name.to_string(),
            id: format!("{name} 0.1.0 (path+file:///fake)"),
            publish: if publish_false { Some(vec![]) } else { None },
            license: license.map(str::to_string),
            license_file: None,
        }
    }

    fn make_meta(pkgs: Vec<NoDepsPackage>) -> NoDepsMetadata {
        let workspace_members = pkgs.iter().map(|p| p.id.clone()).collect();
        NoDepsMetadata { packages: pkgs, workspace_members }
    }

    #[test]
    fn clean_metadata_no_violations() {
        let meta = make_meta(vec![make_pkg("perl-token", false, Some("MIT OR Apache-2.0"))]);
        let allowlist = vec!["perl-token".to_string()];
        assert!(check_metadata(&meta, &allowlist).is_empty());
    }

    #[test]
    fn drift_a_allowlist_has_publish_false_crate() {
        let meta = make_meta(vec![
            make_pkg("perl-token", true, Some("MIT OR Apache-2.0")), // publish = false
        ]);
        let allowlist = vec!["perl-token".to_string()];
        let v = check_metadata(&meta, &allowlist);
        assert!(v.iter().any(|s| s.contains("drift:")), "expected drift violation, got: {v:?}");
    }

    #[test]
    fn drift_b_publishable_crate_absent_from_allowlist() {
        let meta = make_meta(vec![make_pkg("perl-token", false, Some("MIT OR Apache-2.0"))]);
        let allowlist: Vec<String> = vec![]; // perl-token missing from allowlist
        let v = check_metadata(&meta, &allowlist);
        assert!(v.iter().any(|s| s.contains("drift:")), "expected drift violation, got: {v:?}");
    }

    #[test]
    fn missing_license_detected() {
        let meta = make_meta(vec![make_pkg("perl-token", false, None)]);
        let allowlist = vec!["perl-token".to_string()];
        let v = check_metadata(&meta, &allowlist);
        assert!(v.iter().any(|s| s.contains("license:")), "expected license violation, got: {v:?}");
    }
}
