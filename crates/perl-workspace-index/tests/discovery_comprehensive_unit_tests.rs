//! Comprehensive unit tests for perl-workspace-discovery.
//!
//! Covers edge cases around the public API, symlinks, unicode paths,
//! root-level files, large workspaces, and discovery result invariants
//! not exercised by the existing test suites.

use perl_workspace::discovery::{DiscoveryMethod, DiscoveryResult, discover_perl_files};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

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

// --- discover_perl_files (public API) ---

#[test]
fn discover_perl_files_returns_non_negative_duration() -> TestResult {
    let tmp = TempDir::new()?;
    let result = discover_perl_files(tmp.path());
    // Duration should be representable and not absurd
    assert!(result.duration.as_secs() < 60);
    Ok(())
}

#[test]
fn discover_perl_files_empty_dir_returns_zero_files() -> TestResult {
    let tmp = TempDir::new()?;
    let result = discover_perl_files(tmp.path());
    assert!(result.files.is_empty());
    assert_eq!(result.excluded_count, 0);
    Ok(())
}

#[test]
fn discover_perl_files_finds_root_level_perl_files() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "app.pl")?;
    create_file(root, "Lib.pm")?;

    let result = discover_perl_files(root);
    assert_eq!(result.files.len(), 2);
    assert!(result.files.iter().any(|p| p.ends_with("app.pl")));
    assert!(result.files.iter().any(|p| p.ends_with("Lib.pm")));
    Ok(())
}

#[test]
fn discover_perl_files_returns_absolute_paths() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Module.pm")?;

    let result = discover_perl_files(root);
    for path in &result.files {
        assert!(path.is_absolute(), "expected absolute path, got: {path:?}");
    }
    Ok(())
}

#[test]
fn discover_perl_files_no_duplicates() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/A.pm")?;
    create_file(root, "lib/B.pm")?;
    create_file(root, "lib/sub/C.pm")?;
    create_file(root, "t/basic.t")?;
    create_file(root, "bin/run.pl")?;

    let result = discover_perl_files(root);
    let unique: HashSet<_> = result.files.iter().collect();
    assert_eq!(unique.len(), result.files.len(), "duplicate paths detected");
    Ok(())
}

#[test]
fn discover_perl_files_all_returned_paths_are_perl_sources() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Module.pm")?;
    create_file(root, "script.pl")?;
    create_file(root, "t/test.t")?;
    create_file(root, "app.psgi")?;
    create_file(root, "templates/page.tt")?;
    create_file(root, "templates/layout.tt2")?;
    create_file(root, "README.md")?;
    create_file(root, "Cargo.toml")?;

    let result = discover_perl_files(root);
    let valid_extensions: HashSet<&str> = ["pl", "pm", "t", "psgi", "xs", "ep", "tt", "tt2"]
        .iter()
        .copied()
        .collect();

    for path in &result.files {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| format!("no extension on: {path:?}"))?;
        assert!(
            valid_extensions.contains(ext),
            "unexpected extension {ext:?} in {path:?}"
        );
    }
    Ok(())
}

#[test]
fn discover_perl_files_excluded_count_matches_non_perl_visible_files() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    // 2 Perl files
    create_file(root, "lib/A.pm")?;
    create_file(root, "bin/run.pl")?;
    // 3 non-Perl files (these should be counted as excluded)
    create_file(root, "README.md")?;
    create_file(root, "Cargo.toml")?;
    create_file(root, "data.json")?;

    let result = discover_perl_files(root);
    assert_eq!(result.files.len(), 2);
    // excluded_count includes non-Perl files and skipped-dir files
    assert!(result.excluded_count >= 3);
    Ok(())
}

// --- Symlink behavior (follow_links is false) ---

#[cfg(unix)]
#[test]
fn discover_perl_files_does_not_follow_symlinks() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "real/Module.pm")?;
    fs::create_dir_all(root.join("links"))?;
    std::os::unix::fs::symlink(root.join("real"), root.join("links/real_link"))?;

    let result = discover_perl_files(root);
    // Should only find the real file, not the symlink target
    let paths_with_link: Vec<_> = result
        .files
        .iter()
        .filter(|p| p.to_string_lossy().contains("links/real_link"))
        .collect();
    assert!(
        paths_with_link.is_empty(),
        "symlinked directories should not be followed"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn discover_perl_files_handles_dangling_symlinks_gracefully() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Real.pm")?;
    std::os::unix::fs::symlink("/nonexistent/path", root.join("lib/Dangling.pm"))?;

    let result = discover_perl_files(root);
    // Should not crash on dangling symlinks and still find real files
    assert!(result.files.iter().any(|p| p.ends_with("Real.pm")));
    Ok(())
}

