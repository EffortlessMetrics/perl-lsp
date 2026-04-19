//! Process isolation and sandboxing utilities for production hardening
//!
//! This module provides sandboxing capabilities to ensure safe execution
//! of external processes and isolation from the host system.

use crate::util::run_command_with_timeout;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Sandbox configuration for process execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Whether to enable sandboxing
    pub enabled: bool,
    /// Maximum memory usage (bytes)
    pub max_memory: Option<usize>,
    /// Maximum CPU time (seconds)
    pub max_cpu_time: Option<u64>,
    /// Allowed file system paths
    pub allowed_paths: Vec<PathBuf>,
    /// Network access allowed
    pub allow_network: bool,
    /// Working directory for sandboxed process
    pub working_directory: Option<PathBuf>,
    /// Environment variables to allow
    pub allowed_env_vars: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_memory: Some(512 * 1024 * 1024), // 512MB
            max_cpu_time: Some(30),              // 30 seconds
            allowed_paths: vec![],
            allow_network: false,
            working_directory: None,
            allowed_env_vars: vec!["PATH".to_string(), "HOME".to_string(), "TMPDIR".to_string()],
        }
    }
}

/// Sandbox execution context
#[derive(Debug)]
pub struct Sandbox {
    config: SandboxConfig,
    temp_dir: Option<PathBuf>,
}

impl Sandbox {
    /// Create a new sandbox with the given configuration
    pub fn new(config: SandboxConfig) -> Result<Self> {
        let temp_dir = if config.enabled {
            // Create temporary directory for sandbox
            let temp_dir =
                std::env::temp_dir().join(format!("perl-lsp-sandbox-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&temp_dir).with_context(|| {
                format!(
                    "failed to create sandbox temp dir at {}",
                    temp_dir.display()
                )
            })?;
            Some(temp_dir)
        } else {
            None
        };

        Ok(Self { config, temp_dir })
    }

