//! Native formatter contract types.
//!
//! This module defines the Rust-native formatter API that future formatter
//! engines should implement. It intentionally lives beside the existing
//! subprocess-backed `PerlTidyFormatter` adapter so consumers can start moving
//! toward native formatting without changing the current runtime path.

use serde::{Deserialize, Serialize};

const PARSE_ERROR_CODE: &str = "native.format.parse_error";
const PARSE_PRESERVATION_CODE: &str = "native.format.parse_preservation";
const LITERAL_PRESERVE_CODE: &str = "native.format.literal_preserve_region";

/// Native formatter document tree.
///
/// This is the small, lossless-friendly formatting IR from the replacement
/// contract. It is deliberately independent of Perl syntax for now; later
/// parser-facing formatter passes should lower CST/AST fragments into this
/// tree and then render it deterministically.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormatDoc {
    /// Literal text that may be laid out with surrounding IR.
    Text(String),
    /// One ordinary space.
    Space,
    /// A newline at the current indentation level.
    Line,
    /// A line break that becomes a space when its containing group fits.
    SoftLine,
    /// A newline that cannot be flattened.
    HardLine,
    /// A layout group that may render flat or broken.
    Group(Vec<FormatDoc>),
    /// A nested document rendered one indentation level deeper when broken.
    Indent(Vec<FormatDoc>),
    /// Render one branch when broken and another branch when flat.
    IfBreak {
        /// Document to render when the containing group breaks.
        broken: Box<FormatDoc>,
        /// Document to render when the containing group fits flat.
        flat: Box<FormatDoc>,
    },
    /// Literal source text that must be preserved byte-for-byte.
    LiteralPreserve(String),
}

impl FormatDoc {
    /// Create literal text.
    #[must_use]
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    /// Create a layout group.
    #[must_use]
    pub fn group(parts: impl Into<Vec<FormatDoc>>) -> Self {
        Self::Group(parts.into())
    }

    /// Create an indented document.
    #[must_use]
    pub fn indent(parts: impl Into<Vec<FormatDoc>>) -> Self {
        Self::Indent(parts.into())
    }

    /// Create an if-break choice.
    #[must_use]
    pub fn if_break(broken: FormatDoc, flat: FormatDoc) -> Self {
        Self::IfBreak { broken: Box::new(broken), flat: Box::new(flat) }
    }

    /// Create a literal-preserve region.
    #[must_use]
    pub fn literal_preserve(value: impl Into<String>) -> Self {
        Self::LiteralPreserve(value.into())
    }

    /// Render this document using the native formatter configuration.
    #[must_use]
    pub fn render(&self, config: &FormatConfig) -> String {
        let mut renderer = DocRenderer::new(config);
        renderer.render_doc(self, 0, false, false);
        renderer.output
    }

    fn flat_width(&self) -> Option<usize> {
        match self {
            Self::Text(text) | Self::LiteralPreserve(text) => {
                (!text.contains('\n')).then_some(text.chars().count())
            }
            Self::Space | Self::SoftLine => Some(1),
            Self::Line | Self::HardLine => None,
            Self::Group(parts) | Self::Indent(parts) => {
                parts.iter().try_fold(0_usize, |sum, doc| doc.flat_width().map(|width| sum + width))
            }
            Self::IfBreak { flat, .. } => flat.flat_width(),
        }
    }
}

struct DocRenderer<'a> {
    config: &'a FormatConfig,
    output: String,
    column: usize,
}

impl<'a> DocRenderer<'a> {
    fn new(config: &'a FormatConfig) -> Self {
        Self { config, output: String::new(), column: 0 }
    }

    fn render_doc(&mut self, doc: &FormatDoc, indent_level: usize, flat: bool, broken: bool) {
        match doc {
            FormatDoc::Text(text) | FormatDoc::LiteralPreserve(text) => self.push_text(text),
            FormatDoc::Space => self.push_text(" "),
            FormatDoc::Line | FormatDoc::HardLine => self.push_line(indent_level),
            FormatDoc::SoftLine if flat => self.push_text(" "),
            FormatDoc::SoftLine => self.push_line(indent_level),
            FormatDoc::Group(parts) => {
                let fits = doc
                    .flat_width()
                    .is_some_and(|width| self.column + width <= self.config.line_width as usize);
                for part in parts {
                    self.render_doc(part, indent_level, fits, !fits);
                }
            }
            FormatDoc::Indent(parts) => {
                for part in parts {
                    self.render_doc(part, indent_level + 1, flat, broken);
                }
            }
            FormatDoc::IfBreak { broken: broken_doc, flat: flat_doc } => {
                let selected = if broken { broken_doc } else { flat_doc };
                self.render_doc(selected, indent_level, flat, broken);
            }
        }
    }

    fn push_text(&mut self, text: &str) {
        self.output.push_str(text);
        if let Some((_, tail)) = text.rsplit_once('\n') {
            self.column = tail.chars().count();
        } else {
            self.column += text.chars().count();
        }
    }

