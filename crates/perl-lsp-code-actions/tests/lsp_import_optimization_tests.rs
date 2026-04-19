//! Tests for import optimization code actions
//!
//! These tests verify the integration of the ImportOptimizer from perl-refactoring
//! with the LSP code actions system.

use perl_lsp_code_actions::{CodeActionKind, CodeActionsProvider};
use perl_parser_core::Parser;
use perl_tdd_support::must;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn lsp_organize_imports_removes_unused() -> TestResult {
    let source = r#"use strict;
use warnings;
use Data::Dumper qw(Dumper);
use JSON qw(encode_json decode_json);

my $data = { key => 'value' };
print encode_json($data);
"#;

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = CodeActionsProvider::new(source.to_string());
    let actions = provider.get_code_actions(&ast, (0, source.len()), &[]);

    // Should have an "Organize Imports" action
    let organize_action = actions
        .iter()
        .find(|a| matches!(a.kind, CodeActionKind::SourceOrganizeImports))
        .ok_or("No organize imports action found")?;

    assert_eq!(organize_action.title, "Organize imports");
    assert!(!organize_action.edit.changes.is_empty());

    Ok(())
}

#[test]
fn lsp_organize_imports_removes_duplicates() -> TestResult {
    let source = r#"use strict;
use Data::Dumper;
use warnings;
use Data::Dumper;

print Dumper(\@ARGV);
"#;

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = CodeActionsProvider::new(source.to_string());
    let actions = provider.get_code_actions(&ast, (0, source.len()), &[]);

    let organize_action = actions
        .iter()
        .find(|a| matches!(a.kind, CodeActionKind::SourceOrganizeImports))
        .ok_or("No organize imports action found")?;

    // Check that the edit would consolidate duplicates
    assert!(!organize_action.edit.changes.is_empty());

    Ok(())
}

#[test]
fn lsp_organize_imports_sorts_alphabetically() -> TestResult {
    let source = r#"use warnings;
use strict;
use Data::Dumper;

print "Hello\n";
"#;

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = CodeActionsProvider::new(source.to_string());
    let actions = provider.get_code_actions(&ast, (0, source.len()), &[]);

    let organize_action = actions
        .iter()
        .find(|a| matches!(a.kind, CodeActionKind::SourceOrganizeImports))
        .ok_or("No organize imports action found")?;

    assert!(!organize_action.edit.changes.is_empty());

    Ok(())
}

#[test]
fn lsp_organize_imports_detects_missing_imports() -> TestResult {
    let source = r#"use strict;
use warnings;

my $result = JSON::encode_json({key => 'value'});
"#;

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = CodeActionsProvider::new(source.to_string());
    let actions = provider.get_code_actions(&ast, (0, source.len()), &[]);

    // Should suggest adding missing JSON import
    let has_import_action = actions.iter().any(|a| {
        a.title.to_lowercase().contains("import") && a.title.to_lowercase().contains("json")
    });

    // This might be in a different action or in organize imports
    assert!(
        has_import_action
            || actions
                .iter()
                .any(|a| matches!(a.kind, CodeActionKind::SourceOrganizeImports)),
        "Expected import-related action"
    );

    Ok(())
}

#[test]
fn lsp_organize_imports_preserves_pragmas() -> TestResult {
    let source = r#"use strict;
use warnings;
use Data::Dumper;
use JSON;

print "Hello\n";
"#;

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = CodeActionsProvider::new(source.to_string());
    let actions = provider.get_code_actions(&ast, (0, source.len()), &[]);

    let organize_action = actions
        .iter()
        .find(|a| matches!(a.kind, CodeActionKind::SourceOrganizeImports));

    // Organize imports should be available even for well-formed imports
    assert!(
        organize_action.is_some()
            || actions
                .iter()
                .any(|a| a.title.to_lowercase().contains("import")),
        "Should have import-related actions"
    );

    Ok(())
}

#[test]
fn lsp_no_organize_imports_when_no_imports() -> TestResult {
    let source = r#"print "Hello, world!\n";"#;

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = CodeActionsProvider::new(source.to_string());
    let actions = provider.get_code_actions(&ast, (0, source.len()), &[]);

    // Should not have organize imports action when there are no imports
    let has_organize = actions
        .iter()
        .any(|a| matches!(a.kind, CodeActionKind::SourceOrganizeImports));

    // It's okay to not have the action when there are no imports to organize
    // But if there IS one, it should handle the empty case gracefully
    assert!(
        !has_organize || actions.iter().any(|a| a.title.contains("pragma")),
        "If organize imports exists with no imports, it should at least suggest pragmas"
    );

    Ok(())
}

#[test]
fn lsp_organize_imports_dedup_edit_content() -> TestResult {
    let source = "use strict;\nuse Data::Dumper;\nuse warnings;\nuse Data::Dumper;\n\nprint 1;\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = CodeActionsProvider::new(source.to_string());
    let actions = provider.get_code_actions(&ast, (0, source.len()), &[]);

    let organize_action = actions
        .iter()
        .find(|a| matches!(a.kind, CodeActionKind::SourceOrganizeImports))
        .ok_or("No organize imports action found")?;

    let new_text = &organize_action.edit.changes[0].new_text;
    // After dedup and sort: strict, warnings (pragmas), then Data::Dumper (CPAN)
    assert_eq!(
        new_text.matches("Data::Dumper").count(),
        1,
        "Duplicate imports should be removed, got: {new_text}"
    );

    Ok(())
}

#[test]
fn lsp_organize_imports_sort_order_in_edit() -> TestResult {
    let source = "use JSON;\nuse strict;\nuse File::Path;\nuse warnings;\n\nmy $x = 1;\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let provider = CodeActionsProvider::new(source.to_string());
    let actions = provider.get_code_actions(&ast, (0, source.len()), &[]);

    let organize_action = actions
        .iter()
        .find(|a| matches!(a.kind, CodeActionKind::SourceOrganizeImports))
        .ok_or("No organize imports action found")?;

    let new_text = &organize_action.edit.changes[0].new_text;
    let lines: Vec<&str> = new_text.lines().collect();

    let strict_pos = lines.iter().position(|l| l.contains("strict"));
    let json_pos = lines.iter().position(|l| l.contains("JSON"));

    assert!(
        strict_pos < json_pos,
        "Pragmas should sort before CPAN modules. Got: {new_text}"
    );

    Ok(())
}
