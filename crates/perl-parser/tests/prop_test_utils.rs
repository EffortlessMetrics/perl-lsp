use proptest::prelude::*;
use std::collections::HashSet;

/// All delimiter pairs we'll consider for q/qq/qr/m/s/tr/y.
pub const DELIMS: &[(char, char)] = &[
    // Paired
    ('(', ')'), ('[', ']'), ('{', '}'), ('<', '>'),
    // Symmetric
    ('|', '|'), ('/', '/'), ('!', '!'), ('#', '#'), ('~', '~'),
];

/// Strategy choosing a delimiter pair uniformly.
pub fn quote_delim_strategy() -> impl Strategy<Value = (char, char)> {
    prop::sample::select(DELIMS.to_vec())
}

/// Basic, safe regex "atoms"; keep them simple so shrinking is nice.
fn regex_atom() -> impl Strategy<Value = String> {
    prop_oneof![
        // literals
        "[A-Za-z]{1,4}".prop_map(|s| s.to_string()),
        // common escapes and classes
        prop::sample::select(vec![r"\w", r"\d", r"\s", r"\W", r"\D", r"\S", r"."]).prop_map(|s| s.to_string()),
        // anchors
        prop::sample::select(vec![r"^", r"$"]).prop_map(|s| s.to_string()),
        // tiny char classes
        prop::sample::select(vec!["[a-z]", "[A-Z]", "[0-9]", "[a-f]"]).prop_map(|s| s.to_string()),
        // a non‑capturing group with a tiny literal
        "[A-Za-z]{1,3}".prop_map(|s| format!("(?:{})", s)),
    ]
}

/// A small regex pattern as 1–5 atoms concatenated.
pub fn regex_pattern() -> impl Strategy<Value = String> {
    prop::collection::vec(regex_atom(), 1..=5).prop_map(|v| v.join(""))
}

/// Quote payloads that *may* interpolate (qq) and can contain punctuation.
/// Avoid NULs; keep short for good shrinking.
pub fn quote_payload() -> impl Strategy<Value = String> {
    prop_oneof![
        // a bit of variety; allow some $ and \ so qq is meaningfully different
        "[A-Za-z0-9 _.-]{0,8}\\$[a-z_]{1,4}[A-Za-z0-9 _.-]{0,8}".prop_map(|s| s.to_string()),
        "[A-Za-z0-9 _.-]{1,16}".prop_map(|s| s.to_string()),
        // maybe an escaped backslash chunk
        "[A-Za-z]{0,6}\\\\[A-Za-z]{0,6}".prop_map(|s| s.to_string()),
    ]
}

/// Quote payloads that **cannot** interpolate: no `$`, `@`, `\`.
pub fn quote_payload_no_interp() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 ,._-]{0,24}".prop_map(|s| s.to_string())
}

/// True if s ends with an even number of backslashes (0, 2, 4, ...).
/// This guarantees that the first char after `s` (the closing delimiter)
/// is **not** escaped by a trailing `\`.
pub fn closing_safe(s: &str) -> bool {
    s.chars().rev().take_while(|&c| c == '\\').count() % 2 == 0
}

/// Make any payload safe for immediate closing delimiter by appending
/// one extra backslash when the trailing run is odd.
pub fn closing_safe_payload<S: Into<String>>(s: S) -> String {
    let mut s = s.into();
    let tail = s.chars().rev().take_while(|&c| c == '\\').count();
    if tail % 2 == 1 { s.push('\\'); }
    s
}

/// Quote payloads that are safe to be followed by a closing delimiter.
pub fn quote_payload_safe() -> impl Strategy<Value = String> {
    quote_payload().prop_map(closing_safe_payload)
}

/// Quote payloads that cannot interpolate and are safe for closing.
pub fn quote_payload_no_interp_safe() -> impl Strategy<Value = String> {
    quote_payload_no_interp().prop_map(closing_safe_payload)
}

/// Command payloads (for qx/backticks) that are safe for closing delimiter.
pub fn command_payload_safe() -> impl Strategy<Value = String> {
    // Commands can contain backslashes too, so make them safe
    "[A-Za-z0-9 ./_-]{0,20}".prop_map(closing_safe_payload)
}

