use tree_sitter_perl_rs::Parser;

fn main() {
    let source = "use strict;\nmy $value = 1;\n$value + 2;\n";
    let mut parser = Parser::new();
    let Some(tree) = parser.parse(source) else {
        return;
    };

    let overlay = tree.semantic_overlay();
    let Some(offset) = source.find("$value +") else {
        eprintln!("pattern not found in source");
        return;
    };

    if let Some(definition) = overlay.definition_at_offset(offset) {
        println!(
            "definition: {} at {}..{}",
            definition.qualified_name, definition.start_byte, definition.end_byte
        );
    }

    let imports = overlay.visible_imports_at_offset(offset);
    println!(
        "visible imports: {}",
        imports.iter().map(|import| import.module.as_str()).collect::<Vec<_>>().join(", ")
    );

    let pragma_state = overlay.pragma_state_at_offset(offset);
    println!(
        "pragma state: strict_refs={}, warnings={}",
        pragma_state.strict_refs, pragma_state.warnings
    );
}
