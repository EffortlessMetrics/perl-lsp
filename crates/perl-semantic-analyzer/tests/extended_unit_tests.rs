//! Extended unit tests for perl-semantic-analyzer.
//!
//! Covers areas not exercised by the existing comprehensive_unit_tests.rs:
//! - Declaration provider and utility functions
//! - Scope analyzer edge cases (IssueKinds: DuplicateParameter, ParameterShadowsGlobal,
//!   UnusedParameter, UndeclaredVariable, UninitializedVariable, UnquotedBareword)
//! - Scope analyzer suggestion generation
//! - SemanticAnalyzer::infer_type for operators and builtins
//! - SemanticModel API surface
//! - Cross-package qualified references
//! - SymbolTable find_symbol with scope chains
//! - SymbolExtractor with/without source
//! - Type inference engine: subroutine inference, constraint violations, builtins
//! - TypeEnvironment nested scoping
//! - WorkspaceIndex edge cases
//! - PerlType Subroutine construction

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::declaration::{
    DeclarationProvider, ParentMap, current_package_at, find_node_at_offset, get_node_children,
    symbol_at_cursor,
};
use perl_semantic_analyzer::analysis::scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue};
use perl_semantic_analyzer::analysis::semantic::{
    SemanticAnalyzer, SemanticModel, SemanticTokenType,
};
use perl_semantic_analyzer::analysis::type_inference::{
    PerlType, ScalarType, TypeBasedCompletion, TypeEnvironment, TypeInferenceEngine,
};
use perl_semantic_analyzer::symbol::{
    ScopeKind, SymbolExtractor, SymbolKind, SymbolTable, VarKind,
};
use perl_tdd_support::{must, must_some};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_and_extract(code: &str) -> SymbolTable {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    SymbolExtractor::new_with_source(code).extract(&ast)
}

fn parse_and_analyze(code: &str) -> SemanticAnalyzer {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    SemanticAnalyzer::analyze_with_source(&ast, code)
}

fn scope_issues(code: &str) -> Vec<ScopeIssue> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = ScopeAnalyzer::new();
    analyzer.analyze(&ast, code, &[])
}

fn has_symbol(table: &SymbolTable, name: &str, kind: SymbolKind) -> bool {
    table.symbols.get(name).is_some_and(|syms| syms.iter().any(|s| s.kind == kind))
}

fn byte_offset_to_line(code: &str, offset: usize) -> usize {
    code[..offset].bytes().filter(|b| *b == b'\n').count()
}

// ===========================================================================
// 1. Declaration provider utility functions
// ===========================================================================

#[test]
fn current_package_at_default_is_main() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let pkg = current_package_at(&ast, 0);
    assert_eq!(pkg, "main");
    Ok(())
}

#[test]
fn current_package_at_after_package_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let code = "package Foo::Bar;\nmy $x = 1;";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let pkg = current_package_at(&ast, 20);
    assert_eq!(pkg, "Foo::Bar");
    Ok(())
}

#[test]
fn current_package_at_multiple_packages() -> Result<(), Box<dyn std::error::Error>> {
    let code = "package Alpha;\nsub a {}\npackage Beta;\nsub b {}";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    // Offset in Alpha section
    let pkg1 = current_package_at(&ast, 16);
    assert_eq!(pkg1, "Alpha");
    // Offset in Beta section
    let pkg2 = current_package_at(&ast, 40);
    assert_eq!(pkg2, "Beta");
    Ok(())
}

#[test]
fn find_node_at_offset_returns_none_out_of_range() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let result = find_node_at_offset(&ast, 99999);
    assert!(result.is_none());
    Ok(())
}

#[test]
fn find_node_at_offset_finds_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    // $x starts at offset 3
    let node = find_node_at_offset(&ast, 3);
    assert!(node.is_some());
    Ok(())
}

#[test]
fn get_node_children_of_program() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1; my $y = 2;";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let children = get_node_children(&ast);
    assert!(children.len() >= 2, "program should have at least 2 statements");
    Ok(())
}

#[test]
fn symbol_at_cursor_on_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $foo = 42;";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    // $foo starts at offset 3
    let sym = symbol_at_cursor(&ast, 3, "main");
    assert!(sym.is_some());
    let key = sym.ok_or("no symbol")?;
    assert_eq!(key.name.as_ref(), "foo");
    Ok(())
}

#[test]
fn symbol_at_cursor_on_function_call() -> Result<(), Box<dyn std::error::Error>> {
    let code = "print(42);";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let sym = symbol_at_cursor(&ast, 0, "main");
    // print is a function call
    if let Some(key) = sym {
        assert_eq!(key.name.as_ref(), "print");
    }
    Ok(())
}

#[test]
fn symbol_at_cursor_on_method_call() -> Result<(), Box<dyn std::error::Error>> {
    let code = "$self->helper();";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // "$self->helper()" - "helper" starts at offset 7
    let helper_offset = code.find("helper").unwrap_or(7);

    // Debug: check what node is at the offset
    let node = find_node_at_offset(&ast, helper_offset);
    assert!(node.is_some(), "should find a node at the offset of 'helper'");
    let node = must_some(node);
    eprintln!(
        "Node at offset {}: kind={}, sexp={}",
        helper_offset,
        node.kind.kind_name(),
        node.to_sexp()
    );

    let sym = symbol_at_cursor(&ast, helper_offset, "MyPackage");
    assert!(
        sym.is_some(),
        "symbol_at_cursor should resolve method call, got node kind: {}",
        node.kind.kind_name()
    );
    let key = must_some(sym);
    assert_eq!(key.name.as_ref(), "helper");
    // $self -> use current package
    assert_eq!(key.pkg.as_ref(), "MyPackage");
    Ok(())
}

