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
//! # #[tokio::main]
//! # async fn main() -> anyhow::Result<()> {
//! let mut adapter = BridgeAdapter::new();
//! adapter.spawn_pls_dap().await?;
//! adapter.proxy_messages().await?;
//! adapter.shutdown().await?;
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
#[cfg(unix)]
use nix::sys::signal::{self, Signal};
#[cfg(unix)]
use nix::unistd::Pid;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::time::sleep;

const PLS_SHUTDOWN_GRACE_MS: u64 = 500;
const PLS_SHUTDOWN_POLL_MS: u64 = 25;

/// Perl debugger flag to activate DAP protocol mode in Perl::LanguageServer
const PLS_DAP_FLAG: &str = "-d:LanguageServer::DAP";

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

    /// Create a bridge adapter around an already spawned process.
    ///
    /// This constructor is mainly intended for deterministic lifecycle testing.
    #[doc(hidden)]
    pub fn from_spawned_child(child: Child) -> Self {
        Self { child_process: Some(child) }
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
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let mut adapter = BridgeAdapter::new();
    /// adapter.spawn_pls_dap().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn spawn_pls_dap(&mut self) -> Result<()> {
        // Ensure any existing process is cleaned up
        if self.child_process.is_some() {
            let _ = self.shutdown().await;
        }

        // Find perl binary using platform module
        let perl_path =
            crate::platform::resolve_perl_path().context("Failed to find perl binary on PATH")?;

        // Spawn Perl::LanguageServer in DAP mode
        let child = Command::new(perl_path)
            .arg(PLS_DAP_FLAG)
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
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let mut adapter = BridgeAdapter::new();
    /// adapter.spawn_pls_dap().await?;
    /// adapter.proxy_messages().await?;
    /// adapter.shutdown().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn proxy_messages(&mut self) -> Result<()> {
        // Verify child process is running
        let Some(child) = self.child_process.as_mut() else {
            anyhow::bail!("Child process not spawned. Call spawn_pls_dap() first.");
        };

        // Get handles to child stdin/stdout
        let mut child_stdin = child.stdin.take().context("Failed to capture child stdin")?;
        let mut child_stdout = child.stdout.take().context("Failed to capture child stdout")?;

        // Get handles to current process stdin/stdout
        let mut parent_stdin = tokio::io::stdin();
        let mut parent_stdout = tokio::io::stdout();

        // DAP uses Content-Length framing, so we can safely proxy raw bytes
        // without message-level inspection. The protocol is self-framing.
        // The proxying strategy uses bidirectional tokio::io::copy for maximum efficiency.

        // Create bidirectional copy tasks
        // Task 1: Client (Parent Stdin) -> Server (Child Stdin)
        let mut client_to_server = tokio::spawn(async move {
            tokio::io::copy(&mut parent_stdin, &mut child_stdin)
                .await
                .context("Error copying from client to server")?;
            // Shut down child_stdin to signal EOF to the server
            let _ = child_stdin.shutdown().await;
            Ok::<(), anyhow::Error>(())
        });

        // Task 2: Server (Child Stdout) -> Client (Parent Stdout)
        let mut server_to_client = tokio::spawn(async move {
            tokio::io::copy(&mut child_stdout, &mut parent_stdout)
                .await
                .context("Error copying from server to client")?;
            parent_stdout.flush().await.context("Error flushing to client")?;
            Ok::<(), anyhow::Error>(())
        });

        // Avoid hanging forever when the child exits but client stdin remains open.
        // If the child exits first, we abort the client->server copy task.
        tokio::select! {
            status = child.wait() => {
                status.context("Failed waiting for Perl::LanguageServer process")?;
                client_to_server.abort();
                let _ = client_to_server.await;
                match server_to_client.await {
                    Ok(result) => result?,
                    Err(join_err) => anyhow::bail!("Server-to-client proxy task failed: {join_err}"),
                }
            }
            res2 = &mut server_to_client => {
                match res2 {
                    Ok(result) => result?,
                    Err(join_err) => anyhow::bail!("Server-to-client proxy task failed: {join_err}"),
                }
                client_to_server.abort();
                let _ = client_to_server.await;
            }
            res1 = &mut client_to_server => {
                match res1 {
                    Ok(result) => result?,
                    Err(join_err) => anyhow::bail!("Client-to-server proxy task failed: {join_err}"),
                }
                match server_to_client.await {
                    Ok(result) => result?,
                    Err(join_err) => anyhow::bail!("Server-to-client proxy task failed: {join_err}"),
                }
            }
        }

        Ok(())
    }

    /// Shutdown the bridge adapter and the Perl::LanguageServer process
    ///
    /// This method tries a graceful termination first and falls back to kill.
    /// It should be used for cleanup in async contexts.
    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(mut child) = self.child_process.take() {
            if !Self::wait_for_child_exit(&mut child, Duration::from_millis(0)).await {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.shutdown().await;
                }

                #[cfg(unix)]
                {
                    if let Some(pid) = child.id() {
                        if let Ok(()) = signal::kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
                            if Self::wait_for_child_exit(
                                &mut child,
                                Duration::from_millis(PLS_SHUTDOWN_GRACE_MS),
                            )
                            .await
                            {
                                return Ok(());
                            }
                        }
                    }
                }

                let _ = child.kill().await;
                if !Self::wait_for_child_exit(
                    &mut child,
                    Duration::from_millis(PLS_SHUTDOWN_GRACE_MS),
                )
                .await
                {
                    let _ = child.wait().await?;
                }
            }
        }
        Ok(())
    }

    async fn wait_for_child_exit(child: &mut Child, timeout: Duration) -> bool {
        if let Ok(Some(_)) = child.try_wait() {
            return true;
        }

        if timeout.is_zero() {
            return false;
        }

        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) => return true,
                Ok(None) => sleep(Duration::from_millis(PLS_SHUTDOWN_POLL_MS)).await,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to poll Perl::LanguageServer process");
                    return false;
                }
            }
        }

        matches!(child.try_wait(), Ok(Some(_)))
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
        // Note: In async code, drop is synchronous, so we can't await `child.kill()`
        // But `start_kill` is non-blocking (available in newer tokio versions)
        // or we can use the synchronous API if we held the std handle, but we don't.
        // For tokio::process::Child, start_kill() starts the killing.
        if let Some(mut child) = self.child_process.take() {
            let _ = child.start_kill();
        }
    }
}
