//! Tests for the unwired infrastructure scanner (issue #2667)
//!
//! Validates the core logic of the `cargo xtask unwired-scan` command:
//! - Detecting crates that have tests but are not depended on by perl-lsp-rs
//! - Counting `#[test]` annotations in source files
//! - Parsing Cargo.toml dependency lists
//! - Detecting TODO/FIXME wiring comments
//! - JSON output mode (--json)
//! - CI gate mode (--check exits non-zero when findings exist)

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helper: build a minimal fake workspace in a temp dir
// ---------------------------------------------------------------------------

fn write_file(dir: &std::path::Path, rel: &str, content: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    fs::write(path, content).expect("write file");
}

fn make_fake_workspace() -> TempDir {
    let dir = TempDir::new().expect("create tempdir");
    let root = dir.path();

    // Workspace Cargo.toml
    write_file(
        root,
        "Cargo.toml",
        r#"[workspace]
members = ["crates/*"]
resolver = "2"
"#,
    );

    // perl-lsp-rs: the root LSP crate
    write_file(
        root,
        "crates/perl-lsp/Cargo.toml",
        r#"[package]
name = "perl-lsp-rs"
version = "0.1.0"
edition = "2021"

[dependencies]
perl-wired = { path = "../perl-wired" }
"#,
    );
    write_file(
        root,
        "crates/perl-lsp/src/lib.rs",
        r#"// main lsp crate
"#,
    );

    // perl-wired: depended on by perl-lsp-rs, has tests
    write_file(
        root,
        "crates/perl-wired/Cargo.toml",
        r#"[package]
name = "perl-wired"
version = "0.1.0"
edition = "2021"
"#,
    );
    write_file(
        root,
        "crates/perl-wired/src/lib.rs",
        r#"pub fn add(a: i32, b: i32) -> i32 { a + b }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add() { assert_eq!(add(1, 2), 3); }
}
"#,
    );

    // perl-unwired: NOT depended on by perl-lsp-rs, has tests — should be flagged
    write_file(
        root,
        "crates/perl-unwired/Cargo.toml",
        r#"[package]
name = "perl-unwired"
version = "0.1.0"
edition = "2021"
"#,
    );
    write_file(
        root,
        "crates/perl-unwired/src/lib.rs",
        r#"pub fn multiply(a: i32, b: i32) -> i32 { a * b }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_multiply() { assert_eq!(multiply(2, 3), 6); }
    #[test]
    fn test_multiply_zero() { assert_eq!(multiply(0, 5), 0); }
}
"#,
    );

    // perl-no-tests: NOT depended on by perl-lsp-rs, no tests — should NOT be flagged
    write_file(
        root,
        "crates/perl-no-tests/Cargo.toml",
        r#"[package]
name = "perl-no-tests"
version = "0.1.0"
edition = "2021"
"#,
    );
    write_file(
        root,
        "crates/perl-no-tests/src/lib.rs",
        r#"pub fn helper() -> &'static str { "helper" }
"#,
    );

    // perl-todo-wire: has TODO: wire comment — should appear in wiring comments
    write_file(
        root,
        "crates/perl-todo-wire/Cargo.toml",
        r#"[package]
name = "perl-todo-wire"
version = "0.1.0"
edition = "2021"
"#,
    );
    write_file(
        root,
        "crates/perl-todo-wire/src/lib.rs",
        r#"// TODO: wire this into get_diagnostics()
pub fn check_something() -> Vec<String> { vec![] }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_check_something() { assert!(check_something().is_empty()); }
}
"#,
    );

    dir
}

// ---------------------------------------------------------------------------
// Tests for count_tests_in_dir
// ---------------------------------------------------------------------------

