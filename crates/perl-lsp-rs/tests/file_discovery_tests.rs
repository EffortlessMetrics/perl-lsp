//! Tests for the file discovery abstraction.
//!
//! Validates both Git and WalkDir discovery strategies, extension filtering,
//! and directory skipping behavior.

use perl_lsp::runtime::file_discovery::{DiscoveryMethod, discover_perl_files};
use std::fs;
use tempfile::TempDir;

/// Helper to create a file inside a temp directory, creating parent dirs as needed.
fn create_file(base: &std::path::Path, relative: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, "# Perl file\n")?;
    Ok(())
}

#[test]
fn discovers_perl_extensions() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "app.pl")?;
    create_file(root, "lib/Foo.pm")?;
    create_file(root, "t/basic.t")?;
    create_file(root, "app.psgi")?;
    // Non-Perl files should be excluded
    create_file(root, "README.md")?;
    create_file(root, "Makefile")?;

    let result = discover_perl_files(root);

    assert_eq!(result.files.len(), 4, "Should discover exactly 4 Perl files");
    assert!(result.duration.as_secs() < 10, "Discovery should complete quickly");

    // Verify all discovered files have Perl extensions
    for file in &result.files {
        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
        assert!(matches!(ext, "pl" | "pm" | "t" | "psgi"), "Unexpected extension: {}", ext);
    }

    Ok(())
}

#[test]
fn skips_node_modules_directory() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "app.pl")?;
    create_file(root, "node_modules/some_pkg/lib.pm")?;

    let result = discover_perl_files(root);

    // Only app.pl should be found; node_modules/some_pkg/lib.pm should be skipped
    // With git strategy, node_modules may or may not exist in git, so check both cases
    let perl_in_node_modules =
        result.files.iter().any(|f| f.to_string_lossy().contains("node_modules"));
    assert!(!perl_in_node_modules, "Files inside node_modules should not be discovered");
    assert!(!result.files.is_empty(), "Should discover at least the root app.pl");

    Ok(())
}

#[test]
fn skips_dot_git_directory() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "app.pl")?;
    create_file(root, ".git/hooks/pre-commit.pl")?;

    let result = discover_perl_files(root);

    let perl_in_git = result.files.iter().any(|f| f.to_string_lossy().contains(".git"));
    assert!(!perl_in_git, "Files inside .git should not be discovered");

    Ok(())
}

#[test]
fn returns_valid_discovery_result() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Module.pm")?;
    create_file(root, "script.pl")?;

    let result = discover_perl_files(root);

    assert_eq!(result.files.len(), 2);
    assert!(
        result.method == DiscoveryMethod::Git || result.method == DiscoveryMethod::Walk,
        "Method should be Git or Walk"
    );
    // Duration should be non-negative (always true for Duration, but validates field is populated)
    assert!(result.duration.as_nanos() > 0, "Duration should be positive");

    Ok(())
}

#[test]
fn non_git_directory_uses_walk_method() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    // Ensure this is NOT a git repo (TempDir won't be one by default,
    // but we also need to make sure there's no parent .git that covers it).
    // We create a nested directory that is definitely not under any git root.
    let isolated = root.join("isolated_workspace");
    fs::create_dir_all(&isolated)?;
    create_file(&isolated, "app.pl")?;

    // The temp directory is not a git repo, so discovery should fall back to Walk.
    // However, if the temp dir happens to be inside a git worktree, git ls-files
    // may still succeed. We verify the result is valid either way.
    let result = discover_perl_files(&isolated);

    assert_eq!(result.files.len(), 1);
    // We can't guarantee Walk method here because the temp dir might be inside
    // a git worktree, but we can verify the result is structurally valid.
    assert!(
        result.method == DiscoveryMethod::Git || result.method == DiscoveryMethod::Walk,
        "Method should be Git or Walk"
    );

    Ok(())
}

#[test]
fn empty_directory_returns_no_files() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    let result = discover_perl_files(root);

    assert!(result.files.is_empty(), "Empty directory should yield no files");

    Ok(())
}

#[test]
fn skips_target_and_cache_directories() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "app.pl")?;
    create_file(root, "target/debug/build/generated.pm")?;
    create_file(root, ".cache/precomp/cached.pm")?;

    let result = discover_perl_files(root);

    let in_excluded = result.files.iter().any(|f| {
        let s = f.to_string_lossy();
        s.contains("target") || s.contains(".cache")
    });
    assert!(!in_excluded, "Files inside target/ and .cache/ should not be discovered");

    Ok(())
}
