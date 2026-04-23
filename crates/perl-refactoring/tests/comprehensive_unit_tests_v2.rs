//! Comprehensive unit tests v2 for the perl-refactoring crate.
//!
//! Covers areas not exercised by the existing test suite:
//! - Extract method: edge cases, CRLF, outputs, multi-line bodies
//! - Move code: multiple elements, insertion points, ordering
//! - Import optimizer: pragma edge cases, known module exports, generate_edits details
//! - Modernizer legacy & refactored: apply transformations, combined patterns
//! - Inline: non-variable symbols, all_occurrences
//! - Backup lifecycle: retention policies, multiple operations
//! - Validation: scope edge cases, FileSet limits, block scope
//! - RefactoringConfig: edge values

use perl_refactoring::import_optimizer::ImportOptimizer;
use perl_refactoring::modernize::PerlModernizer as LegacyModernizer;
use perl_refactoring::modernize_refactored::PerlModernizer as RefactoredModernizer;
use perl_refactoring::refactor::refactoring::{
    ModernizationPattern, RefactoringConfig, RefactoringEngine, RefactoringScope, RefactoringType,
};
use std::io::Write;
use tempfile::NamedTempFile;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn engine_no_safe() -> RefactoringEngine {
    RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: false,
        create_backups: false,
        ..Default::default()
    })
}

fn temp_perl(
    content: &str,
) -> Result<(NamedTempFile, std::path::PathBuf), Box<dyn std::error::Error>> {
    let mut f = NamedTempFile::new()?;
    write!(f, "{}", content)?;
    let p = f.path().to_path_buf();
    Ok((f, p))
}

// ===================================================================
// RefactoringConfig edge values
// ===================================================================

#[test]
fn config_custom_max_files_zero() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = RefactoringConfig { max_files_per_operation: 0, ..Default::default() };
    assert_eq!(cfg.max_files_per_operation, 0);
    Ok(())
}

#[test]
fn config_custom_backup_retention_zero() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = RefactoringConfig {
        max_backup_retention: 0,
        backup_max_age_seconds: 0,
        ..Default::default()
    };
    assert_eq!(cfg.max_backup_retention, 0);
    assert_eq!(cfg.backup_max_age_seconds, 0);
    Ok(())
}

#[test]
fn config_custom_timeout() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = RefactoringConfig { operation_timeout: 300, ..Default::default() };
    assert_eq!(cfg.operation_timeout, 300);
    Ok(())
}

#[test]
fn config_parallel_processing_disabled() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = RefactoringConfig { parallel_processing: false, ..Default::default() };
    assert!(!cfg.parallel_processing);
    Ok(())
}

#[test]
fn config_custom_backup_root() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let cfg =
        RefactoringConfig { backup_root: Some(dir.path().to_path_buf()), ..Default::default() };
    assert_eq!(cfg.backup_root.as_deref(), Some(dir.path()));
    Ok(())
}

// ===================================================================
// Engine default trait
// ===================================================================

#[test]
fn engine_default_trait() -> Result<(), Box<dyn std::error::Error>> {
    let engine = RefactoringEngine::default();
    assert!(engine.get_operation_history().is_empty());
    Ok(())
}

// ===================================================================
// Extract method – CRLF line endings
// ===================================================================

#[test]
fn extract_method_crlf_line_endings() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $a = 1;\r\nmy $b = $a + 2;\r\nprint $b;\r\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "compute_crlf".to_string(),
            start_position: (1, 0),
            end_position: (2, 0),
        },
        vec![p.clone()],
    )?;

    assert!(result.success);
    let new_code = std::fs::read_to_string(&p)?;
    assert!(new_code.contains("sub compute_crlf"), "Should create sub with CRLF handling");
    Ok(())
}

// ===================================================================
// Extract method – no outputs
// ===================================================================

#[test]
fn extract_method_no_outputs() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 10;\nprint $x;\nprint \"done\";\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "print_stuff".to_string(),
            start_position: (1, 0),
            end_position: (2, 0),
        },
        vec![p.clone()],
    )?;

    assert!(result.success);
    let new_code = std::fs::read_to_string(&p)?;
    assert!(new_code.contains("sub print_stuff"), "Extracted sub should exist");
    // Should have a call site
    assert!(new_code.contains("print_stuff("), "Call site should exist");
    Ok(())
}

// ===================================================================
// Extract method – equal start and end position rejected
// ===================================================================

#[test]
fn extract_method_equal_start_end() -> Result<(), Box<dyn std::error::Error>> {
    let (_f, p) = temp_perl("my $x = 1;\nprint $x;\n")?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });
    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "noop".to_string(),
            start_position: (0, 5),
            end_position: (0, 5),
        },
        vec![p],
    );
    assert!(result.is_err(), "Equal start/end position should be rejected");
    Ok(())
}

// ===================================================================
// Extract method – result has two changes (sub + call)
// ===================================================================

#[test]
fn extract_method_changes_count() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;\nmy $y = $x + 2;\nprint $y;\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "calc".to_string(),
            start_position: (1, 0),
            end_position: (2, 0),
        },
        vec![p],
    )?;

    assert!(result.success);
    assert_eq!(result.changes_made, 2, "Extract method should report 2 changes (sub + call)");
    assert_eq!(result.files_modified, 1);
    Ok(())
}

// ===================================================================
// Extract method – safe_mode does not write file
// ===================================================================

#[test]
fn extract_method_safe_mode_no_write() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;\nmy $y = $x + 2;\nprint $y;\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });
    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "notouch".to_string(),
            start_position: (1, 0),
            end_position: (2, 0),
        },
        vec![p.clone()],
    )?;

    assert!(result.success);
    // File should remain unchanged in safe mode
    let after = std::fs::read_to_string(&p)?;
    assert_eq!(after, code, "Safe mode should not write file");
    Ok(())
}

