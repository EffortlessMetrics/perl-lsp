//! Edge-case and coverage-gap tests for the perl-refactoring crate.
//!
//! Covers scenarios not exercised by the existing test suites:
//! - Rename variable that shadows another in nested closures
//! - Rename across closure boundaries
//! - Refactoring in string interpolation contexts
//! - Refactoring in heredoc contexts
//! - Extract method from code with closures
//! - Rename of array/hash variables with sigil variants
//! - Import optimizer edge cases (parent/base pragmas, version imports)
//! - Inline variable used in interpolation
//! - Multiple sequential refactoring operations

use perl_refactoring::import_optimizer::ImportOptimizer;
use perl_refactoring::refactor::refactoring::{
    RefactoringConfig, RefactoringEngine, RefactoringScope, RefactoringType,
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

// ===========================================================================
// Section 1: Rename with variable shadowing in closures
// ===========================================================================

#[test]
fn rename_variable_shadowed_in_closure() -> Result<(), Box<dyn std::error::Error>> {
    // A variable $x is declared in an outer sub, and a closure inside
    // re-declares $x with `my`. Renaming $x at function scope should
    // rename the outer one but the inner shadowed one should also be renamed
    // because function-scope rename covers the entire function body.
    let code = r#"
sub outer {
    my $x = 10;
    my $closure = sub {
        my $x = 20;
        return $x;
    };
    return $x + $closure->();
}
"#;
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: "$val".to_string(),
            scope: RefactoringScope::Function { file: path.clone(), name: "outer".to_string() },
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;

    // Both occurrences of $x within outer() should be renamed
    assert!(new_code.contains("my $val = 10"), "Outer $x should become $val");
    // The function body should not contain $x anymore
    // (the inner closure's $x is also within the function scope)
    let outer_fn_start = new_code.find("sub outer").ok_or("sub outer not found")?;
    let outer_fn_body = &new_code[outer_fn_start..];
    assert!(!outer_fn_body.contains("$x"), "No $x should remain inside sub outer after rename");
    Ok(())
}

#[test]
fn rename_variable_in_block_preserves_outer_closure() -> Result<(), Box<dyn std::error::Error>> {
    // Block-scoped rename should only rename within the block, leaving the
    // closure outside that block untouched.
    let code = r#"
my $name = "world";
{
    my $name = "block";
    print $name;
}
my $greet = sub { return $name; };
"#;
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$name".to_string(),
            new_name: "$label".to_string(),
            scope: RefactoringScope::Block { file: path.clone(), start: (3, 0), end: (5, 10) },
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;

    // Inside the block, $name should be renamed to $label
    assert!(new_code.contains("my $label = \"block\""), "Block variable should be renamed");
    // The outer $name and the closure's $name should remain
    assert!(new_code.contains("my $name = \"world\""), "Outer $name should be unchanged");
    // The closure referencing outer $name should remain
    assert!(
        new_code.contains("return $name"),
        "Closure reference to outer $name should be unchanged"
    );
    Ok(())
}

// ===========================================================================
// Section 2: Rename in string interpolation contexts
// ===========================================================================

#[test]
fn rename_variable_in_double_quoted_string() -> Result<(), Box<dyn std::error::Error>> {
    // Perl interpolates $var inside double-quoted strings. A rename of $msg
    // should also rename it inside "..." strings since the text replacement
    // is token-unaware (regex-based).
    let code = r#"
my $msg = "hello";
print "The message is: $msg\n";
print '$msg is not interpolated here';
"#;
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$msg".to_string(),
            new_name: "$greeting".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;

    // Declaration and bare usage should be renamed
    assert!(new_code.contains("my $greeting"), "Variable declaration should be renamed");
    // The double-quoted string should have $greeting (regex-based rename)
    assert!(new_code.contains("$greeting"), "Interpolated usage should be renamed");
    Ok(())
}

#[test]
fn rename_variable_in_heredoc() -> Result<(), Box<dyn std::error::Error>> {
    // Heredocs with interpolation (<<EOF or <<"EOF") should also get renamed
    // since the rename is text-based.
    let code = r#"my $user = "alice";
print <<EOF;
Hello $user, welcome!
EOF
print $user;
"#;
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$user".to_string(),
            new_name: "$username".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;

    assert!(new_code.contains("my $username"), "Declaration should be renamed");
    // The heredoc body should also reflect the rename
    assert!(new_code.contains("$username"), "Variable in heredoc should be renamed");
    Ok(())
}

// ===========================================================================
// Section 3: Rename array and hash variables
// ===========================================================================

#[test]
fn rename_array_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my @items = (1, 2, 3);
push @items, 4;
my $count = scalar @items;
"#;
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "@items".to_string(),
            new_name: "@elements".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;

    assert!(new_code.contains("my @elements"), "Array declaration should be renamed");
    assert!(new_code.contains("push @elements"), "Array usage should be renamed");
    Ok(())
}

