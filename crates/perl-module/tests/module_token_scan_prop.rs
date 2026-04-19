use perl_module::token_parser::{ModuleTokenSpan, parse_module_token};
use proptest::prelude::*;

fn head_chars() -> Vec<char> {
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_"
        .chars()
        .collect()
}

fn body_chars() -> Vec<char> {
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_"
        .chars()
        .collect()
}

fn identifier_segment() -> impl Strategy<Value = String> {
    (
        prop::sample::select(head_chars()),
        prop::collection::vec(prop::sample::select(body_chars()), 0..6),
    )
        .prop_map(|(head, body)| {
            let mut token = String::new();
            token.push(head);
            token.extend(body);
            token
        })
}

fn module_name() -> impl Strategy<Value = String> {
    (
        identifier_segment(),
        prop::collection::vec(
            (identifier_segment(), prop::sample::select(vec!["::", "'"])),
            0..3,
        ),
    )
        .prop_map(|(first, rest)| {
            let mut token = first;
            for (segment, sep) in rest {
                token.push_str(sep);
                token.push_str(&segment);
            }
            token
        })
}

proptest! {
    #[test]
    fn parses_generated_module_tokens(module in module_name()) {
        let line = format!("use {module};");
        let span = parse_module_token(&line, 4)
            .ok_or_else(|| TestCaseError::Fail("should parse valid module token".into()))?;
        let expected_end = 4 + module.len();

        prop_assert_eq!(span, ModuleTokenSpan { start: 4, end: expected_end });
    }

    #[test]
    fn trailing_separator_is_rejected(module in module_name()) {
        let line = format!("{module}::");
        let expected_start = match line.find(&module) {
            Some(s) => s,
            None => { return Err(TestCaseError::Fail("module token present".into())); }
        };
        prop_assert!(parse_module_token(&line, expected_start).is_none());
    }
}
