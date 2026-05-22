use serde::{Deserialize, Serialize};

/// Zero-based text position using UTF-16 code units, matching LSP positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextPosition {
    /// Zero-based line.
    pub line: u32,
    /// Zero-based UTF-16 character offset.
    pub character: u32,
}

impl TextPosition {
    /// Create a text position.
    #[must_use]
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

/// Text range using UTF-16 positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRange {
    /// Inclusive start position.
    pub start: TextPosition,
    /// Exclusive end position.
    pub end: TextPosition,
}

impl TextRange {
    /// Create a text range.
    #[must_use]
    pub fn new(start: TextPosition, end: TextPosition) -> Self {
        Self { start, end }
    }

    /// Create a range that covers a complete source document.
    #[must_use]
    pub fn whole_document(source: &str) -> Self {
        let lines: Vec<&str> = source.lines().collect();
        let last_line = lines.len().saturating_sub(1);
        let last_character = lines.get(last_line).map_or(0, |line| utf16_len(line) as u32);

        Self {
            start: TextPosition::new(0, 0),
            end: TextPosition::new(last_line as u32, last_character),
        }
    }
}

/// Text edit produced by the native formatter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEdit {
    /// Range to replace.
    pub range: TextRange,
    /// Replacement text.
    pub new_text: String,
}

impl TextEdit {
    /// Create a text edit.
    #[must_use]
    pub fn new(range: TextRange, new_text: impl Into<String>) -> Self {
        Self { range, new_text: new_text.into() }
    }
}

/// Formatter diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FormatDiagnosticSeverity {
    /// Informational diagnostic.
    Info,
    /// Warning diagnostic.
    Warning,
    /// Error diagnostic.
    Error,
}

/// Diagnostic produced while deciding whether formatting is safe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatDiagnostic {
    /// Stable diagnostic code.
    pub code: String,
    /// Diagnostic severity.
    pub severity: FormatDiagnosticSeverity,
    /// Optional source range for the diagnostic.
    pub range: Option<TextRange>,
    /// Human-readable message.
    pub message: String,
}

impl FormatDiagnostic {
    /// Create a formatter diagnostic.
    #[must_use]
    pub fn new(
        code: impl Into<String>,
        severity: FormatDiagnosticSeverity,
        range: Option<TextRange>,
        message: impl Into<String>,
    ) -> Self {
        Self { code: code.into(), severity, range, message: message.into() }
    }
}

/// Structured native formatting result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatResult {
    /// Full formatted document text.
    pub formatted: String,
    /// Text edits needed to apply formatting.
    pub edits: Vec<TextEdit>,
    /// Whether formatting produced a content change.
    pub changed: bool,
    /// Diagnostics produced by the formatter.
    pub diagnostics: Vec<FormatDiagnostic>,
}

impl FormatResult {
    /// Build an unchanged formatting result.
    #[must_use]
    pub fn unchanged(source: impl Into<String>) -> Self {
        Self {
            formatted: source.into(),
            edits: Vec::new(),
            changed: false,
            diagnostics: Vec::new(),
        }
    }

    /// Build a whole-document replacement result.
    #[must_use]
    pub fn replace_document(source: &str, formatted: impl Into<String>) -> Self {
        let formatted = formatted.into();
        if formatted == source {
            return Self::unchanged(formatted);
        }

        Self {
            formatted: formatted.clone(),
            edits: vec![TextEdit::new(TextRange::whole_document(source), formatted)],
            changed: true,
            diagnostics: Vec::new(),
        }
    }

    /// Build an unsafe-to-format result with no edits.
    #[must_use]
    pub fn unsafe_to_format(
        source: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            formatted: source.into(),
            edits: Vec::new(),
            changed: false,
            diagnostics: vec![FormatDiagnostic::new(
                code,
                FormatDiagnosticSeverity::Warning,
                None,
                message,
            )],
        }
    }
}

pub(super) fn utf16_len(s: &str) -> usize {
    s.chars().map(|ch| if ch as u32 >= 0x10000 { 2 } else { 1 }).sum()
}

impl TextRange {
    pub(super) fn at_byte_offset(source: &str, offset: usize) -> Self {
        let clamped = offset.min(source.len());
        let mut line = 0_u32;
        let mut line_start = 0_usize;

        for (idx, ch) in source.char_indices() {
            if idx >= clamped {
                break;
            }
            if ch == '\n' {
                line += 1;
                line_start = idx + ch.len_utf8();
            }
        }

        let character = utf16_len(&source[line_start..clamped]) as u32;
        let position = TextPosition::new(line, character);
        Self::new(position, position)
    }
}
