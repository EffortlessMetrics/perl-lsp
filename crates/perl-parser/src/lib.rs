//! # perl-parser v3 — native Perl parser + LSP + TDD
//!
//! - **Tree-sitter compatible** kinds/fields/points.
//! - Parser: ~100% edge cases; fast UTF-16 mapping.
//! - LSP: contract-driven capability surface; ~82% functional in v0.8.6.
//! - **TDD Support**: Auto-detecting TestGenerator with intelligent return value analysis.
//!
//! ## Quick use (library)
//! ```rust
//! use perl_parser::Parser;
//! let mut p = Parser::new(r#"sub hello { print "hi\n"; }"#);
//! let ast = p.parse().unwrap();
//! println!("{}", ast.to_sexp());
//! ```
//!
//! ## Test Generation (TDD)
//! ```rust
//! use perl_parser::{Parser, TestGenerator, TestFramework};
//! let mut p = Parser::new(r#"sub add { my ($a, $b) = @_; return $a + $b; }"#);
//! let ast = p.parse().unwrap();
//!
//! let generator = TestGenerator::new(TestFramework::TestMore);
//! let tests = generator.generate_tests(&ast, "");
//! // Auto-detects that add(1, 2) should return 3
//! ```
//!
//! ## LSP server
//! ```bash
//! cargo install perl-parser --bin perl-lsp
//! perl-lsp --stdio
//! ```
//!
//! **Capability policy:** see `docs/LSP_CAPABILITY_POLICY.md`.

#![deny(unsafe_code)]
#![deny(unreachable_pub)] // prevent stray pub items from escaping
#![warn(rust_2018_idioms)]
// NOTE: missing_docs enabled with baseline enforcement (Issue #197)
// Baseline enforced via ci/missing_docs_baseline.txt
#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(
    // Core allows for parser/lexer code
    clippy::too_many_lines,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,

    // Parser-specific patterns that are fine
    clippy::wildcard_imports,
    clippy::enum_glob_use,
    clippy::match_same_arms,
    clippy::if_not_else,
    clippy::struct_excessive_bools,
    clippy::items_after_statements,
    clippy::return_self_not_must_use,
    clippy::unused_self,
    clippy::collapsible_match,
    clippy::collapsible_if,
    clippy::only_used_in_recursion,
    clippy::items_after_test_module,
    clippy::while_let_loop,
    clippy::single_range_in_vec_init,
    clippy::arc_with_non_send_sync,
    clippy::needless_range_loop,
    clippy::result_large_err,
    clippy::if_same_then_else,
    clippy::should_implement_trait,
    clippy::manual_flatten,

    // String handling in parsers
    clippy::needless_raw_string_hashes,
    clippy::single_char_pattern,
    clippy::uninlined_format_args
)]
//! ## Architecture
//!
//! The parser follows a recursive descent design with operator precedence handling,
//! maintaining a clean separation from the lexing phase. This modular approach
//! enables:
//!
//! - Independent testing of parsing logic
//! - Easy integration with different lexer implementations
//! - Clear error boundaries between lexing and parsing phases
//! - Optimal performance through single-pass parsing
//!
//! ## Example
//!
//! ```rust
//! use perl_parser::Parser;
//!
//! let code = "my $x = 42;";
//! let mut parser = Parser::new(code);
//!
//! match parser.parse() {
//!     Ok(ast) => println!("AST: {}", ast.to_sexp()),
//!     Err(e) => eprintln!("Parse error: {}", e),
//! }
//! ```