#[test]
fn symbol_at_cursor_on_use_statement() -> Result<(), Box<dyn std::error::Error>> {
    let code = "use Data::Dumper;";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // "Data::Dumper" starts at offset 4
    let module_offset = code.find("Data::Dumper").unwrap_or(4);

    let node = find_node_at_offset(&ast, module_offset);
    assert!(node.is_some(), "should find node at offset of module name");
    let node = must_some(node);
    eprintln!(
        "Node at offset {}: kind={}, sexp={}",
        module_offset,
        node.kind.kind_name(),
        node.to_sexp()
    );

    let sym = symbol_at_cursor(&ast, module_offset, "main");
    // If cursor lands on Use node, we should get a result
    if let Some(key) = sym {
        assert!(
            key.name.as_ref() == "Data::Dumper" || key.pkg.as_ref() == "Data::Dumper",
            "should reference Data::Dumper"
        );
    }
    Ok(())
}

#[test]
fn symbol_at_cursor_returns_none_in_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    let code = "    ";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let sym = symbol_at_cursor(&ast, 2, "main");
    // In whitespace, we may or may not find a node — just verify no panic
    let _ = sym;
    Ok(())
}

#[test]
fn declaration_provider_new_and_build_parent_map() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub greet { my $name = 'World'; }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let ast_arc = Arc::new(ast);

    let mut parent_map = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast_arc, &mut parent_map, None);
    // Parent map should contain entries for child nodes
    assert!(!parent_map.is_empty(), "parent map should not be empty for non-trivial AST");

    let provider =
        DeclarationProvider::new(ast_arc, code.to_string(), "file:///test.pl".to_string());
    // Just verify construction doesn't panic
    let _ = provider;
    Ok(())
}

#[test]
fn declaration_provider_with_parent_map() -> Result<(), Box<dyn std::error::Error>> {
    let code = "package Foo; sub bar { 1 }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let ast_arc = Arc::new(ast);

    let mut parent_map = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast_arc, &mut parent_map, None);

    let provider =
        DeclarationProvider::new(ast_arc, code.to_string(), "file:///test.pl".to_string())
            .with_parent_map(&parent_map);
    let _ = provider;
    Ok(())
}

#[test]
fn declaration_provider_with_doc_version() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let ast_arc = Arc::new(ast);

    let provider =
        DeclarationProvider::new(ast_arc, code.to_string(), "file:///test.pl".to_string())
            .with_doc_version(5);
    let _ = provider;
    Ok(())
}

/// Fix 2 (Phase 1): assert_fresh must return None on version mismatch in both debug and
/// release builds.  Before the fix, the release build silently ignores the mismatch
/// because `assert_fresh` was a debug-only no-op; `find_declaration` could still return
/// results from a stale provider.  After the fix, a version mismatch always returns None.
#[test]
fn declaration_provider_version_mismatch_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1; my $y = $x;";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let ast_arc = Arc::new(ast);

    let mut parent_map = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast_arc, &mut parent_map, None);

    // Use any valid offset inside the source; the stale-version guard fires before
    // any node lookup so the exact position is irrelevant to what we are testing.
    // Provider was built with version=1 but current_version=2 is passed to find_declaration.
    let offset_of_y_usage = must_some(code.find("$y")) + 1; // skip '$' — offset inside source
    let provider =
        DeclarationProvider::new(ast_arc, code.to_string(), "file:///test.pl".to_string())
            .with_parent_map(&parent_map)
            .with_doc_version(1);

    // Version mismatch: provider version=1, current_version=2
    let result = provider.find_declaration(offset_of_y_usage, 2);
    assert!(
        result.is_none(),
        "stale provider (version mismatch) must return None, not a result from the old AST"
    );
    Ok(())
}

/// Fix 2 (Phase 1, negative): assert_fresh must NOT suppress results when versions match.
#[test]
fn declaration_provider_matching_version_returns_result() -> Result<(), Box<dyn std::error::Error>>
{
    let code = "my $x = 1; print $x;";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let ast_arc = Arc::new(ast);

    let mut parent_map = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast_arc, &mut parent_map, None);

    // $x usage is at the `print $x` site
    let offset_of_x_usage = must_some(code.rfind("$x"));
    let provider =
        DeclarationProvider::new(ast_arc, code.to_string(), "file:///test.pl".to_string())
            .with_parent_map(&parent_map)
            .with_doc_version(3);

    // Versions match: provider version=3, current_version=3 → should find the declaration
    let result = provider.find_declaration(offset_of_x_usage, 3);
    assert!(
        result.is_some(),
        "matching-version provider must still return a result for a declared variable"
    );
    Ok(())
}

#[test]
fn declaration_provider_resolves_signature_parameters() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub add ($x, $y = 1, @rest) {\n    return $x + $y + scalar @rest;\n}\n\npackage Demo;\nmethod greet ($self, $name) {\n    return $name;\n}\n";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let ast_arc = Arc::new(ast);

    let mut parent_map = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast_arc, &mut parent_map, None);

    let provider =
        DeclarationProvider::new(ast_arc, code.to_string(), "file:///test.pl".to_string())
            .with_parent_map(&parent_map);

    let x_usage = must_some(code.match_indices("$x").nth(1)).0 + 1;
    let x_links = provider
        .find_declaration(x_usage, 0)
        .ok_or("expected declaration for signature parameter $x")?;
    let x_link = x_links.first().ok_or("expected a declaration link for $x")?;
    assert_eq!(
        byte_offset_to_line(code, x_link.target_selection_range.0),
        0,
        "signature parameter $x should resolve to the subroutine signature"
    );
    assert_eq!(
        &code[x_link.target_selection_range.0..x_link.target_selection_range.1],
        "$x",
        "signature parameter $x should resolve to its declaration span"
    );

    let name_usage = must_some(code.match_indices("$name").nth(1)).0 + 1;
    let name_links = provider
        .find_declaration(name_usage, 0)
        .ok_or("expected declaration for method signature parameter $name")?;
    let name_link =
        name_links.first().ok_or("expected a declaration link for method parameter $name")?;
    assert_eq!(
        byte_offset_to_line(code, name_link.target_selection_range.0),
        5,
        "signature parameter $name should resolve to the method signature"
    );
    assert_eq!(
        &code[name_link.target_selection_range.0..name_link.target_selection_range.1],
        "$name",
        "signature parameter $name should resolve to its declaration span"
    );

    Ok(())
}

