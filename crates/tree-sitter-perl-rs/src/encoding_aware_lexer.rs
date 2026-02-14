//! Encoding-aware lexer for handling mid-file encoding changes
//!
//! This module tracks encoding pragmas and ensures correct heredoc
//! delimiter matching across different character encodings.

use encoding_rs::{Encoding, UTF_8, WINDOWS_1252};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
pub struct EncodingContext {
    /// Current encoding at this point in the file
    current_encoding: &'static Encoding,
    /// Stack of encoding changes with line numbers
    encoding_stack: Vec<EncodingChange>,
    /// Cached normalizations for performance
    normalization_cache: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct EncodingChange {
    pub encoding: &'static Encoding,
    pub line_number: usize,
    pub pragma_type: PragmaType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PragmaType {
    UseEncoding(String), // use encoding 'latin1';
    UseUtf8,             // use utf8;
    NoUtf8,              // no utf8;
    UseLocale,           // use locale;
    UseBytesIO,          // use bytes ':io';
}

#[derive(Debug)]
pub struct EncodingAwareLexer {
    context: EncodingContext,
    warnings: Vec<EncodingWarning>,
}

#[derive(Debug, Clone)]
pub struct EncodingWarning {
    pub line: usize,
    pub message: String,
    pub severity: WarningSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WarningSeverity {
    Info,
    Warning,
    Error,
}

// Patterns for encoding-related pragmas
static ENCODING_PRAGMA: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(r#"(?m)^\s*use\s+encoding\s+['"]([\w\-]+)['"]"#) {
        Ok(re) => re,
        Err(_) => unreachable!("ENCODING_PRAGMA failed to compile"),
    }
});

static UTF8_PRAGMA: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(r"(?m)^\s*(use|no)\s+utf8\s*;") {
        Ok(re) => re,
        Err(_) => unreachable!("UTF8_PRAGMA failed to compile"),
    }
});

static LOCALE_PRAGMA: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(r"(?m)^\s*use\s+locale\s*;") {
        Ok(re) => re,
        Err(_) => unreachable!("LOCALE_PRAGMA failed to compile"),
    }
});

impl Default for EncodingAwareLexer {
    fn default() -> Self {
        Self::new()
    }
}

impl EncodingAwareLexer {
    pub fn new() -> Self {
        Self {
            context: EncodingContext {
                current_encoding: UTF_8,
                encoding_stack: Vec::new(),
                normalization_cache: HashMap::new(),
            },
            warnings: Vec::new(),
        }
    }

    /// Scan source code for encoding pragmas
    pub fn scan_encoding_pragmas(&mut self, source: &str) {
        let mut line_num = 0;

        for line in source.lines() {
            line_num += 1;

            // Check for 'use encoding'
            if let Some(caps) = ENCODING_PRAGMA.captures(line)
                && let Some(encoding_name) = caps.get(1)
            {
                self.handle_encoding_pragma(encoding_name.as_str(), line_num);
            }

            // Check for use/no utf8
            if let Some(caps) = UTF8_PRAGMA.captures(line)
                && let Some(m) = caps.get(1)
            {
                let is_use = m.as_str() == "use";
                self.handle_utf8_pragma(is_use, line_num);
            }

            // Check for use locale
            if LOCALE_PRAGMA.is_match(line) {
                self.handle_locale_pragma(line_num);
            }
        }
    }

