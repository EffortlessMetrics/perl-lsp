//! Targeted test coverage for perl-workspace-discovery.
//!
//! Covers file discovery, filtering, symlink handling, hidden directory skipping,
//! large directory handling, and git/walk strategy selection edge cases that are
//! not exercised by the existing test suites.

use perl_workspace::discovery::{DiscoveryMethod, discover_perl_files};
use std::collections::HashSet;
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
    fs::write(path, "# coverage fixture\n")?;
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

// ============================================================
// Git-based file discovery
// ============================================================

#[test]
fn git_discovery_finds_committed_and_untracked_perl_files() -> TestResult {
    if !git_available() {
        return Ok(());
    }

    let tmp = TempDir::new()?;
    let root = tmp.path();

    run_git(root, &["init", "--quiet"])?;

    // Committed file
    create_file(root, "lib/Committed.pm")?;
    run_git(root, &["add", "lib/Committed.pm"])?;
    run_git(root, &["commit", "-m", "initial", "--quiet"])?;

    // Untracked file
    create_file(root, "lib/Untracked.pm")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Git);
    assert!(result.files.iter().any(|p| p.ends_with("Committed.pm")));
    assert!(result.files.iter().any(|p| p.ends_with("Untracked.pm")));
    assert_eq!(result.files.len(), 2);

    Ok(())
}

#[test]
fn git_discovery_excludes_gitignored_perl_files() -> TestResult {
    if !git_available() {
        return Ok(());
    }

    let tmp = TempDir::new()?;
    let root = tmp.path();

    run_git(root, &["init", "--quiet"])?;
    fs::write(root.join(".gitignore"), "build/\n*.bak\n")?;

    create_file(root, "lib/Visible.pm")?;
    create_file(root, "build/Generated.pm")?;
    create_file(root, "lib/Old.pm.bak")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Git);
    assert!(result.files.iter().any(|p| p.ends_with("Visible.pm")));
    assert!(!result.files.iter().any(|p| p.to_string_lossy().contains("build/")));
    // .bak files are not perl extensions anyway, but gitignore also suppresses them
    assert!(!result.files.iter().any(|p| p.to_string_lossy().contains(".bak")));

    Ok(())
}

#[test]
fn git_discovery_finds_root_level_perl_files() -> TestResult {
    if !git_available() {
        return Ok(());
    }

    let tmp = TempDir::new()?;
    let root = tmp.path();

    run_git(root, &["init", "--quiet"])?;
    create_file(root, "app.pl")?;
    create_file(root, "Module.pm")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Git);
    assert_eq!(result.files.len(), 2);
    assert!(result.files.iter().any(|p| p.ends_with("app.pl")));
    assert!(result.files.iter().any(|p| p.ends_with("Module.pm")));

    Ok(())
}

#[test]
fn git_discovery_returns_absolute_paths() -> TestResult {
    if !git_available() {
        return Ok(());
    }

    let tmp = TempDir::new()?;
    let root = tmp.path();

    run_git(root, &["init", "--quiet"])?;
    create_file(root, "lib/Abs.pm")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Git);
    for path in &result.files {
        assert!(path.is_absolute(), "expected absolute path, got: {path:?}");
    }

    Ok(())
}

#[test]
fn git_discovery_with_gitignore_glob_patterns() -> TestResult {
    if !git_available() {
        return Ok(());
    }

    let tmp = TempDir::new()?;
    let root = tmp.path();

    run_git(root, &["init", "--quiet"])?;
    fs::write(root.join(".gitignore"), "*.generated.pm\ntmp_*\n")?;

    create_file(root, "lib/Clean.pm")?;
    create_file(root, "lib/Auto.generated.pm")?;
    create_file(root, "tmp_scratch/Work.pm")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Git);
    assert!(result.files.iter().any(|p| p.ends_with("Clean.pm")));
    assert!(!result.files.iter().any(|p| p.to_string_lossy().contains("generated")));
    assert!(!result.files.iter().any(|p| p.to_string_lossy().contains("tmp_scratch")));

    Ok(())
}

