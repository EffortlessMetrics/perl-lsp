use perl_module::reference::{extract_module_reference, find_module_reference};
use perl_module::token_parser::parse_module_token;

#[test]
fn parser_and_reference_coordinates_match_for_use_statement() {
    let source = "use App::Config::Loader;\n";
    let line = source.lines().next().unwrap_or("");
    let start = line.find("App::Config::Loader").unwrap_or(0);

    let span = parse_module_token(line, start);
    assert!(span.is_some(), "valid token span");
    let span = span.unwrap_or(perl_module::token_parser::ModuleTokenSpan { start: 0, end: 0 });

    let reference = find_module_reference(source, span.start);
    assert!(reference.is_some(), "cursor match in source");
    if let Some(reference) = reference {
        assert_eq!(reference.module_start, span.start);
        assert_eq!(reference.module_end, span.end);
        assert_eq!(reference.canonical_module_name(), "App::Config::Loader");
    }

    let extracted = extract_module_reference(source, span.start);
    assert_eq!(extracted, Some("App::Config::Loader".to_string()));
}

#[test]
fn parser_with_legacy_separator_still_informs_reference_extraction() {
    let source = "use Foo'Bar;\n";
    let line = source.lines().next().unwrap_or("");
    let start = line.find("Foo").unwrap_or(0);

    let span = parse_module_token(line, start);
    assert!(span.is_some(), "valid legacy token span");
    let span = span.unwrap_or(perl_module::token_parser::ModuleTokenSpan { start: 0, end: 0 });

    let extracted = extract_module_reference(source, start + 3);

    let span2 = parse_module_token(line, start)
        .unwrap_or(perl_module::token_parser::ModuleTokenSpan { start: 0, end: 0 });
    assert_eq!(span, span2);
    assert_eq!(extracted, Some("Foo::Bar".to_string()));
    assert_eq!(span.start, start);
}
