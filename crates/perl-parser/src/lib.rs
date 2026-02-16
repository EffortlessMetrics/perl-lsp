//! # perl-parser — Production-grade Perl parser and Language Server Protocol engine
//!
//! A comprehensive Perl parser built on recursive descent principles, providing robust AST
//! generation, LSP feature providers, workspace indexing, and test-driven development support.
//!
//! ## Key Features
//!
//! - **Tree-sitter Compatible**: AST with kinds, fields, and position tracking compatible with tree-sitter grammar
//! - **Comprehensive Parsing**: ~100% edge case coverage for Perl 5.8-5.40 syntax
//! - **LSP Integration**: Full Language Server Protocol feature set (~82% coverage in v0.8.6)
//! - **TDD Workflow**: Intelligent test generation with return value analysis
//! - **Incremental Parsing**: Efficient re-parsing for real-time editing
//! - **Error Recovery**: Graceful handling of malformed input with detailed diagnostics
//! - **Workspace Navigation**: Cross-file symbol resolution and reference tracking
//!
//! ## Quick Start
//!
//! ### Basic Parsing
//!
//! ```rust
//! use perl_parser::Parser;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let code = r#"sub hello { print "Hello, world!\n"; }"#;
//! let mut parser = Parser::new(code);
//!
//! match parser.parse() {
//!     Ok(ast) => {
//!         println!("AST: {}", ast.to_sexp());
//!         println!("Parsed {} nodes", ast.count_nodes());
//!     }
//!     Err(e) => eprintln!("Parse error: {}", e),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Test-Driven Development
//!
//! Generate tests automatically from parsed code:
//!
//! ```rust
//! use perl_parser::{Parser, TestGenerator, TestFramework};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let code = r#"sub add { my ($a, $b) = @_; return $a + $b; }"#;
//! let mut parser = Parser::new(code);
//! let ast = parser.parse()?;
//!
//! let generator = TestGenerator::new(TestFramework::TestMore);
//! let tests = generator.generate_tests(&ast, "");
//!
//! // Outputs test cases with intelligent assertions
//! // Auto-detects that add(1, 2) should return 3
//! println!("{}", tests);
//! # Ok(())
//! # }
//! ```
//!
//! ### LSP Integration
//!
//! Use as a library for LSP features (see [`perl_lsp`] for the standalone server):
//!
//! ```rust
//! use perl_parser::{Parser, SemanticAnalyzer};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let code = "my $x = 42;";
//! let mut parser = Parser::new(code);
//! let ast = parser.parse()?;
//!
//! // Semantic analysis for hover, completion, etc.
//! let analyzer = SemanticAnalyzer::new();
//! let model = analyzer.analyze(&ast);
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture
//!
//! The parser is organized into distinct layers for maintainability and testability:
//!
//! ### Core Engine ([`engine`])
//!
//! - **[`parser`]**: Recursive descent parser with operator precedence
//! - **[`ast`]**: Abstract Syntax Tree definitions and node types
//! - **[`error`]**: Error classification, recovery strategies, and diagnostics
//! - **[`position`]**: UTF-16 position mapping for LSP protocol compliance
//! - **[`quote_parser`]**: Specialized parser for quote-like operators
//! - **[`heredoc_collector`]**: FIFO heredoc collection with indent stripping
//!
//! ### IDE Integration ([`ide`])
//!
//! - **[`lsp`]**: Core LSP protocol types and message handling
//! - **[`lsp_compat`]**: LSP feature providers (completion, hover, etc.)
//! - **[`diagnostics_catalog`]**: Stable diagnostic codes and messages
//! - **[`cancellation`]**: Request cancellation infrastructure
//! - **[`call_hierarchy_provider`]**: Function call navigation
//!
//! ### Analysis ([`analysis`])
//!
//! - **[`scope_analyzer`]**: Variable and subroutine scoping resolution
//! - **[`type_inference`]**: Perl type inference engine
//! - **[`semantic`]**: Semantic model with hover information
//! - **[`symbol`]**: Symbol table and reference tracking
//! - **[`dead_code_detector`]**: Unused code detection
//!
//! ### Workspace ([`workspace`])
//!
//! - **[`workspace_index`]**: Cross-file symbol indexing
//! - **[`workspace_rename`]**: Multi-file refactoring
//! - **[`document_store`]**: Document state management
//!
//! ### Refactoring ([`refactor`])
//!
//! - **[`refactoring`]**: Unified refactoring engine
//! - **[`modernize`]**: Code modernization utilities
//! - **[`import_optimizer`]**: Import statement analysis and optimization
//!
//! ### Test Support ([`tdd`])
//!
//! - **[`test_generator`]**: Intelligent test case generation
//! - **[`test_runner`]**: Test execution and validation
//! - **[`tdd_workflow`]**: TDD cycle management and coverage tracking
//!
//! ## LSP Feature Support
//!
//! This crate provides the engine for LSP features. The standalone server is in [`perl_lsp`].
//!
//! ### Implemented Features
//!
//! - **Completion**: Context-aware code completion with type inference
//! - **Hover**: Documentation and type information on hover
//! - **Definition**: Go-to-definition with cross-file support
//! - **References**: Find all references with workspace indexing
//! - **Rename**: Symbol renaming with conflict detection
//! - **Diagnostics**: Syntax errors and semantic warnings
//! - **Formatting**: Code formatting via perltidy integration
//! - **Folding**: Code folding for blocks and regions
//! - **Semantic Tokens**: Fine-grained syntax highlighting
//! - **Call Hierarchy**: Function call navigation
//! - **Type Hierarchy**: Class inheritance navigation
//!
//! See `docs/LSP_CAPABILITY_POLICY.md` for the complete capability matrix.
//!
//! ## Incremental Parsing
//!
//! Enable efficient re-parsing for real-time editing:
//!
//! ```rust,ignore
//! use perl_parser::{IncrementalState, apply_edits, Edit};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut state = IncrementalState::new("my $x = 1;");
//! let ast = state.parse()?;
//!
//! // Apply an edit
//! let edit = Edit {
//!     start_byte: 3,
//!     old_end_byte: 5,
//!     new_end_byte: 5,
//!     text: "$y".to_string(),
//! };
//! apply_edits(&mut state, vec![edit]);
//!
//! // Incremental re-parse reuses unchanged nodes
//! let new_ast = state.parse()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Error Recovery
//!
//! The parser uses intelligent error recovery to continue parsing after errors:
//!
//! ```rust
//! use perl_parser::Parser;
//!
//! let code = "sub broken { if (";  // Incomplete code
//! let mut parser = Parser::new(code);
//!
//! // Parser recovers and builds partial AST
//! let result = parser.parse();
//! assert!(result.is_ok());
//!
//! // Check recorded errors
//! let errors = parser.errors();
//! assert!(!errors.is_empty());
//! ```
//!
//! ## Workspace Indexing
//!
//! Build cross-file indexes for workspace-wide navigation:
//!
//! ```rust,ignore
//! use perl_parser::workspace_index::WorkspaceIndex;
//!
//! let mut index = WorkspaceIndex::new();
//! index.index_file("lib/Foo.pm", "package Foo; sub bar { }");
//! index.index_file("lib/Baz.pm", "use Foo; Foo::bar();");
//!
//! // Find all references to Foo::bar
//! let refs = index.find_references("Foo::bar");
//! ```
//!
//! ## Testing with perl-corpus
//!
//! The parser is tested against the comprehensive [`perl_corpus`] test suite:
//!
//! ```bash
//! # Run parser tests with full corpus coverage
//! cargo test -p perl-parser
//!
//! # Run specific test category
//! cargo test -p perl-parser --test regex_tests
//!
//! # Validate documentation examples
//! cargo test --doc
//! ```
//!
//! ## Command-Line Tools
//!
//! Build and install the LSP server binary:
//!
//! ```bash
//! # Build LSP server
//! cargo build -p perl-lsp --release
//!
//! # Install globally
//! cargo install --path crates/perl-lsp
//!
//! # Run LSP server
//! perl-lsp --stdio
//!
//! # Check server health
//! perl-lsp --health
//! ```
//!
//! ## Integration Examples
//!
//! ### VSCode Extension
//!
//! Configure the LSP server in VSCode settings:
//!
//! ```json
//! {
//!   "perl.lsp.path": "/path/to/perl-lsp",
//!   "perl.lsp.args": ["--stdio"]
//! }
//! ```
//!
//! ### Neovim Integration
//!
//! ```lua
//! require'lspconfig'.perl.setup{
//!   cmd = { "/path/to/perl-lsp", "--stdio" },
//! }
//! ```
//!
//! ## Performance Characteristics
//!
//! - **Single-pass parsing**: O(n) complexity for well-formed input
//! - **UTF-16 mapping**: Fast bidirectional offset conversion for LSP
//! - **Incremental updates**: Reuses unchanged AST nodes for efficiency
//! - **Memory efficiency**: Streaming token processing with bounded lookahead
//!
//! ## Compatibility
//!
//! - **Perl Versions**: 5.8 through 5.40 (covers 99% of CPAN)
//! - **LSP Protocol**: LSP 3.17 specification
//! - **Tree-sitter**: Compatible AST format and position tracking
//! - **UTF-16**: Full Unicode support with correct LSP position mapping
//!
//! ## Related Crates
//!
//! - [`perl_lsp`]: Standalone LSP server runtime (moved from this crate)
//! - [`perl_lexer`]: Context-aware Perl tokenizer
//! - [`perl_corpus`]: Comprehensive test corpus and generators
//! - [`perl_dap`]: Debug Adapter Protocol implementation
//!
//! ## Documentation
//!
//! - **API Docs**: See module documentation below
//! - **LSP Guide**: `docs/LSP_IMPLEMENTATION_GUIDE.md`
//! - **Capability Policy**: `docs/LSP_CAPABILITY_POLICY.md`
//! - **Commands**: `docs/COMMANDS_REFERENCE.md`
//! - **Current Status**: `docs/CURRENT_STATUS.md`

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

