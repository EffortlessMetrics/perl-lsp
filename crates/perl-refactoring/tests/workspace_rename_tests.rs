//! Integration tests for workspace-wide rename refactoring
//!
//! Tests map to acceptance criteria in WORKSPACE_RENAME_SPECIFICATION.md
//! Each test is tagged with // AC:ACx for traceability
//!
//! Run with: cargo test -p perl-refactoring --features workspace-rename-tests --test workspace_rename_tests
#![cfg(feature = "workspace-rename-tests")]

use perl_refactoring::workspace_rename::{WorkspaceRename, WorkspaceRenameConfig};
use tempfile::TempDir;

/// Test helper to set up a temporary workspace
fn setup_workspace(files: &[(&str, &str)]) -> Result<TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    for (name, content) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
    }
    Ok(dir)
}

// ============================================================================
// AC1: Workspace Symbol Identification
// ============================================================================

#[test]
// AC:AC1
fn workspace_rename_identifies_all_occurrences() -> Result<(), Box<dyn std::error::Error>> {
    // Test: Multi-file corpus with qualified and bare references
    // Expected: All occurrences identified via dual indexing
    //
    // Setup:
    // - lib/Utils.pm: sub process { 1 }
    // - main.pl: Utils::process(); process();
    // - test.pl: use Utils qw(process); process();
    //
    // Expected: 4 occurrences (1 definition + 3 references)

    let _workspace = setup_workspace(&[
        ("lib/Utils.pm", "package Utils;\nsub process { 1 }\n1;"),
        ("main.pl", "use lib 'lib';\nuse Utils;\nUtils::process();\nprocess();\n"),
        ("test.pl", "use lib 'lib';\nuse Utils qw(process);\nprocess();\n"),
    ])?;

    // TODO: Implement after WorkspaceIndex integration
    // let index = build_workspace_index(&workspace);
    // let rename_engine = WorkspaceRename::new(config);
    // let result = rename_engine.rename_symbol(...);
    // assert_eq!(result.statistics.total_changes, 4);
    Ok(())
}

// ============================================================================
// AC2: Name Conflict Validation
// ============================================================================

#[test]
// AC:AC2
fn workspace_rename_detects_conflicts() -> Result<(), Box<dyn std::error::Error>> {
    // Test: Rename to existing symbol name
    // Expected: NameConflict error
    //
    // Setup:
    // - lib/Utils.pm: sub old_name { 1 }; sub new_name { 2 };
    //
    // Expected: Error - new_name already exists

    let _workspace = setup_workspace(&[(
        "lib/Utils.pm",
        "package Utils;\nsub old_name { 1 }\nsub new_name { 2 }\n1;",
    )])?;

    // TODO: Implement after conflict detection
    // let result = rename_engine.rename_symbol("old_name", "new_name", ...);
    // assert!(matches!(result, Err(WorkspaceRenameError::NameConflict { .. })));
    Ok(())
}

// ============================================================================
// AC3: Atomic Multi-File Changes
// ============================================================================

#[test]
// AC:AC3
fn workspace_rename_atomic_rollback() -> Result<(), Box<dyn std::error::Error>> {
    // Test: Partial failure triggers complete rollback
    // Expected: All changes rolled back on any failure
    //
    // Setup:
    // - file1.pl: my $var = 1;
    // - file2.pl: my $var = 2;
    // - readonly.pl: my $var = 3; (read-only)
    //
    // Expected: Rollback all changes, original content preserved

    let _workspace = setup_workspace(&[
        ("file1.pl", "my $var = 1;\n"),
        ("file2.pl", "my $var = 2;\n"),
        ("readonly.pl", "my $var = 3;\n"),
    ])?;

    // TODO: Implement after transaction support
    // Make readonly.pl read-only
    // let result = rename_engine.rename_symbol("$var", "$renamed", ...);
    // assert!(result.is_err());
    // assert!(read_file("file1.pl").contains("$var")); // Unchanged
    Ok(())
}

// ============================================================================
// AC4: Perl Scoping Rules
// ============================================================================

#[test]
// AC:AC4
fn workspace_rename_respects_scoping() -> Result<(), Box<dyn std::error::Error>> {
    // Test: Package scope boundaries
    // Expected: Only symbols in matching scope are renamed
    //
    // Setup:
    // - lib/Package.pm:
    //   package Package; sub name { ... }
    //   package Other; sub name { ... }
    //
    // Expected: Only Package::name renamed, Other::name unchanged

    let _workspace = setup_workspace(&[(
        "lib/Package.pm",
        r#"
package Package;
sub name { 'Package::name' }

package Other;
sub name { 'Other::name' }

1;
"#,
    )])?;

    // TODO: Implement after scope analysis
    // let result = rename_engine.rename_symbol("Package::name", "Package::renamed", ...);
    // let content = read_file("lib/Package.pm");
    // assert!(content.contains("sub renamed"));
    // assert!(content.contains("Other::name")); // Unchanged
    Ok(())
}

// ============================================================================
// AC5: Backup Creation
// ============================================================================

