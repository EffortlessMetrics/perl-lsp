//! BDD-style behavior tests for workspace discovery.

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
    fs::write(path, "# fixture\n")?;
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
fn given_non_git_workspace_when_discovering_then_walk_fallback_finds_perl_sources() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "app.pl")?;
    create_file(root, "lib/Foo.pm")?;
    create_file(root, "README.md")?;
    create_file(root, "node_modules/vendor.pm")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Walk);
    assert_eq!(result.files.len(), 2);
    assert!(result.files.iter().any(|path| path.ends_with("app.pl")));
    assert!(result.files.iter().any(|path| path.ends_with("lib/Foo.pm")));
    assert!(
        !result
            .files
            .iter()
            .any(|path| path.to_string_lossy().contains("node_modules"))
    );

    Ok(())
}

#[test]
fn given_git_workspace_when_discovering_then_git_strategy_respects_gitignore() -> TestResult {
    if !git_available() {
        return Ok(());
    }

    let tmp = TempDir::new()?;
    let root = tmp.path();

    run_git(root, &["init", "--quiet"])?;
    fs::write(root.join(".gitignore"), "node_modules/\n")?;

    create_file(root, "script.pl")?;
    create_file(root, "lib/App/Worker.pm")?;
    create_file(root, "docs/readme.md")?;
    create_file(root, "node_modules/ignored.pm")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Git);
    assert!(result.files.iter().any(|path| path.ends_with("script.pl")));
    assert!(
        result
            .files
            .iter()
            .any(|path| path.ends_with("lib/App/Worker.pm"))
    );
    assert!(
        !result
            .files
            .iter()
            .any(|path| path.to_string_lossy().contains("node_modules"))
    );

    Ok(())
}

#[test]
fn given_git_workspace_with_untracked_sources_when_discovering_then_git_strategy_includes_them()
-> TestResult {
    if !git_available() {
        return Ok(());
    }

    let tmp = TempDir::new()?;
    let root = tmp.path();

    run_git(root, &["init", "--quiet"])?;
    create_file(root, "tracked/Module.pm")?;
    create_file(root, "untracked/script.pl")?;
    create_file(root, "notes.txt")?;
    run_git(root, &["add", "tracked/Module.pm"])?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Git);
    assert!(
        result
            .files
            .iter()
            .any(|path| path.ends_with("tracked/Module.pm"))
    );
    assert!(
        result
            .files
            .iter()
            .any(|path| path.ends_with("untracked/script.pl"))
    );
    assert!(!result.files.iter().any(|path| path.ends_with("notes.txt")));

    Ok(())
}

#[test]
fn given_git_workspace_with_tracked_noise_inside_skipped_dir_when_discovering_then_skip_rules_still_win()
-> TestResult {
    if !git_available() {
        return Ok(());
    }

    let tmp = TempDir::new()?;
    let root = tmp.path();

    run_git(root, &["init", "--quiet"])?;
    create_file(root, "lib/Kept.pm")?;
    create_file(root, "target/generated/Tracked.pm")?;
    create_file(root, "node_modules/vendor/TrackedToo.pm")?;
    run_git(root, &["add", "."])?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Git);
    assert_eq!(result.files.len(), 1);
    assert!(
        result
            .files
            .iter()
            .any(|path| path.ends_with("lib/Kept.pm"))
    );
    assert!(
        !result
            .files
            .iter()
            .any(|path| path.to_string_lossy().contains("target/generated"))
    );
    assert!(
        !result
            .files
            .iter()
            .any(|path| path.to_string_lossy().contains("node_modules/vendor"))
    );
    assert!(result.excluded_count >= 2);

    Ok(())
}

#[test]
fn given_git_workspace_with_only_ignored_or_non_perl_files_when_discovering_then_result_is_empty_but_git_strategy_is_used()
-> TestResult {
    if !git_available() {
        return Ok(());
    }

    let tmp = TempDir::new()?;
    let root = tmp.path();

    run_git(root, &["init", "--quiet"])?;
    fs::write(root.join(".gitignore"), "build/\n")?;
    create_file(root, "README.md")?;
    create_file(root, "build/ignored.pm")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Git);
    assert!(result.files.is_empty());
    assert!(result.excluded_count >= 1);

    Ok(())
}
