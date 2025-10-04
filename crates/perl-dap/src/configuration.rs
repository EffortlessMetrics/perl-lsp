//! DAP launch and attach configuration structures
//!
//! This module provides configuration types for DAP debugging sessions,
//! supporting both launch (start new process) and attach (connect to running process) modes.
//!
//! # Examples
//!
//! ## Launch Configuration
//!
//! ```
//! use perl_dap::LaunchConfiguration;
//! use std::path::PathBuf;
//!
//! let mut config = LaunchConfiguration {
//!     program: PathBuf::from("script.pl"),
//!     args: vec!["--verbose".to_string()],
//!     cwd: Some(PathBuf::from("/workspace")),
//!     env: std::collections::HashMap::new(),
//!     perl_path: None,
//!     include_paths: vec![],
//! };
//!
//! config.validate().expect("Valid configuration");
//! ```
//!
//! ## Attach Configuration
//!
//! ```
//! use perl_dap::AttachConfiguration;
//!
//! let config = AttachConfiguration {
//!     host: "localhost".to_string(),
//!     port: 13603,
//! };
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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
    /// use perl_dap::LaunchConfiguration;
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
    pub fn resolve_paths(&mut self, workspace_root: &PathBuf) -> Result<()> {
        // Resolve program path
        if !self.program.is_absolute() {
            self.program = workspace_root.join(&self.program);
        }

        // Resolve working directory
        if let Some(ref mut cwd) = self.cwd {
            if !cwd.is_absolute() {
                *cwd = workspace_root.join(&cwd);
            }
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
    /// use perl_dap::LaunchConfiguration;
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
        if !self.program.exists() {
            anyhow::bail!("Program file does not exist: {}", self.program.display());
        }

        if !self.program.is_file() {
            anyhow::bail!("Program path is not a file: {}", self.program.display());
        }

        // Verify working directory exists (if specified)
        if let Some(ref cwd) = self.cwd {
            if !cwd.exists() {
                anyhow::bail!("Working directory does not exist: {}", cwd.display());
            }

            if !cwd.is_dir() {
                anyhow::bail!("Working directory is not a directory: {}", cwd.display());
            }
        }

        // Verify perl binary exists (if specified)
        if let Some(ref perl_path) = self.perl_path {
            if !perl_path.exists() {
                anyhow::bail!("Perl binary does not exist: {}", perl_path.display());
            }
        }

        Ok(())
    }
}

/// Attach configuration for connecting to a running Perl debugging session
///
/// This configuration is used when attaching to an already-running Perl process
/// that has been started with the Perl::LanguageServer DAP module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachConfiguration {
    /// Host to connect to (typically "localhost")
    pub host: String,

    /// Port number for the DAP server (default: 13603)
    pub port: u16,
}

impl Default for AttachConfiguration {
    fn default() -> Self {
        Self { host: "localhost".to_string(), port: 13603 }
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
/// use perl_dap::create_launch_json_snippet;
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
    serde_json::to_string_pretty(&json).unwrap()
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
/// use perl_dap::create_attach_json_snippet;
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
        "timeout": 5000
    });
    serde_json::to_string_pretty(&json).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launch_configuration_serialization() {
        let config = LaunchConfiguration {
            program: PathBuf::from("/path/to/script.pl"),
            args: vec!["--verbose".to_string()],
            cwd: Some(PathBuf::from("/workspace")),
            env: HashMap::new(),
            perl_path: Some(PathBuf::from("/usr/bin/perl")),
            include_paths: vec![PathBuf::from("/workspace/lib")],
        };

        let json = serde_json::to_string(&config).expect("Serialization failed");
        assert!(json.contains("perlPath"));
        assert!(json.contains("includePaths"));
    }

    #[test]
    fn test_attach_configuration_default() {
        let config = AttachConfiguration::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 13603);
    }

    #[test]
    fn test_launch_json_snippet() {
        let snippet = create_launch_json_snippet();
        assert!(snippet.contains("\"type\""));
        assert!(snippet.contains("perl"));
        assert!(snippet.contains("\"request\""));
        assert!(snippet.contains("launch"));
        assert!(snippet.contains("program"));
    }

    #[test]
    fn test_attach_json_snippet() {
        let snippet = create_attach_json_snippet();
        assert!(snippet.contains("\"type\""));
        assert!(snippet.contains("perl"));
        assert!(snippet.contains("\"request\""));
        assert!(snippet.contains("attach"));
        assert!(snippet.contains("13603"));
    }
}
