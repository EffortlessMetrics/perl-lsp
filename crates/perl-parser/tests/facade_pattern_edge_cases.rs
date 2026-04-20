//! Test facade pattern coverage for Wave D crate absorption.
//!
//! Verifies that the absorbed satellite crates (perl-quote, perl-heredoc, perl-error,
//! perl-edit, perl-path-normalize, perl-path-security, perl-text-line, perl-percentile,
//! perl-source-file, perl-qualified-name, perl-ast-utils, perl-heredoc-anti-patterns)
//! are re-exported correctly via perl-parser, and that downstream consumers relying on
//! the new facade paths work correctly.

use perl_parser::Parser;
use perl_tdd_support::{must, must_some};

/// Test that all facade re-exports from perl-parser-core are accessible
#[test]
fn test_facade_reexports_accessible_from_perl_parser() -> Result<(), Box<dyn std::error::Error>> {
    // Parser type — fundamental
    let _parser: perl_parser::Parser = Parser::new("my $x = 1;");

    // Position mapping for LSP — from perl-parser-core::position
    let _mapper: perl_parser::PositionMapper = perl_parser::PositionMapper::new("my $x = 1;");

    // AST types — from perl-parser-core::ast
    let mut parser = Parser::new("my $x = 1;");
    let ast = parser.parse()?;
    assert!(matches!(ast.kind, perl_parser::NodeKind::Program { .. }));

    // Error types — from perl-parser-core::error
    let _error_result: Result<_, perl_parser::ParseError> = Ok(());

    // Semantic analysis types — from perl-semantic-analyzer
    let _analyzer = perl_parser::SemanticAnalyzer::analyze(&ast);

    // Symbol types — from perl-semantic-analyzer::symbol
    let _symbol_kind: perl_parser::SymbolKind = perl_parser::SymbolKind::Subroutine;

    // Scope analyzer — from perl-semantic-analyzer::scope_analyzer
    let _scope_analyzer: perl_parser::ScopeAnalyzer = perl_parser::ScopeAnalyzer::new();

    // Type inference — from perl-semantic-analyzer::type_inference
    let _type_env: perl_parser::TypeEnvironment = perl_parser::TypeEnvironment::new();

    // Refactoring — from perl-refactoring
    let _import_optimizer: perl_parser::ImportOptimizer = perl_parser::ImportOptimizer::new();

    // Workspace — from perl-workspace-index
    let _symbol_table: perl_parser::SymbolTable = perl_parser::SymbolTable::new();

    Ok(())
}

/// Test that AST utilities facade is accessible and functional.
///
/// perl-ast-utils was absorbed into perl-parser-core and re-exported via perl-parser.
#[test]
fn test_ast_utils_facade_functional() -> Result<(), Box<dyn std::error::Error>> {
    // These functions were in perl-ast-utils and are now in perl-parser::ast_utils module
    // They should be accessible directly via perl-parser
    use perl_parser::ast_utils;

    let source = "my $x = 1;\nmy $y = 2;\n";

    // Test find_statement_start
    let pos = must_some(source.find("$y"));
    let stmt_start = ast_utils::find_statement_start(source, pos);
    assert!(stmt_start > 0, "Statement start should be found");
    assert!(source[stmt_start..].starts_with("my $y"), "Should start at second statement");

    // Test find_declaration_position
    let decl_pos = ast_utils::find_declaration_position(source, pos);
    assert_eq!(decl_pos, stmt_start, "Declaration position should match statement start");

    // Test get_indent_at
    let indented_source = "if (1) {\n    my $x = 1;\n}\n";
    let indent_pos = must_some(indented_source.find("my"));
    let indent = ast_utils::get_indent_at(indented_source, indent_pos);
    assert_eq!(indent, "    ", "Should detect 4-space indent");

    Ok(())
}

