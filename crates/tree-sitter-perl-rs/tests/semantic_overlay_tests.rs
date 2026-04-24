use std::error::Error;

use tree_sitter_perl_rs::Parser;

#[test]
fn semantic_overlay_definition_lookup_for_reference() -> Result<(), Box<dyn Error>> {
    let source = "my $value = 41;\nmy $copy = $value;\n";
    let mut parser = Parser::new();
    let tree = parser.parse(source).ok_or("expected parse tree")?;
    let overlay = tree.semantic_overlay();

    let reference_offset = source.rfind("$value").ok_or("reference not found")?;
    let definition =
        overlay.definition_at_offset(reference_offset).ok_or("definition not found")?;

    assert_eq!(definition.name, "value");
    assert_eq!(definition.location.start, 3);
    Ok(())
}

#[test]
fn semantic_overlay_visible_imports_at_offset() -> Result<(), Box<dyn Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = 1;\n";
    let mut parser = Parser::new();
    let tree = parser.parse(source).ok_or("expected parse tree")?;
    let overlay = tree.semantic_overlay();

    let offset = source.find("my $x").ok_or("offset marker not found")?;
    let imports = overlay.visible_imports_at(offset);

    assert_eq!(imports.len(), 2);
    assert_eq!(imports[0].token, "strict");
    assert_eq!(imports[1].token, "warnings");
    Ok(())
}

#[test]
fn semantic_overlay_effective_pragma_state_at_offset() -> Result<(), Box<dyn Error>> {
    let source = "use strict;\nno strict 'refs';\nmy $x = 1;\n";
    let mut parser = Parser::new();
    let tree = parser.parse(source).ok_or("expected parse tree")?;
    let overlay = tree.semantic_overlay();

    let offset = source.find("my $x").ok_or("offset marker not found")?;
    let pragma_state = overlay.effective_pragma_state_at(offset);

    assert!(!pragma_state.strict_refs);
    assert!(pragma_state.strict_vars);
    Ok(())
}
