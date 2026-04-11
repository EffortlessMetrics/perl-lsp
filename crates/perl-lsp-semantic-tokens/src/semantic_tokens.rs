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
//! ```ignore
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
//! ```ignore
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
//! ```ignore
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

use perl_lexer::{PerlLexer, StringPart, TokenType};
use perl_parser_core::ast::{Node, NodeKind};
use regex::Regex;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::sync::LazyLock;

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
/// ```rust,ignore
/// use perl_lsp_providers::ide::lsp_compat::semantic_tokens::legend;
///
/// let legend = legend();
/// assert!(legend.token_types.contains(&"function".to_string()));
/// assert!(legend.token_types.contains(&"keyword".to_string()));
/// ```
pub fn legend() -> TokensLegend {
    // IMPORTANT: this ordering must exactly match the token_types vec in
    // `perl-lsp-protocol/src/capabilities.rs` `capabilities_for()`.
    // Clients decode emitted tokenType indices using the advertised legend;
    // any ordering mismatch renders every token with the wrong colour.
    let types = vec![
        "namespace",           // 0
        "type",                // 1
        "class",               // 2
        "interface",           // 3
        "enum",                // 4
        "enumMember",          // 5
        "typeParameter",       // 6
        "function",            // 7
        "method",              // 8
        "property",            // 9
        "macro",               // 10
        "variable",            // 11
        "parameter",           // 12
        "keyword",             // 13
        "modifier",            // 14
        "comment",             // 15
        "string",              // 16
        "number",              // 17
        "regexp",              // 18
        "operator",            // 19
        "sql_string",          // 20 — DBI/SQL string context (Issue #2337)
        "sql_heredoc_keyword", // 21 — SQL keyword inside <<SQL heredoc (Issue #2059)
        "json_heredoc_key",    // 22 — JSON object key inside <<JSON heredoc (Issue #2059)
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();

    // IMPORTANT: this ordering must exactly match the token_modifiers vec in
    // `perl-lsp-protocol/src/capabilities.rs` `capabilities_for()`.
    // Modifier bitmasks (1 << bit_position) are decoded using the advertised legend.
    // Each modifier's numeric value is 2^bit_position (e.g., defaultLibrary at bit 9 = 512).
    let modifiers = vec![
        "declaration",    // bit 0  → 1
        "definition",     // bit 1  → 2
        "readonly",       // bit 2  → 4
        "static",         // bit 3  → 8
        "deprecated",     // bit 4  → 16
        "abstract",       // bit 5  → 32
        "async",          // bit 6  → 64
        "modification",   // bit 7  → 128
        "documentation",  // bit 8  → 256
        "defaultLibrary", // bit 9  → 512
        "scalarVariable", // bit 10 → 1024
        "arrayVariable",  // bit 11 → 2048
        "hashVariable",   // bit 12 → 4096
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

// ---------------------------------------------------------------------------
// Heredoc language injection helpers (Issue #2059)
// ---------------------------------------------------------------------------

/// Regex matching SQL keywords (case-insensitive).
///
/// Matches word-boundary-delimited SQL keywords in a heredoc body and emits
/// `sql_heredoc_keyword` semantic tokens for each match.
static SQL_KW_RE: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(SELECT|FROM|WHERE|AND|OR|NOT|IN|IS|NULL|LIKE|BETWEEN|JOIN|INNER|LEFT|RIGHT|OUTER|FULL|CROSS|ON|AS|DISTINCT|GROUP|BY|ORDER|HAVING|LIMIT|OFFSET|UNION|ALL|INSERT|INTO|VALUES|UPDATE|SET|DELETE|CREATE|DROP|ALTER|TABLE|INDEX|VIEW|RETURNING|WITH|CASE|WHEN|THEN|ELSE|END|EXISTS|EXCEPT|INTERSECT)\b"
    ).ok()
});

/// Regex matching JSON object keys: `"key":`.
static JSON_KEY_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#""([^"\\]|\\.)*"\s*:"#).ok());

