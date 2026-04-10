//! BDD-style scenarios for the `perl-refactoring` crate.
//!
//! The goal of this suite is to capture user-facing behavior in
//! Given/When/Then form so regression intent is obvious while reading tests.

use perl_refactoring::modernize_refactored::PerlModernizer;
use perl_refactoring::refactor::refactoring::{
    RefactoringConfig, RefactoringEngine, RefactoringScope, RefactoringType,
};
use std::fs;
use std::io::Write;
use tempfile::NamedTempFile;

fn temp_perl_file(
    content: &str,
) -> Result<(NamedTempFile, std::path::PathBuf), Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::new()?;
    write!(file, "{}", content)?;
    let path = file.path().to_path_buf();
    Ok((file, path))
}

#[test]
fn given_legacy_script_when_analyzed_then_pragmas_are_suggested() {
    // Given: a script-like Perl source without strict/warnings
    let source = "#!/usr/bin/perl\nprint \"Hello\\n\";\n";

    // When: modernizer analyzes the source
    let modernizer = PerlModernizer::new();
    let suggestions = modernizer.analyze(source);

    // Then: it should suggest adding strict and warnings pragmas
    assert!(
        suggestions
            .iter()
            .any(|suggestion| suggestion.new_pattern.contains("use strict;\nuse warnings;")),
        "Expected a pragma modernization suggestion"
    );
}

#[test]
fn given_invalid_rename_request_when_refactor_runs_then_validation_error_is_returned()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a file and a rename where old and new names are the same
    let (_file, path) = temp_perl_file("my $value = 1;\nprint $value;\n")?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });

    // When: refactor is requested
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$value".to_string(),
            new_name: "$value".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path],
    );

    // Then: validation should fail
    assert!(result.is_err(), "Expected validation error for no-op rename");
    Ok(())
}

#[test]
fn given_file_scope_rename_when_refactor_runs_then_operation_is_recorded()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a small Perl file with repeated variable use
    let (_file, path) = temp_perl_file("my $count = 1;\nprint $count;\n")?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: false,
        create_backups: false,
        ..Default::default()
    });

    // When: a file-scoped symbol rename is executed
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$count".to_string(),
            new_name: "$total".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    // Then: the operation should return a structured result and be tracked in history
    assert!(result.success || !result.errors.is_empty());
    assert_eq!(engine.get_operation_history().len(), 1);

    // And: the source file should remain readable after the operation
    let updated = fs::read_to_string(path)?;
    assert!(!updated.is_empty());
    Ok(())
}

#[test]
fn given_risky_eval_pattern_when_analyzed_then_manual_review_is_required() {
    // Given: source using string eval
    let source = "#!/usr/bin/perl\neval \"print 1\";\n";

    // When: modernization suggestions are produced
    let modernizer = PerlModernizer::new();
    let suggestions = modernizer.analyze(source);

    // Then: risky eval suggestion should be present and marked manual review
    let eval_suggestion =
        suggestions.iter().find(|suggestion| suggestion.old_pattern == "eval \"...\"");
    assert!(
        eval_suggestion.is_some_and(|suggestion| suggestion.manual_review_required),
        "Expected eval modernization suggestion requiring manual review"
    );
}
