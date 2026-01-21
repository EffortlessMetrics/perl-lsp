use proptest::prelude::*;

use super::declarations::package_name;
use super::qw::identifier;

fn basic_sigils() -> impl Strategy<Value = String> {
    (identifier(), identifier(), identifier()).prop_map(|(scalar, array, hash)| {
        format!(
            "my ${} = 1;\nmy @{} = (1, 2, 3);\nmy %{} = (a => 1, b => 2);\n",
            scalar, array, hash
        )
    })
}

fn package_sigils() -> impl Strategy<Value = String> {
    (package_name(), identifier(), identifier(), identifier()).prop_map(
        |(pkg, scalar, array, hash)| {
            format!(
                "our ${}::{} = 1;\nour @{}::{} = (1, 2);\nour %{}::{} = (a => 1);\n",
                pkg, scalar, pkg, array, pkg, hash
            )
        },
    )
}

fn special_variable() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "$_".to_string(),
        "$!".to_string(),
        "$?".to_string(),
        "$0".to_string(),
        "$@".to_string(),
        "$^X".to_string(),
        "$^O".to_string(),
        "$^V".to_string(),
        "$|".to_string(),
        "$;".to_string(),
        "$:".to_string(),
        "$,".to_string(),
        "$#".to_string(),
        "$$".to_string(),
        "@ARGV".to_string(),
        "%ENV".to_string(),
    ])
}

fn special_variable_usage() -> impl Strategy<Value = String> {
    special_variable().prop_map(|var| {
        if var.starts_with('@') {
            format!("my @copy = {};\n", var)
        } else if var.starts_with('%') {
            format!("my %copy = {};\n", var)
        } else {
            format!("my $snapshot = {};\n", var)
        }
    })
}

fn deref_usage() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(
            "my $scalar = 42;\nmy $scalar_ref = \\$scalar;\nmy $value = $$scalar_ref;\n"
                .to_string(),
        ),
        Just(
            "my $array_ref = [1, 2, 3];\nmy @list = @$array_ref;\nmy $elem = $array_ref->[0];\n"
                .to_string(),
        ),
        Just(
            "my $hash_ref = { key => 1 };\nmy %copy = %$hash_ref;\nmy $value = $hash_ref->{key};\n"
                .to_string(),
        ),
        Just(
            "sub helper { return 1; }\nmy $code_ref = \\&helper;\nmy $result = $code_ref->();\n"
                .to_string(),
        ),
    ]
}

fn typeglob_usage() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("local *STDOUT = *DATA;\n".to_string()),
        (package_name(), package_name(), identifier(), identifier()).prop_map(
            |(pkg, other, name, other_name)| {
                format!(
                    "*{}::{} = \\&{}::{};\n",
                    pkg, name, other, other_name
                )
            },
        ),
    ]
}

fn postfix_deref_usage() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(
            "my $array_ref = [1, 2, 3];\nmy @slice = $array_ref->@[0, 2];\n".to_string(),
        ),
        Just(
            "my $hash_ref = { a => 1, b => 2 };\nmy %copy = $hash_ref->%*;\nmy @keys = $hash_ref->@{qw(a b)};\n"
                .to_string(),
        ),
    ]
}

/// Generate sigil-heavy Perl snippets (variables, special vars, deref, typeglobs).
pub fn sigil_in_context() -> impl Strategy<Value = String> {
    prop_oneof![
        basic_sigils(),
        package_sigils(),
        special_variable_usage(),
        deref_usage(),
        typeglob_usage(),
        postfix_deref_usage(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn sigil_snippets_include_sigil(code in sigil_in_context()) {
            assert!(
                code.contains('$')
                    || code.contains('@')
                    || code.contains('%')
                    || code.contains('&')
                    || code.contains('*'),
                "Expected sigil in: {}",
                code
            );
        }
    }
}
