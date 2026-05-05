use crate::refactor::workspace_refactor::{FileEdit, TextEdit};
use std::collections::HashSet;
use std::path::Path;

/// Validation failures for refactoring edit plans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditValidationError {
    /// File path escaped configured workspace roots.
    FileOutsideWorkspace { path: String },
    /// Edit has invalid range ordering or bounds.
    InvalidRange { file: String, start: usize, end: usize, len: usize },
    /// Edits overlap within a single file.
    OverlappingRanges { file: String, previous_end: usize, next_start: usize },
    /// Edit boundary falls inside a UTF-8 codepoint.
    NonUtf8Boundary { file: String, offset: usize },
}

/// Validate generated file edits against basic safety invariants.
pub fn validate_file_edits(
    file_edits: &[FileEdit],
    workspace_roots: &[&Path],
    file_text: &dyn Fn(&Path) -> Option<String>,
) -> Result<(), EditValidationError> {
    let mut seen_files = HashSet::new();

    for file_edit in file_edits {
        let canonical = file_edit.file_path.canonicalize().unwrap_or(file_edit.file_path.clone());
        if !workspace_roots.is_empty()
            && !workspace_roots.iter().any(|root| canonical.starts_with(root))
        {
            return Err(EditValidationError::FileOutsideWorkspace {
                path: file_edit.file_path.display().to_string(),
            });
        }

        if !seen_files.insert(canonical.clone()) {
            // Allow multiple entries targeting same file only if callers merge first.
            return Err(EditValidationError::OverlappingRanges {
                file: file_edit.file_path.display().to_string(),
                previous_end: 0,
                next_start: 0,
            });
        }

        let content = file_text(&file_edit.file_path).unwrap_or_default();
        validate_text_edits(&file_edit.edits, &content, &file_edit.file_path)?;
    }

    Ok(())
}

fn validate_text_edits(
    edits: &[TextEdit],
    content: &str,
    path: &Path,
) -> Result<(), EditValidationError> {
    let mut sorted = edits.to_vec();
    sorted.sort_by_key(|e| (e.start, e.end));

    let mut previous_end = 0usize;
    for (idx, edit) in sorted.iter().enumerate() {
        if edit.start > edit.end || edit.end > content.len() {
            return Err(EditValidationError::InvalidRange {
                file: path.display().to_string(),
                start: edit.start,
                end: edit.end,
                len: content.len(),
            });
        }
        if !content.is_char_boundary(edit.start) {
            return Err(EditValidationError::NonUtf8Boundary {
                file: path.display().to_string(),
                offset: edit.start,
            });
        }
        if !content.is_char_boundary(edit.end) {
            return Err(EditValidationError::NonUtf8Boundary {
                file: path.display().to_string(),
                offset: edit.end,
            });
        }
        if idx > 0 && edit.start < previous_end {
            return Err(EditValidationError::OverlappingRanges {
                file: path.display().to_string(),
                previous_end,
                next_start: edit.start,
            });
        }
        previous_end = edit.end;
    }

    Ok(())
}
