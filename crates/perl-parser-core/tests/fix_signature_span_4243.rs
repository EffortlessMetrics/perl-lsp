/// Tests for issue #4243: Signature/Prototype AST nodes must have non-zero spans
/// that cover the full `(...)` text.
///
/// Before the fix, both nodes were constructed with
/// `SourceLocation { start: self.current_position(), end: self.current_position() }`
/// AFTER consuming the `(...)` tokens, yielding an empty span.
mod cpan_test_helpers;
use cpan_test_helpers::parse;
use perl_parser_core::Node;

/// Walk the AST and return the first node whose kind_name matches the given name.
fn find_node_by_kind<'a>(node: &'a Node, target: &str) -> Option<&'a Node> {
    if node.kind.kind_name() == target {
        return Some(node);
    }
    for child in node.children() {
        if let Some(found) = find_node_by_kind(child, target) {
            return Some(found);
        }
    }
    None
}

// ----- Subroutine Signature -----

#[test]
fn test_signature_span_covers_parens() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub foo ($x, $y) { 1 }";
    let ast = parse(source);
    let sig = find_node_by_kind(&ast, "Signature").ok_or("no Signature node found in AST")?;

    let expected = "($x, $y)";
    let sliced = source.get(sig.location.start..sig.location.end).ok_or_else(|| {
        format!(
            "span {}..{} out of bounds for source len {}",
            sig.location.start,
            sig.location.end,
            source.len()
        )
    })?;

    assert_eq!(
        sliced, expected,
        "Signature span should cover '($x, $y)', got {:?} (span {}..{})",
        sliced, sig.location.start, sig.location.end
    );
    Ok(())
}

#[test]
fn test_signature_span_is_nonempty() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub bar ($a) { $a + 1 }";
    let ast = parse(source);
    let sig = find_node_by_kind(&ast, "Signature").ok_or("no Signature node found in AST")?;

    assert!(
        !sig.location.is_empty(),
        "Signature location must not be empty (got span {}..{})",
        sig.location.start,
        sig.location.end
    );
    Ok(())
}

#[test]
fn test_signature_span_with_default_value() -> Result<(), Box<dyn std::error::Error>> {
    // Three-param signature with default value — verifies span covers entire `(...)`
    let source = "sub baz ($x, $y = 0, @rest) { $x }";
    let ast = parse(source);
    let sig = find_node_by_kind(&ast, "Signature").ok_or("no Signature node found in AST")?;

    let expected = "($x, $y = 0, @rest)";
    let sliced = source.get(sig.location.start..sig.location.end).ok_or_else(|| {
        format!("span {}..{} out of bounds", sig.location.start, sig.location.end)
    })?;

    assert_eq!(
        sliced, expected,
        "Signature with defaults span should cover '($x, $y = 0, @rest)', got {:?}",
        sliced
    );
    Ok(())
}

// ----- Method Signature -----

#[test]
fn test_method_signature_span_covers_parens() -> Result<(), Box<dyn std::error::Error>> {
    let source = "class Foo { method greet ($name) { } }";
    let ast = parse(source);
    let sig =
        find_node_by_kind(&ast, "Signature").ok_or("no Signature node found in method AST")?;

    let expected = "($name)";
    let sliced = source.get(sig.location.start..sig.location.end).ok_or_else(|| {
        format!("span {}..{} out of bounds", sig.location.start, sig.location.end)
    })?;

    assert_eq!(sliced, expected, "Method Signature span should cover '($name)', got {:?}", sliced);
    Ok(())
}

// ----- Prototype (bonus fix) -----

#[test]
fn test_prototype_span_covers_parens() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub add ($$) { $_[0] + $_[1] }";
    let ast = parse(source);
    let proto = find_node_by_kind(&ast, "Prototype").ok_or("no Prototype node found in AST")?;

    let expected = "($$)";
    let sliced = source.get(proto.location.start..proto.location.end).ok_or_else(|| {
        format!("span {}..{} out of bounds", proto.location.start, proto.location.end)
    })?;

    assert_eq!(sliced, expected, "Prototype span should cover '($$)', got {:?}", sliced);
    Ok(())
}

#[test]
fn test_prototype_span_is_nonempty() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub add ($$) { 1 }";
    let ast = parse(source);
    let proto = find_node_by_kind(&ast, "Prototype").ok_or("no Prototype node found in AST")?;

    assert!(
        !proto.location.is_empty(),
        "Prototype location must not be empty (got span {}..{})",
        proto.location.start,
        proto.location.end
    );
    Ok(())
}