/// Determine the injection language for a heredoc start token.
///
/// `text` is the full heredoc start token text (e.g. `<<SQL`, `<<'SQL'`, `<<~SQL`,
/// `<<"sql"`). Returns `Some("sql")` or `Some("json")` for known language tags,
/// and `None` for everything else (including backtick command heredocs).
///
/// # Edge cases handled
///
/// - `<<~SQL` — indented heredoc, strip the `~`
/// - `<<'SQL'`, `<<"SQL"` — quoted delimiters, strip quotes
/// - Case-insensitive: `<<sql`, `<<Sql`, `<<SQL` all map to `"sql"`
/// - Backtick heredoc `<<\`SQL\`` — returns `None` (command exec, not injection)
fn heredoc_injection_language(text: &str) -> Option<&'static str> {
    let rest = text.strip_prefix("<<")?;
    // Strip optional indented-heredoc `~`
    let rest = rest.strip_prefix('~').unwrap_or(rest);
    // Backtick delimiter → command heredoc, never inject
    if rest.starts_with('`') {
        return None;
    }
    // Strip matching quote characters (single or double)
    let label = rest.trim_start_matches(['\'', '"']).trim_end_matches(['\'', '"']);
    match label.to_ascii_lowercase().as_str() {
        "sql" | "mysql" | "postgres" | "postgresql" | "sqlite" => Some("sql"),
        "json" => Some("json"),
        _ => None,
    }
}

/// Emit semantic tokens for SQL keyword matches inside a heredoc body.
fn tokenize_sql_body(
    body: &str,
    body_start: usize,
    to_pos16: &impl Fn(usize) -> (u32, u32),
    leg: &TokensLegend,
    out: &mut Vec<(u32, u32, u32, u32, u32)>,
) {
    let re = match SQL_KW_RE.as_ref() {
        Some(r) => r,
        None => return,
    };
    let kind = kind_idx(leg, "sql_heredoc_keyword");
    for mat in re.find_iter(body) {
        let offset = body_start + mat.start();
        let (sl, sc) = to_pos16(offset);
        let (el, ec) = to_pos16(body_start + mat.end());
        let len = if sl == el { ec.saturating_sub(sc) } else { 0 };
        if len > 0 {
            out.push((sl, sc, len, kind, 0));
        }
    }
}

/// Emit semantic tokens for JSON key matches inside a heredoc body.
fn tokenize_json_body(
    body: &str,
    body_start: usize,
    to_pos16: &impl Fn(usize) -> (u32, u32),
    leg: &TokensLegend,
    out: &mut Vec<(u32, u32, u32, u32, u32)>,
) {
    let re = match JSON_KEY_RE.as_ref() {
        Some(r) => r,
        None => return,
    };
    let kind = kind_idx(leg, "json_heredoc_key");
    for mat in re.find_iter(body) {
        // Highlight only the key string (before the colon), not the colon itself.
        // The regex matches `"key":` — trim the trailing colon off the token length.
        let match_text = mat.as_str();
        let colon_offset = match_text.rfind(':').unwrap_or(match_text.len());
        let key_start = body_start + mat.start();
        let key_end = key_start + colon_offset;
        let (sl, sc) = to_pos16(key_start);
        let (el, ec) = to_pos16(key_end);
        let len = if sl == el { ec.saturating_sub(sc) } else { 0 };
        if len > 0 {
            out.push((sl, sc, len, kind, 0));
        }
    }
}

/// Dispatch to the appropriate body tokenizer based on the injection language.
fn tokenize_heredoc_body(
    body: &str,
    body_start: usize,
    lang: &str,
    to_pos16: &impl Fn(usize) -> (u32, u32),
    leg: &TokensLegend,
    out: &mut Vec<(u32, u32, u32, u32, u32)>,
) {
    match lang {
        "sql" => tokenize_sql_body(body, body_start, to_pos16, leg, out),
        "json" => tokenize_json_body(body, body_start, to_pos16, leg, out),
        _ => {}
    }
}

