use super::config::TcpAttachConfig;
use super::event::DapEvent;
use super::reader::spawn_reader;
use anyhow::{Context, Result};
use perl_lsp_rs_core::transport::framing::frame;
use std::io::Write;
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

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

        let address = format!("{}:{}", config.host, config.port);
        tracing::info!(address, "Connecting to Perl debugger");

        let stream = TcpStream::connect_timeout(&address.parse()?, config.timeout_duration())
            .context(format!("Failed to connect to {}", address))?;

        let timeout = Some(config.timeout_duration());
        stream.set_read_timeout(timeout)?;
        stream.set_write_timeout(timeout)?;

        self.stream = Some(stream);
        self.set_connected(true);

        tracing::info!(address, "Successfully connected to Perl debugger");
        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected.lock().map(|guard| *guard).unwrap_or(false)
    }

    /// Disconnect from the debugger
    pub fn disconnect(&mut self) -> Result<()> {
        if let Some(stream) = self.stream.take() {
            stream.shutdown(std::net::Shutdown::Both)?;
            tracing::info!("Disconnected from Perl debugger");
        }
        self.set_connected(false);
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
        let stream = self
            .stream
            .as_ref()
            .context("No stream available")?
            .try_clone()
            .context("Failed to clone TCP stream for reader thread")?;

        spawn_reader(stream, Arc::clone(&self.connected), self.event_sender.clone());
        Ok(())
    }

    fn set_connected(&self, connected: bool) {
        *self.connected.lock().unwrap_or_else(|error| error.into_inner()) = connected;
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
    fn new_session_starts_disconnected() {
        let session = TcpAttachSession::new();
        assert!(!session.is_connected());
    }
}
