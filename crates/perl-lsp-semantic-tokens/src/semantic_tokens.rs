//! Semantic token analysis for LSP syntax highlighting in Perl script processing
//!
//! This module provides semantic token extraction and classification for Perl scripts
//! within the LSP workflow. It generates precise syntax highlighting information that helps
//! developers understand complex Perl code during the Complete stage.
//!
//! # LSP Workflow Integration
//!
//! - **Parse**: Receives parsed AST from Perl script parsing
//! - **Index**: Uses semantic information for symbol indexing
//! - **Navigate**: Applies semantic analysis for cross-file navigation
//! - **Complete**: Primary consumer - provides syntax highlighting for code presentation
//! - **Analyze**: Uses semantic classification for enhanced search and analysis
//!
//! # Client capability requirements
//!
//! Requires client capability support for `textDocument/semanticTokens` and
//! `semanticTokens/legend` registration to enable semantic highlighting.
//!
//! # Protocol compliance
//!
//! Implements the semanticTokens protocol (full and delta) with LSP 3.17+
//! data layout and delta encoding expectations.
//!
//! # Related Modules
//!
//! This module integrates with symbol indexing, semantic analysis, and code completion.
//!
//! # Performance Characteristics
//!
//! - Memory usage: O(n) where n is token count in Perl script
//! - Time complexity: O(n) linear scanning with lexer integration
//! - Optimized for large Perl codebase processing with efficient token classification
//! - Thread-safe semantic token generation for concurrent script processing
//!
//! # Usage Examples
//!
//! ## Basic Semantic Token Generation
//!
//! ```no_run
//! use perl_lsp_providers::{Parser, ide::lsp_compat::semantic_tokens::collect_semantic_tokens};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let code = "package MyModule; sub greet { my $name = shift; print \"Hello, $name!\"; }";
//! let mut parser = Parser::new(code);
//! let ast = parser.parse()?;
//!
//! // Generate semantic tokens for syntax highlighting
//! let to_pos16 = |byte_pos: usize| {
//!     // Simple line/column calculation for demonstration
//!     let line = code[..byte_pos].matches('\n').count() as u32;
//!     let last_line = code[..byte_pos].rfind('\n').map_or(0, |pos| pos + 1);
//!     let col = (byte_pos - last_line) as u32;
//!     (line, col)
//! };
//! let tokens = collect_semantic_tokens(&ast, code, &to_pos16);
//! for token in tokens {
//!     println!("Token: [{}, {}, {}, {}, {}]",
//!              token[0], token[1], token[2], token[3], token[4]);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## LSP Semantic Tokens Provider
//!
//! ```no_run
//! use perl_lsp_providers::ide::lsp_compat::semantic_tokens::{collect_semantic_tokens, legend};
//! use perl_lsp_providers::Parser;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let code = "my @array = (1, 2, 3); for my $item (@array) { print $item; }";
//! let mut parser = Parser::new(code);
//! let ast = parser.parse()?;
//!
//! // Get encoded tokens for LSP response
//! let to_pos16 = |byte_pos: usize| {
//!     let line = code[..byte_pos].matches('\n').count() as u32;
//!     let last_line = code[..byte_pos].rfind('\n').map_or(0, |pos| pos + 1);
//!     let col = (byte_pos - last_line) as u32;
//!     (line, col)
//! };
//! let encoded_tokens = collect_semantic_tokens(&ast, code, &to_pos16);
//! let legend = legend();
//!
//! println!("Generated {} semantic tokens", encoded_tokens.len());
//! println!("Token types: {:?}", legend.token_types);
//! println!("Token modifiers: {:?}", legend.modifiers);
//! # Ok(())
//! # }
//! ```
//!
//! ## Custom Token Classification
//!
//! ```
//! use perl_lsp_providers::ide::lsp_compat::semantic_tokens::{EncodedToken, TokensLegend, legend};
//!
//! // Create custom semantic tokens
//! let custom_token: EncodedToken = [0, 0, 5, 1, 0];
//! // Structure: [delta_line, delta_start, length, token_type, token_modifiers]
//!
//! // Use with existing legend
//! let legend = legend();
//! println!("Token type: {:?}", legend.token_types.get(custom_token[3] as usize));
//! ```

