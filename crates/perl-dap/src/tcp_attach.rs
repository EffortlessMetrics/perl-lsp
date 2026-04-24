//! TCP Attach Module for DAP
//!
//! This module provides TCP-based attachment to running Perl debugger processes.
//! It enables connecting to Perl::LanguageServer DAP instances via TCP sockets.
//!
//! # Architecture
//!
//! ```text
//! VS Code <-> Native DAP Adapter <-> TCP Socket <-> Perl::LanguageServer DAP
//! ```
//!
//! # Features
//!
//! - TCP socket connection with configurable timeout
//! - Bidirectional message proxying
//! - Connection state management
//! - Error recovery and reconnection
//! - Cross-platform support (Windows, macOS, Linux)

use anyhow::{Context, Result};
use perl_lsp_rs_core::transport::framing::{ContentLengthFramer, frame};
use serde_json::Value;
use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Maximum connection timeout in milliseconds (5 minutes)
const MAX_TIMEOUT_MS: u32 = 300_000;

/// Default connection timeout in milliseconds
const DEFAULT_TIMEOUT_MS: u32 = 5000;

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

/// TCP attach session
///
/// Manages a TCP connection to a Perl debugger process.
pub struct TcpAttachSession {
    /// TCP stream to the debugger
    stream: Option<TcpStream>,
    /// Connection state
    connected: Arc<Mutex<bool>>,
    /// Event sender for DAP events
    event_sender: Option<Sender<DapEvent>>,
}

/// DAP event from TCP attach session
#[derive(Debug, Clone)]
pub enum DapEvent {
    /// Output event from debugger
    Output { category: String, output: String },
    /// Stopped event (breakpoint hit, step, etc.)
    Stopped { reason: String, thread_id: i32 },
    /// Continued event (execution resumed)
    Continued { thread_id: i32 },
    /// Terminated event (debugger exited)
    Terminated { reason: String },
    /// Error event
    Error { message: String },
}

impl TcpAttachSession {
    /// Create a new TCP attach session
    pub fn new() -> Self {
        Self { stream: None, connected: Arc::new(Mutex::new(false)), event_sender: None }
    }

    /// Set the event sender
    pub fn set_event_sender(&mut self, sender: Sender<DapEvent>) {
        self.event_sender = Some(sender);
    }

    /// Connect to the debugger via TCP
    pub fn connect(&mut self, config: &TcpAttachConfig) -> Result<()> {
        config.validate()?;
        let _ = self.disconnect();

        let address = format!("{}:{}", config.host, config.port);
        tracing::info!(address, "Connecting to Perl debugger");

        // Connect with timeout
        let stream = TcpStream::connect_timeout(&address.parse()?, config.timeout_duration())
            .context(format!("Failed to connect to {}", address))?;

        // Set read/write timeouts
        let timeout = Some(config.timeout_duration());
        stream.set_read_timeout(timeout)?;
        stream.set_write_timeout(timeout)?;

        self.stream = Some(stream);
        *self.connected.lock().unwrap_or_else(|e| e.into_inner()) = true;

        tracing::info!(address, "Successfully connected to Perl debugger");
        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected.lock().map(|g| *g).unwrap_or(false)
    }

    /// Disconnect from the debugger
    pub fn disconnect(&mut self) -> Result<()> {
        *self.connected.lock().unwrap_or_else(|e| e.into_inner()) = false;

        if let Some(stream) = self.stream.take() {
            stream.shutdown(std::net::Shutdown::Both)?;
            tracing::info!("Disconnected from Perl debugger");
        }
        Ok(())
    }

    /// Send a DAP message to the debugger
    pub fn send_message(&mut self, message: &str) -> Result<()> {
        let stream = self.stream.as_mut().context("Not connected to debugger")?;
        let framed = frame(message.as_bytes());
        stream.write_all(&framed).context("Failed to write to debugger")?;

        stream.flush().context("Failed to flush stream")?;
        Ok(())
    }