#[test]
// AC:AC5
fn workspace_rename_creates_backups() -> Result<(), Box<dyn std::error::Error>> {
    // Test: Backup creation when enabled
    // Expected: Backup directory with original files
    //
    // Setup:
    // - main.pl: my $var = 1;
    // - config.create_backups = true
    //
    // Expected: Backup created with operation ID, contains original content

    let _workspace = setup_workspace(&[("main.pl", "my $var = 1;\n")])?;

    let config = WorkspaceRenameConfig { create_backups: true, ..Default::default() };

    let _rename_engine = WorkspaceRename::new(config);

    // TODO: Implement after backup support
    // let result = rename_engine.rename_symbol("$var", "$renamed", ...);
    // let backup_info = result.backup_info.expect("backup created");
    // assert!(backup_info.backup_dir.exists());
    Ok(())
}

// ============================================================================
// AC6: Operation Timeout
// ============================================================================

#[test]
// AC:AC6
fn workspace_rename_respects_timeout() {
    // Test: Timeout enforcement for large workspaces
    // Expected: Operation completes or times out within limit
    //
    // Setup:
    // - 100+ files with common symbol
    // - config.operation_timeout = 2 seconds
    //
    // Expected: Completes within 3 seconds (timeout + grace)

    let config = WorkspaceRenameConfig { operation_timeout: 2, ..Default::default() };

    let _rename_engine = WorkspaceRename::new(config);

    // TODO: Implement after timeout support
    // Create large workspace
    // Measure elapsed time
    // Assert completion or timeout within limit
}

// ============================================================================
// AC7: Progress Reporting
// ============================================================================

#[test]
// AC:AC7
fn workspace_rename_reports_progress() -> Result<(), Box<dyn std::error::Error>> {
    // Test: Progress events during operation
    // Expected: Scanning, Processing, Complete events
    //
    // Setup:
    // - 3 files (2 with matches, 1 without)
    //
    // Expected: Progress events with accurate counts

    let _workspace = setup_workspace(&[
        ("file1.pl", "my $var = 1;\n"),
        ("file2.pl", "print $var;\n"),
        ("file3.pl", "my $other = 2;\n"), // No match
    ])?;

    // TODO: Implement after progress support
    // let (tx, rx) = mpsc::channel();
    // let result = rename_engine.rename_symbol_with_progress(..., tx);
    // let events: Vec<_> = rx.iter().collect();
    // assert!(events.iter().any(|e| matches!(e, Progress::Scanning { .. })));
    Ok(())
}

// ============================================================================
// AC8: Dual Indexing Update
// ============================================================================

#[test]
// AC:AC8
fn workspace_rename_updates_dual_index() -> Result<(), Box<dyn std::error::Error>> {
    // Test: Index updated with qualified and bare forms
    // Expected: Both forms findable post-rename, old forms removed
    //
    // Setup:
    // - lib/Utils.pm: sub process { 1 }
    // - Rename to enhanced_process
    //
    // Expected:
    // - Utils::enhanced_process findable
    // - enhanced_process findable
    // - Utils::process not findable
    // - process not findable

    let _workspace = setup_workspace(&[("lib/Utils.pm", "package Utils;\nsub process { 1 }\n1;")])?;

    // TODO: Implement after index integration
    // let result = rename_engine.rename_symbol("process", "enhanced_process", ...);
    // result.apply();
    // let index = workspace.index();
    // assert!(index.find_definition("Utils::enhanced_process").is_some());
    // assert!(index.find_definition("enhanced_process").is_some());
    // assert!(index.find_definition("Utils::process").is_none());
    Ok(())
}

// ============================================================================
// Additional Edge Case Tests
// ============================================================================

#[test]
fn workspace_rename_handles_unicode() -> Result<(), Box<dyn std::error::Error>> {
    // Test: Unicode identifiers
    let _workspace =
        setup_workspace(&[("unicode.pl", "use utf8;\nmy $数据 = '测试';\nprint $数据;\n")])?;

    // TODO: Test unicode identifier rename
    Ok(())
}

#[test]
fn workspace_rename_handles_circular_deps() -> Result<(), Box<dyn std::error::Error>> {
    // Test: Circular module dependencies
    let _workspace = setup_workspace(&[
        ("lib/CircularA.pm", "package CircularA;\nuse CircularB;\nsub function_a { 1 }\n1;"),
        ("lib/CircularB.pm", "package CircularB;\nuse CircularA;\nsub function_b { 2 }\n1;"),
    ])?;

    // TODO: Test rename in circular dependency graph
    Ok(())
}

#[test]
fn workspace_rename_config_defaults() {
    let config = WorkspaceRenameConfig::default();

    assert!(config.atomic_mode, "atomic_mode should default to true");
    assert!(config.create_backups, "create_backups should default to true");
    assert_eq!(config.operation_timeout, 60, "timeout should default to 60s");
    assert!(config.parallel_processing, "parallel_processing should default to true");
    assert_eq!(config.batch_size, 10, "batch_size should default to 10");
    assert_eq!(config.max_files, 0, "max_files should default to 0");
    assert!(config.report_progress, "report_progress should default to true");
    assert!(config.validate_syntax, "validate_syntax should default to true");
    assert!(!config.follow_symlinks, "follow_symlinks should default to false");
}
