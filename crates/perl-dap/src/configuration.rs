//! DAP launch and attach configuration structures
//!
//! This module provides configuration types for DAP debugging sessions,
//! supporting both launch (start new process) and attach (connect to running process) modes.
//!
//! # Examples
//!
//! ## Launch Configuration
//!
//! ```no_run
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
//!     timeout_ms: Some(5000),
//! };
//! ```

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
}

impl Default for AttachConfiguration {
    fn default() -> Self {
        Self { host: "localhost".to_string(), port: 13603, timeout_ms: Some(5000) }
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
    /// use perl_dap::AttachConfiguration;
    ///
    /// let config = AttachConfiguration {
    ///     host: "localhost".to_string(),
    ///     port: 13603,
    ///     timeout_ms: Some(5000),
    /// };
    ///
    /// config.validate().expect("Valid configuration");
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
    serde_json::to_string_pretty(&json).unwrap_or_else(|e| {
        eprintln!("Failed to serialize launch.json snippet: {}", e);
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
    serde_json::to_string_pretty(&json).unwrap_or_else(|e| {
        eprintln!("Failed to serialize attach.json snippet: {}", e);
        "{}".to_string()
    })
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
        assert_eq!(config.timeout_ms, Some(5000));
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

    // Edge case tests for mutation testing hardening

    #[test]
    fn test_launch_config_validation_missing_program() {
        // Test: program file doesn't exist
        let config = LaunchConfiguration {
            program: PathBuf::from("/nonexistent/script.pl"),
            args: vec![],
            cwd: None,
            env: HashMap::new(),
            perl_path: None,
            include_paths: vec![],
        };

        let result = config.validate();
        assert!(result.is_err(), "Should fail validation for missing program file");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("does not exist"),
            "Error should mention file doesn't exist"
        );
    }

    #[test]
    fn test_launch_config_validation_program_is_directory() {
        // Test: program path is a directory, not a file
        use std::env;
        let config = LaunchConfiguration {
            program: env::current_dir().expect("Failed to get current dir"),
            args: vec![],
            cwd: None,
            env: HashMap::new(),
            perl_path: None,
            include_paths: vec![],
        };

        let result = config.validate();
        assert!(result.is_err(), "Should fail validation when program is a directory");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not a file"), "Error should mention path is not a file");
    }

    #[test]
    fn test_launch_config_validation_invalid_cwd() {
        // Test: cwd is not a directory
        use std::env;

        // Create a config with a file as cwd (should fail)
        let temp_file = env::temp_dir().join("test_file.txt");
        std::fs::write(&temp_file, "test").expect("Failed to create temp file");

        let config = LaunchConfiguration {
            program: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
            args: vec![],
            cwd: Some(temp_file.clone()),
            env: HashMap::new(),
            perl_path: None,
            include_paths: vec![],
        };

        let result = config.validate();
        // Clean up temp file
        let _ = std::fs::remove_file(&temp_file);

        assert!(result.is_err(), "Should fail validation when cwd is not a directory");
    }

    #[test]
    fn test_launch_config_validation_missing_cwd() {
        // Test: cwd directory doesn't exist
        let config = LaunchConfiguration {
            program: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
            args: vec![],
            cwd: Some(PathBuf::from("/nonexistent/directory")),
            env: HashMap::new(),
            perl_path: None,
            include_paths: vec![],
        };

        let result = config.validate();
        assert!(result.is_err(), "Should fail validation for missing cwd");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("does not exist"),
            "Error should mention directory doesn't exist"
        );
    }

    #[test]
    fn test_launch_config_validation_invalid_perl_path() {
        // Test: perl_path doesn't exist
        let config = LaunchConfiguration {
            program: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
            args: vec![],
            cwd: None,
            env: HashMap::new(),
            perl_path: Some(PathBuf::from("/nonexistent/perl")),
            include_paths: vec![],
        };

        let result = config.validate();
        assert!(result.is_err(), "Should fail validation for missing perl binary");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("does not exist"),
            "Error should mention perl binary doesn't exist"
        );
    }

    #[test]
    fn test_launch_config_path_resolution_absolute() {
        // Test: absolute paths don't get modified
        let mut config = LaunchConfiguration {
            program: PathBuf::from("/absolute/path/script.pl"),
            args: vec![],
            cwd: Some(PathBuf::from("/absolute/cwd")),
            env: HashMap::new(),
            perl_path: None,
            include_paths: vec![PathBuf::from("/absolute/lib")],
        };

        config.resolve_paths(&PathBuf::from("/workspace")).expect("resolve_paths failed");

        assert_eq!(
            config.program,
            PathBuf::from("/absolute/path/script.pl"),
            "Absolute program path should be preserved"
        );
        assert_eq!(
            config.cwd.as_ref().unwrap(),
            &PathBuf::from("/absolute/cwd"),
            "Absolute cwd should be preserved"
        );
        assert_eq!(
            config.include_paths[0],
            PathBuf::from("/absolute/lib"),
            "Absolute include path should be preserved"
        );
    }

