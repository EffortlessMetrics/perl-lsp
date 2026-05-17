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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::{NamedTempFile, tempdir};

    fn minimal_launch(program: PathBuf) -> LaunchConfiguration {
        LaunchConfiguration {
            program,
            args: Vec::new(),
            cwd: None,
            env: HashMap::new(),
            perl_path: None,
            include_paths: Vec::new(),
        }
    }

    #[test]
    fn resolve_paths_converts_relative_launch_paths() -> Result<()> {
        let workspace = tempdir()?;
        let mut config = LaunchConfiguration {
            program: PathBuf::from("bin/app.pl"),
            args: vec!["--flag".to_string()],
            cwd: Some(PathBuf::from("work")),
            env: HashMap::new(),
            perl_path: None,
            include_paths: vec![PathBuf::from("lib"), PathBuf::from("vendor/lib")],
        };

        config.resolve_paths(workspace.path())?;

        assert_eq!(config.program, workspace.path().join("bin/app.pl"));
        assert_eq!(config.cwd, Some(workspace.path().join("work")));
        assert_eq!(
            config.include_paths,
            vec![workspace.path().join("lib"), workspace.path().join("vendor/lib")]
        );
        assert_eq!(config.args, vec!["--flag"]);
        Ok(())
    }

    #[test]
    fn resolve_paths_preserves_absolute_launch_paths() -> Result<()> {
        let workspace = tempdir()?;
        let absolute_program = workspace.path().join("script.pl");
        let absolute_cwd = workspace.path().join("cwd");
        let absolute_include = workspace.path().join("abs-lib");
        let mut config = LaunchConfiguration {
            program: absolute_program.clone(),
            args: Vec::new(),
            cwd: Some(absolute_cwd.clone()),
            env: HashMap::new(),
            perl_path: None,
            include_paths: vec![absolute_include.clone()],
        };

        config.resolve_paths(workspace.path())?;

        assert_eq!(config.program, absolute_program);
        assert_eq!(config.cwd, Some(absolute_cwd));
        assert_eq!(config.include_paths, vec![absolute_include]);
        Ok(())
    }

    #[test]
    fn launch_validate_accepts_existing_program_cwd_and_perl_binary() -> Result<()> {
        let program = NamedTempFile::new()?;
        let perl_binary = NamedTempFile::new()?;
        let cwd = tempdir()?;
        let config = LaunchConfiguration {
            program: program.path().to_path_buf(),
            args: Vec::new(),
            cwd: Some(cwd.path().to_path_buf()),
            env: HashMap::new(),
            perl_path: Some(perl_binary.path().to_path_buf()),
            include_paths: Vec::new(),
        };

        config.validate()?;

        Ok(())
    }

    #[test]
    fn launch_validate_rejects_missing_program() -> Result<()> {
        let missing_program = tempdir()?.path().join("missing.pl");
        let config = minimal_launch(missing_program);

        let error = config.validate().err();

        assert!(error.is_some_and(|err| err.to_string().contains("Program file does not exist")));
        Ok(())
    }

    #[test]
    fn launch_validate_rejects_directory_program() -> Result<()> {
        let dir = tempdir()?;
        let config = minimal_launch(dir.path().to_path_buf());

        let error = config.validate().err();

        assert!(error.is_some_and(|err| err.to_string().contains("Program file is not a file")));
        Ok(())
    }

    #[test]
    fn launch_validate_rejects_invalid_cwd_and_perl_binary() -> Result<()> {
        let program = NamedTempFile::new()?;
        let file_as_cwd = NamedTempFile::new()?;
        let config = LaunchConfiguration {
            program: program.path().to_path_buf(),
            args: Vec::new(),
            cwd: Some(file_as_cwd.path().to_path_buf()),
            env: HashMap::new(),
            perl_path: None,
            include_paths: Vec::new(),
        };

        let cwd_error = config.validate().err();

        assert!(
            cwd_error.is_some_and(|err| err
                .to_string()
                .contains("Working directory is not a directory"))
        );

        let missing_perl = tempdir()?.path().join("perl");
        let config = LaunchConfiguration {
            program: program.path().to_path_buf(),
            args: Vec::new(),
            cwd: None,
            env: HashMap::new(),
            perl_path: Some(missing_perl),
            include_paths: Vec::new(),
        };

        let perl_error = config.validate().err();

        assert!(
            perl_error.is_some_and(|err| err.to_string().contains("Perl binary does not exist"))
        );
        Ok(())
    }

    #[test]
    fn attach_default_matches_documented_endpoint() {
        let config = AttachConfiguration::default();

        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 13603);
        assert_eq!(config.timeout_ms, Some(5000));
        assert_eq!(config.stop_on_entry, None);
    }

    #[test]
    fn attach_validate_accepts_boundary_timeout_values() -> Result<()> {
        for timeout_ms in [None, Some(1), Some(300_000)] {
            let config = AttachConfiguration {
                host: "127.0.0.1".to_string(),
                port: 1,
                timeout_ms,
                stop_on_entry: Some(true),
            };

            config.validate()?;
        }

        Ok(())
    }

    #[test]
    fn attach_validate_rejects_blank_host_zero_port_and_bad_timeouts() {
        let blank_host = AttachConfiguration {
            host: " \t\n ".to_string(),
            port: 13603,
            timeout_ms: Some(5000),
            stop_on_entry: None,
        };
        assert!(blank_host.validate().is_err_and(|err| err.to_string() == "Host cannot be empty"));

        let zero_port = AttachConfiguration {
            host: "localhost".to_string(),
            port: 0,
            timeout_ms: Some(5000),
            stop_on_entry: None,
        };
        assert!(
            zero_port
                .validate()
                .is_err_and(|err| err.to_string() == "Port must be in range 1-65535")
        );

        let zero_timeout = AttachConfiguration {
            host: "localhost".to_string(),
            port: 13603,
            timeout_ms: Some(0),
            stop_on_entry: None,
        };
        assert!(
            zero_timeout
                .validate()
                .is_err_and(|err| err.to_string() == "Timeout must be greater than 0 milliseconds")
        );

        let excessive_timeout = AttachConfiguration {
            host: "localhost".to_string(),
            port: 13603,
            timeout_ms: Some(300_001),
            stop_on_entry: None,
        };
        assert!(excessive_timeout.validate().is_err_and(|err| {
            err.to_string() == "Timeout cannot exceed 300000 milliseconds (5 minutes)"
        }));
    }

    #[test]
    fn launch_snippet_contains_deserializable_launch_defaults() -> Result<()> {
        let value: Value = serde_json::from_str(&create_launch_json_snippet())?;

        assert_eq!(value["type"], "perl");
        assert_eq!(value["request"], "launch");
        assert_eq!(value["program"], "${workspaceFolder}/script.pl");
        assert_eq!(value["perlPath"], "perl");
        assert_eq!(value["cwd"], "${workspaceFolder}");
        assert_eq!(value["includePaths"][0], "${workspaceFolder}/lib");
        Ok(())
    }

    #[test]
    fn attach_snippet_contains_deserializable_attach_defaults() -> Result<()> {
        let value: Value = serde_json::from_str(&create_attach_json_snippet())?;

        assert_eq!(value["type"], "perl");
        assert_eq!(value["request"], "attach");
        assert_eq!(value["host"], "localhost");
        assert_eq!(value["port"], 13603);
        assert_eq!(value["timeout"], 5000);
        assert_eq!(value["stopOnEntry"], false);
        Ok(())
    }

    #[test]
    fn launch_validate_accepts_program_created_with_fs_write() -> Result<()> {
        let dir = tempdir()?;
        let program = dir.path().join("script.pl");
        fs::write(&program, "use strict;\n")?;
        let config = minimal_launch(program);

        config.validate()?;

        Ok(())
    }
}
