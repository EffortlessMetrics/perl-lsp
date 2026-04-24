//! Perl::Critic integration for code quality analysis.
//!
//! Provides integration with Perl::Critic for static code analysis
//! and policy enforcement in Perl code.

mod analyzer;
mod built_in;
mod quick_fix;
mod types;

pub use analyzer::CriticAnalyzer;
pub use built_in::{BuiltInAnalyzer, Policy};
pub use quick_fix::{QuickFix, TextEdit};
pub use types::{CriticConfig, Severity, Violation};

#[cfg(not(feature = "lsp-compat"))]
pub use types::ViolationSummary;

pub(crate) use quick_fix::built_in_quick_fix;
#[cfg(feature = "lsp-compat")]
pub(crate) use quick_fix::perlcritic_quick_fix;
pub(crate) use types::insertion_range;