#[test]
fn git_discovery_with_nested_gitignore() -> TestResult {
    if !git_available() {
        return Ok(());
    }

    let tmp = TempDir::new()?;
    let root = tmp.path();

    run_git(root, &["init", "--quiet"])?;

    // Root .gitignore ignores nothing Perl
    fs::write(root.join(".gitignore"), "*.log\n")?;
    // Nested .gitignore ignores vendor/
    fs::create_dir_all(root.join("lib"))?;
    fs::write(root.join("lib/.gitignore"), "vendor/\n")?;

    create_file(root, "lib/App.pm")?;
    create_file(root, "lib/vendor/External.pm")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Git);
    assert!(result.files.iter().any(|p| p.ends_with("App.pm")));
    assert!(!result.files.iter().any(|p| p.to_string_lossy().contains("vendor")));

    Ok(())
}

// ============================================================
// WalkDir fallback
// ============================================================

#[test]
fn walk_fallback_when_no_git_repo_exists() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Module.pm")?;
    create_file(root, "t/test.t")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Walk);
    assert_eq!(result.files.len(), 2);

    Ok(())
}

#[test]
fn walk_fallback_finds_files_in_nested_subdirectories() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "a/b/c/Deep.pm")?;
    create_file(root, "x/y/Script.pl")?;
    create_file(root, "tests/unit/basic.t")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Walk);
    assert_eq!(result.files.len(), 3);
    assert!(result.files.iter().any(|p| p.ends_with("Deep.pm")));
    assert!(result.files.iter().any(|p| p.ends_with("Script.pl")));
    assert!(result.files.iter().any(|p| p.ends_with("basic.t")));

    Ok(())
}

#[test]
fn walk_fallback_excluded_count_tracks_non_perl_files() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Module.pm")?;
    create_file(root, "README.md")?;
    create_file(root, "Cargo.toml")?;
    create_file(root, "Makefile")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Walk);
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.excluded_count, 3);

    Ok(())
}

// ============================================================
// .gitignore respect
// ============================================================

#[test]
fn gitignore_directory_pattern_excludes_entire_subtree() -> TestResult {
    if !git_available() {
        return Ok(());
    }

    let tmp = TempDir::new()?;
    let root = tmp.path();

    run_git(root, &["init", "--quiet"])?;
    fs::write(root.join(".gitignore"), "vendor/\n")?;

    create_file(root, "lib/Kept.pm")?;
    create_file(root, "vendor/External.pm")?;
    create_file(root, "vendor/deep/nested/Also.pm")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Git);
    assert_eq!(result.files.len(), 1);
    assert!(result.files.iter().any(|p| p.ends_with("Kept.pm")));

    Ok(())
}

#[test]
fn gitignore_negation_pattern_re_includes_file_level() -> TestResult {
    if !git_available() {
        return Ok(());
    }

    let tmp = TempDir::new()?;
    let root = tmp.path();

    run_git(root, &["init", "--quiet"])?;
    // Ignore all .pm files then re-include Important.pm specifically
    fs::write(root.join(".gitignore"), "lib/*.pm\n!lib/Important.pm\n")?;

    create_file(root, "lib/Ignored.pm")?;
    create_file(root, "lib/Important.pm")?;
    create_file(root, "lib/Also_Ignored.pm")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Git);
    assert!(result.files.iter().any(|p| p.ends_with("Important.pm")));
    assert!(!result.files.iter().any(|p| p.ends_with("Ignored.pm")));
    assert!(!result.files.iter().any(|p| p.ends_with("Also_Ignored.pm")));

    Ok(())
}

// ============================================================
// Perl file extension filtering (.pl, .pm, .t, .psgi, .xs, .i, .ep, .tt, .tt2)
// ============================================================

