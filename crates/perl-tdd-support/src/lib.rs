//! Test-driven development helpers and generators for Perl.

#![deny(unsafe_code)]
#![deny(unreachable_pub)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(
    clippy::too_many_lines,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::wildcard_imports,
    clippy::enum_glob_use,
    clippy::match_same_arms,
    clippy::if_not_else,
    clippy::struct_excessive_bools,
    clippy::items_after_statements,
    clippy::return_self_not_must_use,
    clippy::unused_self,
    clippy::collapsible_match,
    clippy::collapsible_if,
    clippy::only_used_in_recursion,
    clippy::items_after_test_module,
    clippy::while_let_loop,
    clippy::single_range_in_vec_init,
    clippy::arc_with_non_send_sync,
    clippy::needless_range_loop,
    clippy::result_large_err,
    clippy::if_same_then_else,
    clippy::should_implement_trait,
    clippy::manual_flatten,
    clippy::needless_raw_string_hashes,
    clippy::single_char_pattern,
    clippy::uninlined_format_args
)]

pub use perl_parser_core::{Parser, ast, position};
pub use perl_parser_core::{Node, NodeKind, SourceLocation};
pub use perl_parser_core::{ParseError, ParseResult, error, parser};

/// Test-driven development helpers and generators.
pub mod tdd;

#[allow(missing_docs)]
pub use tdd::tdd_basic;
pub use tdd::tdd_workflow;
pub use tdd::test_generator;
/// Test execution and TDD support functionality.
pub use tdd::test_runner;
