mod code_execution;
mod complexity;
mod config;
mod group;
mod nested_quantifier;

pub use config::RegexValidationConfig;

use crate::error::RegexError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexFinding {
    pub offset: usize,
    pub message: &'static str,
}

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
        if let Some(finding) = self.find_code_execution(pattern, start_pos) {
            return Err(RegexError::syntax(finding.message, finding.offset));
        }
        if let Some(finding) = self.find_nested_quantifier(pattern, start_pos) {
            return Err(RegexError::syntax(finding.message, finding.offset));
        }
        complexity::check_complexity(pattern, start_pos, &self.config)
    }

    pub fn find_code_execution(&self, pattern: &str, start_pos: usize) -> Option<RegexFinding> {
        code_execution::find_code_execution(pattern).map(|offset| RegexFinding {
            offset: start_pos + offset,
            message: "Embedded code execution construct is not allowed",
        })
    }

    pub fn find_nested_quantifier(&self, pattern: &str, start_pos: usize) -> Option<RegexFinding> {
        nested_quantifier::find_nested_quantifier(pattern).map(|offset| RegexFinding {
            offset: start_pos + offset,
            message: "Nested quantifiers can cause catastrophic backtracking",
        })
    }

    pub fn detects_code_execution(&self, pattern: &str) -> bool {
        code_execution::detects_code_execution(pattern)
    }

    pub fn detect_nested_quantifiers(&self, pattern: &str) -> bool {
        nested_quantifier::detect_nested_quantifiers(pattern)
    }
}