#[test]
fn extension_filtering_accepts_all_nine_perl_extensions() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "script.pl")?;
    create_file(root, "Module.pm")?;
    create_file(root, "test.t")?;
    create_file(root, "app.psgi")?;
    create_file(root, "native.xs")?;
    create_file(root, "interface.i")?;
    create_file(root, "templates/page.html.ep")?;
    create_file(root, "templates/page.tt")?;
    create_file(root, "templates/layout.tt2")?;

    let result = discover_perl_files(root);

    let extensions: HashSet<String> = result
        .files
        .iter()
        .filter_map(|p| p.extension().and_then(|e| e.to_str()).map(String::from))
        .collect();

    assert_eq!(extensions.len(), 9);
    assert!(extensions.contains("pl"));
    assert!(extensions.contains("pm"));
    assert!(extensions.contains("t"));
    assert!(extensions.contains("psgi"));
    assert!(extensions.contains("xs"));
    assert!(extensions.contains("i"));
    assert!(extensions.contains("ep"));
    assert!(extensions.contains("tt"));
    assert!(extensions.contains("tt2"));

    Ok(())
}

#[test]
fn extension_filtering_rejects_non_perl_extensions() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "README.md")?;
    create_file(root, "config.yaml")?;
    create_file(root, "data.json")?;
    create_file(root, "code.rs")?;
    create_file(root, "code.py")?;
    create_file(root, "code.rb")?;
    create_file(root, "script.sh")?;
    create_file(root, "Makefile.PL")?; // .PL is not .pl in the extension check context

    let result = discover_perl_files(root);

    // Makefile.PL has extension "PL" which is_perl_source_extension matches case-insensitively
    let non_makefile_pl: Vec<_> =
        result.files.iter().filter(|p| !p.to_string_lossy().contains("Makefile.PL")).collect();

    // None of the non-perl files should appear (except possibly Makefile.PL)
    for path in &non_makefile_pl {
        let ext = path.extension().and_then(|e| e.to_str()).ok_or("missing extension")?;
        assert!(
            ["pl", "pm", "t", "psgi"].contains(&ext.to_lowercase().as_str()),
            "unexpected non-perl file: {path:?}"
        );
    }

    Ok(())
}

#[test]
fn extension_filtering_rejects_extensionless_files() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "Makefile")?;
    create_file(root, "cpanfile")?;
    create_file(root, "LICENSE")?;
    create_file(root, "lib/Module.pm")?;

    let result = discover_perl_files(root);

    assert_eq!(result.files.len(), 1);
    assert!(result.files.iter().any(|p| p.ends_with("Module.pm")));

    Ok(())
}

#[test]
fn extension_filtering_rejects_double_extensions() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Module.pm.bak")?;
    create_file(root, "lib/Module.pm.orig")?;
    create_file(root, "lib/Module.pm.swp")?;
    create_file(root, "lib/Real.pm")?;

    let result = discover_perl_files(root);

    // Only Real.pm should be found; .bak/.orig/.swp are the actual extensions
    assert_eq!(result.files.len(), 1);
    assert!(result.files.iter().any(|p| p.ends_with("Real.pm")));

    Ok(())
}

// ============================================================
// Symlink handling (should NOT follow)
// ============================================================

#[cfg(unix)]
#[test]
fn symlink_to_directory_is_not_followed() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    // Real directory with a Perl file
    create_file(root, "real_lib/Module.pm")?;

    // Symlink to that directory
    std::os::unix::fs::symlink(root.join("real_lib"), root.join("linked_lib"))?;

    let result = discover_perl_files(root);

    // Should find the file in real_lib but NOT via the symlink
    let linked_files: Vec<_> =
        result.files.iter().filter(|p| p.to_string_lossy().contains("linked_lib")).collect();

    assert!(linked_files.is_empty(), "symlinked directory should not be followed");
    assert!(result.files.iter().any(|p| p.ends_with("real_lib/Module.pm")));

    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_to_file_is_not_counted_as_regular_file() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "real/Original.pm")?;
    fs::create_dir_all(root.join("links"))?;
    std::os::unix::fs::symlink(root.join("real/Original.pm"), root.join("links/Linked.pm"))?;

    let result = discover_perl_files(root);

    // The real file should be found
    assert!(result.files.iter().any(|p| p.ends_with("real/Original.pm")));

    // The symlink should NOT be treated as a regular file
    let linked: Vec<_> =
        result.files.iter().filter(|p| p.to_string_lossy().contains("links/Linked.pm")).collect();
    assert!(linked.is_empty(), "symlink to file should not be followed");

    Ok(())
}