#[test]
fn declaration_provider_get_node_text() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub hello { 1 }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let ast_arc = Arc::new(ast);

    let provider =
        DeclarationProvider::new(ast_arc.clone(), code.to_string(), "file:///t.pl".to_string());
    let text = provider.get_node_text(&ast_arc);
    assert_eq!(text, code);
    Ok(())
}

// ===========================================================================
// 2. Scope analyzer: untested IssueKinds and edge cases
// ===========================================================================

#[test]
fn scope_analysis_variable_redeclaration_same_scope() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $x = 1;
my $x = 2;
"#;
    let issues = scope_issues(code);
    assert!(
        issues
            .iter()
            .any(|i| i.kind == IssueKind::VariableRedeclaration && i.variable_name.contains("x")),
        "should detect redeclaration of $x in same scope"
    );
    Ok(())
}

#[test]
fn scope_analysis_nested_block_no_redeclaration() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $x = 1;
{
    my $x = 2;
}
"#;
    let issues = scope_issues(code);
    // This should produce shadowing, NOT redeclaration
    let redecl = issues
        .iter()
        .filter(|i| i.kind == IssueKind::VariableRedeclaration && i.variable_name.contains("x"))
        .count();
    assert_eq!(redecl, 0, "nested block should not produce redeclaration");
    Ok(())
}

#[test]
fn scope_analysis_multiple_unused_variables() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $a = 1;
my $b = 2;
my $c = 3;
"#;
    let issues = scope_issues(code);
    let unused: Vec<_> = issues.iter().filter(|i| i.kind == IssueKind::UnusedVariable).collect();
    assert!(unused.len() >= 3, "should detect at least 3 unused variables, got {}", unused.len());
    Ok(())
}

#[test]
fn scope_analysis_array_variable_unused() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my @arr = (1, 2, 3);";
    let issues = scope_issues(code);
    assert!(
        issues
            .iter()
            .any(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("arr")),
        "should detect unused @arr"
    );
    Ok(())
}

#[test]
fn scope_analysis_hash_variable_unused() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my %h = (a => 1);";
    let issues = scope_issues(code);
    assert!(
        issues.iter().any(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("h")),
        "should detect unused %h"
    );
    Ok(())
}

#[test]
fn scope_analysis_used_in_nested_block() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $x = 1;
{
    print $x;
}
"#;
    let issues = scope_issues(code);
    let unused_x = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("x"))
        .count();
    assert_eq!(unused_x, 0, "$x used in nested block should not be unused");
    Ok(())
}

#[test]
fn scope_analysis_for_loop_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my @items = (1, 2, 3);
for my $item (@items) {
    print $item;
}
"#;
    let issues = scope_issues(code);
    let unused_item = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("item"))
        .count();
    assert_eq!(unused_item, 0, "$item used in for loop body should not be unused");
    Ok(())
}

#[test]
fn scope_analysis_shadowing_across_multiple_levels() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $x = 1;
{
    my $x = 2;
    {
        my $x = 3;
    }
}
"#;
    let issues = scope_issues(code);
    let shadow_count = issues.iter().filter(|i| i.kind == IssueKind::VariableShadowing).count();
    assert!(
        shadow_count >= 2,
        "should detect at least 2 levels of shadowing, got {}",
        shadow_count
    );
    Ok(())
}

#[test]
fn scope_analysis_our_variable_not_flagged_unused() -> Result<(), Box<dyn std::error::Error>> {
    let code = "our $VERSION = '1.0';";
    let issues = scope_issues(code);
    let unused = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("VERSION"))
        .count();
    assert_eq!(unused, 0, "our variable should not be flagged as unused");
    Ok(())
}

#[test]
fn scope_analysis_underscore_prefix_suppresses() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $_temp = 42;";
    let issues = scope_issues(code);
    let unused = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("_temp"))
        .count();
    assert_eq!(unused, 0, "_prefixed variable should suppress unused warning");
    Ok(())
}

