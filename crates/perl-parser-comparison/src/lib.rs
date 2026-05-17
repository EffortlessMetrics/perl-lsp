//! Differential parser test harness for v1/v2/v3 Perl parsers.
//!
//! This crate provides a structured harness for measuring how each of the three
//! perl-lsp parsers handles constructs that historically defeated tree-sitter
//! (documented in `docs/articles/research/TREE_SITTER_BREAKAGE.md`).
//!
//! # Design
//!
//! Each test case records a [`Verdict`] for each parser — not a pass/fail bit,
//! but a *category* of outcome: `Correct`, `WrongButPlausible`, `SilentlyEmpty`,
//! `Errors`, or `Crashes`.  The suite asserts that each parser produces its
//! *expected* verdict.  When a parser improves (or regresses) the expected
//! verdict must be updated intentionally, making the disagreement table the
//! durable artifact.
//!
//! # Parsers
//!
//! | Label | Crate | Description |
//! |-------|-------|-------------|
//! | v1 | `tree-sitter-perl-c` | C tree-sitter FFI binding |
//! | v2 | `perl-parser-pest` | Pest/PEG legacy parser |
//! | v3 | `perl-parser-core` | Recursive-descent production parser |
//!
//! # Usage
//!
//! ```no_run
//! use perl_parser_comparison::{parse_v1, parse_v2, parse_v3, Verdict};
//!
//! let src = "my $x = 42;";
//! assert_eq!(parse_v1(src).verdict, Verdict::Correct);
//! assert_eq!(parse_v2(src).verdict, Verdict::Correct);
//! assert_eq!(parse_v3(src).verdict, Verdict::Correct);
//! ```

#![deny(unreachable_pub)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![allow(
    clippy::too_many_lines,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]
// Tests in this crate may use panic-based helpers (must/must_some).
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]

pub mod corpus_walker;
pub mod harness;
pub mod outcomes;

pub use corpus_walker::{
    AggregateStats, DisagreementKind, FileRecord, classify, format_report, walk_corpora,
};
pub use harness::{ParseResult, parse_v1, parse_v2, parse_v3};
pub use outcomes::Verdict;
