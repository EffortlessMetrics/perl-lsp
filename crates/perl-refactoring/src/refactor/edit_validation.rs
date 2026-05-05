use std::path::Path;

use crate::refactor::workspace_refactor::FileEdit;

#[derive(Debug)]
pub enum EditValidationError {
    InvalidRange { file: String, start: usize, end: usize },
    OverlappingRanges { file: String, prior_end: usize, next_start: usize },
    InvalidUtf8Boundary { file: String, start: usize, end: usize },
}

impl std::fmt::Display for EditValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRange { file, start, end } => {
                write!(f, "edit range is invalid for file {file}: start={start}, end={end}")
            }
            Self::OverlappingRanges { file, prior_end, next_start } => write!(
                f,
                "edit ranges overlap in file {file}: prior_end={prior_end}, next_start={next_start}"
            ),
            Self::InvalidUtf8Boundary { file, start, end } => write!(
                f,
                "edit range is not aligned to UTF-8 boundaries in file {file}: start={start}, end={end}"
            ),
        }
    }
}

impl std::error::Error for EditValidationError {}

pub fn validate_file_edits<F>(file_edits: &[FileEdit], mut get_contents: F) -> Result<(), EditValidationError>
where
    F: FnMut(&Path) -> Option<String>,
{
    for file_edit in file_edits {
        let file = file_edit.file_path.display().to_string();
        let contents = get_contents(&file_edit.file_path).unwrap_or_default();
        let mut ordered_edits = file_edit.edits.clone();
        ordered_edits.sort_by_key(|edit| (edit.start, edit.end));

        let mut prior_end = 0usize;
        let mut has_prior = false;

        for edit in ordered_edits {
            if edit.start > edit.end || edit.end > contents.len() {
                return Err(EditValidationError::InvalidRange {
                    file,
                    start: edit.start,
                    end: edit.end,
                });
            }

            if !contents.is_char_boundary(edit.start) || !contents.is_char_boundary(edit.end) {
                return Err(EditValidationError::InvalidUtf8Boundary {
                    file,
                    start: edit.start,
                    end: edit.end,
                });
            }

            if has_prior && edit.start < prior_end {
                return Err(EditValidationError::OverlappingRanges {
                    file,
                    prior_end,
                    next_start: edit.start,
                });
            }

            prior_end = edit.end;
            has_prior = true;
        }
    }

    Ok(())
}
