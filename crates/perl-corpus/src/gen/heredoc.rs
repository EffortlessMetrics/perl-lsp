use proptest::prelude::*;

/// Generate heredoc identifiers
pub fn heredoc_id() -> impl Strategy<Value = String> {
    prop_oneof![
        "[A-Z_]{2,8}",
        "EOF",
        "END",
        "HEREDOC",
        "SQL",
        "HTML",
        "XML",
        "JSON",
        "SHELL",
        "DATA",
    ]
    .prop_map(|s| s.to_string())
}

/// Generate heredoc content
pub fn heredoc_content() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(
        prop_oneof![
            "Line of text",
            "  Indented line",
            "\tTabbed line",
            "$var interpolation",
            "@array interpolation",
            "Plain text with spaces",
        ]
        .prop_map(|s| s.to_string()),
        1..5,
    )
}

/// Generate a basic heredoc
pub fn basic_heredoc() -> impl Strategy<Value = String> {
    (heredoc_id(), heredoc_content()).prop_map(|(id, lines)| {
        let mut result = format!("my $x = <<{};\n", id);
        for line in lines {
            result.push_str(&line);
            result.push('\n');
        }
        result.push_str(&id);
        result.push('\n');
        result
    })
}

/// Generate quoted heredoc (no interpolation)
pub fn quoted_heredoc() -> impl Strategy<Value = String> {
    (heredoc_id(), heredoc_content()).prop_map(|(id, lines)| {
        let mut result = format!("my $x = <<'{}';\n", id);
        for line in lines {
            result.push_str(&line);
            result.push('\n');
        }
        result.push_str(&id);
        result.push('\n');
        result
    })
}

/// Generate indented heredoc (Perl 5.26+)
pub fn indented_heredoc() -> impl Strategy<Value = String> {
    (heredoc_id(), heredoc_content(), "[ \\t]{0,4}").prop_map(|(id, lines, indent)| {
        let mut result = format!("my $x = <<~{};\n", id);
        for line in lines {
            result.push_str(&indent);
            result.push_str(&line);
            result.push('\n');
        }
        result.push_str(&indent);
        result.push_str(&id);
        result.push('\n');
        result
    })
}

/// Generate backtick heredoc (command execution)
pub fn backtick_heredoc() -> impl Strategy<Value = String> {
    (
        heredoc_id(),
        prop::collection::vec(
            prop::sample::select(vec!["echo 'hello'", "ls -la", "date", "pwd", "whoami"]),
            1..3,
        ),
    )
        .prop_map(|(id, commands)| {
            let mut result = format!("my $out = <<`{}`;\n", id);
            for cmd in commands {
                result.push_str(cmd);
                result.push('\n');
            }
            result.push_str(&id);
            result.push('\n');
            result
        })
}

/// Generate multiple heredocs in sequence
pub fn multiple_heredocs() -> impl Strategy<Value = String> {
    (heredoc_id(), heredoc_id(), heredoc_content(), heredoc_content()).prop_map(
        |(id1, id2, lines1, lines2)| {
            let mut result = format!("print <<{}, <<{};\n", id1, id2);

            // First heredoc body
            for line in lines1 {
                result.push_str(&line);
                result.push('\n');
            }
            result.push_str(&id1);
            result.push('\n');

            // Second heredoc body
            for line in lines2 {
                result.push_str(&line);
                result.push('\n');
            }
            result.push_str(&id2);
            result.push('\n');

            result
        },
    )
}

/// Generate heredoc in various contexts
pub fn heredoc_in_context() -> impl Strategy<Value = String> {
    (
        heredoc_id(),
        heredoc_content(),
        prop::sample::select(vec!["print ", "my $x = ", "push @arr, ", "return ", "die ", "warn "]),
    )
        .prop_map(|(id, lines, prefix)| {
            let mut result = format!("{}<<{};\n", prefix, id);
            for line in lines {
                result.push_str(&line);
                result.push('\n');
            }
            result.push_str(&id);
            if prefix.starts_with("my") {
                result.push_str(";\n");
            } else {
                result.push('\n');
            }
            result
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn heredoc_has_matching_delimiters(doc in basic_heredoc()) {
            let lines: Vec<&str> = doc.lines().collect();
            assert!(lines.len() >= 3); // At least introducer, content, terminator

            // Extract identifier from first line
            if let Some(start) = doc.find("<<") {
                let id_start = start + 2;
                if let Some(id_end) = doc[id_start..].find(';') {
                    let id = &doc[id_start..id_start + id_end];
                    // Check last line matches (before any trailing semicolon)
                    let last = lines[lines.len() - 1].trim_end_matches(';');
                    assert_eq!(id, last);
                }
            }
        }

        #[test]
        fn indented_heredoc_uses_tilde(doc in indented_heredoc()) {
            assert!(doc.contains("<<~"));
        }

        #[test]
        fn quoted_heredoc_has_quotes(doc in quoted_heredoc()) {
            assert!(doc.contains("<<'") && doc.contains("'"));
        }
    }
}