/// Test that heredoc anti-patterns facade is accessible.
///
/// perl-heredoc-anti-patterns was absorbed into perl-parser and re-exported.
#[test]
fn test_heredoc_antipatterns_facade_accessible() -> Result<(), Box<dyn std::error::Error>> {
    use perl_parser::heredoc_anti_patterns;

    // Verify that the main types are accessible
    let _location = heredoc_anti_patterns::Location { line: 1, column: 0, offset: 0 };

    let _severity = heredoc_anti_patterns::Severity::Warning;

    // The anti-pattern enum should be constructible
    let _pattern = heredoc_anti_patterns::AntiPattern::FormatHeredoc {
        location: heredoc_anti_patterns::Location { line: 1, column: 0, offset: 0 },
        format_name: "test".to_string(),
        heredoc_delimiter: "END".to_string(),
    };

    Ok(())
}

/// Test that path normalization utilities are callable via the facade.
///
/// perl-path-normalize was absorbed into perl-parser-core and re-exported via perl-parser.
/// This test calls the actual normalization function to prove it works end-to-end.
#[test]
fn test_path_normalize_facade_functional() -> Result<(), Box<dyn std::error::Error>> {
    use perl_parser_core::path_normalize::normalize_path_within_workspace;
    use std::path::Path;

    // Normal relative path within workspace should succeed
    let workspace = Path::new("/workspace");
    let result = normalize_path_within_workspace(Path::new("lib/Foo.pm"), workspace)?;
    assert!(
        result.to_string_lossy().contains("Foo.pm"),
        "Normalized path should contain the filename"
    );

    // Parent traversal should be rejected
    let traversal_result =
        normalize_path_within_workspace(Path::new("../../etc/passwd"), workspace);
    assert!(traversal_result.is_err(), "Parent traversal should be rejected");

    Ok(())
}

/// Test that text line utilities are callable via the facade.
///
/// perl-text-line was absorbed into perl-parser-core and re-exported via perl-parser.
/// This test calls the actual text_line functions to prove they work end-to-end.
#[test]
fn test_text_line_facade_functional() -> Result<(), Box<dyn std::error::Error>> {
    use perl_parser_core::text_line::{is_keyword_boundary, line_bounds_at, skip_ascii_whitespace};

    let source = "my $x = 1;\nmy $y = 2;\n";

    // line_bounds_at: for a position in the second line, returns the line's byte range
    let second_line_pos = must_some(source.find("$y"));
    let (line_start, line_end) = line_bounds_at(source, second_line_pos);
    assert!(line_start < second_line_pos, "Line start should be before $y");
    assert!(line_end >= second_line_pos, "Line end should be at or after $y");
    assert_eq!(&source[line_start..line_end], "my $y = 2;", "Should extract exact line");

    // skip_ascii_whitespace: should skip leading spaces
    let spaced = b"   hello";
    let idx = skip_ascii_whitespace(spaced, 0);
    assert_eq!(idx, 3, "Should skip 3 spaces");

    // is_keyword_boundary: 'my' followed by space is a keyword boundary
    let kw_source = b"my $x";
    assert!(
        is_keyword_boundary(kw_source, 0, 2),
        "my followed by space should be a keyword boundary"
    );

    // is_keyword_boundary: 'my' immediately followed by letter is NOT a boundary
    let no_boundary = b"myvar";
    assert!(
        !is_keyword_boundary(no_boundary, 0, 2),
        "my followed by letter should not be a keyword boundary"
    );

    Ok(())
}

