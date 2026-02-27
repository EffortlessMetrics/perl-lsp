//! BDD-style behavior tests for workspace discovery.

use perl_workspace_discovery::{DiscoveryMethod, discover_perl_files};
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
    assert!(!result.files.iter().any(|path| path.to_string_lossy().contains("node_modules")));

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
    assert!(result.files.iter().any(|path| path.ends_with("lib/App/Worker.pm")));
    assert!(!result.files.iter().any(|path| path.to_string_lossy().contains("node_modules")));

    Ok(())
}
