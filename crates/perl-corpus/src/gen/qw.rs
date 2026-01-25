use proptest::prelude::*;
use std::collections::BTreeSet;

/// Paired and symmetric delimiters for qw
pub fn delim() -> impl Strategy<Value = (char, char)> {
    prop_oneof![
        // Paired delimiters
        Just(('(', ')')),
        Just(('[', ']')),
        Just(('{', '}')),
        Just(('<', '>')),
        // Symmetric delimiters
        Just(('|', '|')),
        Just(('!', '!')),
        Just(('#', '#')),
        Just(('/', '/')),
        Just(('~', '~')),
        Just((',', ',')),
        Just(('.', '.')),
        Just((':', ':')),
    ]
}

/// Generate valid Perl identifiers
pub fn identifier() -> impl Strategy<Value = String> {
    "[A-Za-z_][A-Za-z0-9_]{0,8}".prop_map(|s| s.to_string())
}

/// Generate a list of words for qw
pub fn words() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(identifier(), 1..6)
}

/// Generate whitespace (spaces, tabs, newlines)
pub fn whitespace() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(" ".to_string()),
        Just("  ".to_string()),
        Just("\t".to_string()),
        Just("\n".to_string()),
        Just("\n  ".to_string()),
    ]
}

/// Build a `use constant` with 1..4 qw-groups
pub fn use_constant_qw() -> impl Strategy<Value = (String, Vec<String>)> {
    (1usize..4, words(), delim(), whitespace()).prop_map(
        |(groups, word_list, (open, close), ws)| {
            let mut all_words = BTreeSet::new();
            let mut src = String::from("use constant ");

            for g in 0..groups {
                // Vary subset size by group index
                let subset: Vec<_> =
                    word_list.iter().take(1 + (g % word_list.len())).cloned().collect();

                let inner = subset.join(&ws);
                for w in &subset {
                    all_words.insert(w.clone());
                }

                src.push_str(&format!("qw{}{}{}", open, inner, close));

                if g + 1 < groups {
                    src.push_str(" => 1, ");
                } else {
                    src.push(';');
                }
            }

            (src, all_words.into_iter().collect())
        },
    )
}

/// Generate a simple qw expression
pub fn simple_qw() -> impl Strategy<Value = String> {
    (words(), delim(), whitespace()).prop_map(|(word_list, (open, close), ws)| {
        format!("qw{}{}{}", open, word_list.join(&ws), close)
    })
}

/// Generate qw with various contexts (assignment, push, etc.)
pub fn qw_in_context() -> impl Strategy<Value = String> {
    (
        simple_qw(),
        prop::sample::select(vec![
            "my @x = ",
            "push @arr, ",
            "unshift @list, ",
            "for (",
            "grep { $_ } ",
            "map { uc } ",
        ]),
    )
        .prop_map(|(qw, prefix)| {
            if prefix.starts_with("for") {
                format!("{}{}) {{ }}", prefix, qw)
            } else if prefix.starts_with("grep") || prefix.starts_with("map") {
                format!("{}{}", prefix, qw)
            } else {
                format!("{}{};", prefix, qw)
            }
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn delimiters_are_balanced((open, close) in delim()) {
            // Paired delimiters
            if open != close {
                assert!(matches!((open, close),
                    ('(', ')') | ('[', ']') | ('{', '}') | ('<', '>')));
            }
            // Symmetric delimiters
            else {
                assert!(open == close);
            }
        }

        #[test]
        fn identifiers_are_valid(id in identifier()) {
            // Identifiers must not be empty (guaranteed by generator)
            let first_char = id.chars().next()
                .expect("BUG: identifier generator produced empty string");
            assert!(first_char.is_ascii_alphabetic() || id.starts_with('_'));
            assert!(id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
        }

        #[test]
        fn use_constant_produces_valid_syntax((src, _expected) in use_constant_qw()) {
            assert!(src.starts_with("use constant "));
            assert!(src.ends_with(';'));
            assert!(src.contains("qw"));
        }
    }
}
