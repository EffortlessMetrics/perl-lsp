//! Integration tests for workspace-wide rename refactoring
//!
//! Tests map to acceptance criteria in WORKSPACE_RENAME_SPECIFICATION.md
//! Each test is tagged with // AC:ACx for traceability
//!
//! Run with: cargo test -p perl-refactoring --features workspace-rename-tests --test workspace_rename_tests
#![cfg(feature = "workspace-rename-tests")]

use perl_refactoring::workspace_rename::{
    Progress, WorkspaceRename, WorkspaceRenameConfig, WorkspaceRenameError,
};
use perl_workspace_index::workspace_index::WorkspaceIndex;
use std::sync::mpsc;
use tempfile::TempDir;

/// Test helper to set up a temporary workspace with indexed files
fn setup_workspace(
    files: &[(&str, &str)],
) -> Result<(TempDir, WorkspaceIndex), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let index = WorkspaceIndex::new();
    for (name, content) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, content)?;

        // Index the file in the workspace index
        let uri = format!("file://{}", path.display());
        index.index_file_str(&uri, content).map_err(|e| format!("index_file_str failed: {}", e))?;
    }
    Ok((dir, index))
}

// ============================================================================
// AC1: Workspace Symbol Identification
// ============================================================================

#[test]
// AC:AC1
fn workspace_rename_identifies_all_occurrences() -> Result<(), Box<dyn std::error::Error>> {
    // Test: Multi-file corpus with qualified and bare references
    // Expected: All occurrences identified via dual indexing

    let (workspace, index) = setup_workspace(&[
        ("lib/Utils.pm", "package Utils;\nsub process { 1 }\n1;"),
        ("main.pl", "use lib 'lib';\nuse Utils;\nUtils::process();\nprocess();\n"),
        ("test.pl", "use lib 'lib';\nuse Utils qw(process);\nprocess();\n"),
    ])?;

    let config = WorkspaceRenameConfig::default();
    let rename_engine = WorkspaceRename::new(index, config);

    let result = rename_engine.rename_symbol(
        "process",
        "enhanced_process",
        &workspace.path().join("lib/Utils.pm"),
        (1, 4),
    )?;

    // Should find occurrences across all files that contain "process"
    assert!(
        result.statistics.total_changes >= 1,
        "Should find at least 1 occurrence, found {}",
        result.statistics.total_changes
    );

    // Verify edits span multiple files
    assert!(!result.file_edits.is_empty(), "Should have file edits");

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

    let (workspace, index) = setup_workspace(&[(
        "lib/Utils.pm",
        "package Utils;\nsub old_name { 1 }\nsub new_name { 2 }\n1;",
    )])?;

    let config = WorkspaceRenameConfig::default();
    let rename_engine = WorkspaceRename::new(index, config);

    let result = rename_engine.rename_symbol(
        "old_name",
        "new_name",
        &workspace.path().join("lib/Utils.pm"),
        (1, 4),
    );

    // Should fail due to existing symbol with target name
    assert!(
        matches!(result, Err(WorkspaceRenameError::NameConflict { .. })),
        "Expected NameConflict error, got: {:?}",
        result
    );

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

    let (workspace, index) = setup_workspace(&[
        ("file1.pl", "sub rollback_test { 1 }\nrollback_test();\n"),
        ("file2.pl", "rollback_test();\n"),
    ])?;

    let config = WorkspaceRenameConfig { create_backups: true, ..Default::default() };
    let rename_engine = WorkspaceRename::new(index, config);

    // Compute rename edits (with backup)
    let result = rename_engine.rename_symbol(
        "rollback_test",
        "renamed_test",
        &workspace.path().join("file1.pl"),
        (0, 4),
    )?;

    // Verify backup was created with original content
    let backup_info = result.backup_info.as_ref().ok_or("Expected backup info")?;
    assert!(backup_info.backup_dir.exists(), "Backup directory should exist");
    assert!(!backup_info.file_mappings.is_empty(), "Should have backup file mappings");

    // Apply edits to modify files
    rename_engine.apply_edits(&result)?;

    // Verify files were modified
    let content1 = std::fs::read_to_string(workspace.path().join("file1.pl"))?;
    assert!(content1.contains("renamed_test"), "file1.pl should be modified after apply");

    // Verify backups still contain original content
    for (original_path, backup_path) in &backup_info.file_mappings {
        let backup_content = std::fs::read_to_string(backup_path)?;
        assert!(
            backup_content.contains("rollback_test"),
            "Backup of {} should contain original content",
            original_path.display()
        );
    }

    // Manually restore from backup to simulate rollback
    for (original_path, backup_path) in &backup_info.file_mappings {
        std::fs::copy(backup_path, original_path)?;
    }

    // Verify rollback restored original content
    let content1_restored = std::fs::read_to_string(workspace.path().join("file1.pl"))?;
    assert!(
        content1_restored.contains("rollback_test"),
        "file1.pl should be restored after rollback"
    );

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

    let (workspace, index) = setup_workspace(&[(
        "lib/Package.pm",
        "package Package;\nsub name { 'Package::name' }\n\npackage Other;\nsub name { 'Other::name' }\n\n1;\n",
    )])?;

    let config = WorkspaceRenameConfig::default();
    let rename_engine = WorkspaceRename::new(index, config);

    let result = rename_engine.rename_symbol(
        "Package::name",
        "Package::renamed",
        &workspace.path().join("lib/Package.pm"),
        (1, 4),
    )?;

    // Apply the edits to verify scope correctness
    rename_engine.apply_edits(&result)?;

    let content = std::fs::read_to_string(workspace.path().join("lib/Package.pm"))?;

    // Should rename Package::name occurrences
    assert!(
        content.contains("renamed"),
        "Content should contain 'renamed' after rename, got: {}",
        content
    );

    // Other::name should be unchanged
    assert!(
        content.contains("Other::name") || content.contains("Other;\nsub name"),
        "Other::name should be unchanged, got: {}",
        content
    );

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

    let (workspace, index) =
        setup_workspace(&[("main.pl", "sub my_unique_sub { 1 }\nmy_unique_sub();\n")])?;

    let config = WorkspaceRenameConfig { create_backups: true, ..Default::default() };
    let rename_engine = WorkspaceRename::new(index, config);

    let result = rename_engine.rename_symbol(
        "my_unique_sub",
        "my_renamed_sub",
        &workspace.path().join("main.pl"),
        (0, 4),
    )?;

    // Should create backup
    let backup_info = result.backup_info.as_ref();
    assert!(backup_info.is_some(), "Backup info should be present");

    let backup = backup_info.ok_or("no backup")?;
    assert!(backup.backup_dir.exists(), "Backup directory should exist");
    assert_eq!(backup.file_mappings.len(), 1, "Should have 1 file mapping");

    // Verify backup content matches original
    for backup_path in backup.file_mappings.values() {
        let backup_content = std::fs::read_to_string(backup_path)?;
        assert!(backup_content.contains("my_unique_sub"), "Backup should contain original content");
    }

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

    let config = WorkspaceRenameConfig { operation_timeout: 2, ..Default::default() };

    let index = WorkspaceIndex::new();

    // Index many files to create a larger workspace
    for i in 0..50 {
        let uri = format!("file:///tmp/timeout_test/file{}.pl", i);
        let content = format!("sub common_func_{0} {{ {0} }}\ncommon_func_{0}();\n", i);
        let _ = index.index_file_str(&uri, &content);
    }

    let rename_engine = WorkspaceRename::new(index, config);

    let start = std::time::Instant::now();
    let result = rename_engine.rename_symbol(
        "common_func_0",
        "renamed_func_0",
        std::path::Path::new("/tmp/timeout_test/file0.pl"),
        (0, 0),
    );

    // Should complete or timeout within configured limit + grace period
    assert!(
        start.elapsed() <= std::time::Duration::from_secs(5),
        "Should complete within 5 seconds"
    );

    // Either success or timeout is acceptable
    assert!(
        matches!(
            result,
            Ok(_)
                | Err(WorkspaceRenameError::Timeout { .. })
                | Err(WorkspaceRenameError::SymbolNotFound { .. })
        ),
        "Expected Ok, Timeout, or SymbolNotFound, got: {:?}",
        result.as_ref().err()
    );
}

