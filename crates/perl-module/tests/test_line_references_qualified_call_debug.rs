use perl_module::rename::line_references_qualified_call;

#[test]
fn test_package_declaration_line() {
    let line = "package My::Module;";
    let result = line_references_qualified_call(line, "My::Module");
    println!("package line: {} (expected: false)", result);
    assert!(!result, "package declaration should NOT be detected as qualified call");
}

#[test]
fn test_string_literal_line() {
    let line = "my $s = 'My::Module';";
    let result = line_references_qualified_call(line, "My::Module");
    println!("string literal line: {} (expected: false)", result);
    assert!(!result, "string literal should NOT be detected as qualified call");
}

#[test]
fn test_actual_qualified_call() {
    let line = "My::Module::func();";
    let result = line_references_qualified_call(line, "My::Module");
    println!("qualified call line: {} (expected: true)", result);
    assert!(result, "actual qualified call SHOULD be detected");
}