/// Quote payload that may include nested paired delimiters for stress testing.
/// Use sparingly (10-20% of cases) to avoid complicating shrinks.
pub fn quote_payload_nested_paired() -> impl Strategy<Value = String> {
    prop_oneof![
        // 80% regular safe payloads
        4 => quote_payload_safe(),
        // 20% payloads with nested paired delimiters
        1 => "[A-Za-z]{0,3}\\([A-Za-z]{0,3}\\)[A-Za-z]{0,3}".prop_map(closing_safe_payload),
        1 => "[A-Za-z]{0,3}\\{[A-Za-z]{0,3}\\}[A-Za-z]{0,3}".prop_map(closing_safe_payload),
        1 => "[A-Za-z]{0,3}\\[[A-Za-z]{0,3}\\][A-Za-z]{0,3}".prop_map(closing_safe_payload),
    ]
}

/// Helper: dedup characters, preserving first occurrence.
fn dedup_preserve_order(s: &str) -> String {
    let mut seen = HashSet::new();
    s.chars().filter(|&c| seen.insert(c)).collect()
}

/// Put modifiers in a canonical order (helps shrinking & comparisons).
fn canon_order_mods(run: &str, charset: Option<&str>) -> String {
    // canonical order for "run" flags:
    // i m s x p n g c e r o (we'll only use those that make sense for the op)
    let mut out = String::new();
    for c in ['i','m','s','x','p','n','g','c','e','r','o'] {
        if run.contains(c) { out.push(c); }
    }
    if let Some(cs) = charset {
        out.push_str(cs);
    }
    out
}

/// Choose at most one of the charset class: "", "a", "aa", "d", "l", or "u".
fn charset_flag() -> impl Strategy<Value = Option<&'static str>> {
    prop::sample::select(vec![None, Some("a"), Some("aa"), Some("d"), Some("l"), Some("u")])
}

/// `qr//` modifiers: a subset of `i m s x p n` plus one charset.
pub fn qr_modifiers() -> impl Strategy<Value = String> {
    (prop::collection::vec(prop::sample::select(vec!['i','m','s','x','p','n']), 0..=4),
     charset_flag()).prop_map(|(v, cs)| {
        let run = dedup_preserve_order(&v.into_iter().collect::<String>());
        canon_order_mods(&run, cs)
    })
}

/// `m//` modifiers: `i m s x p n` + optional `g`/`c` + one charset.
pub fn m_modifiers() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(prop::sample::select(vec!['i','m','s','x','p','n']), 0..=4),
        prop::collection::vec(prop::sample::select(vec!['g','c']), 0..=2),
        charset_flag()
    ).prop_map(|(mut a, b, cs)| {
        a.extend(b);
        let run = dedup_preserve_order(&a.into_iter().collect::<String>());
        canon_order_mods(&run, cs)
    })
}

/// `s///` modifiers: `i m s x p n` + optional `e`/`r` + one charset.
pub fn s_modifiers() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(prop::sample::select(vec!['i','m','s','x','p','n']), 0..=4),
        prop::collection::vec(prop::sample::select(vec!['e','r']), 0..=2),
        charset_flag()
    ).prop_map(|(mut a, b, cs)| {
        a.extend(b);
        let run = dedup_preserve_order(&a.into_iter().collect::<String>());
        canon_order_mods(&run, cs)
    })
}

/// `tr///`/`y///` modifiers: subset of `c d s r`.
pub fn tr_modifiers() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(vec!['c','d','s','r']), 0..=3)
        .prop_map(|v| dedup_preserve_order(&v.into_iter().collect::<String>()))
}

/// Optional: choose a delimiter pair that **avoids** all chars in `texts`.
/// (Use when you want to eliminate `prop_assume!` collisions entirely.)
#[allow(dead_code)]
pub fn delims_avoiding(texts: Vec<String>) -> impl Strategy<Value = (char,char)> {
    prop::collection::vec(Just(0u8), 0..1).prop_flat_map(move |_| {
        let forbid: HashSet<char> = texts.iter().flat_map(|t| t.chars()).collect();
        let choices: Vec<(char,char)> =
            DELIMS.iter().copied()
                  .filter(|(o,c)| !forbid.contains(o) && !forbid.contains(c))
                  .collect();
        prop::sample::select(choices)
    })
}

/* ------------------ AST shape helpers ------------------ */

use perl_parser::ast::{Node, NodeKind};

