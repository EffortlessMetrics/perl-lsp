use perl_module::token_core::{
    ModuleTokenSpan, has_standalone_module_token_boundaries, parse_module_token,
};

#[test]
fn given_canonical_form_when_token_starts_at_offset_it_is_parsed() {
    let line = "use App::Config;";
    let span = parse_module_token(line, 4);

    assert_eq!(span, Some(ModuleTokenSpan { start: 4, end: 15 }));
}

#[test]
fn given_legacy_form_when_token_starts_at_offset_it_is_parsed() {
    let line = "use App'Config;";
    let span = parse_module_token(line, 4);

    assert_eq!(span, Some(ModuleTokenSpan { start: 4, end: 14 }));
}

#[test]
fn given_partial_standalone_prefix_when_matching_boundaries_then_false() {
    let line = "use App::Config::Loader;";
    let span = parse_module_token(line, 4);

    assert!(span.is_some(), "token should be parsed");
    let span = span.unwrap_or(ModuleTokenSpan { start: 0, end: 0 });
    let partial_end = span.start + "App::Config".len();

    assert!(!has_standalone_module_token_boundaries(line, span.start, partial_end));
}

#[test]
fn given_plain_text_when_token_absent_then_no_span() {
    assert!(parse_module_token("use App::", 4).is_none());
    assert!(parse_module_token("  42App", 0).is_none());
}