/// Parser engine components and supporting utilities.
pub mod engine;
/// Legacy module aliases for moved engine components.
pub use engine::{error, parser, position};

/// Abstract Syntax Tree (AST) definitions for Perl parsing.
pub use engine::ast;
/// Experimental second-generation AST (work in progress).
pub use engine::ast_v2;
/// Edit tracking for incremental parsing.
pub use engine::edit;
/// Heredoc content collector with FIFO ordering and indent stripping.
pub use engine::heredoc_collector;
pub use engine::parser::Parser;
/// Parser context with error recovery support.
pub use engine::parser_context;
/// Pragma tracking for `use` and related directives.
pub use engine::pragma_tracker;
/// Parser for Perl quote and quote-like operators.
pub use engine::quote_parser;
#[cfg(not(target_arch = "wasm32"))]
/// Error classification and recovery strategies for parse failures.
pub use error::classifier as error_classifier;
pub use error::recovery as error_recovery;
pub use error::recovery_parser;
/// Parser utilities and helpers.
pub use perl_parser_core::util;

pub use perl_parser_core::line_index;
pub use position::{LineEnding, PositionMapper};

pub mod analysis;
pub mod builtins;
#[cfg(feature = "incremental")]
pub mod incremental;
pub mod refactor;
pub mod tdd;
pub mod tokens;
pub mod tooling;
pub mod workspace;