/// Returns `true` when `full_name` is a well-known Perl built-in special variable.
///
/// `full_name` is the sigil concatenated with the bare name, e.g. `"$_"`, `"@_"`,
/// `"%ENV"`.  These variables exist in every Perl program and are never declared
/// with `my`/`our`, so they deserve the `defaultLibrary` semantic-token modifier
/// to let editors colour them distinctly from user-defined variables.
///
/// Note: hash elements accessed as `$ENV{KEY}` appear in the AST as
/// `Variable { sigil: "$", name: "ENV" }`.  We include both `$ENV` and `%ENV`
/// so that all access forms receive the modifier.
fn is_special_variable(full_name: &str) -> bool {
    matches!(
        full_name,
        "$_" | "@_"
            | "$!"
            | "$@"
            | "$?"
            | "$/"
            | "$\\"
            | "$$"
            | "$0"
            | "$;"
            | "$,"
            | "$."
            | "$&"
            | "$'"
            | "$`"
            | "$+"
            | "$^W"
            | "$^O"
            | "$^V"
            | "$^T"
            | "$^A"
            | "@ISA"
            | "@INC"
            | "@ARGV"
            | "%ENV"
            | "$ENV"  // hash element access: $ENV{KEY}
            | "%INC"
            | "$INC"  // hash element access: $INC{'Foo.pm'}
            | "%SIG"
            | "$SIG" // hash element access: $SIG{INT}
            | "$PL_sv_yes"
            | "$PL_sv_no"
            | "$PL_sv_undef"
    )
}

