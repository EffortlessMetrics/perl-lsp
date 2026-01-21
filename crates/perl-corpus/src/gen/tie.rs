use proptest::prelude::*;

/// Generate tie/untie samples.
pub fn tie_in_context() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("tie my %hash, \"DB_File\", \"file.db\", 0, 0666;\nuntie %hash;\n".to_string(),),
        Just("tie my @array, \"Tie::Array\";\n".to_string()),
        Just("tie my $scalar, \"Tie::Scalar\";\n".to_string()),
        Just("tie *FH, \"Tie::Handle\";\n".to_string()),
        Just("my $obj = tie my %cache, \"Tie::StdHash\";\n$cache{a} = 1;\n".to_string(),),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn tie_contains_keyword(code in tie_in_context()) {
            assert!(code.contains("tie"));
        }
    }
}
