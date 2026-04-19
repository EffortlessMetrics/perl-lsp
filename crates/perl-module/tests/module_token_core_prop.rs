use perl_module::token_core::{
    ModuleTokenSpan, has_standalone_module_token_boundaries, parse_module_token,
};
use proptest::prelude::*;

fn head_chars() -> impl Strategy<Value = String> {
    prop::char::range('A', 'z')
        .prop_filter("invalid head chars", |c| {
            c.is_ascii_alphabetic() || *c == '_'
        })
        .prop_map(|c| c.to_string())
}

fn body_chars() -> impl Strategy<Value = String> {
    prop::char::range('A', 'z')
        .prop_filter("invalid body chars", |c| {
            c.is_ascii_alphanumeric() || *c == '_'
        })
        .prop_map(|c| c.to_string())
}

fn token_segment() -> impl Strategy<Value = String> {
    (head_chars(), prop::collection::vec(body_chars(), 0..4)).prop_map(|(head, body)| {
        let mut token = head;
        body.into_iter().for_each(|part| token.push_str(&part));
        token
    })
}

fn module_name() -> impl Strategy<Value = String> {
    (
        token_segment(),
        prop::collection::vec(
            (token_segment(), prop::sample::select(vec!["::", "'"])),
            0..3,
        ),
    )
        .prop_map(|(first, mut rest)| {
            let mut token = first;
            for (segment, sep) in rest.drain(..) {
                token.push_str(sep);
                token.push_str(&segment);
            }
            token
        })
}

fn left_boundary_char() -> impl Strategy<Value = char> {
    prop_oneof![
        proptest::char::range('a', 'z'),
        proptest::char::range('A', 'Z'),
        proptest::char::range('0', '9'),
        Just('_'),
        Just(':'),
    ]
}

proptest! {
    #[test]
    fn prop_parsed_span_in_use_line_matches_length(module in module_name()) {
        let line = format!("use {module};");
        let span = parse_module_token(&line, 4)
            .ok_or_else(|| TestCaseError::Fail("valid module token in use line".into()))?;

        prop_assert_eq!(span, ModuleTokenSpan { start: 4, end: 4 + module.len() });
    }

    #[test]
    fn prop_standalone_boundaries_hold_for_exact_token_in_use_line(module in module_name()) {
        let line = format!("use {module};");
        let token = &line[4..4 + module.len()];
        let span = parse_module_token(&line, 4)
            .ok_or_else(|| TestCaseError::Fail("valid module token in use line".into()))?;

        prop_assert_eq!(token, &line[span.start..span.end]);
        prop_assert!(has_standalone_module_token_boundaries(&line, span.start, span.end));
    }

    #[test]
    fn prop_left_boundary_rejects_embedded_tokens(module in module_name(), prefix in left_boundary_char()) {
        let line = format!("{prefix}{module};");
        let start = prefix.len_utf8();
        let span = parse_module_token(&line, start)
            .ok_or_else(|| TestCaseError::Fail("embedded token should still parse from explicit offset".into()))?;

        prop_assert_eq!(span, ModuleTokenSpan { start, end: start + module.len() });
        prop_assert!(!has_standalone_module_token_boundaries(&line, span.start, span.end));
    }

    #[test]
    fn prop_right_boundary_rejects_colon_context(module in module_name()) {
        let line = format!("use {module}:");
        let span = parse_module_token(&line, 4)
            .ok_or_else(|| TestCaseError::Fail("token before colon should parse from explicit offset".into()))?;

        prop_assert_eq!(span, ModuleTokenSpan { start: 4, end: 4 + module.len() });
        prop_assert!(!has_standalone_module_token_boundaries(&line, span.start, span.end));
    }

    #[test]
    fn prop_apostrophe_context_breaks_standalone_boundaries(
        module in module_name(),
        left_ident in token_segment(),
        right_ident in token_segment(),
    ) {
        let left_line = format!("{left_ident}'{module};");
        let left_start = left_ident.len() + 1;
        let left_span = parse_module_token(&left_line, left_start)
            .ok_or_else(|| TestCaseError::Fail("token after legacy separator should parse from explicit offset".into()))?;
        prop_assert!(!has_standalone_module_token_boundaries(&left_line, left_span.start, left_span.end));

        let right_line = format!("use {module}::{right_ident}'{right_ident}");
        let right_end = 4 + module.len();
        prop_assert!(!has_standalone_module_token_boundaries(&right_line, 4, right_end));
    }

    #[test]
    fn prop_trailing_separators_are_rejected(
        module in module_name(),
        separator in prop_oneof![Just("::"), Just("'")],
    ) {
        let line = format!("{module}{separator}");
        prop_assert!(parse_module_token(&line, 0).is_none());
    }

    #[test]
    fn prop_never_panics_on_random_offsets(line in ".{0,128}", start in 0..129usize) {
        if start <= line.len() {
            let span = parse_module_token(&line, start);
            if let Some(span) = span {
                prop_assert!(span.start <= span.end);
                prop_assert!(span.end <= line.len());
            }
        }
    }
}
