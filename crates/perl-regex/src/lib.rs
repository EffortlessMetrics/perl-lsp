//! Perl regex validation and analysis
//!
//! This module provides tools to validate Perl regular expressions
//! and detect potential security or performance issues like catastrophic backtracking.

mod analyzer;
mod validator;

pub use analyzer::{CaptureGroup, RegexAnalyzer};
pub use validator::{RegexError, RegexValidator};
