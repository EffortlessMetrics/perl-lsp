use perl_module::import_match::line_references_module_import;

#[test]
fn given_use_statement_when_target_matches_then_line_is_marked_for_rewrite() {
    assert!(line_references_module_import(
        "use My::Module;",
        "My::Module"
    ));
}

#[test]
fn given_require_statement_when_target_matches_then_line_is_marked_for_rewrite() {
    assert!(line_references_module_import(
        "require My::Module;",
        "My::Module"
    ));
}

#[test]
fn given_parent_or_base_statement_when_target_is_present_then_line_is_marked_for_rewrite() {
    assert!(line_references_module_import(
        "use parent qw(My::Module Other::Base);",
        "My::Module"
    ));
    assert!(line_references_module_import(
        "use base 'My::Module';",
        "My::Module"
    ));
}

#[test]
fn given_non_import_line_when_target_looks_like_module_then_line_is_not_marked() {
    assert!(!line_references_module_import(
        "my $pkg = 'My::Module';",
        "My::Module"
    ));
}

#[test]
fn given_partial_module_token_when_matching_then_line_is_not_marked() {
    assert!(!line_references_module_import(
        "use My::ModuleX;",
        "My::Module"
    ));
}
