use perl_module::rename::line_references_package_declaration;

#[test]
fn test_package_declaration() {
    let line = "package My::Module;";
    let result = line_references_package_declaration(line, "My::Module");
    println!("package declaration with 'My::Module': {}", result);

    let result2 = line_references_package_declaration(line, "My'Module");
    println!("package declaration with 'My'Module': {}", result2);
}