/// Depth‑first "shape" that's stable across minor AST data changes.
/// Return `Vec<String>` to keep it simple and printable in failures.
pub fn extract_ast_shape(root: &Node) -> Vec<String> {
    let mut out = Vec::new();
    extract_shape_rec(root, &mut out);
    out
}

/// Shorter alias used in some tests.
pub fn shape(root: &Node) -> Vec<String> { extract_ast_shape(root) }

fn push_variant_name(n: &Node, out: &mut Vec<String>) {
    // Variant name from Debug up to '(' or '{'
    let s = format!("{:?}", n.kind);
    let name = s.split(|c| c == '(' || c == '{').next().unwrap_or(&s).to_string();
    out.push(name);
}

fn extract_shape_rec(node: &Node, out: &mut Vec<String>) {
    use NodeKind::*;
    push_variant_name(node, out);

    match &node.kind {
        Program { statements } => {
            for s in statements { extract_shape_rec(s, out); }
        }

        VariableDeclaration { variable, initializer, .. } => {
            extract_shape_rec(variable, out);
            if let Some(init) = initializer { extract_shape_rec(init, out); }
        }

        VariableListDeclaration { variables, initializer, .. } => {
            for v in variables { extract_shape_rec(v, out); }
            if let Some(init) = initializer { extract_shape_rec(init, out); }
        }

        Assignment { lhs, rhs, .. } => {
            extract_shape_rec(lhs, out);
            extract_shape_rec(rhs, out);
        }

        Binary { left, right, .. } => {
            extract_shape_rec(left, out);
            extract_shape_rec(right, out);
        }

        Unary { operand, .. } => {
            extract_shape_rec(operand, out);
        }

        Ternary { condition, then_expr, else_expr } => {
            extract_shape_rec(condition, out);
            extract_shape_rec(then_expr, out);
            extract_shape_rec(else_expr, out);
        }

        Block { statements } => {
            for s in statements { extract_shape_rec(s, out); }
        }

        If { condition, then_branch, elsif_branches, else_branch } => {
            extract_shape_rec(condition, out);
            extract_shape_rec(then_branch, out);
            for (cond, br) in elsif_branches {
                extract_shape_rec(cond, out);
                extract_shape_rec(br, out);
            }
            if let Some(else_br) = else_branch { extract_shape_rec(else_br, out); }
        }

        While { condition, body, continue_block, .. } => {
            extract_shape_rec(condition, out);
            extract_shape_rec(body, out);
            if let Some(cont) = continue_block { extract_shape_rec(cont, out); }
        }

        Foreach { variable, list, body } => {
            extract_shape_rec(variable, out);
            extract_shape_rec(list, out);
            extract_shape_rec(body, out);
        }

        For { init, condition, update, body, continue_block } => {
            if let Some(i) = init { extract_shape_rec(i, out); }
            if let Some(c) = condition { extract_shape_rec(c, out); }
            if let Some(u) = update { extract_shape_rec(u, out); }
            extract_shape_rec(body, out);
            if let Some(cont) = continue_block { extract_shape_rec(cont, out); }
        }

        Subroutine { body, .. } => {
            extract_shape_rec(body, out);
        }

        FunctionCall { args, .. } => {
            for a in args { extract_shape_rec(a, out); }
        }

        MethodCall { object, args, .. } => {
            extract_shape_rec(object, out);
            for a in args { extract_shape_rec(a, out); }
        }

        ArrayLiteral { elements } => {
            for e in elements { extract_shape_rec(e, out); }
        }

        HashLiteral { pairs } => {
            for (k,v) in pairs {
                extract_shape_rec(k, out);
                extract_shape_rec(v, out);
            }
        }

        Return { value } => {
            if let Some(val) = value {
                extract_shape_rec(val, out);
            }
        }

        // Regex/quote‑like variants we only need to give a stable footprint for:
        Match { expr, .. } => {
            extract_shape_rec(expr, out);
            out.push("Regex".to_string());
        }
        Substitution { expr, .. } => {
            extract_shape_rec(expr, out);
            out.push("Pattern".to_string());
            out.push("Replacement".to_string());
        }
        Transliteration { expr, .. } => {
            extract_shape_rec(expr, out);
            out.push("SearchList".to_string());
            out.push("ReplaceList".to_string());
        }

        // Most leaf variants fall through:
        _ => {}
    }
}