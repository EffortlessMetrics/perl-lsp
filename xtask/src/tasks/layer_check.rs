//! Layer-check: enforce crate dependency direction constraints.
//!
//! Leaf crates (diagnostic codes, pure analysis) must not depend on
//! higher-level crates (LSP providers, wire format). This prevents
//! circular dependency introduction and keeps the crate dependency graph clean.
//!
//! # Current rules
//!
//! - `perl-diagnostics` must NOT depend on any `perl-lsp-*` crate.
//!   Reason: perl-diagnostics is a stable kernel of diagnostic codes and types.
//!   LSP-specific formatting belongs in the LSP provider layer, not here.
//!
//! # Dev-dep filter
//!
//! Only **normal** (runtime) dependencies are checked. Dev-dependencies (`kind == "dev"`)
//! and build-dependencies (`kind == "build"`) are permitted to cross layers freely,
//! so tests and build scripts may pull in higher-layer crates for fixtures and
//! integration testing without triggering a violation.

use color_eyre::eyre::{Result, bail};
use std::process::Command;

/// A layer constraint: `crate_name` must not depend on any crate matching `forbidden_prefix`.
struct LayerRule {
    /// The crate being constrained.
    crate_name: &'static str,
    /// Crate name prefix that is forbidden as a direct dependency.
    forbidden_prefix: &'static str,
    /// Human-readable explanation for the constraint.
    rationale: &'static str,
}

/// The complete set of layer constraints enforced by this check.
const LAYER_RULES: &[LayerRule] = &[LayerRule {
    crate_name: "perl-diagnostics",
    forbidden_prefix: "perl-lsp-",
    rationale: "perl-diagnostics is a stable leaf crate (diagnostic codes/types/catalog). \
                    It must not depend on LSP wire types or provider crates. \
                    LSP-specific logic belongs in the perl-lsp-* layer above it.",
}];

/// Entry point used by `cargo xtask layer-check`.
pub fn run() -> Result<()> {
    run_with_metadata(None)
}

/// Run the layer check, optionally against synthetic metadata (for tests).
///
/// When `metadata` is `None`, shells out to `cargo metadata --no-deps` to
/// introspect the real workspace. When `Some(value)` is provided, that value
/// is used directly in place of the cargo invocation — this is how unit tests
/// exercise the rule engine without needing to mutate real `Cargo.toml` files.
pub fn run_with_metadata(metadata: Option<serde_json::Value>) -> Result<()> {
    println!("Checking crate layer constraints...");

    let metadata: serde_json::Value = match metadata {
        Some(m) => m,
        None => {
            let output = Command::new("cargo")
                .args(["metadata", "--no-deps", "--format-version=1"])
                .output()?;

            if !output.status.success() {
                bail!(
                    "cargo metadata failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }

            serde_json::from_slice(&output.stdout)?
        }
    };

    let mut violations = Vec::new();

    for rule in LAYER_RULES {
        // Find the package for this crate in metadata
        let packages = metadata["packages"]
            .as_array()
            .ok_or_else(|| color_eyre::eyre::eyre!("cargo metadata: expected packages array"))?;

        for pkg in packages {
            let name = pkg["name"].as_str().unwrap_or("");
            if name != rule.crate_name {
                continue;
            }

            // Check each dependency for forbidden prefix
            if let Some(deps) = pkg["dependencies"].as_array() {
                for dep in deps {
                    // Only enforce on normal (runtime) deps.
                    // cargo_metadata records `kind` as null for normal deps,
                    // "dev" for dev-dependencies, and "build" for build-dependencies.
                    let kind = dep["kind"].as_str();
                    if kind.is_some() {
                        continue;
                    }

                    let dep_name = dep["name"].as_str().unwrap_or("");
                    if dep_name.starts_with(rule.forbidden_prefix) {
                        violations.push(format!(
                            "VIOLATION: `{crate}` depends on `{dep}` (prefix `{prefix}` is forbidden)\n  \
                             Rationale: {rationale}",
                            crate = rule.crate_name,
                            dep = dep_name,
                            prefix = rule.forbidden_prefix,
                            rationale = rule.rationale,
                        ));
                    }
                }
            }
        }
    }

    if violations.is_empty() {
        println!(
            "Layer check passed: all {} rule(s) satisfied.",
            LAYER_RULES.len()
        );
        Ok(())
    } else {
        for v in &violations {
            eprintln!("{v}");
        }
        bail!(
            "Layer check failed: {} violation(s) found.",
            violations.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Sanity check: running against the real workspace must succeed.
    ///
    /// This guards against regressions — if someone reintroduces a
    /// forbidden dep inside `perl-diagnostics`, this test will fail
    /// alongside the CI gate.
    #[test]
    fn run_passes_on_clean_workspace() {
        let result = run_with_metadata(None);
        assert!(
            result.is_ok(),
            "layer-check should pass on the real workspace; got: {result:?}"
        );
    }

    /// Synthetic metadata: `perl-diagnostics` has a dev-dependency on a
    /// `perl-lsp-*` crate. The dev-dep filter must skip it and the check
    /// must pass.
    #[test]
    fn kind_filter_skips_dev_deps() {
        let metadata = json!({
            "packages": [
                {
                    "name": "perl-diagnostics",
                    "dependencies": [
                        { "name": "perl-lsp-core", "kind": "dev" },
                        { "name": "perl-lsp-types", "kind": "dev" }
                    ]
                }
            ]
        });

        let result = run_with_metadata(Some(metadata));
        assert!(
            result.is_ok(),
            "dev-deps crossing layer boundaries should be allowed; got: {result:?}"
        );
    }

    /// Synthetic metadata: `perl-diagnostics` has a build-dependency on a
    /// `perl-lsp-*` crate. Build-deps (used by build scripts) may cross layers
    /// freely, so the check must pass.
    #[test]
    fn kind_filter_skips_build_deps() {
        let metadata = json!({
            "packages": [
                {
                    "name": "perl-diagnostics",
                    "dependencies": [
                        { "name": "perl-lsp-core", "kind": "build" }
                    ]
                }
            ]
        });

        let result = run_with_metadata(Some(metadata));
        assert!(
            result.is_ok(),
            "build-deps crossing layer boundaries should be allowed; got: {result:?}"
        );
    }

    /// Synthetic metadata: `perl-diagnostics` has a **normal** dependency on
    /// a `perl-lsp-*` crate. The check must fail.
    #[test]
    fn kind_filter_enforces_normal_deps() {
        let metadata = json!({
            "packages": [
                {
                    "name": "perl-diagnostics",
                    "dependencies": [
                        // kind: null => normal runtime dep
                        { "name": "perl-lsp-core", "kind": null }
                    ]
                }
            ]
        });

        let result = run_with_metadata(Some(metadata));
        assert!(
            result.is_err(),
            "normal deps crossing layer boundaries must be rejected; got: {result:?}"
        );
    }
}
