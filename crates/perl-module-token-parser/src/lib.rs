//! Single-line Perl module token parsing.
//!
//! This microcrate has one responsibility: parse Perl package-style tokens from
//! byte offsets in a source line. It supports canonical (`::`) and legacy (`'`)
//! package separators.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

pub use perl_module_token_core::{ModuleTokenSpan, parse_module_token};

#[cfg(test)]
mod tests {
    use super::{ModuleTokenSpan, parse_module_token};

    #[test]
    fn parses_canonical_and_legacy_tokens() {
        assert_eq!(
            parse_module_token("use Foo::Bar;", 4),
            Some(ModuleTokenSpan { start: 4, end: 12 })
        );
        assert_eq!(
            parse_module_token("use Foo'Bar;", 4),
            Some(ModuleTokenSpan { start: 4, end: 11 })
        );
    }

    #[test]
    fn rejects_invalid_token_starts() {
        assert!(parse_module_token("5Foo", 0).is_none());
        assert!(parse_module_token("Foo::", 0).is_none());
        assert!(parse_module_token("Foo'", 0).is_none());
    }
}