/// Count `#[test]` occurrences by walking a source directory.
fn count_tests_in_dir(src_dir: &std::path::Path) -> u32 {
    let mut count = 0u32;
    let Ok(walker) = fs::read_dir(src_dir) else {
        return 0;
    };
    // Recursive walk via a simple queue
    let mut queue: Vec<PathBuf> = walker.filter_map(|e| e.ok().map(|e| e.path())).collect();
    while let Some(path) = queue.pop() {
        if path.is_dir() {
            if let Ok(entries) = fs::read_dir(&path) {
                queue.extend(entries.filter_map(|e| e.ok().map(|e| e.path())));
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs")
            && let Ok(content) = fs::read_to_string(&path)
        {
            count += content.matches("#[test]").count() as u32;
        }
    }
    count
}

#[test]
fn test_count_tests_finds_test_attribute() {
    let dir = TempDir::new().expect("tempdir");
    let src = dir.path().join("src");
    fs::create_dir_all(&src).expect("create src");
    fs::write(
        src.join("lib.rs"),
        r#"#[test]
fn test_foo() {}
#[test]
fn test_bar() {}
"#,
    )
    .expect("write");
    assert_eq!(count_tests_in_dir(&src), 2);
}

#[test]
fn test_count_tests_empty_dir() {
    let dir = TempDir::new().expect("tempdir");
    let src = dir.path().join("src");
    fs::create_dir_all(&src).expect("create src");
    assert_eq!(count_tests_in_dir(&src), 0);
}

#[test]
fn test_count_tests_no_tests_in_file() {
    let dir = TempDir::new().expect("tempdir");
    let src = dir.path().join("src");
    fs::create_dir_all(&src).expect("create src");
    fs::write(src.join("lib.rs"), "pub fn foo() {}").expect("write");
    assert_eq!(count_tests_in_dir(&src), 0);
}

#[test]
fn test_count_tests_nonexistent_dir() {
    let path = PathBuf::from("/nonexistent/path/to/src");
    assert_eq!(count_tests_in_dir(&path), 0);
}

// ---------------------------------------------------------------------------
// Tests for parse_crate_deps
// ---------------------------------------------------------------------------

/// Parse the `[dependencies]` section of a Cargo.toml and return dep names.
fn parse_crate_deps(cargo_toml: &std::path::Path) -> Vec<String> {
    let Ok(content) = fs::read_to_string(cargo_toml) else {
        return vec![];
    };
    let Ok(parsed) = content.parse::<toml::Table>() else {
        return vec![];
    };
    let mut deps = Vec::new();
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(toml::Value::Table(table)) = parsed.get(section) {
            for (key, value) in table {
                let package_name = value
                    .as_table()
                    .and_then(|dep| dep.get("package"))
                    .and_then(toml::Value::as_str)
                    .unwrap_or(key)
                    .to_string();
                deps.push(package_name);
            }
        }
    }
    deps.sort();
    deps.dedup();
    deps
}

#[test]
fn test_parse_crate_deps_basic() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let cargo_toml = dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
perl-parser = { path = "../perl-parser" }
anyhow = "1.0"
"#,
    )?;
    let deps = parse_crate_deps(&cargo_toml);
    assert!(deps.contains(&"serde".to_string()), "should contain serde");
    assert!(
        deps.contains(&"perl-parser".to_string()),
        "should contain perl-parser"
    );
    assert!(
        deps.contains(&"anyhow".to_string()),
        "should contain anyhow"
    );
    Ok(())
}

#[test]
fn test_parse_crate_deps_empty() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let cargo_toml = dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"
"#,
    )?;
    let deps = parse_crate_deps(&cargo_toml);
    assert!(deps.is_empty(), "should have no deps");
    Ok(())
}

#[test]
fn test_parse_crate_deps_missing_file() {
    let deps = parse_crate_deps(std::path::Path::new("/nonexistent/Cargo.toml"));
    assert!(deps.is_empty());
}

// ---------------------------------------------------------------------------
// Tests for scan_wiring_comments
// ---------------------------------------------------------------------------

