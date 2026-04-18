use perl_module::rename::line_references_qualified_call;

#[test]
fn test_true_positives() {
    assert!(line_references_qualified_call("My::Module::func();", "My::Module"));
    assert!(line_references_qualified_call("My::Module::method()", "My::Module"));
    assert!(line_references_qualified_call("$obj->My::Module::func();", "My::Module"));
}

#[test]
fn test_non_qualified_forms_rejected() {
    assert!(!line_references_qualified_call("package My::Module;", "My::Module"));
    assert!(!line_references_qualified_call("my $s = 'My::Module';", "My::Module"));
    assert!(!line_references_qualified_call("use My::Module;", "My::Module"));
    assert!(!line_references_qualified_call("# My::Module::", "My::Module"));
}

#[test]
fn test_comment_context_rejected() {
    assert!(!line_references_qualified_call("# My::Module::func", "My::Module"));
}

#[test]
fn test_string_context_rejected() {
    assert!(!line_references_qualified_call("my $s = 'My::Module::something';", "My::Module"));
    assert!(!line_references_qualified_call("my $s = \"My::Module::something\";", "My::Module"));
}

#[test]
fn test_package_declaration_with_submodule_rejected() {
    assert!(!line_references_qualified_call("package Foo::Bar::Baz;", "Foo::Bar"));
}

#[test]
fn test_call_after_closed_string_accepted() {
    assert!(line_references_qualified_call("'text'; My::Module::func()", "My::Module"));
}
