//! File path completion with security and performance safeguards

use super::context::CompletionContext;
use super::items::{CompletionItem, CompletionItemKind};
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Component, Path, PathBuf};

/// Add file path completions with comprehensive security and performance safeguards
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn add_file_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    sanitize_path: impl Fn(&str) -> Option<String>,
    split_path_components: impl Fn(&str) -> (String, String),
    resolve_safe_directory: impl Fn(&str) -> Option<PathBuf>,
    is_hidden_or_forbidden: impl Fn(&walkdir::DirEntry) -> bool,
    is_safe_filename: impl Fn(&str) -> bool,
    build_completion_path: impl Fn(&str, &str, bool) -> String,
    get_file_completion_metadata: impl Fn(&walkdir::DirEntry) -> (String, Option<String>),
) {
    add_file_completions_with_cancellation(
        completions,
        context,
        &|| false,
        sanitize_path,
        split_path_components,
        resolve_safe_directory,
        is_hidden_or_forbidden,
        is_safe_filename,
        build_completion_path,
        get_file_completion_metadata,
    )
}

/// Add file path completions with comprehensive security and performance safeguards
#[cfg(target_arch = "wasm32")]
pub(crate) fn add_file_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    _sanitize_path: impl Fn(&str) -> Option<String>,
    _split_path_components: impl Fn(&str) -> (String, String),
    _resolve_safe_directory: impl Fn(&str) -> Option<String>,
    _is_hidden_or_forbidden: impl Fn(&walkdir::DirEntry) -> bool,
    _is_safe_filename: impl Fn(&str) -> bool,
    _build_completion_path: impl Fn(&str, &str, bool) -> String,
    _get_file_completion_metadata: impl Fn(&walkdir::DirEntry) -> (String, Option<String>),
) {
    // File system traversal isn't available on wasm32 targets.
    let _ = (completions, context);
}

/// Add file path completions with cancellation support
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn add_file_completions_with_cancellation(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    is_cancelled: &dyn Fn() -> bool,
    sanitize_path: impl Fn(&str) -> Option<String>,
    split_path_components: impl Fn(&str) -> (String, String),
    resolve_safe_directory: impl Fn(&str) -> Option<PathBuf>,
    is_hidden_or_forbidden: impl Fn(&walkdir::DirEntry) -> bool,
    is_safe_filename: impl Fn(&str) -> bool,
    build_completion_path: impl Fn(&str, &str, bool) -> String,
    get_file_completion_metadata: impl Fn(&walkdir::DirEntry) -> (String, Option<String>),
) {
    use walkdir::WalkDir;

    // Early cancellation check
    if is_cancelled() {
        return;
    }

    let prefix = context.prefix.as_str().trim();

    // Security: Reject dangerous prefixes (but allow empty for current directory completion)
    if prefix.len() > 1024 {
        return;
    }

    // Security: Sanitize and validate the input path
    let safe_prefix = match sanitize_path(prefix) {
        Some(path) => path,
        None => return, // Path was deemed unsafe
    };

    // Split into directory and filename components
    let (dir_part, file_part) = split_path_components(&safe_prefix);

    // Security: Ensure directory is safe to traverse
    let base_dir = match resolve_safe_directory(&dir_part) {
        Some(dir) => dir,
        None => return, // Directory traversal not allowed
    };

    // Performance: Limit the scope of filesystem operations
    let max_results = 50; // Limit number of completions
    let max_depth = 1; // Only traverse immediate directory
    let max_entries = 200; // Limit total entries examined

    let mut result_count = 0;
    let mut entries_examined = 0;

    // Use walkdir for safe, controlled filesystem traversal
    for entry in WalkDir::new(&base_dir)
        .max_depth(max_depth)
        .follow_links(false) // Security: don't follow symlinks
        .into_iter()
        .filter_entry(|e| {
            // Security: Skip hidden files and certain patterns
            !is_hidden_or_forbidden(e)
        })
    {
        // Cancellation check for responsiveness
        if is_cancelled() {
            break;
        }

        // Performance: Limit entries examined
        entries_examined += 1;
        if entries_examined > max_entries {
            break;
        }

        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // Skip entries we can't read
        };

        // Skip the base directory itself
        if entry.path() == base_dir {
            continue;
        }

        let file_name = match entry.file_name().to_str() {
            Some(name) => name,
            None => continue, // Skip non-UTF8 filenames
        };

        // Filter by file part prefix
        if !file_name.starts_with(&file_part) {
            continue;
        }

        // Security: Additional filename validation
        if !is_safe_filename(file_name) {
            continue;
        }

        // Build the completion path
        let completion_path =
            build_completion_path(&dir_part, file_name, entry.file_type().is_dir());

        let (detail, documentation) = get_file_completion_metadata(&entry);

        completions.push(CompletionItem {
            label: completion_path.clone(),
            kind: CompletionItemKind::File,
            detail: Some(detail),
            documentation,
            insert_text: Some(completion_path.clone()),
            sort_text: Some(format!("1_{}", completion_path)),
            filter_text: Some(completion_path.clone()),
            additional_edits: vec![],
            text_edit_range: Some((context.prefix_start, context.position)),
        });

        result_count += 1;
        if result_count >= max_results {
            break;
        }
    }
}

