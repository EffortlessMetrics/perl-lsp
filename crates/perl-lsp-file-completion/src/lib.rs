#![warn(missing_docs)]
//! Secure string-literal file path completion.
//!
//! This microcrate isolates bounded filesystem traversal and path sanitization
//! for completion providers that want to offer file suggestions without owning
//! the security policy themselves.

use perl_lsp_completion_item::{CompletionItem, CompletionItemKind};

/// Minimal request context for file-path completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileCompletionContext {
    /// The raw path prefix already typed by the user.
    pub prefix: String,
    /// Byte offset where the prefix starts.
    pub prefix_start: usize,
    /// Current byte offset of the cursor.
    pub position: usize,
}

impl FileCompletionContext {
    /// Create a new file completion context.
    #[must_use]
    pub fn new(prefix: impl Into<String>, prefix_start: usize, position: usize) -> Self {
        Self { prefix: prefix.into(), prefix_start, position }
    }
}

/// Produce secure file-path completion items.
#[must_use]
#[cfg(not(target_arch = "wasm32"))]
pub fn complete_file_paths(
    context: &FileCompletionContext,
    is_cancelled: &dyn Fn() -> bool,
) -> Vec<CompletionItem> {
    use perl_parser_core::path_security::{
        build_completion_path, is_hidden_or_forbidden_entry_name, is_safe_completion_filename,
        resolve_completion_base_directory, sanitize_completion_path_input,
        split_completion_path_components,
    };
    use walkdir::WalkDir;

    if is_cancelled() {
        return Vec::new();
    }

    let prefix = context.prefix.trim();
    if prefix.len() > 1024 {
        return Vec::new();
    }

    let Some(safe_prefix) = sanitize_completion_path_input(prefix) else {
        return Vec::new();
    };

    let (dir_part, file_part) = split_completion_path_components(&safe_prefix);
    let Some(base_dir) = resolve_completion_base_directory(&dir_part) else {
        return Vec::new();
    };

    let mut completions = Vec::new();
    let mut entries_examined = 0usize;

    for entry in
        WalkDir::new(&base_dir).max_depth(1).follow_links(false).into_iter().filter_entry(|entry| {
            !is_hidden_or_forbidden_entry_name(entry.file_name().to_string_lossy().as_ref())
        })
    {
        if is_cancelled() {
            break;
        }

        entries_examined += 1;
        if entries_examined > 200 {
            break;
        }

        let Ok(entry) = entry else {
            continue;
        };

        if entry.path() == base_dir {
            continue;
        }

        let Some(file_name) = entry.file_name().to_str() else {
            continue;
        };

        if !file_name.starts_with(&file_part) || !is_safe_completion_filename(file_name) {
            continue;
        }

        let completion_path =
            build_completion_path(&dir_part, file_name, entry.file_type().is_dir());
        let (detail, documentation) = file_completion_metadata(&entry);
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
            commit_characters: None,
        });

        if completions.len() >= 50 {
            break;
        }
    }

    completions
}

/// Produce secure file-path completion items.
#[must_use]
#[cfg(target_arch = "wasm32")]
pub fn complete_file_paths(
    _context: &FileCompletionContext,
    _is_cancelled: &dyn Fn() -> bool,
) -> Vec<CompletionItem> {
    Vec::new()
}

#[cfg(not(target_arch = "wasm32"))]
fn file_completion_metadata(entry: &walkdir::DirEntry) -> (String, Option<String>) {
    let file_type = entry.file_type();
    if file_type.is_dir() {
        ("directory".to_string(), Some("Directory".to_string()))
    } else if file_type.is_file() {
        let extension = entry.path().extension().and_then(|ext| ext.to_str()).unwrap_or("");
        let file_type_desc = match extension.to_ascii_lowercase().as_str() {
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