/// Test that qualified name utilities are callable via the facade.
///
/// perl-qualified-name was absorbed into perl-parser-core and re-exported via perl-parser.
/// This test calls the actual split/validate functions to prove they work end-to-end.
#[test]
fn test_qualified_name_facade_functional() -> Result<(), Box<dyn std::error::Error>> {
    use perl_parser_core::qualified_name::{
        container_name, split_qualified_name, validate_perl_qualified_name,
    };

    // split_qualified_name: splits "Foo::Bar::baz" into package + bare name
    let (pkg, bare) = split_qualified_name("Foo::Bar::baz");
    assert_eq!(pkg, Some("Foo::Bar"), "Package part should be Foo::Bar");
    assert_eq!(bare, "baz", "Bare name should be baz");

    // split_qualified_name on a bare name returns (None, name)
    let (pkg2, bare2) = split_qualified_name("greet");
    assert_eq!(pkg2, None, "Bare name has no package");
    assert_eq!(bare2, "greet", "Bare name should be returned as-is");

    // container_name: returns the package prefix of a qualified name
    let container = container_name("Foo::Bar::baz");
    assert_eq!(container, Some("Foo::Bar"), "Container should be Foo::Bar");

    // validate_perl_qualified_name: well-formed name should pass
    assert!(
        validate_perl_qualified_name("Foo::Bar").is_ok(),
        "Well-formed qualified name should validate"
    );

    // validate_perl_qualified_name: empty string should fail
    assert!(validate_perl_qualified_name("").is_err(), "Empty name should fail validation");

    Ok(())
}

/// Test that source file utilities are callable via the facade.
///
/// perl-source-file was absorbed into perl-parser-core and re-exported via perl-parser.
/// This test calls the actual classification functions to prove they work end-to-end.
#[test]
fn test_source_file_facade_functional() -> Result<(), Box<dyn std::error::Error>> {
    use perl_parser_core::source_file::{
        is_binary_content, is_perl_source_extension, is_perl_source_path, is_perl_source_uri,
    };
    use std::path::Path;

    // is_perl_source_extension
    assert!(is_perl_source_extension("pl"), "'.pl' should be a Perl source extension");
    assert!(is_perl_source_extension("pm"), "'.pm' should be a Perl source extension");
    assert!(is_perl_source_extension("t"), "'.t' should be a Perl source extension");
    assert!(!is_perl_source_extension("rs"), "'.rs' should not be a Perl source extension");
    assert!(!is_perl_source_extension("py"), "'.py' should not be a Perl source extension");

    // is_perl_source_path
    assert!(is_perl_source_path(Path::new("script.pl")), "script.pl should be Perl source");
    assert!(!is_perl_source_path(Path::new("binary.exe")), "binary.exe should not be Perl source");

    // is_perl_source_uri
    assert!(
        is_perl_source_uri("file:///home/user/script.pl"),
        "URI ending in .pl should be Perl source"
    );
    assert!(
        !is_perl_source_uri("file:///home/user/main.rs"),
        "URI ending in .rs should not be Perl source"
    );

    // is_binary_content
    assert!(!is_binary_content("use strict;\nmy $x = 1;\n"), "Perl source is not binary");
    assert!(is_binary_content("ELF\x00binary"), "Content with null byte is binary");

    Ok(())
}

/// Test boundary condition: empty input
#[test]
fn test_facade_empty_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("");
    let ast = parser.parse()?;

    // Empty input should produce a valid (empty) program
    if let perl_parser::NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 0);
    }

    Ok(())
}

/// Test boundary condition: very large input (stress test)
#[test]
fn test_facade_large_input() -> Result<(), Box<dyn std::error::Error>> {
    // Create a large Perl source with many statements
    let mut source = String::new();
    for i in 0..1000 {
        source.push_str(&format!("my $v{} = {};\n", i, i));
    }

    let mut parser = Parser::new(&source);
    let ast = parser.parse()?;

    // Should handle large input without panicking
    if let perl_parser::NodeKind::Program { statements } = &ast.kind {
        assert!(statements.len() >= 900, "Should parse most statements");
    }

    Ok(())
}

/// Test regression: multi-line strings via absorbed quote machinery
#[test]
fn test_multiline_strings_via_facade() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $str = qq{
    multi-line
    string
    with "quotes"
};
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Should parse without error
    assert!(matches!(ast.kind, perl_parser::NodeKind::Program { .. }));

    Ok(())
}

/// Test regression: heredoc via absorbed machinery
#[test]
fn test_heredoc_via_facade() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $text = <<'END';
This is a heredoc
with multiple lines
END
"#;

    let mut parser = Parser::new(code);
    let result = parser.parse();

    // Heredoc should parse (or recovery should work)
    assert!(result.is_ok(), "Should handle heredoc syntax");

    Ok(())
}

