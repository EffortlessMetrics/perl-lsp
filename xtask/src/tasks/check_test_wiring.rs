//! Check that Rust test-bearing files are reachable from the crate/module tree.
//!
//! Guard A from issue #4102: detect `.rs` files under `crates/*/src/` or
//! `crates/*/tests/` that contain `#[test]` or `#[cfg(test)]` but are not
//! reachable from a crate root or integration-test root.
//!
//! The audit is intentionally structural:
//! - `src/lib.rs` and `src/main.rs` are crate roots.
//! - direct `tests/*.rs` files are integration-test roots.
//! - nested modules are followed via `mod foo;` and `#[path = "..."] mod foo;`.
//!
//! This is enough to catch dormant helper/test files such as a missing
//! `mod unclosed_block_recovery_tests;` declaration, while avoiding the false
//! positives that would come from treating every `tests/*.rs` file as a nested
//! module.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use regex::Regex;
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use walkdir::WalkDir;

static PATH_ATTR_RE: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::expect_used)]
    Regex::new(r#"#\s*\[\s*path\s*=\s*"([^"]+)"\s*\]"#).expect("static path attr regex compiles")
});

static MODULE_DECL_RE: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::expect_used)]
    Regex::new(r#"\b(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;"#)
        .expect("static module regex compiles")
});

/// Report for a file that contains tests but is not reachable from the module tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offender {
    /// Crate name that owns the file.
    pub crate_name: String,
    /// Workspace-relative path to the file.
    pub path: String,
    /// Short explanation of why it was flagged.
    pub reason: String,
}

/// Summary of the wiring audit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WiringAuditReport {
    /// Number of crates scanned.
    pub crates_scanned: usize,
    /// Number of Rust files scanned for test markers.
    pub test_files_scanned: usize,
    /// Files that contain tests but were not reachable from the module tree.
    pub offenders: Vec<Offender>,
}

/// Run the wiring audit against the current workspace.
pub fn run() -> Result<()> {
    let root = project_root()?;
    let report = scan(&root)?;

    print_report(&report);

    if !report.offenders.is_empty() {
        bail!(
            "{} unwired test file(s) found — run `cargo xtask check-test-wiring` to inspect",
            report.offenders.len()
        );
    }

    Ok(())
}

/// Scan the workspace and return the wiring audit report.
pub fn scan(workspace_root: &Path) -> Result<WiringAuditReport> {
    let crates_dir = workspace_root.join("crates");
    let crates = load_workspace_crates(&crates_dir);

    let mut report = WiringAuditReport::default();
    let mut offenders = Vec::new();

    for crate_dir in crates {
        report.crates_scanned += 1;
        let crate_name = parse_package_name(&crate_dir.join("Cargo.toml")).unwrap_or_else(|| {
            crate_dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("<unknown>")
                .to_string()
        });

        let crate_offenders = scan_crate(&crate_dir, workspace_root, &crate_name)
            .with_context(|| format!("scan crate {}", crate_name))?;
        report.test_files_scanned += crate_offenders.scanned_files;
        offenders.extend(crate_offenders.offenders);
    }

    offenders.sort_by(|a, b| a.path.cmp(&b.path));
    report.offenders = offenders;
    Ok(report)
}

struct CrateScan {
    scanned_files: usize,
    offenders: Vec<Offender>,
}

fn scan_crate(crate_dir: &Path, workspace_root: &Path, crate_name: &str) -> Result<CrateScan> {
    let src_dir = crate_dir.join("src");
    let tests_dir = crate_dir.join("tests");

    let mut roots = Vec::new();
    roots.extend(crate_root_files(&src_dir));
    roots.extend(integration_test_roots(&tests_dir));

    let reachable = reachable_files(&roots)?;
    let candidate_files = test_candidate_files(&src_dir)
        .into_iter()
        .chain(test_candidate_files(&tests_dir))
        .collect::<Vec<_>>();

    let mut offenders = Vec::new();
    let mut scanned_files = 0usize;

    for file in candidate_files {
        scanned_files += 1;
        let canonical = match fs::canonicalize(&file) {
            Ok(path) => path,
            Err(_) => continue,
        };
        let content = match fs::read_to_string(&canonical) {
            Ok(content) => content,
            Err(_) => continue,
        };
        if !contains_test_markers(&content) {
            continue;
        }
        if reachable.contains(&canonical) {
            continue;
        }

        let rel = canonical
            .strip_prefix(workspace_root)
            .unwrap_or(&canonical)
            .display()
            .to_string();
        offenders.push(Offender {
            crate_name: crate_name.to_string(),
            path: rel,
            reason: missing_module_reason(&canonical),
        });
    }

    Ok(CrateScan {
        scanned_files,
        offenders,
    })
}