#[cfg(unix)]
#[test]
fn dangling_symlink_does_not_crash_discovery() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Real.pm")?;

    // Dangling symlink pointing to nonexistent target
    fs::create_dir_all(root.join("lib"))?;
    std::os::unix::fs::symlink("/nonexistent/path/Module.pm", root.join("lib/Dangling.pm"))?;

    let result = discover_perl_files(root);

    // Should gracefully skip the dangling symlink and still find Real.pm
    assert!(result.files.iter().any(|p| p.ends_with("Real.pm")));
    assert!(!result.files.iter().any(|p| p.ends_with("Dangling.pm")));

    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_cycle_does_not_cause_infinite_loop() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Module.pm")?;

    // Create a symlink cycle: a -> b -> a
    fs::create_dir_all(root.join("a"))?;
    fs::create_dir_all(root.join("b"))?;
    std::os::unix::fs::symlink(root.join("b"), root.join("a/link_to_b"))?;
    std::os::unix::fs::symlink(root.join("a"), root.join("b/link_to_a"))?;

    // This should complete without hanging
    let result = discover_perl_files(root);

    assert!(result.files.iter().any(|p| p.ends_with("Module.pm")));

    Ok(())
}

// ============================================================
// Hidden directory skipping
// ============================================================

#[test]
fn hidden_directories_in_skip_list_are_skipped() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    // These hidden dirs are in the skip list
    create_file(root, ".git/hooks/hook.pm")?;
    create_file(root, ".cache/fast.pm")?;
    create_file(root, ".hermes/conveyor/work-1234/session.pm")?;

    // Visible file for comparison
    create_file(root, "lib/Visible.pm")?;

    let result = discover_perl_files(root);

    assert_eq!(result.files.len(), 1);
    assert!(result.files.iter().any(|p| p.ends_with("Visible.pm")));

    Ok(())
}

#[test]
fn hidden_directories_not_in_skip_list_are_traversed() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    // Hidden dirs NOT in the skip list should be traversed
    create_file(root, ".perltidyrc/config.pm")?;
    create_file(root, ".local/share/Module.pm")?;
    create_file(root, ".config/perl/Config.pm")?;

    let result = discover_perl_files(root);

    // All three should be found because .perltidyrc, .local, .config are not skipped
    assert_eq!(result.files.len(), 3);

    Ok(())
}

#[test]
fn hidden_perl_files_at_root_are_discovered() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, ".hidden_script.pl")?;
    create_file(root, "visible.pl")?;

    let result = discover_perl_files(root);

    assert_eq!(result.files.len(), 2);
    assert!(result.files.iter().any(|p| p.ends_with(".hidden_script.pl")));
    assert!(result.files.iter().any(|p| p.ends_with("visible.pl")));

    Ok(())
}

#[test]
fn all_core_skipped_directories_are_excluded_from_walk() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    let skipped = [".git", ".hg", ".svn", "target", "node_modules", ".cache", ".hermes"];
    for dir in skipped {
        create_file(root, &format!("{dir}/nested/Module.pm"))?;
    }
    create_file(root, "lib/Kept.pm")?;

    let result = discover_perl_files(root);

    assert_eq!(result.files.len(), 1);
    assert!(result.files.iter().any(|p| p.ends_with("Kept.pm")));

    // Verify none of the skipped dirs leaked through
    for path in &result.files {
        let path_str = path.to_string_lossy();
        for dir in &skipped {
            assert!(
                !path_str.contains(&format!("/{dir}/")),
                "skipped dir {dir} leaked into results: {path_str}"
            );
        }
    }

    Ok(())
}