/// Dead code detection for Perl workspaces.
#[cfg(not(target_arch = "wasm32"))]
pub use analysis::dead_code_detector;
pub use analysis::declaration;
#[cfg(not(target_arch = "wasm32"))]
pub use analysis::index;
/// Scope analysis for variable and subroutine resolution.
pub use analysis::scope_analyzer;
pub use analysis::semantic;
pub use analysis::symbol;
/// Type inference engine for Perl variable analysis.
pub use analysis::type_inference;
pub use builtins::builtin_signatures;
pub use builtins::builtin_signatures_phf;

// Re-exports from extracted microcrates
/// LSP code actions for automated refactoring and fixes.
pub mod code_actions {
    pub use perl_lsp_code_actions::*;
}
pub use perl_lsp_code_actions::EnhancedCodeActionsProvider;
/// LSP completion for code suggestions.
pub mod completion {
    pub use perl_lsp_completion::*;
}
/// LSP diagnostics for error reporting.
pub mod diagnostics {
    pub use perl_lsp_diagnostics::*;
}
/// LSP document links provider for file and URL navigation.
pub mod document_links {
    pub use perl_lsp_navigation::*;
}
/// LSP implementation provider.
pub mod implementation_provider {
    pub use perl_lsp_navigation::*;
}
/// LSP inlay hints for inline type and parameter information.
pub mod inlay_hints {
    pub use perl_lsp_inlay_hints::*;
}
/// LSP inlay hints provider implementation.
pub mod inlay_hints_provider {
    pub use perl_lsp_inlay_hints::*;
}
/// LSP references provider for symbol usage analysis.
pub mod references {
    pub use perl_lsp_navigation::*;
}
/// LSP rename for symbol renaming.
pub mod rename {
    pub use perl_lsp_rename::*;
}
/// LSP semantic tokens provider for syntax highlighting.
pub mod semantic_tokens {
    pub use perl_lsp_semantic_tokens::*;
}
/// LSP semantic tokens provider implementation.
pub mod semantic_tokens_provider {
    pub use perl_lsp_semantic_tokens::*;
}
/// LSP type definition provider.
#[cfg(feature = "lsp-compat")]
pub mod type_definition {
    pub use perl_lsp_navigation::*;
}
/// LSP type hierarchy provider for inheritance navigation.
pub mod type_hierarchy {
    pub use perl_lsp_navigation::*;
}
/// LSP workspace symbols provider.
pub mod workspace_symbols {
    pub use perl_lsp_navigation::*;
}

