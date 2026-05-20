//! Parser harness - runs each of the three Perl parsers on a source string
//! and produces a structured [`ParseResult`].
//!
//! Each parser is called through a thin wrapper that:
//! 1. Catches panics with `std::panic::catch_unwind` so one crash never kills
//!    the entire suite.
//! 2. Extracts the S-expression / AST sexp for structural inspection.
//! 3. Records a [`Verdict`] describing the outcome category.
//!
//! The caller is responsible for asserting the *expected* verdict.  The
//! harness only measures; it never asserts.

use std::panic;

use crate::outcomes::Verdict;

/// The output of running one parser on one input.
#[derive(Debug)]
#[non_exhaustive]
pub struct ParseResult {
    /// Which parser produced this result.
    pub parser: ParserLabel,
    /// The source string that was parsed.
    pub source: String,
    /// The verdict - outcome category.
    pub verdict: Verdict,
    /// S-expression or description of the parse output (for diagnostics).
    /// Empty string if the parser crashed or returned no tree.
    pub sexp: String,
    /// Error message if the parser rejected the input (`Verdict::Errors`).
    pub error: Option<String>,
}

impl ParseResult {
    /// Returns `true` if the sexp contains the given substring.
    pub fn sexp_contains(&self, needle: &str) -> bool {
        self.sexp.contains(needle)
    }
}

/// Identifies which parser produced a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParserLabel {
    /// v1: C tree-sitter grammar via FFI (`tree-sitter-perl-c`).
    V1TreeSitterC,
    /// v2: Pest/PEG legacy parser (`perl-parser-pest`).
    V2Pest,
    /// v3: Recursive-descent production parser (`perl-parser-core`).
    V3RecursiveDescent,
}

impl std::fmt::Display for ParserLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1TreeSitterC => write!(f, "v1(tree-sitter-c)"),
            Self::V2Pest => write!(f, "v2(pest)"),
            Self::V3RecursiveDescent => write!(f, "v3(recursive-descent)"),
        }
    }
}

// -- v1: C tree-sitter ---------------------------------------------------------

/// Parse with the v1 C tree-sitter grammar.
///
/// The tree-sitter API always returns a tree - it never returns an error.
/// Instead, `tree.root_node().has_error()` indicates whether the tree
/// contains ERROR nodes.  We classify:
///
/// - `has_error() == false` -> verdict determined by the caller's structural check
/// - `has_error() == true`  -> [`Verdict::Errors`]
/// - panic                   -> [`Verdict::Crashes`]
///
/// The caller receives the raw S-expression regardless of error status.
pub fn parse_v1(source: &str) -> ParseResult {
    let source_owned = source.to_owned();
    let result = panic::catch_unwind(move || {
        use tree_sitter_perl_c::try_parse_perl_code;
        match try_parse_perl_code(&source_owned) {
            Ok(tree) => {
                let sexp = tree.root_node().to_sexp();
                let has_error = tree.root_node().has_error();
                (sexp, has_error, None::<String>)
            }
            Err(e) => {
                // try_parse_perl_code only fails on language-setup or None-return
                let msg = format!("{e}");
                (String::new(), true, Some(msg))
            }
        }
    });

    match result {
        Err(_panic) => ParseResult {
            parser: ParserLabel::V1TreeSitterC,
            source: source.to_owned(),
            verdict: Verdict::Crashes,
            sexp: String::new(),
            error: Some("v1 panicked".to_owned()),
        },
        Ok((sexp, has_error, err_msg)) => {
            let verdict = if err_msg.is_some() || has_error {
                Verdict::Errors
            } else {
                // Caller inspects structural properties and refines verdict
                Verdict::Correct
            };
            ParseResult {
                parser: ParserLabel::V1TreeSitterC,
                source: source.to_owned(),
                verdict,
                sexp,
                error: err_msg,
            }
        }
    }
}

// -- v2: Pest parser -----------------------------------------------------------

/// Parse with the v2 Pest legacy parser.
///
/// Returns an `Ok(AstNode)` or `Err(ParseError)`.  We capture the S-expression
/// and return the raw result; the caller classifies the verdict.
pub fn parse_v2(source: &str) -> ParseResult {
    let source_owned = source.to_owned();
    let result = panic::catch_unwind(move || {
        use perl_parser_pest::PureRustPerlParser;
        let mut parser = PureRustPerlParser::new();
        match parser.parse(&source_owned) {
            Ok(ast) => {
                let sexp = parser.to_sexp(&ast);
                (Some(sexp), None::<String>, Some(ast))
            }
            Err(e) => {
                let msg = format!("{e}");
                (None, Some(msg), None)
            }
        }
    });

    match result {
        Err(_panic) => ParseResult {
            parser: ParserLabel::V2Pest,
            source: source.to_owned(),
            verdict: Verdict::Crashes,
            sexp: String::new(),
            error: Some("v2 panicked".to_owned()),
        },
        Ok((Some(sexp), None, _ast)) => ParseResult {
            parser: ParserLabel::V2Pest,
            source: source.to_owned(),
            // Caller refines: may be Correct, WrongButPlausible, or SilentlyEmpty
            verdict: Verdict::Correct,
            sexp,
            error: None,
        },
        Ok((_, Some(err), _)) => ParseResult {
            parser: ParserLabel::V2Pest,
            source: source.to_owned(),
            verdict: Verdict::Errors,
            sexp: String::new(),
            error: Some(err),
        },
        Ok((None, None, _)) => ParseResult {
            parser: ParserLabel::V2Pest,
            source: source.to_owned(),
            verdict: Verdict::Errors,
            sexp: String::new(),
            error: Some("parse returned neither Ok nor Err".to_owned()),
        },
    }
}

// -- v3: Recursive-descent parser ---------------------------------------------

/// Parse with the v3 recursive-descent production parser.
///
/// v3 is highly error-tolerant: it almost always returns an AST.  Structural
/// errors are reported via `parser.errors()`, but the AST is still produced.
/// We use `parse_with_recovery()` to get both the tree and diagnostics.
pub fn parse_v3(source: &str) -> ParseResult {
    let source_owned = source.to_owned();
    let result = panic::catch_unwind(move || {
        use perl_parser_core::Parser;
        let mut parser = Parser::new(&source_owned);
        let output = parser.parse_with_recovery();
        let sexp = output.ast.to_sexp();
        let has_errors = !output.diagnostics.is_empty();
        (sexp, has_errors)
    });

    match result {
        Err(_panic) => ParseResult {
            parser: ParserLabel::V3RecursiveDescent,
            source: source.to_owned(),
            verdict: Verdict::Crashes,
            sexp: String::new(),
            error: Some("v3 panicked".to_owned()),
        },
        Ok((sexp, has_errors)) => ParseResult {
            parser: ParserLabel::V3RecursiveDescent,
            source: source.to_owned(),
            // Caller refines based on structural inspection.
            // has_errors indicates diagnostic messages but AST is always present.
            verdict: if has_errors {
                // Non-fatal errors: tree was produced but with error nodes
                Verdict::Errors
            } else {
                Verdict::Correct
            },
            sexp,
            error: None,
        },
    }
}
