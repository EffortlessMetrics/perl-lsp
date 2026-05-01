//! Perl regex validation and analysis

mod analyzer;
mod error;
mod validator;

pub use analyzer::{CaptureGroup, RegexAnalyzer};
pub use error::RegexError;
pub use validator::RegexValidator;

#[cfg(test)]
mod tests;