pub use refactor::import_optimizer;
/// Code modernization utilities for Perl best practices.
pub use refactor::modernize;
/// Enhanced code modernization with refactoring capabilities.
pub use refactor::modernize_refactored;
/// Unified refactoring engine for comprehensive code transformations.
pub use refactor::refactoring;
pub use tokens::token_stream;
pub use tokens::token_wrapper;
pub use tokens::trivia;
pub use tokens::trivia_parser;
pub use tooling::performance;
pub use tooling::perl_critic;
pub use tooling::perltidy;

#[cfg(feature = "incremental")]
pub use incremental::incremental_advanced_reuse;
#[cfg(feature = "incremental")]
pub use incremental::incremental_checkpoint;
#[cfg(feature = "incremental")]
pub use incremental::incremental_document;
#[cfg(feature = "incremental")]
pub use incremental::incremental_edit;
#[cfg(feature = "incremental")]
#[deprecated(note = "LSP server moved to perl-lsp; perl-parser no longer handles didChange")]
pub use incremental::incremental_handler_v2;
#[cfg(feature = "incremental")]
pub use incremental::incremental_integration;
#[cfg(feature = "incremental")]
pub use incremental::incremental_simple;
#[cfg(feature = "incremental")]
pub use incremental::incremental_v2;

pub use tdd::tdd_basic;
/// TDD workflow integration for Test-Driven Development support.
pub use tdd::tdd_workflow;
pub use tdd::test_generator;
/// Test execution and TDD support functionality.
pub use tdd::test_runner;

pub use workspace::document_store;
pub use workspace::workspace_index;
#[cfg(not(target_arch = "wasm32"))]
pub use workspace::workspace_refactor;
pub use workspace::workspace_rename;

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

