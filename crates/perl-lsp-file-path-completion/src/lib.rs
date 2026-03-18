#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]
//! Secure, cancellation-aware file path completion extraction.
//!
//! This microcrate owns one responsibility: turning a path-like prefix into
//! LSP completion items while enforcing path-safety and traversal limits.

use perl_lsp_completion_item::{CompletionItem, CompletionItemKind};

#[cfg(not(target_arch = "wasm32"))]
use perl_path_security::{
    build_completion_path as shared_build_completion_path, is_hidden_or_forbidden_entry_name,
    is_safe_completion_filename, resolve_completion_base_directory, sanitize_completion_path_input,
    split_completion_path_components,
};
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

/// Minimal request context needed to generate file path completions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePathCompletionContext {
    /// Cursor byte position in the source.
    pub position: usize,
    /// Start byte offset for the prefix being replaced.
    pub prefix_start: usize,
    /// Prefix text to complete.
    pub prefix: String,
}

impl FilePathCompletionContext {
    /// Create a new file path completion context.
    #[must_use]
    pub fn new(position: usize, prefix_start: usize, prefix: impl Into<String>) -> Self {
        Self { position, prefix_start, prefix: prefix.into() }
    }
}

/// Bundled callbacks for file path completion operations.
#[cfg(not(target_arch = "wasm32"))]
pub struct FilePathCallbacks<'a> {
    /// Sanitizes and validates input paths for security.
    pub sanitize_path: &'a dyn Fn(&str) -> Option<String>,
    /// Splits path into directory and filename components.
    pub split_path_components: &'a dyn Fn(&str) -> (String, String),
    /// Resolves and validates directory for safe traversal.
    pub resolve_safe_directory: &'a dyn Fn(&str) -> Option<PathBuf>,
    /// Checks if a directory entry should be filtered out.
    pub is_hidden_or_forbidden: &'a dyn Fn(&walkdir::DirEntry) -> bool,
    /// Validates filename for safety.
    pub is_safe_filename: &'a dyn Fn(&str) -> bool,
    /// Builds the completion path string.
    pub build_completion_path: &'a dyn Fn(&str, &str, bool) -> String,
    /// Gets metadata for file completion items.
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

/// Add file path completions using bundled callbacks.
#[cfg(not(target_arch = "wasm32"))]
pub fn add_file_completions_with_callbacks(
    completions: &mut Vec<CompletionItem>,
    context: &FilePathCompletionContext,
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

/// No-op wasm32 variant because filesystem traversal is unavailable there.
#[cfg(target_arch = "wasm32")]
pub fn add_file_completions_with_callbacks(
    completions: &mut Vec<CompletionItem>,
    context: &FilePathCompletionContext,
    _callbacks: &(),
    _is_cancelled: &dyn Fn() -> bool,
) {
    let _ = (completions, context);
}

/// Add file path completions with secure defaults and no cancellation callback.
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub fn add_file_completions(
    completions: &mut Vec<CompletionItem>,
    context: &FilePathCompletionContext,
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

/// No-op wasm32 variant because filesystem traversal is unavailable there.
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub fn add_file_completions(
    completions: &mut Vec<CompletionItem>,
    context: &FilePathCompletionContext,
    _sanitize_path: impl Fn(&str) -> Option<String>,
    _split_path_components: impl Fn(&str) -> (String, String),
    _resolve_safe_directory: impl Fn(&str) -> Option<String>,
    _is_hidden_or_forbidden: impl Fn(&walkdir::DirEntry) -> bool,
    _is_safe_filename: impl Fn(&str) -> bool,
    _build_completion_path: impl Fn(&str, &str, bool) -> String,
    _get_file_completion_metadata: impl Fn(&walkdir::DirEntry) -> (String, Option<String>),
) {
    let _ = (completions, context);
}

/// Add file path completions with cancellation support.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub fn add_file_completions_with_cancellation(
    completions: &mut Vec<CompletionItem>,
    context: &FilePathCompletionContext,
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

    if is_cancelled() {
        return;
    }

    let prefix = context.prefix.trim();
    if prefix.len() > 1024 {
        return;
    }

    let safe_prefix = match sanitize_path(prefix) {
        Some(path) => path,
        None => return,
    };

    let (dir_part, file_part) = split_path_components(&safe_prefix);
    let base_dir = match resolve_safe_directory(&dir_part) {
        Some(dir) => dir,
        None => return,
    };

    let max_results = 50;
    let max_depth = 1;
    let max_entries = 200;

    let mut result_count = 0;
    let mut entries_examined = 0;

    for entry in WalkDir::new(&base_dir)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_hidden_or_forbidden(e))
    {
        if is_cancelled() {
            break;
        }

        entries_examined += 1;
        if entries_examined > max_entries {
            break;
        }

        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        if entry.path() == base_dir {
            continue;
        }

        let file_name = match entry.file_name().to_str() {
            Some(name) => name,
            None => continue,
        };

        if !file_name.starts_with(&file_part) || !is_safe_filename(file_name) {
            continue;
        }

        let completion_path =
            build_completion_path(&dir_part, file_name, entry.file_type().is_dir());
        let (detail, documentation) = get_file_completion_metadata(&entry);

        completions.push(CompletionItem {
            label: completion_path.clone(),
            kind: CompletionItemKind::File,
            detail: Some(detail),
            documentation,
            insert_text: Some(completion_path.clone()),
            sort_text: Some(format!("1_{completion_path}")),
            filter_text: Some(completion_path.clone()),
            additional_edits: Vec::new(),
            text_edit_range: Some((context.prefix_start, context.position)),
        });

        result_count += 1;
        if result_count >= max_results {
            break;
        }
    }
}

