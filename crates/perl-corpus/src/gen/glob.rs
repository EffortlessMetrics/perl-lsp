use proptest::prelude::*;

/// Generate glob expression samples.
pub fn glob_in_context() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("my @files = glob \"*.pl\";\n".to_string()),
        Just("my @files = <*.pm>;\n".to_string()),
        Just("my @all = glob \"**/*.pm\";\n".to_string()),
        Just("my @hidden = glob \".*\";\n".to_string()),
        Just(
            "while (my $file = glob \"*.txt\") {\n    print $file;\n}\n".to_string(),
        ),
        Just("my @matches = glob \"file{1,2,3}.txt\";\n".to_string()),
        Just("my @chars = glob \"[a-z]*.pl\";\n".to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn glob_contains_pattern(code in glob_in_context()) {
            assert!(code.contains("glob") || code.contains("<"));
        }
    }
}
