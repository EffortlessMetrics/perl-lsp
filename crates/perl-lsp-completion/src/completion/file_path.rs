//! File path completion with security and performance safeguards
//!
//! This module provides secure file path completion for string literals in Perl code.
//! It implements comprehensive security measures to prevent path traversal attacks
//! while providing responsive, cancellation-aware filesystem traversal.
//!
//! # Security Model
//!
//! The completion system uses a defense-in-depth approach:
//!
//! 1. **Input sanitization**: Rejects null bytes, path traversal patterns (`../`),
//!    absolute paths, and Windows drive letters
//! 2. **Filesystem isolation**: Only traverses relative paths in safe directories
//! 3. **Entry filtering**: Skips hidden files, system directories, and unsafe filenames
//! 4. **Resource limits**: Caps traversal depth, entry count, and result count
//!
//! # Builder Pattern
//!
//! For cleaner API usage, callbacks can be bundled into a [`FilePathCallbacks`] struct:
//!
//! ```ignore
//! use file_path::{FilePathCallbacks, add_file_completions_with_callbacks};
//!
//! // Use default secure callbacks
//! let callbacks = FilePathCallbacks::default();
//! add_file_completions_with_callbacks(&mut completions, &context, &callbacks, &|| false);
//!
//! // Or customize specific callbacks
//! let callbacks = FilePathCallbacks::default()
//!     .with_is_safe_filename(|name| name.ends_with(".pl"));
//! ```
//!
//! # Performance Characteristics
//!
//! - **Max traversal depth**: 1 directory level
//! - **Max results**: 50 completions
//! - **Max entries examined**: 200 filesystem entries
//! - **Symlink following**: Disabled for security

use super::context::CompletionContext;
use super::items::{CompletionItem, CompletionItemKind};
#[cfg(not(target_arch = "wasm32"))]
use perl_path_security::{
    build_completion_path as shared_build_completion_path, is_hidden_or_forbidden_entry_name,
    is_safe_completion_filename, resolve_completion_base_directory, sanitize_completion_path_input,
    split_completion_path_components,
};
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

/// Bundled callbacks for file path completion operations
///
/// This struct groups all security and filesystem callbacks into a single
/// unit, reducing function argument counts and enabling builder-style
/// configuration. Use [`FilePathCallbacks::default()`] for secure defaults.
///
/// # Example
///
/// ```ignore
/// let callbacks = FilePathCallbacks::default();
/// // Or with customization:
/// let callbacks = FilePathCallbacks::default()
///     .with_is_safe_filename(|name| !name.starts_with('.'));
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub struct FilePathCallbacks<'a> {
    /// Sanitizes and validates input paths for security
    pub sanitize_path: &'a dyn Fn(&str) -> Option<String>,
    /// Splits path into directory and filename components
    pub split_path_components: &'a dyn Fn(&str) -> (String, String),
    /// Resolves and validates directory for safe traversal
    pub resolve_safe_directory: &'a dyn Fn(&str) -> Option<PathBuf>,
    /// Checks if a directory entry should be filtered out
    pub is_hidden_or_forbidden: &'a dyn Fn(&walkdir::DirEntry) -> bool,
    /// Validates filename for safety
    pub is_safe_filename: &'a dyn Fn(&str) -> bool,
    /// Builds the completion path string
    pub build_completion_path: &'a dyn Fn(&str, &str, bool) -> String,
    /// Gets metadata for file completion item
    pub get_file_completion_metadata: &'a dyn Fn(&walkdir::DirEntry) -> (String, Option<String>),
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for FilePathCallbacks<'_> {
    fn default() -> Self {
        Self {
            sanitize_path: &sanitize_path,
            split_path_components: &split_path_components,
            resolve_safe_directory: &resolve_safe_directory,
            is_hidden_or_forbidden: &is_hidden_or_forbidden,
            is_safe_filename: &is_safe_filename,
            build_completion_path: &build_completion_path,
            get_file_completion_metadata: &get_file_completion_metadata,
        }
    }
}

/// Add file path completions using bundled callbacks
///
/// This is the preferred API for file path completion, using the builder pattern
/// to reduce argument count and improve readability.
///
/// # Arguments
///
/// * `completions` - Output vector for completion items
/// * `context` - Completion context with prefix and position information
/// * `callbacks` - Bundled security and filesystem callbacks
/// * `is_cancelled` - Cancellation check callback for responsive editing
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn add_file_completions_with_callbacks(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    callbacks: &FilePathCallbacks<'_>,
    is_cancelled: &dyn Fn() -> bool,
) {
    add_file_completions_with_cancellation(
        completions,
        context,
        is_cancelled,
        callbacks.sanitize_path,
        callbacks.split_path_components,
        callbacks.resolve_safe_directory,
        callbacks.is_hidden_or_forbidden,
        callbacks.is_safe_filename,
        callbacks.build_completion_path,
        callbacks.get_file_completion_metadata,
    )
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn add_file_completions_with_callbacks(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    _callbacks: &(),
    _is_cancelled: &dyn Fn() -> bool,
) {
    // File system traversal isn't available on wasm32 targets.
    let _ = (completions, context);
}

/// Add file path completions with comprehensive security and performance safeguards
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)] // Kept for API completeness; callers may use _with_cancellation directly
#[allow(clippy::too_many_arguments)] // Intentional: dependency injection for security callbacks
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
#[allow(dead_code)] // Kept for API completeness; callers may use _with_cancellation directly
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
#[allow(clippy::too_many_arguments)] // Intentional: dependency injection for security callbacks
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
    sanitize_completion_path_input(path)
}

/// Split path into directory and filename components safely
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn split_path_components(path: &str) -> (String, String) {
    split_completion_path_components(path)
}

/// Resolve and validate a directory path for safe traversal
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn resolve_safe_directory(dir_part: &str) -> Option<PathBuf> {
    resolve_completion_base_directory(dir_part)
}

/// Check if a directory entry should be filtered out for security
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn is_hidden_or_forbidden(entry: &walkdir::DirEntry) -> bool {
    let file_name = entry.file_name().to_string_lossy();
    is_hidden_or_forbidden_entry_name(file_name.as_ref())
}

/// Validate filename for safety
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn is_safe_filename(filename: &str) -> bool {
    is_safe_completion_filename(filename)
}

/// Build the completion path string
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn build_completion_path(dir_part: &str, filename: &str, is_dir: bool) -> String {
    shared_build_completion_path(dir_part, filename, is_dir)
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