/// No-op wasm32 variant because filesystem traversal is unavailable there.
#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
pub fn add_file_completions_with_cancellation(
    completions: &mut Vec<CompletionItem>,
    context: &FilePathCompletionContext,
    _is_cancelled: &dyn Fn() -> bool,
    _sanitize_path: impl Fn(&str) -> Option<String>,
    _split_path_components: impl Fn(&str) -> (String, String),
    _resolve_safe_directory: impl Fn(&str) -> Option<String>,
    _is_hidden_or_forbidden: impl Fn(&walkdir::DirEntry) -> bool,
    _is_safe_filename: impl Fn(&str) -> bool,
    _build_completion_path: impl Fn(&str, &str, bool) -> String,
    _get_file_completion_metadata: impl Fn(&walkdir::DirEntry) -> (String, Option<String>),
) {
    let _ = (completions, context, _is_cancelled);
}

/// Sanitize and validate a file path for security.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn sanitize_path(path: &str) -> Option<String> {
    sanitize_completion_path_input(path)
}

/// Split path into directory and filename components safely.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn split_path_components(path: &str) -> (String, String) {
    split_completion_path_components(path)
}

/// Resolve and validate a directory path for safe traversal.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn resolve_safe_directory(dir_part: &str) -> Option<PathBuf> {
    resolve_completion_base_directory(dir_part)
}

/// Check if a directory entry should be filtered out for security.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn is_hidden_or_forbidden(entry: &walkdir::DirEntry) -> bool {
    let file_name = entry.file_name().to_string_lossy();
    is_hidden_or_forbidden_entry_name(file_name.as_ref())
}

/// Validate a filename for safety.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn is_safe_filename(filename: &str) -> bool {
    is_safe_completion_filename(filename)
}

/// Build the completion path string.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn build_completion_path(dir_part: &str, filename: &str, is_dir: bool) -> String {
    shared_build_completion_path(dir_part, filename, is_dir)
}

/// Get metadata describing a file completion item.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn get_file_completion_metadata(entry: &walkdir::DirEntry) -> (String, Option<String>) {
    let file_type = entry.file_type();

    if file_type.is_dir() {
        ("directory".to_string(), Some("Directory".to_string()))
    } else if file_type.is_file() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Result, anyhow};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDirGuard {
        path: PathBuf,
        old_cwd: PathBuf,
    }

    impl TempDirGuard {
        fn new() -> Result<Self> {
            let nonce =
                SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
            let path = std::env::temp_dir().join(format!("perl_lsp_file_path_completion_{nonce}"));
            let old_cwd = std::env::current_dir()?;
            fs::create_dir_all(&path)?;
            Ok(Self { path, old_cwd })
        }

        fn set_current_dir(&self) -> Result<()> {
            std::env::set_current_dir(&self.path)?;
            Ok(())
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.old_cwd);
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn rejects_unsafe_prefixes() {
        let mut completions = Vec::new();
        let context = FilePathCompletionContext::new(5, 0, "../secret");

        add_file_completions_with_callbacks(
            &mut completions,
            &context,
            &FilePathCallbacks::default(),
            &|| false,
        );

        assert!(completions.is_empty());
    }

    #[test]
    fn completes_matching_files_and_sets_replace_range() -> Result<()> {
        let dir = TempDirGuard::new()?;
        fs::write(dir.path.join("alpha.pm"), "package Alpha;")?;
        fs::write(dir.path.join("beta.pm"), "package Beta;")?;
        fs::create_dir_all(dir.path.join("config"))?;
        dir.set_current_dir()?;

        let mut completions = Vec::new();
        let context = FilePathCompletionContext::new(7, 2, "a");
        add_file_completions_with_callbacks(
            &mut completions,
            &context,
            &FilePathCallbacks::default(),
            &|| false,
        );

        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].label, "alpha.pm");
        assert_eq!(completions[0].text_edit_range, Some((2, 7)));
        assert_eq!(completions[0].kind, CompletionItemKind::File);
        Ok(())
    }

    #[test]
    fn classifies_directory_metadata() -> Result<()> {
        let dir = TempDirGuard::new()?;
        let nested = dir.path.join("nested");
        let file = dir.path.join("notes.txt");
        fs::create_dir_all(&nested)?;
        fs::write(&file, "hello")?;

        let nested_entry = walkdir::WalkDir::new(&dir.path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .find_map(|result| match result {
                Ok(entry) if entry.path() == nested => Some(entry),
                _ => None,
            })
            .ok_or_else(|| anyhow!("missing nested entry"))?;
        let file_entry = walkdir::WalkDir::new(&dir.path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .find_map(|result| match result {
                Ok(entry) if entry.path() == file => Some(entry),
                _ => None,
            })
            .ok_or_else(|| anyhow!("missing file entry"))?;

        let (file_detail, file_doc) = get_file_completion_metadata(&file_entry);
        let (nested_detail, nested_doc) = get_file_completion_metadata(&nested_entry);

        assert_eq!(file_detail, "Text file");
        assert!(file_doc.is_none());
        assert_eq!(nested_detail, "directory");
        assert_eq!(nested_doc.as_deref(), Some("Directory"));
        Ok(())
    }

    #[test]
    fn stops_when_cancelled() -> Result<()> {
        let dir = TempDirGuard::new()?;
        fs::write(dir.path.join("alpha.pm"), "package Alpha;")?;
        dir.set_current_dir()?;

        let mut completions = Vec::new();
        let context = FilePathCompletionContext::new(1, 0, "a");
        add_file_completions_with_callbacks(
            &mut completions,
            &context,
            &FilePathCallbacks::default(),
            &|| true,
        );

        assert!(completions.is_empty());
        Ok(())
    }
}