// --- Unicode file paths ---

#[test]
fn discover_perl_files_handles_unicode_directory_names() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "données/Module.pm")?;
    create_file(root, "日本語/Script.pl")?;

    let result = discover_perl_files(root);
    assert_eq!(result.files.len(), 2);
    Ok(())
}

#[test]
fn discover_perl_files_handles_unicode_filenames() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Ünïcödé.pm")?;

    let result = discover_perl_files(root);
    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].ends_with("Ünïcödé.pm"));
    Ok(())
}

// --- Deep nesting ---

#[test]
fn discover_perl_files_handles_deeply_nested_structures() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    let deep = "a/b/c/d/e/f/g/h/i/j/k/l/m/n/o";
    create_file(root, &format!("{deep}/Deep.pm"))?;

    let result = discover_perl_files(root);
    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].ends_with("Deep.pm"));
    Ok(())
}

// --- Many files ---

#[test]
fn discover_perl_files_handles_many_files() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    for i in 0..200 {
        create_file(root, &format!("lib/Module{i}.pm"))?;
    }

    let result = discover_perl_files(root);
    assert_eq!(result.files.len(), 200);
    Ok(())
}

// --- Skipped directory names as files (not directories) ---

#[test]
fn discover_perl_files_does_not_skip_files_named_like_skipped_dirs() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    // "target" here is part of the filename, not a directory component
    create_file(root, "lib/target.pm")?;

    let result = discover_perl_files(root);
    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].ends_with("target.pm"));
    Ok(())
}

// --- Files with no extension ---

#[test]
fn discover_perl_files_excludes_extensionless_files() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Module.pm")?;
    create_file(root, "Makefile")?;
    create_file(root, "cpanfile")?;

    let result = discover_perl_files(root);
    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].ends_with("Module.pm"));
    Ok(())
}

// --- Dotfiles that are Perl sources ---

#[test]
fn discover_perl_files_finds_hidden_perl_files_outside_skipped_dirs() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, ".hidden.pl")?;
    create_file(root, "lib/Visible.pm")?;

    let result = discover_perl_files(root);
    // Hidden .pl file should be discovered (it's not in a skipped dir)
    assert!(result.files.iter().any(|p| p.ends_with(".hidden.pl")));
    assert!(result.files.iter().any(|p| p.ends_with("Visible.pm")));
    Ok(())
}

// --- DiscoveryResult structural tests ---

#[test]
fn discovery_result_files_start_with_root() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Foo.pm")?;
    create_file(root, "t/bar.t")?;

    let result = discover_perl_files(root);
    for path in &result.files {
        assert!(
            path.starts_with(root),
            "path {path:?} does not start with root {root:?}"
        );
    }
    Ok(())
}

#[test]
fn discovery_result_method_is_walk_for_non_git_dir() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Foo.pm")?;

    let result = discover_perl_files(root);
    assert_eq!(result.method, DiscoveryMethod::Walk);
    Ok(())
}

#[test]
fn discovery_result_is_clonable() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Foo.pm")?;

    let result = discover_perl_files(root);
    let cloned: DiscoveryResult = result.clone();
    assert_eq!(cloned.files.len(), result.files.len());
    assert_eq!(cloned.method, result.method);
    assert_eq!(cloned.excluded_count, result.excluded_count);
    Ok(())
}

#[test]
fn discovery_result_debug_format_is_not_empty() -> TestResult {
    let tmp = TempDir::new()?;
    let result = discover_perl_files(tmp.path());
    let debug_str = format!("{result:?}");
    assert!(!debug_str.is_empty());
    Ok(())
}

// --- DiscoveryMethod trait coverage ---

#[test]
fn discovery_method_eq_symmetry() {
    assert_eq!(DiscoveryMethod::Git, DiscoveryMethod::Git);
    assert_eq!(DiscoveryMethod::Walk, DiscoveryMethod::Walk);
    assert_ne!(DiscoveryMethod::Git, DiscoveryMethod::Walk);
    assert_ne!(DiscoveryMethod::Walk, DiscoveryMethod::Git);
}