    /// Handle 'use encoding' pragma
    fn handle_encoding_pragma(&mut self, encoding_name: &str, line: usize) {
        let encoding = match encoding_name.to_lowercase().as_str() {
            "utf8" | "utf-8" => UTF_8,
            "latin1" | "iso-8859-1" => WINDOWS_1252, // Close enough
            "ascii" => UTF_8,                        // ASCII is a subset of UTF-8
            "cp1252" | "windows-1252" => WINDOWS_1252,
            _ => {
                self.warnings.push(EncodingWarning {
                    line,
                    message: format!(
                        "Unsupported encoding '{}', defaulting to UTF-8",
                        encoding_name
                    ),
                    severity: WarningSeverity::Warning,
                });
                UTF_8
            }
        };

        self.context.encoding_stack.push(EncodingChange {
            encoding: self.context.current_encoding,
            line_number: line,
            pragma_type: PragmaType::UseEncoding(encoding_name.to_string()),
        });

        self.context.current_encoding = encoding;

        // Warn about encoding changes affecting heredocs
        self.warnings.push(EncodingWarning {
            line,
            message: format!(
                "Encoding changed to {} - heredoc delimiters may be affected",
                encoding_name
            ),
            severity: WarningSeverity::Info,
        });
    }

    /// Handle use/no utf8 pragma
    fn handle_utf8_pragma(&mut self, is_use: bool, line: usize) {
        let new_encoding = if is_use { UTF_8 } else { WINDOWS_1252 };

        self.context.encoding_stack.push(EncodingChange {
            encoding: self.context.current_encoding,
            line_number: line,
            pragma_type: if is_use { PragmaType::UseUtf8 } else { PragmaType::NoUtf8 },
        });

        self.context.current_encoding = new_encoding;
    }

    /// Handle use locale pragma
    fn handle_locale_pragma(&mut self, line: usize) {
        self.warnings.push(EncodingWarning {
            line,
            message: "use locale detected - encoding depends on runtime environment".to_string(),
            severity: WarningSeverity::Warning,
        });

        self.context.encoding_stack.push(EncodingChange {
            encoding: self.context.current_encoding,
            line_number: line,
            pragma_type: PragmaType::UseLocale,
        });
    }

    /// Normalize a string for delimiter matching in current encoding
    pub fn normalize_for_delimiter(&mut self, text: &str) -> Result<String, EncodingError> {
        // Check cache first
        if let Some(cached) = self.context.normalization_cache.get(text) {
            return Ok(cached.clone());
        }

        // If already UTF-8, no conversion needed
        if self.context.current_encoding == UTF_8 {
            return Ok(text.to_string());
        }

        // Convert from current encoding to UTF-8
        let (decoded, _, had_errors) = self.context.current_encoding.decode(text.as_bytes());

        if had_errors {
            return Err(EncodingError::ConversionFailed {
                from: self.context.current_encoding.name().to_string(),
                text: text.to_string(),
            });
        }

        let normalized = decoded.into_owned();
        self.context.normalization_cache.insert(text.to_string(), normalized.clone());

        Ok(normalized)
    }

    /// Check if delimiter matching might be affected by encoding
    pub fn check_delimiter_encoding_safety(&self, delimiter: &str) -> DelimiterSafety {
        // ASCII delimiters are always safe
        if delimiter.is_ascii() {
            return DelimiterSafety::Safe;
        }

        // Check if we've had encoding changes
        if self.context.encoding_stack.is_empty() {
            return DelimiterSafety::Safe;
        }

        // Non-ASCII with encoding changes is risky
        DelimiterSafety::Risky {
            reason: "Non-ASCII delimiter with encoding changes".to_string(),
            suggestion: "Use ASCII-only delimiters for better compatibility".to_string(),
        }
    }

    /// Get encoding at a specific line
    pub fn encoding_at_line(&self, line: usize) -> &'static Encoding {
        // Find the most recent encoding change before this line
        for change in self.context.encoding_stack.iter().rev() {
            if change.line_number <= line {
                return change.encoding;
            }
        }

