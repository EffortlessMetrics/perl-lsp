//! Shared metadata for LSP hint providers.
//!
//! This crate centralizes lightweight builtin metadata used by hint providers so
//! those providers can focus on AST traversal and rendering.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use perl_builtins::builtin_signatures_phf;

/// Return parameter names for a known Perl builtin function.
#[must_use]
pub fn parameter_names(function_name: &str) -> &'static [&'static str] {
    builtin_signatures_phf::get_param_names(function_name)
}

/// Return a coarse return-type hint label for known functions.
#[must_use]
pub fn return_type_hint(function_name: &str) -> Option<&'static str> {
    match function_name {
        "new" => Some("object"),
        "split" => Some("ARRAY"),
        "keys" | "values" => Some("ARRAY"),
        "reverse" => Some("ARRAY"),
        "sort" => Some("ARRAY"),
        "grep" | "map" => Some("ARRAY"),
        "localtime" | "gmtime" => Some("ARRAY"),
        "stat" | "lstat" => Some("ARRAY"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{parameter_names, return_type_hint};

    #[test]
    fn resolves_builtin_parameter_names() {
        assert_eq!(parameter_names("substr"), ["EXPR", "OFFSET", "LENGTH", "REPLACEMENT"]);
    }

    #[test]
    fn returns_type_hints_for_known_functions() {
        assert_eq!(return_type_hint("split"), Some("ARRAY"));
        assert_eq!(return_type_hint("foo"), None);
    }
}
