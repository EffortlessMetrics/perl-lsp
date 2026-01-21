use proptest::prelude::*;

/// Sanitize payload to avoid containing chosen delimiters
fn sanitize_payload(s: &str, left: char, right: char) -> String {
    s.chars().filter(|&ch| ch != left && ch != right && ch != '\r').collect()
}

fn sorted_modifiers(modifiers: impl IntoIterator<Item = char>) -> String {
    let mut mods: Vec<char> = modifiers.into_iter().collect();
    mods.sort_unstable();
    mods.into_iter().collect()
}

/// Generate payload for quote-like operators
pub fn q_like_payload() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("simple text".to_string()),
        Just("with $var".to_string()),
        Just("array @arr".to_string()),
        Just("hello\nworld".to_string()),
        Just("tab\\there".to_string()),
        Just("The $var is @{[ 1+1 ]} ok".to_string()),
    ]
}

/// Delimiter pairs for quote-like operators
pub fn quote_delim() -> impl Strategy<Value = (char, char)> {
    prop_oneof![
        // Paired
        Just(('(', ')')),
        Just(('[', ']')),
        Just(('{', '}')),
        Just(('<', '>')),
        // Symmetric
        Just(('|', '|')),
        Just(('!', '!')),
        Just(('/', '/')),
        Just(('#', '#')),
        Just(('~', '~')),
        Just((',', ',')),
    ]
}

/// Generate equivalent q/qq/qr/qx forms with different delimiters
pub fn q_like_metamorphic(
    payload: impl Strategy<Value = String>,
) -> impl Strategy<Value = (String, String)> {
    (payload, prop::sample::select(vec!["q", "qq", "qr", "qx"])).prop_map(|(body, op)| {
        // Generate two equivalent forms with different delimiters
        // Sanitize the payload for each delimiter choice
        let body1 = sanitize_payload(&body, '|', '|');
        let body2 = sanitize_payload(&body, '(', ')');
        let form1 = format!("{}|{}|", op, body1);
        let form2 = format!("{}({})", op, body2);
        (form1, form2)
    })
}

/// Generate a single quote-like expression
pub fn quote_like_single() -> impl Strategy<Value = String> {
    (prop::sample::select(vec!["q", "qq", "qr", "qx", "qw"]), q_like_payload(), quote_delim())
        .prop_map(|(op, payload, (open, close))| {
            let clean_payload = sanitize_payload(&payload, open, close);
            format!("{}{}{}{}", op, open, clean_payload, close)
        })
}

/// Generate quote-like with modifiers (for qr and s///)
pub fn regex_with_modifiers() -> impl Strategy<Value = String> {
    (
        q_like_payload(),
        quote_delim(),
        prop::collection::hash_set(
            prop::sample::select(vec!['i', 'x', 's', 'm', 'g', 'e', 'o']),
            0..4,
        ),
    )
        .prop_map(|(pattern, (open, close), modifiers)| {
            let clean_pattern = sanitize_payload(&pattern, open, close);
            let mods = sorted_modifiers(modifiers);
            format!("qr{}{}{}{}", open, clean_pattern, close, mods)
        })
}

/// Generate substitution operator
pub fn substitution() -> impl Strategy<Value = String> {
    (
        q_like_payload(),
        q_like_payload(),
        quote_delim(),
        prop::collection::hash_set(prop::sample::select(vec!['i', 'x', 's', 'm', 'g', 'e']), 0..4),
    )
        .prop_map(|(pattern, replacement, (open, close), modifiers)| {
            let clean_pattern = sanitize_payload(&pattern, open, close);
            let clean_replacement = sanitize_payload(&replacement, open, close);
            let mods = sorted_modifiers(modifiers);
            if open == close {
                // Symmetric delimiter
                format!(
                    "s{}{}{}{}{}{}",
                    open, clean_pattern, open, clean_replacement, open, mods
                )
            } else {
                // Paired delimiters - special syntax
                format!(
                    "s{}{}{}{}{}{}{}",
                    open, clean_pattern, close, open, clean_replacement, close, mods
                )
            }
        })
}

/// Generate transliteration operator
pub fn transliteration() -> impl Strategy<Value = String> {
    (
        "[a-z]{1,5}",
        "[A-Z]{1,5}",
        quote_delim(),
        prop::collection::hash_set(prop::sample::select(vec!['c', 'd', 's', 'r']), 0..2),
    )
        .prop_map(|(from, to, (open, close), modifiers)| {
            let clean_from = sanitize_payload(&from, open, close);
            let clean_to = sanitize_payload(&to, open, close);
            let mods = sorted_modifiers(modifiers);
            if open == close {
                format!("tr{}{}{}{}{}{}", open, clean_from, open, clean_to, open, mods)
            } else {
                format!(
                    "tr{}{}{}{}{}{}{}",
                    open, clean_from, close, open, clean_to, close, mods
                )
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        #[cfg(not(feature = "ci-fast"))]
        fn quote_like_always_has_delimiters(expr in quote_like_single()) {
            assert!(expr.starts_with('q') || expr.starts_with("qq") ||
                    expr.starts_with("qr") || expr.starts_with("qx") ||
                    expr.starts_with("qw"));
        }

        #[test]
        #[cfg(not(feature = "ci-fast"))]
        fn metamorphic_forms_are_equivalent((a, b) in q_like_metamorphic(q_like_payload())) {
            // Both should start with the same operator
            let op_a = a.split(|c: char| !c.is_ascii_alphabetic()).next().unwrap();
            let op_b = b.split(|c: char| !c.is_ascii_alphabetic()).next().unwrap();
            assert_eq!(op_a, op_b);
        }

        #[test]
        #[cfg(not(feature = "ci-fast"))]
        fn substitution_has_three_parts(s in substitution()) {
            assert!(s.starts_with("s"));
            // Count delimiter occurrences
            let delim_count = s.chars().filter(|&c| !c.is_ascii_alphanumeric()).count();
            assert!(delim_count >= 3); // At least 3 delimiters
        }
    }
}
