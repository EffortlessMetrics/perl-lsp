//! Perl regex validation and analysis.

pub mod analyzer;
pub mod error;
pub mod prelude;
pub mod validator;

mod syntax;

pub use analyzer::{CaptureGroup, RegexAnalyzer};
pub use error::RegexError;
pub use validator::RegexValidator;