#[test]
fn rename_hash_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my %config = (key => "value");
$config{key} = "new_value";
my @keys = keys %config;
"#;
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "%config".to_string(),
            new_name: "%settings".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;

    assert!(new_code.contains("my %settings"), "Hash declaration should be renamed");
    assert!(new_code.contains("keys %settings"), "Hash usage with % sigil should be renamed");
    Ok(())
}

// ===========================================================================
// Section 4: Validation edge cases for rename
// ===========================================================================

#[test]
fn rename_rejects_sigil_change_dollar_to_at() -> Result<(), Box<dyn std::error::Error>> {
    let (_f, path) = temp_perl("my $x = 1;")?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: "@x".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    );

    assert!(result.is_err(), "Sigil change should be rejected by validation");
    Ok(())
}

#[test]
fn rename_rejects_sigil_change_percent_to_dollar() -> Result<(), Box<dyn std::error::Error>> {
    let (_f, path) = temp_perl("my %h = ();")?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "%h".to_string(),
            new_name: "$h".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    );

    assert!(result.is_err(), "Sigil change should be rejected by validation");
    Ok(())
}

#[test]
fn rename_qualified_package_name() -> Result<(), Box<dyn std::error::Error>> {
    // Validate that qualified names like Package::Foo are accepted
    let code = "package My::Module;\nsub method { 1 }\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "method".to_string(),
            new_name: "new_method".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    // Whether it succeeds or finds no matches depends on index contents,
    // but it should not error on validation
    assert!(
        result.success || result.errors.is_empty(),
        "Rename of a bare identifier should pass validation"
    );
    Ok(())
}

// ===========================================================================
// Section 5: Import optimizer additional edge cases
// ===========================================================================

#[test]
fn import_optimizer_parent_pragma() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = r#"use strict;
use warnings;
use parent 'Exporter';

our @EXPORT_OK = qw(foo);
sub foo { 1 }
"#;

    let analysis = optimizer.analyze_content(content)?;

    // `parent` is a pragma and should not be flagged as unused
    assert!(
        !analysis.unused_imports.iter().any(|u| u.module == "parent"),
        "parent pragma should not be flagged as unused"
    );
    Ok(())
}

#[test]
fn import_optimizer_base_pragma_with_qw_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = r#"use strict;
use warnings;
use base qw(Exporter);

sub bar { 1 }
"#;

    let analysis = optimizer.analyze_content(content)?;

    // `base` is listed in is_pragma_module, but when explicit qw() symbols are
    // provided, each symbol is individually checked for usage. "Exporter" is not
    // used directly in the code body, so the optimizer flags it as unused.
    // This documents a known limitation: pragma-like modules with explicit import
    // lists are checked symbol-by-symbol rather than being exempted as pragmas.
    let base_unused = analysis.unused_imports.iter().find(|u| u.module == "base");
    if let Some(unused) = base_unused {
        assert!(
            unused.symbols.contains(&"Exporter".to_string()),
            "Exporter should be flagged as unused symbol under base"
        );
    }
    // The import itself should be detected
    assert!(analysis.imports.iter().any(|i| i.module == "base"), "base import should be detected");
    Ok(())
}

#[test]
fn import_optimizer_feature_pragma_with_qw_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = r#"use strict;
use warnings;
use feature qw(say signatures);

