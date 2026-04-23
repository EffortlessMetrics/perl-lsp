use proptest::prelude::*;

/// Generate format statement samples.
pub fn format_statement() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(
            "my ($name, $age) = (\"Ada\", 37);\nformat STDOUT =\n@<<<<<< @>>>>>\n$name, $age\n.\nwrite;\n"
                .to_string(),
        ),
        Just(
            "my $amount = 1234.50;\nformat REPORT =\nAmount: @######.##\n$amount\n.\n"
                .to_string(),
        ),
        Just(
            "format STDOUT_TOP =\nPage @<\n$%\n.\n"
                .to_string(),
        ),
        Just(
            "my $title = \"Report\";\nformat =\n@<<<<<<<<<<<<<<<<<<<<\n$title\n.\n"
                .to_string(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn format_contains_terminator(code in format_statement()) {
            assert!(code.contains("format"));
            assert!(code.contains("\n.\n"));
        }
    }
}
