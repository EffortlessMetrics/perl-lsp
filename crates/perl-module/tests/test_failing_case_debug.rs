use perl_module::rename::plan_module_rename_edits;

#[test]
fn test_failing_case_debug() {
    let source = "package My::Module;\nmy $s = 'My::Module';\n";

    println!("\nSource:");
    println!("{:?}", source);

    let edits = plan_module_rename_edits(source, "My::Module", "My::Renamed");

    println!("\nEdits generated: {}", edits.len());
    for (i, edit) in edits.iter().enumerate() {
        println!("  Edit {}: line={}, new_text={:?}", i, edit.line, edit.new_text);
    }

    assert!(edits.is_empty(), "Should not generate any edits");
}