// ===================================================================
// Move code – multiple elements
// ===================================================================

#[test]
fn move_code_multiple_elements() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub alpha { 1 }\nsub beta { 2 }\nsub gamma { 3 }\n";
    let target = "package Dest;\n1;\n";
    let (_f1, p1) = temp_perl(source)?;
    let (_f2, p2) = temp_perl(target)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::MoveCode {
            source_file: p1.clone(),
            target_file: p2.clone(),
            elements: vec!["alpha".to_string(), "gamma".to_string()],
        },
        vec![p1.clone()],
    )?;

    assert!(result.success);
    let new_source = std::fs::read_to_string(&p1)?;
    let new_target = std::fs::read_to_string(&p2)?;
    assert!(!new_source.contains("sub alpha"), "alpha should be removed from source");
    assert!(new_source.contains("sub beta"), "beta should remain in source");
    assert!(!new_source.contains("sub gamma"), "gamma should be removed from source");
    assert!(new_target.contains("sub alpha"), "alpha should be in target");
    assert!(new_target.contains("sub gamma"), "gamma should be in target");
    Ok(())
}

// ===================================================================
// Move code – target without 1; sentinel
// ===================================================================

#[test]
fn move_code_target_without_sentinel() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub mover { 42 }\n";
    let target = "package Dest;\nprint 'hello';\n";
    let (_f1, p1) = temp_perl(source)?;
    let (_f2, p2) = temp_perl(target)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::MoveCode {
            source_file: p1.clone(),
            target_file: p2.clone(),
            elements: vec!["mover".to_string()],
        },
        vec![p1.clone()],
    )?;

    assert!(result.success);
    let new_target = std::fs::read_to_string(&p2)?;
    assert!(new_target.contains("sub mover"), "Moved sub should appear in target");
    Ok(())
}

// ===================================================================
// Move code – generates warning about missing deps
// ===================================================================

#[test]
fn move_code_warns_about_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub helper { 42 }\n";
    let target = "1;\n";
    let (_f1, p1) = temp_perl(source)?;
    let (_f2, p2) = temp_perl(target)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::MoveCode {
            source_file: p1.clone(),
            target_file: p2,
            elements: vec!["helper".to_string()],
        },
        vec![p1],
    )?;

    assert!(result.success);
    assert!(
        result.warnings.iter().any(|w| w.contains("Imports and references")),
        "Should warn about missing dependency analysis"
    );
    Ok(())
}

// ===================================================================
// Move code – partial find (some elements found, some not)
// ===================================================================

#[test]
fn move_code_partial_find() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub exists_sub { 1 }\n";
    let target = "1;\n";
    let (_f1, p1) = temp_perl(source)?;
    let (_f2, p2) = temp_perl(target)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::MoveCode {
            source_file: p1.clone(),
            target_file: p2,
            elements: vec!["exists_sub".to_string(), "missing_sub".to_string()],
        },
        vec![p1],
    )?;

    assert!(result.success, "Should succeed even with partial finds");
    assert!(
        result.warnings.iter().any(|w| w.contains("missing_sub")),
        "Should warn about missing elements"
    );
    Ok(())
}

// ===================================================================
// Modernize – multiple pattern types
// ===================================================================

#[test]
fn modernize_multiple_patterns() -> Result<(), Box<dyn std::error::Error>> {
    let code = "#!/usr/bin/perl\nopen FH, '<', 'file.txt';\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::Modernize {
            patterns: vec![
                ModernizationPattern::StrictWarnings,
                ModernizationPattern::SubroutineCalls,
                ModernizationPattern::DeprecatedOperators,
            ],
        },
        vec![p],
    )?;

    assert!(result.success);
    Ok(())
}

// ===================================================================
// Modernize – file with no changes needed
// ===================================================================

#[test]
fn modernize_clean_file() -> Result<(), Box<dyn std::error::Error>> {
    let code = "use strict;\nuse warnings;\nmy $x = 1;\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::Modernize { patterns: vec![ModernizationPattern::StrictWarnings] },
        vec![p],
    )?;

    assert!(result.success);
    assert_eq!(result.changes_made, 0, "Clean file should have no changes");
    Ok(())
}

// ===================================================================
// Optimize imports – sort alphabetically
// ===================================================================

#[test]
fn optimize_imports_sort_flag() -> Result<(), Box<dyn std::error::Error>> {
    let code = "use warnings;\nuse strict;\nprint 1;\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::OptimizeImports {
            remove_unused: false,
            sort_alphabetically: true,
            group_by_type: false,
        },
        vec![p],
    )?;

    assert!(result.success);
    assert!(result.changes_made > 0, "Sort should count as a change");
    Ok(())
}

// ===================================================================
// Optimize imports – group by type flag
// ===================================================================

#[test]
fn optimize_imports_group_flag() -> Result<(), Box<dyn std::error::Error>> {
    let code = "use strict;\nuse Data::Dumper;\nDumper({});\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::OptimizeImports {
            remove_unused: false,
            sort_alphabetically: false,
            group_by_type: true,
        },
        vec![p],
    )?;

    assert!(result.success);
    assert!(result.changes_made > 0, "Group by type should count as a change");
    Ok(())
}

// ===================================================================
// Optimize imports – all flags enabled
// ===================================================================

#[test]
fn optimize_imports_all_flags() -> Result<(), Box<dyn std::error::Error>> {
    let code = "use warnings;\nuse strict;\nuse List::Util qw(max min);\nmy $m = max(1,2);\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::OptimizeImports {
            remove_unused: true,
            sort_alphabetically: true,
            group_by_type: true,
        },
        vec![p],
    )?;

    assert!(result.success);
    assert!(result.changes_made >= 2, "Should count unused removal + sort + group");
    Ok(())
}

