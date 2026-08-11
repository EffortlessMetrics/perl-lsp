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

// This module retains the compatibility implementation while its public types
// guide new callers toward the native adapter through deprecation diagnostics.
#![allow(deprecated)]

use anyhow::{Context, Result};
#[cfg(unix)]
use nix::sys::signal::{self, Signal};
#[cfg(unix)]
use nix::unistd::Pid;
use perl_lsp_rs_core::config::PerlOracleEnv;
use std::path::PathBuf;
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

/// Environment passthrough policy for the Perl::LanguageServer DAP bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
#[deprecated(
    note = "legacy Perl::LanguageServer compatibility; use native perl-dap configuration instead"
)]
pub struct DapBridgeEnvConfig {
    /// Pass parent `PERL5LIB` through to the bridged Perl process.
    pub perl5lib_passthrough: bool,
    /// Pass parent `PERL5OPT` through to the bridged Perl process.
    pub perl5opt_passthrough: bool,
}

impl DapBridgeEnvConfig {
    /// Create an explicit bridge environment policy.
    pub const fn new(perl5lib_passthrough: bool, perl5opt_passthrough: bool) -> Self {
        Self { perl5lib_passthrough, perl5opt_passthrough }
    }
}

/// Bridge adapter that proxies DAP messages to Perl::LanguageServer
///
/// This adapter spawns Perl::LanguageServer in DAP mode and forwards
/// all DAP protocol messages bidirectionally via stdio.
#[deprecated(note = "legacy Perl::LanguageServer compatibility; use native perl-dap instead")]
pub struct BridgeAdapter {
    /// The spawned Perl::LanguageServer process
    child_process: Option<Child>,
    /// Env passthrough policy for the bridge subprocess.
    env_config: DapBridgeEnvConfig,
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
        Self::with_env_config(DapBridgeEnvConfig::default())
    }

    /// Create a bridge adapter with an explicit subprocess environment policy.
    pub fn with_env_config(env_config: DapBridgeEnvConfig) -> Self {
        Self { child_process: None, env_config }
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
        let child = self
            .build_pls_dap_command(perl_path)
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

    fn build_pls_dap_command(&self, perl_path: PathBuf) -> Command {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let oracle = PerlOracleEnv::for_dap_bridge(
            perl_path,
            cwd,
            self.env_config.perl5lib_passthrough,
            self.env_config.perl5opt_passthrough,
        );
        let mut command = Command::new(&oracle.perl_binary);
        oracle.configure_command(command.as_std_mut());
        command
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

        // JoinHandle<T> is already Unpin; no pin! macro needed here.
        let mut client_to_server = client_to_server;
        let mut server_to_client = server_to_client;

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
                            client_to_server.abort();
                            client_result = Some(Ok(()));
                        }
                        if server_result.is_none() {
                            server_to_client.abort();
                            server_result = Some(Ok(()));
                        }
                    }
                }
            }
        }

        // Propagate errors from both directions; log the secondary error so it isn't silently lost.
        let client_err = client_result.and_then(|r| r.err());
        let server_err = server_result.and_then(|r| r.err());
        match (client_err, server_err) {
            (Some(ce), Some(se)) => {
                tracing::warn!(error = %se, "proxy server->client also failed");
                return Err(ce);
            }
            (Some(ce), None) => return Err(ce),
            (None, Some(se)) => return Err(se),
            (None, None) => {}
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
    use super::{BridgeAdapter, DapBridgeEnvConfig};
    use anyhow::Result;
    use std::path::PathBuf;
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

    #[test]
    fn bridge_env_config_defaults_to_deny_ambient_perl_env() {
        let adapter = BridgeAdapter::new();
        let command = adapter.build_pls_dap_command(PathBuf::from("perl"));
        let envs: Vec<_> = command.as_std().get_envs().collect();

        assert!(
            envs.iter().all(|(key, _)| *key != "PERL5LIB"),
            "default bridge config must not pass PERL5LIB"
        );
        assert!(
            envs.iter().all(|(key, _)| *key != "PERL5OPT"),
            "default bridge config must not pass PERL5OPT"
        );
        assert!(
            command.as_std().get_current_dir().is_some(),
            "bridge command should set cwd explicitly"
        );
    }

    #[test]
    fn bridge_env_config_allows_debug_passthrough_when_opted_in() {
        let adapter = BridgeAdapter::with_env_config(DapBridgeEnvConfig::new(true, true));
        let oracle = perl_lsp_rs_core::config::PerlOracleEnv::for_dap_bridge(
            PathBuf::from("perl"),
            PathBuf::from("."),
            adapter.env_config.perl5lib_passthrough,
            adapter.env_config.perl5opt_passthrough,
        );

        assert!(oracle.allow_perl5lib, "opt-in bridge config should allow PERL5LIB");
        assert!(oracle.allow_perl5opt, "opt-in bridge config should allow PERL5OPT");
        assert!(!oracle.allow_local_lib, "bridge config should not widen local::lib");
    }

    async fn process_is_running(pid: u32) -> Result<bool> {
        #[cfg(unix)]
        {
            let status = Command::new("sh")
                .arg("-c")
                .arg(format!("kill -0 {pid} 2>/dev/null"))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await?;
            Ok(status.success())
        }

        #[cfg(windows)]
        {
            let output = Command::new("tasklist")
                .arg("/FI")
                .arg(format!("PID eq {pid}"))
                .arg("/NH")
                .output()
                .await?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("tasklist failed while checking child process {pid}: {stderr}");
            }
            Ok(String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
        }
    }

    #[tokio::test]
    async fn proxy_returns_error_when_child_already_exited() -> Result<()> {
        let mut adapter = BridgeAdapter::new();
        let mut child = spawn_short_lived_child().await?;
        // Wait for the process to actually exit before handing it to the adapter.
        // A sleep-based approach is flaky on slow CI (cmd.exe on Windows can take >40ms).
        // child.wait() guarantees the exit status is available before try_wait() is called.
        let _ = child.wait().await?;
        adapter.child_process = Some(child);

        let result =
            tokio::time::timeout(Duration::from_millis(500), adapter.proxy_messages()).await;
        assert!(result.is_ok(), "proxy_messages should complete quickly for exited child");
        let proxy_result = result?;
        assert!(proxy_result.is_err(), "proxy_messages should error for exited child");

        Ok(())
    }

    #[tokio::test]
    async fn repeated_start_stop_cycles_do_not_leave_child_processes() -> Result<()> {
        let mut observed_pids = Vec::new();

        for _ in 0..5 {
            let mut adapter = BridgeAdapter::new();
            let child = spawn_long_running_child().await?;
            let pid = child
                .id()
                .ok_or_else(|| anyhow::anyhow!("spawned child should have a process id"))?;
            observed_pids.push(pid);
            adapter.child_process = Some(child);

            let result = tokio::time::timeout(Duration::from_secs(2), adapter.shutdown()).await;
            assert!(result.is_ok(), "shutdown should not hang for child process {pid}");
            result??;

            assert!(
                !process_is_running(pid).await?,
                "DAP bridge shutdown left child process {pid} running"
            );
        }

        observed_pids.sort_unstable();
        observed_pids.dedup();
        assert_eq!(observed_pids.len(), 5, "each cycle should own a distinct child process");
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

    /// Verify the poll-arm path: proxy_messages() must not hang when the child
    /// process exits *during* active proxying (after the pre-flight liveness check
    /// has already passed but while the I/O copy tasks are still running).
    ///
    /// We use a child that sleeps briefly then exits on its own, bypassing the
    /// pre-flight check (which only fires when the child is already dead at call
    /// time). The select! poll arm is responsible for detecting this mid-proxy
    /// exit and aborting the I/O tasks within PROXY_EXIT_POLL_MS + margin.
    #[tokio::test]
    async fn proxy_completes_when_child_exits_during_proxy() -> Result<()> {
        // Spawn a child that stays alive briefly, then exits.
        // We need it to survive the pre-flight try_wait() call (so it must be alive
        // at proxy_messages() entry), then exit during the poll loop.
        #[cfg(unix)]
        let child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg("sleep 0.1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        #[cfg(windows)]
        let child = tokio::process::Command::new("cmd")
            .arg("/C")
            // ping with -n 1 exits after ~0ms; -w 100 waits 100ms for the reply
            // (it will fail, but the process exits after the timeout).
            .arg("ping -n 1 -w 100 127.0.0.1 >NUL")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let mut adapter = BridgeAdapter::new();
        adapter.child_process = Some(child);

        // Give the runtime a yield so the child can be scheduled; the child
        // should still be alive at this point (it sleeps 100ms on Unix,
        // 100ms ping timeout on Windows).
        tokio::task::yield_now().await;

        // proxy_messages() must return without hanging. Allow up to 5 seconds:
        // 100ms child lifetime + PROXY_EXIT_POLL_MS (25ms) + generous CI margin.
        let result = tokio::time::timeout(Duration::from_secs(5), adapter.proxy_messages()).await;

        assert!(
            result.is_ok(),
            "proxy_messages should not hang when child exits mid-proxy; timed out"
        );
        // The function should return Ok(()) -- the child exited cleanly and the
        // poll arm aborted the I/O tasks without an I/O error.
        let proxy_result = result?;
        assert!(
            proxy_result.is_ok(),
            "proxy_messages should return Ok(()) when child exits cleanly mid-proxy, got: {proxy_result:?}"
        );

        Ok(())
    }
}
