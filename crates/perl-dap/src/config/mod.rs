//! Standalone DAP launch and attach configuration structures
//!
//! This module provides configuration types for DAP debugging sessions,
//! supporting both launch (start new process) and attach (connect to running process) modes.
//!
//! # Examples
//!
//! ## Launch Configuration
//!
//! ```no_run
//! use perl_dap_config::LaunchConfiguration;
//! use std::path::PathBuf;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut config = LaunchConfiguration {
//!     program: PathBuf::from("script.pl"),
//!     args: vec!["--verbose".to_string()],
//!     cwd: Some(PathBuf::from("/workspace")),
//!     env: std::collections::HashMap::new(),
//!     perl_path: None,
//!     include_paths: vec![],
//! };
//!
//! config.validate()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Attach Configuration
//!
//! ```
//! use perl_dap_config::AttachConfiguration;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = AttachConfiguration {
//!     host: "localhost".to_string(),
//!     port: 13603,
//!     timeout_ms: Some(5000),
//!     stop_on_entry: None,
//! };
//!
//! config.validate()?;
//! # Ok(())
//! # }
//! ```

// Lint enforcement: library code must use tracing, not direct stderr/stdout prints.
#![deny(clippy::print_stderr, clippy::print_stdout)]
#![cfg_attr(test, allow(clippy::print_stderr, clippy::print_stdout))]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Validate that a path exists and is a file
fn validate_file_exists(path: &Path, description: &str) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("{} does not exist: {}", description, path.display());
    }
    if !path.is_file() {
        anyhow::bail!("{} is not a file: {}", description, path.display());
    }
    Ok(())
}

/// Validate that a path exists and is a directory
fn validate_directory_exists(path: &Path, description: &str) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("{} does not exist: {}", description, path.display());
    }
    if !path.is_dir() {
        anyhow::bail!("{} is not a directory: {}", description, path.display());
    }
    Ok(())
}

/// Launch configuration for starting a new Perl debugging session
///
/// This configuration is used when starting a new Perl process for debugging.
/// It includes the program path, arguments, environment variables, and Perl-specific
/// settings like include paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchConfiguration {
    /// Path to the Perl script to debug (required)
    pub program: PathBuf,

    /// Command-line arguments to pass to the script
    #[serde(default)]
    pub args: Vec<String>,

    /// Working directory for the debugged process
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,

    /// Environment variables to set for the debugged process
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Path to the perl binary (defaults to "perl" on PATH)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub perl_path: Option<PathBuf>,

    /// Additional paths to add to @INC (Perl's include path)
    #[serde(default)]
    pub include_paths: Vec<PathBuf>,
}

impl LaunchConfiguration {
    /// Resolve workspace-relative paths to absolute paths
    ///
    /// This method converts relative paths in the configuration to absolute paths
    /// based on the workspace root. It handles:
    /// - Program path resolution
    /// - Working directory resolution
    /// - Include path resolution
    ///
    /// # Arguments
    ///
    /// * `workspace_root` - The workspace root directory
    ///
    /// # Errors
    ///
    /// Returns an error if path resolution fails
    ///
    /// # Examples
    ///
    /// ```
    /// use perl_dap_config::LaunchConfiguration;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let mut config = LaunchConfiguration {
    ///     program: PathBuf::from("script.pl"),
    ///     args: vec![],
    ///     cwd: None,
    ///     env: std::collections::HashMap::new(),
    ///     perl_path: None,
    ///     include_paths: vec![PathBuf::from("lib")],
    /// };
    ///
    /// config.resolve_paths(&PathBuf::from("/workspace"))?;
    /// assert!(config.program.is_absolute());
    /// # Ok(())
    /// # }
    /// ```
    pub fn resolve_paths(&mut self, workspace_root: &Path) -> Result<()> {
        // Resolve program path
        if !self.program.is_absolute() {
            self.program = workspace_root.join(&self.program);
        }

        // Resolve working directory
        if let Some(ref mut cwd) = self.cwd
            && !cwd.is_absolute()
        {
            *cwd = workspace_root.join(&cwd);
        }

        // Resolve include paths
        for include_path in &mut self.include_paths {
            if !include_path.is_absolute() {
                *include_path = workspace_root.join(&include_path);
            }
        }

        Ok(())
    }

    /// Validate the configuration
    ///
    /// This method checks that:
    /// - Program path exists and is a file
    /// - Working directory exists (if specified)
    /// - Perl binary exists (if specified)
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use perl_dap_config::LaunchConfiguration;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let config = LaunchConfiguration {
    ///     program: PathBuf::from("/path/to/script.pl"),
    ///     args: vec![],
    ///     cwd: None,
    ///     env: std::collections::HashMap::new(),
    ///     perl_path: None,
    ///     include_paths: vec![],
    /// };
    ///
    /// config.validate()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn validate(&self) -> Result<()> {
        // Verify program exists
        validate_file_exists(&self.program, "Program file")?;

        // Verify working directory exists (if specified)
        if let Some(ref cwd) = self.cwd {
            validate_directory_exists(cwd, "Working directory")?;
        }

        // Verify perl binary exists (if specified)
        if let Some(ref perl_path) = self.perl_path {
            validate_file_exists(perl_path, "Perl binary")?;
        }

        Ok(())
    }
}