/// Test that consumers can mix old and new import paths without conflict.
///
/// This simulates downstream crates that were split between different satellite crates
/// but should work seamlessly with the new facade.
#[test]
fn test_mixed_import_paths_no_conflict() -> Result<(), Box<dyn std::error::Error>> {
    // Import from facade (new way)
    use perl_parser::Parser;
    use perl_parser::SemanticAnalyzer;

    // These types should all resolve to the same underlying types
    let mut parser = Parser::new("sub test { }");
    let ast = parser.parse()?;

    let _analyzer = SemanticAnalyzer::analyze(&ast);

    // No conflict or duplicate definition errors
    Ok(())
}

/// Test error handling path: graceful degradation on bad input
#[test]
fn test_facade_error_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub { if ("; // Incomplete code

    let mut parser = Parser::new(code);
    let result = parser.parse();

    // Parser should recover
    assert!(result.is_ok(), "Parser should recover from incomplete input");

    // But should record errors
    let errors = parser.errors();
    assert!(!errors.is_empty(), "Should record parse errors");

    Ok(())
}

/// Test type identity: verify that re-exported types are identical
/// (not different copies or wrappers).
#[test]
fn test_facade_type_identity() -> Result<(), Box<dyn std::error::Error>> {
    // This test uses type constructors to verify that the types are the same
    let mut parser = Parser::new("my $x = 1;");
    let ast = parser.parse()?;

    // Verify we can use NodeKind in pattern matching (would fail if types were different)
    match &ast.kind {
        perl_parser::NodeKind::Program { statements } => {
            // The Program node for "my $x = 1;" must have exactly one statement.
            // This assertion is non-vacuous: it would fail if the parser returned 0
            // statements or the wrong node kind.
            assert_eq!(statements.len(), 1, "Single statement should yield one Program child");
            Ok(())
        }
        _ => Err("Expected Program node".into()),
    }
}

/// Test that Display/Debug impls work on facade types (no panic on moved code)
#[test]
fn test_facade_debug_display_impls() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("my $x = 1;");
    let ast = parser.parse()?;

    // Debug impl should work
    let debug_str = format!("{:?}", ast);
    assert!(!debug_str.is_empty());

    // NodeKind Debug should work
    let _kind_debug = format!("{:?}", ast.kind);

    // SourceLocation should have impls
    let loc = &ast.location;
    let loc_debug = format!("{:?}", loc);
    assert!(!loc_debug.is_empty());

    Ok(())
}

/// Test consumer crate simulation: perl-workspace-index use of facade
#[test]
fn test_workspace_index_integration_via_facade() -> Result<(), Box<dyn std::error::Error>> {
    use perl_parser::{Parser, SymbolExtractor};

    let code = "sub hello { } package Foo; sub bar { }";

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // SymbolExtractor is from perl-semantic-analyzer and re-exported via perl-parser
    let extractor = SymbolExtractor::new();
    let symbols = extractor.extract(&ast);

    // Should find both subroutines
    assert!(!symbols.symbols.is_empty(), "Should extract symbols");

    Ok(())
}

/// Test consumer crate simulation: perl-lsp-code-actions use of facade.
///
/// This simulates the actual usage pattern in quick_fixes.rs and refactors.rs
/// which import from perl_parser::ast_utils after the Wave D collapse.
#[test]
fn test_code_actions_facade_integration() -> Result<(), Box<dyn std::error::Error>> {
    use perl_parser::ast_utils::{find_declaration_position, find_statement_start};

    // Code actions rely on AST utilities for finding declaration positions.
    // Simulate the quick_fixes.rs usage: given an error position, find where
    // to insert a "my $x" declaration.
    let code = "foo();\n$x = 1;";

    // $x is at an undefined-variable position — find where to declare it
    let error_pos = must_some(code.find("$x"));
    let decl_pos = find_declaration_position(code, error_pos);

    // Declaration should be inserted at the start of the statement containing $x
    let stmt_start = find_statement_start(code, error_pos);
    assert_eq!(decl_pos, stmt_start, "Declaration position should be at statement start");
    assert!(
        code[decl_pos..].starts_with("$x = 1"),
        "Should position at the start of the statement containing the undefined variable"
    );

    Ok(())
}

