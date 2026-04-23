use proptest::prelude::*;

fn file_target() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "$path".to_string(),
        "$file".to_string(),
        "$target".to_string(),
        "$log".to_string(),
        "$fh".to_string(),
    ])
}

fn filetest_single() -> impl Strategy<Value = String> {
    (
        prop::sample::select(vec![
            "-e", "-f", "-d", "-r", "-w", "-x", "-s", "-z", "-l", "-T", "-B", "-o", "-O", "-u",
            "-g", "-R", "-W", "-X", "-S", "-p", "-c", "-b", "-k",
        ]),
        file_target(),
    )
        .prop_map(|(op, target)| format!("if ({} {}) {{\n    print \"ok\";\n}}\n", op, target))
}

fn filetest_time() -> impl Strategy<Value = String> {
    (prop::sample::select(vec!["-M", "-A", "-C"]), file_target())
        .prop_map(|(op, target)| format!("my $age = {} {};\n", op, target))
}

fn filetest_stacked() -> impl Strategy<Value = String> {
    file_target().prop_map(|target| format!("if (-r -w -x {}) {{\n    print \"rw\";\n}}\n", target))
}

fn filetest_handle() -> impl Strategy<Value = String> {
    Just("open my $fh, '<', \"file.txt\";\nif (-t $fh) {\n    print \"tty\";\n}\n".to_string())
}

/// Generate filetest operator statements.
pub fn filetest_in_context() -> impl Strategy<Value = String> {
    prop_oneof![filetest_single(), filetest_time(), filetest_stacked(), filetest_handle(),]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn filetests_include_operator(code in filetest_in_context()) {
            assert!(
                code.contains("-e")
                    || code.contains("-f")
                    || code.contains("-d")
                    || code.contains("-r")
                    || code.contains("-w")
                    || code.contains("-x")
                    || code.contains("-s")
                    || code.contains("-z")
                    || code.contains("-l")
                    || code.contains("-T")
                    || code.contains("-B")
                    || code.contains("-o")
                    || code.contains("-O")
                    || code.contains("-u")
                    || code.contains("-g")
                    || code.contains("-R")
                    || code.contains("-W")
                    || code.contains("-X")
                    || code.contains("-S")
                    || code.contains("-p")
                    || code.contains("-c")
                    || code.contains("-b")
                    || code.contains("-k")
                    || code.contains("-M")
                    || code.contains("-A")
                    || code.contains("-C")
                    || code.contains("-t"),
                "Expected filetest operator in: {}",
                code
            );
        }
    }
}
