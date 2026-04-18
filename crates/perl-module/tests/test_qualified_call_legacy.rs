use perl_module::rename::line_references_qualified_call;

#[test]
fn test_qualified_call_with_legacy_separator() {
    let line = "package My::Module;";
    let result = line_references_qualified_call(line, "My'Module");
    println!("package with legacy variant 'My'Module': {}", result);
}

#[test]
fn test_qualified_call_with_canonical_separator() {
    let line = "package My::Module;";
    let result = line_references_qualified_call(line, "My::Module");
    println!("package with canonical variant 'My::Module': {}", result);
}