        // Default encoding
        UTF_8
    }

    /// Generate encoding-related diagnostics
    pub fn generate_diagnostics(&self) -> Vec<EncodingDiagnostic> {
        let mut diagnostics = Vec::new();

        // Check for multiple encoding changes
        if self.context.encoding_stack.len() > 1 {
            diagnostics.push(EncodingDiagnostic {
                message: format!(
                    "Multiple encoding changes detected ({} total)",
                    self.context.encoding_stack.len()
                ),
                severity: DiagnosticSeverity::Warning,
                locations: self.context.encoding_stack.iter().map(|c| c.line_number).collect(),
            });
        }

        // Check for problematic patterns
        for window in self.context.encoding_stack.windows(2) {
            let prev = &window[0];
            let next = &window[1];

            if prev.encoding != next.encoding {
                diagnostics.push(EncodingDiagnostic {
                    message: format!(
                        "Encoding changed from {} to {} at line {}",
                        prev.encoding.name(),
                        next.encoding.name(),
                        next.line_number
                    ),
                    severity: DiagnosticSeverity::Info,
                    locations: vec![prev.line_number, next.line_number],
                });
            }
        }

        diagnostics
    }
}

#[derive(Debug)]
pub enum EncodingError {
    ConversionFailed { from: String, text: String },
    UnsupportedEncoding(String),
}

#[derive(Debug)]
pub enum DelimiterSafety {
    Safe,
    Risky { reason: String, suggestion: String },
}

#[derive(Debug)]
pub struct EncodingDiagnostic {
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub locations: Vec<usize>,
}

#[derive(Debug, PartialEq)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// Integration with heredoc parser
pub struct EncodingAwareHeredocScanner<'a> {
    lexer: &'a mut EncodingAwareLexer,
    #[allow(dead_code)]
    source: &'a str,
}

impl<'a> EncodingAwareHeredocScanner<'a> {
    pub fn new(lexer: &'a mut EncodingAwareLexer, source: &'a str) -> Self {
        Self { lexer, source }
    }

    /// Scan for heredoc with encoding awareness
    pub fn scan_heredoc(&mut self, delimiter: &str, line: usize) -> Result<String, EncodingError> {
        // Normalize delimiter for current encoding
        let normalized_delimiter = self.lexer.normalize_for_delimiter(delimiter)?;

        // Check delimiter safety
        match self.lexer.check_delimiter_encoding_safety(delimiter) {
            DelimiterSafety::Risky { reason, suggestion } => {
                self.lexer.warnings.push(EncodingWarning {
                    line,
                    message: format!("{} - {}", reason, suggestion),
                    severity: WarningSeverity::Warning,
                });
            }
            DelimiterSafety::Safe => {}
        }

        Ok(normalized_delimiter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoding_pragma_detection() {
        let mut lexer = EncodingAwareLexer::new();
        let code = r#"
use encoding 'latin1';
my $text = <<'END';
Some text
END

use utf8;
my $unicode = <<'終';
Unicode delimiter
終
"#;

        lexer.scan_encoding_pragmas(code);
        assert_eq!(lexer.context.encoding_stack.len(), 2);
        assert_eq!(lexer.warnings.len(), 2); // One for latin1, one for utf8
    }

    #[test]
    fn test_delimiter_safety_check() {
        let lexer = EncodingAwareLexer::new();

        // ASCII is always safe
        match lexer.check_delimiter_encoding_safety("EOF") {
            DelimiterSafety::Safe => {}
            _ => unreachable!("ASCII should be safe"),
        }

        // Non-ASCII without encoding changes is safe
        match lexer.check_delimiter_encoding_safety("終") {
            DelimiterSafety::Safe => {}
            _ => unreachable!("Should be safe without encoding changes"),
        }
    }

    #[test]
    fn test_encoding_at_line() {
        let mut lexer = EncodingAwareLexer::new();
        lexer.context.encoding_stack.push(EncodingChange {
            encoding: WINDOWS_1252,
            line_number: 10,
            pragma_type: PragmaType::UseEncoding("latin1".to_string()),
        });

        assert_eq!(lexer.encoding_at_line(5), UTF_8);
        assert_eq!(lexer.encoding_at_line(15), WINDOWS_1252);
    }
}