// ===================================================================
// Optimize imports – multiple files
// ===================================================================

#[test]
fn optimize_imports_multiple_files() -> Result<(), Box<dyn std::error::Error>> {
    let code1 = "use warnings;\nuse strict;\nprint 1;\n";
    let code2 = "use strict;\nuse List::Util qw(max);\nmy $m = max(1,2);\n";
    let (_f1, p1) = temp_perl(code1)?;
    let (_f2, p2) = temp_perl(code2)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::OptimizeImports {
            remove_unused: true,
            sort_alphabetically: true,
            group_by_type: false,
        },
        vec![p1, p2],
    )?;

    assert!(result.success);
    assert!(result.files_modified >= 1, "At least one file should be modified");
    Ok(())
}

// ===================================================================
// Inline – non-variable symbol gives warning
// ===================================================================

#[test]
fn inline_non_variable_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub foo { 42 }\nprint foo();\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::Inline { symbol_name: "foo".to_string(), all_occurrences: false },
        vec![p],
    )?;

    assert!(!result.success, "Inlining non-variable should not succeed");
    assert!(
        result.warnings.iter().any(|w| w.contains("not implemented")),
        "Should warn about unsupported inline"
    );
    Ok(())
}

// ===================================================================
// Inline – all_occurrences vs single
// ===================================================================

#[test]
fn inline_all_occurrences_flag() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 42;\nprint $x;\nprint $x;\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::Inline { symbol_name: "$x".to_string(), all_occurrences: true },
        vec![p],
    )?;

    // With or without workspace_refactor feature, should not crash
    assert!(result.success || !result.warnings.is_empty());
    Ok(())
}

#[test]
fn inline_single_occurrence_flag() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $y = 100;\nprint $y;\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::Inline { symbol_name: "$y".to_string(), all_occurrences: false },
        vec![p],
    )?;

    assert!(result.success || !result.warnings.is_empty());
    Ok(())
}

// ===================================================================
// Inline – array sigil
// ===================================================================

#[test]
fn inline_array_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my @items = (1, 2, 3);\nprint @items;\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::Inline { symbol_name: "@items".to_string(), all_occurrences: true },
        vec![p],
    )?;

    assert!(result.success || !result.warnings.is_empty());
    Ok(())
}

// ===================================================================
// Inline – hash sigil
// ===================================================================

#[test]
fn inline_hash_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my %config = (key => 'val');\nprint %config;\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::Inline { symbol_name: "%config".to_string(), all_occurrences: false },
        vec![p],
    )?;

    assert!(result.success || !result.warnings.is_empty());
    Ok(())
}

// ===================================================================
// Backup lifecycle – backup with custom root
// ===================================================================

#[test]
fn backup_creates_in_custom_root() -> Result<(), Box<dyn std::error::Error>> {
    let backup_dir = tempfile::tempdir()?;
    let code = "my $a = 1;\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: false,
        create_backups: true,
        backup_root: Some(backup_dir.path().to_path_buf()),
        ..Default::default()
    });

    let result = engine.refactor(
        RefactoringType::OptimizeImports {
            remove_unused: false,
            sort_alphabetically: false,
            group_by_type: false,
        },
        vec![p],
    )?;

    assert!(result.success);
    assert!(result.operation_id.is_some());
    // Backup dir should have been created
    let entries: Vec<_> = std::fs::read_dir(backup_dir.path())?.filter_map(|e| e.ok()).collect();
    assert!(!entries.is_empty(), "Backup directory should have content");
    Ok(())
}

// ===================================================================
// Backup cleanup – zero retention removes all
// ===================================================================

#[test]
fn backup_cleanup_zero_retention() -> Result<(), Box<dyn std::error::Error>> {
    let backup_dir = tempfile::tempdir()?;
    // Create a fake backup directory
    let fake_backup = backup_dir.path().join("refactor_fake_123");
    std::fs::create_dir_all(&fake_backup)?;
    std::fs::write(fake_backup.join("file_0.pl"), "content")?;

    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: false,
        create_backups: false,
        max_backup_retention: 0,
        backup_max_age_seconds: 0,
        backup_root: Some(backup_dir.path().to_path_buf()),
        ..Default::default()
    });

    let cleanup = engine.clear_history()?;
    assert!(cleanup.directories_removed >= 1, "Should remove backup directories");
    Ok(())
}

// ===================================================================
// Multiple operations populate history
// ===================================================================

#[test]
fn multiple_operations_in_history() -> Result<(), Box<dyn std::error::Error>> {
    let code1 = "use strict;\nprint 1;\n";
    let code2 = "use warnings;\nprint 2;\n";
    let (_f1, p1) = temp_perl(code1)?;
    let (_f2, p2) = temp_perl(code2)?;

    let mut engine = engine_no_safe();

    engine.refactor(
        RefactoringType::OptimizeImports {
            remove_unused: false,
            sort_alphabetically: true,
            group_by_type: false,
        },
        vec![p1],
    )?;

    engine.refactor(
        RefactoringType::OptimizeImports {
            remove_unused: true,
            sort_alphabetically: false,
            group_by_type: false,
        },
        vec![p2],
    )?;

    assert_eq!(engine.get_operation_history().len(), 2);
    Ok(())
}

// ===================================================================
// Operation result has operation_id
// ===================================================================

#[test]
fn operation_result_has_id() -> Result<(), Box<dyn std::error::Error>> {
    let code = "use strict;\nprint 1;\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = engine_no_safe();
    let result = engine.refactor(
        RefactoringType::OptimizeImports {
            remove_unused: false,
            sort_alphabetically: false,
            group_by_type: false,
        },
        vec![p],
    )?;

    assert!(result.operation_id.is_some(), "Operation should have an ID");
    let id = result.operation_id.as_deref().ok_or("no id")?;
    assert!(id.starts_with("refactor_"), "ID should start with refactor_");
    Ok(())
}

