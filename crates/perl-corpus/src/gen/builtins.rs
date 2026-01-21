use proptest::prelude::*;

fn pack_unpack() -> impl Strategy<Value = String> {
    Just(
        "my $packed = pack(\"C*\", 65, 66, 67);\nmy @bytes = unpack(\"C*\", $packed);\n"
            .to_string(),
    )
}

fn split_join() -> impl Strategy<Value = String> {
    Just(
        "my $line = \"a,b,c\";\nmy @parts = split /,/, $line;\nmy $joined = join \":\", @parts;\n"
            .to_string(),
    )
}

fn printf_sprintf() -> impl Strategy<Value = String> {
    Just(
        "my $name = \"Ada\";\nmy $count = 3;\nmy $msg = sprintf(\"%s:%d\", $name, $count);\nprintf \"%s\\n\", $msg;\n"
            .to_string(),
    )
}

fn system_call() -> impl Strategy<Value = String> {
    Just("system \"echo\", \"ok\";\n".to_string())
}

fn time_localtime() -> impl Strategy<Value = String> {
    Just("my $when = localtime(time);\n".to_string())
}

/// Generate built-in function call statements.
pub fn builtin_in_context() -> impl Strategy<Value = String> {
    prop_oneof![
        pack_unpack(),
        split_join(),
        printf_sprintf(),
        system_call(),
        time_localtime(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn builtins_include_keyword(code in builtin_in_context()) {
            assert!(
                code.contains("pack")
                    || code.contains("split")
                    || code.contains("sprintf")
                    || code.contains("system")
                    || code.contains("localtime"),
                "Expected builtin keyword in: {}",
                code
            );
        }
    }
}
