//! Error types and recovery helpers for the parser engine.
//!
//! Defines error classifications and recovery workflows used during the Parse and
//! Analyze stages of the LSP pipeline. These types are surfaced in diagnostics,
//! syntax checking, and recovery-oriented parsing APIs.
//!
//! # Examples
//!
//! ```rust
//! use perl_parser_core::error::ParseError;
//!
//! let err = ParseError::UnexpectedEof;
//! println!("Parse error: {}", err);
//! ```

/// Implementation of ErrorRecovery trait for ParserContext.
pub mod context_impls;

/// Error classification and diagnostic generation.
pub mod classifier {
    pub use crate::syntax::error::classifier::*;
}
/// Error recovery strategies and traits for the Perl parser.
pub mod recovery {
    pub use crate::syntax::error::recovery::*;
}

/// Error types and result aliases used by the parser engine.
pub use crate::syntax::error::{
    BudgetTracker, ErrorContext, ParseBudget, ParseError, ParseOutput, ParseResult, RecoveryKind,
    RecoverySite, get_error_contexts,
};