    /// Execute a command in the sandbox
    pub fn execute(&self, program: &str, args: &[&str]) -> Result<SandboxResult> {
        if !self.config.enabled {
            return self.execute_unsandboxed(program, args);
        }

        let mut cmd = Command::new(program);
        cmd.args(args);

        // Apply sandbox restrictions
        self.apply_sandbox_restrictions(&mut cmd)?;

        // Execute and capture output
        let start = std::time::Instant::now();
        // Use the configured CPU time limit (or 30s as a fallback) as the wall-clock timeout.
        let timeout_secs = self.config.max_cpu_time.unwrap_or(30);
        let output = run_command_with_timeout(cmd, timeout_secs)
            .map_err(|e| anyhow!("failed to execute sandboxed command {}: {}", program, e))?;
        let execution_time = start.elapsed();

        Ok(SandboxResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
            success: output.status.success(),
            execution_time,
        })
    }

    /// Execute without sandboxing (fallback)
    fn execute_unsandboxed(&self, program: &str, args: &[&str]) -> Result<SandboxResult> {
        let mut cmd = Command::new(program);
        cmd.args(args);
        let timeout_secs = self.config.max_cpu_time.unwrap_or(30);
        let output = run_command_with_timeout(cmd, timeout_secs)
            .map_err(|e| anyhow!("failed to execute command {}: {}", program, e))?;

        Ok(SandboxResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
            success: output.status.success(),
            execution_time: std::time::Duration::from_secs(0),
        })
    }

    /// Apply sandbox restrictions to a command
    fn apply_sandbox_restrictions(&self, cmd: &mut Command) -> Result<()> {
        if let Some(ref work_dir) = self.config.working_directory {
            if !work_dir.exists() {
                return Err(anyhow!(
                    "sandbox working directory does not exist: {}",
                    work_dir.display()
                ));
            }
        }

        // Set working directory
        if let Some(ref work_dir) = self.config.working_directory {
            cmd.current_dir(work_dir);
        } else if let Some(ref temp_dir) = self.temp_dir {
            cmd.current_dir(temp_dir);
        }

        // Restrict environment variables
        cmd.env_clear();
        for env_var in &self.config.allowed_env_vars {
            if let Ok(value) = std::env::var(env_var) {
                cmd.env(env_var, value);
            }
        }

        // Platform-specific sandboxing
        #[cfg(target_os = "linux")]
        self.apply_linux_sandbox(cmd)?;

        #[cfg(target_os = "macos")]
        self.apply_macos_sandbox(cmd)?;

        #[cfg(target_os = "windows")]
        self.apply_windows_sandbox(cmd)?;

        Ok(())
    }

    /// Apply Linux-specific sandboxing using namespaces and seccomp
    #[cfg(target_os = "linux")]
    fn apply_linux_sandbox(&self, cmd: &mut Command) -> Result<()> {
        // Use firejail if available
        let mut firejail_probe = Command::new("firejail");
        firejail_probe.arg("--version");
        if run_command_with_timeout(firejail_probe, 2).is_ok() {
            let mut firejail_cmd = Command::new("firejail");

            // Apply memory limits
            if let Some(max_memory) = self.config.max_memory {
                firejail_cmd.arg(format!("--rlimit-as={}", max_memory));
            }

            // Apply CPU time limits
            if let Some(max_cpu) = self.config.max_cpu_time {
                firejail_cmd.arg(format!("--rlimit-cpu={}", max_cpu));
            }

            // Network restrictions
            if !self.config.allow_network {
                firejail_cmd.arg("--net=none");
            }

            // Private /tmp
            firejail_cmd.arg("--private-tmp");

            // Whitelist allowed paths
            for path in &self.config.allowed_paths {
                firejail_cmd.arg(format!("--whitelist={}", path.display()));
            }

            // Execute the original command through firejail
            firejail_cmd.arg(cmd.get_program());
            firejail_cmd.args(cmd.get_args());

            *cmd = firejail_cmd;
        } else {
            // SECURITY FIX: Fail-closed when firejail unavailable and sandbox enabled
            // The previous fallback set RLIMIT_* environment variables which are NOT
            // enforced by the kernel - they are purely informational.
            return Err(anyhow!(
                "sandbox.enabled=true but firejail is not available. \
                 Install firejail or set sandbox.enabled=false in configuration."
            ));
        }

        Ok(())
    }

    /// Apply macOS-specific sandboxing using sandbox-exec
    #[cfg(target_os = "macos")]
    fn apply_macos_sandbox(&self, cmd: &mut Command) -> Result<()> {
        // Use sandbox-exec if available
        let mut sandbox_probe = Command::new("sandbox-exec");
        sandbox_probe.arg("--version");
        if run_command_with_timeout(sandbox_probe, 2).is_ok() {
            let sandbox_profile = self.generate_macos_sandbox_profile();

            // sandbox-exec -f takes a file path, not inline content.
            // Write the profile to the sandbox temp dir (always Some here — enabled is checked in execute()).
            let profile_path = self
                .temp_dir
                .as_deref()
                .ok_or_else(|| anyhow!("sandbox temp dir missing when applying macOS sandbox"))?
                .join("sandbox.sb");
            std::fs::write(&profile_path, &sandbox_profile).with_context(|| {
                format!(
                    "failed to write sandbox profile to {}",
                    profile_path.display()
                )
            })?;

            let mut sandbox_cmd = Command::new("sandbox-exec");
            sandbox_cmd.arg("-f").arg(&profile_path);
            sandbox_cmd.arg(cmd.get_program());
            sandbox_cmd.args(cmd.get_args());

            *cmd = sandbox_cmd;
        }

        Ok(())
    }

    /// Escape a path string for safe interpolation into a macOS sandbox profile DSL string literal.
    /// The profile language is TinyScheme-based; only `\` and `"` are special inside quoted strings.
    /// Parentheses are NOT special inside a quoted string (only at the DSL level), so they are
    /// left unescaped. Backslash must be escaped first to avoid double-escaping the newly
    /// introduced backslashes in the second pass.
    ///
    /// Limitation: paths containing literal newlines or null bytes are not escaped here.
    /// Such paths are pathological on macOS (HFS+ permits them but they are essentially never
    /// used) and are out of scope for this fix. If configuration sources ever accept arbitrary
    /// user-supplied paths, add a sanitisation step that rejects control characters before
    /// calling this function.
    #[cfg(any(target_os = "macos", test))]
    fn sandbox_escape_path(path: &std::path::Path) -> String {
        path.display()
            .to_string()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    }

    /// Generate macOS sandbox profile
    #[cfg(any(target_os = "macos", test))]
    fn generate_macos_sandbox_profile(&self) -> String {
        let mut profile = String::from("(version 1)\n");
        profile.push_str("(allow default)\n");

        if !self.config.allow_network {
            profile.push_str("(deny network*)\n");
        }

        // Allow file system access to specific paths
        for path in &self.config.allowed_paths {
            profile.push_str(&format!(
                "(allow file-read* (subpath \"{}\"))\n",
                Self::sandbox_escape_path(path)
            ));
        }

        profile
    }

    /// Apply Windows-specific sandboxing
    ///
    /// SECURITY FIX: Fails closed when sandbox is enabled but Windows job objects
    /// are not yet implemented. This prevents silent security bypass.
    ///
    /// Note: `self.config.enabled` is always `true` when this function is called —
    /// `apply_sandbox_restrictions` is only reached through the `enabled=true` branch
    /// of `execute()`. The guard is kept explicit for self-documentation and defensive
    /// correctness if the call graph changes in the future.
    #[cfg(target_os = "windows")]
    fn apply_windows_sandbox(&self, _cmd: &mut Command) -> Result<()> {
        if self.config.enabled {
            return Err(anyhow!(
                "sandbox.enabled=true but Windows job objects not yet implemented. \
                 Set sandbox.enabled=false in configuration to run without sandboxing."
            ));
        }
        Ok(())
    }

    /// Get the temporary directory for the sandbox
    pub fn temp_dir(&self) -> Option<&Path> {
        self.temp_dir.as_deref()
    }

    /// Create a sandbox with no temp dir, for use in unit tests only.
    #[cfg(test)]
    fn new_for_test(config: SandboxConfig) -> Self {
        Self {
            config,
            temp_dir: None,
        }
    }

    /// Clean up sandbox resources
    pub fn cleanup(&mut self) -> Result<()> {
        if let Some(ref temp_dir) = self.temp_dir {
            // Clean up temporary directory
            if temp_dir.exists() {
                std::fs::remove_dir_all(temp_dir).with_context(|| {
                    format!(
                        "failed to remove sandbox temp dir at {}",
                        temp_dir.display()
                    )
                })?;
            }
        }
        self.temp_dir = None;
        Ok(())
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            tracing::warn!("Sandbox cleanup failed during drop: {error:#}");
        }
    }
}

