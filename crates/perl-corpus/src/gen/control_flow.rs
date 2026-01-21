use proptest::prelude::*;

/// Generate loop control samples (next/redo/continue).
pub fn loop_with_control() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(
            "my $i = 0;\nwhile ($i < 3) {\n    $i++;\n    next if $i == 2;\n    print $i;\n}\n"
                .to_string(),
        ),
        Just(
            "my $i = 0;\nwhile ($i < 3) {\n    $i++;\n    redo if $i == 2;\n    last if $i == 3;\n}\n"
                .to_string(),
        ),
        Just(
            "OUTER: for my $i (1..3) {\n    INNER: for my $j (1..3) {\n        next OUTER if $i == $j;\n        redo INNER if $j == 1;\n    }\n}\n"
                .to_string(),
        ),
        Just(
            "for my $i (1..3) {\n    next if $i == 2;\n} continue {\n    my $j = $i * 2;\n}\n"
                .to_string(),
        ),
        Just(
            "use v5.10;\nmy $value = 2;\ngiven ($value) {\n    when (1) { print \"one\"; }\n    when (2) { print \"two\"; }\n    default { print \"other\"; }\n}\n"
                .to_string(),
        ),
        Just(
            "try {\n    die \"boom\";\n} catch ($e) {\n    warn $e;\n} finally {\n    print \"done\";\n}\n"
                .to_string(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn control_flow_contains_keywords(code in loop_with_control()) {
            assert!(
                code.contains("next")
                    || code.contains("redo")
                    || code.contains("continue")
                    || code.contains("given")
                    || code.contains("when")
                    || code.contains("try")
                    || code.contains("catch")
                    || code.contains("finally"),
                "Expected loop control keyword in: {}",
                code
            );
        }
    }
}
