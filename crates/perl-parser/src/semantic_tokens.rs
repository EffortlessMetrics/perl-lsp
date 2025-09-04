// crates/perl-parser/src/semantic_tokens.rs
use crate::ast::{Node, NodeKind};
use perl_lexer::{PerlLexer, TokenType};
use rustc_hash::FxHashMap;

/// LSP wants [deltaLine, deltaStartChar, length, tokenTypeIndex, tokenModBits]
pub type EncodedToken = [u32; 5];

pub struct TokensLegend {
    pub token_types: Vec<String>,
    pub modifiers: Vec<String>,
    pub map: FxHashMap<String, u32>,
}

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

    // 3) Sort by position and encode with deltas (thread-safe)
    encode_raw_tokens_to_deltas(raw_tokens)
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

    for child in crate::declaration::get_node_children(node) {
        if !walk_ast(child, visitor) {
            return false;
        }
    }

    true
}