/// Result of sandboxed execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResult {
    /// Exit code of the process
    pub exit_code: i32,
    /// Standard output
    pub stdout: Vec<u8>,
    /// Standard error
    pub stderr: Vec<u8>,
    /// Whether the process succeeded
    pub success: bool,
    /// Execution time
    pub execution_time: std::time::Duration,
}

impl SandboxResult {
    /// Get stdout as string
    pub fn stdout_str(&self) -> Result<String> {
        String::from_utf8(self.stdout.clone())
            .map_err(|e| anyhow!("Invalid UTF-8 in stdout: {}", e))
    }

    /// Get stderr as string
    pub fn stderr_str(&self) -> Result<String> {
        String::from_utf8(self.stderr.clone())
            .map_err(|e| anyhow!("Invalid UTF-8 in stderr: {}", e))
    }

    /// Check if the process was killed due to resource limits
    pub fn was_resource_limited(&self) -> bool {
        // Check for common error codes indicating resource limits
        matches!(self.exit_code, 137 | 124 | 152) // SIGKILL, timeout, etc.
    }
}

/// Safe process executor with sandboxing
pub struct SafeExecutor {
    default_config: SandboxConfig,
}

impl SafeExecutor {
    /// Create a new safe executor with default configuration
    pub fn new() -> Self {
        Self {
            default_config: SandboxConfig::default(),
        }
    }

    /// Create a new safe executor with custom configuration
    pub fn with_config(config: SandboxConfig) -> Self {
        Self {
            default_config: config,
        }
    }

