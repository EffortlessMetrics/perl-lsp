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

const PLS_SHUTDOWN_GRACE_MS: u64 = 250;
const PLS_SHUTDOWN_POLL_MS: u64 = 25;
const PROXY_EXIT_POLL_MS: u64 = 25;

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

        let mut child = child;
        if let Some(status) =
            child.try_wait().context("Failed to check Perl::LanguageServer startup status")?
        {
            anyhow::bail!(
                "Perl::LanguageServer DAP process exited immediately with status: {status}"
            );
        }

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

        if let Some(status) = child
            .try_wait()
            .context("Failed to check Perl::LanguageServer process state before proxying")?
        {
            anyhow::bail!(
                "Perl::LanguageServer DAP process exited before proxying with status: {status}"
            );
        }

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
        let client_to_server = tokio::spawn(async move {
            tokio::io::copy(&mut parent_stdin, &mut child_stdin)
                .await
                .context("Error copying from client to server")?;
            // Shut down child_stdin to signal EOF to the server
            let _ = child_stdin.shutdown().await;
            Ok::<(), anyhow::Error>(())
        });

        // Task 2: Server (Child Stdout) -> Client (Parent Stdout)
        let server_to_client = tokio::spawn(async move {
            tokio::io::copy(&mut child_stdout, &mut parent_stdout)
                .await
                .context("Error copying from server to client")?;
            parent_stdout.flush().await.context("Error flushing to client")?;
            Ok::<(), anyhow::Error>(())
        });

        tokio::pin!(client_to_server);
        tokio::pin!(server_to_client);

        let mut client_result: Option<Result<()>> = None;
        let mut server_result: Option<Result<()>> = None;

        while client_result.is_none() || server_result.is_none() {
            tokio::select! {
                join_result = &mut client_to_server, if client_result.is_none() => {
                    client_result = Some(Self::join_result_to_anyhow(join_result, "client->server"));
                }
                join_result = &mut server_to_client, if server_result.is_none() => {
                    server_result = Some(Self::join_result_to_anyhow(join_result, "server->client"));
                }
                _ = sleep(Duration::from_millis(PROXY_EXIT_POLL_MS)) => {
                    if child
                        .try_wait()
                        .context("Failed polling Perl::LanguageServer process during proxy")?
                        .is_some()
                    {
                        if client_result.is_none() {
                            client_to_server.as_mut().abort();
                            client_result = Some(Ok(()));
                        }
                        if server_result.is_none() {
                            server_to_client.as_mut().abort();
                            server_result = Some(Ok(()));
                        }
                    }
                }
            }
        }

        if let Some(result) = client_result {
            result?;
        }
        if let Some(result) = server_result {
            result?;
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

        false
    }

    fn join_result_to_anyhow(
        join_result: std::result::Result<Result<()>, tokio::task::JoinError>,
        direction: &'static str,
    ) -> Result<()> {
        match join_result {
            Ok(inner) => inner,
            Err(join_error) if join_error.is_cancelled() => Ok(()),
            Err(join_error) => Err(anyhow::anyhow!(
                "Proxy task {direction} panicked or failed to join: {join_error}"
            )),
        }
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

#[cfg(test)]
mod tests {
    use super::BridgeAdapter;
    use anyhow::Result;
    use std::process::Stdio;
    use std::time::Duration;
    use tokio::process::Command;

    async fn spawn_short_lived_child() -> Result<tokio::process::Child> {
        #[cfg(unix)]
        {
            let child = Command::new("sh")
                .arg("-c")
                .arg("exit 0")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?;
            Ok(child)
        }

        #[cfg(windows)]
        {
            let child = Command::new("cmd")
                .arg("/C")
                .arg("exit 0")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?;
            Ok(child)
        }
    }

    async fn spawn_long_running_child() -> Result<tokio::process::Child> {
        #[cfg(unix)]
        {
            let child = Command::new("sh")
                .arg("-c")
                .arg("sleep 30")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?;
            Ok(child)
        }

        #[cfg(windows)]
        {
            let child = Command::new("cmd")
                .arg("/C")
                .arg("timeout /T 30 /NOBREAK >NUL")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?;
            Ok(child)
        }
    }

    #[tokio::test]
    async fn proxy_returns_error_when_child_already_exited() -> Result<()> {
        let mut adapter = BridgeAdapter::new();
        adapter.child_process = Some(spawn_short_lived_child().await?);
        tokio::time::sleep(Duration::from_millis(40)).await;

        let result =
            tokio::time::timeout(Duration::from_millis(500), adapter.proxy_messages()).await;
        assert!(result.is_ok(), "proxy_messages should complete quickly for exited child");
        let proxy_result = result?;
        assert!(proxy_result.is_err(), "proxy_messages should error for exited child");

        Ok(())
    }

    #[tokio::test]
    async fn shutdown_terminates_running_child() -> Result<()> {
        let mut adapter = BridgeAdapter::new();
        adapter.child_process = Some(spawn_long_running_child().await?);

        let result = tokio::time::timeout(Duration::from_secs(2), adapter.shutdown()).await;
        assert!(result.is_ok(), "shutdown should not hang for a running child process");
        result??;

        Ok(())
    }
}