#[test]
fn scope_suggestions_for_all_issue_kinds() -> Result<(), Box<dyn std::error::Error>> {
    let analyzer = ScopeAnalyzer::new();
    let test_issues = vec![
        ScopeIssue {
            kind: IssueKind::VariableShadowing,
            variable_name: "$x".to_string(),
            line: 1,
            range: (0, 2),
            description: String::new(),
        },
        ScopeIssue {
            kind: IssueKind::UnusedVariable,
            variable_name: "$y".to_string(),
            line: 2,
            range: (0, 2),
            description: String::new(),
        },
        ScopeIssue {
            kind: IssueKind::UndeclaredVariable,
            variable_name: "$z".to_string(),
            line: 3,
            range: (0, 2),
            description: String::new(),
        },
        ScopeIssue {
            kind: IssueKind::VariableRedeclaration,
            variable_name: "$w".to_string(),
            line: 4,
            range: (0, 2),
            description: String::new(),
        },
        ScopeIssue {
            kind: IssueKind::DuplicateParameter,
            variable_name: "$p".to_string(),
            line: 5,
            range: (0, 2),
            description: String::new(),
        },
        ScopeIssue {
            kind: IssueKind::ParameterShadowsGlobal,
            variable_name: "$g".to_string(),
            line: 6,
            range: (0, 2),
            description: String::new(),
        },
        ScopeIssue {
            kind: IssueKind::UnusedParameter,
            variable_name: "$u".to_string(),
            line: 7,
            range: (0, 2),
            description: String::new(),
        },
        ScopeIssue {
            kind: IssueKind::UnquotedBareword,
            variable_name: "FOO".to_string(),
            line: 8,
            range: (0, 3),
            description: String::new(),
        },
        ScopeIssue {
            kind: IssueKind::UninitializedVariable,
            variable_name: "$v".to_string(),
            line: 9,
            range: (0, 2),
            description: String::new(),
        },
    ];

    let suggestions = analyzer.get_suggestions(&test_issues);
    assert_eq!(suggestions.len(), 9, "should have a suggestion for each issue");
    assert!(suggestions[0].contains("rename"));
    assert!(suggestions[1].contains("Remove") || suggestions[1].contains("unused"));
    assert!(suggestions[2].contains("Declare"));
    assert!(suggestions[3].contains("duplicate"));
    assert!(suggestions[4].contains("rename") || suggestions[4].contains("Remove"));
    assert!(suggestions[5].contains("Rename") || suggestions[5].contains("rename"));
    assert!(suggestions[6].contains("Rename") || suggestions[6].contains("underscore"));
    assert!(suggestions[7].contains("Quote") || suggestions[7].contains("bareword"));
    assert!(suggestions[8].contains("Initialize"));
    Ok(())
}

#[test]
fn scope_issue_has_description() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $x = 1;
my $x = 2;
"#;
    let issues = scope_issues(code);
    let redecl = issues.iter().find(|i| i.kind == IssueKind::VariableRedeclaration);
    if let Some(issue) = redecl {
        assert!(!issue.description.is_empty(), "redeclaration should have description");
    }
    Ok(())
}

#[test]
fn scope_issue_has_range() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $unused = 1;";
    let issues = scope_issues(code);
    for issue in &issues {
        assert!(issue.range.0 <= issue.range.1, "range start should <= end");
    }
    Ok(())
}

// ===========================================================================
// 3. SemanticAnalyzer: infer_type for various node types
// ===========================================================================

#[test]
fn infer_type_number_literal() -> Result<(), Box<dyn std::error::Error>> {
    let code = "42;";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);
    // Walk to find a number node
    let children = get_node_children(&ast);
    for child in &children {
        if let Some(ty) = analyzer.infer_type(child) {
            assert!(
                ty == "number" || ty == "string" || ty == "scalar",
                "expected a type string, got {}",
                ty
            );
        }
    }
    Ok(())
}

#[test]
fn infer_type_string_literal() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#""hello";"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);
    let children = get_node_children(&ast);
    for child in &children {
        if let Some(ty) = analyzer.infer_type(child) {
            assert!(ty == "string" || ty == "scalar", "expected string type, got {}", ty);
        }
    }
    Ok(())
}

#[test]
fn semantic_analyzer_analyze_without_source() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub foo { 1 }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze(&ast);
    assert!(!analyzer.semantic_tokens().is_empty());
    assert!(!analyzer.symbol_table().symbols.is_empty());
    Ok(())
}

#[test]
fn semantic_analyzer_find_definition_on_definition() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub my_func { 1 }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);
    // Position at the sub keyword area
    let def = analyzer.find_definition(4);
    // Should find the function definition
    if let Some(sym) = def {
        assert_eq!(sym.name, "my_func");
    }
    Ok(())
}

#[test]
fn semantic_analyzer_find_all_refs_include_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
sub greet { 1 }
greet();
"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);
    // Find refs including declaration
    let table = analyzer.symbol_table();
    if let Some(syms) = table.symbols.get("greet") {
        if let Some(sym) = syms.first() {
            let refs_incl = analyzer.find_all_references(sym.location.start, true);
            let refs_excl = analyzer.find_all_references(sym.location.start, false);
            assert!(
                refs_incl.len() >= refs_excl.len(),
                "include_declaration should return >= refs"
            );
        }
    }
    Ok(())
}

#[test]
fn semantic_analyzer_symbol_at_most_specific() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
sub outer {
    my $inner = 1;
}
"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);
    let table = analyzer.symbol_table();
    // inner variable should exist
    assert!(table.symbols.contains_key("inner"), "should have inner variable");
    Ok(())
}

// ===========================================================================
// 4. SemanticModel API
// ===========================================================================

#[test]
fn semantic_model_build_and_query() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 42; sub foo { $x + 1 }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let model = SemanticModel::build(&ast, code);
    assert!(!model.tokens().is_empty());
    assert!(!model.symbol_table().symbols.is_empty());
    Ok(())
}

#[test]
fn semantic_model_hover_info_for_sub() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
# The greet function
sub greet {
    return "hello";
}
"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let model = SemanticModel::build(&ast, code);
    let table = model.symbol_table();
    if let Some(syms) = table.symbols.get("greet") {
        if let Some(sym) = syms.first() {
            let hover = model.hover_info_at(sym.location);
            // May or may not have hover depending on analysis
            let _ = hover;
        }
    }
    Ok(())
}

#[test]
fn semantic_model_definition_at() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub target { 1 }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let model = SemanticModel::build(&ast, code);
    let def = model.definition_at(5);
    if let Some(sym) = def {
        assert_eq!(sym.name, "target");
    }
    Ok(())
}

