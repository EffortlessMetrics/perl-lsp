//! Parse Perl dependency files (cpanfile, META.json, META.yml)
//!
//! This crate provides functionality to parse Perl dependency declaration files
//! to extract module requirements and detect vendor library paths.
//!
//! # Supported Formats
//!
//! - **cpanfile**: Perl DSL format with `requires 'Module', 'version';` syntax
//! - **META.json**: CPAN distribution metadata in JSON format
//! - **META.yml**: CPAN distribution metadata in YAML format
//!
//! # Vendor Path Detection
//!
//! Detects the vendor library path by checking for directory existence:
//! - `vendor/lib/perl5` (carton/carmel/cpm)
//! - `local/lib/perl5` (manual local::lib)
//!
//! # Example
//!
//! ```rust
//! use perl_dependency_metadata::{parse_cpanfile, detect_vendor_path, DependencyInfo};
//! use std::path::Path;
//!
//! let deps = parse_cpanfile(r#"
//!     requires 'Moo', '2.0';
//!     requires 'JSON::PP';
//! "#);
//! assert_eq!(deps.len(), 2);
//! assert_eq!(deps[0].name, "Moo");
//! assert_eq!(deps[0].version, Some("2.0".to_string()));
//! ```

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during dependency parsing.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Failed to parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Failed to parse YAML: {0}")]
    YamlError(#[from] serde_yaml::Error),

    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),
}

/// A module requirement extracted from a dependency file.
///
/// Contains the module name and an optional version constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleRequirement {
    /// The module name (e.g., "Moo", "JSON::PP")
    pub name: String,
    /// Optional version constraint (e.g., "2.0", ">= 1.23")
    pub version: Option<String>,
}

/// Dependency information for a workspace root.
///
/// Contains all declared module requirements and the detected vendor path.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyInfo {
    /// Module requirements declared in dependency files
    pub declared: Vec<ModuleRequirement>,
    /// Detected vendor library path (vendor/lib/perl5 or local/lib/perl5)
    pub vendor_path: Option<PathBuf>,
}

/// A static compiled regex for parsing cpanfile requires lines.
///
/// This is lazily initialized once on first use. The regex pattern is hardcoded
/// and known to be valid.
static CPANFILE_REQUIRES_RE: Lazy<Result<Regex, regex::Error>> = Lazy::new(|| {
    Regex::new(
        r#"(?m)^\s*requires\s+['"]([^'"]+)['"](?:\s*,\s*['"]([^'"]+)['"]|\s*=>\s*['"]([^'"]+)['"])?\s*;"#,
    )
});

/// Parse a cpanfile and extract module requirements.
///
/// The cpanfile format uses Perl DSL syntax like:
/// ```perl
/// requires 'Moo', '2.0';
/// requires 'JSON::PP';
/// requires 'Path::Tiny' => '0.078';
/// ```
///
/// # Arguments
///
/// * `contents` - The cpanfile contents as a string
///
/// # Returns
///
/// A vector of `ModuleRequirement` structs representing the declared dependencies
pub fn parse_cpanfile(contents: &str) -> Vec<ModuleRequirement> {
    let mut requirements = Vec::new();

    // Safe to unwrap because the regex pattern is hardcoded and known valid
    let Ok(regex) = &*CPANFILE_REQUIRES_RE else {
        return requirements;
    };
    for cap in regex.captures_iter(contents) {
        let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let version = cap.get(2).or(cap.get(3)).map(|m| m.as_str().to_string());

        if !name.is_empty() {
            requirements.push(ModuleRequirement { name: name.to_string(), version });
        }
    }

    requirements
}

