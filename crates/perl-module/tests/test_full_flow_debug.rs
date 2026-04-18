use perl_module::{
    import_match::line_references_module_import, line_references_isa_assignment,
    line_references_package_declaration, line_references_qualified_call, plan_module_rename_edits,
};

#[test]
fn test_full_flow_for_package_line() {
    let line = "package My::Module;";
    let module = "My::Module";

    println!("\nTesting line: {:?}", line);
    println!("Module: {:?}\n", module);

    println!("line_references_module_import: {}", line_references_module_import(line, module));
    println!("line_references_isa_assignment: {}", line_references_isa_assignment(line, module));
    println!("line_references_qualified_call: {}", line_references_qualified_call(line, module));
    println!(
        "line_references_package_declaration: {}",
        line_references_package_declaration(line, module)
    );
}

#[test]
fn test_plan_module_rename_edits_debug() {
    let source = "package My::Module;\nmy $s = 'My::Module';\n";

    println!("\nSource:\n{}\n", source);

    let edits = plan_module_rename_edits(source, "My::Module", "My::Renamed");

    println!("Total edits: {}", edits.len());
    for (i, edit) in edits.iter().enumerate() {
        println!("Edit {}: line={}, new_text={:?}", i, edit.line, edit.new_text);
    }
}