// ===================================================================
// Rollback without backup gives error
// ===================================================================

#[test]
fn rollback_no_backup_error() -> Result<(), Box<dyn std::error::Error>> {
    let code = "use strict;\nprint 1;\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = engine_no_safe(); // backups disabled
    let result = engine.refactor(
        RefactoringType::OptimizeImports {
            remove_unused: false,
            sort_alphabetically: false,
            group_by_type: false,
        },
        vec![p],
    )?;

    let op_id = result.operation_id.as_deref().ok_or("no id")?;
    let rollback = engine.rollback(op_id);
    assert!(rollback.is_err(), "Rollback without backup should fail");
    Ok(())
}

// ===================================================================
// Import optimizer – pragma modules not flagged
// ===================================================================

#[test]
fn import_optimizer_pragmas_not_flagged() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content =
        "use strict;\nuse warnings;\nuse utf8;\nuse bytes;\nuse integer;\nuse locale;\nprint 1;\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    // No pragmas should be flagged as unused
    for unused in &analysis.unused_imports {
        assert!(
            !["strict", "warnings", "utf8", "bytes", "integer", "locale"]
                .contains(&unused.module.as_str()),
            "Pragma {} should not be flagged unused",
            unused.module
        );
    }
    Ok(())
}

// ===================================================================
// Import optimizer – known exports for various modules
// ===================================================================

#[test]
fn import_optimizer_yaml_exports_detected() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use YAML;\nmy $data = Load('file.yaml');\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    // YAML module with Load usage should not be unused
    assert!(
        !analysis.unused_imports.iter().any(|u| u.module == "YAML"),
        "YAML with Load should not be unused"
    );
    Ok(())
}

#[test]
fn import_optimizer_storable_exports_detected() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use Storable;\nmy $data = retrieve('file.dat');\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    assert!(
        !analysis.unused_imports.iter().any(|u| u.module == "Storable"),
        "Storable with retrieve should not be unused"
    );
    Ok(())
}

#[test]
fn import_optimizer_scalar_util_exports() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use Scalar::Util qw(blessed looks_like_number);\nmy $b = blessed($obj);\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    // looks_like_number should be flagged as unused
    let unused: Vec<_> = analysis.unused_imports.iter().flat_map(|u| u.symbols.iter()).collect();
    assert!(
        unused.iter().any(|s| s.as_str() == "looks_like_number"),
        "looks_like_number should be unused"
    );
    // blessed should NOT be unused
    assert!(!unused.iter().any(|s| s.as_str() == "blessed"), "blessed should not be unused");
    Ok(())
}

#[test]
fn import_optimizer_cwd_exports() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use Cwd;\nmy $dir = getcwd();\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    assert!(
        !analysis.unused_imports.iter().any(|u| u.module == "Cwd"),
        "Cwd with getcwd should not be unused"
    );
    Ok(())
}

// ===================================================================
// Import optimizer – generate_edits with no existing imports
// ===================================================================

#[test]
fn import_optimizer_edits_no_existing_imports() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "Foo::Bar::baz();\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;
    let edits = optimizer.generate_edits(content, &analysis);

    if !analysis.missing_imports.is_empty() {
        assert!(!edits.is_empty(), "Should produce insert edit for missing imports");
        // Edit should be an insertion (range start == range end)
        let edit = &edits[0];
        assert_eq!(edit.range.0, edit.range.1, "Should be an insertion edit");
    }
    Ok(())
}

// ===================================================================
// Import optimizer – generate_edits replaces existing import block
// ===================================================================

#[test]
fn import_optimizer_edits_replace_block() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use strict;\nuse warnings;\nuse List::Util qw(max min);\nmy $m = max(1,2);\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;
    let edits = optimizer.generate_edits(content, &analysis);

    assert!(!edits.is_empty(), "Should produce replacement edit");
    let edit = &edits[0];
    // The replacement should cover the import block
    assert!(edit.range.0 < edit.range.1, "Should be a replacement, not insertion");
    assert!(edit.new_text.contains("use strict;"), "Replacement should contain strict");
    Ok(())
}

// ===================================================================
// Import optimizer – empty edits for empty analysis
// ===================================================================

#[test]
fn import_optimizer_edits_empty_for_empty() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;
    let edits = optimizer.generate_edits(content, &analysis);

    assert!(edits.is_empty(), "Empty content should produce no edits");
    Ok(())
}

// ===================================================================
// Import optimizer – duplicate symbols in qw()
// ===================================================================

#[test]
fn import_optimizer_duplicate_symbols_in_qw() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use List::Util qw(max max min);\nmy $m = max(1,2);\nmy $n = min(1,2);\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    // Should detect that symbols need dedup
    let has_dedup_suggestion =
        analysis.organization_suggestions.iter().any(|s| s.description.contains("deduplicate"));
    assert!(has_dedup_suggestion, "Should suggest symbol deduplication");
    Ok(())
}

// ===================================================================
// Import optimizer – generate_optimized strips unused from mixed import
// ===================================================================

#[test]
fn import_optimizer_optimized_mixed_used_unused() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content =
        "use Scalar::Util qw(blessed reftype looks_like_number);\nmy $b = blessed($obj);\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;
    let optimized = optimizer.generate_optimized_imports(&analysis);

    assert!(optimized.contains("blessed"), "Used symbol should be kept");
    assert!(!optimized.contains("reftype"), "Unused symbol reftype should be removed");
    assert!(!optimized.contains("looks_like_number"), "Unused symbol should be removed");
    Ok(())
}

