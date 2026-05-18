//! Outcome categories for differential parser testing.
//!
//! Each test case records a [`Verdict`] for each parser.  The verdict is not
//! just pass/fail - it classifies *how* the parser handled the input.  This
//! makes the disagreement table the durable artifact: when a parser improves,
//! the expected verdict changes intentionally rather than silently.

use std::fmt;

/// Outcome category for a single parser on a single input.
///
/// Each variant captures a distinct failure mode (or success mode).  The
/// expected verdict for every (case, parser) combination is recorded in the
/// test and must be updated intentionally when parser behaviour changes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Verdict {
    /// Parser produced a structurally correct AST for this input.
    ///
    /// "Correct" means: the key structural property (e.g. heredoc body is
    /// non-empty, format declaration has body lines, `${^MATCH}` is a single
    /// variable, etc.) is satisfied.
    Correct,

    /// Parser accepted the input but the AST is plausibly wrong.
    ///
    /// Example: Pest parses `map { a => $_ }` as a hash-ref rather than a
    /// block - the parse succeeds, but the semantic interpretation is wrong.
    WrongButPlausible,

    /// Parser accepted the input but key content is silently absent.
    ///
    /// Example: Pest accepts `<<A\nbody\nA\n` but the heredoc body is empty.
    SilentlyEmpty,

    /// Parser returned an error for this input.
    ///
    /// Used for inputs that *should* produce an error (garbage inputs) and
    /// for parsers that legitimately reject hard-but-valid Perl.
    Errors,

    /// Parser panicked on this input (caught with `std::panic::catch_unwind`).
    ///
    /// A crash is always a bug in the parser.  The test records it rather than
    /// propagating the panic, so one crash does not kill the whole suite.
    Crashes,
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Correct => write!(f, "Correct"),
            Self::WrongButPlausible => write!(f, "WrongButPlausible"),
            Self::SilentlyEmpty => write!(f, "SilentlyEmpty"),
            Self::Errors => write!(f, "Errors"),
            Self::Crashes => write!(f, "Crashes"),
        }
    }
}