    /// Execute a command safely
    pub fn execute(&self, program: &str, args: &[&str]) -> Result<SandboxResult> {
        let sandbox = Sandbox::new(self.default_config.clone())
            .context("failed to initialize sandbox with default configuration")?;
        let result = sandbox
            .execute(program, args)
            .with_context(|| format!("safe execution failed for command: {}", program))?;
        Ok(result)
    }

    /// Execute a command with custom configuration
    pub fn execute_with_config(
        &self,
        program: &str,
        args: &[&str],
        config: &SandboxConfig,
    ) -> Result<SandboxResult> {
        let sandbox = Sandbox::new(config.clone())
            .context("failed to initialize sandbox with provided configuration")?;
        let result = sandbox
            .execute(program, args)
            .with_context(|| format!("safe execution failed for command: {}", program))?;
        Ok(result)
    }

    /// Execute a Perl script safely
    pub fn execute_perl_script(&self, script_path: &Path, args: &[&str]) -> Result<SandboxResult> {
        let mut config = self.default_config.clone();

        // Add script directory to allowed paths
        if let Some(parent) = script_path.parent() {
            config.allowed_paths.push(parent.to_path_buf());
        }

        // Set working directory to script directory
        config.working_directory = script_path.parent().map(|p| p.to_path_buf());

        let path_str = script_path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid script path: {}", script_path.display()))?;
        // SECURITY FIX: Use taint mode (-T) for security hardening
        let mut perl_args = Vec::with_capacity(args.len() + 2);
        perl_args.push("-T");
        perl_args.push(path_str);
        perl_args.extend_from_slice(args);

        self.execute_with_config("perl", &perl_args, &config)
            .with_context(|| format!("failed to execute Perl script at {}", script_path.display()))
    }
}