#[test]
fn directories_with_similar_names_to_skipped_dirs_are_not_skipped() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    // These look similar to skipped dirs but should NOT be skipped
    create_file(root, "target_release/Module.pm")?;
    create_file(root, "my_target/Script.pl")?;
    create_file(root, "node_modules_backup/Old.pm")?;
    create_file(root, "git_tools/Helper.pm")?;
    create_file(root, "caches/Fast.pm")?;

    // These should be skipped
    create_file(root, "target/Build.pm")?;
    create_file(root, "node_modules/Dep.pm")?;

    let result = discover_perl_files(root);

    let found_names: HashSet<String> = result
        .files
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
        .collect();

    // Similar names should be found
    assert!(found_names.contains("Module.pm"), "target_release/ should not be skipped");
    assert!(found_names.contains("Script.pl"), "my_target/ should not be skipped");
    assert!(found_names.contains("Old.pm"), "node_modules_backup/ should not be skipped");
    assert!(found_names.contains("Helper.pm"), "git_tools/ should not be skipped");
    assert!(found_names.contains("Fast.pm"), "caches/ should not be skipped");

    // Exact matches should be skipped
    assert!(!found_names.contains("Build.pm"), "target/ should be skipped");
    assert!(!found_names.contains("Dep.pm"), "node_modules/ should be skipped");

    Ok(())
}

// ============================================================
// Large directory handling
// ============================================================

#[test]
fn large_directory_with_many_perl_files() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    // Create 500 Perl files across multiple directories
    for i in 0..500 {
        let dir = format!("lib/tier{}/sub{}", i / 50, i / 10);
        create_file(root, &format!("{dir}/Module{i}.pm"))?;
    }

    let result = discover_perl_files(root);

    assert_eq!(result.files.len(), 500);
    // All paths should be unique
    let unique: HashSet<_> = result.files.iter().collect();
    assert_eq!(unique.len(), 500, "all discovered paths should be unique");

    Ok(())
}

#[test]
fn large_directory_with_mixed_perl_and_non_perl() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    let mut expected_perl = 0usize;
    let mut expected_excluded = 0usize;

    for i in 0..200 {
        if i % 3 == 0 {
            create_file(root, &format!("lib/Module{i}.pm"))?;
            expected_perl += 1;
        } else if i % 3 == 1 {
            create_file(root, &format!("lib/File{i}.txt"))?;
            expected_excluded += 1;
        } else {
            create_file(root, &format!("lib/Data{i}.json"))?;
            expected_excluded += 1;
        }
    }

    let result = discover_perl_files(root);

    assert_eq!(result.files.len(), expected_perl);
    assert_eq!(result.excluded_count, expected_excluded);

    Ok(())
}

#[test]
fn large_directory_with_many_subdirectories_some_skipped() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    // Real directories
    for i in 0..50 {
        create_file(root, &format!("lib/pkg{i}/Module.pm"))?;
    }

    // Skipped directories (files here should not be found)
    for i in 0..10 {
        create_file(root, &format!("node_modules/dep{i}/index.pm"))?;
    }

    let result = discover_perl_files(root);

    assert_eq!(result.files.len(), 50);

    // No files from node_modules
    for path in &result.files {
        assert!(!path.to_string_lossy().contains("node_modules"));
    }

    Ok(())
}

// ============================================================
// Empty directory edge cases
// ============================================================

#[test]
fn empty_workspace_returns_zero_files() -> TestResult {
    let tmp = TempDir::new()?;
    let result = discover_perl_files(tmp.path());

    assert!(result.files.is_empty());
    assert_eq!(result.excluded_count, 0);

    Ok(())
}

#[test]
fn workspace_with_only_empty_subdirectories() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    fs::create_dir_all(root.join("lib"))?;
    fs::create_dir_all(root.join("t"))?;
    fs::create_dir_all(root.join("bin"))?;

    let result = discover_perl_files(root);

    assert!(result.files.is_empty());
    assert_eq!(result.excluded_count, 0);

    Ok(())
}

// ============================================================
// Discovery result invariants
// ============================================================

