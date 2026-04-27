//! Mutation hardening tests for `path_normalize.rs`.
//!
//! Targets the boundary comparison `stack.len() <= workspace_depth` and the
//! component-dispatch match arms so that cargo-mutants cannot survive by
//! flipping `<=` to `<`, removing the `stack.pop()` call, or silently
//! discarding `ParentDir` components.

use perl_parser_core::path_normalize::normalize_path_within_workspace;
use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// Boundary: stack.len() <= workspace_depth
//
// The guard fires when the stack is exactly AT the workspace root (len ==
// workspace_depth). A mutation to `<` would allow one extra pop, letting
// the path escape by one component.
// ---------------------------------------------------------------------------

#[test]
fn single_parent_at_workspace_boundary_is_rejected() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    // One `..` from the workspace root must be rejected.
    let result = normalize_path_within_workspace(Path::new(".."), &workspace);
    assert!(result.is_err(), "single '..' at workspace root must return Err, got: {result:?}");
    Ok(())
}

#[test]
fn two_parents_at_workspace_boundary_are_rejected() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    let result = normalize_path_within_workspace(Path::new("../.."), &workspace);
    assert!(result.is_err(), "../../ at workspace root must return Err");
    Ok(())
}

#[test]
fn descend_then_escape_is_rejected() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    // Going one level in and then two levels out crosses the boundary.
    let result = normalize_path_within_workspace(Path::new("subdir/../../escape"), &workspace);
    assert!(result.is_err(), "subdir/../../escape must be rejected");
    Ok(())
}

#[test]
fn descend_then_return_within_workspace_is_ok() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    // Going in one level and back one level stays within the workspace.
    let result = normalize_path_within_workspace(Path::new("a/b/../c"), &workspace);
    assert!(result.is_ok(), "a/b/../c inside workspace must succeed, got {result:?}");
    let resolved = result?;
    assert!(resolved.starts_with(&workspace), "resolved path must stay under workspace");
    assert!(resolved.ends_with("c"), "resolved path must end in 'c'");
    Ok(())
}

#[test]
fn current_dir_component_is_ignored() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    let result = normalize_path_within_workspace(Path::new("./lib/./Foo.pm"), &workspace);
    assert!(result.is_ok(), "./lib/./Foo.pm must succeed, got {result:?}");
    let resolved = result?;
    assert!(resolved.starts_with(&workspace));
    assert!(resolved.ends_with("Foo.pm"));
    Ok(())
}

#[test]
fn absolute_component_in_relative_path_is_rejected() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    // A RootDir component triggers PathTraversalAttempt.
    // On Unix "/" appears as a RootDir component.
    let result = normalize_path_within_workspace(Path::new("/etc/passwd"), &workspace);
    assert!(result.is_err(), "absolute path must be rejected");
    Ok(())
}

// ---------------------------------------------------------------------------
// Verify precise output: the returned PathBuf is rooted at workspace_root
// and contains exactly the components we expect.
// ---------------------------------------------------------------------------

#[test]
fn simple_relative_path_resolves_correctly() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    let resolved = normalize_path_within_workspace(Path::new("lib/Module.pm"), &workspace)?;

    assert!(resolved.starts_with(&workspace));
    // The final path must contain both "lib" and "Module.pm" components.
    let components: Vec<_> = resolved.components().collect();
    let names: Vec<_> =
        components.iter().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    assert!(names.contains(&"lib".to_string()), "path must contain 'lib', got {names:?}");
    assert!(
        names.contains(&"Module.pm".to_string()),
        "path must contain 'Module.pm', got {names:?}"
    );
    Ok(())
}

#[test]
fn normalized_path_has_no_parent_dir_components() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    let resolved = normalize_path_within_workspace(Path::new("a/b/../c/d"), &workspace)?;

    for component in resolved.components() {
        assert_ne!(
            component,
            std::path::Component::ParentDir,
            "normalized path must not contain '..' components"
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Deeply nested escape — kills mutations that allow partial traversal.
// ---------------------------------------------------------------------------

#[test]
fn deeply_nested_escape_is_rejected() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().canonicalize()?;

    let result = normalize_path_within_workspace(
        Path::new("a/b/c/d/../../../../../../../../etc/shadow"),
        &workspace,
    );
    assert!(result.is_err(), "deeply nested escape must be rejected");
    Ok(())
}