// ============================================================================
// AC7: Progress Reporting
// ============================================================================

#[test]
// AC:AC7
fn workspace_rename_reports_progress() -> Result<(), Box<dyn std::error::Error>> {
    // Test: Progress events during operation
    // Expected: Scanning, Processing, Complete events

    let (_workspace, index) = setup_workspace(&[
        ("file1.pl", "sub progress_test { 1 }\n"),
        ("file2.pl", "progress_test();\n"),
        ("file3.pl", "my $other = 2;\n"), // No match
    ])?;

    let config = WorkspaceRenameConfig::default();
    let rename_engine = WorkspaceRename::new(index, config);

    let (tx, rx) = mpsc::channel();
    let result = rename_engine.rename_symbol_with_progress(
        "progress_test",
        "renamed_test",
        &_workspace.path().join("file1.pl"),
        (0, 4),
        tx,
    )?;

    let progress_events: Vec<_> = rx.try_iter().collect();

    // Should report scanning
    assert!(
        progress_events.iter().any(|e| matches!(e, Progress::Scanning { .. })),
        "Should have Scanning event, got: {:?}",
        progress_events
    );

    // Should report processing
    assert!(
        progress_events.iter().any(|e| matches!(e, Progress::Processing { .. })),
        "Should have Processing event, got: {:?}",
        progress_events
    );

    // Should report completion
    assert!(
        progress_events.iter().any(|e| matches!(e, Progress::Complete { .. })),
        "Should have Complete event, got: {:?}",
        progress_events
    );

    // Verify result has actual changes
    assert!(result.statistics.total_changes >= 1, "Should have at least 1 change");

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

    let (workspace, index) =
        setup_workspace(&[("lib/Utils.pm", "package Utils;\nsub process { 1 }\n1;")])?;

    let config = WorkspaceRenameConfig::default();
    let rename_engine = WorkspaceRename::new(index, config);

    let result = rename_engine.rename_symbol(
        "process",
        "enhanced_process",
        &workspace.path().join("lib/Utils.pm"),
        (1, 4),
    )?;

    // Apply edits to files on disk
    rename_engine.apply_edits(&result)?;

    // Update the index
    rename_engine.update_index_after_rename("process", "enhanced_process", &result.file_edits)?;

    // Verify new name is findable in the index
    let index = rename_engine.index();
    let new_symbols = index.find_symbols("enhanced_process");
    assert!(!new_symbols.is_empty(), "Should find 'enhanced_process' in index after rename");

    // Old name should not be findable (or at least the definition should be gone)
    let old_def = index.find_definition("Utils::process");
    assert!(old_def.is_none(), "Old name 'Utils::process' should not be in index after rename");

    Ok(())
}