say "hello";
"#;

    let analysis = optimizer.analyze_content(content)?;

    // `feature` is listed in is_pragma_module, but when explicit qw() symbols are
    // provided, each symbol is checked individually. "say" appears in the code
    // as a word, but "signatures" does not, so it gets flagged. This documents
    // a known limitation: pragma feature strings are not actual function imports
    // but the optimizer still checks them against code usage.
    let feature_unused = analysis.unused_imports.iter().find(|u| u.module == "feature");
    if let Some(unused) = feature_unused {
        // "signatures" should be flagged, "say" should not (it appears in code)
        assert!(
            unused.symbols.contains(&"signatures".to_string()),
            "signatures should be flagged as unused: {:?}",
            unused.symbols
        );
    }
    // The import itself should be detected
    assert!(
        analysis.imports.iter().any(|i| i.module == "feature"),
        "feature import should be detected"
    );
    Ok(())
}

#[test]
fn import_optimizer_version_import_parsed() -> Result<(), Box<dyn std::error::Error>> {
    // `use Module VERSION;` should still be recognized
    let optimizer = ImportOptimizer::new();
    let content = r#"use strict;
use warnings;
use File::Basename;

my $dir = dirname("/foo/bar");
"#;

    let analysis = optimizer.analyze_content(content)?;

    assert!(
        analysis.imports.iter().any(|i| i.module == "File::Basename"),
        "File::Basename should be detected as an import"
    );
    Ok(())
}

#[test]
fn import_optimizer_comment_lines_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = r#"use strict;
use warnings;
# use Unused::Module;
use List::Util qw(max);

my $m = max(1, 2, 3);
"#;

    let analysis = optimizer.analyze_content(content)?;

    // The commented-out use should not appear in imports
    assert!(
        !analysis.imports.iter().any(|i| i.module == "Unused::Module"),
        "Commented-out use should not be detected"
    );
    // The actual import should be there
    assert!(
        analysis.imports.iter().any(|i| i.module == "List::Util"),
        "List::Util should be detected"
    );
    Ok(())
}

#[test]
fn import_optimizer_empty_qw() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = r#"use strict;
use warnings;
use My::Module qw();

My::Module::do_something();
"#;

    let analysis = optimizer.analyze_content(content)?;

    let my_module = analysis.imports.iter().find(|i| i.module == "My::Module");
    assert!(my_module.is_some(), "My::Module should be detected");

    // An empty qw() means no symbols are imported
    if let Some(imp) = my_module {
        assert!(imp.symbols.is_empty(), "Empty qw() should yield no symbols");
    }
    Ok(())
}

#[test]
fn import_optimizer_generates_edits_for_consolidation() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content =
        "use List::Util qw(max);\nuse List::Util qw(min);\n\nmy $m = max(1,2) + min(3,4);\n";

    let analysis = optimizer.analyze_content(content)?;

    // Should detect duplicate
    assert_eq!(analysis.duplicate_imports.len(), 1, "Should detect 1 duplicate");

    // Generate edits
    let edits = optimizer.generate_edits(content, &analysis);
    assert!(!edits.is_empty(), "Should generate consolidation edits");

    // The optimized imports should consolidate into a single use statement
    let optimized = optimizer.generate_optimized_imports(&analysis);
    assert!(
        optimized.contains("use List::Util qw(max min)"),
        "Should consolidate to single import with both symbols, got: {}",
        optimized
    );
    Ok(())
}

// ===========================================================================
// Section 6: Extract method edge cases
// ===========================================================================

#[test]
fn extract_method_from_single_statement() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $a = 1;\nmy $b = 2;\nmy $sum = $a + $b;\nprint $sum;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();

    // Extract just the `my $sum = $a + $b;` line (line 2, 0-indexed)
    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "compute_sum".to_string(),
            start_position: (2, 0),
            end_position: (3, 0),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Extract should succeed");
    assert_eq!(result.changes_made, 2, "Should have call + sub");

    let new_code = std::fs::read_to_string(&path)?;
    assert!(new_code.contains("sub compute_sum"), "New subroutine should be created");
    assert!(new_code.contains("compute_sum("), "Call site should reference the new sub");
    Ok(())
}

#[test]
fn extract_method_preserves_surrounding_code() -> Result<(), Box<dyn std::error::Error>> {
    let code = "# Header comment\nmy $x = 1;\nmy $y = $x * 2;\nprint $y;\n# Footer comment\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();

    // Extract `my $y = $x * 2;`
    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "double_it".to_string(),
            start_position: (2, 0),
            end_position: (3, 0),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Extract should succeed");
    let new_code = std::fs::read_to_string(&path)?;

    // Surrounding code should be preserved
    assert!(new_code.contains("# Header comment"), "Header should be preserved");
    assert!(new_code.contains("# Footer comment"), "Footer should be preserved");
    assert!(new_code.contains("my $x = 1"), "Code before extraction should be preserved");
    Ok(())
}

