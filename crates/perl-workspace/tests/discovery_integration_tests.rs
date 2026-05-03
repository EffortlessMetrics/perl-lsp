//! End-to-end integration tests for workspace discovery.

use perl_workspace::discovery::{DiscoveryMethod, discover_perl_files};
use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn create_file(root: &Path, relative: &str) -> TestResult {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, "# integration fixture\n")?;
    Ok(())
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn run_git(root: &Path, args: &[&str]) -> TestResult {
    let status = Command::new("git").args(args).current_dir(root).status()?;
    if status.success() {
        return Ok(());
    }

    Err(format!("git command failed: git {}", args.join(" ")).into())
}

#[test]
fn discovers_files_via_walkdir_when_root_is_not_git_repo() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/One.pm")?;
    create_file(root, "bin/run.pl")?;
    create_file(root, "notes.txt")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Walk);
    assert_eq!(result.files.len(), 2);
    assert!(result.files.iter().any(|path| path.ends_with("lib/One.pm")));
    assert!(result.files.iter().any(|path| path.ends_with("bin/run.pl")));

    Ok(())
}

#[test]
fn discovers_files_via_git_and_honors_gitignore() -> TestResult {
    if !git_available() {
        return Ok(());
    }

    let tmp = TempDir::new()?;
    let root = tmp.path();

    run_git(root, &["init", "--quiet"])?;
    fs::write(root.join(".gitignore"), "target/\n")?;

    create_file(root, "lib/One.pm")?;
    create_file(root, "target/generated/Skipped.pm")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Git);
    assert!(result.files.iter().any(|path| path.ends_with("lib/One.pm")));
    assert!(!result.files.iter().any(|path| path.to_string_lossy().contains("/target/")));

    Ok(())
}

#[test]
fn discovers_git_files_when_repo_root_path_contains_skipped_component_name() -> TestResult {
    if !git_available() {
        return Ok(());
    }

    let tmp = TempDir::new()?;
    let root = tmp.path().join("target").join("workspace");
    fs::create_dir_all(&root)?;

    run_git(&root, &["init", "--quiet"])?;
    create_file(&root, "lib/One.pm")?;

    let result = discover_perl_files(&root);

    assert_eq!(result.method, DiscoveryMethod::Git);
    assert_eq!(result.files.len(), 1);
    assert!(result.files.iter().any(|path| path.ends_with("lib/One.pm")));

    Ok(())
}
