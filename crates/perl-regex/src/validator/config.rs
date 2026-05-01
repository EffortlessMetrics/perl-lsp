#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexValidationConfig {
    pub max_nesting: usize,
    pub max_unicode_properties: usize,
    pub max_branch_reset_branches: usize,
}

impl Default for RegexValidationConfig {
    fn default() -> Self {
        Self { max_nesting: 10, max_unicode_properties: 50, max_branch_reset_branches: 50 }
    }
}
