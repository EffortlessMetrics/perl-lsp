//! Targeted checks for changed crates
//!
//! Detects which crates have changed since a base git ref and runs
//! clippy and/or tests only for those crates. This gives fast feedback
//! during active development without running the full CI suite.

use std::collections::BTreeSet;
use std::path::Path;

use color_eyre::eyre::{Context, Result, eyre};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};

use crate::utils::project_root;

/// Check mode: which checks to run on changed crates.
#[derive(Clone, Debug, clap::ValueEnum)]
pub enum CheckMode {
    /// Run only clippy
    Clippy,
    /// Run only tests
    Test,
    /// Run both clippy and tests
    All,
}

/// Resolve a git ref, falling back to HEAD~1 if the primary ref is not valid.
fn resolve_base_ref(base: &str) -> Result<String> {
    let verify = cmd("git", &["rev-parse", "--verify", base])
        .stdout_null()
        .stderr_null()
        .unchecked()
        .run()
        .context("Failed to run git rev-parse")?;

    if verify.status.success() {
        return Ok(base.to_string());
    }

    eprintln!(
        "Warning: Base ref '{}' not found; falling back to HEAD~1",
        base
    );

    let fallback = "HEAD~1";
    let verify_fallback = cmd("git", &["rev-parse", "--verify", fallback])
        .stdout_null()
        .stderr_null()
        .unchecked()
        .run()
        .context("Failed to run git rev-parse for fallback")?;

    if verify_fallback.status.success() {
        return Ok(fallback.to_string());
    }

    Err(eyre!(
        "Could not resolve a valid base ref (tried '{}' and '{}')",
        base,
        fallback,
    ))
}