/// Find the first occurrence of `needle` in `haystack` starting at byte offset `from`.
///
/// Returns the absolute offset within `haystack` where `needle` begins,
/// or `None` if not found. Used to locate interpolated-string parts within
/// the full token text so we can derive their absolute source positions.
fn find_bytes_at(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(from);
    }
    let end = haystack.len().saturating_sub(needle.len());
    (from..=end).find(|&i| haystack[i..i + needle.len()] == *needle)
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
/// ```rust,ignore
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
    // AST tokens are collected first so that when lexer tokens occupy the same
    // span (e.g. "method" keyword vs method-name token), the AST token wins
    // the stable-sort tie-break in remove_overlapping_tokens.
    let mut ast_tokens: Vec<(u32, u32, u32, u32, u32)> = Vec::new();
    let mut lexer_tokens: Vec<(u32, u32, u32, u32, u32)> = Vec::new();

    // 1) Fast path from lexer categories: conservative single-line emission
    // FIFO queue of pending heredoc injection languages, one entry per heredoc start token
    // encountered in source order. Multiple heredocs on the same line (`<<SQL, <<JSON`)
    // are handled correctly: we push on HeredocStart and pop on HeredocBody.
    let mut pending_heredoc_langs: VecDeque<Option<&'static str>> = VecDeque::new();
    // Use with_body_tokens so the lexer emits HeredocBody tokens (needed for injection).
    let mut lexer = PerlLexer::with_body_tokens(text);
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
                    | "default" | "break" | "continue" | "unless" | "no" | "BEGIN" | "END"
                    | "CHECK" | "INIT" | "UNITCHECK" | "class" | "method" | "try" | "catch"
                    | "finally" => "keyword",
                    _ => continue,
                }
            }

            TokenType::InterpolatedString(parts) => {
                // Split the string into literal fragments (string) and variable
                // interpolations (variable), pushing each as its own token.
                // This avoids the "longer wins" overlap-removal rule that would
                // silently discard variable sub-tokens if we also pushed a
                // whole-string token spanning the entire interpolated string.
                if len > 0 {
                    let text_bytes = tok.text.as_bytes();
                    let mut cursor: usize = 1; // skip opening quote char
                    for part in parts {
                        match part {
                            StringPart::Literal(lit) => {
                                if let Some(rel) = find_bytes_at(text_bytes, cursor, lit.as_bytes())
                                {
                                    let part_start = tok.start + rel;
                                    let part_end = part_start + lit.len();
                                    let (psl, psc) = to_pos16(part_start);
                                    let (pel, pec) = to_pos16(part_end);
                                    let plen = if psl == pel { pec.saturating_sub(psc) } else { 0 };
                                    if plen > 0 {
                                        lexer_tokens.push((
                                            psl,
                                            psc,
                                            plen,
                                            kind_idx(&leg, "string"),
                                            0,
                                        ));
                                    }
                                    cursor = rel + lit.len();
                                }
                            }
                            StringPart::Variable(var) => {
                                if let Some(rel) = find_bytes_at(text_bytes, cursor, var.as_bytes())
                                {
                                    let part_start = tok.start + rel;
                                    let part_end = part_start + var.len();
                                    let (psl, psc) = to_pos16(part_start);
                                    let (pel, pec) = to_pos16(part_end);
                                    let plen = if psl == pel { pec.saturating_sub(psc) } else { 0 };
                                    if plen > 0 {
                                        lexer_tokens.push((
                                            psl,
                                            psc,
                                            plen,
                                            kind_idx(&leg, "variable"),
                                            0,
                                        ));
                                    }
                                    cursor = rel + var.len();
                                }
                            }
                            // Expression, ArraySlice, MethodCall: defined in StringPart but
                            // the current lexer never emits them — only Literal and Variable
                            // are populated. Skip silently.
                            _ => {}
                        }
                    }
                }
                continue;
            }

            TokenType::StringLiteral
            | TokenType::QuoteSingle
            | TokenType::QuoteDouble
            | TokenType::QuoteWords
            | TokenType::QuoteCommand => "string",

            TokenType::HeredocStart => {
                // Record the injection language for this heredoc's upcoming body token.
                pending_heredoc_langs.push_back(heredoc_injection_language(&tok.text));
                "string"
            }

            TokenType::HeredocBody(_) => {
                // Pop the queued injection language for the corresponding heredoc start.
                // NOTE: The Arc<str> inside HeredocBody is always empty_arc(); the actual
                // body text must be sliced from the source using tok.start..tok.end.
                if let Some(maybe_lang) = pending_heredoc_langs.pop_front()
                    && let Some(lang) = maybe_lang
                {
                    let body_end = tok.end.min(text.len());
                    let body = &text[tok.start.min(body_end)..body_end];
                    tokenize_heredoc_body(body, tok.start, lang, to_pos16, &leg, &mut lexer_tokens);
                }
                "string"
            }

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

            // POD documentation blocks
            TokenType::Pod => "comment",
            _ => continue,
        };

        if len > 0 {
            lexer_tokens.push((sl, sc, len, kind_idx(&leg, kind), 0));
        }
    }

    let const_fast_enabled = ast_uses_const_fast(ast);

    // 2a) Collect variable declaration spans for modifier tagging
    let decl_spans = declaration_readonly_flags(ast)
        .into_iter()
        .map(|((start, end), is_readonly)| (start, end, is_readonly))
        .collect::<Vec<_>>();

    // 2b) AST overlays: package/sub/variable with precise spans where available
    walk_ast_full(ast, &mut |node| {
        // For nodes with name_span, use the precise span for better highlighting
        match &node.kind {
            NodeKind::Package { name_span, .. } => {
                let (sl, sc) = to_pos16(name_span.start);
                let (el, ec) = to_pos16(name_span.end);
                let len = if sl == el { ec.saturating_sub(sc) } else { 0 };
                if len > 0 {
                    ast_tokens.push((
                        sl,
                        sc,
                        len,
                        kind_idx(&leg, "namespace"),
                        1, /*declaration*/
                    ));
                }
                return true;
            }
            NodeKind::Subroutine { name: Some(_), name_span: Some(span), .. } => {
                let (sl, sc) = to_pos16(span.start);
                let (el, ec) = to_pos16(span.end);
                let len = if sl == el { ec.saturating_sub(sc) } else { 0 };
                if len > 0 {
                    ast_tokens.push((
                        sl,
                        sc,
                        len,
                        kind_idx(&leg, "function"),
                        1 | 2, /*declaration|definition*/
                    ));
                }
                return true;
            }
            NodeKind::Subroutine { name: Some(_), .. } => {
                let (sl, sc) = to_pos16(node.location.start);
                let (el, ec) = to_pos16(node.location.end);
                let len = if sl == el { ec.saturating_sub(sc) } else { 0 };
                if len > 0 {
                    ast_tokens.push((
                        sl,
                        sc,
                        len,
                        kind_idx(&leg, "function"),
                        1, /*declaration*/
                    ));
                }
                return true;
            }
            NodeKind::Method { .. } => {
                let (sl, sc) = to_pos16(node.location.start);
                let (el, ec) = to_pos16(node.location.end);
                let len = if sl == el { ec.saturating_sub(sc) } else { 0 };
                if len > 0 {
                    ast_tokens.push((
                        sl,
                        sc,
                        len,
                        kind_idx(&leg, "method"),
                        1 | 2, /*declaration|definition*/
                    ));
                }
                return true;
            }
            NodeKind::Class { .. } => {
                let (sl, sc) = to_pos16(node.location.start);
                let (el, ec) = to_pos16(node.location.end);
                let len = if sl == el { ec.saturating_sub(sc) } else { 0 };
                if len > 0 {
                    ast_tokens.push((sl, sc, len, kind_idx(&leg, "class"), 1 /*declaration*/));
                }
                return true;
            }
            NodeKind::PhaseBlock { phase_span: Some(span), .. } => {
                let (sl, sc) = to_pos16(span.start);
                let (el, ec) = to_pos16(span.end);
                let len = if sl == el { ec.saturating_sub(sc) } else { 0 };
                if len > 0 {
                    ast_tokens.push((sl, sc, len, kind_idx(&leg, "macro"), 0));
                }
                return true;
            }
            NodeKind::MethodCall { object, method, args } => {
                // Emit a narrow token for just the method name, not the entire expression.
                // object.location.end is the byte offset after the receiver; +2 skips "->".
                let method_name_start = object.location.end + 2;
                let method_name_end = method_name_start + method.len();
                let (sl, sc) = to_pos16(method_name_start);
                let (el, ec) = to_pos16(method_name_end);
                let len = if sl == el { ec.saturating_sub(sc) } else { 0 };
                if len > 0 {
                    ast_tokens.push((sl, sc, len, kind_idx(&leg, "method"), 0));
                }
                // If this is a SQL-bearing DBI method, classify the first string arg as sql_string.
                let is_sql_method = matches!(
                    method.as_str(),
                    "prepare"
                        | "do"
                        | "query"
                        | "selectrow_array"
                        | "selectrow_arrayref"
                        | "selectrow_hashref"
                        | "selectall_arrayref"
                        | "selectall_hashref"
                        | "fetchall_arrayref"
                        | "fetchall_hashref"
                        | "fetchrow_arrayref"
                        | "fetchrow_hashref"
                        | "execute"
                );
                if is_sql_method
                    && let Some(first_arg) = args.first()
                    && matches!(first_arg.kind, NodeKind::String { .. })
                {
                    let (asl, asc) = to_pos16(first_arg.location.start);
                    let (ael, aec) = to_pos16(first_arg.location.end);
                    let alen = if asl == ael { aec.saturating_sub(asc) } else { 0 };
                    if alen > 0 {
                        ast_tokens.push((asl, asc, alen, kind_idx(&leg, "sql_string"), 0));
                    }
                }
                return true;
            }
            _ => {}
        }

        let (s, e) = (node.location.start, node.location.end);
        let (sl, sc) = to_pos16(s);
        let (el, ec) = to_pos16(e);
        let len = if sl == el { ec.saturating_sub(sc) } else { 0 };

        let (kind, mods): (&str, u32) = match &node.kind {
            NodeKind::FunctionCall { name, .. } => {
                if const_fast_enabled && name == "const" {
                    return true;
                }
                // Skip builtins that should remain as keywords from the lexer pass
                match name.as_str() {
                    "eval" | "do" | "use" | "no" | "return" | "my" | "our" | "local" | "state"
                    | "next" | "last" | "redo" | "goto" => return true,
                    _ => ("function", 0),
                }
            }
            NodeKind::Variable { sigil, name } => {
                let (vs, ve) = (node.location.start, node.location.end);
                let decl_info = decl_spans.iter().find(|(ds, de, _)| *ds <= vs && ve <= *de);
                let full_name = format!("{sigil}{name}");
                let special_mod = if is_special_variable(&full_name) { 512 } else { 0 }; // defaultLibrary bit 9
                let sigil_mod: u32 = match sigil.as_str() {
                    "$" => 1024, // scalarVariable bit 10
                    "@" => 2048, // arrayVariable  bit 11
                    "%" => 4096, // hashVariable   bit 12
                    _ => 0,      // "&" (code ref), "*" (glob), others
                };
                let mods = match decl_info {
                    Some((_, _, true)) => 1 | 4 | special_mod | sigil_mod, // declaration | readonly (our)
                    Some((_, _, false)) => 1 | special_mod | sigil_mod, // declaration (my/local/state)
                    None => special_mod | sigil_mod,
                };
                ("variable", mods)
            }
            _ => return true,
        };

        if len > 0 {
            ast_tokens.push((sl, sc, len, kind_idx(&leg, kind), mods));
        }
        true
    });

    // 3) Merge: AST tokens first so they win stable-sort ties over same-span lexer tokens
    let mut raw_tokens = ast_tokens;
    raw_tokens.extend(lexer_tokens);

    // 4) Remove overlapping tokens (LSP specification compliance)
    let dedup_tokens = remove_overlapping_tokens(raw_tokens);

    // 5) Sort by position and encode with deltas (thread-safe)
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

