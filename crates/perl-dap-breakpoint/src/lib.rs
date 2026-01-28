//! Breakpoint Validation for Perl DAP
//!
//! This crate provides AST-based breakpoint validation for the Perl Debug Adapter Protocol.
//! It validates breakpoint locations against parsed source code to ensure breakpoints are
//! set on executable lines, not on comments, blank lines, or inside heredoc content.
//!
//! # Features
//!
//! - **AST-Based Validation**: Uses the Perl parser AST to validate breakpoint locations
//! - **Line Suggestion**: Suggests the nearest valid line when a breakpoint is on an invalid location
//! - **Validation Reasons**: Provides detailed reasons for why a breakpoint was rejected or adjusted
//!
//! # Example
//!
//! ```rust,ignore
//! use perl_dap_breakpoint::{BreakpointValidator, AstBreakpointValidator};
//!
//! let source = "# comment\nmy $x = 1;\n";
//! let validator = AstBreakpointValidator::new(source)?;
//!
//! let result = validator.validate(1); // Line 1 is a comment
//! assert!(!result.verified);
//! assert_eq!(result.reason, Some(ValidationReason::CommentLine));
//! ```

mod suggestion;
mod validator;

pub use suggestion::find_nearest_valid_line;
pub use validator::{
    AstBreakpointValidator, BreakpointValidation, BreakpointValidator, ValidationReason,
};

/// Error type for breakpoint validation operations
#[derive(Debug, thiserror::Error)]
pub enum BreakpointError {
    /// Failed to parse the source file
    #[error("Failed to parse source: {0}")]
    ParseError(String),

    /// Line number is out of range
    #[error("Line {0} is out of range (file has {1} lines)")]
    LineOutOfRange(i64, usize),
}