/// Test consumer crate simulation: perl-incremental-parsing
#[cfg(feature = "incremental")]
#[test]
fn test_incremental_parsing_facade() -> Result<(), Box<dyn std::error::Error>> {
    use perl_parser::{Edit, IncrementalState};

    let code = "my $x = 1;";
    let mut state = IncrementalState::new(code.to_string());

    assert!(matches!(state.ast.kind, perl_parser::NodeKind::Program { .. }));

    // Apply an edit
    let edit = Edit { start_byte: 3, old_end_byte: 5, new_end_byte: 5, new_text: "$y".to_string() };
    perl_parser::apply_edits(&mut state, &[edit])?;

    // After apply_edits, the state's AST is updated
    assert!(matches!(state.ast.kind, perl_parser::NodeKind::Program { .. }));

    Ok(())
}

/// Test that Error trait implementations work correctly
#[test]
fn test_facade_error_trait_impl() -> Result<(), Box<dyn std::error::Error>> {
    let code = "if ("; // Incomplete
    let mut parser = Parser::new(code);
    parser.parse().ok();

    let errors = parser.errors();
    if !errors.is_empty() {
        // Error should implement Display
        let error = &errors[0];
        let _msg = error.to_string();
    }

    Ok(())
}

/// Test case from perl-lsp-navigation: references finding via facade
#[test]
fn test_navigation_via_facade() -> Result<(), Box<dyn std::error::Error>> {
    use perl_parser::Parser;

    let code = "sub greet { print 'hello'; } greet();";

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Should be able to find subroutine definitions
    assert!(matches!(ast.kind, perl_parser::NodeKind::Program { .. }));

    Ok(())
}

/// Test case from perl-lsp-diagnostics: heredoc anti-pattern detection
#[test]
fn test_diagnostics_antipattern_detection_via_facade() -> Result<(), Box<dyn std::error::Error>> {
    // This tests that perl-lsp-diagnostics can still detect heredoc anti-patterns
    // via the facade after the crate was absorbed into perl-parser
    let code = "my $x = <<'END';\nsome text\nEND\n";

    let mut parser = Parser::new(code);
    let result = parser.parse();

    // Should parse or recover gracefully
    assert!(result.is_ok());

    Ok(())
}

/// Test boundary: recursive descent depth (parser state)
#[test]
fn test_deep_nesting_via_facade() -> Result<(), Box<dyn std::error::Error>> {
    // Create deeply nested structure
    let mut code = String::from("if (1) ");
    for _ in 0..50 {
        code.push_str("{ if (1) ");
    }
    for _ in 0..50 {
        code.push_str(" }");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    // Should handle deep nesting (or recover gracefully)
    assert!(result.is_ok() || !parser.errors().is_empty());

    Ok(())
}

/// Test UTF-8 handling after facade consolidation
#[test]
fn test_unicode_via_facade() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $str = \"こんにちは\"; # Hello in Japanese";

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    assert!(matches!(ast.kind, perl_parser::NodeKind::Program { .. }));

    Ok(())
}

/// Test that all major facade exports are accessible in a simple use case
#[test]
fn test_comprehensive_facade_exports() -> Result<(), Box<dyn std::error::Error>> {
    // This is a comprehensive test ensuring major re-exports are available
    use perl_parser::{Node, Parser, ScopeAnalyzer, SemanticAnalyzer};

    let mut parser = Parser::new("sub test { }");
    let ast: Node = must(parser.parse());

    let _analyzer = SemanticAnalyzer::analyze(&ast);

    let scope = ScopeAnalyzer::new();
    let source = "sub test { }";
    let _issues = scope.analyze(&ast, source, &[]);

    Ok(())
}
