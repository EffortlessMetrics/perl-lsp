use crate::security::SecurityConfig;

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

        tracing::warn!("Security violation #{} recorded: {}", count + 1, violation_type);
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
            last.elapsed() < std::time::Duration::from_mins(1)
        } else {
            false
        }
    }
}
