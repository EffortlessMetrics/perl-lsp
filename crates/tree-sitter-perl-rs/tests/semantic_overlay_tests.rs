use perl_tdd_support::must_some;
use tree_sitter_perl_rs::Parser;

fn parse(source: &str) -> tree_sitter_perl_rs::Tree {
    let mut parser = Parser::new();
    must_some(parser.parse(source))
}

#[test]
fn overlay_reports_package_and_declaration_at_offset() {
    let source = "package Demo::Pkg;\nsub helper { 1 }\nhelper();\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    let helper_call = must_some(source.find("helper();"));
    assert_eq!(overlay.package_at_offset(helper_call), "Demo::Pkg");

    let declaration = must_some(overlay.declaration_at_offset(helper_call));
    assert_eq!(declaration.pkg.as_ref(), "Demo::Pkg");
    assert_eq!(declaration.name.as_ref(), "helper");
}

#[test]
fn overlay_resolves_definition_at_offset_and_node() {
    let source = "my $value = 1;\n$value += 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    let reference_offset = must_some(source.rfind("$value"));
    let by_offset = must_some(overlay.definition_at_offset(reference_offset));
    assert_eq!(by_offset.name, "value");
    assert_eq!(by_offset.location.start, 3);

    fn find_variable_at<'a>(
        node: tree_sitter_perl_rs::Node<'a>,
        offset: usize,
    ) -> Option<tree_sitter_perl_rs::Node<'a>> {
        if node.start_byte() <= offset && offset <= node.end_byte() && node.kind() == "Variable" {
            return Some(node);
        }
        for child in node.children() {
            if let Some(found) = find_variable_at(child, offset) {
                return Some(found);
            }
        }
        None
    }

    let value_node = must_some(find_variable_at(tree.root_node(), reference_offset));

    let by_node = must_some(overlay.definition_for_node(value_node));
    assert_eq!(by_node.location.start, by_offset.location.start);
}

#[test]
fn overlay_lists_visible_imports_and_effective_pragmas() {
    let source = "use strict;\nuse warnings;\nno strict 'refs';\nmy $x = 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    let statement_offset = must_some(source.find("my $x"));
    let imports = overlay.visible_imports_at(statement_offset);
    assert_eq!(imports.len(), 2);
    assert_eq!(imports[0].module, "strict");
    assert_eq!(imports[1].module, "warnings");

    let state = overlay.effective_pragma_state_at(statement_offset);
    assert!(state.strict_vars);
    assert!(state.strict_subs);
    assert!(!state.strict_refs);
    assert!(state.warnings);
}
