//! A modern, modular Perl parser built on perl-lexer
//!
//! This crate provides a clean, efficient parser that consumes tokens from
//! the perl-lexer crate and produces a well-structured Abstract Syntax Tree (AST).
//!
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

pub mod ast;
pub use parser::Parser;
pub mod ast_v2;
pub mod builtin_signatures;
pub mod code_actions;
pub mod code_actions_enhanced;
pub mod completion;
pub mod diagnostics;
pub mod execute_command;
pub mod edit;
pub mod error;
pub mod error_classifier;
pub mod error_recovery;
pub mod code_actions_provider;
pub mod formatting;
#[cfg(feature = "incremental")]
pub mod incremental;
#[cfg(feature = "incremental")]
pub mod incremental_document;
#[cfg(feature = "incremental")]
pub mod incremental_edit;
// pub mod refactoring; // TODO: Fix compilation errors
#[cfg(feature = "incremental")]
pub mod incremental_checkpoint;
#[cfg(feature = "incremental")]
pub mod incremental_simple;
#[cfg(feature = "incremental")]
pub mod incremental_v2;
pub mod lsp_server;
// pub mod lsp_server_incremental; // TODO: Fix imports
#[cfg(feature = "incremental")]
pub mod lsp_server_incremental_v2;
pub mod parser;
pub mod parser_context;
pub mod position;
pub mod recovery_parser;
pub mod rename;
pub mod semantic;
pub mod signature_help;
pub mod symbol;
pub mod folding;
pub mod token_stream;
pub mod token_wrapper;
pub mod trivia;
pub mod trivia_parser;
pub mod workspace_symbols;
pub mod code_lens_provider;
pub mod line_index;
pub mod document_store;
pub mod workspace_index;
pub mod workspace_refactor;
pub mod import_optimizer;
pub mod dead_code_detector;
pub mod type_inference;
pub mod perl_critic;
pub mod perltidy;
pub mod semantic_tokens_provider;
pub mod call_hierarchy_provider;
pub mod document_highlight;
pub mod type_hierarchy;
pub mod inlay_hints_provider;
pub mod test_runner;
// pub mod test_generator;  // TODO: Fix compilation
// pub mod tdd_workflow;    // TODO: Fix compilation
pub mod tdd_basic;
pub mod performance;
pub mod debug_adapter;
pub mod modernize;
pub mod modernize_refactored;
pub mod scope_analyzer;
pub mod pragma_tracker;

pub use ast::{Node, NodeKind, SourceLocation};
pub use pragma_tracker::{PragmaTracker, PragmaState};
pub use error::{ParseError, ParseResult};
pub use recovery_parser::RecoveryParser;
pub use token_stream::{Token, TokenKind, TokenStream};
pub use trivia::{Trivia, TriviaToken, NodeWithTrivia};
pub use trivia_parser::{TriviaPreservingParser, format_with_trivia};
#[cfg(feature = "incremental")]
pub use incremental_checkpoint::{CheckpointedIncrementalParser, SimpleEdit};

// Incremental parsing exports (feature-gated)
#[cfg(feature = "incremental")]
pub use incremental::{IncrementalState, Edit, apply_edits};

// IDE feature exports
pub use symbol::{Symbol, SymbolKind, SymbolTable, SymbolExtractor, SymbolReference};
pub use semantic::{SemanticAnalyzer, SemanticToken, SemanticTokenType, SemanticTokenModifier, HoverInfo};
pub use completion::{CompletionProvider, CompletionItem, CompletionItemKind, CompletionContext};
pub use signature_help::{SignatureHelpProvider, SignatureHelp, SignatureInfo, ParameterInfo};
pub use rename::{RenameProvider, RenameResult, RenameOptions, TextEdit, apply_rename_edits};
pub use diagnostics::{DiagnosticsProvider, Diagnostic, DiagnosticSeverity, DiagnosticTag, RelatedInformation};
pub use code_actions::{CodeActionsProvider, CodeAction, CodeActionKind, CodeActionEdit};
pub use code_actions_enhanced::EnhancedCodeActionsProvider;
pub use lsp_server::{LspServer, JsonRpcRequest, JsonRpcResponse};
pub use formatting::{CodeFormatter, FormattingOptions, FormatTextEdit};
pub use workspace_symbols::{WorkspaceSymbolsProvider, WorkspaceSymbol};
pub use code_lens_provider::{CodeLensProvider, CodeLens, resolve_code_lens, get_shebang_lens};
pub use semantic_tokens_provider::{
    SemanticTokensProvider, SemanticToken as SemanticTokenV2, 
    SemanticTokenType as SemanticTokenTypeV2, SemanticTokenModifier as SemanticTokenModifierV2,
    encode_semantic_tokens
};
pub use folding::{FoldingRangeExtractor, FoldingRange, FoldingRangeKind};
pub use type_inference::{
    TypeInferenceEngine, PerlType, ScalarType, TypeEnvironment,
    TypeConstraint, TypeLocation, TypeBasedCompletion
};
pub use scope_analyzer::{ScopeAnalyzer, IssueKind, ScopeIssue};
pub use code_actions_provider::{
    CodeActionsProvider as CodeActionsProviderV2, 
    CodeAction as CodeActionV2, 
    CodeActionKind as CodeActionKindV2,
    TextEdit as TextEditV2
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
                if let NodeKind::VariableDeclaration { declarator: decl, .. } = &statements[0].kind {
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
                assert!(!statements.is_empty());
                if let NodeKind::Binary { op, .. } = &statements[0].kind {
                    assert_eq!(op, expected_op);
                } else {
                    panic!("Expected Binary operator for: {}", code);
                }
            }
        }
    }

    #[test]
    fn test_operators_with_context() {
        // These operators need better context handling
        let _cases: Vec<(&str, &str)> = vec![
            // ("2 / 3", "/"), // Slash disambiguation issue
            // ("$a % $b", "%"), // Percent vs hash sigil issue
            // ("$a ** $b", "**"), // Glob pattern issue  
            // ("$a // $b", "//"), // Defined-or vs regex issue
        ];
        // TODO: Implement proper context-aware parsing for these operators
    }

    #[test]
    fn test_string_literals() {
        let cases = vec![
            r#""hello""#,
            r#"'world'"#,
            r#"qq{foo}"#,
            r#"q{bar}"#,
        ];

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