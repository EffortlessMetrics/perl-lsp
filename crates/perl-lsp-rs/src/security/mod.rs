//! Security module for production hardening
//!
//! This module provides comprehensive security features including:
//! - Input validation and sanitization
//! - Path traversal prevention
//! - Process isolation and sandboxing
//! - Security monitoring and logging

mod config;
mod context;
pub mod sandbox;
pub mod validation;

pub use config::SecurityConfig;
pub use context::SecurityContext;
pub use sandbox::{SafeExecutor, Sandbox, SandboxConfig, SandboxResult};
pub use validation::{
    sanitize_string, validate_file_content, validate_file_path, validate_lsp_request,
    validate_workspace_root,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        // max_file_size must match the single source of truth in perl-lsp-limits
        assert_eq!(config.max_file_size, perl_lsp_rs_core::runtime::limits::max_file_size_bytes());
        assert_eq!(config.max_file_size, 1_024 * 1_024, "default must be 1MB from LspLimits");
        assert_eq!(config.max_path_length, 4096);
        assert!(config.strict_mode);
        assert_eq!(config.allowed_extensions.len(), 4);
    }

    #[test]
    fn test_security_context_violation_tracking() {
        let config = SecurityConfig::default();
        let context = SecurityContext::new(config);

        assert_eq!(context.violation_count(), 0);
        assert!(!context.is_high_violation_state());

        context.record_violation("test");
        assert_eq!(context.violation_count(), 1);
        assert!(!context.is_high_violation_state());
    }
}