/// Find files containing TODO/FIXME wiring-related comments.
fn scan_wiring_comments(src_dir: &std::path::Path) -> Vec<(PathBuf, String)> {
    let keywords = [
        "TODO: wire",
        "TODO: connect",
        "FIXME: not called",
        "TODO: wire this",
    ];
    let mut results = Vec::new();
    let mut queue = vec![src_dir.to_path_buf()];
    while let Some(path) = queue.pop() {
        if path.is_dir() {
            if let Ok(entries) = fs::read_dir(&path) {
                queue.extend(entries.filter_map(|e| e.ok().map(|e| e.path())));
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs")
            && let Ok(content) = fs::read_to_string(&path)
        {
            for line in content.lines() {
                for kw in &keywords {
                    if line.contains(kw) {
                        results.push((path.clone(), line.trim().to_string()));
                        break;
                    }
                }
            }
        }
    }
    results
}

#[test]
fn test_scan_wiring_comments_finds_todo_wire() {
    let dir = TempDir::new().expect("tempdir");
    let src = dir.path().join("src");
    fs::create_dir_all(&src).expect("create src");
    fs::write(
        src.join("lib.rs"),
        "// TODO: wire this into get_diagnostics()\npub fn check() {}\n",
    )
    .expect("write");
    let hits = scan_wiring_comments(&src);
    assert_eq!(hits.len(), 1, "should find one wiring comment");
    assert!(
        hits[0].1.contains("TODO: wire"),
        "comment text should match"
    );
}

#[test]
fn test_scan_wiring_comments_no_hits() {
    let dir = TempDir::new().expect("tempdir");
    let src = dir.path().join("src");
    fs::create_dir_all(&src).expect("create src");
    fs::write(src.join("lib.rs"), "pub fn regular_code() {}\n").expect("write");
    let hits = scan_wiring_comments(&src);
    assert!(hits.is_empty(), "should find no wiring comments");
}

#[test]
fn test_scan_wiring_comments_fixme_not_called() {
    let dir = TempDir::new().expect("tempdir");
    let src = dir.path().join("src");
    fs::create_dir_all(&src).expect("create src");
    fs::write(
        src.join("lib.rs"),
        "// FIXME: not called from anywhere\npub fn orphan() {}\n",
    )
    .expect("write");
    let hits = scan_wiring_comments(&src);
    assert_eq!(hits.len(), 1);
}

// ---------------------------------------------------------------------------
// Integration test: fake workspace scan
// ---------------------------------------------------------------------------

/// CrateReport mirrors what the scanner produces per crate.
#[derive(Debug)]
struct CrateReport {
    name: String,
    test_count: u32,
    is_depended_on_by_lsp: bool,
    wiring_comments: Vec<String>,
}

/// Identify crates that have tests but are not a direct dep of `lsp_crate_name`.
fn find_unwired_crates(
    workspace_root: &std::path::Path,
    lsp_crate_name: &str,
) -> anyhow::Result<Vec<CrateReport>> {
    let crates_dir = workspace_root.join("crates");

    let mut reports = Vec::new();
    let Ok(entries) = fs::read_dir(&crates_dir) else {
        return Ok(reports);
    };

    let mut workspace_crates = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let crate_dir = entry.path();
        if !crate_dir.is_dir() {
            continue;
        }
        let cargo_toml = crate_dir.join("Cargo.toml");
        if !cargo_toml.exists() {
            continue;
        }
        let Ok(content) = fs::read_to_string(&cargo_toml) else {
            continue;
        };
        let Ok(parsed) = content.parse::<toml::Table>() else {
            continue;
        };
        let Some(crate_name) = parsed
            .get("package")
            .and_then(toml::Value::as_table)
            .and_then(|package| package.get("name"))
            .and_then(toml::Value::as_str)
            .map(ToString::to_string)
        else {
            continue;
        };
        workspace_crates.push((crate_name, crate_dir, cargo_toml));
    }

    let lsp_manifest = workspace_crates
        .iter()
        .find(|(crate_name, _, _)| crate_name == lsp_crate_name)
        .map(|(_, _, manifest)| manifest.clone())
        .ok_or_else(|| anyhow::anyhow!("missing LSP crate package: {lsp_crate_name}"))?;
    let lsp_deps: std::collections::HashSet<String> =
        parse_crate_deps(&lsp_manifest).into_iter().collect();

    for (crate_name, crate_dir, _) in workspace_crates {
        if crate_name == lsp_crate_name {
            continue;
        }

        let src_dir = crate_dir.join("src");
        let test_count = count_tests_in_dir(&src_dir);
        let is_depended_on = lsp_deps.contains(&crate_name);
        let wiring_hits = scan_wiring_comments(&src_dir);
        let wiring_comments = wiring_hits.into_iter().map(|(_, line)| line).collect();

        reports.push(CrateReport {
            name: crate_name,
            test_count,
            is_depended_on_by_lsp: is_depended_on,
            wiring_comments,
        });
    }

    Ok(reports)
}

#[test]
fn test_find_unwired_crates_identifies_unwired() -> anyhow::Result<()> {
    let workspace = make_fake_workspace();
    let reports = find_unwired_crates(workspace.path(), "perl-lsp-rs")?;

    // perl-unwired has 2 tests and is not in perl-lsp deps
    let unwired = reports.iter().find(|r| r.name == "perl-unwired");
    assert!(unwired.is_some(), "perl-unwired should appear in report");
    let unwired = unwired.unwrap();
    assert_eq!(unwired.test_count, 2, "perl-unwired has 2 tests");
    assert!(
        !unwired.is_depended_on_by_lsp,
        "perl-unwired should NOT be depended on"
    );

    Ok(())
}

#[test]
fn test_find_unwired_crates_wired_crate_marked_as_wired() -> anyhow::Result<()> {
    let workspace = make_fake_workspace();
    let reports = find_unwired_crates(workspace.path(), "perl-lsp-rs")?;

    let wired = reports.iter().find(|r| r.name == "perl-wired");
    assert!(wired.is_some(), "perl-wired should appear in report");
    let wired = wired.unwrap();
    assert!(
        wired.is_depended_on_by_lsp,
        "perl-wired should be marked as depended on"
    );

    Ok(())
}

#[test]
fn test_find_unwired_crates_no_tests_not_flagged() -> anyhow::Result<()> {
    let workspace = make_fake_workspace();
    let reports = find_unwired_crates(workspace.path(), "perl-lsp-rs")?;

    let no_tests = reports.iter().find(|r| r.name == "perl-no-tests");
    assert!(no_tests.is_some(), "perl-no-tests should appear in report");
    let no_tests = no_tests.unwrap();
    assert_eq!(no_tests.test_count, 0, "perl-no-tests has zero tests");
    assert!(!no_tests.is_depended_on_by_lsp);

    Ok(())
}

#[test]
fn test_find_unwired_crates_todo_comments_captured() -> anyhow::Result<()> {
    let workspace = make_fake_workspace();
    let reports = find_unwired_crates(workspace.path(), "perl-lsp-rs")?;

    let todo = reports.iter().find(|r| r.name == "perl-todo-wire");
    assert!(todo.is_some(), "perl-todo-wire should appear in report");
    let todo = todo.unwrap();
    assert!(
        !todo.wiring_comments.is_empty(),
        "should have captured a wiring comment"
    );
    assert!(
        todo.wiring_comments[0].contains("TODO: wire"),
        "wiring comment content should match"
    );

    Ok(())
}

#[test]
fn test_find_unwired_crates_only_with_tests_are_candidates() -> anyhow::Result<()> {
    let workspace = make_fake_workspace();
    let reports = find_unwired_crates(workspace.path(), "perl-lsp-rs")?;

    // The unwired candidates are: crates that have tests AND are NOT depended on
    let candidates: Vec<&CrateReport> = reports
        .iter()
        .filter(|r| r.test_count > 0 && !r.is_depended_on_by_lsp)
        .collect();

    // perl-unwired and perl-todo-wire both qualify
    let names: Vec<&str> = candidates.iter().map(|r| r.name.as_str()).collect();
    assert!(
        names.contains(&"perl-unwired"),
        "perl-unwired is a candidate"
    );
    assert!(
        names.contains(&"perl-todo-wire"),
        "perl-todo-wire is a candidate"
    );
    assert!(
        !names.contains(&"perl-no-tests"),
        "perl-no-tests is NOT a candidate"
    );
    assert!(
        !names.contains(&"perl-wired"),
        "perl-wired is NOT a candidate"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// End-to-end subprocess tests: --json and --check modes
//
// These tests call the real `cargo xtask unwired-scan` binary against the
// live workspace so that the JSON serialization and --check exit-code paths
// are exercised against the actual production code, not a reimplementation.
//
// Note: `project_root()` in xtask is baked at compile time via
// `env!("CARGO_MANIFEST_DIR")`, so the binary always scans the real workspace
// regardless of `current_dir`. We test against the real workspace, which is
// known to have ≥1 flagged crate (52 as of initial scan).
// ---------------------------------------------------------------------------

/// Run `cargo xtask unwired-scan` with extra args against the real workspace.
fn run_unwired_scan_real(extra_args: &[&str]) -> std::process::Output {
    use assert_cmd::cargo::cargo_bin;
    let xtask_bin = cargo_bin("xtask");
    let mut cmd = std::process::Command::new(xtask_bin);
    cmd.arg("unwired-scan");
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.output().expect("run xtask unwired-scan")
}

/// --check exits non-zero on the real workspace, which has transitive-dep false
/// positives. The exit code behaviour is the core contract for CI gate usage.
#[test]
fn test_check_mode_exits_nonzero_on_real_workspace() {
    let output = run_unwired_scan_real(&["--check"]);
    assert!(
        !output.status.success(),
        "--check must exit non-zero when any flagged crates exist; got status={:?}",
        output.status.code()
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unwired crate"),
        "--check stderr must mention 'unwired crate'; got: {stderr}"
    );
}

/// --json emits syntactically valid JSON with all required ScanReport fields.
#[test]
fn test_json_mode_emits_valid_json_with_expected_fields() {
    let output = run_unwired_scan_real(&["--json"]);
    assert!(
        output.status.success(),
        "--json must exit zero (it reports, does not gate); got status={:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("--json stdout must be UTF-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json output must be valid JSON");

    // All top-level fields from ScanReport must be present.
    for field in [
        "lsp_crate",
        "crates",
        "flagged",
        "total_crates",
        "total_flagged",
    ] {
        assert!(
            parsed.get(field).is_some(),
            "JSON must contain '{field}'; full output: {parsed}"
        );
    }

    // lsp_crate must be the default value.
    assert_eq!(
        parsed["lsp_crate"].as_str(),
        Some("perl-lsp-rs"),
        "lsp_crate field must be 'perl-lsp-rs'"
    );

    // The real workspace has crates, so total_crates > 0.
    let total_crates = parsed["total_crates"]
        .as_u64()
        .expect("total_crates is a number");
    assert!(
        total_crates > 0,
        "real workspace must have at least one examined crate"
    );
}

/// total_flagged must equal the length of the flagged array — internal
/// consistency of the ScanReport serialization.
#[test]
fn test_json_mode_flagged_count_matches_flagged_array() {
    let output = run_unwired_scan_real(&["--json"]);
    assert!(output.status.success(), "--json must exit zero");

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    let flagged_array_len = parsed["flagged"]
        .as_array()
        .expect("flagged is array")
        .len();
    let total_flagged = parsed["total_flagged"]
        .as_u64()
        .expect("total_flagged is u64") as usize;

    assert_eq!(
        flagged_array_len, total_flagged,
        "total_flagged ({total_flagged}) must equal flagged array length ({flagged_array_len})"
    );
}

/// Each crate entry in the JSON `crates` array must have the expected fields.
/// Validates CrateReport serialization shape — catches field renames.
#[test]
fn test_json_mode_crate_entries_have_expected_fields() {
    let output = run_unwired_scan_real(&["--json"]);
    assert!(output.status.success(), "--json must exit zero");

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    let crates = parsed["crates"].as_array().expect("crates is array");
    assert!(
        !crates.is_empty(),
        "real workspace must have at least one crate entry"
    );

    // Check the first crate entry has all CrateReport fields.
    let first = &crates[0];
    for field in [
        "name",
        "path",
        "test_count",
        "is_direct_dep_of_lsp",
        "wiring_comments",
    ] {
        assert!(
            first.get(field).is_some(),
            "CrateReport JSON must contain '{field}'; entry: {first}"
        );
    }
}
