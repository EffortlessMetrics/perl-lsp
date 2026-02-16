//! Security module for production hardening
//!
//! This module provides comprehensive security features including:
//! - Input validation and sanitization
//! - Path traversal prevention
//! - Process isolation and sandboxing
//! - Security monitoring and logging

pub mod validation;
pub mod sandbox;

pub use validation::{
    validate_file_path,
    validate_file_content,
    validate_lsp_request,
    sanitize_string,
    validate_workspace_root,
};

pub use sandbox::{
    SandboxConfig,
    Sandbox,
    SandboxResult,
    SafeExecutor,
};

/// Security configuration for the LSP server
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Maximum file size for parsing (bytes)
    pub max_file_size: usize,
    /// Maximum path length
    pub max_path_length: usize,
    /// Allowed file extensions
    pub allowed_extensions: Vec<String>,
    /// Whether to enable strict validation
    pub strict_mode: bool,
    /// Maximum LSP parameter size
    pub max_parameter_size: usize,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_path_length: 4096,
            allowed_extensions: vec!["pl".to_string(), "pm".to_string(), "t".to_string(), "pod".to_string()],
            strict_mode: true,
            max_parameter_size: 1_000_000,
        }
    }
}

/// Security context for tracking and monitoring
#[derive(Debug)]
pub struct SecurityContext {
    config: SecurityConfig,
    /// Count of security violations
    violation_count: std::sync::atomic::AtomicUsize,
    /// Last violation timestamp
    last_violation: std::sync::Mutex<Option<std::time::Instant>>,
}

impl SecurityContext {
    /// Create a new security context
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            config,
            violation_count: std::sync::atomic::AtomicUsize::new(0),
            last_violation: std::sync::Mutex::new(None),
        }
    }
    
    /// Get the security configuration
    pub fn config(&self) -> &SecurityConfig {
        &self.config
    }
    
    /// Record a security violation
    pub fn record_violation(&self, violation_type: &str) {
        let count = self.violation_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut last) = self.last_violation.lock() {
            *last = Some(std::time::Instant::now());
        }
        
        log::warn!(
            "Security violation #{} recorded: {}",
            count + 1,
            violation_type
        );
    }
    
    /// Get the number of violations
    pub fn violation_count(&self) -> usize {
        self.violation_count.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    /// Check if we're in a high-violation state (possible attack)
    pub fn is_high_violation_state(&self) -> bool {
        let count = self.violation_count();
        if count < 10 {
            return false;
        }
        
        if let Ok(last_guard) = self.last_violation.lock()
            && let Some(last) = *last_guard
        {
            // If we've had 10+ violations in the last minute
            last.elapsed() < std::time::Duration::from_secs(60)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
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