/// Get the list of files changed between base_ref and HEAD.
fn changed_files(base_ref: &str) -> Result<Vec<String>> {
    let diff_spec = format!("{}...HEAD", base_ref);
    let output = cmd("git", &["diff", "--name-only", &diff_spec])
        .stdout_capture()
        .stderr_capture()
        .unchecked()
        .run()
        .context("Failed to run git diff")?;

    if !output.status.success() {
        // Fall back to two-dot diff (works when there's no merge base)
        let diff_spec_two = format!("{}..HEAD", base_ref);
        let output2 = cmd("git", &["diff", "--name-only", &diff_spec_two])
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

/// Extract unique crate directory prefixes (e.g., "crates/perl-parser") from changed files.
fn extract_crate_dirs(files: &[String]) -> BTreeSet<String> {
    let mut dirs = BTreeSet::new();
    for file in files {
        // Match files under crates/<name>/...
        let parts: Vec<&str> = file.splitn(3, '/').collect();
        if parts.len() >= 2 && parts[0] == "crates" && !parts[1].is_empty() {
            dirs.insert(format!("crates/{}", parts[1]));
        }
    }
    dirs
}

/// Map crate directories to package names by reading each Cargo.toml.
///
/// Uses `cargo metadata` to get authoritative package names and manifest paths,
/// then matches them against the changed crate directories.
fn resolve_package_names(
    project_root: &Path,
    crate_dirs: &BTreeSet<String>,
) -> Result<BTreeSet<String>> {
    let output = cmd("cargo", &["metadata", "--no-deps", "--format-version", "1"])
        .dir(project_root)
        .stdout_capture()
        .stderr_capture()
        .run()
        .context("Failed to run cargo metadata")?;

    let stdout =
        String::from_utf8(output.stdout).context("cargo metadata output was not valid UTF-8")?;

    let metadata: serde_json::Value =
        serde_json::from_str(&stdout).context("Failed to parse cargo metadata JSON")?;

    let packages = metadata
        .get("packages")
        .and_then(|p| p.as_array())
        .ok_or_else(|| eyre!("cargo metadata missing 'packages' array"))?;

    let root_str = project_root.to_string_lossy();
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

        // Convert absolute manifest path to a relative crate directory
        // e.g., "/path/to/project/crates/perl-parser/Cargo.toml" -> "crates/perl-parser"
        let relative = manifest_path
            .strip_prefix(root_str.as_ref())
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

/// Run targeted checks (clippy and/or tests) for the given packages.
fn run_checks(
    project_root: &Path,
    packages: &BTreeSet<String>,
    mode: &CheckMode,
    spinner: &ProgressBar,
) -> Result<()> {
    // Build -p args for all packages at once (matches the bash script behavior)
    let mut package_args: Vec<String> = Vec::new();
    for pkg in packages {
        package_args.push("-p".to_string());
        package_args.push(pkg.clone());
    }

    let run_clippy = matches!(mode, CheckMode::Clippy | CheckMode::All);
    let run_tests = matches!(mode, CheckMode::Test | CheckMode::All);

    if run_clippy {
        spinner.println("");
        spinner.set_message("Running clippy for changed packages...");

        let mut args: Vec<&str> = vec!["clippy"];
        for a in &package_args {
            args.push(a.as_str());
        }
        args.extend_from_slice(&["--locked", "--", "-D", "warnings", "-A", "missing_docs"]);

        let result = cmd("cargo", &args)
            .dir(project_root)
            .unchecked()
            .run()
            .context("Failed to run cargo clippy")?;

        if !result.status.success() {
            return Err(eyre!("Clippy failed for changed packages"));
        }
        spinner.println("Clippy passed for changed packages");
    }

    if run_tests {
        spinner.println("");
        spinner.set_message("Running tests for changed packages...");

        let mut args: Vec<&str> = vec!["test"];
        for a in &package_args {
            args.push(a.as_str());
        }
        args.extend_from_slice(&["--lib", "--locked"]);

        let result = cmd("cargo", &args)
            .dir(project_root)
            .unchecked()
            .run()
            .context("Failed to run cargo test")?;

        if !result.status.success() {
            return Err(eyre!("Tests failed for changed packages"));
        }
        spinner.println("Tests passed for changed packages");
    }

    Ok(())
}

/// Entry point for the targeted-checks subcommand.
pub fn run(base: String, mode: CheckMode) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    let style = ProgressStyle::default_spinner()
        .template("{spinner:.green} {wide_msg}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner());
    spinner.set_style(style);

    let root = project_root()?;
    spinner.set_message("Resolving base ref...");

    let base_ref = resolve_base_ref(&base)?;

    spinner.set_message(format!("Detecting changes since {}...", base_ref));
    let files = changed_files(&base_ref)?;

    let crate_dirs = extract_crate_dirs(&files);
    if crate_dirs.is_empty() {
        spinner.finish_with_message(format!(
            "No crate changes detected since {}; skipping targeted checks",
            base_ref,
        ));
        return Ok(());
    }

    spinner.set_message("Resolving package names...");
    let packages = resolve_package_names(&root, &crate_dirs)?;

    if packages.is_empty() {
        spinner.finish_with_message(
            "Changed crate directories found, but no workspace package names could be resolved",
        );
        return Err(eyre!(
            "Changed crate directories found, but no package names could be resolved"
        ));
    }

    println!("Detected changed packages since {}:", base_ref);
    for pkg in &packages {
        println!("  - {}", pkg);
    }

    run_checks(&root, &packages, &mode, &spinner)?;

    spinner.finish_with_message("Targeted checks completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_crate_dirs_basic() {
        let files = vec![
            "crates/perl-parser/src/lib.rs".to_string(),
            "crates/perl-lsp/src/main.rs".to_string(),
            "crates/perl-parser/tests/test.rs".to_string(),
            "README.md".to_string(),
            "scripts/something.sh".to_string(),
        ];

        let dirs = extract_crate_dirs(&files);
        assert_eq!(dirs.len(), 2);
        assert!(dirs.contains("crates/perl-parser"));
        assert!(dirs.contains("crates/perl-lsp"));
    }

    #[test]
    fn test_extract_crate_dirs_empty() {
        let files = vec!["README.md".to_string(), "justfile".to_string()];

        let dirs = extract_crate_dirs(&files);
        assert!(dirs.is_empty());
    }

    #[test]
    fn test_extract_crate_dirs_nested() {
        let files = vec![
            "crates/perl-lsp-navigation/src/lib.rs".to_string(),
            "crates/perl-lsp-navigation/src/goto.rs".to_string(),
        ];

        let dirs = extract_crate_dirs(&files);
        assert_eq!(dirs.len(), 1);
        assert!(dirs.contains("crates/perl-lsp-navigation"));
    }

    #[test]
    fn test_extract_crate_dirs_ignores_non_crate_paths() {
        let files = vec![
            "docs/reference/STABILITY.md".to_string(),
            "xtask/src/main.rs".to_string(),
            ".github/workflows/ci.yml".to_string(),
            "crates/".to_string(), // bare crates dir, no sub-crate
        ];

        let dirs = extract_crate_dirs(&files);
        assert!(dirs.is_empty());
    }
}
