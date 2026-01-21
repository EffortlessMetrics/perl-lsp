use proptest::prelude::*;

fn phaser_name() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("BEGIN"),
        Just("CHECK"),
        Just("UNITCHECK"),
        Just("INIT"),
        Just("END"),
    ]
}

fn phaser_statement() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("my $x = 1;".to_string()),
        Just("our $VERSION = 1.23;".to_string()),
        Just("$| = 1;".to_string()),
        Just("local $SIG{__WARN__} = sub { };".to_string()),
        Just("my $time = time;".to_string()),
        Just("state $count = 0;".to_string()),
    ]
}

fn phaser_body() -> impl Strategy<Value = String> {
    prop::collection::vec(phaser_statement(), 1..3).prop_map(|lines| lines.join("\n    "))
}

/// Generate compile-time phaser blocks (BEGIN/CHECK/UNITCHECK/INIT/END).
pub fn phaser_block() -> impl Strategy<Value = String> {
    (phaser_name(), phaser_body()).prop_map(|(name, body)| {
        format!("{} {{\n    {}\n}}\n", name, body)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn phaser_contains_keyword(code in phaser_block()) {
            assert!(
                code.contains("BEGIN")
                    || code.contains("CHECK")
                    || code.contains("UNITCHECK")
                    || code.contains("INIT")
                    || code.contains("END"),
                "Expected phaser keyword in: {}",
                code
            );
        }
    }
}
