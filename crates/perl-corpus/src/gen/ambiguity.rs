use proptest::prelude::*;

/// Generate parser-ambiguity stress snippets (slash vs regex, hash vs block, indirect objects).
pub fn ambiguity_in_context() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(
            "my $ratio = $a / $b;\nmy $match = $a =~ /$b/;\nmy $complex = $x / $y / $z;\nmy $regex = /$x\\/$y/;\n"
                .to_string(),
        ),
        Just(
            "sub handle { return 1; }\nhandle { key => 1 };\nhandle({ key => 1 });\n"
                .to_string(),
        ),
        Just(
            "my $logger = new Logger \"app.log\";\nmy $time = new DateTime (year => 2024, month => 1, day => 1);\n"
                .to_string(),
        ),
        Just(
            "my $value = 0;\nif ($value) {\n    if ($value > 1) {\n        if ($value > 2) {\n            if ($value > 3) {\n                if ($value > 4) {\n                    $value++;\n                }\n            }\n        }\n    }\n}\n"
                .to_string(),
        ),
        Just(
            "my $raw = q!literal!;\nmy $interp = qq{value=$raw};\nmy $cmd = qx|echo ok|;\nmy $re = qr#foo.+bar#i;\n"
                .to_string(),
        ),
        Just("print <<'A', <<'B';\nalpha\nA\nbeta\nB\n".to_string()),
        Just(
            "my $danger = \"aaaaaaaaab\";\nif ($danger =~ /^(a+)+b$/) {\n    print \"ok\";\n}\n"
                .to_string(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn ambiguity_snippets_include_construct(code in ambiguity_in_context()) {
            assert!(
                code.contains(" / ")
                    || code.contains("=~")
                    || code.contains("handle {")
                    || code.contains("new Logger")
                    || code.contains("<<")
                    || code.contains("q!")
                    || code.contains("qr#")
                    || code.contains("^(a+)+")
                    || code.contains("if ("),
                "Expected ambiguity construct in: {}",
                code
            );
        }
    }
}