    fn push_line(&mut self, indent_level: usize) {
        self.output.push('\n');
        let indent = if self.config.use_tabs {
            "\t".repeat(indent_level)
        } else {
            " ".repeat(indent_level * self.config.indent_width as usize)
        };
        self.output.push_str(&indent);
        self.column = indent.chars().count();
    }
}

/// Native formatter operating mode.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FormatterMode {
    /// Run the Rust-native formatter.
    #[default]
    Native,
    /// Run native formatting with compatibility defaults for common legacy profiles.
    Compat,
    /// Explicitly use an external legacy formatter adapter.
    ExternalLegacy,
    /// Disable formatting.
    Off,
}

/// Final newline handling policy.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FinalNewline {
    /// Preserve the input's final newline state.
    #[default]
    Preserve,
    /// Ensure exactly one final newline when formatting succeeds.
    Insert,
    /// Remove trailing final newlines when formatting succeeds.
    Trim,
}

/// Configuration shared by native formatter implementations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatConfig {
    /// Formatter engine mode.
    pub mode: FormatterMode,
    /// Preferred line width.
    pub line_width: u32,
    /// Indentation width when spaces are used.
    pub indent_width: u32,
    /// Whether indentation should use tabs instead of spaces.
    pub use_tabs: bool,
    /// Final newline handling.
    pub final_newline: FinalNewline,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            mode: FormatterMode::Native,
            line_width: 100,
            indent_width: 4,
            use_tabs: false,
            final_newline: FinalNewline::Preserve,
        }
    }
}

impl FormatConfig {
    /// Build a compatibility-oriented native configuration.
    #[must_use]
    pub fn compat() -> Self {
        Self { mode: FormatterMode::Compat, ..Self::default() }
    }

    /// Build an explicit external legacy configuration.
    #[must_use]
    pub fn external_legacy() -> Self {
        Self { mode: FormatterMode::ExternalLegacy, ..Self::default() }
    }
}

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

/// Native Perl formatter interface.
pub trait PerlFormatter {
    /// Format a complete source document.
    fn format_document(&self, source: &str, config: &FormatConfig) -> FormatResult;

    /// Format a source range.
    fn format_range(&self, source: &str, range: TextRange, config: &FormatConfig) -> FormatResult;
}

/// Parse-gated Rust-native Perl formatter.
///
/// This initial engine performs only deliberately small syntax layout rewrites
/// and is the safety boundary that future native formatter passes should compose
/// with: source and formatted output must both parse cleanly before any native
/// formatting edit is returned.
#[derive(Debug, Default, Clone, Copy)]
pub struct NativeFormatter;

impl NativeFormatter {
    /// Create a parse-gated native formatter.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    fn validate_clean_parse(source: &str) -> Result<(), FormatDiagnostic> {
        if let Some(kind) = literal_preserve_region(source) {
            return Err(FormatDiagnostic::new(
                LITERAL_PRESERVE_CODE,
                FormatDiagnosticSeverity::Warning,
                None,
                format!("native formatting skipped because {kind} preservation is not enabled yet"),
            ));
        }

        let mut parser = perl_parser_core::Parser::new(source);
        let output = parser.parse_with_recovery();

        if output.terminated_early {
            return Err(FormatDiagnostic::new(
                PARSE_ERROR_CODE,
                FormatDiagnosticSeverity::Warning,
                None,
                "native formatting skipped because parsing terminated early",
            ));
        }

        if let Some(error) = output.diagnostics.first() {
            return Err(FormatDiagnostic::new(
                PARSE_ERROR_CODE,
                FormatDiagnosticSeverity::Warning,
                error.location().map(|offset| TextRange::at_byte_offset(source, offset)),
                format!(
                    "native formatting skipped because the source does not parse cleanly: {error}"
                ),
            ));
        }

        Ok(())
    }

    fn format_safe_subset(source: &str) -> String {
        let mut formatted = String::with_capacity(source.len());

        for line in source.split_inclusive('\n') {
            let (body, line_ending) = split_line_ending(line);
            formatted
                .push_str(&format_simple_lexical_line(body).unwrap_or_else(|| body.to_string()));
            formatted.push_str(line_ending);
        }

        formatted
    }

    fn apply_final_newline(source: &str, config: &FormatConfig) -> String {
        match config.final_newline {
            FinalNewline::Preserve => source.to_string(),
            FinalNewline::Insert => {
                let trimmed = source.trim_end_matches(['\n', '\r']);
                format!("{trimmed}\n")
            }
            FinalNewline::Trim => source.trim_end_matches(['\n', '\r']).to_string(),
        }
    }
}

impl PerlFormatter for NativeFormatter {
    fn format_document(&self, source: &str, config: &FormatConfig) -> FormatResult {
        if matches!(config.mode, FormatterMode::Off) {
            return FormatResult::unchanged(source);
        }

        if let Err(diagnostic) = Self::validate_clean_parse(source) {
            let mut result = FormatResult::unchanged(source);
            result.diagnostics.push(diagnostic);
            return result;
        }

        let formatted = Self::apply_final_newline(&Self::format_safe_subset(source), config);
        if let Err(diagnostic) = Self::validate_clean_parse(&formatted) {
            let mut result = FormatResult::unchanged(source);
            result.diagnostics.push(FormatDiagnostic::new(
                PARSE_PRESERVATION_CODE,
                FormatDiagnosticSeverity::Warning,
                diagnostic.range,
                "native formatting skipped because formatted output did not parse cleanly",
            ));
            return result;
        }

        FormatResult::replace_document(source, formatted)
    }