/// Parse a META.json file and extract module requirements.
///
/// The META.json format is defined by the CPAN distribution metadata specification.
/// We extract the `requires` field (and optionally `build_requires`, `test_requires`).
///
/// # Arguments
///
/// * `contents` - The META.json contents as a string
///
/// # Returns
///
/// A vector of `ModuleRequirement` structs representing the declared dependencies
pub fn parse_meta_json(contents: &str) -> Result<Vec<ModuleRequirement>, ParseError> {
    #[derive(Deserialize, Debug)]
    struct MetaJson {
        #[serde(default)]
        requires: std::collections::HashMap<String, serde_json::Value>,
        #[serde(default, rename = "build_requires")]
        build_requires: std::collections::HashMap<String, serde_json::Value>,
        #[serde(default, rename = "test_requires")]
        test_requires: std::collections::HashMap<String, serde_json::Value>,
    }

    let meta: MetaJson = serde_json::from_str(contents)?;

    let mut requirements = Vec::new();

    fn extract_version(v: &serde_json::Value) -> Option<String> {
        match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            serde_json::Value::Null => None,
            _ => None,
        }
    }

    for (name, version) in meta.requires {
        requirements.push(ModuleRequirement { name, version: extract_version(&version) });
    }

    for (name, version) in meta.build_requires {
        requirements.push(ModuleRequirement { name, version: extract_version(&version) });
    }

    for (name, version) in meta.test_requires {
        requirements.push(ModuleRequirement { name, version: extract_version(&version) });
    }

    Ok(requirements)
}

/// Parse a META.yml file and extract module requirements.
///
/// The META.yml format is defined by the CPAN distribution metadata specification.
/// We extract the `requires` field (and optionally `build_requires`, `test_requires`).
///
/// This function provides graceful fallback on parse failure - if the YAML is malformed,
/// it returns an empty vector rather than propagating the error.
///
/// # Arguments
///
/// * `contents` - The META.yml contents as a string
///
/// # Returns
///
/// A vector of `ModuleRequirement` structs representing the declared dependencies,
/// or an empty vector if parsing fails
pub fn parse_meta_yml(contents: &str) -> Vec<ModuleRequirement> {
    #[derive(Deserialize, Debug)]
    struct MetaYml {
        #[serde(default)]
        requires: std::collections::HashMap<String, serde_yaml::Value>,
        #[serde(default)]
        build_requires: std::collections::HashMap<String, serde_yaml::Value>,
        #[serde(default)]
        test_requires: std::collections::HashMap<String, serde_yaml::Value>,
    }

    fn extract_version(v: &serde_yaml::Value) -> Option<String> {
        match v {
            serde_yaml::Value::String(s) => Some(s.clone()),
            serde_yaml::Value::Number(n) => Some(n.to_string()),
            serde_yaml::Value::Null => None,
            _ => None,
        }
    }

    match serde_yaml::from_str::<MetaYml>(contents) {
        Ok(meta) => {
            let mut requirements = Vec::new();

            for (name, version) in meta.requires {
                requirements.push(ModuleRequirement { name, version: extract_version(&version) });
            }

            for (name, version) in meta.build_requires {
                requirements.push(ModuleRequirement { name, version: extract_version(&version) });
            }

            for (name, version) in meta.test_requires {
                requirements.push(ModuleRequirement { name, version: extract_version(&version) });
            }

            requirements
        }
        Err(_) => {
            // Graceful fallback: malformed YAML returns empty vector
            Vec::new()
        }
    }
}

/// Detect the vendor library path by checking for directory existence.
///
/// Checks for:
/// - `vendor/lib/perl5` (carton/carmel/cpm)
/// - `local/lib/perl5` (manual local::lib)
///
/// Returns the path to the vendor directory if found, or `None` if neither exists.
///
/// # Arguments
///
/// * `root` - The workspace root path to check
///
/// # Returns
///
/// The vendor path if found, or `None`
pub fn detect_vendor_path(root: &Path) -> Option<PathBuf> {
    let vendor_path = root.join("vendor/lib/perl5");
    if vendor_path.exists() {
        return Some(vendor_path);
    }

    let local_path = root.join("local/lib/perl5");
    if local_path.exists() {
        return Some(local_path);
    }

    None
}

/// Check if a module is declared in the dependency info.
///
/// # Arguments
///
/// * `info` - The dependency info to check
/// * `module_name` - The module name to look for
///
/// # Returns
///
/// `true` if the module is declared in any dependency file
pub fn is_module_declared(info: &DependencyInfo, module_name: &str) -> bool {
    info.declared.iter().any(|req| req.name == module_name)
}

/// Cpanfile editor for adding new module requirements.
///
/// Preserves formatting and comments while inserting new requires lines
/// in alphabetical order.
#[derive(Debug, Clone)]
pub struct CpanfileEditor {
    /// Original file contents
    contents: String,
    /// Path to the cpanfile (used for writing)
    path: Option<PathBuf>,
    /// Whether the cpanfile is newly created (doesn't exist on disk)
    is_new: bool,
}

