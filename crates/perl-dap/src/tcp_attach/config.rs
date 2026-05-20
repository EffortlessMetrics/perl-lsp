use anyhow::Result;
use std::time::Duration;

/// Maximum connection timeout in milliseconds (5 minutes)
pub(crate) const MAX_TIMEOUT_MS: u32 = 300_000;

/// Default connection timeout in milliseconds
pub(crate) const DEFAULT_TIMEOUT_MS: u32 = 5000;

/// TCP attach configuration
#[derive(Debug, Clone)]
pub struct TcpAttachConfig {
    /// Hostname or IP address to connect to
    pub host: String,
    /// Port number to connect to
    pub port: u16,
    /// Connection timeout in milliseconds
    pub timeout_ms: Option<u32>,
}

impl TcpAttachConfig {
    /// Create a new TCP attach configuration
    pub fn new(host: String, port: u16) -> Self {
        Self { host, port, timeout_ms: None }
    }

    /// Set the connection timeout
    pub fn with_timeout(mut self, timeout_ms: u32) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.host.trim().is_empty() {
            anyhow::bail!("Host cannot be empty");
        }
        if self.port == 0 {
            anyhow::bail!("Port must be in range 1-65535");
        }
        if let Some(timeout) = self.timeout_ms {
            if timeout == 0 {
                anyhow::bail!("Timeout must be greater than 0 milliseconds");
            }
            if timeout > MAX_TIMEOUT_MS {
                anyhow::bail!("Timeout cannot exceed {} milliseconds (5 minutes)", MAX_TIMEOUT_MS);
            }
        }
        Ok(())
    }

    /// Get the connection timeout duration
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_millis(self.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_required_host_port_and_timeout_bounds() {
        let config = TcpAttachConfig::new("localhost".to_string(), 13603);
        assert!(config.validate().is_ok());

        let config = TcpAttachConfig::new("".to_string(), 13603);
        assert!(config.validate().is_err());

        let config = TcpAttachConfig::new("localhost".to_string(), 0);
        assert!(config.validate().is_err());

        let config = TcpAttachConfig::new("localhost".to_string(), 13603).with_timeout(5000);
        assert!(config.validate().is_ok());

        let config = TcpAttachConfig::new("localhost".to_string(), 13603).with_timeout(0);
        assert!(config.validate().is_err());

        let config =
            TcpAttachConfig::new("localhost".to_string(), 13603).with_timeout(MAX_TIMEOUT_MS + 1);
        assert!(config.validate().is_err());
    }

    #[test]
    fn timeout_duration_uses_default_or_configured_value() {
        let config = TcpAttachConfig::new("localhost".to_string(), 13603);
        assert_eq!(config.timeout_duration(), Duration::from_millis(DEFAULT_TIMEOUT_MS as u64));

        let config = TcpAttachConfig::new("localhost".to_string(), 13603).with_timeout(10000);
        assert_eq!(config.timeout_duration(), Duration::from_millis(10000));
    }
}