    fn format_range(&self, source: &str, _range: TextRange, config: &FormatConfig) -> FormatResult {
        if matches!(config.mode, FormatterMode::Off) {
            return FormatResult::unchanged(source);
        }

        if let Err(diagnostic) = Self::validate_clean_parse(source) {
            let mut result = FormatResult::unchanged(source);
            result.diagnostics.push(diagnostic);
            return result;
        }

        FormatResult::unchanged(source)
    }
}

fn utf16_len(s: &str) -> usize {
    s.chars().map(|ch| if ch as u32 >= 0x10000 { 2 } else { 1 }).sum()
}

fn split_line_ending(line: &str) -> (&str, &str) {
    if let Some(body) = line.strip_suffix("\r\n") {
        (body, "\r\n")
    } else if let Some(body) = line.strip_suffix('\n') {
        (body, "\n")
    } else {
        (line, "")
    }
}

fn format_simple_lexical_line(line: &str) -> Option<String> {
    let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
    let (indent, body) = line.split_at(indent_len);
    if body.is_empty() || body.contains('#') {
        return None;
    }

    let mut stream = perl_parser_core::TokenStream::new(body);
    let mut tokens = Vec::new();
    loop {
        let token = stream.next().ok()?;
        if token.kind == perl_parser_core::TokenKind::Eof {
            break;
        }
        tokens.push(token);
    }

    let formatted = format_simple_lexical_tokens(&tokens)?;
    Some(format!("{indent}{formatted}"))
}

fn format_simple_lexical_tokens(tokens: &[perl_parser_core::Token]) -> Option<String> {
    use perl_parser_core::TokenKind;

    let keyword = match tokens.first()?.kind {
        TokenKind::My => "my",
        TokenKind::Our => "our",
        TokenKind::State => "state",
        _ => return None,
    };

    let semicolon = tokens.last()?;
    if semicolon.kind != TokenKind::Semicolon {
        return None;
    }

    let (variable, next_index) = format_variable_tokens(tokens, 1)?;
    let semicolon_index = tokens.len() - 1;
    if next_index == semicolon_index {
        Some(format!("{keyword} {variable};"))
    } else if next_index + 2 == semicolon_index && tokens[next_index].kind == TokenKind::Assign {
        let value = simple_value_text(&tokens[next_index + 1])?;
        Some(format!("{keyword} {variable} = {value};"))
    } else {
        None
    }
}

fn format_variable_tokens(
    tokens: &[perl_parser_core::Token],
    start: usize,
) -> Option<(String, usize)> {
    use perl_parser_core::TokenKind;

    let first = tokens.get(start)?;
    if first.kind == TokenKind::Identifier
        && first.text.chars().next().is_some_and(|ch| matches!(ch, '$' | '@' | '%'))
    {
        return Some((first.text.to_string(), start + 1));
    }

    let sigil = first;
    let name = tokens.get(start + 1)?;
    if !matches!(sigil.kind, TokenKind::ScalarSigil | TokenKind::ArraySigil | TokenKind::HashSigil)
    {
        return None;
    }
    if name.kind != TokenKind::Identifier {
        return None;
    }

    Some((format!("{}{}", sigil.text, name.text), start + 2))
}

fn simple_value_text(token: &perl_parser_core::Token) -> Option<&str> {
    use perl_parser_core::TokenKind;

    matches!(token.kind, TokenKind::Number | TokenKind::String | TokenKind::Identifier)
        .then_some(token.text.as_ref())
}

fn literal_preserve_region(source: &str) -> Option<&'static str> {
    for line in source.lines() {
        let trimmed = line.trim_start();
        if is_pod_start(trimmed) {
            return Some("POD");
        }
        if matches!(trimmed, "__DATA__" | "__END__") {
            return Some("DATA section");
        }
        if contains_likely_heredoc_start(line) {
            return Some("heredoc");
        }
    }
    None
}

fn is_pod_start(trimmed_line: &str) -> bool {
    matches!(
        trimmed_line.split_whitespace().next(),
        Some(
            "=pod"
                | "=head1"
                | "=head2"
                | "=head3"
                | "=head4"
                | "=over"
                | "=item"
                | "=back"
                | "=begin"
                | "=end"
                | "=for"
                | "=encoding"
                | "=cut"
        )
    )
}

fn contains_likely_heredoc_start(line: &str) -> bool {
    let Some((_, after_marker)) = line.split_once("<<") else {
        return false;
    };
    if after_marker.starts_with('<') {
        return false;
    }

    let after_indent = after_marker.trim_start();
    let marker = after_indent.strip_prefix('~').unwrap_or(after_indent).trim_start();
    let marker = marker.strip_prefix(['\'', '"', '`']).unwrap_or(marker);
    marker.chars().next().is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
}

impl TextRange {
    fn at_byte_offset(source: &str, offset: usize) -> Self {
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
