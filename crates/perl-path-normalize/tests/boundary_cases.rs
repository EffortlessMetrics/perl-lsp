//! Mutation-killing tests for perl-path-normalize.
//!
//! The existing 2 tests cover: safe relative path and parent-traversal escape.
//! Untested branches:
//!   - Component::CurDir ('.'): must be silently ignored
//!   - Component::RootDir ('/') in path triggers PathTraversalAttempt
//!   - Traversal at exact workspace depth: should fail (not pop below root)
//!   - Mixed safe + parent components resolve correctly within workspace
//!   - Error display format
//!   - Deeply nested path resolution

use perl_path_normalize::{NormalizePathError, normalize_path_within_workspace};
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// CurDir component ('.'): must be silently ignored
// ---------------------------------------------------------------------------

#[test]
fn cur_dir_component_is_ignored() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    // "./src/main.pl" should be equivalent to "src/main.pl"
    let result = normalize_path_within_workspace(&PathBuf::from("./src/main.pl"), &workspace)?;
    assert!(result.starts_with(&workspace), "CurDir must not prevent valid resolution");
    assert!(result.to_string_lossy().contains("main.pl"));
    Ok(())
}

#[test]
fn multiple_cur_dir_components_are_ignored() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    let result = normalize_path_within_workspace(&PathBuf::from("././lib/Foo.pm"), &workspace)?;
    assert!(result.starts_with(&workspace));
    assert!(result.to_string_lossy().contains("Foo.pm"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Absolute path triggers RootDir → PathTraversalAttempt
// ---------------------------------------------------------------------------

#[test]
fn absolute_path_is_rejected_as_traversal() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    let result =
        normalize_path_within_workspace(&PathBuf::from("/etc/passwd"), &workspace);
    assert!(
        matches!(result, Err(NormalizePathError::PathTraversalAttempt(_))),
        "Absolute path must trigger PathTraversalAttempt"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Traversal at exact workspace depth must fail
// ---------------------------------------------------------------------------

#[test]
fn parent_traversal_at_workspace_depth_is_rejected() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    // Exactly at workspace root, another '..' must be rejected
    let result = normalize_path_within_workspace(&PathBuf::from(".."), &workspace);
    assert!(
        matches!(result, Err(NormalizePathError::PathTraversalAttempt(_))),
        "'..' at workspace root must be rejected"
    );
    Ok(())
}

#[test]
fn single_step_in_then_parent_stays_in_workspace() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    // "src/../lib/Foo.pm" → stays within workspace
    let result = normalize_path_within_workspace(
        &PathBuf::from("src/../lib/Foo.pm"),
        &workspace,
    )?;
    assert!(result.starts_with(&workspace), "src/../lib must stay within workspace");
    assert!(result.to_string_lossy().contains("lib"));
    assert!(result.to_string_lossy().contains("Foo.pm"));
    Ok(())
}

#[test]
fn two_steps_in_then_two_parents_stay_in_workspace() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    // "a/b/../../c" → resolves to <workspace>/c
    let result = normalize_path_within_workspace(
        &PathBuf::from("a/b/../../c"),
        &workspace,
    )?;
    assert!(result.starts_with(&workspace));
    assert!(result.to_string_lossy().ends_with("c"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Deeply nested safe path
// ---------------------------------------------------------------------------

#[test]
fn deeply_nested_path_resolves_correctly() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    let result = normalize_path_within_workspace(
        &PathBuf::from("a/b/c/d/e/f.pl"),
        &workspace,
    )?;
    assert!(result.starts_with(&workspace));
    assert!(result.to_string_lossy().ends_with("f.pl"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Error display format
// ---------------------------------------------------------------------------

#[test]
fn path_traversal_error_display_contains_path_info() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    let err = normalize_path_within_workspace(&PathBuf::from("../../../etc"), &workspace)
        .unwrap_err();
    let display = err.to_string();
    assert!(
        display.contains("workspace") || display.contains("etc") || display.contains("traversal"),
        "Error display must contain path context: {display}"
    );
    Ok(())
}

#[test]
fn path_traversal_error_implements_clone_and_eq() {
    let e1 = NormalizePathError::PathTraversalAttempt("test".to_string());
    let e2 = e1.clone();
    assert_eq!(e1, e2);
}

// ---------------------------------------------------------------------------
// Empty relative path → returns workspace root
// ---------------------------------------------------------------------------

#[test]
fn empty_path_returns_workspace_root() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    let result = normalize_path_within_workspace(&PathBuf::from(""), &workspace)?;
    assert_eq!(result, workspace, "empty path must return workspace root");
    Ok(())
}
