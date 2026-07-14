//! Perl 5.44 (PPC0024): named parameters in signatures accept `=`, `//=`, and
//! `||=` default operators. These must parse cleanly without ERROR nodes AND
//! record the actual operator that introduced the default, so each test pairs a
//! clean-parse check with a strong whole-sequence assertion on the recovered
//! `default_operator` values — the assertion discriminates the changed parser
//! behavior rather than merely confirming the input parses.

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{Node, NodeKind};

/// Collect the `default_operator` of every `NamedParameter` in the tree, in
/// source order. A pattern-match walk (no per-node name comparison) so the
/// helper adds no branch seam of its own; the returned `Vec` supports a
/// whole-object equality assertion — the strongest discriminating oracle.
fn named_param_default_operators(source: &str) -> Vec<Option<String>> {
    fn walk(node: &Node, out: &mut Vec<Option<String>>) {
        if let NodeKind::NamedParameter { default_operator, .. } = &node.kind {
            out.push(default_operator.clone());
        }
        for child in node.children() {
            walk(child, out);
        }
    }
    let ast = parse(source);
    let mut out = Vec::new();
    walk(&ast, &mut out);
    out
}

#[test]
fn named_param_equals_default_parses() {
    let src = "sub f (:$alpha = 1) { }";
    assert_clean_parse(src);
    assert_eq!(
        named_param_default_operators(src),
        vec![Some("=".to_string())],
        ":$alpha = 1 records the `=` default operator",
    );
}

#[test]
fn named_param_defined_or_default_parses() {
    let src = "sub f (:$alpha //= 1) { }";
    assert_clean_parse(src);
    assert_eq!(
        named_param_default_operators(src),
        vec![Some("//=".to_string())],
        ":$alpha //= 1 records the `//=` default operator",
    );
}

#[test]
fn named_param_logical_or_default_parses() {
    let src = "sub f (:$alpha ||= 1) { }";
    assert_clean_parse(src);
    assert_eq!(
        named_param_default_operators(src),
        vec![Some("||=".to_string())],
        ":$alpha ||= 1 records the `||=` default operator",
    );
}

#[test]
fn mixed_named_default_operators_parse() {
    // `$host` is positional (not a NamedParameter); the three named params
    // record `=`, `//=`, `||=` in source order.
    let src = "sub configure ($host, :$port = 8080, :$secure //= 0, :$retries ||= 3) { }";
    assert_clean_parse(src);
    assert_eq!(
        named_param_default_operators(src),
        vec![Some("=".to_string()), Some("//=".to_string()), Some("||=".to_string())],
        "mixed signature records =, //=, ||= for :$port, :$secure, :$retries",
    );
}

#[test]
fn method_named_param_default_operators_parse() {
    let src = r#"
use feature 'class';
class C {
    method m (:$alpha //= 1, :$beta ||= 2) { }
}
"#;
    assert_clean_parse(src);
    assert_eq!(
        named_param_default_operators(src),
        vec![Some("//=".to_string()), Some("||=".to_string())],
        "class method named params record //= and ||= in order",
    );
}

// --- Negative / boundary coverage: the `//=` / `||=` operators are named-only.

#[test]
fn positional_defined_or_default_is_rejected() {
    // `//=` is valid only for named params (PPC0024); on a positional parameter
    // the parser must report an error rather than consume it as a default.
    assert_has_error("sub f ($x //= 1) { }", "error");
}

#[test]
fn positional_logical_or_default_is_rejected() {
    assert_has_error("sub f ($x ||= 1) { }", "error");
}

#[test]
fn named_slurpy_hash_defined_or_default_parses() {
    // The named-slurpy branch also carries the new default-operator handling:
    // the leading colon makes `:%rest` a NamedParameter (slurpy variable) that
    // records the `//=` operator.
    let src = "sub f (:%rest //= {}) { }";
    assert_clean_parse(src);
    assert_eq!(
        named_param_default_operators(src),
        vec![Some("//=".to_string())],
        ":%rest //= {{}} records the `//=` default operator",
    );
}