/// Comprehensive AST walker for semantic token extraction.
fn walk_ast_full<F>(node: &Node, visitor: &mut F) -> bool
where
    F: FnMut(&Node) -> bool,
{
    if !visitor(node) {
        return false;
    }

    let children: Vec<&Node> = match &node.kind {
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            statements.iter().collect()
        }
        NodeKind::ExpressionStatement { expression } => vec![expression.as_ref()],
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            let mut c = vec![variable.as_ref()];
            if let Some(init) = initializer {
                c.push(init.as_ref());
            }
            c
        }
        NodeKind::VariableListDeclaration { variables, initializer, .. } => {
            let mut c: Vec<&Node> = variables.iter().collect();
            if let Some(init) = initializer {
                c.push(init.as_ref());
            }
            c
        }
        NodeKind::Assignment { lhs, rhs, .. } => vec![lhs.as_ref(), rhs.as_ref()],
        NodeKind::Binary { left, right, .. } => vec![left.as_ref(), right.as_ref()],
        NodeKind::Ternary { condition, then_expr, else_expr } => {
            vec![condition.as_ref(), then_expr.as_ref(), else_expr.as_ref()]
        }
        NodeKind::Unary { operand, .. } => vec![operand.as_ref()],
        NodeKind::FunctionCall { args, .. } => args.iter().collect(),
        NodeKind::MethodCall { object, args, .. } => {
            let mut c = vec![object.as_ref()];
            c.extend(args.iter());
            c
        }
        NodeKind::IndirectCall { object, args, .. } => {
            let mut c = vec![object.as_ref()];
            c.extend(args.iter());
            c
        }
        NodeKind::Subroutine { prototype, signature, body, .. } => {
            let mut c = Vec::new();
            if let Some(proto) = prototype {
                c.push(proto.as_ref());
            }
            if let Some(sig) = signature {
                c.push(sig.as_ref());
            }
            c.push(body.as_ref());
            c
        }
        NodeKind::Method { signature, body, .. } => {
            let mut c = Vec::new();
            if let Some(sig) = signature {
                c.push(sig.as_ref());
            }
            c.push(body.as_ref());
            c
        }
        NodeKind::Signature { parameters } => parameters.iter().collect(),
        NodeKind::MandatoryParameter { variable }
        | NodeKind::SlurpyParameter { variable }
        | NodeKind::NamedParameter { variable } => {
            vec![variable.as_ref()]
        }
        NodeKind::OptionalParameter { variable, default_value } => {
            vec![variable.as_ref(), default_value.as_ref()]
        }
        NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
            let mut c = vec![condition.as_ref(), then_branch.as_ref()];
            for (cond, body) in elsif_branches {
                c.push(cond.as_ref());
                c.push(body.as_ref());
            }
            if let Some(eb) = else_branch {
                c.push(eb.as_ref());
            }
            c
        }
        NodeKind::While { condition, body, continue_block } => {
            let mut c = vec![condition.as_ref(), body.as_ref()];
            if let Some(cb) = continue_block {
                c.push(cb.as_ref());
            }
            c
        }
        NodeKind::For { init, condition, update, body, continue_block } => {
            let mut c = Vec::new();
            if let Some(i) = init {
                c.push(i.as_ref());
            }
            if let Some(cond) = condition {
                c.push(cond.as_ref());
            }
            if let Some(upd) = update {
                c.push(upd.as_ref());
            }
            c.push(body.as_ref());
            if let Some(cb) = continue_block {
                c.push(cb.as_ref());
            }
            c
        }
        NodeKind::Foreach { variable, list, body, continue_block } => {
            let mut c = vec![variable.as_ref(), list.as_ref(), body.as_ref()];
            if let Some(cb) = continue_block {
                c.push(cb.as_ref());
            }
            c
        }
        NodeKind::Package { block, .. } => {
            let mut c = Vec::new();
            if let Some(b) = block {
                c.push(b.as_ref());
            }
            c
        }
        NodeKind::Class { body, .. } => vec![body.as_ref()],
        NodeKind::Eval { block } | NodeKind::Do { block } | NodeKind::Defer { block } => {
            vec![block.as_ref()]
        }
        NodeKind::Try { body, catch_blocks, finally_block } => {
            let mut c = vec![body.as_ref()];
            for (_var, handler) in catch_blocks {
                c.push(handler.as_ref());
            }
            if let Some(fb) = finally_block {
                c.push(fb.as_ref());
            }
            c
        }
        NodeKind::StatementModifier { statement, condition, .. } => {
            vec![statement.as_ref(), condition.as_ref()]
        }
        NodeKind::Return { value } => {
            let mut c = Vec::new();
            if let Some(v) = value {
                c.push(v.as_ref());
            }
            c
        }
        NodeKind::ArrayLiteral { elements } => elements.iter().collect(),
        NodeKind::HashLiteral { pairs } => {
            let mut c = Vec::new();
            for (k, v) in pairs {
                c.push(k);
                c.push(v);
            }
            c
        }
        NodeKind::LabeledStatement { statement, .. } => vec![statement.as_ref()],
        NodeKind::Given { expr, body } | NodeKind::When { condition: expr, body } => {
            vec![expr.as_ref(), body.as_ref()]
        }
        NodeKind::Default { body } => vec![body.as_ref()],
        NodeKind::PhaseBlock { block, .. } => vec![block.as_ref()],
        NodeKind::VariableWithAttributes { variable, .. } => vec![variable.as_ref()],
        NodeKind::Match { expr, .. }
        | NodeKind::Substitution { expr, .. }
        | NodeKind::Transliteration { expr, .. } => {
            vec![expr.as_ref()]
        }
        NodeKind::Tie { variable, package, args } => {
            let mut c = vec![variable.as_ref(), package.as_ref()];
            c.extend(args.iter());
            c
        }
        NodeKind::Untie { variable } => vec![variable.as_ref()],
        NodeKind::Error { partial, .. } => {
            let mut c = Vec::new();
            if let Some(p) = partial {
                c.push(p.as_ref());
            }
            c
        }
        // Leaf nodes with no children
        _ => vec![],
    };

    for child in children {
        if !walk_ast_full(child, visitor) {
            return false;
        }
    }

    true
}

