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

/// Collect semantic tokens from parsed Perl script AST for LSP highlighting
///
/// Analyzes the provided AST and source text to generate semantic tokens suitable
/// for LSP client consumption. Combines lexer-based token classification with
/// AST-based semantic analysis to provide comprehensive syntax highlighting for
/// Perl script content within the LSP Complete stage.
///
/// # Arguments
///
/// * `ast` - Parsed Perl script AST containing semantic information
/// * `text` - Original source text of the Perl script for token extraction
/// * `to_pos16` - Position conversion function mapping byte offsets to LSP coordinates
///
/// # Returns
///
/// Vector of encoded semantic tokens ready for LSP transmission, sorted by
/// position and delta-encoded according to LSP specification.
///
/// # Performance
///
/// - Time complexity: O(n) where n is the number of tokens in the Perl script
/// - Memory usage: O(n) for token storage and classification
/// - Optimized for large Perl scripts found in enterprise development workflows
/// - Thread-safe operation suitable for concurrent Perl parsing
///
/// # Examples
///
/// ```rust
/// use perl_lsp_providers::{Parser, ide::lsp_compat::semantic_tokens::collect_semantic_tokens};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let script = "my $data_filter = qr/valid/;";
/// let mut parser = Parser::new(script);
/// let ast = parser.parse()?;
///
/// let pos_mapper = |pos| (0u32, pos as u32); // Simple line-based mapping
/// let tokens = collect_semantic_tokens(&ast, script, &pos_mapper);
///
/// assert!(!tokens.is_empty());
/// // Tokens are delta-encoded for LSP transmission
/// # Ok(())
/// # }
/// ```
///
/// # Email Processing Context
///
/// This function is particularly effective for highlighting:
/// - Email filtering and routing logic with regular expressions
/// - Email template processing code with variable interpolation
/// - Configuration scripts embedded in Perl code
/// - Message processing automation with proper keyword highlighting
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
}
