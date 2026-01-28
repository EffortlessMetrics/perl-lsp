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
/// Recovery-oriented parser utilities for error-tolerant parsing.
pub mod recovery_parser;

/// Error types and result aliases used by the parser engine.
pub use perl_error::*;
/// Parse recovery wrapper used in error-tolerant workflows.
pub use recovery_parser::RecoveryParser;