/// Abstract Syntax Tree (AST) definitions for Perl parsing.
#[allow(missing_docs)]
pub mod ast;
pub use parser::Parser;
/// Experimental second‑generation AST (work in progress).
#[allow(missing_docs)]
pub mod ast_v2;
pub mod builtin_signatures;
pub mod builtin_signatures_phf;
/// LSP call hierarchy provider for function call navigation.
pub mod call_hierarchy_provider;
pub mod cancellation;
pub mod capabilities;
pub mod code_actions;
pub mod code_actions_enhanced;
pub mod code_actions_pragmas;
/// LSP code actions provider for automated refactoring and fixes.
pub mod code_actions_provider;
pub mod code_lens_provider;
pub mod completion;
#[cfg(not(target_arch = "wasm32"))]
pub mod dead_code_detector;
pub mod debug_adapter;
pub mod declaration;
pub mod diagnostics;
pub mod diagnostics_catalog;
pub mod document_highlight;
/// LSP document links provider for file and URL navigation.
pub mod document_links;
pub mod document_store;
pub mod edit;
pub mod error;
/// Error classification and recovery strategies for parse failures.
pub mod error_classifier;
pub mod error_recovery;
#[cfg(not(target_arch = "wasm32"))]
pub mod execute_command;
/// Feature flags and capability management for LSP server functionality.
pub mod features;
pub mod folding;
pub mod formatting;
/// Heredoc content collector with FIFO ordering and indent stripping.
pub mod heredoc_collector;
pub mod implementation_provider;
pub mod import_optimizer;
#[cfg(feature = "incremental")]
pub mod incremental;
#[cfg(feature = "incremental")]
pub mod incremental_advanced_reuse;
#[cfg(feature = "incremental")]
pub mod incremental_checkpoint;
#[cfg(feature = "incremental")]
pub mod incremental_document;
#[cfg(feature = "incremental")]
pub mod incremental_edit;
#[cfg(feature = "incremental")]
pub mod incremental_handler_v2;
#[cfg(feature = "incremental")]
pub mod incremental_integration;
#[cfg(feature = "incremental")]
pub mod incremental_simple;
#[cfg(feature = "incremental")]
pub mod incremental_v2;
pub mod index;
/// LSP inlay hints for inline type and parameter information.
pub mod inlay_hints;
/// LSP inlay hints provider implementation.
pub mod inlay_hints_provider;
pub mod inline_completions;
pub mod line_index;
/// LSP linked editing provider for synchronized symbol renaming.
pub mod linked_editing;
/// Modular LSP server implementation (migration target)
/// Note: server/transport submodules are gated off on wasm32.
pub mod lsp;
#[cfg(not(target_arch = "wasm32"))]
pub mod lsp_document_link;
#[cfg(not(target_arch = "wasm32"))]
pub mod lsp_errors;
pub mod lsp_on_type_formatting;
pub mod lsp_selection_range;
#[cfg(not(target_arch = "wasm32"))]
pub mod lsp_server;
pub mod lsp_utils;
/// Code modernization utilities for Perl best practices.
pub mod modernize;
/// Enhanced code modernization with refactoring capabilities.
pub mod modernize_refactored;
/// LSP on-type formatting provider for automatic code formatting.
pub mod on_type_formatting;
pub mod parser;
pub mod parser_context;
pub mod performance;
pub mod perl_critic;
pub mod perltidy;
pub mod position;
pub mod position_mapper;
#[doc(hidden)]
pub mod positions;
pub mod pragma_tracker;
/// Parser for Perl quote and quote-like operators.
pub mod quote_parser;
pub mod recovery_parser;
/// Unified refactoring engine for comprehensive code transformations.
pub mod refactoring;
/// LSP references provider for symbol usage analysis.
pub mod references;
pub mod rename;
/// Scope analysis for variable and subroutine resolution.
#[allow(missing_docs)]
pub mod scope_analyzer;
/// LSP selection range provider for smart text selection.
pub mod selection_range;
pub mod semantic;
/// LSP semantic tokens provider for syntax highlighting.
pub mod semantic_tokens;
pub mod semantic_tokens_provider;
pub mod signature_help;
pub mod symbol;
#[allow(missing_docs)]
pub mod tdd_basic;
/// TDD workflow integration for Test-Driven Development support.
pub mod tdd_workflow;
pub mod test_generator;
/// Test execution and TDD support functionality.
pub mod test_runner;
pub mod textdoc;
pub mod token_stream;
pub mod token_wrapper;
pub mod trivia;
pub mod trivia_parser;
pub mod type_definition;
/// LSP type hierarchy provider for inheritance navigation.
pub mod type_hierarchy;
/// Type inference engine for Perl variable analysis.
pub mod type_inference;
pub mod uri;
pub mod util;
pub mod workspace_index;
#[cfg(not(target_arch = "wasm32"))]
pub mod workspace_refactor;
pub mod workspace_rename;
pub mod workspace_symbols;