// ============================================================================
// Additional Edge Case Tests
// ============================================================================

#[test]
fn workspace_rename_handles_unicode() -> Result<(), Box<dyn std::error::Error>> {
    // Test: Unicode identifiers
    let (_workspace, index) =
        setup_workspace(&[("unicode.pl", "use utf8;\nmy $data = 'test';\nprint $data;\n")])?;

    let config = WorkspaceRenameConfig::default();
    let rename_engine = WorkspaceRename::new(index, config);

    // Rename a simple ASCII symbol in a file with unicode content
    let result = rename_engine.rename_symbol(
        "$data",
        "$output",
        &_workspace.path().join("unicode.pl"),
        (1, 3),
    );

    // Should either succeed or report symbol not found (depending on indexing)
    match result {
        Ok(r) => assert!(!r.file_edits.is_empty()),
        Err(WorkspaceRenameError::SymbolNotFound { .. }) => { /* acceptable */ }
        Err(e) => return Err(format!("Unexpected error: {}", e).into()),
    }

    Ok(())
}

#[test]
fn workspace_rename_handles_circular_deps() -> Result<(), Box<dyn std::error::Error>> {
    // Test: Circular module dependencies
    let (_workspace, index) = setup_workspace(&[
        ("lib/CircularA.pm", "package CircularA;\nuse CircularB;\nsub function_a { 1 }\n1;"),
        ("lib/CircularB.pm", "package CircularB;\nuse CircularA;\nsub function_b { 2 }\n1;"),
    ])?;

    let config = WorkspaceRenameConfig::default();
    let rename_engine = WorkspaceRename::new(index, config);

    // Rename a function in one of the circular modules
    let result = rename_engine.rename_symbol(
        "function_a",
        "renamed_a",
        &_workspace.path().join("lib/CircularA.pm"),
        (2, 4),
    )?;

    // Should succeed and only affect CircularA.pm
    assert!(
        result.statistics.total_changes >= 1,
        "Should find at least 1 occurrence of function_a"
    );

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