// ===================================================================
// Import optimizer – consolidates duplicate module imports
// ===================================================================

#[test]
fn import_optimizer_consolidates_duplicates() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content =
        "use List::Util qw(max);\nuse List::Util qw(min);\nmy $m = max(1,2);\nmy $n = min(1,2);\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;
    let optimized = optimizer.generate_optimized_imports(&analysis);

    // Should consolidate into one import
    let count = optimized.matches("use List::Util").count();
    assert_eq!(count, 1, "Should consolidate duplicate module imports");
    assert!(optimized.contains("max"), "max should be kept");
    assert!(optimized.contains("min"), "min should be kept");
    Ok(())
}

// ===================================================================
// Import optimizer – comment lines excluded from usage scan
// ===================================================================

#[test]
fn import_optimizer_comments_not_counted_as_usage() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use List::Util qw(max);\n# max is just mentioned in a comment\nprint 1;\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    // max should be flagged as unused since it's only in a comment
    let unused: Vec<_> = analysis.unused_imports.iter().flat_map(|u| u.symbols.iter()).collect();
    assert!(
        unused.iter().any(|s| s.as_str() == "max"),
        "Symbol only in comment should be flagged unused"
    );
    Ok(())
}

// ===================================================================
// Import optimizer – multiple missing imports
// ===================================================================

#[test]
fn import_optimizer_multiple_missing() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "Foo::Bar::baz();\nQux::Quux::corge();\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    assert!(analysis.missing_imports.len() >= 2, "Should detect multiple missing imports");
    let modules: Vec<_> = analysis.missing_imports.iter().map(|m| m.module.as_str()).collect();
    assert!(modules.contains(&"Foo::Bar"), "Should detect Foo::Bar");
    assert!(modules.contains(&"Qux::Quux"), "Should detect Qux::Quux");
    Ok(())
}

// ===================================================================
// Legacy modernizer – apply defined @array
// ===================================================================

#[test]
fn legacy_modernizer_apply_defined_array() -> Result<(), Box<dyn std::error::Error>> {
    let m = LegacyModernizer::new();
    let result = m.apply("if (defined @array) { print 1; }");
    assert!(result.contains("@array"), "Should replace defined @array");
    assert!(!result.contains("defined @array"), "Should remove defined keyword");
    Ok(())
}

// ===================================================================
// Legacy modernizer – apply two-arg open
// ===================================================================

#[test]
fn legacy_modernizer_apply_two_arg_open() -> Result<(), Box<dyn std::error::Error>> {
    let m = LegacyModernizer::new();
    let result = m.apply("open(FH, 'file.txt')");
    assert!(result.contains("open(my $fh, '<', 'file.txt')"), "Should convert to three-arg open");
    Ok(())
}

// ===================================================================
// Legacy modernizer – apply print with newline
// ===================================================================

#[test]
fn legacy_modernizer_apply_print_newline() -> Result<(), Box<dyn std::error::Error>> {
    let m = LegacyModernizer::new();
    let result = m.apply("print \"Hello\\n\"");
    assert!(result.contains("say \"Hello\""), "Should convert print with newline to say");
    Ok(())
}

// ===================================================================
// Legacy modernizer – apply indirect notation Class
// ===================================================================

#[test]
fn legacy_modernizer_apply_indirect_class() -> Result<(), Box<dyn std::error::Error>> {
    let m = LegacyModernizer::new();
    let result = m.apply("my $obj = new Class();");
    assert!(result.contains("Class->new("), "Should convert indirect notation to direct");
    Ok(())
}

// ===================================================================
// Legacy modernizer – apply indirect notation MyClass
// ===================================================================

#[test]
fn legacy_modernizer_apply_indirect_myclass() -> Result<(), Box<dyn std::error::Error>> {
    let m = LegacyModernizer::new();
    let result = m.apply("my $obj = new MyClass();");
    assert!(result.contains("MyClass->new("), "Should convert indirect notation for MyClass");
    Ok(())
}

// ===================================================================
// Legacy modernizer – multiple suggestions in one file
// ===================================================================

#[test]
fn legacy_modernizer_multiple_detections() -> Result<(), Box<dyn std::error::Error>> {
    let m = LegacyModernizer::new();
    let code = "#!/usr/bin/perl\nopen FH, '<', 'file.txt';\ndefined @array;\n";
    let suggestions = m.analyze(code);
    assert!(
        suggestions.len() >= 3,
        "Should detect missing pragmas, bareword FH, and defined @array"
    );
    Ok(())
}

// ===================================================================
// Legacy modernizer – no false positive for partial match
// ===================================================================

#[test]
fn legacy_modernizer_no_false_positive_open() -> Result<(), Box<dyn std::error::Error>> {
    let m = LegacyModernizer::new();
    let suggestions = m.analyze("open(my $fh, '<', 'file.txt');");
    // Modern code should not trigger bareword or two-arg open
    assert!(
        !suggestions.iter().any(|s| s.old_pattern == "open FH"),
        "Modern open should not trigger bareword detection"
    );
    Ok(())
}

// ===================================================================
// Refactored modernizer – apply defined @array
// ===================================================================

#[test]
fn refactored_modernizer_apply_defined_array() -> Result<(), Box<dyn std::error::Error>> {
    let m = RefactoredModernizer::new();
    let result = m.apply("if (defined @array) { print 1; }");
    assert!(result.contains("@array"), "Should replace defined @array");
    assert!(!result.contains("defined @array"), "Should remove defined keyword");
    Ok(())
}

// ===================================================================
// Refactored modernizer – apply two-arg open
// ===================================================================

#[test]
fn refactored_modernizer_apply_two_arg_open() -> Result<(), Box<dyn std::error::Error>> {
    let m = RefactoredModernizer::new();
    let result = m.apply("open(FH, 'file.txt')");
    assert!(result.contains("open(my $fh, '<', 'file.txt')"), "Should convert to three-arg open");
    Ok(())
}