// Compatibility module for tests using old API
#[cfg(any(test, feature = "test-compat"))]
pub mod compat;

pub use ast::{Node, NodeKind, SourceLocation};
pub use error::{ParseError, ParseResult};
#[cfg(feature = "incremental")]
pub use incremental_checkpoint::{CheckpointedIncrementalParser, SimpleEdit};
pub use pragma_tracker::{PragmaState, PragmaTracker};
pub use recovery_parser::RecoveryParser;
pub use token_stream::{Token, TokenKind, TokenStream};
pub use trivia::{NodeWithTrivia, Trivia, TriviaToken};
pub use trivia_parser::{TriviaPreservingParser, format_with_trivia};

// Incremental parsing exports (feature-gated)
#[cfg(feature = "incremental")]
pub use incremental::{Edit, IncrementalState, apply_edits};

// IDE feature exports
pub use semantic::{
    HoverInfo, SemanticAnalyzer, SemanticModel, SemanticToken, SemanticTokenModifier,
    SemanticTokenType,
};
pub use symbol::{Symbol, SymbolExtractor, SymbolKind, SymbolReference, SymbolTable};

#[cfg(test)]
mod workspace_index_utf16_test;
pub use code_actions::{CodeAction, CodeActionEdit, CodeActionKind, CodeActionsProvider};
pub use code_actions_enhanced::EnhancedCodeActionsProvider;
pub use code_actions_provider::{
    CodeAction as CodeActionV2, CodeActionKind as CodeActionKindV2,
    CodeActionsProvider as CodeActionsProviderV2, TextEdit as TextEditV2,
};
pub use code_lens_provider::{CodeLens, CodeLensProvider, get_shebang_lens, resolve_code_lens};
pub use completion::{CompletionContext, CompletionItem, CompletionItemKind, CompletionProvider};
pub use diagnostics::{
    Diagnostic, DiagnosticSeverity, DiagnosticTag, DiagnosticsProvider, RelatedInformation,
};
pub use document_links::compute_links;
pub use folding::{FoldingRange, FoldingRangeExtractor, FoldingRangeKind};
pub use formatting::{CodeFormatter, FormatTextEdit, FormattingOptions};
pub use import_optimizer::{
    DuplicateImport, ImportAnalysis, ImportEntry, ImportOptimizer, MissingImport,
    OrganizationSuggestion, SuggestionPriority, UnusedImport,
};
pub use inlay_hints::{parameter_hints, trivial_type_hints};
// Export LSP types from the new modular structure (not available on wasm32)
#[cfg(not(target_arch = "wasm32"))]
pub use lsp::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
#[cfg(not(target_arch = "wasm32"))]
pub use lsp_server::LspServer;
pub use on_type_formatting::compute_on_type_edit;
pub use rename::{RenameOptions, RenameProvider, RenameResult, TextEdit, apply_rename_edits};
pub use scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue};
pub use selection_range::{build_parent_map, selection_chain};
pub use semantic_tokens::{
    EncodedToken, TokensLegend, collect_semantic_tokens, legend as semantic_legend,
};
pub use semantic_tokens_provider::{
    SemanticToken as SemanticTokenV2, SemanticTokenModifier as SemanticTokenModifierV2,
    SemanticTokenType as SemanticTokenTypeV2, SemanticTokensProvider, encode_semantic_tokens,
};
pub use signature_help::{ParameterInfo, SignatureHelp, SignatureHelpProvider, SignatureInfo};
pub use test_generator::{
    CoverageReport, Priority, RefactoringCategory, RefactoringSuggester, RefactoringSuggestion,
    TestCase, TestFramework, TestGenerator, TestGeneratorOptions, TestResults, TestRunner,
};
pub use type_inference::{
    PerlType, ScalarType, TypeBasedCompletion, TypeConstraint, TypeEnvironment,
    TypeInferenceEngine, TypeLocation,
};
pub use workspace_symbols::{WorkspaceSymbol, WorkspaceSymbolsProvider};

