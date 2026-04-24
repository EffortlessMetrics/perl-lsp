use perl_module::import::{ModuleImportKind, parse_module_import_head};

#[test]
fn given_use_statement_when_parsed_then_use_kind_and_module_token_are_returned() {
    let parsed = parse_module_import_head("use My::Module;");

    assert!(parsed.is_some());
    if let Some(head) = parsed {
        assert_eq!(head.kind, ModuleImportKind::Use);
        assert_eq!(head.token, "My::Module");
        assert_eq!(head.token_start, 4);
        assert_eq!(head.token_end, 14);
    }
}

#[test]
fn given_require_statement_when_parsed_then_require_kind_and_module_token_are_returned() {
    let parsed = parse_module_import_head("require My::Module;");

    assert!(parsed.is_some());
    if let Some(head) = parsed {
        assert_eq!(head.kind, ModuleImportKind::Require);
        assert_eq!(head.token, "My::Module");
        assert_eq!(head.token_start, 8);
        assert_eq!(head.token_end, 18);
    }
}

#[test]
fn given_use_parent_statement_when_parsed_then_use_parent_kind_is_returned() {
    let parsed = parse_module_import_head("use parent 'My::Module';");

    assert!(parsed.is_some());
    if let Some(head) = parsed {
        assert_eq!(head.kind, ModuleImportKind::UseParent);
        assert_eq!(head.token, "parent");
    }
}

#[test]
fn given_use_base_statement_when_parsed_then_use_base_kind_is_returned() {
    let parsed = parse_module_import_head("use base qw(My::Module Other::Base);");

    assert!(parsed.is_some());
    if let Some(head) = parsed {
        assert_eq!(head.kind, ModuleImportKind::UseBase);
        assert_eq!(head.token, "base");
    }
}

#[test]
fn given_non_import_line_when_parsed_then_none_is_returned() {
    assert!(parse_module_import_head("my $x = use_module();").is_none());
    assert!(parse_module_import_head("package My::Module;").is_none());
}