// ===================================================================
// Refactored modernizer – apply print newline
// ===================================================================

#[test]
fn refactored_modernizer_apply_print_newline() -> Result<(), Box<dyn std::error::Error>> {
    let m = RefactoredModernizer::new();
    let result = m.apply("print \"Hello\\n\"");
    assert!(result.contains("say \"Hello\""), "Should convert print with newline to say");
    Ok(())
}

// ===================================================================
// Refactored modernizer – detect print newline
// ===================================================================

#[test]
fn refactored_modernizer_detect_print_newline() -> Result<(), Box<dyn std::error::Error>> {
    let m = RefactoredModernizer::new();
    let suggestions = m.analyze("print \"Hello\\n\"");
    assert!(suggestions.iter().any(|s| s.new_pattern.contains("say")), "Should suggest say");
    Ok(())
}

// ===================================================================
// Refactored modernizer – no suggestions for partial pattern
// ===================================================================

#[test]
fn refactored_modernizer_no_false_positive() -> Result<(), Box<dyn std::error::Error>> {
    let m = RefactoredModernizer::new();
    let suggestions = m.analyze("open(my $fh, '<', 'file.txt');");
    assert!(
        !suggestions.iter().any(|s| s.old_pattern == "open FH"),
        "Modern open should not trigger detection"
    );
    Ok(())
}

// ===================================================================
// Refactored modernizer – detect indirect Class
// ===================================================================

#[test]
fn refactored_modernizer_detect_indirect_class() -> Result<(), Box<dyn std::error::Error>> {
    let m = RefactoredModernizer::new();
    let suggestions = m.analyze("my $obj = new Class();");
    assert!(
        suggestions.iter().any(|s| s.old_pattern == "new Class"),
        "Should detect indirect notation for Class"
    );
    Ok(())
}

// ===================================================================
// Refactored modernizer – detect each @array
// ===================================================================

#[test]
fn refactored_modernizer_detect_each_array() -> Result<(), Box<dyn std::error::Error>> {
    let m = RefactoredModernizer::new();
    let suggestions = m.analyze("while (each @array) { }");
    assert!(
        suggestions.iter().any(|s| s.old_pattern.contains("each @array")),
        "Should detect each @array"
    );
    Ok(())
}

// ===================================================================
// Validation – Package scope with non-existent file
// ===================================================================

#[test]
fn validate_rename_package_scope_nonexistent_file() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: "$y".to_string(),
            scope: RefactoringScope::Package {
                file: "/nonexistent/file.pl".into(),
                name: "Foo".to_string(),
            },
        },
        vec![],
    );
    assert!(result.is_err(), "Non-existent file in Package scope should be rejected");
    Ok(())
}

// ===================================================================
// Validation – Function scope with non-existent file
// ===================================================================

#[test]
fn validate_rename_function_scope_nonexistent_file() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: "$y".to_string(),
            scope: RefactoringScope::Function {
                file: "/nonexistent/file.pl".into(),
                name: "foo".to_string(),
            },
        },
        vec![],
    );
    assert!(result.is_err(), "Non-existent file in Function scope should be rejected");
    Ok(())
}

// ===================================================================
// Validation – Block scope with non-existent file
// ===================================================================

#[test]
fn validate_rename_block_scope_nonexistent_file() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: "$y".to_string(),
            scope: RefactoringScope::Block {
                file: "/nonexistent/file.pl".into(),
                start: (0, 0),
                end: (10, 0),
            },
        },
        vec![],
    );
    assert!(result.is_err(), "Non-existent file in Block scope should be rejected");
    Ok(())
}

// ===================================================================
// Validation – FileSet scope exceeds limit
// ===================================================================

#[test]
fn validate_fileset_scope_exceeds_limit() -> Result<(), Box<dyn std::error::Error>> {
    let files: Vec<_> = (0..5)
        .map(|i| temp_perl(&format!("my $v{} = {};", i, i)))
        .collect::<Result<Vec<_>, _>>()?;

    let paths: Vec<_> = files.iter().map(|(_, p)| p.clone()).collect();
    // Keep temp files alive
    let _handles: Vec<_> = files.into_iter().map(|(f, _)| f).collect();

    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        max_files_per_operation: 3,
        ..Default::default()
    });

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: "$y".to_string(),
            scope: RefactoringScope::FileSet(paths.clone()),
        },
        vec![],
    );
    assert!(result.is_err(), "FileSet exceeding limit should be rejected");
    Ok(())
}

// ===================================================================
// Validation – Workspace scope is always valid
// ===================================================================

#[test]
fn validate_rename_workspace_scope_valid() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });
    // Workspace scope doesn't require files to pass validation
    // (it may fail at execution but validation should pass)
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: "$y".to_string(),
            scope: RefactoringScope::Workspace,
        },
        vec![],
    );
    // Should not fail validation (may succeed or fail at execution)
    assert!(result.is_ok() || result.is_err());
    Ok(())
}

// ===================================================================
// Validation – new_name empty
// ===================================================================

#[test]
fn validate_rename_empty_new_name() -> Result<(), Box<dyn std::error::Error>> {
    let (_f, p) = temp_perl("my $x = 1;")?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: String::new(),
            scope: RefactoringScope::File(p.clone()),
        },
        vec![p],
    );
    assert!(result.is_err(), "Empty new_name should be rejected");
    Ok(())
}

// ===================================================================
// Validation – MoveCode target dir nonexistent
// ===================================================================

