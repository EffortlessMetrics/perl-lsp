//! Shared formatting types for Perl LSP integrations.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use serde::{Deserialize, Serialize};

/// Text edit for formatting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatTextEdit {
    /// The range to replace.
    pub range: FormatRange,
    /// The new text.
    #[serde(rename = "newText")]
    pub new_text: String,
}

/// Position in a document (UTF-16 based).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatPosition {
    /// Line position (0-based).
    pub line: u32,
    /// Character position (UTF-16, 0-based).
    pub character: u32,
}

impl FormatPosition {
    /// Create a new position.
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

/// Range in a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatRange {
    /// Start position.
    pub start: FormatPosition,
    /// End position.
    pub end: FormatPosition,
}

/// Count the number of UTF-16 code units in `s`.
///
/// LSP positions use UTF-16 code units (see Language Server Protocol spec §3.1).
/// Characters in the Basic Multilingual Plane (U+0000–U+FFFF) count as 1 unit;
/// supplementary-plane characters (U+10000 and above) count as 2 units.
fn utf16_len(s: &str) -> usize {
    s.chars().map(|c| if c as u32 >= 0x10000 { 2 } else { 1 }).sum()
}

impl FormatRange {
    /// Create a range covering the entire document.
    pub fn whole_document(content: &str) -> Self {
        let lines: Vec<&str> = content.lines().collect();
        let last_line = if lines.is_empty() { 0 } else { (lines.len() - 1) as u32 };

        FormatRange {
            start: FormatPosition { line: 0, character: 0 },
            end: FormatPosition {
                line: last_line,
                character: lines
                    .get(last_line as usize)
                    .map(|line| utf16_len(line) as u32)
                    .unwrap_or(0),
            },
        }
    }

    /// Create a new range from positions.
    pub fn new(start: FormatPosition, end: FormatPosition) -> Self {
        Self { start, end }
    }
}

/// Formatting options.
#[derive(Debug, Clone, Deserialize)]
pub struct FormattingOptions {
    /// Size of a tab in spaces.
    #[serde(rename = "tabSize")]
    pub tab_size: u32,
    /// Prefer spaces over tabs.
    #[serde(rename = "insertSpaces")]
    pub insert_spaces: bool,
    /// Trim trailing whitespace on a line.
    #[serde(rename = "trimTrailingWhitespace")]
    pub trim_trailing_whitespace: Option<bool>,
    /// Insert a newline character at the end of the file.
    #[serde(rename = "insertFinalNewline")]
    pub insert_final_newline: Option<bool>,
    /// Trim all newlines after the final newline at the end of the file.
    #[serde(rename = "trimFinalNewlines")]
    pub trim_final_newlines: Option<bool>,
}

/// Formatted document result.
#[derive(Debug, Clone)]
pub struct FormattedDocument {
    /// The formatted text.
    pub text: String,
    /// Text edits to apply formatting.
    pub edits: Vec<FormatTextEdit>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatting_options() {
        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: Some(true),
            insert_final_newline: Some(true),
            trim_final_newlines: Some(true),
        };

        assert_eq!(options.tab_size, 4);
        assert!(options.insert_spaces);
    }

    #[test]
    fn test_format_position() {
        let position = FormatPosition::new(5, 10);
        assert_eq!(position.line, 5);
        assert_eq!(position.character, 10);
    }

    #[test]
    fn test_format_range() {
        let start = FormatPosition::new(0, 0);
        let end = FormatPosition::new(10, 20);
        let range = FormatRange::new(start, end);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.end.line, 10);
    }
}
