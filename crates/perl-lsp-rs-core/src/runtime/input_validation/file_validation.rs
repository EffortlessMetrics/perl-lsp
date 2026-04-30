use crate::runtime::input_validation::constants::{
    ALLOWED_EXTENSIONS, MAX_LINE_LENGTH, MAX_PATH_LENGTH, SUSPICIOUS_PATTERNS,
};
use crate::runtime::limits::max_file_size_bytes as limits_max_file_size_bytes;
use anyhow::{Result, anyhow};
use perl_parser_core::path_security::validate_workspace_path;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Validates and sanitizes a file path to prevent path traversal attacks.
pub fn validate_file_path<P: AsRef<Path>>(path: P, workspace_root: &Path) -> Result<PathBuf> {
    let path = path.as_ref();

    if path.to_string_lossy().len() > MAX_PATH_LENGTH {
        return Err(anyhow!("Path too long: {}", path.display()));
    }

    let validated = validate_workspace_path(path, workspace_root)
        .map_err(|error| anyhow!("Invalid workspace path {}: {error}", path.display()))?;

    if let Some(extension) = validated.extension().and_then(OsStr::to_str)
        && !ALLOWED_EXTENSIONS.contains(&extension)
    {
        return Err(anyhow!(
            "File extension '{}' not allowed. Allowed: {:?}",
            extension,
            ALLOWED_EXTENSIONS
        ));
    }

    Ok(validated)
}

/// Validates file content before parsing to prevent resource exhaustion.
pub fn validate_file_content(content: &str, file_path: &Path) -> Result<()> {
    let max_file_size = limits_max_file_size_bytes();
    if content.len() > max_file_size {
        return Err(anyhow!(
            "File {} too large: {} bytes (max: {} bytes) â€” adjust perl.limits.maxFileSizeBytes to increase",
            file_path.display(),
            content.len(),
            max_file_size
        ));
    }

    if content.contains('\0') {
        return Err(anyhow!("File {} contains null bytes", file_path.display()));
    }

    for (index, line) in content.lines().enumerate() {
        if line.len() > MAX_LINE_LENGTH {
            return Err(anyhow!(
                "Line {} in file {} is too long: {} characters",
                index + 1,
                file_path.display(),
                line.len()
            ));
        }
    }

    let lowercase = content.to_lowercase();
    for pattern in SUSPICIOUS_PATTERNS {
        if lowercase.contains(pattern) {
            return Err(anyhow!(
                "File {} contains suspicious pattern: {}",
                file_path.display(),
                pattern
            ));
        }
    }

    Ok(())
}
