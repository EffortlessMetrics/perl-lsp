//! Generators for syntactically conservative Perl snippets.
//!
//! These strategies intentionally produce a small, valid subset of Perl so
//! parser property tests can spend cases on AST invariants instead of filtering
//! out invalid random text.

mod chars;
mod expressions;
mod identifiers;
mod literals;
mod statements;

pub use expressions::simple_expression;
pub use identifiers::{perl_identifier, scalar_variable};
pub use literals::{integer_literal, single_quoted_string_literal};
pub use statements::{simple_program, simple_statement};

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn identifiers_are_not_empty(identifier in perl_identifier()) {
            prop_assert!(!identifier.is_empty(), "identifier must not be empty");
        }

        #[test]
        fn scalar_variables_start_with_dollar(variable in scalar_variable()) {
            prop_assert!(variable.starts_with('$'), "scalar variable must start with '$': {variable}");
            prop_assert!(variable.len() > 1, "scalar variable must include a name: {variable}");
        }

        #[test]
        fn single_quoted_literals_are_balanced(literal in single_quoted_string_literal()) {
            prop_assert!(literal.starts_with('\''), "literal must start with quote: {literal}");
            prop_assert!(literal.ends_with('\''), "literal must end with quote: {literal}");
        }

        #[test]
        fn simple_programs_do_not_contain_nul(program in simple_program()) {
            prop_assert!(!program.contains('\0'), "generated program contained NUL");
        }
    }
}