/// Attach configuration for connecting to a running Perl debugging session
///
/// This configuration is used when attaching to an already-running Perl process
/// that has been started with the Perl::LanguageServer DAP module.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachConfiguration {
    /// Host to connect to (typically "localhost")
    pub host: String,

    /// Port number for the DAP server (default: 13603)
    pub port: u16,

    /// Connection timeout in milliseconds (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,

    /// If true, pause execution at the first opportunity after attaching.
    ///
    /// Equivalent to the DAP `stopOnEntry` field. When set, the adapter emits a
    /// `stopped` event with `reason = "entry"` immediately after the attach
    /// handshake completes. Defaults to `false` when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_on_entry: Option<bool>,
}

impl Default for AttachConfiguration {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 13603,
            timeout_ms: Some(5000),
            stop_on_entry: None,
        }
    }
}

impl AttachConfiguration {
    /// Validate the attach configuration
    ///
    /// This method checks that:
    /// - Host is not empty
    /// - Port is in valid range (1-65535)
    /// - Timeout is reasonable (if specified)
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails
    ///
    /// # Examples
    ///
    /// ```
    /// use perl_dap_config::AttachConfiguration;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = AttachConfiguration {
    ///     host: "localhost".to_string(),
    ///     port: 13603,
    ///     timeout_ms: Some(5000),
    ///     stop_on_entry: None,
    /// };
    ///
    /// config.validate()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn validate(&self) -> Result<()> {
        // Verify host is not empty
        if self.host.trim().is_empty() {
            anyhow::bail!("Host cannot be empty");
        }

        // Port is u16, so it's automatically in range 0-65535
        // But we should reject port 0 as it's not valid for connecting
        if self.port == 0 {
            anyhow::bail!("Port must be in range 1-65535");
        }

        // Verify timeout is reasonable (if specified)
        if let Some(timeout) = self.timeout_ms {
            if timeout == 0 {
                anyhow::bail!("Timeout must be greater than 0 milliseconds");
            }
            if timeout > 300_000 {
                // 5 minutes max
                anyhow::bail!("Timeout cannot exceed 300000 milliseconds (5 minutes)");
            }
        }

        Ok(())
    }
}

/// Create a launch.json configuration snippet
///
/// This function generates a JSON snippet suitable for use in VS Code's launch.json
/// file. The snippet includes placeholders for the program path and other common options.
///
/// # Returns
///
/// A JSON string containing the launch configuration template
///
/// # Examples
///
/// ```
/// use perl_dap_config::create_launch_json_snippet;
///
/// let snippet = create_launch_json_snippet();
/// assert!(snippet.contains("\"type\""));
/// assert!(snippet.contains("\"launch\""));
/// ```
pub fn create_launch_json_snippet() -> String {
    let json = serde_json::json!({
        "type": "perl",
        "request": "launch",
        "name": "Launch Perl Script",
        "program": "${workspaceFolder}/script.pl",
        "args": [],
        "perlPath": "perl",
        "includePaths": ["${workspaceFolder}/lib"],
        "cwd": "${workspaceFolder}",
        "env": {}
    });
    serde_json::to_string_pretty(&json).unwrap_or_else(|e| {
        tracing::error!(error = %e, "Failed to serialize launch.json snippet");
        "{}".to_string()
    })
}

/// Create an attach.json configuration snippet
///
/// This function generates a JSON snippet for attaching to a running Perl::LanguageServer
/// DAP session via TCP.
///
/// # Returns
///
/// A JSON string containing the attach configuration template
///
/// # Examples
///
/// ```
/// use perl_dap_config::create_attach_json_snippet;
///
/// let snippet = create_attach_json_snippet();
/// assert!(snippet.contains("\"type\""));
/// assert!(snippet.contains("\"attach\""));
/// assert!(snippet.contains("13603"));
/// ```
pub fn create_attach_json_snippet() -> String {
    let json = serde_json::json!({
        "type": "perl",
        "request": "attach",
        "name": "Attach to Perl::LanguageServer",
        "host": "localhost",
        "port": 13603,
        "timeout": 5000,
        "stopOnEntry": false
    });
    serde_json::to_string_pretty(&json).unwrap_or_else(|e| {
        tracing::error!(error = %e, "Failed to serialize attach.json snippet");
        "{}".to_string()
    })
}