fn print_report(report: &WiringAuditReport) {
    println!("[INFO] Test wiring audit");
    println!("[INFO] Crates scanned: {}", report.crates_scanned);
    println!("[INFO] Rust files scanned: {}", report.test_files_scanned);
    println!();

    if report.offenders.is_empty() {
        println!("[OK] No unwired test files found.");
        return;
    }

    println!(
        "[WARN] {} unwired test file(s) found:",
        report.offenders.len()
    );
    for offender in &report.offenders {
        println!(
            "  [{}] {} — {}",
            offender.crate_name, offender.path, offender.reason
        );
    }
    println!();
}

fn load_workspace_crates(crates_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(crates_dir) else {
        return Vec::new();
    };

    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| path.join("Cargo.toml").exists())
        .collect()
}

fn parse_package_name(cargo_toml: &Path) -> Option<String> {
    let content = fs::read_to_string(cargo_toml).ok()?;
    let parsed = content.parse::<toml::Table>().ok()?;
    parsed
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
}

fn crate_root_files(src_dir: &Path) -> Vec<PathBuf> {
    let mut roots = [src_dir.join("lib.rs"), src_dir.join("main.rs")]
        .into_iter()
        .filter(|path| path.exists())
        .collect::<Vec<_>>();

    let bin_dir = src_dir.join("bin");
    if let Ok(entries) = fs::read_dir(&bin_dir) {
        roots.extend(
            entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("rs")
                }),
        );
    }

    roots
}

fn integration_test_roots(tests_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(tests_dir) else {
        return Vec::new();
    };

    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("rs")
        })
        .collect()
}

