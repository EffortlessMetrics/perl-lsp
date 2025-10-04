//! Bridge adapter for Perl::LanguageServer DAP
//!
//! This module provides a bridge between VS Code's DAP client and Perl::LanguageServer's
//! DAP implementation. It proxies messages via stdio, enabling immediate debugging capability
//! while the native Rust adapter is developed.
//!
//! # Architecture
//!
//! ```text
//! VS Code ↔ BridgeAdapter (Rust) ↔ Perl::LanguageServer (Perl)
//!          (stdio)                  (stdio)
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use perl_dap::BridgeAdapter;
//!
//! # fn main() -> anyhow::Result<()> {
//! let mut adapter = BridgeAdapter::new();
//! adapter.spawn_pls_dap()?;
//! adapter.proxy_messages()?;
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use std::process::{Child, Command, Stdio};

/// Bridge adapter that proxies DAP messages to Perl::LanguageServer
///
/// This adapter spawns Perl::LanguageServer in DAP mode and forwards
/// all DAP protocol messages bidirectionally via stdio.
pub struct BridgeAdapter {
    /// The spawned Perl::LanguageServer process
    child_process: Option<Child>,
}

impl BridgeAdapter {
    /// Create a new bridge adapter
    ///
    /// # Examples
    ///
    /// ```
    /// use perl_dap::BridgeAdapter;
    ///
    /// let adapter = BridgeAdapter::new();
    /// ```
    pub fn new() -> Self {
        Self { child_process: None }
    }

    /// Spawn Perl::LanguageServer in DAP mode
    ///
    /// This method starts the Perl::LanguageServer process with DAP protocol support.
    /// It uses the platform-specific perl binary resolution.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Perl binary cannot be found on PATH
    /// - Perl::LanguageServer module is not installed
    /// - Process spawn fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use perl_dap::BridgeAdapter;
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let mut adapter = BridgeAdapter::new();
    /// adapter.spawn_pls_dap()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn spawn_pls_dap(&mut self) -> Result<()> {
        // Find perl binary using platform module
        let perl_path =
            crate::platform::resolve_perl_path().context("Failed to find perl binary on PATH")?;

        // Spawn Perl::LanguageServer in DAP mode
        // The -d:LanguageServer::DAP flag activates DAP protocol mode
        let child = Command::new(perl_path)
            .arg("-d:LanguageServer::DAP")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to spawn Perl::LanguageServer DAP process")?;

        self.child_process = Some(child);
        Ok(())
    }

    /// Proxy messages between VS Code and Perl::LanguageServer
    ///
    /// This method forwards stdin/stdout bidirectionally between the DAP client
    /// and the Perl::LanguageServer process.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Child process not spawned (call `spawn_pls_dap()` first)
    /// - I/O error during message proxying
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use perl_dap::BridgeAdapter;
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let mut adapter = BridgeAdapter::new();
    /// adapter.spawn_pls_dap()?;
    /// adapter.proxy_messages()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn proxy_messages(&mut self) -> Result<()> {
        // Verify child process is running
        if self.child_process.is_none() {
            anyhow::bail!("Child process not spawned. Call spawn_pls_dap() first.");
        }

        // TODO: Implement bidirectional message proxying
        // This is a placeholder for Phase 1 - actual proxying will be implemented
        // when needed for the full workflow tests
        Ok(())
    }
}

impl Default for BridgeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BridgeAdapter {
    fn drop(&mut self) {
        // Clean up child process on drop
        if let Some(mut child) = self.child_process.take() {
            let _ = child.kill();
        }
    }
}