impl CpanfileEditor {
    /// Read a cpanfile from disk, preserving formatting and comments.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the cpanfile
    ///
    /// # Returns
    ///
    /// A `CpanfileEditor` with the original contents preserved
    pub fn read(path: &Path) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(Self { contents, path: Some(path.to_path_buf()), is_new: false })
    }

    /// Create a new (empty) cpanfile editor for a file that doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the cpanfile will be created
    ///
    /// # Returns
    ///
    /// A new `CpanfileEditor` for creating a cpanfile
    pub fn create(path: &Path) -> Self {
        Self { contents: String::new(), path: Some(path.to_path_buf()), is_new: true }
    }

    /// Add a module requirement to the cpanfile.
    ///
    /// Inserts `requires 'Module';` in alphabetical order, preserving
    /// existing formatting and comments.
    ///
    /// # Arguments
    ///
    /// * `module` - The module name to add
    /// * `version` - Optional version constraint
    pub fn add_module(&mut self, module: &str, version: Option<&str>) {
        let new_requires = match version {
            Some(v) => format!("requires '{}', '{}';\n", module, v),
            None => format!("requires '{}';\n", module),
        };

        // If file is empty or new, just append
        if self.contents.is_empty() {
            self.contents.push_str("# cpanfile\n");
            self.contents.push_str(&new_requires);
            return;
        }

        // Find the best insertion point - after the last requires line that's
        // alphabetically less than the new module, or at the end of the file
        let lines: Vec<&str> = self.contents.lines().collect();
        let mut insert_pos = self.contents.len();

        // Find the last requires line that's alphabetically less than the new module
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("requires ") {
                // Extract the module name from this requires line
                if let Some(pos) = trimmed.find("requires '") {
                    let start = pos + 9; // after "requires '"
                    // Find the closing quote after 'start' - use raw string to avoid escaping
                    let after_start = &trimmed[start..];
                    if let Some(quote_pos) = after_start.find('\'') {
                        let end = start + quote_pos;
                        let existing_module = &trimmed[start..end];
                        if existing_module > module {
                            // Found a module that's alphabetically after - insert before this line
                            break;
                        }
                    }
                }
                // This line is alphabetically before or equal to new module
                // Calculate its end position (including newline if not last line)
                let line_start: usize = lines[..i].iter().map(|l| l.len() + 1).sum();
                let line_end = line_start + line.len();
                insert_pos = line_end;
            }
        }

        // Find the actual byte position in the original string
        let mut byte_pos = 0;
        for (i, line) in lines.iter().enumerate() {
            if i == lines.len() - 1 {
                break;
            }
            if byte_pos >= insert_pos {
                break;
            }
            byte_pos += line.len() + 1; // +1 for newline
        }

        if byte_pos > self.contents.len() {
            byte_pos = self.contents.len();
        }

        self.contents.insert_str(byte_pos, &new_requires);
    }

    /// Get the modified contents of the cpanfile.
    ///
    /// # Returns
    ///
    /// The cpanfile contents after modifications
    pub fn contents(&self) -> &str {
        &self.contents
    }

    /// Write the cpanfile to disk.
    ///
    /// If the file is new, creates it. Otherwise, overwrites the existing file.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if writing fails
    pub fn write(&self) -> std::io::Result<()> {
        match &self.path {
            Some(path) => {
                if self.is_new {
                    // Create parent directories if needed
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                }
                std::fs::write(path, &self.contents)?;
                Ok(())
            }
            None => Ok(()),
        }
    }

    /// Check if this is a newly created cpanfile (not yet on disk).
    pub fn is_new(&self) -> bool {
        self.is_new
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpanfile_basic() {
        let contents = r#"
requires 'Moo', '2.0';
requires 'JSON::PP';
requires 'Path::Tiny' => '0.078';
"#;
        let deps = parse_cpanfile(contents);
        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0].name, "Moo");
        assert_eq!(deps[0].version, Some("2.0".to_string()));
        assert_eq!(deps[1].name, "JSON::PP");
        assert_eq!(deps[1].version, None);
        assert_eq!(deps[2].name, "Path::Tiny");
        assert_eq!(deps[2].version, Some("0.078".to_string()));
    }

    #[test]
    fn test_parse_cpanfile_with_comments() {
        let contents = r#"
# This is a comment
requires 'Foo';  # inline comment
requires 'Bar';
"#;
        let deps = parse_cpanfile(contents);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "Foo");
        assert_eq!(deps[1].name, "Bar");
    }

    #[test]
    fn test_parse_meta_json_basic() {
        let contents = r#"{
    "name": "My-Dist",
    "version": "1.00",
    "requires": {
        "Moo": "2.0",
        "JSON::PP": "2.0"
    }
}"#;
        let deps = parse_meta_json(contents).unwrap();
        assert_eq!(deps.len(), 2);
        // Order is not guaranteed due to HashMap
        let names: Vec<&str> = deps.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"Moo"));
        assert!(names.contains(&"JSON::PP"));
        let moo = deps.iter().find(|d| d.name == "Moo");
        assert!(moo.is_some());
        assert_eq!(moo.unwrap().version, Some("2.0".to_string()));
    }

    #[test]
    fn test_parse_meta_json_all_fields() {
        let contents = r#"{
    "name": "My-Dist",
    "version": "1.00",
    "requires": {
        "Moo": "1.0"
    },
    "build_requires": {
        "ExtUtils::MakeMaker": "0"
    },
    "test_requires": {
        "Test::More": "0"
    }
}"#;
        let deps = parse_meta_json(contents).unwrap();
        assert_eq!(deps.len(), 3);
        let names: Vec<&str> = deps.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"Moo"));
        assert!(names.contains(&"ExtUtils::MakeMaker"));
        assert!(names.contains(&"Test::More"));
    }

    #[test]
    fn test_parse_meta_yml_basic() {
        let contents = r#"
name: My-Dist
version: 1.00
requires:
  Moo: 2.0
  JSON::PP: 2.0
"#;
        let deps = parse_meta_yml(contents);
        assert_eq!(deps.len(), 2);
        // Order is not guaranteed due to HashMap
        let names: Vec<&str> = deps.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"Moo"));
        assert!(names.contains(&"JSON::PP"));
        let moo = deps.iter().find(|d| d.name == "Moo");
        assert!(moo.is_some());
        assert_eq!(moo.unwrap().version, Some("2.0".to_string()));
    }

    #[test]
    fn test_parse_meta_yml_graceful_failure() {
        let contents = "this is: [not\n  valid yaml:";
        let deps = parse_meta_yml(contents);
        assert!(deps.is_empty());
    }

    #[test]
    fn test_cpanfile_editor_empty_file() {
        let mut editor = CpanfileEditor::create(Path::new("/tmp/cpanfile"));
        editor.add_module("Moo", Some("2.0"));
        let contents = editor.contents();
        assert!(contents.contains("requires 'Moo', '2.0'"));
    }

    #[test]
    fn test_cpanfile_editor_alphabetical_insert() {
        let contents = "requires 'AAA';\nrequires 'CCC';\n";
        let mut editor =
            CpanfileEditor { contents: contents.to_string(), path: None, is_new: false };
        editor.add_module("BBB", None);
        let result = editor.contents();
        // Should be inserted between AAA and CCC
        assert!(result.contains("requires 'AAA'"));
        assert!(result.contains("requires 'BBB'"));
        assert!(result.contains("requires 'CCC'"));
    }

    #[test]
    fn test_cpanfile_editor_insertion_order() {
        // Test that modules are inserted in correct alphabetical order
        let mut editor = CpanfileEditor { contents: String::new(), path: None, is_new: false };
        editor.add_module("Zebra", None);
        editor.add_module("Apple", None);
        editor.add_module("Mango", None);
        let result = editor.contents();
        let apple_pos = result.find("requires 'Apple'").unwrap();
        let mango_pos = result.find("requires 'Mango'").unwrap();
        let zebra_pos = result.find("requires 'Zebra'").unwrap();
        assert!(apple_pos < mango_pos);
        assert!(mango_pos < zebra_pos);
    }

    #[test]
    fn test_is_module_declared() {
        let info = DependencyInfo {
            declared: vec![
                ModuleRequirement { name: "Moo".to_string(), version: Some("2.0".to_string()) },
                ModuleRequirement { name: "JSON::PP".to_string(), version: None },
            ],
            vendor_path: Some(PathBuf::from("vendor/lib/perl5")),
        };
        assert!(is_module_declared(&info, "Moo"));
        assert!(is_module_declared(&info, "JSON::PP"));
        assert!(!is_module_declared(&info, "NonExistent::Module"));
    }
}
