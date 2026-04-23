#![warn(missing_docs)]
//! Secure string-literal file path completion.
//!
//! This microcrate isolates bounded filesystem traversal and path sanitization
//! for completion providers that want to offer file suggestions without owning
//! the security policy themselves.

use crate::providers::completion_item::{CompletionItem, CompletionItemKind};

/// Options for configuring the file completion provider.
#[derive(Debug, Clone)]
pub struct FileCompletionOptions {
    /// Whether file completion is enabled.
    pub enabled: bool,
    /// Maximum number of completion items to return.
    pub max_items: usize,
}

impl Default for FileCompletionOptions {
    fn default() -> Self {
        Self { enabled: true, max_items: 50 }
    }
}

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
    }

    completions.sort_by(|left, right| left.label.cmp(&right.label));
    if completions.len() > 50 {
        completions.truncate(50);
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
        let directory_name = entry.file_name().to_string_lossy();
        if matches!(
            directory_name.to_ascii_lowercase().as_str(),
            "docs" | "doc" | "documentation"
        ) {
            ("documentation directory".to_string(), Some("Documentation directory".to_string()))
        } else {
            ("directory".to_string(), Some("Directory".to_string()))
        }
    } else if file_type.is_file() {
        let extension = entry.path().extension().and_then(|ext| ext.to_str()).unwrap_or("");
        let file_name = entry.file_name().to_string_lossy();
        let file_type_desc = match extension.to_ascii_lowercase().as_str() {
            "pl" | "pm" | "t" => "Perl file",
            "rs" => "Rust source file",
            "js" => "JavaScript file",
            "py" => "Python file",
            "txt" => "Text file",
            "md" | "mdx" | "rst" | "adoc" | "asciidoc" | "pod" => "Documentation file",
            "json" => "JSON file",
            "yaml" | "yml" => "YAML file",
            "toml" => "TOML file",
            _ if matches!(
                file_name.to_ascii_lowercase().as_str(),
                "readme"
                    | "readme.md"
                    | "readme.rst"
                    | "changelog"
                    | "changelog.md"
                    | "contributing"
                    | "contributing.md"
                    | "license"
                    | "license.md"
            ) =>
            {
                "Project documentation file"
            }
            _ => "file",
        };
        (file_type_desc.to_string(), None)
    } else {
        ("file".to_string(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::{FileCompletionContext, complete_file_paths};
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::{LazyLock, Mutex},
    };
    use tempfile::tempdir;

    static CWD_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    struct CurrentDirGuard {
        previous: PathBuf,
    }

    impl CurrentDirGuard {
        fn change_to(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
            let previous = std::env::current_dir()?;
            std::env::set_current_dir(path)?;
            Ok(Self { previous })
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.previous);
        }
    }

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn file_completion_labels_are_sorted() -> TestResult {
        let _cwd_guard = CWD_LOCK.lock()?;
        let temp = tempdir()?;
        let _dir_guard = CurrentDirGuard::change_to(temp.path())?;

        fs::create_dir_all("fixtures")?;
        fs::write("fixtures/z-last.pm", "1;")?;
        fs::write("fixtures/a-first.pm", "1;")?;
        fs::create_dir_all("fixtures/m-middle-dir")?;

        let context = FileCompletionContext::new("fixtures/", 0, "fixtures/".len());
        let completions = complete_file_paths(&context, &|| false);
        let labels: Vec<&str> = completions.iter().map(|item| item.label.as_str()).collect();

        assert_eq!(
            labels,
            vec!["fixtures/a-first.pm", "fixtures/m-middle-dir/", "fixtures/z-last.pm"]
        );

        Ok(())
    }

    #[test]
    fn file_completion_marks_docs_as_documentation() -> TestResult {
        let _cwd_guard = CWD_LOCK.lock()?;
        let temp = tempdir()?;
        let _dir_guard = CurrentDirGuard::change_to(temp.path())?;

        fs::create_dir_all("docs")?;
        fs::write("docs/guide.mdx", "# Guide")?;

        let context = FileCompletionContext::new("docs/g", 0, "docs/g".len());
        let completions = complete_file_paths(&context, &|| false);
        let guide = completions
            .iter()
            .find(|item| item.label == "docs/guide.mdx")
            .ok_or("docs/guide.mdx completion missing")?;

        assert_eq!(guide.detail.as_deref(), Some("Documentation file"));
        Ok(())
    }

    #[test]
    fn file_completion_marks_docs_directory() -> TestResult {
        let _cwd_guard = CWD_LOCK.lock()?;
        let temp = tempdir()?;
        let _dir_guard = CurrentDirGuard::change_to(temp.path())?;

        fs::create_dir_all("docs")?;

        let context = FileCompletionContext::new("d", 0, "d".len());
        let completions = complete_file_paths(&context, &|| false);
        let docs_dir = completions
            .iter()
            .find(|item| item.label == "docs/")
            .ok_or("docs/ completion missing")?;

        assert_eq!(docs_dir.detail.as_deref(), Some("documentation directory"));
        assert_eq!(docs_dir.documentation.as_deref(), Some("Documentation directory"));
        Ok(())
    }
}
