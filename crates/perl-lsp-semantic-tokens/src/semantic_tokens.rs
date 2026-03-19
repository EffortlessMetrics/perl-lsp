//! Semantic token analysis for LSP syntax highlighting in Perl script processing
//!
//! This module provides semantic token extraction and classification for Perl scripts
//! within the LSP workflow.

use perl_builtins_phf::is_builtin;
use perl_lexer::{PerlLexer, TokenType};
use perl_parser_core::ast::{Node, NodeKind};
use rustc_hash::FxHashMap;

/// declaration modifier (bit 0)
pub const MOD_DECLARATION: u32 = 1 << 0;
/// definition modifier (bit 1)
pub const MOD_DEFINITION: u32 = 1 << 1;
/// readonly modifier (bit 2)
pub const MOD_READONLY: u32 = 1 << 2;
/// defaultLibrary modifier (bit 3)
pub const MOD_DEFAULT_LIBRARY: u32 = 1 << 3;
/// static modifier (bit 5)
pub const MOD_STATIC: u32 = 1 << 5;
/// modification modifier (bit 7)
pub const MOD_MODIFICATION: u32 = 1 << 7;

/// Returns true if the variable (sigil + name) is a Perl built-in variable.
fn is_builtin_variable(sigil: &str, name: &str) -> bool {
    matches!(
        (sigil, name),
        ("$", "_" | "!" | "@" | "/" | "\\" | ";" | "," | "." | "+" | "&" | "`" | "'" | "0")
            | ("@", "_" | "ARGV" | "INC" | "ISA")
            | ("%", "ENV" | "INC" | "SIG")
    )
}

/// LSP semantic token encoding format for client transmission
pub type EncodedToken = [u32; 5];

/// Semantic token legend mapping token types and modifiers to indices
pub struct TokensLegend {
    /// List of token type names in index order
    pub token_types: Vec<String>,
    /// List of modifier names in index order
    pub modifiers: Vec<String>,
    /// Fast lookup map from token type names to indices
    pub map: FxHashMap<String, u32>,
}

