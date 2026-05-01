mod code_execution;
mod complexity;
mod config;
mod group;
mod nested_quantifier;
mod scanner;
mod unicode_property;

pub use config::RegexValidationConfig;

use crate::RegexError;

/// Validator for Perl regular expressions to prevent security and performance issues
pub struct RegexValidator {
    config: RegexValidationConfig,
}

impl Default for RegexValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl RegexValidator {
    pub fn new() -> Self {
        Self { config: RegexValidationConfig::default() }
    }
    pub fn with_config(config: RegexValidationConfig) -> Self {
        Self { config }
    }
    pub fn config(&self) -> &RegexValidationConfig {
        &self.config
    }
    pub fn validate(&self, pattern: &str, start_pos: usize) -> Result<(), RegexError> {
        complexity::check_complexity(pattern, start_pos, &self.config)
    }
    pub fn detects_code_execution(&self, pattern: &str) -> bool {
        code_execution::detects_code_execution(pattern)
    }
    pub fn detect_nested_quantifiers(&self, pattern: &str) -> bool {
        nested_quantifier::detect_nested_quantifiers(pattern)
    }
}
