use perl_text_line::{line_bounds_at, skip_ascii_whitespace};
use proptest::prelude::*;

fn ascii_text_chars() -> Vec<char> {
    let mut chars: Vec<char> = vec!['\n', ' ', '\t', '\r'];
    chars.extend('a'..='z');
    chars.extend('A'..='Z');
    chars.extend('0'..='9');
    chars.extend([':', ';', '_', '\'', '/', '[', ']']);
    chars
}

fn text_and_cursor() -> impl Strategy<Value = (String, usize)> {
    prop::collection::vec(prop::sample::select(ascii_text_chars()), 0..=256).prop_flat_map(
        |chars| {
            let text: String = chars.into_iter().collect();
            let len = text.len();
            (Just(text), 0usize..=len)
        },
    )
}

fn spaces_and_cursor() -> impl Strategy<Value = (String, usize)> {
    prop::collection::vec(Just(' '), 0..20).prop_flat_map(|chars| {
        let text: String = chars.into_iter().collect();
        let len = text.len();
        (Just(text), 0usize..=len)
    })
}

proptest! {
    #[test]
    fn line_bounds_is_derived_from_nearest_newlines((text, cursor) in text_and_cursor()) {
        let (start, end) = line_bounds_at(&text, cursor);
        let expected_start = text[..cursor].rfind('\n').map_or(0, |idx| idx + 1);
        let expected_end = text[cursor..].find('\n').map_or(text.len(), |idx| cursor + idx);

        assert_eq!(start, expected_start);
        assert_eq!(end, expected_end);
        assert!(start <= cursor);
        assert!(cursor <= end);
        assert!(!text[start..end].contains('\n'));
    }

    #[test]
    fn skip_ascii_whitespace_only_advances_from_ascii_space((text, cursor) in spaces_and_cursor()) {
        let parsed = skip_ascii_whitespace(text.as_bytes(), cursor);
        assert!(parsed <= text.len());
    }
}
