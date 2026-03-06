//! Cross-platform utilities for Perl path resolution and environment setup.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

#[cfg(windows)]
const PATH_SEPARATOR: char = ';';
#[cfg(not(windows))]
const PATH_SEPARATOR: char = ':';

#[cfg(windows)]
const PERL_EXECUTABLE: &str = "perl.exe";
#[cfg(not(windows))]
const PERL_EXECUTABLE: &str = "perl";

/// Resolve the perl binary path on the current platform.
pub fn resolve_perl_path() -> Result<PathBuf> {
    let path_env = env::var("PATH").context("PATH environment variable not set")?;
    resolve_perl_path_from_path_env(&path_env)
}

fn resolve_perl_path_from_path_env(path_env: &str) -> Result<PathBuf> {
    for path_dir in path_env.split(PATH_SEPARATOR) {
        let perl_path = PathBuf::from(path_dir).join(PERL_EXECUTABLE);
        if perl_path.exists() && perl_path.is_file() {
            return Ok(perl_path);
        }
    }

    anyhow::bail!("perl binary not found on PATH. Please install Perl or add it to PATH.")
}

/// Normalize a file path for cross-platform compatibility.
pub fn normalize_path(path: &std::path::Path) -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        if let Some(path_str) = path.to_str()
            && path_str.starts_with("/mnt/")
            && path_str.len() > 6
        {
            let drive_letter = &path_str[5..6];
            let rest = &path_str[6..];
            let windows_path =
                format!("{}:{}", drive_letter.to_uppercase(), rest.replace('/', "\\"));
            return PathBuf::from(windows_path);
        }
    }

    #[cfg(windows)]
    {
        if let Some(path_str) = path.to_str() {
            if path_str.len() >= 2
                && path_str.chars().nth(1) == Some(':')
                && let Some(first_char) = path_str.chars().next()
            {
                let drive_letter = first_char.to_uppercase();
                let rest = &path_str[1..];
                return PathBuf::from(format!("{}{}", drive_letter, rest));
            }

            if path_str.starts_with("\\\\") {
                return path.to_path_buf();
            }
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(canonical) = path.canonicalize() {
            return canonical;
        }
    }

    path.to_path_buf()
}

/// Setup environment variables for Perl execution.
pub fn setup_environment(include_paths: &[PathBuf]) -> HashMap<String, String> {
    let mut env = HashMap::new();

    if !include_paths.is_empty() {
        let perl5lib = include_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(&PATH_SEPARATOR.to_string());

        env.insert("PERL5LIB".to_string(), perl5lib);
    }

    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_perl_path() {
        if let Ok(path) = resolve_perl_path() {
            assert!(path.exists());
            assert!(path.is_file());
        }
    }

    #[test]
    fn test_normalize_path_basic() {
        let normalized = normalize_path(&PathBuf::from("script.pl"));
        assert!(!normalized.as_os_str().is_empty());
    }

    #[test]
    fn test_setup_environment_empty() {
        let env = setup_environment(&[]);
        assert!(!env.contains_key("PERL5LIB"));
    }

    #[test]
    fn test_setup_environment_with_paths() {
        let env =
            setup_environment(&[PathBuf::from("/workspace/lib"), PathBuf::from("/custom/lib")]);
        assert!(env.contains_key("PERL5LIB"));
    }
}