/// Add file path completions with cancellation support
#[cfg(target_arch = "wasm32")]
pub(crate) fn add_file_completions_with_cancellation(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    _is_cancelled: &dyn Fn() -> bool,
    _sanitize_path: impl Fn(&str) -> Option<String>,
    _split_path_components: impl Fn(&str) -> (String, String),
    _resolve_safe_directory: impl Fn(&str) -> Option<String>,
    _is_hidden_or_forbidden: impl Fn(&walkdir::DirEntry) -> bool,
    _is_safe_filename: impl Fn(&str) -> bool,
    _build_completion_path: impl Fn(&str, &str, bool) -> String,
    _get_file_completion_metadata: impl Fn(&walkdir::DirEntry) -> (String, Option<String>),
) {
    // File system traversal isn't available on wasm32 targets.
    let _ = (completions, context, _is_cancelled);
}

/// Sanitize and validate a file path for security
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn sanitize_path(path: &str) -> Option<String> {
    // Handle empty path (current directory completion)
    if path.is_empty() {
        return Some(String::new());
    }

    // Security checks
    if path.contains('\0') {
        return None; // Null bytes not allowed
    }

    // Check for path traversal attempts
    let path_obj = Path::new(path);
    for component in path_obj.components() {
        match component {
            Component::ParentDir => return None, // No .. allowed
            Component::RootDir if path != "/" => return None, // Absolute paths generally not allowed
            Component::Prefix(_) => return None,              // Windows drive letters not allowed
            _ => {}
        }
    }

    // Additional dangerous pattern checks
    if path.contains("../") || path.contains("..\\") || path.starts_with('/') && path != "/" {
        return None;
    }

    // Normalize path separators for cross-platform compatibility
    Some(path.replace('\\', "/"))
}

/// Split path into directory and filename components safely
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn split_path_components(path: &str) -> (String, String) {
    match path.rsplit_once('/') {
        Some((dir, file)) if !dir.is_empty() => (dir.to_string(), file.to_string()),
        _ => (".".to_string(), path.to_string()),
    }
}

/// Resolve and validate a directory path for safe traversal
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn resolve_safe_directory(dir_part: &str) -> Option<PathBuf> {
    let path = Path::new(dir_part);

    // Security: Only allow relative paths and current directory
    if path.is_absolute() && dir_part != "/" {
        return None;
    }

    // For current directory, just return it directly
    if dir_part == "." {
        return Some(Path::new(".").to_path_buf());
    }

    // Convert to canonical path to resolve any remaining issues
    match path.canonicalize() {
        Ok(canonical) => {
            // For tests and scenarios where cwd has changed, be more permissive
            Some(canonical)
        }
        Err(_) => {
            // If canonicalization fails, try the original path if it exists and is safe
            if path.exists() && path.is_dir() { Some(path.to_path_buf()) } else { None }
        }
    }
}

/// Check if a directory entry should be filtered out for security
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn is_hidden_or_forbidden(entry: &walkdir::DirEntry) -> bool {
    let file_name = entry.file_name().to_string_lossy();

    // Skip hidden files (Unix convention)
    if file_name.starts_with('.') && file_name.len() > 1 {
        return true;
    }

    // Skip certain system directories and files
    matches!(
        file_name.as_ref(),
        "node_modules"
            | ".git"
            | ".svn"
            | ".hg"
            | "target"
            | "build"
            | ".cargo"
            | ".rustup"
            | "System Volume Information"
            | "$RECYCLE.BIN"
            | "__pycache__"
            | ".pytest_cache"
            | ".mypy_cache"
    )
}

/// Validate filename for safety
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn is_safe_filename(filename: &str) -> bool {
    // Basic safety checks
    if filename.is_empty() || filename.len() > 255 {
        return false;
    }

    // Check for null bytes or other control characters
    if filename.contains('\0') || filename.chars().any(|c| c.is_control()) {
        return false;
    }

    // Check for Windows reserved names (even on Unix for cross-platform safety)
    let name_upper = filename.to_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    for reserved_name in &reserved {
        if name_upper == *reserved_name || name_upper.starts_with(&format!("{}.", reserved_name)) {
            return false;
        }
    }

    true
}

/// Build the completion path string
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn build_completion_path(dir_part: &str, filename: &str, is_dir: bool) -> String {
    let mut path = if dir_part == "." {
        filename.to_string()
    } else {
        format!("{}/{}", dir_part.trim_end_matches('/'), filename)
    };

    // Add trailing slash for directories
    if is_dir {
        path.push('/');
    }

    path
}

/// Get metadata for file completion item
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn get_file_completion_metadata(entry: &walkdir::DirEntry) -> (String, Option<String>) {
    let file_type = entry.file_type();

    if file_type.is_dir() {
        ("directory".to_string(), Some("Directory".to_string()))
    } else if file_type.is_file() {
        // Try to provide helpful information about file type
        let extension = entry.path().extension().and_then(|ext| ext.to_str()).unwrap_or("");

        let file_type_desc = match extension.to_lowercase().as_str() {
            "pl" | "pm" | "t" => "Perl file",
            "rs" => "Rust source file",
            "js" => "JavaScript file",
            "py" => "Python file",
            "txt" => "Text file",
            "md" => "Markdown file",
            "json" => "JSON file",
            "yaml" | "yml" => "YAML file",
            "toml" => "TOML file",
            _ => "file",
        };

        (file_type_desc.to_string(), None)
    } else {
        ("file".to_string(), None)
    }
}