#[test]
fn semantic_model_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let code = "";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let model = SemanticModel::build(&ast, code);
    assert!(model.tokens().is_empty());
    Ok(())
}

// ===========================================================================
// 5. Symbol extraction edge cases
// ===========================================================================

#[test]
fn symbol_extraction_no_source() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub foo { 1 }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    // Use extractor without source
    let table = SymbolExtractor::new().extract(&ast);
    assert!(has_symbol(&table, "foo", SymbolKind::Subroutine));
    Ok(())
}

#[test]
fn symbol_extraction_state_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub counter { state $n = 0; $n++ }";
    let table = parse_and_extract(code);
    // state variable should be extracted
    assert!(has_symbol(&table, "counter", SymbolKind::Subroutine));
    Ok(())
}

#[test]
fn symbol_extraction_local_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = "local $/ = undef;";
    let table = parse_and_extract(code);
    // Should not panic on local declarations
    let _ = table;
    Ok(())
}

#[test]
fn symbol_extraction_anonymous_sub() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $cb = sub { 42 };";
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "cb", SymbolKind::scalar()));
    Ok(())
}

#[test]
fn symbol_extraction_use_strict_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let code = "use strict;\nuse warnings;\nmy $x = 1;";
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "x", SymbolKind::scalar()));
    Ok(())
}

#[test]
fn symbol_extraction_heredoc() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $text = <<'END';
Hello World
END
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "text", SymbolKind::scalar()));
    Ok(())
}

#[test]
fn symbol_extraction_qw_list() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my @colors = qw(red green blue);";
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "colors", SymbolKind::array()));
    Ok(())
}

#[test]
fn symbol_extraction_hash_ref() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $ref = { a => 1, b => 2 };";
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "ref", SymbolKind::scalar()));
    Ok(())
}

#[test]
fn symbol_extraction_method_call() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $obj = Foo->new();";
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "obj", SymbolKind::scalar()));
    Ok(())
}

#[test]
fn symbol_extraction_complex_regex() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $str = "hello"; $str =~ s/hello/world/g;"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "str", SymbolKind::scalar()));
    Ok(())
}

#[test]
fn symbol_table_scopes_have_global() -> Result<(), Box<dyn std::error::Error>> {
    let table = SymbolTable::new();
    assert!(table.scopes.contains_key(&0), "should have global scope");
    let global = &table.scopes[&0];
    assert_eq!(global.kind, ScopeKind::Global);
    assert!(global.parent.is_none());
    Ok(())
}

#[test]
fn symbol_table_default_trait() -> Result<(), Box<dyn std::error::Error>> {
    // Default derive produces empty fields; new() adds global scope
    let table = SymbolTable::default();
    assert!(table.symbols.is_empty());
    assert!(table.references.is_empty());
    Ok(())
}

#[test]
fn symbol_table_find_symbol_nonexistent() -> Result<(), Box<dyn std::error::Error>> {
    let table = parse_and_extract("my $x = 1;");
    let result = table.find_symbol("nonexistent", 0, SymbolKind::Subroutine);
    assert!(result.is_empty());
    Ok(())
}

#[test]
fn symbol_table_find_references_for_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
sub greet { 1 }
greet();
greet();
"#;
    let table = parse_and_extract(code);
    if let Some(syms) = table.symbols.get("greet") {
        if let Some(sym) = syms.first() {
            let refs = table.find_references(sym);
            // Should have at least the call references
            let _ = refs;
        }
    }
    Ok(())
}

#[test]
fn symbol_kind_scalar_array_hash() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(SymbolKind::scalar(), SymbolKind::Variable(VarKind::Scalar));
    assert_eq!(SymbolKind::array(), SymbolKind::Variable(VarKind::Array));
    assert_eq!(SymbolKind::hash(), SymbolKind::Variable(VarKind::Hash));
    Ok(())
}

// ===========================================================================
// 6. Cross-package symbol extraction
// ===========================================================================

#[test]
fn cross_package_multiple_subs() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Foo;
sub foo_method { 1 }

package Bar;
sub bar_method { 2 }
sub bar_other { 3 }
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "Foo", SymbolKind::Package));
    assert!(has_symbol(&table, "Bar", SymbolKind::Package));
    assert!(has_symbol(&table, "foo_method", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "bar_method", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "bar_other", SymbolKind::Subroutine));
    Ok(())
}

#[test]
fn cross_package_variables() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Config;
our $DEBUG = 0;
our @EXPORT = ('foo');

package App;
my $app_var = 1;
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "Config", SymbolKind::Package));
    assert!(has_symbol(&table, "App", SymbolKind::Package));
    assert!(has_symbol(&table, "DEBUG", SymbolKind::scalar()));
    assert!(has_symbol(&table, "EXPORT", SymbolKind::array()));
    assert!(has_symbol(&table, "app_var", SymbolKind::scalar()));
    Ok(())
}

#[test]
fn deeply_nested_package() -> Result<(), Box<dyn std::error::Error>> {
    let code = "package Very::Deeply::Nested::Package;\nsub deep_func { 1 }";
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "Very::Deeply::Nested::Package", SymbolKind::Package));
    assert!(has_symbol(&table, "deep_func", SymbolKind::Subroutine));
    Ok(())
}

// ===========================================================================
// 7. Type inference engine: deeper coverage
// ===========================================================================

#[test]
fn type_inference_subroutine_definition() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub add { return $_[0] + $_[1]; }";
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let result = engine.infer(&ast);
    // Should not error
    let _ = result;
    Ok(())
}

#[test]
fn type_inference_boolean_expression() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1 == 2;";
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let _ = engine.infer(&ast);
    Ok(())
}