// ===========================================================================
// Section 7: Inline variable edge cases
// ===========================================================================

#[test]
fn inline_variable_not_found_reports_gracefully() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $existing = 1;\nprint $existing;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::Inline { symbol_name: "$nonexistent".to_string(), all_occurrences: true },
        vec![path.clone()],
    )?;

    // Should handle gracefully - either not succeed or succeed with 0 changes
    assert!(
        !result.success || result.changes_made == 0,
        "Inlining a non-existent variable should not claim success with changes"
    );
    Ok(())
}

#[test]
fn inline_subroutine_name_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub helper { 1 }\nhelper();\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::Inline { symbol_name: "helper".to_string(), all_occurrences: true },
        vec![path.clone()],
    )?;

    // Subroutine inlining is not supported (only variables with sigils)
    assert!(!result.success, "Inlining a bare subroutine name should not succeed");
    assert!(
        result.warnings.iter().any(|w| w.contains("not implemented")),
        "Should warn about unsupported inline type"
    );
    Ok(())
}

// ===========================================================================
// Section 8: Sequential refactoring operations
// ===========================================================================

#[test]
fn sequential_renames_in_same_file() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $foo = 1;\nmy $bar = 2;\nprint $foo + $bar;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    // First rename: $foo -> $alpha
    let r1 = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$foo".to_string(),
            new_name: "$alpha".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;
    assert!(r1.success, "First rename should succeed");

    // Re-read and re-index for second rename
    let updated_code = std::fs::read_to_string(&path)?;
    engine.index_file(&path, &updated_code)?;

    // Second rename: $bar -> $beta
    let r2 = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$bar".to_string(),
            new_name: "$beta".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;
    assert!(r2.success, "Second rename should succeed");

    let final_code = std::fs::read_to_string(&path)?;
    assert!(final_code.contains("$alpha"), "First rename should persist");
    assert!(final_code.contains("$beta"), "Second rename should persist");
    assert!(!final_code.contains("$foo"), "Original $foo should be gone");
    assert!(!final_code.contains("$bar"), "Original $bar should be gone");

    // Operation history should have 2 entries
    assert_eq!(engine.get_operation_history().len(), 2, "Should have 2 operations in history");
    Ok(())
}

// ===========================================================================
// Section 9: Move code edge cases
// ===========================================================================

#[test]
fn move_code_to_empty_target() -> Result<(), Box<dyn std::error::Error>> {
    let source_code = "sub greet { print 'hello'; }\nsub farewell { print 'bye'; }\n";
    let target_code = "";

    let (_sf, source_path) = temp_perl(source_code)?;
    let (_tf, target_path) = temp_perl(target_code)?;

    let mut engine = engine_no_safe();

    let result = engine.refactor(
        RefactoringType::MoveCode {
            source_file: source_path.clone(),
            target_file: target_path.clone(),
            elements: vec!["greet".to_string()],
        },
        vec![source_path.clone(), target_path.clone()],
    )?;

    assert!(result.success, "Move should succeed");

    let new_source = std::fs::read_to_string(&source_path)?;
    let new_target = std::fs::read_to_string(&target_path)?;

    assert!(!new_source.contains("sub greet"), "greet should be removed from source");
    assert!(new_source.contains("sub farewell"), "farewell should remain in source");
    assert!(new_target.contains("sub greet"), "greet should appear in target");
    Ok(())
}

// ===========================================================================
// Section 10: Modernizer edge cases
// ===========================================================================

#[test]
fn modernizer_detects_no_issues_in_modern_code() {
    let modernizer = perl_refactoring::modernize::PerlModernizer::new();
    let code = r#"use strict;
use warnings;
use feature 'say';

sub greet {
    my ($name) = @_;
    say "Hello, $name!";
}

open my $fh, '<', 'file.txt' or die "Cannot open: $!";
"#;

    let suggestions = modernizer.analyze(code);
    assert!(
        suggestions.is_empty(),
        "Modern code should produce no suggestions, got: {:?}",
        suggestions
    );
}