fn declaration_readonly_flags(ast: &Node) -> FxHashMap<(usize, usize), bool> {
    let mut flags = FxHashMap::default();
    let const_fast_enabled = ast_uses_const_fast(ast);

    walk_ast_full(ast, &mut |node| {
        match &node.kind {
            NodeKind::VariableDeclaration { declarator, variable, .. } => {
                let is_readonly = declarator == "our";
                flags
                    .entry((variable.location.start, variable.location.end))
                    .and_modify(|flag| *flag |= is_readonly)
                    .or_insert(is_readonly);
            }
            NodeKind::VariableListDeclaration { declarator, variables, .. } => {
                let is_readonly = declarator == "our";
                for variable in variables {
                    flags
                        .entry((variable.location.start, variable.location.end))
                        .and_modify(|flag| *flag |= is_readonly)
                        .or_insert(is_readonly);
                }
            }
            NodeKind::FunctionCall { name, args } if const_fast_enabled && name == "const" => {
                mark_const_fast_decl_flags(args, &mut flags);
            }
            _ => {}
        }
        true
    });

    flags
}

fn ast_uses_const_fast(ast: &Node) -> bool {
    let mut enabled = false;
    walk_ast_full(ast, &mut |node| {
        if matches!(&node.kind, NodeKind::Use { module, .. } if module == "Const::Fast") {
            enabled = true;
            return false;
        }
        true
    });
    enabled
}

fn mark_const_fast_decl_flags(args: &[Node], flags: &mut FxHashMap<(usize, usize), bool>) {
    for arg in args {
        match &arg.kind {
            NodeKind::VariableDeclaration { variable, .. } => {
                flags.insert((variable.location.start, variable.location.end), true);
            }
            NodeKind::VariableListDeclaration { variables, .. } => {
                for variable in variables {
                    flags.insert((variable.location.start, variable.location.end), true);
                }
            }
            _ => {}
        }
    }
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
