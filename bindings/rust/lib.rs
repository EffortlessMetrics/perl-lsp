//! This crate provides perl language support for the [tree-sitter][] parsing library.
//!
//! Typically, you will use the [language][language func] function to add this language to a
//! tree-sitter [Parser][], and then use the parser to parse some code:
//!
//! ```
//! let code = "";
//! let mut parser = tree_sitter::Parser::new();
//! parser.set_language(&tree_sitter_perl::language()).expect("Error loading perl grammar");
//! let tree = parser.parse(code, None).unwrap();
//! ```
//!
//! [Language]: https://docs.rs/tree-sitter/*/tree_sitter/struct.Language.html
//! [language func]: fn.language.html
//! [Parser]: https://docs.rs/tree-sitter/*/tree_sitter/struct.Parser.html
//! [tree-sitter]: https://tree-sitter.github.io/

use tree_sitter::Language;

extern "C" {
    fn tree_sitter_perl() -> Language;
}

/// Get the tree-sitter [Language][] for this grammar.
///
/// [Language]: https://docs.rs/tree-sitter/*/tree_sitter/struct.Language.html
pub fn language() -> Language {
    unsafe { tree_sitter_perl() }
}

/// The content of the [`node-types.json`][] file for this grammar.
///
/// [`node-types.json`]: https://tree-sitter.github.io/tree-sitter/using-parsers#static-node-types
pub const NODE_TYPES: &'static str = include_str!("../../src/node-types.json");

// Uncomment these to include any queries that this grammar contains

// pub const HIGHLIGHTS_QUERY: &'static str = include_str!("../../queries/highlights.scm");
// pub const INJECTIONS_QUERY: &'static str = include_str!("../../queries/injections.scm");
// pub const LOCALS_QUERY: &'static str = include_str!("../../queries/locals.scm");
// pub const TAGS_QUERY: &'static str = include_str!("../../queries/tags.scm");

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::language())
            .expect("Error loading perl language");
    }
}

// Include comprehensive test suite
#[cfg(test)]
mod test_harness;
#[cfg(test)]
mod simple_test;
#[cfg(test)]
mod integration_corpus;
#[cfg(test)]
mod integration_highlight;
#[cfg(test)]
mod unit_unicode;
#[cfg(test)]
mod unit_scanner;
#[cfg(test)]
mod property_scanner;
#[cfg(test)]
mod performance;

#[cfg(test)]
mod comprehensive_tests {
    // use super::*;
    use crate::test_harness::parse_perl_code;

    #[test]
    fn test_basic_parsing_works() {
        let code = "print 'Hello, World!';";
        let result = parse_perl_code(code);
        assert!(result.is_ok(), "Failed to parse basic Perl code: {:?}", result);
    }

    #[test]
    fn test_empty_string_parses() {
        let code = "";
        let result = parse_perl_code(code);
        assert!(result.is_ok(), "Failed to parse empty string: {:?}", result);
    }

    #[test]
    fn test_variable_declaration_parses() {
        let code = "my $var = 42;";
        let result = parse_perl_code(code);
        assert!(result.is_ok(), "Failed to parse variable declaration: {:?}", result);
    }
}