#[test]
fn discovery_files_plus_excluded_equals_total_visible_files() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    // 4 Perl files
    create_file(root, "a.pl")?;
    create_file(root, "b.pm")?;
    create_file(root, "c.t")?;
    create_file(root, "d.psgi")?;

    // 3 non-Perl files
    create_file(root, "readme.md")?;
    create_file(root, "config.yaml")?;
    create_file(root, "data.csv")?;

    let result = discover_perl_files(root);

    assert_eq!(result.files.len(), 4);
    assert_eq!(result.excluded_count, 3);
    assert_eq!(result.files.len() + result.excluded_count, 7);

    Ok(())
}

#[test]
fn discovery_duration_is_finite() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Module.pm")?;

    let result = discover_perl_files(root);

    // Duration should be measurable and not absurdly large
    assert!(result.duration.as_secs() < 30);
    // And not zero nanos (it should have taken some time)
    // Note: on very fast systems this could be zero, but duration should be representable
    let _ = result.duration.as_nanos();

    Ok(())
}

#[test]
fn discovery_method_is_walk_for_tempdir() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/Module.pm")?;

    let result = discover_perl_files(root);

    // Tempdir is not a git repo, so Walk should be used
    assert_eq!(result.method, DiscoveryMethod::Walk);

    Ok(())
}

#[test]
fn discovery_method_is_git_for_initialized_repo() -> TestResult {
    if !git_available() {
        return Ok(());
    }

    let tmp = TempDir::new()?;
    let root = tmp.path();

    run_git(root, &["init", "--quiet"])?;
    create_file(root, "lib/Module.pm")?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Git);

    Ok(())
}

// ============================================================
// Paths with special characters
// ============================================================

#[test]
fn paths_with_spaces_are_discovered() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "my project/lib/My Module.pm")?;
    create_file(root, "test dir/basic test.t")?;

    let result = discover_perl_files(root);

    assert_eq!(result.files.len(), 2);

    Ok(())
}

#[test]
fn paths_with_hyphens_and_underscores_are_discovered() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "my-project/lib/My_Module.pm")?;
    create_file(root, "test_suite/unit-tests/basic_test.t")?;

    let result = discover_perl_files(root);

    assert_eq!(result.files.len(), 2);

    Ok(())
}

// ============================================================
// Git discovery skipped-component filtering
// ============================================================

#[test]
fn git_discovery_still_applies_skip_rules_even_if_tracked() -> TestResult {
    if !git_available() {
        return Ok(());
    }

    let tmp = TempDir::new()?;
    let root = tmp.path();

    run_git(root, &["init", "--quiet"])?;

    // Force-add files in normally-skipped directories
    create_file(root, "lib/Kept.pm")?;
    create_file(root, "node_modules/Tracked.pm")?;
    run_git(root, &["add", "--force", "."])?;

    let result = discover_perl_files(root);

    assert_eq!(result.method, DiscoveryMethod::Git);
    // node_modules should be filtered even if git-tracked
    assert!(result.files.iter().any(|p| p.ends_with("Kept.pm")));
    assert!(
        !result.files.iter().any(|p| p.to_string_lossy().contains("node_modules")),
        "skipped directories should be filtered even when git-tracked"
    );

    Ok(())
}

// ============================================================
// Repeated discovery is stable
// ============================================================

#[test]
fn repeated_discovery_yields_same_result() -> TestResult {
    let tmp = TempDir::new()?;
    let root = tmp.path();

    create_file(root, "lib/A.pm")?;
    create_file(root, "lib/B.pm")?;
    create_file(root, "t/test.t")?;
    create_file(root, "README.md")?;

    let r1 = discover_perl_files(root);
    let r2 = discover_perl_files(root);

    let mut f1: Vec<_> = r1.files.clone();
    let mut f2: Vec<_> = r2.files.clone();
    f1.sort();
    f2.sort();

    assert_eq!(f1, f2);
    assert_eq!(r1.method, r2.method);
    assert_eq!(r1.excluded_count, r2.excluded_count);

    Ok(())
}
