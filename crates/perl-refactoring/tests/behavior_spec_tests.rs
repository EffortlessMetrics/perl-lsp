//! BDD-style behavior specification tests for `perl-refactoring`.
//!
//! These tests document user-visible refactoring outcomes with scenario-oriented
//! names (`when_<condition>_then_<outcome>`). They intentionally focus on
//! behavior rather than implementation details.

use perl_refactoring::import_optimizer::ImportOptimizer;
use perl_refactoring::refactor::refactoring::{
    RefactoringConfig, RefactoringEngine, RefactoringScope, RefactoringType,
};
use std::io::Write;
use tempfile::NamedTempFile;

fn engine_apply_changes() -> RefactoringEngine {
    RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: false,
        create_backups: false,
        ..Default::default()
    })
}

fn temp_perl_file(
    content: &str,
) -> Result<(NamedTempFile, std::path::PathBuf), Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    write!(file, "{content}")?;
    let path = file.path().to_path_buf();
    Ok((file, path))
}

// ===========================================================================
// Scenario group: symbol rename
// ===========================================================================

#[test]
fn when_renaming_file_scoped_symbol_then_declaration_and_usages_are_updated()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "my $name = 'world';\nprint $name;\n";
    let (_file, path) = temp_perl_file(source)?;
    let mut engine = engine_apply_changes();
    engine.index_file(&path, source)?;

    // When
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$name".to_string(),
            new_name: "$label".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    // Then
    assert!(result.success, "rename should succeed");
    let rewritten = std::fs::read_to_string(&path)?;
    assert!(rewritten.contains("my $label = 'world';"));
    assert!(rewritten.contains("print $label;"));
    assert!(!rewritten.contains("$name"));
    Ok(())
}

#[test]
fn when_renaming_to_different_sigil_then_validation_rejects_operation()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "my $item = 1;\nprint $item;\n";
    let (_file, path) = temp_perl_file(source)?;
    let mut engine = RefactoringEngine::new();

    // When
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$item".to_string(),
            new_name: "@item".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path],
    );

    // Then
    assert!(result.is_err(), "sigil-mismatched rename should fail validation");
    Ok(())
}

// ===========================================================================
// Scenario group: method extraction
// ===========================================================================

#[test]
fn when_extracting_method_then_engine_inserts_subroutine_and_call_site()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "my $x = 2;\nmy $y = $x + 3;\nprint $y;\n";
    let (_file, path) = temp_perl_file(source)?;
    let mut engine = engine_apply_changes();

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
    assert!(result.success, "extract method should succeed");
    let rewritten = std::fs::read_to_string(&path)?;
    assert!(rewritten.contains("sub compute_value"));
    assert!(rewritten.contains("compute_value("));
    Ok(())
}

#[test]
fn when_extract_method_name_is_not_identifier_then_safe_mode_validation_fails()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "my $x = 2;\nprint $x;\n";
    let (_file, path) = temp_perl_file(source)?;
    let mut engine = RefactoringEngine::new();

    // When
    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "&bad_name".to_string(),
            start_position: (0, 0),
            end_position: (1, 0),
        },
        vec![path],
    );

    // Then
    assert!(result.is_err(), "invalid method name should be rejected");
    Ok(())
}

// ===========================================================================
// Scenario group: import optimization behavior
// ===========================================================================

#[test]
fn when_analyzing_duplicate_use_statements_then_optimizer_reports_deduplication()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "use strict;\nuse warnings;\nuse strict;\nprint 1;\n";
    let optimizer = ImportOptimizer::new();

    // When
    let analysis = optimizer.analyze_content(source)?;

    // Then
    assert!(
        analysis.duplicate_imports.iter().any(|d| d.module == "strict"),
        "duplicate strict imports should be detected"
    );
    assert!(
        analysis.organization_suggestions.iter().any(|s| s
            .description
            .to_lowercase()
            .contains("duplicate")
            || s.description.to_lowercase().contains("dedup")),
        "analysis should include a deduplication organization suggestion"
    );
    Ok(())
}

#[test]
fn when_generating_optimized_import_block_then_pragmas_are_preserved()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "use strict;\nuse warnings;\nuse List::Util qw(sum max);\nprint sum(1,2);\n";
    let optimizer = ImportOptimizer::new();
    let analysis = optimizer.analyze_content(source)?;

    // When
    let optimized = optimizer.generate_optimized_imports(&analysis);

    // Then
    assert!(optimized.contains("use strict;"));
    assert!(optimized.contains("use warnings;"));
    assert!(optimized.contains("use List::Util"));
    Ok(())
}
