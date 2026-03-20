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

pub(crate) fn resolve_perl_path_from_path_env(path_env: &str) -> Result<PathBuf> {
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

    #[test]
    fn resolve_from_path_env_finds_perl_in_first_dir() {
        use std::fs;
        let tempdir = tempfile::tempdir().unwrap();
        let bin = tempdir.path().join(PERL_EXECUTABLE);
        fs::write(&bin, "").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&bin).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&bin, perms).unwrap();
        }
        let path_str = tempdir.path().to_string_lossy().to_string();
        let result = resolve_perl_path_from_path_env(&path_str);
        assert!(result.is_ok(), "expected perl found, got: {:?}", result);
        assert_eq!(result.unwrap(), bin);
    }

    #[test]
    fn resolve_from_path_env_empty_path_returns_error() {
        let result = resolve_perl_path_from_path_env("");
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("perl") || msg.contains("PATH"),
            "error should mention perl/PATH: {msg}"
        );
    }

    #[test]
    fn resolve_from_path_env_no_perl_on_path_returns_error() {
        let tempdir = tempfile::tempdir().unwrap();
        let path_str = tempdir.path().to_string_lossy().to_string();
        let result = resolve_perl_path_from_path_env(&path_str);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn normalize_path_wsl_mnt_translated_to_windows_style() {
        let wsl_path = std::path::Path::new("/mnt/c/Users/user/script.pl");
        let normalized = normalize_path(wsl_path);
        let s = normalized.to_string_lossy();
        assert!(
            s.starts_with("C:\\") || s.starts_with("C:/"),
            "expected Windows-style path, got: {s}"
        );
        assert!(s.contains("Users"), "path content preserved: {s}");
    }

    #[test]
    fn normalize_path_non_wsl_unix_path_unchanged_on_linux() {
        let path = std::path::Path::new("/usr/local/bin/perl");
        let normalized = normalize_path(path);
        assert!(
            !normalized.to_string_lossy().contains('\\'),
            "non-WSL path should not be Windows-escaped"
        );
    }
}
