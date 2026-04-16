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

pub fn run() -> Result<()> {
    println!("Checking crate layer constraints...");

    // Collect cargo metadata to inspect dependencies
    let output =
        Command::new("cargo").args(["metadata", "--no-deps", "--format-version=1"]).output()?;

    if !output.status.success() {
        bail!("cargo metadata failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;

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
        println!("Layer check passed: all {} rule(s) satisfied.", LAYER_RULES.len());
        Ok(())
    } else {
        for v in &violations {
            eprintln!("{v}");
        }
        bail!("Layer check failed: {} violation(s) found.", violations.len());
    }
}