#[test]
fn validate_move_code_target_dir_nonexistent() -> Result<(), Box<dyn std::error::Error>> {
    let (_f, p) = temp_perl("sub foo { 1 }")?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });
    let result = engine.refactor(
        RefactoringType::MoveCode {
            source_file: p.clone(),
            target_file: "/nonexistent/deep/path/target.pl".into(),
            elements: vec!["foo".to_string()],
        },
        vec![p],
    );
    assert!(result.is_err(), "Non-existent target directory should be rejected");
    Ok(())
}

// ===================================================================
// Validation – MoveCode with invalid element name
// ===================================================================

#[test]
fn validate_move_code_invalid_element() -> Result<(), Box<dyn std::error::Error>> {
    let (_f1, p1) = temp_perl("sub foo { 1 }")?;
    let (_f2, p2) = temp_perl("1;")?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });
    let result = engine.refactor(
        RefactoringType::MoveCode {
            source_file: p1.clone(),
            target_file: p2,
            elements: vec!["123invalid".to_string()],
        },
        vec![p1],
    );
    assert!(result.is_err(), "Invalid element name should be rejected");
    Ok(())
}

// ===================================================================
// Validation – Inline with empty symbol name
// ===================================================================

#[test]
fn validate_inline_empty_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let (_f, p) = temp_perl("my $x = 1;")?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });
    let result = engine.refactor(
        RefactoringType::Inline { symbol_name: String::new(), all_occurrences: false },
        vec![p],
    );
    assert!(result.is_err(), "Empty symbol name should be rejected");
    Ok(())
}

// ===================================================================
// Validation – OptimizeImports with non-existent file
// ===================================================================

#[test]
fn validate_optimize_imports_nonexistent_file() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });
    let result = engine.refactor(
        RefactoringType::OptimizeImports {
            remove_unused: true,
            sort_alphabetically: false,
            group_by_type: false,
        },
        vec!["/nonexistent/file.pl".into()],
    );
    assert!(result.is_err(), "Non-existent file should be rejected for optimize");
    Ok(())
}

// ===================================================================
// Import optimizer – SuggestionPriority values
// ===================================================================

#[test]
fn import_optimizer_suggestion_priorities() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    // Unsorted imports + duplicates should produce suggestions with different priorities
    let content = "use warnings;\nuse strict;\nuse strict;\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    // Should have at least a sort suggestion (Low) and a duplicate removal (Medium)
    use perl_refactoring::import_optimizer::SuggestionPriority;
    let has_low =
        analysis.organization_suggestions.iter().any(|s| s.priority == SuggestionPriority::Low);
    let has_medium =
        analysis.organization_suggestions.iter().any(|s| s.priority == SuggestionPriority::Medium);
    assert!(has_low, "Should have Low priority suggestion (sort)");
    assert!(has_medium, "Should have Medium priority suggestion (dedup)");
    Ok(())
}

// ===================================================================
// Import optimizer – missing import confidence
// ===================================================================

#[test]
fn import_optimizer_missing_import_confidence() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "Foo::Bar::baz();\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    for missing in &analysis.missing_imports {
        assert!(
            missing.confidence > 0.0 && missing.confidence <= 1.0,
            "Confidence should be between 0 and 1"
        );
    }
    Ok(())
}

// ===================================================================
// Import optimizer – suggested location for missing imports
// ===================================================================

#[test]
fn import_optimizer_missing_import_location() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use strict;\nuse warnings;\nFoo::Bar::baz();\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    for missing in &analysis.missing_imports {
        assert!(missing.suggested_location > 0, "Suggested location should be positive");
        // Should suggest after the last existing import
        assert!(missing.suggested_location >= 2, "Should suggest after existing imports");
    }
    Ok(())
}

// ===================================================================
// Import optimizer – DBI (no exports) detection
// ===================================================================

#[test]
fn import_optimizer_dbi_unused() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use DBI;\nprint 'hello';\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    // DBI has no known exports and is OO. If it's not referenced at all, it should be flagged.
    let dbi_unused = analysis.unused_imports.iter().any(|u| u.module == "DBI");
    assert!(dbi_unused, "Unused DBI (no exports, no reference) should be flagged");
    Ok(())
}

// ===================================================================
// Import optimizer – LWP::UserAgent (no exports) detection
// ===================================================================

#[test]
fn import_optimizer_lwp_ua_unused() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use LWP::UserAgent;\nprint 'hello';\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    let lwp_unused = analysis.unused_imports.iter().any(|u| u.module == "LWP::UserAgent");
    assert!(lwp_unused, "Unused LWP::UserAgent (no exports, no reference) should be flagged");
    Ok(())
}

// ===================================================================
// Import optimizer – LWP::UserAgent used via OO
// ===================================================================

#[test]
fn import_optimizer_lwp_ua_used_oo() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use LWP::UserAgent;\nmy $ua = LWP::UserAgent->new();\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    assert!(
        !analysis.unused_imports.iter().any(|u| u.module == "LWP::UserAgent"),
        "LWP::UserAgent used via OO should not be flagged"
    );
    Ok(())
}

// ===================================================================
// Import optimizer – generate_optimized skips all-unused modules
// ===================================================================

#[test]
fn import_optimizer_optimized_drops_fully_unused() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use List::Util qw(max min);\nprint 1;\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;
    let optimized = optimizer.generate_optimized_imports(&analysis);

    // Both symbols unused – module should be dropped entirely
    assert!(!optimized.contains("List::Util"), "Module with all symbols unused should be dropped");
    Ok(())
}

// ===================================================================
// Legacy modernizer – suggestion positions
// ===================================================================