// =============================================================================
// LSP Feature Exports (DEPRECATED - migrated to perl-lsp crate)
// =============================================================================
// These exports are commented out during the migration period.
// Use `perl_lsp` crate for LSP functionality instead.
//
// pub use code_actions::{CodeAction, CodeActionEdit, CodeActionKind, CodeActionsProvider};
// pub use code_actions_enhanced::EnhancedCodeActionsProvider;
// pub use code_actions_provider::{...};
// pub use code_lens_provider::{CodeLens, CodeLensProvider, ...};
// pub use completion::{CompletionContext, CompletionItem, CompletionItemKind, CompletionProvider};
// pub use diagnostics::{Diagnostic, DiagnosticSeverity, DiagnosticTag, ...};
// pub use document_links::compute_links;
// pub use folding::{FoldingRange, FoldingRangeExtractor, FoldingRangeKind};
// pub use formatting::{CodeFormatter, FormatTextEdit, FormattingOptions};
// pub use inlay_hints::{parameter_hints, trivial_type_hints};
// pub use lsp::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
// pub use lsp_server::LspServer;
// pub use on_type_formatting::compute_on_type_edit;
// pub use rename::{RenameOptions, RenameProvider, RenameResult, TextEdit, apply_rename_edits};
// pub use selection_range::{build_parent_map, selection_chain};
// pub use semantic_tokens::{...};
// pub use semantic_tokens_provider::{...};
// pub use signature_help::{ParameterInfo, SignatureHelp, SignatureHelpProvider, SignatureInfo};
// pub use workspace_symbols::{WorkspaceSymbol, WorkspaceSymbolsProvider};
// =============================================================================

// Engine exports (these stay in perl-parser)
pub use import_optimizer::{
    DuplicateImport, ImportAnalysis, ImportEntry, ImportOptimizer, MissingImport,
    OrganizationSuggestion, SuggestionPriority, UnusedImport,
};
pub use scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue};
pub use test_generator::{
    CoverageReport, Priority, RefactoringCategory, RefactoringSuggester, RefactoringSuggestion,
    TestCase, TestFramework, TestGenerator, TestGeneratorOptions, TestResults, TestRunner,
};
pub use type_inference::{
    PerlType, ScalarType, TypeBasedCompletion, TypeConstraint, TypeEnvironment,
    TypeInferenceEngine, TypeLocation,
};

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
    use perl_tdd_support::must;

    #[test]
    fn test_basic_parsing() {
        let mut parser = Parser::new("my $x = 42;");
        let result = parser.parse();
        assert!(result.is_ok());

        let ast = must(result);
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

            let ast = must(result);
            if let NodeKind::Program { statements } = &ast.kind {
                assert_eq!(statements.len(), 1);
                let is_var_decl =
                    matches!(statements[0].kind, NodeKind::VariableDeclaration { .. });
                assert!(is_var_decl, "Expected VariableDeclaration for: {}", code);
                if let NodeKind::VariableDeclaration { declarator: decl, .. } = &statements[0].kind
                {
                    assert_eq!(decl, declarator);
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

            let ast = must(result);
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

                assert!(
                    binary_node.is_some(),
                    "Expected Binary operator for: {}. Found: {:?}",
                    code,
                    statements[0].kind
                );
                if let Some((op, left, right)) = binary_node {
                    assert_eq!(op, expected_op, "Operator mismatch for: {}", code);

                    // Additional diagnostic information
                    println!("Parsing: {}", code);
                    println!("Left node: {:?}", left);
                    println!("Right node: {:?}", right);
                }
            }
            assert!(
                matches!(ast.kind, NodeKind::Program { .. }),
                "Expected Program node, found: {:?}",
                ast.kind
            );
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

            let ast = must(result);
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

                assert!(
                    binary_node.is_some(),
                    "Expected Binary operator for: {}. Found: {:?}",
                    code,
                    statements[0].kind
                );
                if let Some(op) = binary_node {
                    assert_eq!(op, expected_op, "Operator mismatch for: {}", code);
                }
            }
            assert!(
                matches!(ast.kind, NodeKind::Program { .. }),
                "Expected Program node, found: {:?}",
                ast.kind
            );
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

            let ast = must(result);
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

            // With error recovery, parse() succeeds but collects errors
            assert!(result.is_ok(), "Parser should recover from errors for: {}", code);

            // Check that errors were recorded
            let errors = parser.errors();
            assert!(!errors.is_empty(), "Expected recorded errors for: {}", code);
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