#[test]
fn discovery_method_copy_semantics() {
    let a = DiscoveryMethod::Git;
    let b = a; // Copy
    assert_eq!(a, b);

    let c = DiscoveryMethod::Walk;
    let d = c; // Copy
    assert_eq!(c, d);
}

// --- Multiple discovery calls on same workspace ---

#[test]
fn discover_perl_files_is_deterministic_across_calls() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/A.pm")?;
    create_file(root, "lib/B.pm")?;
    create_file(root, "t/test.t")?;

    let r1 = discover_perl_files(root);
    let r2 = discover_perl_files(root);

    let mut files1: Vec<_> = r1.files.iter().collect();
    let mut files2: Vec<_> = r2.files.iter().collect();
    files1.sort();
    files2.sort();

    assert_eq!(files1, files2, "repeated discovery should yield same files");
    assert_eq!(r1.method, r2.method);
    assert_eq!(r1.excluded_count, r2.excluded_count);
    Ok(())
}

// --- Adjacent skipped and non-skipped directories ---

#[test]
fn discover_perl_files_distinguishes_skipped_from_non_skipped_siblings() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    // Sibling directories: one skipped, one not
    create_file(root, "node_modules/Skipped.pm")?;
    create_file(root, "node_helpers/Visible.pm")?;
    create_file(root, "target/Build.pm")?;
    create_file(root, "target_utils/Helper.pm")?;

    let result = discover_perl_files(root);

    let found_names: HashSet<String> = result
        .files
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
        .collect();

    assert!(
        found_names.contains("Visible.pm"),
        "node_helpers/ should not be skipped"
    );
    assert!(
        found_names.contains("Helper.pm"),
        "target_utils/ should not be skipped"
    );
    assert!(
        !found_names.contains("Skipped.pm"),
        "node_modules/ should be skipped"
    );
    assert!(
        !found_names.contains("Build.pm"),
        "target/ should be skipped"
    );
    Ok(())
}

// --- Workspace with only skipped directories ---

#[test]
fn discover_perl_files_workspace_with_only_skipped_dirs() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "node_modules/A.pm")?;
    create_file(root, "target/B.pm")?;
    create_file(root, ".cache/C.pm")?;

    let result = discover_perl_files(root);
    assert!(result.files.is_empty());
    Ok(())
}

// --- Mixed extensions in same directory ---

#[test]
fn discover_perl_files_mixed_extensions_same_dir() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Module.pm")?;
    create_file(root, "lib/Module.pm.bak")?;
    create_file(root, "lib/Module.pm~")?;
    create_file(root, "lib/README.md")?;
    create_file(root, "lib/script.pl")?;

    let result = discover_perl_files(root);
    let found_names: HashSet<String> = result
        .files
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
        .collect();

    assert!(found_names.contains("Module.pm"));
    assert!(found_names.contains("script.pl"));
    assert!(!found_names.contains("Module.pm.bak"));
    assert!(!found_names.contains("Module.pm~"));
    assert!(!found_names.contains("README.md"));
    Ok(())
}

// --- Spaces in paths ---

#[test]
fn discover_perl_files_handles_spaces_in_paths() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "my project/lib/My Module.pm")?;
    create_file(root, "path with spaces/script.pl")?;

    let result = discover_perl_files(root);
    assert_eq!(result.files.len(), 2);
    Ok(())
}

// --- All eight Perl extensions in one workspace ---

#[test]
fn discover_perl_files_finds_all_eight_perl_extensions() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "script.pl")?;
    create_file(root, "Module.pm")?;
    create_file(root, "test.t")?;
    create_file(root, "app.psgi")?;
    create_file(root, "native.xs")?;
    create_file(root, "templates/page.html.ep")?;
    create_file(root, "templates/page.tt")?;
    create_file(root, "templates/layout.tt2")?;

    let result = discover_perl_files(root);
    assert_eq!(result.files.len(), 8);

    let extensions: HashSet<String> = result
        .files
        .iter()
        .filter_map(|p| p.extension().and_then(|e| e.to_str()).map(String::from))
        .collect();

    assert!(extensions.contains("pl"));
    assert!(extensions.contains("pm"));
    assert!(extensions.contains("t"));
    assert!(extensions.contains("psgi"));
    assert!(extensions.contains("xs"));
    assert!(extensions.contains("ep"));
    assert!(extensions.contains("tt"));
    assert!(extensions.contains("tt2"));
    Ok(())
}