fn test_candidate_files(dir: &Path) -> Vec<PathBuf> {
    if fs::read_dir(dir).is_err() {
        return Vec::new();
    }

    WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rs"))
        .filter(|path| !should_ignore_candidate(path))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn should_ignore_candidate(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    path.components()
        .any(|component| component.as_os_str() == "fixtures")
        || path_str.contains("/src/gen/")
}

fn reachable_files(roots: &[PathBuf]) -> Result<HashSet<PathBuf>> {
    let mut reachable = HashSet::new();
    let mut stack = roots.to_vec();

    while let Some(file) = stack.pop() {
        let canonical = match fs::canonicalize(&file) {
            Ok(path) => path,
            Err(_) => continue,
        };
        if !reachable.insert(canonical.clone()) {
            continue;
        }

        let children = module_children(&canonical)
            .with_context(|| format!("parse module declarations in {}", canonical.display()))?;
        stack.extend(children);
    }

    Ok(reachable)
}

fn module_children(file: &Path) -> Result<Vec<PathBuf>> {
    let content = fs::read_to_string(file)
        .with_context(|| format!("read module source {}", file.display()))?;
    let parent_dir = file.parent().unwrap_or_else(|| Path::new("."));
    let mut pending_path: Option<PathBuf> = None;
    let mut children = Vec::new();

    for raw_line in content.lines() {
        let line = strip_line_comment(raw_line);
        if line.is_empty() {
            continue;
        }

        if let Some(path) = extract_path_attr(line) {
            pending_path = Some(path);
        }

        if let Some(module_name) = extract_module_name(line) {
            let resolved = if let Some(path) = pending_path.take() {
                parent_dir.join(path)
            } else {
                resolve_module_path(parent_dir, &module_name)
            };

            if let Ok(canonical) = fs::canonicalize(&resolved) {
                children.push(canonical);
            }
        }
    }

    Ok(children)
}

fn resolve_module_path(parent_dir: &Path, module_name: &str) -> PathBuf {
    let flat = parent_dir.join(format!("{module_name}.rs"));
    if flat.exists() {
        return flat;
    }

    parent_dir.join(module_name).join("mod.rs")
}

fn strip_line_comment(line: &str) -> &str {
    line.split_once("//")
        .map(|(prefix, _)| prefix)
        .unwrap_or(line)
        .trim()
}

fn extract_path_attr(line: &str) -> Option<PathBuf> {
    PATH_ATTR_RE
        .captures(line)
        .and_then(|caps| caps.get(1))
        .map(|m| PathBuf::from(m.as_str()))
}

fn extract_module_name(line: &str) -> Option<String> {
    MODULE_DECL_RE
        .captures(line)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

fn contains_test_markers(content: &str) -> bool {
    content.contains("#[test]") || content.contains("#[cfg(test)]")
}

fn missing_module_reason(file: &Path) -> String {
    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("<module>");
    if stem == "mod" {
        "contains #[test] or #[cfg(test)] but is not reachable from its crate/module tree; add the missing module declaration in the parent module".to_string()
    } else {
        format!(
            "contains #[test] or #[cfg(test)] but is not reachable from its crate/module tree; add `mod {stem};` in the parent module"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn regex_helpers_initialize() {
        assert!(extract_path_attr(r#"#[path = "child.rs"]"#).is_some());
        assert_eq!(extract_module_name("mod child;").as_deref(), Some("child"));
    }

    fn write(root: &Path, rel: &str, content: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn make_workspace() -> TempDir {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write(
            root,
            "Cargo.toml",
            r#"[workspace]
members = ["crates/*"]
resolver = "2"
"#,
        );
        dir
    }

    #[test]
    fn flags_unwired_src_test_module() {
        let dir = make_workspace();
        let root = dir.path();

        write(
            root,
            "crates/perl-audit/Cargo.toml",
            r#"[package]
name = "perl-audit"
version = "0.1.0"
edition = "2021"
"#,
        );
        write(
            root,
            "crates/perl-audit/src/lib.rs",
            r#"
mod wired;
"#,
        );
        write(
            root,
            "crates/perl-audit/src/wired.rs",
            r#"
#[cfg(test)]
mod tests {
    #[test]
    fn wired() {}
}
"#,
        );
        write(
            root,
            "crates/perl-audit/src/orphan.rs",
            r#"
#[cfg(test)]
mod tests {
    #[test]
    fn orphan() {}
}
"#,
        );

        let report = scan(root).unwrap();
        assert_eq!(report.offenders.len(), 1);
        assert!(
            report.offenders[0]
                .path
                .ends_with("crates/perl-audit/src/orphan.rs")
        );
    }

    #[test]
    fn honors_integration_test_roots_and_path_attrs() {
        let dir = make_workspace();
        let root = dir.path();

        write(
            root,
            "crates/perl-audit/Cargo.toml",
            r#"[package]
name = "perl-audit"
version = "0.1.0"
edition = "2021"
"#,
        );
        write(
            root,
            "crates/perl-audit/tests/root.rs",
            r#"
#[path = "support/mod.rs"]
mod support;
"#,
        );
        write(
            root,
            "crates/perl-audit/tests/support/mod.rs",
            r#"
#[cfg(test)]
mod tests {
    #[test]
    fn helper() {}
}
"#,
        );
        write(
            root,
            "crates/perl-audit/tests/direct.rs",
            r#"
#[test]
fn direct_root() {}
"#,
        );

        let report = scan(root).unwrap();
        assert!(report.offenders.is_empty(), "{:#?}", report.offenders);
    }

    #[test]
    fn ignores_fixture_and_generated_candidates() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        write(
            root,
            "src/real.rs",
            r#"
#[test]
fn real_test() {}
"#,
        );
        write(
            root,
            "src/fixtures/fixture_case.rs",
            r#"
#[test]
fn fixture_test() {}
"#,
        );
        write(
            root,
            "src/gen/generated_case.rs",
            r#"
#[test]
fn generated_test() {}
"#,
        );

        let candidates = test_candidate_files(&root.join("src"));

        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].ends_with("src/real.rs"));
    }
}
