use perl_module::rename::line_references_qualified_call;

#[test]
fn test_qualified_call_with_package_keyword() {
    let line = "package My::Module;";
    let result = line_references_qualified_call(line, "My::Module");
    println!("package keyword + module name: {}", result);
    // The bug: this returns true when it should return false!
}
