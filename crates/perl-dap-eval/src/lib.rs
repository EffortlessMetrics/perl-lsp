//! Safe Expression Evaluation Validation for Perl DAP
//!
//! This crate provides security validation for expressions evaluated during Perl debugging.
//! It detects and blocks potentially dangerous operations that could mutate state,
//! execute arbitrary code, or perform I/O operations during debug evaluation.
//!
//! # Features
//!
//! - **Operation Detection**: Detects dangerous Perl operations (eval, system, exec, etc.)
//! - **Mutation Detection**: Detects assignment and increment/decrement operators
//! - **Shell Execution Detection**: Blocks backticks and qx() for shell execution
//! - **Context-Aware**: Avoids false positives for sigil-prefixed identifiers ($print, @say)
//!
//! # Example
//!
//! ```rust
//! use perl_dap_eval::{SafeEvaluator, ValidationResult};
//!
//! let evaluator = SafeEvaluator::new();
//!
//! // Safe expressions pass validation
//! assert!(evaluator.validate("$x + $y").is_ok());
//!
//! // Dangerous operations are blocked
//! let result = evaluator.validate("system('rm -rf /')");
//! assert!(result.is_err());
//! ```
//!
//! # Security Model
//!
//! The safe evaluator blocks the following categories of operations:
//!
//! - **Code Execution**: eval, require, do (file)
//! - **Process Control**: system, exec, fork, exit, kill, etc.
//! - **I/O Operations**: print, say, open, close, etc.
//! - **Filesystem**: mkdir, rmdir, unlink, chmod, etc.
//! - **Network**: socket, connect, bind, etc.
//! - **Tie Mechanism**: tie/untie (can execute arbitrary code)
//! - **Mutation**: Assignment operators, ++/--, regex mutation (s///)

mod patterns;
mod validator;

pub use validator::{SafeEvaluator, ValidationError, ValidationResult};

// Re-export pattern constants for testing/extension
pub use patterns::DANGEROUS_OPERATIONS;
