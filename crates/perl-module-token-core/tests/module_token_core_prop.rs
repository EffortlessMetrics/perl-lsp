use perl_module_token_core::{
    ModuleTokenSpan, has_standalone_module_token_boundaries, parse_module_token,
};
use proptest::prelude::*;

fn head_chars() -> impl Strategy<Value = String> {
    prop::char::range('A', 'z')
        .prop_filter("invalid head chars", |c| c.is_ascii_alphabetic() || *c == '_')
        .prop_map(|c| c.to_string())
}

fn body_chars() -> impl Strategy<Value = String> {
    prop::char::range('A', 'z')
        .prop_filter("invalid body chars", |c| c.is_ascii_alphanumeric() || *c == '_')
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
        prop::collection::vec((token_segment(), prop::sample::select(vec!["::", "'"])), 0..3),
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
