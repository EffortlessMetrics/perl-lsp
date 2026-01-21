use proptest::prelude::*;

use super::qw::identifier;

/// Generate a Perl package name like Foo::Bar.
pub fn package_name() -> impl Strategy<Value = String> {
    prop::collection::vec(identifier(), 1..4).prop_map(|parts| parts.join("::"))
}

/// Generate a package declaration, optionally with a version or block form.
pub fn package_declaration() -> impl Strategy<Value = String> {
    (
        package_name(),
        prop::sample::select(vec!["", " 1.23", " v5.36"]),
        prop::sample::select(vec![true, false]),
    )
        .prop_map(|(name, version, block)| {
            if block {
                format!(
                    "package {}{} {{\n    sub helper {{ return 1; }}\n}}\n",
                    name, version
                )
            } else {
                format!("package {}{};\n", name, version)
            }
        })
}

/// Generate a class declaration with field and method (Perl 5.38+).
pub fn class_declaration() -> impl Strategy<Value = String> {
    (package_name(), identifier(), identifier()).prop_map(|(name, field, method)| {
        format!(
            "class {} {{\n    field ${} :param = 0;\n    method {} {{ return ${}; }}\n}}\n",
            name, field, method, field
        )
    })
}

/// Generate a stateful subroutine declaration.
pub fn stateful_subroutine() -> impl Strategy<Value = String> {
    (identifier(), prop::sample::select(vec!["0", "1", "10"])).prop_map(|(name, init)| {
        format!(
            "sub {} {{\n    state $count = {};\n    return $count++;\n}}\n",
            name, init
        )
    })
}

fn param_token() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("$x".to_string()),
        Just("$y".to_string()),
        Just("$self".to_string()),
        Just("$arg".to_string()),
        Just("$value".to_string()),
        Just("$opt = 0".to_string()),
        Just("$limit = 1".to_string()),
    ]
}

fn slurpy_token() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        Just(Some("@rest".to_string())),
        Just(Some("%opts".to_string())),
    ]
}

/// Generate a named subroutine declaration, optionally with a signature/attribute.
pub fn subroutine_declaration() -> impl Strategy<Value = String> {
    (
        identifier(),
        prop::collection::vec(param_token(), 0..3),
        slurpy_token(),
        prop::sample::select(vec![true, false]),
        prop::sample::select(vec![true, false]),
    )
        .prop_map(|(name, mut params, slurpy, use_signature, use_attribute)| {
            if let Some(extra) = slurpy {
                params.push(extra);
            }

            let signature = if use_signature {
                format!(" ({})", params.join(", "))
            } else {
                String::new()
            };
            let attribute = if use_attribute { " :method" } else { "" };

            format!(
                "sub {}{}{} {{\n    return 1;\n}}\n",
                name, attribute, signature
            )
        })
}

/// Generate an anonymous subroutine with optional signature.
pub fn anonymous_subroutine() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(param_token(), 0..3),
        slurpy_token(),
        prop::sample::select(vec![true, false]),
    )
        .prop_map(|(mut params, slurpy, use_signature)| {
            if let Some(extra) = slurpy {
                params.push(extra);
            }

            let signature = if use_signature {
                format!(" ({})", params.join(", "))
            } else {
                String::new()
            };

            format!("my $handler = sub{} {{ return 1; }};\n", signature)
        })
}

fn call_arg() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("$x".to_string()),
        Just("$y".to_string()),
        Just("1".to_string()),
        Just("\"value\"".to_string()),
        Just("@items".to_string()),
    ]
}

/// Generate a method call in context.
pub fn method_call_in_context() -> impl Strategy<Value = String> {
    (
        prop_oneof![
            Just("$obj".to_string()),
            Just("$self".to_string()),
            package_name(),
        ],
        identifier(),
        prop::collection::vec(call_arg(), 0..3),
    )
        .prop_map(|(target, method, args)| {
            let arg_list = args.join(", ");
            format!("{}->{}({});\n", target, method, arg_list)
        })
}

/// Generate use/require statements.
pub fn use_require_statement() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("use strict;\n".to_string()),
        Just("use warnings;\n".to_string()),
        Just("use v5.36;\n".to_string()),
        Just("use v5.38;\n".to_string()),
        Just("use feature ':5.36';\n".to_string()),
        Just("use feature 'signatures';\n".to_string()),
        Just("use constant PI => 3.14;\n".to_string()),
        Just("use lib \"lib\";\n".to_string()),
        Just("use base qw(Exporter);\n".to_string()),
        Just("use parent qw(Exporter);\n".to_string()),
        Just("use autodie;\n".to_string()),
        Just("no warnings 'experimental::signatures';\n".to_string()),
        package_name().prop_map(|name| format!("use {};\n", name)),
        package_name().prop_map(|name| format!("use {} 1.23;\n", name)),
        package_name().prop_map(|name| format!("require {};\n", name)),
    ]
}

/// Generate declarations in a variety of contexts.
pub fn declaration_in_context() -> impl Strategy<Value = String> {
    prop_oneof![
        package_declaration(),
        class_declaration(),
        subroutine_declaration(),
        anonymous_subroutine(),
        stateful_subroutine(),
        method_call_in_context(),
        use_require_statement(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn package_declaration_includes_keyword(code in package_declaration()) {
            assert!(code.contains("package"));
        }

        #[test]
        fn subroutine_declaration_includes_keyword(code in subroutine_declaration()) {
            assert!(code.contains("sub"));
        }

        #[test]
        fn anonymous_subroutine_includes_sub(code in anonymous_subroutine()) {
            assert!(code.contains("sub"));
        }

        #[test]
        fn method_call_includes_arrow(code in method_call_in_context()) {
            assert!(code.contains("->"));
        }

        #[test]
        fn use_require_includes_keyword(code in use_require_statement()) {
            assert!(code.contains("use") || code.contains("require") || code.contains("no"));
        }
    }
}
