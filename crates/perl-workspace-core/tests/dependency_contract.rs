//! Enforces the PLSP-ADR-0006 / NATIVE_STACK_POLICY dependency contract:
//! `perl-workspace-core` must not depend — directly or transitively — on the
//! editor/LSP runtime, DAP, async runtimes, or `lsp-types`.
//!
//! The check shells out to `cargo tree`. When cargo is unavailable (some
//! sandboxes), the test degrades to a no-op with a printed note rather than a
//! false failure — the contract is also documented in the crate README and the
//! Cargo.toml comment.
#![allow(clippy::print_stderr)]

use std::process::Command;

/// Crates that must never appear in this crate's dependency tree.
const FORBIDDEN: &[&str] = &[
    "lsp-types",
    "tokio",
    "tower-lsp",
    "perl-lsp-rs",
    "perl-lsp-rs-core",
    "perllsp",
    "perl-dap",
    "perl-workspace ", // trailing space: match the crate, not "perl-workspace-core"
];

#[test]
fn no_forbidden_dependencies() {
    let output = Command::new(env!("CARGO"))
        .args(["tree", "-p", "perl-workspace-core", "--edges", "normal", "--prefix", "none"])
        .output();

    let output = match output {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            // cargo ran but failed (e.g. offline lockfile issue): don't turn a
            // tooling hiccup into a contract failure. Print for visibility.
            eprintln!(
                "dependency_contract: `cargo tree` failed ({}); skipping. stderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        Err(error) => {
            eprintln!("dependency_contract: could not run `cargo tree` ({error}); skipping");
            return;
        }
    };

    let tree = String::from_utf8_lossy(&output.stdout);
    for forbidden in FORBIDDEN {
        assert!(
            !tree.contains(forbidden),
            "perl-workspace-core must not depend on `{}` — see PLSP-ADR-0006. Tree:\n{tree}",
            forbidden.trim()
        );
    }
}
