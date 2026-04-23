use proptest::prelude::*;

use super::declarations::package_name;
use super::qw::identifier;

/// Generate a tie class name
fn tie_class() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Tie::Hash".to_string()),
        Just("Tie::StdHash".to_string()),
        Just("Tie::Array".to_string()),
        Just("Tie::StdArray".to_string()),
        Just("Tie::Scalar".to_string()),
        Just("Tie::StdScalar".to_string()),
        Just("Tie::Handle".to_string()),
        Just("Tie::StdHandle".to_string()),
        Just("DB_File".to_string()),
        Just("GDBM_File".to_string()),
        Just("NDBM_File".to_string()),
        Just("SDBM_File".to_string()),
        package_name(),
    ]
}

/// Generate optional tie arguments
fn tie_args() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::new()),
        Just(", \"file.db\"".to_string()),
        Just(", \"file.db\", 0, 0644".to_string()),
        Just(", \"file.db\", 0, 0666".to_string()),
        (identifier(), 0i32..100).prop_map(|(k, v)| format!(", {} => {}", k, v)),
    ]
}

/// Generate a tied hash declaration
fn tie_hash() -> impl Strategy<Value = String> {
    (identifier(), tie_class(), tie_args())
        .prop_map(|(name, class, args)| format!("tie my %{}, \"{}\"{};\n", name, class, args))
}

/// Generate a tied array declaration
fn tie_array() -> impl Strategy<Value = String> {
    (identifier(), tie_class(), tie_args())
        .prop_map(|(name, class, args)| format!("tie my @{}, \"{}\"{};\n", name, class, args))
}

/// Generate a tied scalar declaration
fn tie_scalar() -> impl Strategy<Value = String> {
    (identifier(), tie_class(), tie_args())
        .prop_map(|(name, class, args)| format!("tie my ${}, \"{}\"{};\n", name, class, args))
}

/// Generate a tied filehandle
fn tie_handle() -> impl Strategy<Value = String> {
    (identifier(), tie_class(), tie_args()).prop_map(|(name, class, args)| {
        format!("tie *{}, \"{}\"{};\n", name.to_uppercase(), class, args)
    })
}

/// Generate tie with object capture
fn tie_with_object() -> impl Strategy<Value = String> {
    (identifier(), identifier(), tie_class()).prop_map(|(obj, var, class)| {
        format!("my ${} = tie my %{}, \"{}\";\n${}{{\"key\"}} = 1;\n", obj, var, class, var)
    })
}

/// Generate tie followed by untie
fn tie_untie() -> impl Strategy<Value = String> {
    (identifier(), tie_class(), tie_args()).prop_map(|(name, class, args)| {
        format!(
            "tie my %{}, \"{}\"{};\n${}{{key}} = \"value\";\nuntie %{};\n",
            name, class, args, name, name
        )
    })
}

/// Generate tied() check
fn tied_check() -> impl Strategy<Value = String> {
    (identifier(), tie_class()).prop_map(|(name, class)| {
        format!(
            "tie my %{}, \"{}\";\nif (tied %{}) {{\n    print \"tied\\n\";\n}}\n",
            name, class, name
        )
    })
}

/// Generate tie with hash operations
fn tie_hash_ops() -> impl Strategy<Value = String> {
    (identifier(), tie_class(), identifier(), 0i32..100).prop_map(|(name, class, key, val)| {
        format!(
            "tie my %{}, \"{}\";\n${}{{{}}} = {};\nmy $v = ${}{{{}}};\ndelete ${}{{{}}};\n",
            name, class, name, key, val, name, key, name, key
        )
    })
}

/// Generate tie with array operations
fn tie_array_ops() -> impl Strategy<Value = String> {
    (identifier(), tie_class(), 0i32..10, 0i32..100).prop_map(|(name, class, idx, val)| {
        format!(
            "tie my @{}, \"{}\";\n${}[{}] = {};\npush @{}, {};\nmy $v = pop @{};\n",
            name, class, name, idx, val, name, val, name
        )
    })
}

/// Generate tie/untie samples.
pub fn tie_in_context() -> impl Strategy<Value = String> {
    prop_oneof![
        tie_hash(),
        tie_array(),
        tie_scalar(),
        tie_handle(),
        tie_with_object(),
        tie_untie(),
        tied_check(),
        tie_hash_ops(),
        tie_array_ops(),
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

        #[test]
        fn tie_hash_has_sigil(code in tie_hash()) {
            assert!(code.contains("%"));
            assert!(code.contains("tie"));
        }

        #[test]
        fn tie_array_has_sigil(code in tie_array()) {
            assert!(code.contains("@"));
            assert!(code.contains("tie"));
        }

        #[test]
        fn tie_scalar_has_sigil(code in tie_scalar()) {
            assert!(code.contains("$"));
            assert!(code.contains("tie"));
        }

        #[test]
        fn tie_untie_has_both(code in tie_untie()) {
            assert!(code.contains("tie"));
            assert!(code.contains("untie"));
        }
    }
}
