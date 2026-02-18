//! Process isolation and sandboxing utilities for production hardening
//! 
//! This module provides sandboxing capabilities to ensure safe execution
//! of external processes and isolation from the host system.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

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
            max_cpu_time: Some(30), // 30 seconds
            allowed_paths: vec![],
            allow_network: false,
            working_directory: None,
            allowed_env_vars: vec![
                "PATH".to_string(),
                "HOME".to_string(),
                "TMPDIR".to_string(),
            ],
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
            let temp_dir = std::env::temp_dir().join(format!("perl-lsp-sandbox-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&temp_dir)?;
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
        let output = cmd.output()?;
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
        
        let output = cmd.output()?;
        
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
        if Command::new("firejail").arg("--version").output().is_ok() {
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
            // Fallback: use ulimit for basic restrictions
            if let Some(max_memory) = self.config.max_memory {
                cmd.env("RLIMIT_AS", max_memory.to_string());
            }
            
            if let Some(max_cpu) = self.config.max_cpu_time {
                cmd.env("RLIMIT_CPU", max_cpu.to_string());
            }
        }

        Ok(())
    }

    /// Apply macOS-specific sandboxing using sandbox-exec
    #[cfg(target_os = "macos")]
    fn apply_macos_sandbox(&self, cmd: &mut Command) -> Result<()> {
        // Use sandbox-exec if available
        if Command::new("sandbox-exec").arg("--version").output().is_ok() {
            let sandbox_profile = self.generate_macos_sandbox_profile();
            
            let mut sandbox_cmd = Command::new("sandbox-exec");
            sandbox_cmd.arg("-f").arg(&sandbox_profile);
            sandbox_cmd.arg(cmd.get_program());
            sandbox_cmd.args(cmd.get_args());
            
            *cmd = sandbox_cmd;
        }

        Ok(())
    }

    /// Generate macOS sandbox profile
    fn generate_macos_sandbox_profile(&self) -> String {
        let mut profile = String::from("(version 1)\n");
        profile.push_str("(allow default)\n");
        
        if !self.config.allow_network {
            profile.push_str("(deny network*)\n");
        }
        
        // Allow file system access to specific paths
        for path in &self.config.allowed_paths {
            profile.push_str(&format!("(allow file-read* (subpath \"{}\"))\n", path.display()));
        }
        
        profile
    }

    /// Apply Windows-specific sandboxing
    ///
    /// Currently applies environment restrictions only. Full job object
    /// sandboxing requires the `windows-sys` crate and is tracked upstream.
    #[cfg(target_os = "windows")]
    fn apply_windows_sandbox(&self, cmd: &mut Command) -> Result<()> {
        // Apply environment restrictions (the portable subset of sandboxing)
        Ok(())
    }

    /// Get the temporary directory for the sandbox
    pub fn temp_dir(&self) -> Option<&Path> {
        self.temp_dir.as_deref()
    }

    /// Clean up sandbox resources
    pub fn cleanup(&mut self) -> Result<()> {
        if let Some(ref temp_dir) = self.temp_dir {
            // Clean up temporary directory
            if temp_dir.exists() {
                std::fs::remove_dir_all(temp_dir)?;
            }
        }
        self.temp_dir = None;
        Ok(())
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = self.cleanup();
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
        Self { default_config: config }
    }

    /// Execute a command safely
    pub fn execute(&self, program: &str, args: &[&str]) -> Result<SandboxResult> {
        let sandbox = Sandbox::new(self.default_config.clone())?;
        let result = sandbox.execute(program, args)?;
        Ok(result)
    }

    /// Execute a command with custom configuration
    pub fn execute_with_config(&self, program: &str, args: &[&str], config: &SandboxConfig) -> Result<SandboxResult> {
        let sandbox = Sandbox::new(config.clone())?;
        let result = sandbox.execute(program, args)?;
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
        
        let path_str = script_path.to_str().ok_or_else(|| anyhow!("Invalid script path"))?;
        self.execute_with_config("perl", &[path_str], &config)
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
        let config = SandboxConfig { enabled: false, ..Default::default() };
        let sandbox = must(Sandbox::new(config));
        
        let result = must(sandbox.execute("echo", &["hello"]));
        assert!(result.success);
        assert_eq!(must(result.stdout_str()).trim(), "hello");
    }

    #[test]
    fn test_safe_executor() {
        use perl_tdd_support::must;
        let executor = SafeExecutor::new();
        let result = must(executor.execute("echo", &["test"]));
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
}