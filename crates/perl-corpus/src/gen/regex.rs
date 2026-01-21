use proptest::prelude::*;

use super::quote_like::{q_like_payload, quote_delim, regex_with_modifiers, substitution, transliteration};

fn sanitize_payload(s: &str, left: char, right: char) -> String {
    s.chars()
        .filter(|&ch| ch != left && ch != right && ch != '\r')
        .collect()
}

fn sorted_modifiers(modifiers: impl IntoIterator<Item = char>) -> String {
    let mut mods: Vec<char> = modifiers.into_iter().collect();
    mods.sort_unstable();
    mods.into_iter().collect()
}

fn regex_match_expr() -> impl Strategy<Value = String> {
    (
        q_like_payload(),
        quote_delim(),
        prop::collection::hash_set(prop::sample::select(vec!['i', 'm', 's', 'x']), 0..3),
    )
        .prop_map(|(pattern, (open, close), modifiers)| {
            let clean_pattern = sanitize_payload(&pattern, open, close);
            let mods = sorted_modifiers(modifiers);
            format!("m{}{}{}{}", open, clean_pattern, close, mods)
        })
}

fn advanced_pattern() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("(?<word>foo)".to_string()),
        Just("(?|foo|bar)".to_string()),
        Just("(?>foo|fo)bar".to_string()),
        Just("foo(?=bar)".to_string()),
        Just("(?<=foo)bar".to_string()),
        Just("(?R)".to_string()),
        Just("\\p{Latin}+".to_string()),
        Just("a(*SKIP)(*FAIL)|abc".to_string()),
    ]
}

fn regex_match_advanced_expr() -> impl Strategy<Value = String> {
    (
        advanced_pattern(),
        quote_delim(),
        prop::collection::hash_set(prop::sample::select(vec!['i', 'm', 's', 'x']), 0..3),
    )
        .prop_map(|(pattern, (open, close), modifiers)| {
            let clean_pattern = sanitize_payload(&pattern, open, close);
            let mods = sorted_modifiers(modifiers);
            format!("m{}{}{}{}", open, clean_pattern, close, mods)
        })
}

/// Generate regex match statements in common contexts.
pub fn regex_match_in_context() -> impl Strategy<Value = String> {
    let target = prop::sample::select(vec!["$text", "$line", "$input", "$_"]);

    prop_oneof![
        (target.clone(), regex_match_expr())
            .prop_map(|(target, expr)| format!("{} =~ {};\n", target, expr)),
        (target.clone(), regex_match_expr())
            .prop_map(|(target, expr)| {
                format!(
                    "if ({} =~ {}) {{\n    print {};\n}}\n",
                    target, expr, target
                )
            }),
        (target, regex_with_modifiers()).prop_map(|(target, expr)| {
            format!(
                "my $re = {};\nif ({} =~ $re) {{\n    print {};\n}}\n",
                expr, target, target
            )
        }),
    ]
}

/// Generate advanced regex match statements (branch reset, lookarounds, verbs).
pub fn regex_advanced_match_in_context() -> impl Strategy<Value = String> {
    let target = prop::sample::select(vec!["$text", "$line", "$input", "$_"]);
    (target, regex_match_advanced_expr())
        .prop_map(|(target, expr)| format!("{} =~ {};\n", target, expr))
}

/// Generate substitution statements in context.
pub fn substitution_in_context() -> impl Strategy<Value = String> {
    (
        prop::sample::select(vec!["$text", "$line", "$value", "$_"]),
        substitution(),
    )
        .prop_map(|(target, expr)| format!("{} =~ {};\n", target, expr))
}

/// Generate transliteration statements in context.
pub fn transliteration_in_context() -> impl Strategy<Value = String> {
    (
        prop::sample::select(vec!["$text", "$line", "$value", "$_"]),
        transliteration(),
    )
        .prop_map(|(target, expr)| format!("{} =~ {};\n", target, expr))
}

/// Generate regex-related statements (match, substitution, transliteration).
pub fn regex_in_context() -> impl Strategy<Value = String> {
    prop_oneof![
        regex_match_in_context(),
        regex_advanced_match_in_context(),
        substitution_in_context(),
        transliteration_in_context(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn regex_contains_match_operator(code in regex_in_context()) {
            assert!(code.contains("=~"));
        }
    }
}
