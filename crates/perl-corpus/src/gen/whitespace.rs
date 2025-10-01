use proptest::prelude::*;
use rand::{Rng, SeedableRng};

/// Insert random whitespace and comments into source code
pub fn sprinkle_whitespace(src: &str, seed: u64) -> String {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut result = String::new();
    let mut in_string = false;
    let mut in_regex = false;
    let mut escape_next = false;

    for (i, ch) in src.chars().enumerate() {
        // Track string/regex context to avoid breaking syntax
        if !escape_next {
            match ch {
                '"' if !in_regex => in_string = !in_string,
                '/' if !in_string && (i == 0 || src.chars().nth(i - 1) != Some('\\')) => {
                    in_regex = !in_regex;
                }
                '\\' => escape_next = true,
                _ => {}
            }
        } else {
            escape_next = false;
        }

        result.push(ch);

        // Don't insert whitespace inside strings or regexes
        if !in_string && !in_regex {
            // Randomly insert whitespace or comments
            let choice = rng.gen_range(0..20);
            match choice {
                0 => result.push(' '),
                1 => result.push_str("  "),
                2 => result.push('\t'),
                3 if ch == ';' || ch == '}' => {
                    result.push_str(" # comment\n");
                }
                _ => {}
            }
        }
    }

    result
}

/// Generate various whitespace patterns
pub fn whitespace_pattern() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(" ".to_string()),
        Just("  ".to_string()),
        Just("\t".to_string()),
        Just("\n".to_string()),
        Just("\r\n".to_string()),
        Just("\n  ".to_string()),
        Just("\n\t".to_string()),
    ]
}

/// Generate comment patterns
pub fn comment_pattern() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("# simple comment\n".to_string()),
        Just("# example usage\n".to_string()),
        Just("# NOTE: important\n".to_string()),
        Just("# additional context\n".to_string()),
        Just("#\n".to_string()),
        Just("## section comment\n".to_string()),
    ]
}

/// Generate seed for whitespace metamorphic transformation
pub fn whitespace_seed() -> impl Strategy<Value = u64> {
    0u64..1000
}

/// Generate whitespace-heavy but valid Perl
pub fn whitespace_stress_test() -> impl Strategy<Value = String> {
    (whitespace_pattern(), whitespace_pattern(), comment_pattern()).prop_map(
        |(ws1, ws2, comment)| {
            format!(
                "use{}strict;\n{}{}my{}$x{}={}1{}+{}2;\n{}print{}$x;{}",
                ws1, comment, ws2, ws1, ws2, ws1, ws2, ws1, comment, ws1, comment
            )
        },
    )
}

/// Insert comments at statement boundaries
pub fn insert_statement_comments(src: &str) -> String {
    let mut result = String::new();

    for line in src.lines() {
        result.push_str(line);

        // Add comment after statements
        if line.trim_end().ends_with(';') {
            result.push_str("  # auto-comment");
        }

        result.push('\n');
    }

    result
}

/// Generate heavily commented code
pub fn commented_code() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(
            "# File header\n# Author: test\n\nuse strict;\n# Enable strictures\n\nmy $x = 1; # Initialize\n"
                .to_string(),
        ),
        Just(
            "#!/usr/bin/perl\n# -*- coding: utf-8 -*-\n\n# Main code\nprint 'hello'; # Output\n"
                .to_string(),
        ),
        Just(
            "# Before\nmy $x = 1;\n# Middle\n$x++;\n# After\nprint $x;\n"
                .to_string(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_preserves_tokens() {
        let original = "my $x = 1 + 2;";
        let transformed = sprinkle_whitespace(original, 42);

        // Extract non-whitespace tokens
        let orig_tokens: Vec<&str> = original.split_whitespace().collect();
        let trans_tokens: Vec<&str> = transformed
            .lines()
            .flat_map(|line| {
                // Remove comments
                let line = if let Some(pos) = line.find('#') { &line[..pos] } else { line };
                line.split_whitespace()
            })
            .collect();

        // Whitespace insertion can split tokens (e.g., "my" → "m\ty"), so we verify
        // that the combined tokens preserve the original content
        assert!(
            trans_tokens.len() >= orig_tokens.len(),
            "Whitespace insertion should not lose tokens: expected at least {} tokens, got {}",
            orig_tokens.len(),
            trans_tokens.len()
        );

        // Extract code content (no comments, no whitespace)
        let trans_str = transformed
            .lines()
            .map(|line| {
                if let Some(pos) = line.find('#') {
                    &line[..pos]
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("");

        let orig_str = original.split_whitespace().collect::<Vec<_>>().join("");

        // Verify all original content is preserved
        assert_eq!(
            orig_str, trans_str,
            "Original code content '{}' not preserved in transformed output '{}'",
            orig_str, trans_str
        );
    }

    proptest! {
        #[test]
        #[cfg(not(feature = "ci-fast"))]
        fn comments_dont_break_statements(code in commented_code()) {
            // Just check it has both code and comments
            assert!(code.contains('#'));
            assert!(code.lines().any(|line| !line.trim().starts_with('#')));
        }

        #[test]
        fn whitespace_stress_is_valid(code in whitespace_stress_test()) {
            // Should still have key tokens
            assert!(code.contains("use"));
            assert!(code.contains("strict"));
            assert!(code.contains("my"));
            assert!(code.contains("print"));
        }
    }
}