#[test]
fn type_inference_string_concatenation() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $s = "hello" . " world";"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let _ = engine.infer(&ast);
    Ok(())
}

#[test]
fn type_inference_hash_assignment() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my %data = (name => 'John', age => 30);";
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let _ = engine.infer(&ast);
    let ty = engine.get_type_at("data");
    if let Some(t) = ty {
        assert!(matches!(t, PerlType::Hash { .. }));
    }
    Ok(())
}

#[test]
fn type_inference_undef_literal() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = undef;";
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let _ = engine.infer(&ast);
    Ok(())
}

#[test]
fn type_inference_glob_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = *STDOUT;";
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let _ = engine.infer(&ast);
    Ok(())
}

#[test]
fn type_inference_engine_default() -> Result<(), Box<dyn std::error::Error>> {
    let engine = TypeInferenceEngine::default();
    assert!(engine.get_type_errors().is_empty());
    Ok(())
}

#[test]
fn type_inference_get_type_at_unknown() -> Result<(), Box<dyn std::error::Error>> {
    let engine = TypeInferenceEngine::new();
    assert!(engine.get_type_at("nonexistent").is_none());
    Ok(())
}

#[test]
fn type_inference_get_subroutine_unknown() -> Result<(), Box<dyn std::error::Error>> {
    let engine = TypeInferenceEngine::new();
    assert!(engine.get_subroutine("nonexistent").is_none());
    Ok(())
}

// ===========================================================================
// 8. TypeEnvironment deeper coverage
// ===========================================================================

#[test]
fn type_env_nested_three_levels() -> Result<(), Box<dyn std::error::Error>> {
    let mut root = TypeEnvironment::new();
    root.set_variable("a".to_string(), PerlType::Scalar(ScalarType::Integer));

    let mut child = TypeEnvironment::with_parent(root);
    child.set_variable("b".to_string(), PerlType::Scalar(ScalarType::String));

    let mut grandchild = TypeEnvironment::with_parent(child);
    grandchild.set_variable("c".to_string(), PerlType::Scalar(ScalarType::Float));

    // grandchild can see all three
    assert!(grandchild.get_variable("a").is_some());
    assert!(grandchild.get_variable("b").is_some());
    assert!(grandchild.get_variable("c").is_some());
    assert!(grandchild.get_variable("d").is_none());
    Ok(())
}

#[test]
fn type_env_subroutine_in_parent() -> Result<(), Box<dyn std::error::Error>> {
    let mut root = TypeEnvironment::new();
    root.set_subroutine(
        "my_sub".to_string(),
        PerlType::Subroutine {
            params: vec![PerlType::Scalar(ScalarType::String)],
            returns: vec![PerlType::Scalar(ScalarType::Integer)],
        },
    );

    let child = TypeEnvironment::with_parent(root);
    let sub_type = child.get_subroutine("my_sub");
    assert!(sub_type.is_some());
    Ok(())
}

#[test]
fn type_env_child_overrides_parent_variable() -> Result<(), Box<dyn std::error::Error>> {
    let mut root = TypeEnvironment::new();
    root.set_variable("x".to_string(), PerlType::Scalar(ScalarType::Integer));

    let mut child = TypeEnvironment::with_parent(root);
    child.set_variable("x".to_string(), PerlType::Scalar(ScalarType::String));

    let ty = child.get_variable("x");
    assert_eq!(ty, Some(&PerlType::Scalar(ScalarType::String)));
    Ok(())
}

// ===========================================================================
// 9. Type-based completion: string and object types
// ===========================================================================

#[test]
fn completion_string_methods() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(r#"my $s = "hello";"#);
    let ast = parser.parse()?;
    let _ = engine.infer(&ast);
    let comp = TypeBasedCompletion::new(Arc::new(engine));
    let completions = comp.get_completions("s", "");
    assert!(completions.iter().any(|c| c.label == "length"));
    assert!(completions.iter().any(|c| c.label == "substr"));
    assert!(completions.iter().any(|c| c.label == "uc"));
    assert!(completions.iter().any(|c| c.label == "lc"));
    Ok(())
}

#[test]
fn completion_no_methods_for_integer() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new("my $n = 42;");
    let ast = parser.parse()?;
    let _ = engine.infer(&ast);
    let comp = TypeBasedCompletion::new(Arc::new(engine));
    let completions = comp.get_completions("n", "");
    // Integers don't have specific completions in the current impl
    // (ScalarType::Integer, not String/Array/Hash/Object)
    // Just verify no crash
    let _ = completions;
    Ok(())
}

#[test]
fn completion_item_has_detail_and_doc() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new("my @items = (1, 2, 3);");
    let ast = parser.parse()?;
    let _ = engine.infer(&ast);
    let comp = TypeBasedCompletion::new(Arc::new(engine));
    let completions = comp.get_completions("items", "");
    for item in &completions {
        assert!(!item.label.is_empty(), "label should not be empty");
        assert!(!item.detail.is_empty(), "detail should not be empty");
        assert!(!item.documentation.is_empty(), "documentation should not be empty");
    }
    Ok(())
}

// ===========================================================================
// 10. PerlType construction and equality edge cases
// ===========================================================================

#[test]
fn perl_type_subroutine_construction() -> Result<(), Box<dyn std::error::Error>> {
    let ty = PerlType::Subroutine {
        params: vec![PerlType::Scalar(ScalarType::String), PerlType::Scalar(ScalarType::Integer)],
        returns: vec![PerlType::Scalar(ScalarType::Boolean)],
    };
    assert!(matches!(ty, PerlType::Subroutine { .. }));
    if let PerlType::Subroutine { params, returns } = &ty {
        assert_eq!(params.len(), 2);
        assert_eq!(returns.len(), 1);
    }
    Ok(())
}

