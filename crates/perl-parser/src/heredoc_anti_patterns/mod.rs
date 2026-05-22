//! Anti-pattern detection for heredoc edge cases.
//!
//! This module provides detection and analysis of problematic Perl patterns
//! that make static parsing difficult or impossible, particularly around heredocs.
//!
//! The [`crate::heredoc_anti_patterns::AntiPatternDetector`] scans Perl source
//! for seven categories of heredoc-related anti-patterns and produces
//! [`crate::heredoc_anti_patterns::Diagnostic`]s describing each finding, with
//! severity, explanation, suggested fix, and documentation references.

mod detectors;
mod model;
mod utils;

pub use detectors::AntiPatternDetector;
pub use model::{AntiPattern, Diagnostic, Location, Severity};
