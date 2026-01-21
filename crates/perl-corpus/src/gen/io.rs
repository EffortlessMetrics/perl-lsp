use proptest::prelude::*;

fn file_path() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "\"input.txt\"".to_string(),
        "\"output.log\"".to_string(),
        "\"./data.pl\"".to_string(),
        "\"/tmp/report.txt\"".to_string(),
    ])
}

fn handle_name() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "$fh".to_string(),
        "$in".to_string(),
        "$out".to_string(),
        "$log".to_string(),
    ])
}

/// Generate I/O and filehandle statements in context.
pub fn io_in_context() -> impl Strategy<Value = String> {
    prop_oneof![
        (handle_name(), file_path()).prop_map(|(handle, file)| {
            format!(
                "open my {}, '<', {} or die $!;\nmy $line = <{}>;\n",
                handle, file, handle
            )
        }),
        (handle_name(), file_path()).prop_map(|(handle, file)| {
            format!(
                "open my {}, '>', {} or die $!;\nprint {{{}}} \"hello\\n\";\n",
                handle, file, handle
            )
        }),
        (handle_name(), file_path()).prop_map(|(handle, file)| {
            format!(
                "open my {}, '>>', {} or die $!;\nprint {} \"append\\n\";\n",
                handle, file, handle
            )
        }),
        (handle_name(), file_path()).prop_map(|(handle, file)| {
            format!(
                "sysopen my {}, {}, 0 or die $!;\nbinmode {};\n",
                handle, file, handle
            )
        }),
        Just(
            "opendir my $dh, \".\" or die $!;\nmy @entries = readdir $dh;\nclosedir $dh;\n"
                .to_string(),
        ),
        Just(
            "pipe my $reader, my $writer;\nprint {$writer} \"data\\n\";\nclose $writer;\nmy $line = <$reader>;\n"
                .to_string(),
        ),
        Just(
            "open my $fh, '<', \"input.txt\" or die $!;\nseek $fh, 0, 0;\nmy $pos = tell $fh;\n"
                .to_string(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn io_snippets_include_io_keywords(code in io_in_context()) {
            assert!(
                code.contains("open")
                    || code.contains("opendir")
                    || code.contains("pipe")
                    || code.contains("sysopen"),
                "Expected IO keywords in: {}",
                code
            );
        }
    }
}