// TDD workflow and refactoring exports
pub use refactoring::{
    ModernizationPattern, RefactoringConfig, RefactoringEngine, RefactoringOperation,
    RefactoringResult, RefactoringScope, RefactoringType,
};
pub use tdd_workflow::{
    AnnotationSeverity, CoverageAnnotation, TddAction, TddConfig, TddCycleResult, TddWorkflow,
    TestType, WorkflowState, WorkflowStatus,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        let mut parser = Parser::new("my $x = 42;");
        let result = parser.parse();
        assert!(result.is_ok());

        let ast = result.unwrap();
        assert!(matches!(ast.kind, NodeKind::Program { .. }));
    }

    #[test]
    fn test_variable_declaration() {
        let cases = vec![
            ("my $x;", "my"),
            ("our $y;", "our"),
            ("local $z;", "local"),
            ("state $w;", "state"),
        ];

        for (code, declarator) in cases {
            let mut parser = Parser::new(code);
            let result = parser.parse();
            assert!(result.is_ok(), "Failed to parse: {}", code);

            let ast = result.unwrap();
            if let NodeKind::Program { statements } = &ast.kind {
                assert_eq!(statements.len(), 1);
                if let NodeKind::VariableDeclaration { declarator: decl, .. } = &statements[0].kind
                {
                    assert_eq!(decl, declarator);
                } else {
                    panic!("Expected VariableDeclaration for: {}", code);
                }
            }
        }
    }

    #[test]
    fn test_operators() {
        // Test operators that work correctly
        let cases = vec![
            ("$a + $b", "+"),
            ("$a - $b", "-"),
            ("$a * $b", "*"),
            ("$a . $b", "."),
            ("$a && $b", "&&"),
            ("$a || $b", "||"),
        ];

        for (code, expected_op) in cases {
            let mut parser = Parser::new(code);
            let result = parser.parse();
            assert!(result.is_ok(), "Failed to parse: {}", code);

            let ast = result.unwrap();
            if let NodeKind::Program { statements } = &ast.kind {
                assert!(!statements.is_empty(), "No statements found in AST for: {}", code);

                // Find the binary node, which might be wrapped in an ExpressionStatement
                let binary_node = match &statements[0].kind {
                    NodeKind::ExpressionStatement { expression } => match &expression.kind {
                        NodeKind::Binary { op, left, right } => Some((op, left, right)),
                        _ => None,
                    },
                    NodeKind::Binary { op, left, right } => Some((op, left, right)),
                    _ => None,
                };

                if let Some((op, left, right)) = binary_node {
                    assert_eq!(op, expected_op, "Operator mismatch for: {}", code);

                    // Additional diagnostic information
                    println!("Parsing: {}", code);
                    println!("Left node: {:?}", left);
                    println!("Right node: {:?}", right);
                } else {
                    panic!(
                        "Expected Binary operator for: {}. Found: {:?}",
                        code, statements[0].kind
                    );
                }
            } else {
                panic!("Expected Program node, found: {:?}", ast.kind);
            }
        }
    }

    #[test]
    fn test_operators_with_context() {
        // These operators require context-aware parsing to disambiguate from similar syntax:
        // - `/` could be division or regex delimiter
        // - `%` could be modulo or hash sigil
        // - `**` could be exponent or glob pattern
        // - `//` could be defined-or or regex delimiter
        // The lexer handles disambiguation via LexerMode::ExpectTerm tracking.
        let cases: Vec<(&str, &str)> = vec![
            ("2 / 3", "/"),     // Division (not regex)
            ("$a % $b", "%"),   // Modulo (not hash sigil)
            ("$a ** $b", "**"), // Exponent (not glob)
            ("$a // $b", "//"), // Defined-or (not regex)
        ];

        for (code, expected_op) in cases {
            let mut parser = Parser::new(code);
            let result = parser.parse();
            assert!(result.is_ok(), "Failed to parse: {}", code);

            let ast = result.unwrap();
            if let NodeKind::Program { statements } = &ast.kind {
                assert!(!statements.is_empty(), "No statements found in AST for: {}", code);

                // Find the binary node, which might be wrapped in an ExpressionStatement
                let binary_node = match &statements[0].kind {
                    NodeKind::ExpressionStatement { expression } => match &expression.kind {
                        NodeKind::Binary { op, .. } => Some(op),
                        _ => None,
                    },
                    NodeKind::Binary { op, .. } => Some(op),
                    _ => None,
                };

                if let Some(op) = binary_node {
                    assert_eq!(op, expected_op, "Operator mismatch for: {}", code);
                } else {
                    panic!(
                        "Expected Binary operator for: {}. Found: {:?}",
                        code, statements[0].kind
                    );
                }
            } else {
                panic!("Expected Program node, found: {:?}", ast.kind);
            }
        }
    }

    #[test]
    fn test_string_literals() {
        let cases = vec![r#""hello""#, r#"'world'"#, r#"qq{foo}"#, r#"q{bar}"#];

        for code in cases {
            let mut parser = Parser::new(code);
            let result = parser.parse();
            assert!(result.is_ok(), "Failed to parse: {}", code);
        }
    }

    #[test]
    fn test_arrays_and_hashes() {
        let cases = vec![
            "@array",
            "%hash",
            "$array[0]",
            "$hash{key}",
            "@array[1, 2, 3]",
            "@hash{'a', 'b'}",
        ];

        for code in cases {
            let mut parser = Parser::new(code);
            let result = parser.parse();
            assert!(result.is_ok(), "Failed to parse: {}", code);
        }
    }

    #[test]
    fn test_subroutines() {
        let cases = vec![
            "sub foo { }",
            "sub bar { return 42; }",
            "sub baz ($x, $y) { $x + $y }",
            "sub qux :method { }",
        ];

        for code in cases {
            let mut parser = Parser::new(code);
            let result = parser.parse();
            assert!(result.is_ok(), "Failed to parse: {}", code);

            let ast = result.unwrap();
            if let NodeKind::Program { statements } = &ast.kind {
                assert_eq!(statements.len(), 1);
                assert!(matches!(statements[0].kind, NodeKind::Subroutine { .. }));
            }
        }
    }

    #[test]
    fn test_control_flow() {
        let cases = vec![
            "if ($x) { }",
            "if ($x) { } else { }",
            "if ($x) { } elsif ($y) { } else { }",
            "unless ($x) { }",
            "while ($x) { }",
            "until ($x) { }",
            "for (my $i = 0; $i < 10; $i++) { }",
            "foreach my $x (@array) { }",
        ];

        for code in cases {
            let mut parser = Parser::new(code);
            let result = parser.parse();
            assert!(result.is_ok(), "Failed to parse: {}", code);
        }
    }

    #[test]
    fn test_regex() {
        let cases = vec![
            "/pattern/",
            "m/pattern/",
            "s/old/new/",
            "tr/a-z/A-Z/",
            r#"qr/\d+/"#,
            "$x =~ /foo/",
            "$x !~ /bar/",
        ];

        for code in cases {
            let mut parser = Parser::new(code);
            let result = parser.parse();
            assert!(result.is_ok(), "Failed to parse: {}", code);
        }
    }

    #[test]
    fn test_error_cases() {
        let cases = vec![
            ("if (", "Unexpected end of input"),
            ("sub (", "Unexpected end of input"),
            ("my (", "Unexpected end of input"),
            ("{", "Unexpected end of input"),
        ];

        for (code, _expected_error) in cases {
            let mut parser = Parser::new(code);
            let result = parser.parse();
            assert!(result.is_err(), "Expected error for: {}", code);
        }
    }

    #[test]
    fn test_modern_perl_features() {
        let cases = vec![
            "class Point { }",
            "method new { }",
            "try { } catch ($e) { }",
            // "defer { }", // defer is not yet supported by the lexer
            "my $x :shared = 42;",
        ];

        for code in cases {
            let mut parser = Parser::new(code);
            let result = parser.parse();
            assert!(result.is_ok(), "Failed to parse: {}", code);
        }
    }

    #[test]
    fn test_edge_cases() {
        let cases = vec![
            // Indirect object syntax
            "print STDOUT 'hello';",
            "new Class;",
            // Multi-variable declarations
            "my ($x, $y) = (1, 2);",
            "my ($a :shared, $b :locked);",
            // Complex expressions
            "$x->@*",
            "$x->%*",
            "$x->$*",
            // Defined-or
            "$x // 'default'",
            // ISA operator
            "$obj ISA 'Class'",
        ];

        for code in cases {
            let mut parser = Parser::new(code);
            let result = parser.parse();
            assert!(result.is_ok(), "Failed to parse edge case: {}", code);
        }
    }
}
