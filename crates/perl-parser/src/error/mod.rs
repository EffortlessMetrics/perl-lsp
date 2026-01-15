//! Error module compatibility re-exports.
//!
//! New code should use `perl_parser::engine::error`.

/// Re-export parse error types and recovery helpers from `engine::error`.
pub use crate::engine::error::*;