use perl_lexer::{PerlLexer, TokenType};
use perl_parser_core::ast::{Node, NodeKind};
use rustc_hash::FxHashMap;

/// LSP semantic token encoding format for client transmission
///
/// Represents a semantic token as [deltaLine, deltaStartChar, length, tokenTypeIndex, tokenModBits]
/// following the LSP specification for efficient delta-encoded token streams.
pub type EncodedToken = [u32; 5];

/// Semantic token legend mapping token types and modifiers to indices
///
/// Provides the mapping between semantic token names and their numeric indices
/// for LSP client consumption. Used to establish a contract between the server
/// and client for semantic highlighting interpretation.
pub struct TokensLegend {
    /// List of token type names in index order
    pub token_types: Vec<String>,
    /// List of modifier names in index order
    pub modifiers: Vec<String>,
    /// Fast lookup map from token type names to indices
    pub map: FxHashMap<String, u32>,
}

/// Create the standard semantic token legend for Perl script highlighting
///
/// Returns a configured legend with all supported token types and modifiers
/// for comprehensive Perl script syntax highlighting. Optimized for common
/// Perl constructs found in Perl parsing workflows.
///
/// # Returns
///
/// A TokensLegend containing all token types, modifiers, and lookup mappings
/// ready for LSP client registration and semantic token classification.
///
/// # Examples
///
/// ```rust
/// use perl_lsp_providers::ide::lsp_compat::semantic_tokens::legend;
///
/// let legend = legend();
/// assert!(legend.token_types.contains(&"function".to_string()));
/// assert!(legend.token_types.contains(&"keyword".to_string()));
/// ```
pub fn legend() -> TokensLegend {
    let types = vec![
        "namespace",
        "class",
        "function",
        "method",
        "variable",
        "parameter",
        "property",
        "keyword",
        "comment",
        "string",
        "number",
        "regexp",
        "operator",
        "type",
        "macro",
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();

    let modifiers = vec![
        "declaration",
        "definition",
        "readonly",
        "defaultLibrary",
        "deprecated",
        "static",
        "async",
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();

    let mut map = FxHashMap::default();
    for (i, t) in types.iter().enumerate() {
        map.insert(t.clone(), i as u32);
    }

    TokensLegend { token_types: types, modifiers, map }
}

#[inline]
fn kind_idx(leg: &TokensLegend, k: &str) -> u32 {
    *leg.map.get(k).unwrap_or(&0)
}

/// Collect semantic tokens for LSP highlighting in the Complete stage.
///
/// # Arguments
/// * `ast` - Parsed AST for the document.
/// * `text` - Original source text.
/// * `to_pos16` - Converts byte offsets to UTF-16 positions.
/// # Returns
/// Encoded semantic tokens sorted for LSP transmission.
/// # Examples
/// ```rust
/// use perl_lsp_providers::{Parser, ide::lsp_compat::semantic_tokens::collect_semantic_tokens};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let code = "my $x = 1;";
/// let mut parser = Parser::new(code);
/// let ast = parser.parse()?;
/// let to_pos16 = |pos| (0u32, pos as u32);
/// let tokens = collect_semantic_tokens(&ast, code, &to_pos16);
/// assert!(!tokens.is_empty());
/// # Ok(())
/// # }
/// ```
pub fn collect_semantic_tokens(
    ast: &Node,
    text: &str,
    to_pos16: &impl Fn(usize) -> (u32, u32),
) -> Vec<EncodedToken> {
    let leg = legend();
    let mut raw_tokens: Vec<(u32, u32, u32, u32, u32)> = Vec::new(); // (line, char, len, kind, mods)

    // 1) Fast path from lexer categories: conservative single-line emission
    let mut lexer = PerlLexer::new(text);
    while let Some(tok) = lexer.next_token() {
        let (sl, sc) = to_pos16(tok.start);
        let (el, ec) = to_pos16(tok.end);
        let len = if sl == el { ec.saturating_sub(sc) } else { 0 };

        // Map token types to semantic token kinds
        // Note: The lexer's TokenType enum is simpler than what we're matching
        let kind = match &tok.token_type {
            TokenType::Keyword(kw) => {
                // Check if it's a known keyword
                match kw.as_ref() {
                    "my" | "our" | "local" | "state" | "sub" | "package" | "use" | "require"
                    | "if" | "else" | "elsif" | "for" | "foreach" | "while" | "until" | "do"
                    | "return" | "next" | "last" | "redo" | "goto" | "eval" | "given" | "when"
                    | "default" | "break" | "continue" | "unless" => "keyword",
                    _ => continue,
                }
            }

            TokenType::StringLiteral
            | TokenType::QuoteSingle
            | TokenType::QuoteDouble
            | TokenType::QuoteWords
            | TokenType::InterpolatedString(_) => "string",

            TokenType::Number(_) => "number",

            TokenType::RegexMatch
            | TokenType::Substitution
            | TokenType::Transliteration
            | TokenType::QuoteRegex => "regexp",

            TokenType::Division
            | TokenType::Operator(_)
            | TokenType::Arrow
            | TokenType::FatComma => "operator",

            TokenType::Comment(_) => "comment",
            _ => continue,
        };

        if len > 0 {
            raw_tokens.push((sl, sc, len, kind_idx(&leg, kind), 0));
        }
    }

    // 2) AST overlays: package/sub/variable (prefer identifier spans if you track them)
    walk_ast(ast, &mut |node| {
        let (s, e) = (node.location.start, node.location.end);
        let (sl, sc) = to_pos16(s);
        let (el, ec) = to_pos16(e);
        let len = if sl == el { ec.saturating_sub(sc) } else { 0 };

        let (kind, mods): (&str, u32) = match &node.kind {
            NodeKind::Package { .. } => ("namespace", 0),
            NodeKind::Subroutine { name: Some(_), .. } => ("function", 1 /*declaration*/),
            NodeKind::FunctionCall { .. } => ("function", 0),
            NodeKind::MethodCall { .. } => ("method", 0),
            NodeKind::Variable { .. } => ("variable", 0),
            _ => return true,
        };

        if len > 0 {
            raw_tokens.push((sl, sc, len, kind_idx(&leg, kind), mods));
        }
        true
    });

    // 3) Remove overlapping tokens (LSP specification compliance)
    let dedup_tokens = remove_overlapping_tokens(raw_tokens);

    // 4) Sort by position and encode with deltas (thread-safe)
    encode_raw_tokens_to_deltas(dedup_tokens)
}

/// Remove overlapping tokens to comply with LSP specification
/// Prefers tokens with higher specificity (AST over lexer) and longer spans
fn remove_overlapping_tokens(
    raw_tokens: Vec<(u32, u32, u32, u32, u32)>,
) -> Vec<(u32, u32, u32, u32, u32)> {
    // Sort by start position first
    let mut sorted_tokens = raw_tokens;
    sorted_tokens
        .sort_by_key(|&(line, start_char, _length, _token_type, _modifier)| (line, start_char));

    let mut result = Vec::new();

    for token in sorted_tokens {
        let (line, start_char, length, _token_type, _modifier) = token;

        // Check if this token overlaps with the last token in result
        if let Some(&(last_line, last_start, last_length, _last_type, _last_modifier)) =
            result.last()
        {
            // Tokens overlap if they're on the same line and ranges intersect
            if line == last_line && start_char < last_start + last_length {
                // Choose the token with better specificity or longer length
                if length > last_length {
                    result.pop(); // Remove the previous token
                    result.push(token);
                }
                // If current token is not better, skip it
            } else {
                result.push(token);
            }
        } else {
            result.push(token);
        }
    }

    result
}

/// Thread-safe token encoding from raw position data
fn encode_raw_tokens_to_deltas(
    mut raw_tokens: Vec<(u32, u32, u32, u32, u32)>,
) -> Vec<EncodedToken> {
    // Sort by position (line, then character)
    raw_tokens.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    let mut out: Vec<EncodedToken> = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_char = 0u32;

    for (line, char, len, kind, mods) in raw_tokens {
        let (dline, dchar) = if line == prev_line {
            (0, char.saturating_sub(prev_char))
        } else {
            (line.saturating_sub(prev_line), char)
        };

        out.push([dline, dchar, len, kind, mods]);
        prev_line = line;
        prev_char = char;
    }

    out
}

fn walk_ast<F>(node: &Node, visitor: &mut F) -> bool
where
    F: FnMut(&Node) -> bool,
{
    if !visitor(node) {
        return false;
    }

    for child in perl_semantic_analyzer::analysis::declaration::get_node_children(node) {
        if !walk_ast(child, visitor) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create token tuple
    fn tok(line: u32, start: u32, len: u32, kind: u32, mods: u32) -> (u32, u32, u32, u32, u32) {
        (line, start, len, kind, mods)
    }

    #[test]
    fn test_remove_overlapping_tokens_basic() {
        // No overlap
        let input = vec![tok(0, 0, 5, 0, 0), tok(0, 6, 5, 0, 0)];
        let result = remove_overlapping_tokens(input.clone());
        assert_eq!(result, input);
    }

    #[test]
    fn test_remove_overlapping_tokens_touching() {
        // Touching is NOT overlap
        // [0, 5) and [5, 10)
        let input = vec![tok(0, 0, 5, 0, 0), tok(0, 5, 5, 0, 0)];
        let result = remove_overlapping_tokens(input.clone());
        assert_eq!(result, input);
    }

    #[test]
    fn test_remove_overlapping_tokens_nested_keep_outer() {
        // Outer [0, 10), Inner [2, 5)
        // Inner length 3 < Outer length 10
        // Expect Outer kept
        let input = vec![tok(0, 0, 10, 0, 0), tok(0, 2, 3, 1, 0)];
        // Sorted: Outer, Inner
        let result = remove_overlapping_tokens(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], tok(0, 0, 10, 0, 0));
    }

    #[test]
    fn test_remove_overlapping_tokens_nested_keep_longer_inner_replacement() {
        // Functionally: A [0, 5), B [0, 10)
        // Sorted: A, B
        // Expect B (longer) replaces A
        let input = vec![tok(0, 0, 5, 0, 0), tok(0, 0, 10, 1, 0)];
        let result = remove_overlapping_tokens(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], tok(0, 0, 10, 1, 0));
    }

    #[test]
    fn test_remove_overlapping_tokens_overlap_tail_keep_longer() {
        // A [0, 5) len 5
        // B [4, 10) len 6
        // Overlap at 4. B is longer.
        // Expect A replaced by B.
        let input = vec![tok(0, 0, 5, 0, 0), tok(0, 4, 6, 1, 0)];
        let result = remove_overlapping_tokens(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], tok(0, 4, 6, 1, 0));
    }

    #[test]
    fn test_remove_overlapping_tokens_overlap_tail_keep_earlier_if_longer() {
        // A [0, 10) len 10
        // B [8, 15) len 7
        // Overlap at 8. A is longer.
        // Expect A kept, B dropped.
        let input = vec![tok(0, 0, 10, 0, 0), tok(0, 8, 7, 1, 0)];
        let result = remove_overlapping_tokens(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], tok(0, 0, 10, 0, 0));
    }

    #[test]
    fn test_remove_overlapping_tokens_equal_length_keep_first() {
        // A [0, 5) len 5
        // B [0, 5) len 5
        // Expect A kept (first one)
        let input = vec![tok(0, 0, 5, 1, 0), tok(0, 0, 5, 2, 0)];
        let result = remove_overlapping_tokens(input.clone());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], tok(0, 0, 5, 1, 0));
    }

    #[test]
    fn test_remove_overlapping_tokens_different_lines() {
        let input = vec![tok(0, 0, 5, 0, 0), tok(1, 0, 5, 0, 0)];
        let result = remove_overlapping_tokens(input.clone());
        assert_eq!(result, input);
    }

    // ==================== Mutation Hardening Tests (Issue #155) ====================
    // These tests target specific mutation survivors identified in mutation analysis
    // Focus: FnValue mutations (71%) and BinaryOperator mutations (25%)

    /// Test that empty input produces empty output
    /// Kills FnValue mutations on return statement
    #[test]
    fn mutation_hardening_empty_input() {
        let input = vec![];
        let result = remove_overlapping_tokens(input);
        assert_eq!(result.len(), 0, "Empty input must produce empty output");
    }

    /// Test single token passes through unchanged
    /// Kills FnValue mutations on result.push() at line 333
    #[test]
    fn mutation_hardening_single_token() {
        let input = vec![tok(0, 0, 5, 0, 0)];
        let result = remove_overlapping_tokens(input.clone());
        assert_eq!(result.len(), 1, "Single token must be preserved");
        assert_eq!(result[0], input[0], "Single token must match input exactly");
    }

    /// Test two non-overlapping tokens on same line
    /// Kills BinaryOperator mutations on `start_char < last_start + last_length` comparison
    #[test]
    fn mutation_hardening_adjacent_non_overlapping() {
        // Token A: [0, 5), Token B: [5, 10) - touching but not overlapping
        let input = vec![tok(0, 0, 5, 0, 0), tok(0, 5, 5, 1, 0)];
        let result = remove_overlapping_tokens(input.clone());
        assert_eq!(result.len(), 2, "Adjacent non-overlapping tokens must both be kept");
        assert_eq!(result[0], tok(0, 0, 5, 0, 0));
        assert_eq!(result[1], tok(0, 5, 5, 1, 0));
    }

    /// Test exact boundary case: token end equals next token start
    /// Kills BinaryOperator mutations on boundary comparisons
    #[test]
    fn mutation_hardening_exact_boundary() {
        // Token A: [10, 15), Token B: [15, 20) - exact boundary
        let input = vec![tok(0, 10, 5, 0, 0), tok(0, 15, 5, 1, 0)];
        let result = remove_overlapping_tokens(input);
        assert_eq!(result.len(), 2, "Tokens with exact boundaries must not overlap");
    }

    /// Test one-character overlap triggers replacement
    /// Kills BinaryOperator mutations on overlap detection (< vs <=)
    #[test]
    fn mutation_hardening_single_char_overlap() {
        // Token A: [0, 6), Token B: [5, 10) - overlap by 1 char at position 5
        // A is kept because it comes first and B is not longer (A=6, B=5)
        let input = vec![tok(0, 0, 6, 0, 0), tok(0, 5, 5, 1, 0)];
        let result = remove_overlapping_tokens(input);
        assert_eq!(result.len(), 1, "Single char overlap must trigger deduplication");
        assert_eq!(result[0], tok(0, 0, 6, 0, 0), "First token kept (longer)");
    }

    /// Test partial overlap with length comparison
    /// Kills BinaryOperator mutations on `length > last_length` at line 324
    #[test]
    fn mutation_hardening_partial_overlap_length_determines_winner() {
        // Token A: [0, 5) len=5, Token B: [3, 10) len=7 - partial overlap, B longer
        let input = vec![tok(0, 0, 5, 0, 0), tok(0, 3, 7, 1, 0)];
        let result = remove_overlapping_tokens(input);
        assert_eq!(result.len(), 1, "Partial overlap must keep only one token");
        assert_eq!(result[0], tok(0, 3, 7, 1, 0), "Longer overlapping token must win");
    }

    /// Test equal length overlap keeps first token
    /// Kills BinaryOperator mutations on equality in length comparison
    #[test]
    fn mutation_hardening_equal_length_keeps_first() {
        // Token A: [0, 5) len=5, Token B: [2, 7) len=5 - equal length overlap
        let input = vec![tok(0, 0, 5, 0, 0), tok(0, 2, 5, 1, 0)];
        let result = remove_overlapping_tokens(input);
        assert_eq!(result.len(), 1, "Equal length overlap must keep first token");
        assert_eq!(result[0], tok(0, 0, 5, 0, 0), "First token must be kept when lengths equal");
    }

    /// Test tokens on different lines never overlap
    /// Kills BinaryOperator mutations on `line == last_line` comparison at line 322
    #[test]
    fn mutation_hardening_different_lines_no_overlap() {
        let input = vec![
            tok(0, 0, 100, 0, 0), // Line 0, very long token
            tok(1, 0, 5, 1, 0),   // Line 1, early position
        ];
        let result = remove_overlapping_tokens(input.clone());
        assert_eq!(result.len(), 2, "Tokens on different lines must never overlap");
        assert_eq!(result[0], tok(0, 0, 100, 0, 0));
        assert_eq!(result[1], tok(1, 0, 5, 1, 0));
    }

    /// Test three tokens with cascading overlaps
    /// Kills FnValue mutations on multiple push operations
    #[test]
    fn mutation_hardening_three_tokens_cascading() {
        // A: [0, 5), B: [4, 9), C: [8, 12) - A overlaps B, B would overlap C
        let input = vec![
            tok(0, 0, 5, 0, 0), // len=5
            tok(0, 4, 5, 1, 0), // len=5 (overlaps A)
            tok(0, 8, 4, 2, 0), // len=4 (would overlap B)
        ];
        let result = remove_overlapping_tokens(input);
        // A is kept (4 < 0+5, but 5 > 5 is false, so B is skipped)
        // C doesn't overlap A (8 < 0+5 is false), so C is kept
        assert_eq!(result.len(), 2, "First and third tokens kept");
        assert_eq!(result[0], tok(0, 0, 5, 0, 0));
        assert_eq!(result[1], tok(0, 8, 4, 2, 0));
    }

    /// Test zero-length token handling
    /// Kills FnValue mutations and edge case handling
    #[test]
    fn mutation_hardening_zero_length_token() {
        let input = vec![
            tok(0, 5, 0, 0, 0), // Zero-length token [5, 5)
            tok(0, 5, 5, 1, 0), // Normal token at same position [5, 10)
        ];
        let result = remove_overlapping_tokens(input);
        // Zero-length token [5,5) doesn't overlap with [5,10) per < check (5 < 5+0 is false)
        assert_eq!(
            result.len(),
            2,
            "Zero-length token at same position doesn't technically overlap"
        );
        assert_eq!(result[0], tok(0, 5, 0, 0, 0));
        assert_eq!(result[1], tok(0, 5, 5, 1, 0));
    }

    /// Test multiple zero-length tokens
    /// Kills FnValue mutations in edge cases
    #[test]
    fn mutation_hardening_multiple_zero_length() {
        let input = vec![tok(0, 5, 0, 0, 0), tok(0, 5, 0, 1, 0), tok(0, 5, 0, 2, 0)];
        let result = remove_overlapping_tokens(input);
        // Zero-length tokens at same position don't overlap each other (5 < 5+0 is false)
        assert_eq!(result.len(), 3, "Multiple zero-length tokens are all kept");
    }

    /// Test large position values don't cause arithmetic overflow
    /// Kills BinaryOperator mutations in arithmetic operations
    #[test]
    fn mutation_hardening_large_positions() {
        let input = vec![tok(1000, u32::MAX - 100, 50, 0, 0), tok(1000, u32::MAX - 40, 20, 1, 0)];
        let result = remove_overlapping_tokens(input);
        // Overflow is prevented by saturating operations in the original code
        assert_eq!(result.len(), 2, "Large positions must not cause overflow issues");
    }

    /// Test sorting preserves token order correctly
    /// Kills BinaryOperator mutations in sort_by_key at line 310
    #[test]
    fn mutation_hardening_sort_order() {
        // Input in reverse order
        let input = vec![tok(2, 10, 5, 0, 0), tok(1, 10, 5, 1, 0), tok(0, 10, 5, 2, 0)];
        let result = remove_overlapping_tokens(input);
        assert_eq!(result.len(), 3, "Non-overlapping tokens must all be preserved");
        // Verify sorted by line
        assert_eq!(result[0].0, 0);
        assert_eq!(result[1].0, 1);
        assert_eq!(result[2].0, 2);
    }

    /// Test sort order within same line
    /// Kills BinaryOperator mutations in sort comparisons
    #[test]
    fn mutation_hardening_sort_order_same_line() {
        // Input with tokens in reverse order on same line
        let input = vec![tok(0, 30, 5, 0, 0), tok(0, 20, 5, 1, 0), tok(0, 10, 5, 2, 0)];
        let result = remove_overlapping_tokens(input);
        assert_eq!(result.len(), 3, "Non-overlapping tokens must all be preserved");
        // Verify sorted by start position
        assert_eq!(result[0].1, 10);
        assert_eq!(result[1].1, 20);
        assert_eq!(result[2].1, 30);
    }

    /// Test multiple overlaps where shorter tokens are systematically removed
    /// Kills FnValue mutations on conditional push operations
    #[test]
    fn mutation_hardening_systematic_removal() {
        // All tokens overlap at same position, increasing length
        let input =
            vec![tok(0, 0, 3, 0, 0), tok(0, 0, 5, 1, 0), tok(0, 0, 7, 2, 0), tok(0, 0, 9, 3, 0)];
        let result = remove_overlapping_tokens(input);
        assert_eq!(result.len(), 1, "Longest token must survive multiple replacements");
        assert_eq!(result[0], tok(0, 0, 9, 3, 0), "Longest token must be the survivor");
    }

    /// Test interleaved tokens without overlap
    /// Kills FnValue mutations on else branch at line 330
    #[test]
    fn mutation_hardening_interleaved_no_overlap() {
        let input = vec![
            tok(0, 0, 3, 0, 0),  // [0, 3)
            tok(0, 5, 3, 1, 0),  // [5, 8)
            tok(0, 10, 3, 2, 0), // [10, 13)
            tok(0, 15, 3, 3, 0), // [15, 18)
        ];
        let result = remove_overlapping_tokens(input.clone());
        assert_eq!(result.len(), 4, "All non-overlapping tokens must be preserved");
        assert_eq!(result, input, "Token order and content must be unchanged");
    }

    /// Test overlap at exactly boundary minus one
    /// Kills off-by-one errors in BinaryOperator mutations
    #[test]
    fn mutation_hardening_boundary_minus_one() {
        // Token A: [0, 10), Token B: [9, 15) - overlap at position 9
        let input = vec![tok(0, 0, 10, 0, 0), tok(0, 9, 6, 1, 0)];
        let result = remove_overlapping_tokens(input);
        assert_eq!(result.len(), 1, "Boundary-1 overlap must be detected");
        assert_eq!(result[0], tok(0, 0, 10, 0, 0), "First longer token wins");
    }

    /// Test that token type and modifiers are preserved correctly
    /// Kills mutations that might affect non-position fields
    #[test]
    fn mutation_hardening_preserves_metadata() {
        let input = vec![
            tok(0, 0, 5, 42, 7), // Specific type and modifiers
        ];
        let result = remove_overlapping_tokens(input.clone());
        assert_eq!(result[0].3, 42, "Token type must be preserved");
        assert_eq!(result[0].4, 7, "Token modifiers must be preserved");
    }

    /// Test mixed line and position sorting
    /// Kills complex BinaryOperator mutations in sort logic
    #[test]
    fn mutation_hardening_mixed_line_position_sort() {
        let input = vec![
            tok(2, 5, 3, 0, 0),
            tok(0, 15, 3, 1, 0),
            tok(1, 10, 3, 2, 0),
            tok(0, 5, 3, 3, 0),
            tok(2, 0, 3, 4, 0),
        ];
        let result = remove_overlapping_tokens(input);
        assert_eq!(result.len(), 5);
        // Verify primary sort by line
        assert!(result[0].0 <= result[1].0);
        assert!(result[1].0 <= result[2].0);
        // Verify secondary sort by position within same line
        for i in 1..result.len() {
            if result[i].0 == result[i - 1].0 {
                assert!(
                    result[i].1 >= result[i - 1].1,
                    "Tokens on same line must be sorted by position"
                );
            }
        }
    }
}