    #[test]
    fn test_launch_config_path_resolution_relative() {
        // Test: relative paths get resolved against workspace root
        let mut config = LaunchConfiguration {
            program: PathBuf::from("script.pl"),
            args: vec![],
            cwd: Some(PathBuf::from("build")),
            env: HashMap::new(),
            perl_path: None,
            include_paths: vec![PathBuf::from("lib")],
        };

        let workspace = PathBuf::from("/workspace");
        config.resolve_paths(&workspace).expect("resolve_paths failed");

        assert_eq!(
            config.program,
            workspace.join("script.pl"),
            "Relative program path should be resolved"
        );
        assert_eq!(
            config.cwd.as_ref().unwrap(),
            &workspace.join("build"),
            "Relative cwd should be resolved"
        );
        assert_eq!(
            config.include_paths[0],
            workspace.join("lib"),
            "Relative include path should be resolved"
        );
    }

    #[test]
    fn test_attach_config_custom_port() {
        // Test: custom port handling
        let config = AttachConfiguration {
            host: "192.168.1.100".to_string(),
            port: 9000,
            timeout_ms: Some(10000),
        };

        let json = serde_json::to_string(&config).expect("Serialization failed");
        assert!(json.contains("192.168.1.100"), "Should contain custom host");
        assert!(json.contains("9000"), "Should contain custom port");
    }

    #[test]
    fn test_attach_config_validation_valid() {
        // Test: valid attach configuration
        let config = AttachConfiguration {
            host: "localhost".to_string(),
            port: 13603,
            timeout_ms: Some(5000),
        };

        assert!(config.validate().is_ok(), "Valid config should pass validation");
    }

    #[test]
    fn test_attach_config_validation_empty_host() {
        // Test: empty host fails validation
        let config =
            AttachConfiguration { host: "".to_string(), port: 13603, timeout_ms: Some(5000) };

        let result = config.validate();
        assert!(result.is_err(), "Empty host should fail validation");
        assert!(result.unwrap_err().to_string().contains("Host"));
    }

    #[test]
    fn test_attach_config_validation_whitespace_host() {
        // Test: whitespace-only host fails validation
        let config =
            AttachConfiguration { host: "   ".to_string(), port: 13603, timeout_ms: Some(5000) };

        let result = config.validate();
        assert!(result.is_err(), "Whitespace host should fail validation");
    }

    #[test]
    fn test_attach_config_validation_zero_port() {
        // Test: port 0 is invalid
        let config =
            AttachConfiguration { host: "localhost".to_string(), port: 0, timeout_ms: Some(5000) };

        let result = config.validate();
        assert!(result.is_err(), "Port 0 should fail validation");
        assert!(result.unwrap_err().to_string().contains("Port"));
    }

    #[test]
    fn test_attach_config_validation_zero_timeout() {
        // Test: zero timeout fails validation
        let config =
            AttachConfiguration { host: "localhost".to_string(), port: 13603, timeout_ms: Some(0) };

        let result = config.validate();
        assert!(result.is_err(), "Zero timeout should fail validation");
        assert!(result.unwrap_err().to_string().contains("Timeout"));
    }

    #[test]
    fn test_attach_config_validation_excessive_timeout() {
        // Test: timeout > 5 minutes fails validation
        let config = AttachConfiguration {
            host: "localhost".to_string(),
            port: 13603,
            timeout_ms: Some(400_000), // 400 seconds
        };

        let result = config.validate();
        assert!(result.is_err(), "Excessive timeout should fail validation");
    }

    #[test]
    fn test_attach_config_validation_no_timeout() {
        // Test: no timeout specified is valid
        let config =
            AttachConfiguration { host: "localhost".to_string(), port: 13603, timeout_ms: None };

        assert!(config.validate().is_ok(), "Config without timeout should be valid");
    }

    #[test]
    fn test_launch_json_snippet_valid_json() {
        // Test: generated JSON snippets parse correctly
        let snippet = create_launch_json_snippet();
        let parsed: serde_json::Value =
            serde_json::from_str(&snippet).expect("Launch JSON snippet should be valid JSON");

        assert_eq!(parsed["type"], "perl");
        assert_eq!(parsed["request"], "launch");
        assert!(parsed["program"].is_string());
        assert!(parsed["args"].is_array());
        assert!(parsed["perlPath"].is_string());
        assert!(parsed["includePaths"].is_array());
    }

    #[test]
    fn test_attach_json_snippet_valid_json() {
        // Test: attach JSON snippet is valid and complete
        let snippet = create_attach_json_snippet();
        let parsed: serde_json::Value =
            serde_json::from_str(&snippet).expect("Attach JSON snippet should be valid JSON");

        assert_eq!(parsed["type"], "perl");
        assert_eq!(parsed["request"], "attach");
        assert_eq!(parsed["host"], "localhost");
        assert_eq!(parsed["port"], 13603);
        assert!(parsed["timeout"].is_number());
    }

    #[test]
    fn test_launch_config_empty_args() {
        // Test: empty args array is valid
        let config = LaunchConfiguration {
            program: PathBuf::from("script.pl"),
            args: vec![],
            cwd: None,
            env: HashMap::new(),
            perl_path: None,
            include_paths: vec![],
        };

        let json = serde_json::to_string(&config).expect("Serialization failed");
        assert!(json.contains("\"args\":[]"), "Empty args should serialize correctly");
    }

    #[test]
    fn test_launch_config_empty_include_paths() {
        // Test: empty include_paths is valid
        let config = LaunchConfiguration {
            program: PathBuf::from("script.pl"),
            args: vec![],
            cwd: None,
            env: HashMap::new(),
            perl_path: None,
            include_paths: vec![],
        };

        let json = serde_json::to_string(&config).expect("Serialization failed");
        assert!(
            json.contains("\"includePaths\":[]"),
            "Empty include_paths should serialize correctly"
        );
    }
}