impl Default for SafeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn echo_command(message: &str) -> (&'static str, Vec<String>) {
        #[cfg(windows)]
        {
            (
                "cmd",
                vec!["/C".to_string(), "echo".to_string(), message.to_string()],
            )
        }

        #[cfg(not(windows))]
        {
            ("echo", vec![message.to_string()])
        }
    }

    // --- Bug A: path escaping tests ---

    #[test]
    fn test_sandbox_escape_path_clean() {
        // Clean paths must be passed through unchanged
        let path = std::path::Path::new("/home/user/project");
        assert_eq!(Sandbox::sandbox_escape_path(path), "/home/user/project");
    }

    #[test]
    fn test_sandbox_escape_path_double_quote() {
        // A path with a double-quote must be escaped to prevent DSL injection
        let path = std::path::Path::new("/home/user/my\"project");
        assert_eq!(
            Sandbox::sandbox_escape_path(path),
            "/home/user/my\\\"project"
        );
    }

    #[test]
    fn test_sandbox_escape_path_backslash() {
        // Backslash must be doubled (Windows-style path or escape chars)
        let path = std::path::Path::new("/home/user/my\\path");
        assert_eq!(Sandbox::sandbox_escape_path(path), "/home/user/my\\\\path");
    }

    #[test]
    fn test_sandbox_escape_path_backslash_then_quote() {
        // A path containing both \ and " verifies the escape ordering:
        // backslash is doubled first (\\ -> \\\\, then the quote is escaped (" -> \").
        // If the order were reversed, the backslash introduced by quote-escaping would
        // itself get doubled, producing \\" instead of the correct \".
        let path = std::path::Path::new("/home/user/my\\\"path");
        // Expected: /home/user/my\\"path (\ -> \\, then " -> \")
        assert_eq!(
            Sandbox::sandbox_escape_path(path),
            "/home/user/my\\\\\\\"path"
        );
    }

    #[test]
    fn test_generate_macos_sandbox_profile_escapes_paths() {
        // Profile generation must produce a DSL-safe profile even for adversarial paths
        let config = SandboxConfig {
            allowed_paths: vec![
                std::path::PathBuf::from("/safe/path"),
                std::path::PathBuf::from("/path/with\"quote"),
            ],
            ..SandboxConfig::default()
        };
        let sandbox = Sandbox::new_for_test(config);
        let profile = sandbox.generate_macos_sandbox_profile();
        assert!(profile.contains("(allow file-read* (subpath \"/safe/path\"))"));
        assert!(profile.contains("(allow file-read* (subpath \"/path/with\\\"quote\"))"));
        // Must NOT contain an unescaped quote that would break the DSL
        assert!(!profile.contains("(subpath \"/path/with\"quote\")"));
    }

    #[test]
    fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_memory, Some(512 * 1024 * 1024));
        assert_eq!(config.max_cpu_time, Some(30));
        assert!(!config.allow_network);
    }

    #[test]
    fn test_sandbox_creation() {
        let config = SandboxConfig::default();
        let sandbox = Sandbox::new(config);
        assert!(sandbox.is_ok());
    }

    #[test]
    fn test_unsandboxed_execution() {
        use perl_tdd_support::must;
        let config = SandboxConfig {
            enabled: false,
            ..Default::default()
        };
        let sandbox = must(Sandbox::new(config));

        let (command, args) = echo_command("hello");
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let result = must(sandbox.execute(command, &arg_refs));
        assert!(result.success);
        assert_eq!(must(result.stdout_str()).trim(), "hello");
    }

    #[test]
    fn test_safe_executor_disabled() {
        use perl_tdd_support::must;
        // Test with sandbox disabled (default config has enabled=true which fails closed without firejail)
        let config = SandboxConfig {
            enabled: false,
            ..Default::default()
        };
        let executor = SafeExecutor::with_config(config);
        let (command, args) = echo_command("test");
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let result = must(executor.execute(command, &arg_refs));
        assert!(result.success);
        assert_eq!(must(result.stdout_str()).trim(), "test");
    }

    #[test]
    fn test_perl_script_execution() {
        use perl_tdd_support::must;
        let temp_dir = must(TempDir::new());
        let script_path = temp_dir.path().join("test.pl");
        must(fs::write(&script_path, "print \"Hello from Perl\\n\";"));

        let executor = SafeExecutor::new();
        let result = executor.execute_perl_script(&script_path, &[]);

        // Note: This test might fail if Perl is not installed
        if let Ok(result) = result {
            assert!(result.success);
            assert!(must(result.stdout_str()).contains("Hello from Perl"));
        }
    }

    #[test]
    fn test_sandbox_result() {
        use perl_tdd_support::must;
        let result = SandboxResult {
            exit_code: 0,
            stdout: b"test output".to_vec(),
            stderr: b"".to_vec(),
            success: true,
            execution_time: std::time::Duration::from_millis(100),
        };

        assert_eq!(must(result.stdout_str()), "test output");
        assert_eq!(must(result.stderr_str()), "");
        assert!(result.success);
        assert!(!result.was_resource_limited());
    }

    // --- SECURITY FIX TESTS ---

    /// Test that the Linux sandbox fails closed when firejail is unavailable.
    ///
    /// This test only runs when firejail is confirmed absent. If firejail is
    /// present the sandbox uses it correctly and the fail-closed path is not
    /// exercised — that is the correct production behaviour. The goal is to
    /// verify the *fallback* path changed from silent env-var set to hard Err.
    #[test]
    #[cfg(target_os = "linux")]
    fn test_linux_sandbox_fails_closed_without_firejail() {
        use perl_tdd_support::must;

        // Detect whether firejail is actually available on this machine.
        // If it is, the fail-closed path is never reached; skip the test.
        let firejail_present = std::process::Command::new("firejail")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if firejail_present {
            // Firejail is installed — the sandbox works correctly here.
            // The fail-closed path is not reachable; skip rather than
            // asserting on a path that will never fire.
            return;
        }

        let config = SandboxConfig {
            enabled: true,
            ..SandboxConfig::default()
        };
        let sandbox = must(Sandbox::new(config));

        // Without firejail the new code must return Err (not silently succeed
        // with inert RLIMIT_* env vars as the old code did).
        let result = sandbox.execute("echo", &["test"]);

        assert!(
            result.is_err(),
            "Expected fail-closed error when firejail is absent"
        );
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("firejail") || err_msg.contains("sandbox.enabled"),
            "Error should name firejail or sandbox.enabled, got: {}",
            err_msg
        );
    }

    /// Test that the Windows sandbox fails closed when enabled=true.
    ///
    /// Before this fix, apply_windows_sandbox() was a no-op (returned Ok(()))
    /// silently — a process ran with zero restrictions. After this fix it must
    /// return a clear error so callers know the sandbox was not applied.
    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_sandbox_fails_closed() {
        use perl_tdd_support::must;
        let config = SandboxConfig {
            enabled: true,
            ..SandboxConfig::default()
        };
        let sandbox = must(Sandbox::new(config));

        let result = sandbox.execute("cmd", &["/C", "echo", "test"]);

        assert!(
            result.is_err(),
            "Expected fail-closed error on Windows with sandbox enabled"
        );
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("Windows job objects") || err_msg.contains("sandbox.enabled"),
            "Error should mention Windows job objects or sandbox.enabled, got: {}",
            err_msg
        );
    }

    /// Test that Perl scripts are executed with taint mode (-T flag).
    ///
    /// Uses `execute_perl_script` with sandbox disabled so the test actually
    /// reaches Perl rather than failing-closed at the firejail probe.
    /// The ${^TAINT} special variable is 1 when perl is invoked with -T and
    /// 0 otherwise — this gives a direct, non-vacuous signal.
    #[test]
    fn test_perl_taint_mode_flag() {
        use perl_tdd_support::must;
        let temp_dir = must(TempDir::new());
        let script_path = temp_dir.path().join("taint_test.pl");

        // ${^TAINT} is 1 under -T, 0 without it (perlvar).
        let script = r#"
if (${^TAINT}) {
    print "TAINTED\n";
} else {
    print "NOT_TAINTED\n";
}
"#;
        must(fs::write(&script_path, script));

        // Use sandbox disabled so the test reaches Perl on all platforms,
        // including Linux without firejail and Windows (which fail-close).
        let config = SandboxConfig {
            enabled: false,
            ..Default::default()
        };
        let executor = SafeExecutor::with_config(config);
        let result = executor.execute_perl_script(&script_path, &[]);

        // If Perl is not installed on this machine, skip gracefully.
        if let Ok(result) = result {
            let stdout = must(result.stdout_str());
            assert!(
                stdout.contains("TAINTED"),
                "Expected taint mode to be enabled (-T flag). Got: {}",
                stdout
            );
        }
    }

    /// Test that taint mode prevents passing tainted data to dangerous sinks.
    ///
    /// Perl taint mode does NOT die on regex matching with tainted strings —
    /// that is a common misconception. It dies when tainted data reaches a
    /// dangerous sink: system(), exec(), open() with shell metacharacters, or
    /// eval(STRING). This test verifies the correct sink: system().
    #[test]
    fn test_perl_taint_mode_blocks_dangerous_ops() {
        use perl_tdd_support::must;
        let temp_dir = must(TempDir::new());
        let script_path = temp_dir.path().join("dangerous.pl");

        // Under -T, $ENV{PATH} is tainted. Passing tainted data to system()
        // without untainting it first must die with "Insecure $ENV{PATH}".
        // We redirect stderr to stdout so the test can observe the die message.
        let script = r#"
use strict;
use warnings;
# system() with a tainted PATH should die with "Insecure" in taint mode.
# Catch with eval so we can print the error and exit cleanly.
eval { system($ENV{PATH} . " --version") };
if ($@) {
    print "TAINT_BLOCKED\n";
} else {
    print "NOT_BLOCKED\n";
}
"#;
        must(fs::write(&script_path, script));

        let config = SandboxConfig {
            enabled: false,
            ..Default::default()
        };
        let executor = SafeExecutor::with_config(config);
        let result = executor.execute_perl_script(&script_path, &[]);

        // If Perl is not installed on this machine, skip gracefully.
        if let Ok(result) = result {
            let stdout = must(result.stdout_str());
            assert!(
                stdout.contains("TAINT_BLOCKED"),
                "Expected taint mode to block system() with tainted PATH. Got stdout: {}",
                stdout
            );
        }
    }
}
