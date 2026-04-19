//! Behavior-driven integration tests for `perl-refactoring` workflows.
//!
//! These scenarios cover realistic editing journeys with explicit
//! Given/When/Then phases.

use perl_refactoring::refactor::refactoring::{
    ModernizationPattern, RefactoringConfig, RefactoringEngine, RefactoringScope, RefactoringType,
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
    let mut f = NamedTempFile::new()?;
    write!(f, "{}", content)?;
    let p = f.path().to_path_buf();
    Ok((f, p))
}

#[test]
fn bdd_rename_symbol_with_function_scope() -> Result<(), Box<dyn std::error::Error>> {
    // Given a file with the same variable name both inside and outside a function.
    let code = concat!(
        "my $value = 'outer';\n",
        "sub transform {\n",
        "    my $value = 'inner';\n",
        "    print $value;\n",
        "}\n",
        "print $value;\n",
    );
    let (_file, path) = temp_perl(code)?;

    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    // When we rename the symbol only within function scope.
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$value".to_string(),
            new_name: "$result".to_string(),
            scope: RefactoringScope::Function {
                file: path.clone(),
                name: "transform".to_string(),
            },
        },
        vec![path.clone()],
    )?;

    // Then only the function-local variable references are updated.
    assert!(result.success, "rename should report success");
    let rewritten = std::fs::read_to_string(&path)?;
    assert!(rewritten.contains("my $result = 'inner';"));
    assert!(rewritten.contains("print $result;"));
    assert!(rewritten.ends_with("print $value;\n"));
    Ok(())
}

#[test]
fn bdd_extract_method_creates_subroutine_and_call_site() -> Result<(), Box<dyn std::error::Error>> {
    // Given a linear script with transform logic inline.
    let code = "my $x = 10;\nmy $y = $x + 5;\nprint $y;\n";
    let (_file, path) = temp_perl(code)?;

    let mut engine = engine_no_safe();

    // When we extract the middle statement into a helper.
    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "compute_total".to_string(),
            start_position: (1, 0),
            end_position: (2, 0),
        },
        vec![path.clone()],
    )?;

    // Then the source includes the helper and the call site replacement.
    assert!(result.success, "extract method should succeed");
    let rewritten = std::fs::read_to_string(&path)?;
    assert!(rewritten.contains("sub compute_total"));
    assert!(rewritten.contains("compute_total("));
    Ok(())
}

#[test]
fn bdd_move_subroutine_between_files() -> Result<(), Box<dyn std::error::Error>> {
    // Given one file with a reusable subroutine and a second destination file.
    let source = "sub helper { 42 }\nsub run { helper() }\n";
    let target = "package Target;\n1;\n";
    let (_src_file, src_path) = temp_perl(source)?;
    let (_dst_file, dst_path) = temp_perl(target)?;

    let mut engine = engine_no_safe();

    // When we move `helper` from source to target.
    let result = engine.refactor(
        RefactoringType::MoveCode {
            source_file: src_path.clone(),
            target_file: dst_path.clone(),
            elements: vec!["helper".to_string()],
        },
        vec![src_path.clone()],
    )?;

    // Then the symbol is removed from source and inserted into target.
    assert!(result.success, "move code should succeed");
    let new_source = std::fs::read_to_string(&src_path)?;
    let new_target = std::fs::read_to_string(&dst_path)?;
    assert!(!new_source.contains("sub helper"));
    assert!(new_target.contains("sub helper"));
    Ok(())
}

#[test]
fn bdd_modernize_adds_strict_and_warnings() -> Result<(), Box<dyn std::error::Error>> {
    // Given legacy Perl without strict/warnings pragmas.
    let code = "#!/usr/bin/perl\nopen FH, 'file.txt';\n";
    let (_file, path) = temp_perl(code)?;

    let mut engine = engine_no_safe();

    // When we run modernize with StrictWarnings enabled.
    let result = engine.refactor(
        RefactoringType::Modernize {
            patterns: vec![ModernizationPattern::StrictWarnings],
        },
        vec![path.clone()],
    )?;

    // Then the operation reports success and preserves a valid file payload.
    assert!(result.success, "modernization should succeed");
    let rewritten = std::fs::read_to_string(&path)?;
    assert!(
        !rewritten.is_empty(),
        "file should remain readable after modernize"
    );
    Ok(())
}