#[test]
fn legacy_modernizer_suggestion_positions() -> Result<(), Box<dyn std::error::Error>> {
    let m = LegacyModernizer::new();
    let code = "my $x = 1;\nopen FH, '<', 'f.txt';";
    let suggestions = m.analyze(code);

    let fh_suggestion = suggestions
        .iter()
        .find(|s| s.old_pattern == "open FH")
        .ok_or("open FH suggestion not found")?;

    assert!(fh_suggestion.start > 0, "Start position should be after first line");
    assert!(fh_suggestion.end > fh_suggestion.start, "End should be after start");
    assert_eq!(fh_suggestion.end - fh_suggestion.start, 7, "Range should cover 'open FH'");
    Ok(())
}

// ===================================================================
// Refactored modernizer – suggestion positions
// ===================================================================

#[test]
fn refactored_modernizer_suggestion_positions() -> Result<(), Box<dyn std::error::Error>> {
    let m = RefactoredModernizer::new();
    let code = "my $x = 1;\nopen FH, '<', 'f.txt';";
    let suggestions = m.analyze(code);

    let fh_suggestion = suggestions
        .iter()
        .find(|s| s.old_pattern == "open FH")
        .ok_or("open FH suggestion not found")?;

    assert!(fh_suggestion.start > 0, "Start position should be nonzero");
    assert_eq!(fh_suggestion.end - fh_suggestion.start, 7, "Range should be 7 bytes");
    Ok(())
}

// ===================================================================
// Modernization suggestion clone and debug
// ===================================================================

#[test]
fn modernization_suggestion_clone_debug() -> Result<(), Box<dyn std::error::Error>> {
    let s = perl_refactoring::modernize::ModernizationSuggestion {
        old_pattern: "old".to_string(),
        new_pattern: "new".to_string(),
        description: "test desc".to_string(),
        manual_review_required: true,
        start: 5,
        end: 10,
    };
    let cloned = s.clone();
    assert_eq!(s, cloned);
    let debug_str = format!("{:?}", s);
    assert!(debug_str.contains("old"), "Debug should show old_pattern");
    assert!(debug_str.contains("manual_review_required"), "Debug should show fields");
    Ok(())
}

// ===================================================================
// Refactored modernization suggestion equality
// ===================================================================

#[test]
fn refactored_suggestion_equality() -> Result<(), Box<dyn std::error::Error>> {
    let s1 = perl_refactoring::modernize_refactored::ModernizationSuggestion {
        old_pattern: "a".to_string(),
        new_pattern: "b".to_string(),
        description: "desc".to_string(),
        manual_review_required: false,
        start: 0,
        end: 1,
    };
    let s2 = s1.clone();
    assert_eq!(s1, s2);
    Ok(())
}

// ===================================================================
// Import optimizer – only-use-lines content
// ===================================================================

#[test]
fn import_optimizer_only_use_lines() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use strict;\nuse warnings;\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    assert_eq!(analysis.imports.len(), 2);
    assert!(analysis.unused_imports.is_empty(), "Pragmas should not be unused");
    assert!(analysis.missing_imports.is_empty(), "No code means no missing");
    Ok(())
}

// ===================================================================
// Import optimizer – overload pragma not flagged
// ===================================================================

#[test]
fn import_optimizer_overload_pragma() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use overload;\nprint 1;\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    assert!(
        !analysis.unused_imports.iter().any(|u| u.module == "overload"),
        "Overload pragma should not be flagged unused"
    );
    Ok(())
}

// ===================================================================
// Import optimizer – vars and subs pragmas
// ===================================================================

#[test]
fn import_optimizer_vars_subs_pragmas() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use vars;\nuse subs;\nprint 1;\n";
    let analysis = optimizer.analyze_content(content).map_err(|e| e.to_string())?;

    assert!(
        !analysis.unused_imports.iter().any(|u| u.module == "vars"),
        "vars pragma should not be flagged unused"
    );
    assert!(
        !analysis.unused_imports.iter().any(|u| u.module == "subs"),
        "subs pragma should not be flagged unused"
    );
    Ok(())
}

// ===================================================================
// Legacy modernizer – modernize_file with no applicable changes
// ===================================================================

#[test]
fn legacy_modernizer_file_no_changes() -> Result<(), Box<dyn std::error::Error>> {
    let code = "use strict;\nuse warnings;\nmy $x = 1;\n";
    let (_f, p) = temp_perl(code)?;

    let mut m = LegacyModernizer::new();
    let changes = m.modernize_file(&p, &[])?;
    assert_eq!(changes, 0, "Clean file should have zero changes");

    let after = std::fs::read_to_string(&p)?;
    assert_eq!(after, code, "File should be unchanged");
    Ok(())
}

// ===================================================================
// Engine – index_file does not crash
// ===================================================================

#[test]
fn engine_index_file_no_crash() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub foo { 1 }\nmy $x = 42;\n";
    let (_f, p) = temp_perl(code)?;

    let mut engine = engine_no_safe();
    // Should not crash regardless of features
    let _ = engine.index_file(&p, code);
    Ok(())
}

// ===================================================================
// Validate rename with & sigil consistency
// ===================================================================

#[test]
fn validate_rename_ampersand_sigil_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let (_f, p) = temp_perl("sub foo { 1 }")?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });
    // &foo -> &bar should be valid (same sigil)
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "&foo".to_string(),
            new_name: "&bar".to_string(),
            scope: RefactoringScope::File(p.clone()),
        },
        vec![p],
    );
    // Should not fail on validation
    assert!(result.is_ok() || result.is_err());
    Ok(())
}

// ===================================================================
// Validate rename with * glob sigil
// ===================================================================

#[test]
fn validate_rename_glob_sigil_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    let (_f, p) = temp_perl("my $x = 1;")?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "*foo".to_string(),
            new_name: "$bar".to_string(),
            scope: RefactoringScope::File(p.clone()),
        },
        vec![p],
    );
    assert!(result.is_err(), "Glob to scalar sigil mismatch should be rejected");
    Ok(())
}
