//! Tests for name_span field in Method, Class, and Format declarations.
//!
//! Issue #1697: Method, Class, and Format declarations were missing `name_span` field
//! which is required for precise LSP navigation (go-to-definition, hover, rename).

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{Node, NodeKind};
use perl_tdd_support::must_some;

// ── Helpers ────────────────────────────────────────────────────────────────────

fn find_first_node_by_kind<F>(node: &Node, predicate: F) -> Option<&Node>
where
    F: Fn(&NodeKind) -> bool + Copy,
{
    if predicate(&node.kind) {
        return Some(node);
    }
    for child in node.children() {
        if let Some(found) = find_first_node_by_kind(child, predicate) {
            return Some(found);
        }
    }
    None
}

// ── Method name_span tests ─────────────────────────────────────────────────────

/// Verify that `method foo { }` captures name_span for 'foo'
#[test]
fn test_method_has_name_span() -> Result<(), String> {
    let source = "method foo { }";
    let ast = parse(source);

    let method_node =
        must_some(find_first_node_by_kind(&ast, |k| matches!(k, NodeKind::Method { .. })));

    let NodeKind::Method { name, name_span, .. } = &method_node.kind else {
        return Err(format!("expected Method node, got {}", method_node.kind.kind_name()));
    };

    if name != "foo" {
        return Err(format!("expected method name 'foo', got '{}'", name));
    }

    let span = must_some(name_span.as_ref());
    // The name 'foo' should span from position 7 to position 10 (0-indexed)
    // "method foo { }" = 0:m 1:e 2:t 3:h 4:o 5:d 6:space 7:f 8:o 9:o
    if span.start != 7 || span.end != 10 {
        return Err(format!("expected name_span to be 7..10, got {}..{}", span.start, span.end));
    }

    Ok(())
}

/// Verify that `method foo(sig) { }` captures name_span only for 'foo', not signature
#[test]
fn test_method_with_signature_has_correct_name_span() -> Result<(), String> {
    let source = "method foo($x, $y) { }";
    let ast = parse(source);

    let method_node =
        must_some(find_first_node_by_kind(&ast, |k| matches!(k, NodeKind::Method { .. })));

    let NodeKind::Method { name, name_span, .. } = &method_node.kind else {
        return Err(format!("expected Method node, got {}", method_node.kind.kind_name()));
    };

    if name != "foo" {
        return Err(format!("expected method name 'foo', got '{}'", name));
    }

    let span = must_some(name_span.as_ref());
    // "method foo($x, $y) { }" — name_span should cover only 'foo' at 7..10
    if span.start != 7 || span.end != 10 {
        return Err(format!("expected name_span to be 7..10, got {}..{}", span.start, span.end));
    }

    Ok(())
}

/// Verify that `method foo :attr { }` captures name_span only for 'foo', not attributes
#[test]
fn test_method_with_attributes_has_correct_name_span() -> Result<(), String> {
    let source = "method foo :lvalue { }";
    let ast = parse(source);

    let method_node =
        must_some(find_first_node_by_kind(&ast, |k| matches!(k, NodeKind::Method { .. })));

    let NodeKind::Method { name, name_span, .. } = &method_node.kind else {
        return Err(format!("expected Method node, got {}", method_node.kind.kind_name()));
    };

    if name != "foo" {
        return Err(format!("expected method name 'foo', got '{}'", name));
    }

    let span = must_some(name_span.as_ref());
    // "method foo :lvalue { }" — name_span should cover only 'foo' at 7..10
    if span.start != 7 || span.end != 10 {
        return Err(format!("expected name_span to be 7..10, got {}..{}", span.start, span.end));
    }

    Ok(())
}

// ── Class name_span tests ──────────────────────────────────────────────────────

