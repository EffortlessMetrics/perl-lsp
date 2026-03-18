//! Recovery-oriented parser building blocks for Perl source code.
//!
//! This crate packages the error-tolerant parser context and the experimental
//! recovery parser so higher-level crates can depend on recovery support without
//! pulling in the full `perl-parser-core` implementation details.

#![deny(unsafe_code)]
#![deny(unreachable_pub)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(
    clippy::module_name_repetitions,
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::wildcard_imports,
    clippy::enum_glob_use,
    clippy::collapsible_if,
    clippy::match_same_arms,
    clippy::if_not_else,
    clippy::must_use_candidate,
    clippy::single_match_else,
    clippy::items_after_statements,
    clippy::struct_excessive_bools,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::return_self_not_must_use,
    clippy::unused_self,
    clippy::while_let_loop,
    clippy::needless_raw_string_hashes,
    clippy::uninlined_format_args
)]

mod context_impls;
/// Parser context with error tracking, positions, and parse-budget accounting.
pub mod parser_context;
/// Recovery-aware parser implementation for IDE-style partial parsing.
pub mod recovery_parser;

pub use parser_context::ParserContext;
pub use recovery_parser::RecoveryParser;