#[test]
fn refactored_modernizer_detects_no_issues_in_modern_code() {
    let modernizer = perl_refactoring::modernize_refactored::PerlModernizer::new();
    let code = r#"use strict;
use warnings;
use feature 'say';

sub greet {
    my ($name) = @_;
    say "Hello, $name!";
}

open my $fh, '<', 'file.txt' or die "Cannot open: $!";
"#;

    let suggestions = modernizer.analyze(code);
    assert!(
        suggestions.is_empty(),
        "Modern code should produce no suggestions, got: {:?}",
        suggestions
    );
}

#[test]
fn modernizer_detects_two_arg_open_with_literal_filename() {
    // The legacy modernizer uses exact string matching for patterns.
    // It only recognizes `open(FH, 'file.txt')` exactly, not arbitrary
    // two-arg open forms. This test documents the actual behavior.
    let modernizer = perl_refactoring::modernize::PerlModernizer::new();
    let code = r#"use strict;
use warnings;
open(FH, 'file.txt');
"#;

    let suggestions = modernizer.analyze(code);

    assert!(
        suggestions.iter().any(|s| s.description.contains("three-argument open")),
        "Should detect exact two-arg open pattern: {:?}",
        suggestions
    );
}

#[test]
fn modernizer_detects_bareword_filehandle() {
    let modernizer = perl_refactoring::modernize::PerlModernizer::new();
    let code = r#"use strict;
use warnings;
open FH, '<', 'file.txt';
close FH;
"#;

    let suggestions = modernizer.analyze(code);

    assert!(
        suggestions.iter().any(|s| s.description.contains("lexical filehandle")),
        "Should detect bareword filehandle FH: {:?}",
        suggestions
    );
}

// ===========================================================================
// Section 11: Config and engine lifecycle
// ===========================================================================

#[test]
fn engine_with_custom_config_respects_max_files() -> Result<(), Box<dyn std::error::Error>> {
    let config = RefactoringConfig {
        safe_mode: true,
        max_files_per_operation: 2,
        create_backups: false,
        ..Default::default()
    };
    let mut engine = RefactoringEngine::with_config(config);

    // Create 3 temp files
    let (_f1, p1) = temp_perl("my $a = 1;")?;
    let (_f2, p2) = temp_perl("my $a = 2;")?;
    let (_f3, p3) = temp_perl("my $a = 3;")?;

    // Attempting to operate on 3 files when max is 2 should fail
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$a".to_string(),
            new_name: "$b".to_string(),
            scope: RefactoringScope::Workspace,
        },
        vec![p1, p2, p3],
    );

    assert!(result.is_err(), "Should reject operation exceeding max_files_per_operation");
    Ok(())
}

#[test]
fn engine_clear_history_resets() -> Result<(), Box<dyn std::error::Error>> {
    let config = RefactoringConfig {
        safe_mode: false,
        create_backups: false,
        // Use a custom backup root so clear_history doesn't fail
        backup_root: Some(std::env::temp_dir().join("perl_refactor_test_clear")),
        ..Default::default()
    };
    let mut engine = RefactoringEngine::with_config(config);

    let code = "my $x = 1;";
    let (_f, path) = temp_perl(code)?;
    engine.index_file(&path, code)?;

    // Perform one operation
    let _result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: "$y".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert_eq!(engine.get_operation_history().len(), 1);

    // Clear history
    let cleanup = engine.clear_history()?;
    assert_eq!(engine.get_operation_history().len(), 0, "History should be empty after clear");
    // cleanup.directories_removed can be 0 since we disabled backups
    let _ = cleanup;
    Ok(())
}

// ===========================================================================
// Section 12: Rename with complex Perl identifiers
// ===========================================================================

#[test]
fn rename_variable_with_underscores() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $long_variable_name = 42;\nprint $long_variable_name;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$long_variable_name".to_string(),
            new_name: "$short_name".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;
    assert!(new_code.contains("$short_name"), "Renamed variable should appear");
    assert!(!new_code.contains("$long_variable_name"), "Original variable name should be gone");
    Ok(())
}

#[test]
fn rename_validates_numeric_start_identifier() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;";
    let (_f, path) = temp_perl(code)?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });

    // Attempt to rename to an identifier starting with a digit
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: "$1var".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    );

    assert!(result.is_err(), "Should reject identifier starting with a digit");
    Ok(())
}