/// Create the standard semantic token legend for Perl script highlighting
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
        "modification",
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
pub fn collect_semantic_tokens(
    ast: &Node,
    text: &str,
    to_pos16: &impl Fn(usize) -> (u32, u32),
) -> Vec<EncodedToken> {
    let leg = legend();
    let mut raw_tokens: Vec<(u32, u32, u32, u32, u32)> = Vec::new();

    // 1) Fast path from lexer categories
    let mut lexer = PerlLexer::new(text);
    while let Some(tok) = lexer.next_token() {
        let (sl, sc) = to_pos16(tok.start);
        let (el, ec) = to_pos16(tok.end);
        let len = if sl == el { ec.saturating_sub(sc) } else { 0 };

        let kind = match &tok.token_type {
            TokenType::Keyword(kw) => match kw.as_ref() {
                "my" | "our" | "local" | "state" | "sub" | "package" | "use" | "require" | "if"
                | "else" | "elsif" | "for" | "foreach" | "while" | "until" | "do" | "return"
                | "next" | "last" | "redo" | "goto" | "eval" | "given" | "when" | "default"
                | "break" | "continue" | "unless" | "no" | "BEGIN" | "END" | "CHECK" | "INIT"
                | "UNITCHECK" | "class" | "method" | "try" | "catch" | "finally" => "keyword",
                _ => continue,
            },
            TokenType::StringLiteral
            | TokenType::QuoteSingle
            | TokenType::QuoteDouble
            | TokenType::QuoteWords
            | TokenType::QuoteCommand
            | TokenType::HeredocStart
            | TokenType::HeredocBody(_)
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
            TokenType::Comment(_) | TokenType::Pod => "comment",
            _ => continue,
        };

        if len > 0 {
            raw_tokens.push((sl, sc, len, kind_idx(&leg, kind), 0));
        }
    }

    // 2a) Collect variable declaration spans and assignment LHS spans
    let mut decl_spans: Vec<(usize, usize, bool)> = Vec::new();
    let mut assign_lhs_spans: Vec<(usize, usize)> = Vec::new();
    walk_ast_full(ast, &mut |node| {
        if let NodeKind::VariableDeclaration { declarator, variable, .. } = &node.kind {
            let is_our = declarator == "our";
            decl_spans.push((variable.location.start, variable.location.end, is_our));
        }
        if let NodeKind::VariableListDeclaration { declarator, variables, .. } = &node.kind {
            let is_our = declarator == "our";
            for v in variables {
                decl_spans.push((v.location.start, v.location.end, is_our));
            }
        }
        if let NodeKind::Assignment { lhs, .. } = &node.kind {
            assign_lhs_spans.push((lhs.location.start, lhs.location.end));
        }
        true
    });

    // 2b) AST overlays
    walk_ast_full(ast, &mut |node| {
        match &node.kind {
            NodeKind::Package { name_span, .. } => {
                let (sl, sc) = to_pos16(name_span.start);
                let (el, ec) = to_pos16(name_span.end);
                let len = if sl == el { ec.saturating_sub(sc) } else { 0 };
                if len > 0 {
                    raw_tokens.push((sl, sc, len, kind_idx(&leg, "namespace"), MOD_DECLARATION));
                }
                return true;
            }
            NodeKind::Subroutine { name: Some(_), name_span: Some(span), .. } => {
                let (sl, sc) = to_pos16(span.start);
                let (el, ec) = to_pos16(span.end);
                let len = if sl == el { ec.saturating_sub(sc) } else { 0 };
                if len > 0 {
                    raw_tokens.push((
                        sl,
                        sc,
                        len,
                        kind_idx(&leg, "function"),
                        MOD_DECLARATION | MOD_DEFINITION,
                    ));
                }
                return true;
            }
            NodeKind::Subroutine { name: Some(_), .. } => {
                let (sl, sc) = to_pos16(node.location.start);
                let (el, ec) = to_pos16(node.location.end);
                let len = if sl == el { ec.saturating_sub(sc) } else { 0 };
                if len > 0 {
                    raw_tokens.push((sl, sc, len, kind_idx(&leg, "function"), MOD_DECLARATION));
                }
                return true;
            }
            NodeKind::Method { .. } => {
                let (sl, sc) = to_pos16(node.location.start);
                let (el, ec) = to_pos16(node.location.end);
                let len = if sl == el { ec.saturating_sub(sc) } else { 0 };
                if len > 0 {
                    raw_tokens.push((
                        sl,
                        sc,
                        len,
                        kind_idx(&leg, "method"),
                        MOD_DECLARATION | MOD_DEFINITION,
                    ));
                }
                return true;
            }
            NodeKind::Class { .. } => {
                let (sl, sc) = to_pos16(node.location.start);
                let (el, ec) = to_pos16(node.location.end);
                let len = if sl == el { ec.saturating_sub(sc) } else { 0 };
                if len > 0 {
                    raw_tokens.push((sl, sc, len, kind_idx(&leg, "class"), MOD_DECLARATION));
                }
                return true;
            }
            NodeKind::PhaseBlock { phase_span: Some(span), .. } => {
                let (sl, sc) = to_pos16(span.start);
                let (el, ec) = to_pos16(span.end);
                let len = if sl == el { ec.saturating_sub(sc) } else { 0 };
                if len > 0 {
                    raw_tokens.push((sl, sc, len, kind_idx(&leg, "macro"), 0));
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
            NodeKind::FunctionCall { name, .. } => match name.as_str() {
                "eval" | "do" | "use" | "no" | "return" | "my" | "our" | "local" | "state"
                | "next" | "last" | "redo" | "goto" => return true,
                _ => {
                    let mods = if is_builtin(name) { MOD_DEFAULT_LIBRARY } else { 0 };
                    ("function", mods)
                }
            },
            NodeKind::MethodCall { .. } => ("method", 0),
            NodeKind::Variable { sigil, name } => {
                let (vs, ve) = (node.location.start, node.location.end);
                let decl_info = decl_spans.iter().find(|(ds, de, _)| *ds <= vs && ve <= *de);
                let is_assigned = assign_lhs_spans.iter().any(|(as_, ae)| *as_ <= vs && ve <= *ae);

                // $self -> parameter type
                if sigil == "$" && name == "self" {
                    let mut mods = 0u32;
                    if let Some((_, _, is_our)) = decl_info {
                        mods |= MOD_DECLARATION;
                        if *is_our {
                            mods |= MOD_READONLY | MOD_STATIC;
                        }
                    }
                    if is_assigned {
                        mods |= MOD_MODIFICATION;
                    }
                    if len > 0 {
                        raw_tokens.push((sl, sc, len, kind_idx(&leg, "parameter"), mods));
                    }
                    return true;
                }

                let is_builtin_var = is_builtin_variable(sigil, name);
                let mut mods = match decl_info {
                    Some((_, _, true)) => MOD_DECLARATION | MOD_READONLY | MOD_STATIC,
                    Some((_, _, false)) => MOD_DECLARATION,
                    None => 0,
                };
                if is_builtin_var {
                    mods |= MOD_DEFAULT_LIBRARY;
                }
                if is_assigned {
                    mods |= MOD_MODIFICATION;
                }

                ("variable", mods)
            }
            _ => return true,
        };

        if len > 0 {
            raw_tokens.push((sl, sc, len, kind_idx(&leg, kind), mods));
        }
        true
    });

    let dedup_tokens = remove_overlapping_tokens(raw_tokens);
    encode_raw_tokens_to_deltas(dedup_tokens)
}

fn remove_overlapping_tokens(
    raw_tokens: Vec<(u32, u32, u32, u32, u32)>,
) -> Vec<(u32, u32, u32, u32, u32)> {
    let mut sorted_tokens = raw_tokens;
    sorted_tokens
        .sort_by_key(|&(line, start_char, _length, _token_type, _modifier)| (line, start_char));

    let mut result = Vec::new();
    for token in sorted_tokens {
        let (line, start_char, length, _token_type, _modifier) = token;
        if let Some(&(last_line, last_start, last_length, _last_type, _last_modifier)) =
            result.last()
        {
            if line == last_line && start_char < last_start + last_length {
                if length > last_length {
                    result.pop();
                    result.push(token);
                }
            } else {
                result.push(token);
            }
        } else {
            result.push(token);
        }
    }
    result
}

fn encode_raw_tokens_to_deltas(
    mut raw_tokens: Vec<(u32, u32, u32, u32, u32)>,
) -> Vec<EncodedToken> {
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
        | NodeKind::NamedParameter { variable } => vec![variable.as_ref()],
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
        NodeKind::Eval { block } | NodeKind::Do { block } => vec![block.as_ref()],
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
        | NodeKind::Transliteration { expr, .. } => vec![expr.as_ref()],
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
        _ => vec![],
    };

    for child in children {
        if !walk_ast_full(child, visitor) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(line: u32, start: u32, len: u32, kind: u32, mods: u32) -> (u32, u32, u32, u32, u32) {
        (line, start, len, kind, mods)
    }

    #[test]
    fn test_remove_overlapping_tokens_basic() {
        let input = vec![tok(0, 0, 5, 0, 0), tok(0, 6, 5, 0, 0)];
        assert_eq!(remove_overlapping_tokens(input.clone()), input);
    }

    #[test]
    fn test_remove_overlapping_tokens_touching() {
        let input = vec![tok(0, 0, 5, 0, 0), tok(0, 5, 5, 0, 0)];
        assert_eq!(remove_overlapping_tokens(input.clone()), input);
    }

    #[test]
    fn test_remove_overlapping_tokens_nested_keep_outer() {
        let r = remove_overlapping_tokens(vec![tok(0, 0, 10, 0, 0), tok(0, 2, 3, 1, 0)]);
        assert_eq!(r, vec![tok(0, 0, 10, 0, 0)]);
    }

    #[test]
    fn test_remove_overlapping_tokens_nested_keep_longer_inner_replacement() {
        let r = remove_overlapping_tokens(vec![tok(0, 0, 5, 0, 0), tok(0, 0, 10, 1, 0)]);
        assert_eq!(r, vec![tok(0, 0, 10, 1, 0)]);
    }

    #[test]
    fn test_remove_overlapping_tokens_overlap_tail_keep_longer() {
        let r = remove_overlapping_tokens(vec![tok(0, 0, 5, 0, 0), tok(0, 4, 6, 1, 0)]);
        assert_eq!(r, vec![tok(0, 4, 6, 1, 0)]);
    }

    #[test]
    fn test_remove_overlapping_tokens_overlap_tail_keep_earlier_if_longer() {
        let r = remove_overlapping_tokens(vec![tok(0, 0, 10, 0, 0), tok(0, 8, 7, 1, 0)]);
        assert_eq!(r, vec![tok(0, 0, 10, 0, 0)]);
    }

    #[test]
    fn test_remove_overlapping_tokens_equal_length_keep_first() {
        let r = remove_overlapping_tokens(vec![tok(0, 0, 5, 1, 0), tok(0, 0, 5, 2, 0)]);
        assert_eq!(r, vec![tok(0, 0, 5, 1, 0)]);
    }

    #[test]
    fn test_remove_overlapping_tokens_different_lines() {
        let input = vec![tok(0, 0, 5, 0, 0), tok(1, 0, 5, 0, 0)];
        assert_eq!(remove_overlapping_tokens(input.clone()), input);
    }

    #[test]
    fn mutation_hardening_empty_input() {
        assert_eq!(remove_overlapping_tokens(vec![]).len(), 0);
    }

    #[test]
    fn mutation_hardening_single_token() {
        let input = vec![tok(0, 0, 5, 0, 0)];
        assert_eq!(remove_overlapping_tokens(input.clone()), input);
    }

    #[test]
    fn mutation_hardening_adjacent_non_overlapping() {
        assert_eq!(
            remove_overlapping_tokens(vec![tok(0, 0, 5, 0, 0), tok(0, 5, 5, 1, 0)]).len(),
            2
        );
    }

    #[test]
    fn mutation_hardening_exact_boundary() {
        assert_eq!(
            remove_overlapping_tokens(vec![tok(0, 10, 5, 0, 0), tok(0, 15, 5, 1, 0)]).len(),
            2
        );
    }

    #[test]
    fn mutation_hardening_single_char_overlap() {
        let r = remove_overlapping_tokens(vec![tok(0, 0, 6, 0, 0), tok(0, 5, 5, 1, 0)]);
        assert_eq!(r, vec![tok(0, 0, 6, 0, 0)]);
    }

    #[test]
    fn mutation_hardening_partial_overlap_length_determines_winner() {
        let r = remove_overlapping_tokens(vec![tok(0, 0, 5, 0, 0), tok(0, 3, 7, 1, 0)]);
        assert_eq!(r, vec![tok(0, 3, 7, 1, 0)]);
    }

    #[test]
    fn mutation_hardening_equal_length_keeps_first() {
        let r = remove_overlapping_tokens(vec![tok(0, 0, 5, 0, 0), tok(0, 2, 5, 1, 0)]);
        assert_eq!(r, vec![tok(0, 0, 5, 0, 0)]);
    }

    #[test]
    fn mutation_hardening_different_lines_no_overlap() {
        assert_eq!(
            remove_overlapping_tokens(vec![tok(0, 0, 100, 0, 0), tok(1, 0, 5, 1, 0)]).len(),
            2
        );
    }

    #[test]
    fn mutation_hardening_three_tokens_cascading() {
        let r = remove_overlapping_tokens(vec![
            tok(0, 0, 5, 0, 0),
            tok(0, 4, 5, 1, 0),
            tok(0, 8, 4, 2, 0),
        ]);
        assert_eq!(r, vec![tok(0, 0, 5, 0, 0), tok(0, 8, 4, 2, 0)]);
    }

    #[test]
    fn mutation_hardening_zero_length_token() {
        assert_eq!(
            remove_overlapping_tokens(vec![tok(0, 5, 0, 0, 0), tok(0, 5, 5, 1, 0)]).len(),
            2
        );
    }

    #[test]
    fn mutation_hardening_multiple_zero_length() {
        assert_eq!(
            remove_overlapping_tokens(vec![
                tok(0, 5, 0, 0, 0),
                tok(0, 5, 0, 1, 0),
                tok(0, 5, 0, 2, 0)
            ])
            .len(),
            3
        );
    }

    #[test]
    fn mutation_hardening_large_positions() {
        assert_eq!(
            remove_overlapping_tokens(vec![
                tok(1000, u32::MAX - 100, 50, 0, 0),
                tok(1000, u32::MAX - 40, 20, 1, 0)
            ])
            .len(),
            2
        );
    }

    #[test]
    fn mutation_hardening_sort_order() {
        let r = remove_overlapping_tokens(vec![
            tok(2, 10, 5, 0, 0),
            tok(1, 10, 5, 1, 0),
            tok(0, 10, 5, 2, 0),
        ]);
        assert_eq!(r.len(), 3);
        assert_eq!(r[0].0, 0);
        assert_eq!(r[1].0, 1);
        assert_eq!(r[2].0, 2);
    }

    #[test]
    fn mutation_hardening_sort_order_same_line() {
        let r = remove_overlapping_tokens(vec![
            tok(0, 30, 5, 0, 0),
            tok(0, 20, 5, 1, 0),
            tok(0, 10, 5, 2, 0),
        ]);
        assert_eq!(r[0].1, 10);
        assert_eq!(r[1].1, 20);
        assert_eq!(r[2].1, 30);
    }

    #[test]
    fn mutation_hardening_systematic_removal() {
        let r = remove_overlapping_tokens(vec![
            tok(0, 0, 3, 0, 0),
            tok(0, 0, 5, 1, 0),
            tok(0, 0, 7, 2, 0),
            tok(0, 0, 9, 3, 0),
        ]);
        assert_eq!(r, vec![tok(0, 0, 9, 3, 0)]);
    }

    #[test]
    fn mutation_hardening_interleaved_no_overlap() {
        let input =
            vec![tok(0, 0, 3, 0, 0), tok(0, 5, 3, 1, 0), tok(0, 10, 3, 2, 0), tok(0, 15, 3, 3, 0)];
        assert_eq!(remove_overlapping_tokens(input.clone()), input);
    }

    #[test]
    fn mutation_hardening_boundary_minus_one() {
        let r = remove_overlapping_tokens(vec![tok(0, 0, 10, 0, 0), tok(0, 9, 6, 1, 0)]);
        assert_eq!(r, vec![tok(0, 0, 10, 0, 0)]);
    }

    #[test]
    fn mutation_hardening_preserves_metadata() {
        let r = remove_overlapping_tokens(vec![tok(0, 0, 5, 42, 7)]);
        assert_eq!(r[0].3, 42);
        assert_eq!(r[0].4, 7);
    }

    #[test]
    fn mutation_hardening_mixed_line_position_sort() {
        let r = remove_overlapping_tokens(vec![
            tok(2, 5, 3, 0, 0),
            tok(0, 15, 3, 1, 0),
            tok(1, 10, 3, 2, 0),
            tok(0, 5, 3, 3, 0),
            tok(2, 0, 3, 4, 0),
        ]);
        assert_eq!(r.len(), 5);
        assert!(r[0].0 <= r[1].0);
        assert!(r[1].0 <= r[2].0);
    }

    // ==================== Modifier Enhancement Tests ====================

    #[test]
    fn test_legend_has_modification_modifier() {
        let leg = legend();
        assert!(leg.modifiers.contains(&"modification".to_string()));
        assert_eq!(leg.modifiers.len(), 8);
    }

    #[test]
    fn test_mod_constants_match_legend_positions() {
        assert_eq!(MOD_DECLARATION, 1 << 0);
        assert_eq!(MOD_DEFINITION, 1 << 1);
        assert_eq!(MOD_READONLY, 1 << 2);
        assert_eq!(MOD_DEFAULT_LIBRARY, 1 << 3);
        assert_eq!(MOD_STATIC, 1 << 5);
        assert_eq!(MOD_MODIFICATION, 1 << 7);
    }

    #[test]
    fn test_is_builtin_variable_scalars() {
        assert!(is_builtin_variable("$", "_"));
        assert!(is_builtin_variable("$", "!"));
        assert!(is_builtin_variable("$", "0"));
        assert!(!is_builtin_variable("$", "foo"));
    }

    #[test]
    fn test_is_builtin_variable_arrays() {
        assert!(is_builtin_variable("@", "_"));
        assert!(is_builtin_variable("@", "ARGV"));
        assert!(!is_builtin_variable("@", "data"));
    }

    #[test]
    fn test_is_builtin_variable_hashes() {
        assert!(is_builtin_variable("%", "ENV"));
        assert!(is_builtin_variable("%", "SIG"));
        assert!(!is_builtin_variable("%", "config"));
    }
}