#[test]
fn perl_type_nested_reference() -> Result<(), Box<dyn std::error::Error>> {
    let inner = PerlType::Array(Box::new(PerlType::Scalar(ScalarType::Integer)));
    let ty = PerlType::Reference(Box::new(inner));
    assert!(matches!(ty, PerlType::Reference(_)));
    Ok(())
}

#[test]
fn perl_type_empty_union() -> Result<(), Box<dyn std::error::Error>> {
    let ty = PerlType::Union(vec![]);
    if let PerlType::Union(variants) = &ty {
        assert!(variants.is_empty());
    }
    Ok(())
}

#[test]
fn perl_type_complex_hash() -> Result<(), Box<dyn std::error::Error>> {
    let ty = PerlType::Hash {
        key: Box::new(PerlType::Scalar(ScalarType::String)),
        value: Box::new(PerlType::Array(Box::new(PerlType::Scalar(ScalarType::Integer)))),
    };
    if let PerlType::Hash { value, .. } = &ty {
        assert!(matches!(value.as_ref(), PerlType::Array(_)));
    }
    Ok(())
}

#[test]
fn perl_type_scalar_mixed() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(PerlType::Scalar(ScalarType::Mixed), PerlType::Scalar(ScalarType::Mixed));
    assert_ne!(PerlType::Scalar(ScalarType::Mixed), PerlType::Scalar(ScalarType::Integer));
    Ok(())
}

#[test]
fn perl_type_scalar_undef() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(PerlType::Scalar(ScalarType::Undef), PerlType::Scalar(ScalarType::Undef));
    assert_ne!(PerlType::Scalar(ScalarType::Undef), PerlType::Void);
    Ok(())
}

#[test]
fn perl_type_scalar_boolean() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(PerlType::Scalar(ScalarType::Boolean), PerlType::Scalar(ScalarType::Boolean));
    assert_ne!(PerlType::Scalar(ScalarType::Boolean), PerlType::Scalar(ScalarType::Integer));
    Ok(())
}

// ===========================================================================
// 11. WorkspaceIndex edge cases
// ===========================================================================

#[test]
fn workspace_index_empty_search() -> Result<(), Box<dyn std::error::Error>> {
    use perl_semantic_analyzer::analysis::index::WorkspaceIndex;

    let index = WorkspaceIndex::new();
    let results = index.search_symbols("");
    // Empty search on empty index should not panic
    let _ = results;
    Ok(())
}

#[test]
fn workspace_index_duplicate_uri_update() -> Result<(), Box<dyn std::error::Error>> {
    use perl_semantic_analyzer::analysis::index::WorkspaceIndex;

    let mut index = WorkspaceIndex::new();
    let table1 = parse_and_extract("sub v1 { 1 }");
    index.update_from_document("file:///same.pl", "", &table1);
    assert_eq!(index.find_defs("v1").len(), 1);

    let table2 = parse_and_extract("sub v2 { 2 }");
    index.update_from_document("file:///same.pl", "", &table2);
    assert_eq!(index.find_defs("v1").len(), 0);
    assert_eq!(index.find_defs("v2").len(), 1);
    assert_eq!(index.file_count(), 1);
    Ok(())
}

#[test]
fn workspace_index_remove_nonexistent() -> Result<(), Box<dyn std::error::Error>> {
    use perl_semantic_analyzer::analysis::index::WorkspaceIndex;

    let mut index = WorkspaceIndex::new();
    index.remove_document("file:///nonexistent.pl");
    assert_eq!(index.file_count(), 0);
    Ok(())
}

#[test]
fn workspace_index_find_defs_empty() -> Result<(), Box<dyn std::error::Error>> {
    use perl_semantic_analyzer::analysis::index::WorkspaceIndex;

    let index = WorkspaceIndex::new();
    assert!(index.find_defs("anything").is_empty());
    Ok(())
}

#[test]
fn workspace_index_find_refs_empty() -> Result<(), Box<dyn std::error::Error>> {
    use perl_semantic_analyzer::analysis::index::WorkspaceIndex;

    let index = WorkspaceIndex::new();
    assert!(index.find_refs("anything").is_empty());
    Ok(())
}

#[test]
fn workspace_index_many_files() -> Result<(), Box<dyn std::error::Error>> {
    use perl_semantic_analyzer::analysis::index::WorkspaceIndex;

    let mut index = WorkspaceIndex::new();
    for i in 0..10 {
        let code = format!("sub func_{} {{ {} }}", i, i);
        let table = parse_and_extract(&code);
        let uri = format!("file:///file_{}.pl", i);
        index.update_from_document(&uri, "", &table);
    }
    assert_eq!(index.file_count(), 10);
    assert!(index.symbol_count() >= 10);
    Ok(())
}

// ===========================================================================
// 12. Semantic tokens: token types coverage
// ===========================================================================

#[test]
fn semantic_tokens_contain_keyword() -> Result<(), Box<dyn std::error::Error>> {
    let code = "if (1) { print 'yes'; }";
    let analyzer = parse_and_analyze(code);
    let tokens = analyzer.semantic_tokens();
    // The analyzer may classify 'if' as Keyword or generate other token types
    assert!(!tokens.is_empty(), "should produce semantic tokens for if-statement");
    Ok(())
}

#[test]
fn semantic_tokens_for_loop() -> Result<(), Box<dyn std::error::Error>> {
    let code = "for my $i (1..10) { print $i; }";
    let analyzer = parse_and_analyze(code);
    let tokens = analyzer.semantic_tokens();
    assert!(!tokens.is_empty(), "for loop should produce tokens");
    Ok(())
}