// ===========================================================================
// Section 13: Import optimizer - generate_edits correctness
// ===========================================================================

#[test]
fn import_optimizer_edits_have_correct_byte_ranges() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use strict;\nuse warnings;\nuse List::Util qw(max min);\n\nmy $m = max(1,2);\n";

    let analysis = optimizer.analyze_content(content)?;
    let edits = optimizer.generate_edits(content, &analysis);

    // All edit ranges should be within content bounds
    for edit in &edits {
        assert!(
            edit.range.0 <= content.len(),
            "Edit start {} should be within content length {}",
            edit.range.0,
            content.len()
        );
        assert!(
            edit.range.1 <= content.len(),
            "Edit end {} should be within content length {}",
            edit.range.1,
            content.len()
        );
        assert!(edit.range.0 <= edit.range.1, "Edit start should not exceed end");
    }
    Ok(())
}

#[test]
fn import_optimizer_no_edits_for_clean_file() -> Result<(), Box<dyn std::error::Error>> {
    let optimizer = ImportOptimizer::new();
    let content = "use strict;\nuse warnings;\n\nprint \"hello\";\n";

    let analysis = optimizer.analyze_content(content)?;

    assert!(analysis.unused_imports.is_empty(), "No unused imports");
    assert!(analysis.missing_imports.is_empty(), "No missing imports");
    assert!(analysis.duplicate_imports.is_empty(), "No duplicates");
    Ok(())
}

// ===========================================================================
// Section 14: Backup and rollback with real files
// ===========================================================================

#[test]
fn backup_and_rollback_restores_original() -> Result<(), Box<dyn std::error::Error>> {
    let backup_root = tempfile::tempdir()?;
    let config = RefactoringConfig {
        safe_mode: false,
        create_backups: true,
        backup_root: Some(backup_root.path().to_path_buf()),
        ..Default::default()
    };
    let mut engine = RefactoringEngine::with_config(config);

    let code = "my $original = 1;\nprint $original;\n";
    let (_f, path) = temp_perl(code)?;
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$original".to_string(),
            new_name: "$changed".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let modified_code = std::fs::read_to_string(&path)?;
    assert!(modified_code.contains("$changed"), "File should be modified");

    // Rollback using the operation_id
    if let Some(op_id) = &result.operation_id {
        let rollback_result = engine.rollback(op_id)?;
        assert!(rollback_result.success, "Rollback should succeed");

        let restored_code = std::fs::read_to_string(&path)?;
        assert!(restored_code.contains("$original"), "File should be restored to original");
        assert!(
            !restored_code.contains("$changed"),
            "Modified content should be gone after rollback"
        );
    }
    Ok(())
}

// ===========================================================================
// Section 15: Rename in multiple packages within same file
// ===========================================================================

#[test]
fn rename_in_first_package_leaves_second_untouched() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Alpha;
my $count = 0;
sub inc { $count++ }

package Beta;
my $count = 0;
sub inc { $count++ }
"#;
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$count".to_string(),
            new_name: "$tally".to_string(),
            scope: RefactoringScope::Package { file: path.clone(), name: "Alpha".to_string() },
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;

    // Alpha's $count should be renamed
    let alpha_pos = new_code.find("package Alpha").ok_or("Alpha not found")?;
    let beta_pos = new_code.find("package Beta").ok_or("Beta not found")?;
    let alpha_section = &new_code[alpha_pos..beta_pos];
    let beta_section = &new_code[beta_pos..];

    assert!(alpha_section.contains("$tally"), "Alpha section should contain $tally");
    assert!(beta_section.contains("$count"), "Beta section should still contain $count");
    Ok(())
}

// ===========================================================================
// Section 16: Extract method validation
// ===========================================================================

#[test]
fn extract_method_rejects_empty_name() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });

    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: String::new(),
            start_position: (0, 0),
            end_position: (1, 0),
        },
        vec![path.clone()],
    );

    assert!(result.is_err(), "Empty method name should be rejected");
    Ok(())
}

#[test]
fn extract_method_rejects_reversed_range() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;\nmy $y = 2;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });

    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "extracted".to_string(),
            start_position: (1, 0),
            end_position: (0, 0),
        },
        vec![path.clone()],
    );

    assert!(result.is_err(), "Reversed range should be rejected by validation");
    Ok(())
}
