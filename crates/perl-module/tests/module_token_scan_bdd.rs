use perl_module::token_parser::{ModuleTokenSpan, parse_module_token};

#[test]
fn handles_simple_canonical_module_tokens() {
    let line = "use Foo::Bar;";
    assert_eq!(parse_module_token(line, 4), Some(ModuleTokenSpan { start: 4, end: 12 }));
}

#[test]
fn handles_legacy_quote_separator_tokens() {
    let line = "use Foo'Bar;";
    assert_eq!(parse_module_token(line, 4), Some(ModuleTokenSpan { start: 4, end: 11 }));
}

#[test]
fn rejects_non_identifier_starts() {
    assert!(parse_module_token("  42Foo", 0).is_none());
    assert!(parse_module_token("use Foo::", 4).is_none());
    assert!(parse_module_token("use Foo'", 4).is_none());
}

#[test]
fn handles_multiple_segments() {
    let line = "require App::Config::Loader;";
    assert_eq!(parse_module_token(line, 8), Some(ModuleTokenSpan { start: 8, end: 27 }));
}