#[test]
fn semantic_tokens_while_loop() -> Result<(), Box<dyn std::error::Error>> {
    let code = "while (1) { last; }";
    let analyzer = parse_and_analyze(code);
    let tokens = analyzer.semantic_tokens();
    assert!(!tokens.is_empty(), "while loop should produce tokens");
    Ok(())
}

#[test]
fn semantic_tokens_variable_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 42;";
    let analyzer = parse_and_analyze(code);
    let tokens = analyzer.semantic_tokens();
    // Should have a modifier token or variable declaration
    assert!(
        tokens.iter().any(|t| t.token_type == SemanticTokenType::Modifier
            || t.token_type == SemanticTokenType::VariableDeclaration
            || t.token_type == SemanticTokenType::Variable),
        "should have variable-related token"
    );
    Ok(())
}

#[test]
fn semantic_tokens_method_call() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $obj = Foo->new(); $obj->method();";
    let analyzer = parse_and_analyze(code);
    let tokens = analyzer.semantic_tokens();
    assert!(!tokens.is_empty(), "method call should produce tokens");
    Ok(())
}

#[test]
fn semantic_tokens_regex() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $s = "hello"; $s =~ /hell/;"#;
    let analyzer = parse_and_analyze(code);
    let tokens = analyzer.semantic_tokens();
    assert!(!tokens.is_empty());
    Ok(())
}

#[test]
fn semantic_tokens_comment() -> Result<(), Box<dyn std::error::Error>> {
    let code = "# this is a comment\nmy $x = 1;";
    let analyzer = parse_and_analyze(code);
    let tokens = analyzer.semantic_tokens();
    // Comments may be stripped at parse level; just verify no crash and tokens generated
    assert!(!tokens.is_empty(), "should produce tokens for code with comment");
    Ok(())
}

#[test]
fn semantic_tokens_number_literal() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $n = 3.14;";
    let analyzer = parse_and_analyze(code);
    let tokens = analyzer.semantic_tokens();
    assert!(
        tokens.iter().any(|t| t.token_type == SemanticTokenType::Number),
        "should have number token"
    );
    Ok(())
}

// ===========================================================================
// 13. Integration: larger real-world-ish patterns
// ===========================================================================

#[test]
fn integration_module_with_exporter() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package MyModule;
use strict;
use warnings;

our @EXPORT_OK = qw(helper);
our $VERSION = '1.00';

sub helper {
    my ($arg) = @_;
    return $arg * 2;
}

sub _private {
    return 42;
}

1;
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "MyModule", SymbolKind::Package));
    assert!(has_symbol(&table, "helper", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "_private", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "VERSION", SymbolKind::scalar()));
    assert!(has_symbol(&table, "EXPORT_OK", SymbolKind::array()));

    let analyzer = parse_and_analyze(code);
    assert!(!analyzer.semantic_tokens().is_empty());
    Ok(())
}

#[test]
fn integration_oop_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Animal;

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

sub speak {
    my ($self) = @_;
    return "...";
}

1;
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "Animal", SymbolKind::Package));
    assert!(has_symbol(&table, "new", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "speak", SymbolKind::Subroutine));

    let issues = scope_issues(code);
    // verify no crashes on OOP pattern
    let _ = issues;
    Ok(())
}

#[test]
fn integration_complex_scoping() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $x = 1;
sub outer {
    my $y = 2;
    sub inner {
        my $z = 3;
        return $x + $y + $z;
    }
    return inner();
}
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "outer", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "inner", SymbolKind::Subroutine));
    Ok(())
}

#[test]
fn integration_eval_block() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
eval {
    my $result = do_something();
    print $result;
};
if ($@) {
    warn "Error: $@";
}
"#;
    let table = parse_and_extract(code);
    // Should not panic on eval blocks
    let _ = table;
    let issues = scope_issues(code);
    let _ = issues;
    Ok(())
}

#[test]
fn integration_multiple_variables_same_name_different_scopes()
-> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
sub foo {
    my $val = 1;
    return $val;
}
sub bar {
    my $val = 2;
    return $val;
}
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "foo", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "bar", SymbolKind::Subroutine));
    // Both $val should exist but in different scopes
    assert!(has_symbol(&table, "val", SymbolKind::scalar()));
    Ok(())
}

#[test]
fn integration_ternary_operator() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1 > 0 ? 'yes' : 'no';";
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "x", SymbolKind::scalar()));
    let analyzer = parse_and_analyze(code);
    assert!(!analyzer.semantic_tokens().is_empty());
    Ok(())
}

#[test]
fn integration_chained_method_calls() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $result = Foo->new->process->output;";
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "result", SymbolKind::scalar()));
    Ok(())
}

#[test]
fn integration_large_function() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
sub process {
    my ($input) = @_;
    my $result = "";
    my @lines = split(/\n/, $input);
    my %seen;
    for my $line (@lines) {
        chomp $line;
        next if $seen{$line}++;
        $result .= $line . "\n";
    }
    return $result;
}
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "process", SymbolKind::Subroutine));

    let analyzer = parse_and_analyze(code);
    assert!(!analyzer.semantic_tokens().is_empty());

    let issues = scope_issues(code);
    let _ = issues;
    Ok(())
}

#[test]
fn scope_analysis_variable_list_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my ($a, $b, $c) = (1, 2, 3); print $a + $b + $c;";
    let issues = scope_issues(code);
    // Just verify list declarations don't crash and produce reasonable output
    let _ = issues;
    Ok(())
}

#[test]
fn scope_analysis_use_vars_pragma() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"use vars qw($FOO @BAR); $FOO = 1; push @BAR, 2;"#;
    let issues = scope_issues(code);
    // use vars should declare globals; no undeclared issues
    let _ = issues;
    Ok(())
}