/// Verify that `class Foo { }` captures name_span for 'Foo'
#[test]
fn test_class_has_name_span() -> Result<(), String> {
    let source = "class Foo { }";
    let ast = parse(source);

    let class_node =
        must_some(find_first_node_by_kind(&ast, |k| matches!(k, NodeKind::Class { .. })));

    let NodeKind::Class { name, name_span, .. } = &class_node.kind else {
        return Err(format!("expected Class node, got {}", class_node.kind.kind_name()));
    };

    if name != "Foo" {
        return Err(format!("expected class name 'Foo', got '{}'", name));
    }

    let span = must_some(name_span.as_ref());
    // "class Foo { }" = 0:c 1:l 2:a 3:s 4:s 5:space 6:F 7:o 8:o
    // name_span should be 6..9
    if span.start != 6 || span.end != 9 {
        return Err(format!("expected name_span to be 6..9, got {}..{}", span.start, span.end));
    }

    Ok(())
}

/// Verify that `class Foo :isa(Parent) { }` captures name_span only for 'Foo'
#[test]
fn test_class_with_parents_has_correct_name_span() -> Result<(), String> {
    let source = "class Foo :isa(Parent) { }";
    let ast = parse(source);

    let class_node =
        must_some(find_first_node_by_kind(&ast, |k| matches!(k, NodeKind::Class { .. })));

    let NodeKind::Class { name, name_span, .. } = &class_node.kind else {
        return Err(format!("expected Class node, got {}", class_node.kind.kind_name()));
    };

    if name != "Foo" {
        return Err(format!("expected class name 'Foo', got '{}'", name));
    }

    let span = must_some(name_span.as_ref());
    // "class Foo :isa(Parent) { }" — name_span should cover only 'Foo' at 6..9
    if span.start != 6 || span.end != 9 {
        return Err(format!("expected name_span to be 6..9, got {}..{}", span.start, span.end));
    }

    Ok(())
}

// ── Format name_span tests ─────────────────────────────────────────────────────

/// Verify that `format MYFORMAT = ...` captures name_span for 'MYFORMAT'
#[test]
fn test_format_has_name_span() -> Result<(), String> {
    let source = "format MYFORMAT =\n@<<<\n$var\n.\n";
    let ast = parse(source);

    let format_node =
        must_some(find_first_node_by_kind(&ast, |k| matches!(k, NodeKind::Format { .. })));

    let NodeKind::Format { name, name_span, .. } = &format_node.kind else {
        return Err(format!("expected Format node, got {}", format_node.kind.kind_name()));
    };

    if name != "MYFORMAT" {
        return Err(format!("expected format name 'MYFORMAT', got '{}'", name));
    }

    let span = must_some(name_span.as_ref());
    // "format MYFORMAT =" — name_span should cover 'MYFORMAT' at 7..15
    if span.start != 7 || span.end != 15 {
        return Err(format!("expected name_span to be 7..15, got {}..{}", span.start, span.end));
    }

    Ok(())
}

/// Verify that unnamed format `format = ...` has name_span set to None or empty
#[test]
fn test_unnamed_format_has_empty_name_span() -> Result<(), String> {
    let source = "format =\n@<<<\n$var\n.\n";
    let ast = parse(source);

    let format_node =
        must_some(find_first_node_by_kind(&ast, |k| matches!(k, NodeKind::Format { .. })));

    let NodeKind::Format { name_span, .. } = &format_node.kind else {
        return Err(format!("expected Format node, got {}", format_node.kind.kind_name()));
    };

    // Unnamed format should have name_span as None
    if name_span.is_some() {
        return Err(format!("expected unnamed format to have name_span None, got {:?}", name_span));
    }

    Ok(())
}

// ── Regression tests: basic parsing still works ─────────────────────────────────

#[test]
fn test_method_parses_clean() {
    assert_clean_parse("method foo { 1 }");
}

#[test]
fn test_method_with_signature_parses_clean() {
    assert_clean_parse("method bar($x, $y) { $x + $y }");
}

#[test]
fn test_class_parses_clean() {
    assert_clean_parse("class MyClass { }");
}

#[test]
fn test_format_parses_clean() {
    assert_clean_parse("format STDOUT =\n@<<<\n$var\n.\n");
}