    /// Start reading messages from the debugger
    pub fn start_reader(&mut self) -> Result<()> {
        let stream = self.stream.take().context("No stream available")?;

        let connected = Arc::clone(&self.connected);
        let event_sender = self.event_sender.clone();

        thread::spawn(move || {
            let mut reader = BufReader::new(stream);
            let mut framer = ContentLengthFramer::new();
            let mut read_buf = [0u8; 8 * 1024];

            loop {
                let bytes_read = match reader.read(&mut read_buf) {
                    Ok(0) => {
                        tracing::debug!("TCP connection closed by debugger");
                        *connected.lock().unwrap_or_else(|e| e.into_inner()) = false;
                        if let Some(ref sender) = event_sender {
                            let _ = sender.send(DapEvent::Terminated {
                                reason: "connection_closed".to_string(),
                            });
                        }
                        return;
                    }
                    Ok(n) => n,
                    Err(e) => {
                        tracing::error!(error = %e, "Error reading from TCP");
                        *connected.lock().unwrap_or_else(|e| e.into_inner()) = false;
                        if let Some(ref sender) = event_sender {
                            let _ = sender.send(DapEvent::Error {
                                message: format!("TCP read error: {}", e),
                            });
                        }
                        return;
                    }
                };

                framer.push(&read_buf[..bytes_read]);

                loop {
                    let buffer = match framer.try_next() {
                        Ok(Some(buffer)) => buffer,
                        Ok(None) => break,
                        Err(error) => {
                            tracing::warn!(%error, "Failed to parse TCP DAP frame");
                            continue;
                        }
                    };

                    if let Ok(text) = std::str::from_utf8(&buffer) {
                        tracing::trace!(output = %text, "Received from debugger");
                    } else {
                        tracing::warn!(
                            bytes = buffer.len(),
                            "Received non-UTF8 message from debugger"
                        );
                    }

                    // Parse DAP message and emit event
                    if let Some(ref sender) = event_sender
                        && let Ok(value) = serde_json::from_slice::<Value>(&buffer)
                        && let Some(event_type) = value.get("type").and_then(|t| t.as_str())
                        && event_type == "event"
                    {
                        let event_name =
                            value.get("event").and_then(|e| e.as_str()).unwrap_or("unknown");

                        match event_name {
                            "output" => {
                                let body = value.get("body");
                                let category = body
                                    .and_then(|b| b.get("category"))
                                    .and_then(|c| c.as_str())
                                    .unwrap_or("stdout")
                                    .to_string();
                                let output = body
                                    .and_then(|b| b.get("output"))
                                    .and_then(|o| o.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                let _ = sender.send(DapEvent::Output { category, output });
                            }
                            "stopped" => {
                                let body = value.get("body");
                                let reason = body
                                    .and_then(|b| b.get("reason"))
                                    .and_then(|r| r.as_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                let thread_id =
                                    body.and_then(|b| b.get("threadId"))
                                        .and_then(|t| t.as_i64())
                                        .unwrap_or(1) as i32;

                                let _ = sender.send(DapEvent::Stopped { reason, thread_id });
                            }
                            "continued" => {
                                let body = value.get("body");
                                let thread_id =
                                    body.and_then(|b| b.get("threadId"))
                                        .and_then(|t| t.as_i64())
                                        .unwrap_or(1) as i32;

                                let _ = sender.send(DapEvent::Continued { thread_id });
                            }
                            "terminated" => {
                                let reason = value
                                    .get("body")
                                    .and_then(|b| b.get("reason"))
                                    .and_then(|r| r.as_str())
                                    .unwrap_or("unknown")
                                    .to_string();

                                let _ = sender.send(DapEvent::Terminated { reason });
                            }
                            _ => {
                                tracing::debug!(event = %event_name, "Unhandled DAP event");
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }
}

impl Default for TcpAttachSession {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TcpAttachSession {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_attach_config_validation() {
        // Valid config
        let config = TcpAttachConfig::new("localhost".to_string(), 13603);
        assert!(config.validate().is_ok());

        // Empty host
        let config = TcpAttachConfig::new("".to_string(), 13603);
        assert!(config.validate().is_err());

        // Invalid port
        let config = TcpAttachConfig::new("localhost".to_string(), 0);
        assert!(config.validate().is_err());

        // Valid timeout
        let config = TcpAttachConfig::new("localhost".to_string(), 13603).with_timeout(5000);
        assert!(config.validate().is_ok());

        // Zero timeout
        let config = TcpAttachConfig::new("localhost".to_string(), 13603).with_timeout(0);
        assert!(config.validate().is_err());

        // Timeout too large
        let config =
            TcpAttachConfig::new("localhost".to_string(), 13603).with_timeout(MAX_TIMEOUT_MS + 1);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_tcp_attach_session_creation() {
        let session = TcpAttachSession::new();
        assert!(!session.is_connected());
    }

    #[test]
    fn test_tcp_attach_timeout_duration() {
        let config = TcpAttachConfig::new("localhost".to_string(), 13603);
        assert_eq!(config.timeout_duration(), Duration::from_millis(DEFAULT_TIMEOUT_MS as u64));

        let config = TcpAttachConfig::new("localhost".to_string(), 13603).with_timeout(10000);
        assert_eq!(config.timeout_duration(), Duration::from_millis(10000));
    }
}
