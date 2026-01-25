//! Lint checks for Perl code analysis
//!
//! This module provides various linting checks for detecting deprecated syntax,
//! strict/warnings issues, and common mistakes in Perl code.
//!
//! # Architecture
//!
//! Lints are organized into focused submodules:
//!
//! - **deprecated**: Deprecated syntax warnings (e.g., `defined(@array)`)
//! - **strict_warnings**: Missing `use strict` and `use warnings` advisories
//! - **common_mistakes**: Frequent programming errors (assignment in conditions, etc.)
//!
//! # Severity Levels
//!
//! Each lint produces diagnostics with appropriate severity:
//!
//! - **Error**: Issues that will cause runtime failures
//! - **Warning**: Potential bugs or deprecated patterns
//! - **Information**: Best practice suggestions
//! - **Hint**: Style recommendations
//!
//! # Integration
//!
//! Lints integrate with the diagnostics pipeline and provide:
//!
//! - Diagnostic codes for IDE quick-fix integration
//! - Related information with suggestions and explanations
//! - Diagnostic tags (Deprecated, Unnecessary) for IDE rendering

pub mod common_mistakes;
pub mod deprecated;
pub mod strict_warnings;
