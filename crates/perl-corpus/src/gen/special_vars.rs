use proptest::prelude::*;

fn process_vars() -> impl Strategy<Value = String> {
    Just("my $pid = $$;\nmy $status = $?;\n".to_string())
}

fn program_vars() -> impl Strategy<Value = String> {
    Just("my $program = $0;\n".to_string())
}

fn os_vars() -> impl Strategy<Value = String> {
    Just("my $os = $^O;\nmy $exe = $^X;\nmy $ver = $^V;\n".to_string())
}

fn eval_error_vars() -> impl Strategy<Value = String> {
    Just("eval { die \"boom\" };\nif ($@) { warn $@; }\n".to_string())
}

fn errno_vars() -> impl Strategy<Value = String> {
    Just("open my $fh, '<', \"missing.txt\";\nmy $err = $!;\n".to_string())
}

fn line_and_separator_vars() -> impl Strategy<Value = String> {
    Just("local $/ = \"\\n\";\nmy $line = <STDIN>;\nmy $num = $.;\n".to_string())
}

fn output_separator_vars() -> impl Strategy<Value = String> {
    Just("$, = \",\";\n$; = \"\\034\";\n$\\ = \"\\n\";\n".to_string())
}

fn env_and_inc_vars() -> impl Strategy<Value = String> {
    Just("my $home = $ENV{HOME};\nmy $inc = join \":\", @INC;\n".to_string())
}

fn signal_vars() -> impl Strategy<Value = String> {
    Just("my $start = $^T;\n$SIG{INT} = sub { warn \"interrupted\"; };\n".to_string())
}

fn regex_capture_vars() -> impl Strategy<Value = String> {
    Just("if (\"abc\" =~ /(a)(b)(c)/) {\n    my @starts = @-;\n    my @ends = @+;\n}\n".to_string())
}

fn arg_vars() -> impl Strategy<Value = String> {
    Just("sub first_arg { return $_[0]; }\nmy $value = first_arg(1, 2);\n".to_string())
}

fn format_vars() -> impl Strategy<Value = String> {
    Just(
        "my $picture = \"@<<\";\nformline $picture, \"ok\";\nmy $out = $^A;\n$^A = \"\";\n"
            .to_string(),
    )
}

/// Generate statements that exercise special variables and punctuation vars.
pub fn special_vars_in_context() -> impl Strategy<Value = String> {
    prop_oneof![
        process_vars(),
        program_vars(),
        os_vars(),
        eval_error_vars(),
        errno_vars(),
        line_and_separator_vars(),
        output_separator_vars(),
        env_and_inc_vars(),
        signal_vars(),
        regex_capture_vars(),
        arg_vars(),
        format_vars(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARKERS: &[&str] = &[
        "$$", "$?", "$0", "$^O", "$^X", "$^V", "$@", "$!", "$/", "$.", "$,", "$;", "$\\", "%ENV",
        "@INC", "%SIG", "$^T", "@-", "@+", "$^A", "$_",
    ];

    proptest! {
        #[test]
        fn special_vars_include_marker(code in special_vars_in_context()) {
            assert!(
                MARKERS.iter().any(|marker| code.contains(marker)),
                "Expected special variable marker in: {}",
                code
            );
        }
    }
}
