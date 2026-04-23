use perl_module::token_core::{
    ModuleTokenSpan, has_standalone_module_token_boundaries, parse_module_token,
};

#[test]
fn parser_and_boundary_align_for_import_statement() {
    let line = "use App::Config::Loader;";
    let span = parse_module_token(line, 4);
    assert!(span.is_some(), "module token should parse");
    let span = span.unwrap_or(ModuleTokenSpan { start: 0, end: 0 });
    let expected_end = line.len() - 1;

    assert_eq!(span, ModuleTokenSpan { start: 4, end: expected_end });
    assert!(has_standalone_module_token_boundaries(line, span.start, span.end));
}

#[test]
fn parser_with_legacy_separator_still_bounds_as_standalone_token() {
    let line = "use App'Config;";
    let span = parse_module_token(line, 4);
    assert!(span.is_some(), "module token should parse");
    let span = span.unwrap_or(ModuleTokenSpan { start: 0, end: 0 });
    let token = &line[span.start..span.end];

    assert_eq!(token, "App'Config");
    assert!(has_standalone_module_token_boundaries(line, span.start, span.end));
}
