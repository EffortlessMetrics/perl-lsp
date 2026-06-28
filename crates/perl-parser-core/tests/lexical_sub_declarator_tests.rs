mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{Node, NodeKind};

/// Extract the declarator field from the first named subroutine in the AST.
/// Only looks at top-level Program statements to avoid interference from
/// nested subs (closures, inner subs, etc.).
fn extract_subroutine_declarator(ast: &Node) -> Option<String> {
    match &ast.kind {
        NodeKind::Program { statements } => {
            for stmt in statements {
                if let NodeKind::Subroutine { declarator, name, .. } = &stmt.kind {
                    if name.is_some() {
                        return declarator.clone();
                    }
                }
            }
        }
        _ => {}
    }
    None
}

/// Extract declarator fields from ALL top-level named subroutines in program order.
fn extract_all_subroutine_declarators(ast: &Node) -> Vec<(String, Option<String>)> {
    let mut result = Vec::new();
    if let NodeKind::Program { statements } = &ast.kind {
        for stmt in statements {
            if let NodeKind::Subroutine { declarator, name, .. } = &stmt.kind {
                if let Some(n) = name {
                    result.push((n.clone(), declarator.clone()));
                }
            }
        }
    }
    result
}

// ── Basic cases ──────────────────────────────────────────────────────────────

#[test]
fn test_my_sub_declarator() {
    let source = r#"my sub counter { return 42; }"#;
    assert_clean_parse(source);
    let ast = parse(source);
    let declarator = extract_subroutine_declarator(&ast);
    assert_eq!(
        declarator,
        Some("my".to_string()),
        "my sub should have declarator field set to 'my'"
    );
}

#[test]
fn test_our_sub_declarator() {
    let source = r#"our sub global_sub { return 42; }"#;
    assert_clean_parse(source);
    let ast = parse(source);
    let declarator = extract_subroutine_declarator(&ast);
    assert_eq!(
        declarator,
        Some("our".to_string()),
        "our sub should have declarator field set to 'our'"
    );
}

#[test]
fn test_state_sub_declarator() {
    let source = r#"state sub memo { return 42; }"#;
    assert_clean_parse(source);
    let ast = parse(source);
    let declarator = extract_subroutine_declarator(&ast);
    assert_eq!(
        declarator,
        Some("state".to_string()),
        "state sub should have declarator field set to 'state'"
    );
}

#[test]
fn test_regular_sub_no_declarator() {
    let source = r#"sub regular { return 42; }"#;
    assert_clean_parse(source);
    let ast = parse(source);
    let declarator = extract_subroutine_declarator(&ast);
    assert_eq!(declarator, None, "regular sub should have no declarator (None)");
}

// ── Edge cases ───────────────────────────────────────────────────────────────

/// A `my sub` with a prototype — the declarator must survive the prototype
/// parsing path (`my sub NAME ($$) { ... }`).
#[test]
fn test_my_sub_with_prototype_has_declarator() {
    let source = r#"my sub add ($$) { $_[0] + $_[1] }"#;
    assert_clean_parse(source);
    let ast = parse(source);
    let declarator = extract_subroutine_declarator(&ast);
    assert_eq!(
        declarator,
        Some("my".to_string()),
        "my sub with prototype should still carry declarator='my'"
    );
}

/// A `my sub` with a Perl 5.20+ signature — declarator must survive the
/// signature parsing path.
#[test]
fn test_my_sub_with_signature_has_declarator() {
    let source = r#"my sub greet ($name) { "hello $name" }"#;
    assert_clean_parse(source);
    let ast = parse(source);
    let declarator = extract_subroutine_declarator(&ast);
    assert_eq!(
        declarator,
        Some("my".to_string()),
        "my sub with signature should still carry declarator='my'"
    );
}

/// A `my sub` forward declaration (stub, no body) — the forward-declaration
/// path in parse_subroutine() must not suppress the declarator injected
/// by the statements.rs caller.
#[test]
fn test_my_sub_forward_declaration_has_declarator() {
    let source = r#"my sub helper;"#;
    assert_clean_parse(source);
    let ast = parse(source);
    let declarator = extract_subroutine_declarator(&ast);
    assert_eq!(
        declarator,
        Some("my".to_string()),
        "my sub forward declaration (stub) should carry declarator='my'"
    );
}

/// Multiple scoped subs in sequence — each must get its own correct declarator,
/// not inherit the declarator of a sibling.
#[test]
fn test_multiple_scoped_subs_have_correct_declarators() {
    let source = r#"
my sub lexical { 1 }
our sub exported { 2 }
state sub memoized { 3 }
sub plain { 4 }
"#;
    assert_clean_parse(source);
    let ast = parse(source);
    let subs = extract_all_subroutine_declarators(&ast);

    assert_eq!(subs.len(), 4, "expected 4 subroutines, got {}", subs.len());
    assert_eq!(
        subs[0],
        ("lexical".to_string(), Some("my".to_string())),
        "first sub (my) wrong: {:?}",
        subs[0]
    );
    assert_eq!(
        subs[1],
        ("exported".to_string(), Some("our".to_string())),
        "second sub (our) wrong: {:?}",
        subs[1]
    );
    assert_eq!(
        subs[2],
        ("memoized".to_string(), Some("state".to_string())),
        "third sub (state) wrong: {:?}",
        subs[2]
    );
    assert_eq!(subs[3], ("plain".to_string(), None), "fourth sub (plain) wrong: {:?}", subs[3]);
}

/// A `my sub` nested inside a closure should not corrupt the outer closure's
/// state. The outer named sub (plain `sub`) should have no declarator.
#[test]
fn test_nested_my_sub_does_not_affect_outer_sub() {
    let source = r#"
sub outer {
    my sub inner { 99 }
    inner()
}
"#;
    assert_clean_parse(source);
    let ast = parse(source);
    // outer is a plain sub — no declarator
    let declarator = extract_subroutine_declarator(&ast);
    assert_eq!(
        declarator, None,
        "outer plain sub should have no declarator even when it contains a my sub"
    );
}
