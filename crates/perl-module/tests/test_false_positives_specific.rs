use perl_module::rename::line_references_qualified_call;

#[test]
fn test_comment_false_positive() {
    // This is the main bug: qualified_call matches inside comments
    let result = line_references_qualified_call("# My::Module::func", "My::Module");
    println!("Comment line result: {}", result);
    assert!(!result, "Qualified call should NOT match inside comments");
}

#[test]
fn test_string_literal_false_positive() {
    // String literal case
    let result1 = line_references_qualified_call("my $s = 'My::Module::something';", "My::Module");
    println!("String literal result: {}", result1);

    // Regular qualified call
    let result2 = line_references_qualified_call("My::Module::func();", "My::Module");
    println!("Regular qualified call result: {}", result2);
}

#[test]
fn test_double_quoted_string() {
    let result = line_references_qualified_call("my $s = \"My::Module::something\";", "My::Module");
    println!("Double-quoted string result: {}", result);
    assert!(!result, "Qualified call should NOT match in double-quoted strings");
}
