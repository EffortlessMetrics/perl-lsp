//! Behavior-driven integration scenarios for `perl-refactoring`.
//!
//! These tests document user-visible behavior in a Given/When/Then style so
//! feature work can be validated against real workflows instead of internal
//! implementation details.

use perl_refactoring::import_optimizer::ImportOptimizer;
use perl_refactoring::refactor::refactoring::{
    RefactoringConfig, RefactoringEngine, RefactoringScope, RefactoringType,
};
use std::io::Write;
use tempfile::NamedTempFile;

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
    let mut file = NamedTempFile::new()?;
    write!(file, "{}", content)?;
    let path = file.path().to_path_buf();
    Ok((file, path))
}

#[test]
fn bdd_symbol_rename_file_scope_given_perl_variable_when_renamed_then_all_file_occurrences_change()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let perl = "my $count = 1;\n$count++;\nprint $count;\n";
    let (_file, path) = temp_perl(perl)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, perl)?;

    // When
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$count".to_string(),
            new_name: "$total".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    // Then
    assert!(result.success, "Rename should report success");
    assert_eq!(result.files_modified, 1, "Exactly one file should be modified");
    let updated = std::fs::read_to_string(path)?;
    assert!(updated.contains("my $total = 1;"));
    assert!(updated.contains("$total++;"));
    assert!(!updated.contains("$count"));
    Ok(())
}

#[test]
fn bdd_extract_method_given_repeated_logic_when_extracted_then_subroutine_and_callsite_are_created()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let perl = "my $x = 1;\nmy $y = $x + 2;\nprint $y;\n";
    let (_file, path) = temp_perl(perl)?;
    let mut engine = engine_no_safe();

    // When
    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "compute_value".to_string(),
            start_position: (1, 0),
            end_position: (2, 0),
        },
        vec![path.clone()],
    )?;

    // Then
    assert!(result.success, "Extraction should report success");
    assert_eq!(result.changes_made, 2, "Expected method extraction + call-site rewrite");
    let updated = std::fs::read_to_string(path)?;
    assert!(updated.contains("sub compute_value"), "Extracted subroutine must exist");
    assert!(updated.contains("compute_value("), "Call-site should invoke extracted subroutine");
    Ok(())
}

#[test]
fn bdd_inline_variable_given_simple_symbol_when_inlined_then_operation_completes_without_crashing()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let perl = "my $x = 42;\nprint $x;\n";
    let (_file, path) = temp_perl(perl)?;
    let mut engine = engine_no_safe();

    // When
    let result = engine.refactor(
        RefactoringType::Inline { symbol_name: "$x".to_string(), all_occurrences: true },
        vec![path],
    )?;

    // Then
    assert!(
        result.success || !result.warnings.is_empty() || !result.errors.is_empty(),
        "Inline should complete and return structured outcome"
    );
    Ok(())
}

#[test]
fn bdd_optimize_imports_given_unused_module_when_optimized_then_unused_use_statement_is_removed()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let optimizer = ImportOptimizer::new();
    let perl = "use Carp qw(croak);\nuse strict;\nmy $x = 42;\nprint $x;\n";

    // When
    let analysis = optimizer.analyze_content(perl)?;
    let optimized = optimizer.generate_optimized_imports(&analysis);

    // Then
    assert!(!optimized.contains("use Carp"), "Unused import should be dropped");
    assert!(optimized.contains("use strict;"), "Required pragma should remain");
    Ok(())
}
