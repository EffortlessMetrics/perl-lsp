//! Native formatter contract types.
//!
//! This module defines the Rust-native formatter API and default formatter
//! implementation. It intentionally lives beside the subprocess-backed
//! `PerlTidyFormatter` adapter so consumers can keep an explicit legacy
//! compatibility path while the LSP runtime uses native formatting by default.

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

/// Trailing comma handling for wrapped delimited expressions.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrailingComma {
    /// Preserve the native formatter's current behavior and do not add commas.
    #[default]
    Preserve,
    /// Add a trailing comma when a call, list, or hash is rendered across lines.
    AddWhenWrapped,
}

/// Opening brace placement for supported native block layouts.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BracePlacement {
    /// Keep the opening brace on the block header line.
    #[default]
    SameLine,
    /// Place the opening brace on its own line at the block indentation.
    NextLine,
}

/// Placement for supported native `else` and `elsif` block tails.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ElsePlacement {
    /// Keep `else` and `elsif` cuddled to the previous closing brace.
    #[default]
    Cuddled,
    /// Place `else` and `elsif` on a fresh line at the block indentation.
    SeparateLine,
}

/// Spacing between supported control keywords and their condition parentheses.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeywordSpacing {
    /// Insert a space between the keyword and condition parentheses.
    #[default]
    Space,
    /// Omit the space between the keyword and condition parentheses.
    Compact,
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
    /// Trailing comma handling for wrapped delimited expressions.
    pub trailing_comma: TrailingComma,
    /// Opening brace placement for supported block layouts.
    pub brace_placement: BracePlacement,
    /// Else/elsif placement for supported block tails.
    pub else_placement: ElsePlacement,
    /// Keyword spacing for supported control-flow condition headers.
    pub keyword_spacing: KeywordSpacing,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            mode: FormatterMode::Native,
            line_width: 100,
            indent_width: 4,
            use_tabs: false,
            final_newline: FinalNewline::Preserve,
            trailing_comma: TrailingComma::Preserve,
            brace_placement: BracePlacement::SameLine,
            else_placement: ElsePlacement::Cuddled,
            keyword_spacing: KeywordSpacing::Space,
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

    fn format_safe_subset(source: &str, config: &FormatConfig) -> String {
        let mut formatted = String::with_capacity(source.len());

        for line in source.split_inclusive('\n') {
            let (body, line_ending) = split_line_ending(line);
            formatted
                .push_str(&format_simple_line(body, config).unwrap_or_else(|| body.to_string()));
            formatted.push_str(line_ending);
        }

        formatted
    }

    fn format_safe_subset_range(
        source: &str,
        range: TextRange,
        config: &FormatConfig,
    ) -> (String, Vec<TextEdit>) {
        let mut formatted = String::with_capacity(source.len());
        let mut edits = Vec::new();

        for (line_index, line) in source.split_inclusive('\n').enumerate() {
            let line_index = line_index as u32;
            let (body, line_ending) = split_line_ending(line);
            let formatted_body = if range_includes_line(range, line_index) {
                format_simple_line(body, config)
            } else {
                None
            };

            if let Some(formatted_line) = formatted_body {
                if formatted_line != body {
                    edits.push(TextEdit::new(
                        TextRange::new(
                            TextPosition::new(line_index, 0),
                            TextPosition::new(line_index, utf16_len(body) as u32),
                        ),
                        formatted_line.clone(),
                    ));
                    formatted.push_str(&formatted_line);
                } else {
                    formatted.push_str(body);
                }
            } else {
                formatted.push_str(body);
            }
            formatted.push_str(line_ending);
        }

        (formatted, edits)
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

        let formatted =
            Self::apply_final_newline(&Self::format_safe_subset(source, config), config);
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

    fn format_range(&self, source: &str, range: TextRange, config: &FormatConfig) -> FormatResult {
        if matches!(config.mode, FormatterMode::Off) {
            return FormatResult::unchanged(source);
        }

        if let Err(diagnostic) = Self::validate_clean_parse(source) {
            let mut result = FormatResult::unchanged(source);
            result.diagnostics.push(diagnostic);
            return result;
        }

        let (formatted, edits) = Self::format_safe_subset_range(source, range, config);
        if let Err(diagnostic) = Self::validate_clean_parse(&formatted) {
            let mut result = FormatResult::unchanged(source);
            result.diagnostics.push(FormatDiagnostic::new(
                PARSE_PRESERVATION_CODE,
                FormatDiagnosticSeverity::Warning,
                diagnostic.range,
                "native range formatting skipped because formatted output did not parse cleanly",
            ));
            return result;
        }

        FormatResult { formatted, changed: !edits.is_empty(), edits, diagnostics: Vec::new() }
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

fn range_includes_line(range: TextRange, line: u32) -> bool {
    line >= range.start.line
        && (line < range.end.line || line == range.end.line && range.end.character > 0)
}

fn format_simple_line(line: &str, config: &FormatConfig) -> Option<String> {
    format_simple_control_block_line(line, config)
        .or_else(|| format_simple_subroutine_line(line, config))
        .or_else(|| format_simple_module_line(line, config))
        .or_else(|| format_simple_statement_line(line, config))
        .or_else(|| format_simple_lexical_line(line, config))
}

fn format_simple_module_line(line: &str, config: &FormatConfig) -> Option<String> {
    let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
    let (indent, body) = line.split_at(indent_len);
    if body.is_empty() {
        return None;
    }
    let (body, trailing_comment) = split_trailing_comment(body);

    let mut stream = perl_parser_core::TokenStream::new(body);
    let mut tokens = Vec::new();
    loop {
        let token = stream.next().ok()?;
        if token.kind == perl_parser_core::TokenKind::Eof {
            break;
        }
        tokens.push(token);
    }

    let formatted = format_simple_module_tokens(&tokens, config)?;
    Some(format!("{indent}{}", append_trailing_comment(formatted, trailing_comment)))
}

fn format_simple_lexical_line(line: &str, config: &FormatConfig) -> Option<String> {
    let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
    let (indent, body) = line.split_at(indent_len);
    if body.is_empty() {
        return None;
    }
    let (body, trailing_comment) = split_trailing_comment(body);

    let mut stream = perl_parser_core::TokenStream::new(body);
    let mut tokens = Vec::new();
    loop {
        let token = stream.next().ok()?;
        if token.kind == perl_parser_core::TokenKind::Eof {
            break;
        }
        tokens.push(token);
    }

    let formatted = format_simple_lexical_tokens(&tokens, config)?;
    Some(format!("{indent}{}", append_trailing_comment(formatted, trailing_comment)))
}

fn format_simple_subroutine_line(line: &str, config: &FormatConfig) -> Option<String> {
    let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
    let (indent, body) = line.split_at(indent_len);
    if body.is_empty() {
        return None;
    }
    let (body, trailing_comment) = split_trailing_comment(body);

    let mut stream = perl_parser_core::TokenStream::new(body);
    let mut tokens = Vec::new();
    loop {
        let token = stream.next().ok()?;
        if token.kind == perl_parser_core::TokenKind::Eof {
            break;
        }
        tokens.push(token);
    }

    let formatted = format_simple_subroutine_tokens(&tokens, indent, config)?;
    Some(append_trailing_comment(formatted, trailing_comment))
}

fn format_simple_control_block_line(line: &str, config: &FormatConfig) -> Option<String> {
    let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
    let (indent, body) = line.split_at(indent_len);
    if body.is_empty() {
        return None;
    }
    let (body, trailing_comment) = split_trailing_comment(body);

    let mut stream = perl_parser_core::TokenStream::new(body);
    let mut tokens = Vec::new();
    loop {
        let token = stream.next().ok()?;
        if token.kind == perl_parser_core::TokenKind::Eof {
            break;
        }
        tokens.push(token);
    }

    let formatted = format_simple_control_block_tokens(&tokens, indent, config)?;
    Some(append_trailing_comment(formatted, trailing_comment))
}

fn format_simple_statement_line(line: &str, config: &FormatConfig) -> Option<String> {
    let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
    let (indent, body) = line.split_at(indent_len);
    if body.is_empty() {
        return None;
    }
    let (body, trailing_comment) = split_trailing_comment(body);

    let mut stream = perl_parser_core::TokenStream::new(body);
    let mut tokens = Vec::new();
    loop {
        let token = stream.next().ok()?;
        if token.kind == perl_parser_core::TokenKind::Eof {
            break;
        }
        tokens.push(token);
    }

    let formatted = format_simple_statement_tokens(&tokens, config)?;
    Some(format!("{indent}{}", append_trailing_comment(formatted, trailing_comment)))
}

fn split_trailing_comment(body: &str) -> (&str, Option<&str>) {
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    let mut escaped = false;

    for (index, ch) in body.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' && (in_single || in_double || in_backtick) {
            escaped = true;
            continue;
        }

        match ch {
            '\'' if !in_double && !in_backtick => in_single = !in_single,
            '"' if !in_single && !in_backtick => in_double = !in_double,
            '`' if !in_single && !in_double => in_backtick = !in_backtick,
            '#' if !in_single && !in_double && !in_backtick => {
                let code = body[..index].trim_end();
                if code.trim().is_empty() {
                    return (body, None);
                }
                return (code, Some(&body[index..]));
            }
            _ => {}
        }
    }

    (body, None)
}

fn append_trailing_comment(mut formatted: String, trailing_comment: Option<&str>) -> String {
    if let Some(comment) = trailing_comment {
        formatted.push(' ');
        formatted.push_str(comment);
    }
    formatted
}

fn format_simple_subroutine_tokens(
    tokens: &[perl_parser_core::Token],
    indent: &str,
    config: &FormatConfig,
) -> Option<String> {
    use perl_parser_core::TokenKind;

    if tokens.len() < 4 {
        return None;
    }
    if tokens[0].kind != TokenKind::Sub
        || tokens[1].kind != TokenKind::Identifier
        || tokens[2].kind != TokenKind::LeftBrace
        || tokens.last()?.kind != TokenKind::RightBrace
    {
        return None;
    }

    let body_tokens = &tokens[3..tokens.len() - 1];
    let statements = format_simple_statement_block(body_tokens, config)?;
    let body_indent = format!("{indent}{}", indent_unit(config));
    Some(render_simple_block_doc(
        format!("{indent}sub {} {{", tokens[1].text),
        &statements,
        indent,
        &body_indent,
        config,
    ))
}

fn format_simple_control_block_tokens(
    tokens: &[perl_parser_core::Token],
    indent: &str,
    config: &FormatConfig,
) -> Option<String> {
    use perl_parser_core::TokenKind;

    if let Some(formatted) = format_simple_c_style_for_block_tokens(tokens, indent, config) {
        return Some(formatted);
    }

    if let Some(formatted) = format_simple_foreach_block_tokens(tokens, indent, config) {
        return Some(formatted);
    }

    if tokens.len() < 6 {
        return None;
    }
    let keyword = match tokens[0].kind {
        TokenKind::If => "if",
        TokenKind::Unless => "unless",
        TokenKind::While => "while",
        TokenKind::Until => "until",
        _ => return None,
    };
    if tokens[1].kind != TokenKind::LeftParen {
        return None;
    }

    let (condition, next_index) = format_simple_condition_tokens(tokens, 2, config)?;
    if tokens.get(next_index)?.kind != TokenKind::RightParen
        || tokens.get(next_index + 1)?.kind != TokenKind::LeftBrace
    {
        return None;
    }

    let body_start = next_index + 2;
    let body_end = tokens[body_start..]
        .iter()
        .position(|token| token.kind == TokenKind::RightBrace)
        .map(|offset| body_start + offset)?;
    let body_tokens = &tokens[body_start..body_end];
    let statements = format_simple_statement_block(body_tokens, config)?;

    let body_indent = format!("{indent}{}", indent_unit(config));
    let mut formatted = render_simple_block_doc(
        render_condition_block_header(indent, keyword, &condition, config),
        &statements,
        indent,
        &body_indent,
        config,
    );

    match keyword {
        "if" | "unless" => {
            let tail = format_simple_control_tail(tokens, body_end, keyword, config)?;
            for (condition, statements) in tail.elsif_branches {
                formatted.push_str(&render_simple_elsif_doc(
                    &condition,
                    &statements,
                    indent,
                    &body_indent,
                    config,
                ));
            }
            if let Some(else_statements) = tail.else_statements {
                formatted.push_str(&render_simple_else_doc(
                    &else_statements,
                    indent,
                    &body_indent,
                    config,
                ));
            }
        }
        "while" | "until" => {
            if let Some(continue_statements) =
                format_simple_continue_tail(tokens, body_end, config)?
            {
                formatted.push_str(&render_simple_continue_doc(
                    &continue_statements,
                    indent,
                    &body_indent,
                    config,
                ));
            }
        }
        _ => return None,
    }
    Some(formatted)
}

fn format_simple_c_style_for_block_tokens(
    tokens: &[perl_parser_core::Token],
    indent: &str,
    config: &FormatConfig,
) -> Option<String> {
    use perl_parser_core::TokenKind;

    if tokens.first()?.kind != TokenKind::For || tokens.get(1)?.kind != TokenKind::LeftParen {
        return None;
    }

    let (first_semicolon, second_semicolon, header_end) = find_for_header_boundaries(tokens, 1)?;
    if tokens.get(header_end + 1)?.kind != TokenKind::LeftBrace {
        return None;
    }

    let init = format_simple_for_init_clause(tokens, 2, first_semicolon, config)?;
    let condition =
        format_simple_for_condition_clause(tokens, first_semicolon + 1, second_semicolon, config)?;
    let update = format_simple_for_update_clause(tokens, second_semicolon + 1, header_end, config)?;

    let body_start = header_end + 2;
    let body_end = tokens[body_start..]
        .iter()
        .position(|token| token.kind == TokenKind::RightBrace)
        .map(|offset| body_start + offset)?;
    let statements = format_simple_statement_block(&tokens[body_start..body_end], config)?;

    let body_indent = format!("{indent}{}", indent_unit(config));
    let mut formatted = render_simple_block_doc(
        format!("{indent}{}", render_simple_for_header(&init, &condition, &update)),
        &statements,
        indent,
        &body_indent,
        config,
    );
    if let Some(continue_statements) = format_simple_continue_tail(tokens, body_end, config)? {
        formatted.push_str(&render_simple_continue_doc(
            &continue_statements,
            indent,
            &body_indent,
            config,
        ));
    }
    Some(formatted)
}

fn find_for_header_boundaries(
    tokens: &[perl_parser_core::Token],
    open_index: usize,
) -> Option<(usize, usize, usize)> {
    use perl_parser_core::TokenKind;

    let mut depth = 0usize;
    let mut first_semicolon = None;
    let mut second_semicolon = None;

    for (index, token) in tokens.iter().enumerate().skip(open_index) {
        match token.kind {
            TokenKind::LeftParen => depth += 1,
            TokenKind::RightParen => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some((first_semicolon?, second_semicolon?, index));
                }
            }
            TokenKind::Semicolon if depth == 1 => {
                if first_semicolon.is_none() {
                    first_semicolon = Some(index);
                } else if second_semicolon.is_none() {
                    second_semicolon = Some(index);
                } else {
                    return None;
                }
            }
            _ => {}
        }
    }
    None
}

fn render_simple_for_header(init: &str, condition: &str, update: &str) -> String {
    let mut header = format!("for ({init};");
    if !condition.is_empty() {
        header.push(' ');
        header.push_str(condition);
    }
    header.push(';');
    if !update.is_empty() {
        header.push(' ');
        header.push_str(update);
    }
    header.push_str(") {");
    header
}

fn format_simple_for_init_clause(
    tokens: &[perl_parser_core::Token],
    start: usize,
    end: usize,
    config: &FormatConfig,
) -> Option<String> {
    use perl_parser_core::TokenKind;

    if start == end {
        return Some(String::new());
    }

    match tokens.get(start)?.kind {
        TokenKind::My | TokenKind::Our | TokenKind::State => {
            format_simple_lexical_clause(tokens, start, end, config)
        }
        _ => format_simple_assignment_clause(tokens, start, end, config),
    }
}

fn format_simple_for_condition_clause(
    tokens: &[perl_parser_core::Token],
    start: usize,
    end: usize,
    config: &FormatConfig,
) -> Option<String> {
    if start == end {
        return Some(String::new());
    }
    format_simple_expression_tokens(tokens, start, end, config, 0)
}

fn format_simple_for_update_clause(
    tokens: &[perl_parser_core::Token],
    start: usize,
    end: usize,
    config: &FormatConfig,
) -> Option<String> {
    use perl_parser_core::TokenKind;

    if start == end {
        return Some(String::new());
    }

    if let Some((variable, next_index)) = format_variable_tokens(tokens, start)
        && next_index + 1 == end
    {
        return match tokens.get(next_index)?.kind {
            TokenKind::Increment => Some(format!("{variable}++")),
            TokenKind::Decrement => Some(format!("{variable}--")),
            _ => None,
        };
    }

    if matches!(tokens.get(start)?.kind, TokenKind::Increment | TokenKind::Decrement) {
        let (variable, next_index) = format_variable_tokens(tokens, start + 1)?;
        if next_index == end {
            return Some(format!("{}{variable}", tokens[start].text));
        }
    }

    format_simple_assignment_clause(tokens, start, end, config)
}

fn format_simple_foreach_block_tokens(
    tokens: &[perl_parser_core::Token],
    indent: &str,
    config: &FormatConfig,
) -> Option<String> {
    use perl_parser_core::TokenKind;

    let keyword = match tokens.first()?.kind {
        TokenKind::For => "for",
        TokenKind::Foreach => "foreach",
        _ => return None,
    };

    let mut index = 1;
    let iterator =
        if matches!(tokens.get(index)?.kind, TokenKind::My | TokenKind::Our | TokenKind::State) {
            let lexical = tokens[index].text.as_ref();
            let (variable, next_index) = format_variable_tokens(tokens, index + 1)?;
            index = next_index;
            format!("{lexical} {variable}")
        } else {
            let (variable, next_index) = format_variable_tokens(tokens, index)?;
            index = next_index;
            variable
        };

    if tokens.get(index)?.kind != TokenKind::LeftParen {
        return None;
    }
    let list_start = index + 1;
    let list_end = tokens[list_start..]
        .iter()
        .position(|token| token.kind == TokenKind::RightParen)
        .map(|offset| list_start + offset)?;
    let list = format_simple_expression_tokens(tokens, list_start, list_end, config, 0)?;

    if tokens.get(list_end)?.kind != TokenKind::RightParen
        || tokens.get(list_end + 1)?.kind != TokenKind::LeftBrace
    {
        return None;
    }

    let body_start = list_end + 2;
    let body_end = tokens[body_start..]
        .iter()
        .position(|token| token.kind == TokenKind::RightBrace)
        .map(|offset| body_start + offset)?;
    let body_tokens = &tokens[body_start..body_end];
    let statements = format_simple_statement_block(body_tokens, config)?;
    let body_indent = format!("{indent}{}", indent_unit(config));
    let mut formatted = render_simple_block_doc(
        format!("{indent}{keyword} {iterator} ({list}) {{"),
        &statements,
        indent,
        &body_indent,
        config,
    );
    if let Some(continue_statements) = format_simple_continue_tail(tokens, body_end, config)? {
        formatted.push_str(&render_simple_continue_doc(
            &continue_statements,
            indent,
            &body_indent,
            config,
        ));
    }
    Some(formatted)
}

fn render_simple_block_doc(
    header: String,
    statements: &[String],
    indent: &str,
    body_indent: &str,
    config: &FormatConfig,
) -> String {
    let mut parts = vec![FormatDoc::text(render_block_header(&header, indent, config))];
    push_simple_block_body_docs(&mut parts, statements, indent, body_indent);
    FormatDoc::group(parts).render(config)
}

fn render_simple_else_doc(
    statements: &[String],
    indent: &str,
    body_indent: &str,
    config: &FormatConfig,
) -> String {
    let header = if config.else_placement == ElsePlacement::SeparateLine {
        format!("\n{indent}else {{")
    } else {
        " else {".to_string()
    };
    let mut parts = vec![FormatDoc::text(render_block_header(&header, indent, config))];
    push_simple_block_body_docs(&mut parts, statements, indent, body_indent);
    FormatDoc::group(parts).render(config)
}

fn render_simple_elsif_doc(
    condition: &str,
    statements: &[String],
    indent: &str,
    body_indent: &str,
    config: &FormatConfig,
) -> String {
    let header = if config.else_placement == ElsePlacement::SeparateLine {
        format!("\n{}", render_condition_block_header(indent, "elsif", condition, config))
    } else {
        let gap = keyword_condition_gap(config);
        format!(" elsif{gap}({condition}) {{")
    };
    let mut parts = vec![FormatDoc::text(render_block_header(&header, indent, config))];
    push_simple_block_body_docs(&mut parts, statements, indent, body_indent);
    FormatDoc::group(parts).render(config)
}

fn render_simple_continue_doc(
    statements: &[String],
    indent: &str,
    body_indent: &str,
    config: &FormatConfig,
) -> String {
    let mut parts = vec![FormatDoc::text(render_block_header(" continue {", indent, config))];
    push_simple_block_body_docs(&mut parts, statements, indent, body_indent);
    FormatDoc::group(parts).render(config)
}

fn render_block_header(header: &str, indent: &str, config: &FormatConfig) -> String {
    if config.brace_placement != BracePlacement::NextLine {
        return header.to_string();
    }

    header
        .strip_suffix(" {")
        .map_or_else(|| header.to_string(), |prefix| format!("{prefix}\n{indent}{{"))
}

fn render_condition_block_header(
    indent: &str,
    keyword: &str,
    condition: &str,
    config: &FormatConfig,
) -> String {
    let gap = keyword_condition_gap(config);
    format!("{indent}{keyword}{gap}({condition}) {{")
}

fn keyword_condition_gap(config: &FormatConfig) -> &'static str {
    match config.keyword_spacing {
        KeywordSpacing::Space => " ",
        KeywordSpacing::Compact => "",
    }
}

fn push_simple_block_body_docs(
    parts: &mut Vec<FormatDoc>,
    statements: &[String],
    indent: &str,
    body_indent: &str,
) {
    for statement in statements {
        parts.push(FormatDoc::HardLine);
        parts.push(FormatDoc::text(format!("{body_indent}{statement}")));
    }
    parts.push(FormatDoc::HardLine);
    parts.push(FormatDoc::text(format!("{indent}}}")));
}

struct SimpleControlTail {
    elsif_branches: Vec<(String, Vec<String>)>,
    else_statements: Option<Vec<String>>,
}

fn format_simple_control_tail(
    tokens: &[perl_parser_core::Token],
    body_end: usize,
    keyword: &str,
    config: &FormatConfig,
) -> Option<SimpleControlTail> {
    use perl_parser_core::TokenKind;

    let mut index = body_end + 1;
    let mut tail = SimpleControlTail { elsif_branches: Vec::new(), else_statements: None };
    if index == tokens.len() {
        return Some(tail);
    }

    while tokens.get(index)?.kind == TokenKind::Elsif {
        if keyword != "if" {
            return None;
        }
        if tokens.get(index + 1)?.kind != TokenKind::LeftParen {
            return None;
        }

        let (condition, next_index) = format_simple_condition_tokens(tokens, index + 2, config)?;
        if tokens.get(next_index)?.kind != TokenKind::RightParen
            || tokens.get(next_index + 1)?.kind != TokenKind::LeftBrace
        {
            return None;
        }

        let elsif_body_start = next_index + 2;
        let elsif_body_end = tokens[elsif_body_start..]
            .iter()
            .position(|token| token.kind == TokenKind::RightBrace)
            .map(|offset| elsif_body_start + offset)?;
        let statements =
            format_simple_statement_block(&tokens[elsif_body_start..elsif_body_end], config)?;
        tail.elsif_branches.push((condition, statements));
        index = elsif_body_end + 1;

        if index == tokens.len() {
            return Some(tail);
        }
    }

    if tokens.get(index)?.kind != TokenKind::Else {
        return None;
    }
    if !matches!(keyword, "if" | "unless")
        || tokens.get(index + 1)?.kind != TokenKind::LeftBrace
        || tokens.last()?.kind != TokenKind::RightBrace
    {
        return None;
    }

    let else_body_start = index + 2;
    let else_body_tokens = &tokens[else_body_start..tokens.len() - 1];
    let statements = format_simple_statement_block(else_body_tokens, config)?;
    tail.else_statements = Some(statements);
    Some(tail)
}

fn format_simple_continue_tail(
    tokens: &[perl_parser_core::Token],
    body_end: usize,
    config: &FormatConfig,
) -> Option<Option<Vec<String>>> {
    use perl_parser_core::TokenKind;

    let next = body_end + 1;
    if next == tokens.len() {
        return Some(None);
    }
    if tokens.get(next)?.kind != TokenKind::Continue
        || tokens.get(next + 1)?.kind != TokenKind::LeftBrace
        || tokens.last()?.kind != TokenKind::RightBrace
    {
        return None;
    }

    let continue_body_start = next + 2;
    let continue_body_tokens = &tokens[continue_body_start..tokens.len() - 1];
    let statements = format_simple_statement_block(continue_body_tokens, config)?;
    Some(Some(statements))
}

fn format_simple_statement_block(
    tokens: &[perl_parser_core::Token],
    config: &FormatConfig,
) -> Option<Vec<String>> {
    use perl_parser_core::TokenKind;

    if tokens.is_empty() {
        return Some(Vec::new());
    }

    let mut statements = Vec::new();
    let mut start = 0;
    for (idx, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Semicolon {
            continue;
        }

        let statement_tokens = &tokens[start..=idx];
        statements.push(format_simple_statement_tokens(statement_tokens, config)?);
        start = idx + 1;
    }

    (start == tokens.len()).then_some(statements)
}

fn format_simple_statement_tokens(
    tokens: &[perl_parser_core::Token],
    config: &FormatConfig,
) -> Option<String> {
    format_simple_lexical_tokens(tokens, config)
        .or_else(|| format_simple_return_tokens(tokens, config))
        .or_else(|| format_simple_loop_control_tokens(tokens))
        .or_else(|| format_simple_assignment_tokens(tokens, config))
        .or_else(|| format_simple_expression_statement_tokens(tokens, config))
}

fn format_simple_module_tokens(
    tokens: &[perl_parser_core::Token],
    config: &FormatConfig,
) -> Option<String> {
    use perl_parser_core::TokenKind;

    if tokens.last()?.kind != TokenKind::Semicolon {
        return None;
    }

    match tokens.first()?.kind {
        TokenKind::Package => format_simple_package_tokens(tokens, config),
        TokenKind::Use => format_simple_import_tokens("use", tokens, 1, config),
        TokenKind::No => format_simple_import_tokens("no", tokens, 1, config),
        TokenKind::Identifier if tokens.first()?.text.as_ref() == "require" => {
            format_simple_import_tokens("require", tokens, 1, config)
        }
        _ => None,
    }
}

fn format_simple_package_tokens(
    tokens: &[perl_parser_core::Token],
    config: &FormatConfig,
) -> Option<String> {
    use perl_parser_core::TokenKind;

    if tokens.len() < 3 || tokens.first()?.kind != TokenKind::Package {
        return None;
    }

    let semicolon_index = tokens.len() - 1;
    let name = tokens.get(1)?;
    if name.kind != TokenKind::Identifier {
        return None;
    }

    if semicolon_index == 2 {
        return Some(format!("package {};", name.text));
    }

    let version = format_simple_module_args(tokens, 2, semicolon_index, config, "package ".len())?;
    Some(format!("package {} {version};", name.text))
}

fn format_simple_import_tokens(
    keyword: &str,
    tokens: &[perl_parser_core::Token],
    args_start: usize,
    config: &FormatConfig,
) -> Option<String> {
    let semicolon_index = tokens.len() - 1;
    let args = format_simple_module_args(
        tokens,
        args_start,
        semicolon_index,
        config,
        keyword.chars().count() + 1,
    )?;
    Some(format!("{keyword} {args};"))
}

fn format_simple_module_args(
    tokens: &[perl_parser_core::Token],
    start: usize,
    end: usize,
    config: &FormatConfig,
    start_column: usize,
) -> Option<String> {
    let mut parts = Vec::new();
    let mut index = start;
    let mut column = start_column;

    while index < end {
        let (part, next_index) = format_simple_atom_tokens(tokens, index, config, column)?;
        column = advance_column(column, &part) + 1;
        parts.push(part);
        index = next_index;
    }

    (!parts.is_empty()).then(|| parts.join(" "))
}

fn format_simple_lexical_tokens(
    tokens: &[perl_parser_core::Token],
    config: &FormatConfig,
) -> Option<String> {
    if tokens.last()?.kind != perl_parser_core::TokenKind::Semicolon {
        return None;
    }

    let semicolon_index = tokens.len() - 1;
    Some(format!("{};", format_simple_lexical_clause(tokens, 0, semicolon_index, config)?))
}

fn format_simple_lexical_clause(
    tokens: &[perl_parser_core::Token],
    start: usize,
    end: usize,
    config: &FormatConfig,
) -> Option<String> {
    use perl_parser_core::TokenKind;

    let keyword = match tokens.get(start)?.kind {
        TokenKind::My => "my",
        TokenKind::Our => "our",
        TokenKind::State => "state",
        _ => return None,
    };

    let (variable, next_index) = format_lexical_target_tokens(tokens, start + 1)?;
    if next_index == end {
        Some(format!("{keyword} {variable}"))
    } else if tokens[next_index].kind == TokenKind::Assign {
        let prefix = format!("{keyword} {variable} = ");
        let value = format_simple_expression_tokens(
            tokens,
            next_index + 1,
            end,
            config,
            prefix.chars().count(),
        )?;
        Some(format!("{prefix}{value}"))
    } else {
        None
    }
}

fn format_lexical_target_tokens(
    tokens: &[perl_parser_core::Token],
    start: usize,
) -> Option<(String, usize)> {
    format_variable_list_tokens(tokens, start).or_else(|| format_variable_tokens(tokens, start))
}

fn format_variable_list_tokens(
    tokens: &[perl_parser_core::Token],
    start: usize,
) -> Option<(String, usize)> {
    use perl_parser_core::TokenKind;

    if tokens.get(start)?.kind != TokenKind::LeftParen {
        return None;
    }

    let mut variables = Vec::new();
    let mut index = start + 1;
    if tokens.get(index)?.kind == TokenKind::RightParen {
        return Some(("()".to_string(), index + 1));
    }

    loop {
        let (variable, next_index) = format_variable_tokens(tokens, index)?;
        variables.push(variable);
        index = next_index;

        match tokens.get(index)?.kind {
            TokenKind::Comma => index += 1,
            TokenKind::RightParen => {
                return Some((format!("({})", variables.join(", ")), index + 1));
            }
            _ => return None,
        }
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

fn format_simple_return_tokens(
    tokens: &[perl_parser_core::Token],
    config: &FormatConfig,
) -> Option<String> {
    use perl_parser_core::TokenKind;

    if tokens.first()?.kind != TokenKind::Return || tokens.last()?.kind != TokenKind::Semicolon {
        return None;
    }

    let semicolon_index = tokens.len() - 1;
    if semicolon_index == 1 {
        return Some("return;".to_string());
    }

    let prefix = "return ";
    let value = format_simple_expression_tokens(
        tokens,
        1,
        semicolon_index,
        config,
        prefix.chars().count(),
    )?;
    Some(format!("return {value};"))
}

fn format_simple_loop_control_tokens(tokens: &[perl_parser_core::Token]) -> Option<String> {
    use perl_parser_core::TokenKind;

    let keyword = match tokens.first()?.kind {
        TokenKind::Next => "next",
        TokenKind::Last => "last",
        TokenKind::Redo => "redo",
        _ => return None,
    };
    if tokens.last()?.kind != TokenKind::Semicolon {
        return None;
    }

    match tokens {
        [_, _] => Some(format!("{keyword};")),
        [_, label, _] if label.kind == TokenKind::Identifier => {
            Some(format!("{keyword} {};", label.text))
        }
        _ => None,
    }
}

fn format_simple_assignment_tokens(
    tokens: &[perl_parser_core::Token],
    config: &FormatConfig,
) -> Option<String> {
    use perl_parser_core::TokenKind;

    if tokens.last()?.kind != TokenKind::Semicolon {
        return None;
    }

    let semicolon_index = tokens.len() - 1;
    Some(format!("{};", format_simple_assignment_clause(tokens, 0, semicolon_index, config)?))
}

fn format_simple_assignment_clause(
    tokens: &[perl_parser_core::Token],
    start: usize,
    end: usize,
    config: &FormatConfig,
) -> Option<String> {
    use perl_parser_core::TokenKind;

    let (variable, next_index) = format_variable_tokens(tokens, start)?;
    if tokens.get(next_index)?.kind != TokenKind::Assign {
        return None;
    }

    let prefix = format!("{variable} = ");
    let value = format_simple_expression_tokens(
        tokens,
        next_index + 1,
        end,
        config,
        prefix.chars().count(),
    )?;
    Some(format!("{variable} = {value}"))
}

fn format_simple_expression_statement_tokens(
    tokens: &[perl_parser_core::Token],
    config: &FormatConfig,
) -> Option<String> {
    use perl_parser_core::TokenKind;

    if tokens.last()?.kind != TokenKind::Semicolon {
        return None;
    }

    let semicolon_index = tokens.len() - 1;
    let (call, next_index) = format_simple_call_tokens(tokens, 0, config, 0)?;
    (next_index == semicolon_index).then(|| format!("{call};"))
}

fn format_simple_condition_tokens(
    tokens: &[perl_parser_core::Token],
    start: usize,
    config: &FormatConfig,
) -> Option<(String, usize)> {
    use perl_parser_core::TokenKind;

    let end = tokens[start..]
        .iter()
        .position(|token| token.kind == TokenKind::RightParen)
        .map(|offset| start + offset)?;
    let condition_config = FormatConfig { line_width: u32::MAX, ..config.clone() };
    let condition = format_simple_expression_tokens(tokens, start, end, &condition_config, 0)?;
    Some((condition, end))
}

fn format_simple_expression_tokens(
    tokens: &[perl_parser_core::Token],
    start: usize,
    end: usize,
    config: &FormatConfig,
    start_column: usize,
) -> Option<String> {
    let (left, next_index) = format_simple_atom_tokens(tokens, start, config, start_column)?;
    if next_index == end {
        return Some(left);
    }

    let operator = simple_binary_operator_text(tokens.get(next_index)?)?;
    let right_column = advance_column(start_column, &left) + operator.chars().count() + 2;
    let (right, final_index) =
        format_simple_atom_tokens(tokens, next_index + 1, config, right_column)?;
    (final_index == end).then(|| format!("{left} {operator} {right}"))
}

fn format_simple_atom_tokens(
    tokens: &[perl_parser_core::Token],
    start: usize,
    config: &FormatConfig,
    start_column: usize,
) -> Option<(String, usize)> {
    if let Some((method_call, next_index)) =
        format_simple_method_call_tokens(tokens, start, config, start_column)
    {
        return Some((method_call, next_index));
    }

    if let Some((variable, next_index)) = format_variable_tokens(tokens, start) {
        return Some((variable, next_index));
    }

    if let Some((call, next_index)) = format_simple_call_tokens(tokens, start, config, start_column)
    {
        return Some((call, next_index));
    }

    if let Some((list, next_index)) = format_simple_list_tokens(tokens, start, config, start_column)
    {
        return Some((list, next_index));
    }

    if let Some((hash, next_index)) = format_simple_hash_tokens(tokens, start, config, start_column)
    {
        return Some((hash, next_index));
    }

    let token = tokens.get(start)?;
    let value = simple_value_text(token)?;
    Some((value.to_string(), start + 1))
}

fn format_simple_method_call_tokens(
    tokens: &[perl_parser_core::Token],
    start: usize,
    config: &FormatConfig,
    start_column: usize,
) -> Option<(String, usize)> {
    use perl_parser_core::TokenKind;

    let (mut expression, mut index) = format_variable_tokens(tokens, start)?;
    let mut saw_method = false;

    loop {
        if tokens.get(index)?.kind != TokenKind::Arrow {
            break;
        }
        let (method_call, next_index) =
            format_simple_method_call_segment(tokens, index, &expression, config, start_column)?;
        expression = method_call;
        index = next_index;
        saw_method = true;
    }

    saw_method.then_some((expression, index))
}

fn format_simple_method_call_segment(
    tokens: &[perl_parser_core::Token],
    arrow_index: usize,
    receiver: &str,
    config: &FormatConfig,
    start_column: usize,
) -> Option<(String, usize)> {
    use perl_parser_core::TokenKind;

    if tokens.get(arrow_index)?.kind != TokenKind::Arrow {
        return None;
    }

    let method = tokens.get(arrow_index + 1)?;
    if method.kind != TokenKind::Identifier
        || tokens.get(arrow_index + 2)?.kind != TokenKind::LeftParen
    {
        return None;
    }

    let mut args = Vec::new();
    let mut index = arrow_index + 3;
    if tokens.get(index)?.kind == TokenKind::RightParen {
        return Some((format!("{receiver}->{}()", method.text), index + 1));
    }

    let open = format!("{receiver}->{}(", method.text);
    let mut arg_column = start_column + open.chars().count();
    loop {
        let (arg, next_index) = format_simple_atom_tokens(tokens, index, config, arg_column)?;
        arg_column = advance_column(arg_column, &arg) + 2;
        args.push(arg);
        index = next_index;

        match tokens.get(index)?.kind {
            TokenKind::Comma => index += 1,
            TokenKind::RightParen => {
                return Some((
                    render_delimited_doc(&open, ")", &args, config, start_column),
                    index + 1,
                ));
            }
            _ => return None,
        }
    }
}

fn format_simple_call_tokens(
    tokens: &[perl_parser_core::Token],
    start: usize,
    config: &FormatConfig,
    start_column: usize,
) -> Option<(String, usize)> {
    use perl_parser_core::TokenKind;

    let name = tokens.get(start)?;
    if name.kind != TokenKind::Identifier || tokens.get(start + 1)?.kind != TokenKind::LeftParen {
        return None;
    }

    let mut args = Vec::new();
    let mut index = start + 2;
    if tokens.get(index)?.kind == TokenKind::RightParen {
        return Some((format!("{}()", name.text), index + 1));
    }

    let open = format!("{}(", name.text);
    let mut arg_column = start_column + open.chars().count();
    loop {
        let (arg, next_index) = format_simple_atom_tokens(tokens, index, config, arg_column)?;
        arg_column = advance_column(arg_column, &arg) + 2;
        args.push(arg);
        index = next_index;

        match tokens.get(index)?.kind {
            TokenKind::Comma => index += 1,
            TokenKind::RightParen => {
                return Some((
                    render_delimited_doc(&open, ")", &args, config, start_column),
                    index + 1,
                ));
            }
            _ => return None,
        }
    }
}

fn format_simple_list_tokens(
    tokens: &[perl_parser_core::Token],
    start: usize,
    config: &FormatConfig,
    start_column: usize,
) -> Option<(String, usize)> {
    use perl_parser_core::TokenKind;

    if tokens.get(start)?.kind != TokenKind::LeftParen {
        return None;
    }

    let mut items = Vec::new();
    let mut index = start + 1;
    if tokens.get(index)?.kind == TokenKind::RightParen {
        return Some(("()".to_string(), index + 1));
    }

    let mut item_column = start_column + 1;
    loop {
        let (item, next_index) = format_simple_atom_tokens(tokens, index, config, item_column)?;
        item_column = advance_column(item_column, &item) + 2;
        items.push(item);
        index = next_index;

        match tokens.get(index)?.kind {
            TokenKind::Comma => index += 1,
            TokenKind::RightParen => {
                return Some((
                    render_delimited_doc("(", ")", &items, config, start_column),
                    index + 1,
                ));
            }
            _ => return None,
        }
    }
}

fn format_simple_hash_tokens(
    tokens: &[perl_parser_core::Token],
    start: usize,
    config: &FormatConfig,
    start_column: usize,
) -> Option<(String, usize)> {
    use perl_parser_core::TokenKind;

    if tokens.get(start)?.kind != TokenKind::LeftBrace {
        return None;
    }

    let mut pairs = Vec::new();
    let mut index = start + 1;
    if tokens.get(index)?.kind == TokenKind::RightBrace {
        return Some(("{}".to_string(), index + 1));
    }

    loop {
        let key = format_simple_hash_key_token(tokens.get(index)?)?;
        index += 1;
        if tokens.get(index)?.kind != TokenKind::FatArrow {
            return None;
        }
        index += 1;

        let value_column = start_column + 1 + key.chars().count() + " => ".len();
        let (value, next_index) = format_simple_atom_tokens(tokens, index, config, value_column)?;
        pairs.push((key, value));
        index = next_index;

        match tokens.get(index)?.kind {
            TokenKind::Comma => index += 1,
            TokenKind::RightBrace => {
                return Some((render_simple_hash_doc(&pairs, config, start_column), index + 1));
            }
            _ => return None,
        }
    }
}

fn format_simple_hash_key_token(token: &perl_parser_core::Token) -> Option<String> {
    simple_value_text(token).map(str::to_string)
}

fn render_simple_hash_doc(
    pairs: &[(String, String)],
    config: &FormatConfig,
    start_column: usize,
) -> String {
    let items = pairs.iter().map(|(key, value)| format!("{key} => {value}")).collect::<Vec<_>>();
    render_delimited_doc("{", "}", &items, config, start_column)
}

fn render_delimited_doc(
    open: &str,
    close: &str,
    items: &[String],
    config: &FormatConfig,
    start_column: usize,
) -> String {
    let render_config = config_for_start_column(config, start_column);
    let mut parts = vec![FormatDoc::text(open)];
    if !items.is_empty() {
        let mut item_docs = vec![FormatDoc::if_break(FormatDoc::SoftLine, FormatDoc::text(""))];
        for (index, item) in items.iter().enumerate() {
            if index > 0 {
                item_docs.push(FormatDoc::text(","));
                item_docs.push(FormatDoc::SoftLine);
            }
            item_docs.push(FormatDoc::text(item));
        }
        if config.trailing_comma == TrailingComma::AddWhenWrapped {
            item_docs.push(FormatDoc::if_break(FormatDoc::text(","), FormatDoc::text("")));
        }
        parts.push(FormatDoc::indent(item_docs));
        parts.push(FormatDoc::if_break(FormatDoc::SoftLine, FormatDoc::text("")));
    }
    parts.push(FormatDoc::text(close));
    FormatDoc::group(parts).render(&render_config)
}

fn config_for_start_column(config: &FormatConfig, start_column: usize) -> FormatConfig {
    if config.line_width == u32::MAX {
        return config.clone();
    }

    let remaining = (config.line_width as usize).saturating_sub(start_column).max(1);
    FormatConfig { line_width: remaining.min(u32::MAX as usize) as u32, ..config.clone() }
}

fn advance_column(start_column: usize, text: &str) -> usize {
    if let Some((_, tail)) = text.rsplit_once('\n') {
        tail.chars().count()
    } else {
        start_column + text.chars().count()
    }
}

fn simple_binary_operator_text(token: &perl_parser_core::Token) -> Option<&str> {
    use perl_parser_core::TokenKind;

    matches!(
        token.kind,
        TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Percent
            | TokenKind::Dot
            | TokenKind::Equal
            | TokenKind::NotEqual
            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::StringCompare
            | TokenKind::Spaceship
            | TokenKind::And
            | TokenKind::Or
            | TokenKind::DefinedOr
            | TokenKind::WordAnd
            | TokenKind::WordOr
    )
    .then_some(token.text.as_ref())
}

fn indent_unit(config: &FormatConfig) -> String {
    if config.use_tabs { "\t".to_string() } else { " ".repeat(config.indent_width as usize) }
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
        if matches!(trimmed.trim_end(), "__DATA__" | "__END__") {
            return Some("DATA/END section");
        }
        if contains_likely_heredoc_start(line) {
            return Some("heredoc");
        }
        if is_format_declaration_start(trimmed) {
            return Some("format body");
        }
    }
    token_literal_preserve_region(source)
}

fn token_literal_preserve_region(source: &str) -> Option<&'static str> {
    use perl_parser_core::TokenKind;

    let mut stream = perl_parser_core::TokenStream::new(source);
    loop {
        let Ok(token) = stream.next() else {
            return None;
        };
        match token.kind {
            TokenKind::Eof => return None,
            TokenKind::Regex => return Some("regex literal"),
            TokenKind::Substitution => return Some("substitution operator"),
            TokenKind::Transliteration => return Some("transliteration operator"),
            TokenKind::QuoteSingle
            | TokenKind::QuoteDouble
            | TokenKind::QuoteWords
            | TokenKind::QuoteCommand => return Some("quote-like operator"),
            TokenKind::FormatBody => return Some("format body"),
            _ => {}
        }
    }
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

fn is_format_declaration_start(trimmed_line: &str) -> bool {
    if !trimmed_line.ends_with('=') {
        return false;
    }

    let Some(rest) = trimmed_line.strip_prefix("format") else {
        return false;
    };
    rest.is_empty() || rest.starts_with(char::is_whitespace)
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

#[cfg(test)]
mod tests {
    use super::{
        TextPosition, TextRange, literal_preserve_region, range_includes_line, split_line_ending,
        split_trailing_comment,
    };

    #[test]
    fn split_trailing_comment_ignores_hash_inside_backticks()
    -> Result<(), Box<dyn std::error::Error>> {
        let (code, comment) = split_trailing_comment("my$out=`printf '#value'`; # trailing");
        assert_eq!(code, "my$out=`printf '#value'`;");
        assert_eq!(comment, Some("# trailing"));

        let (code, comment) = split_trailing_comment("my$out=`printf '#value'`;");
        assert_eq!(code, "my$out=`printf '#value'`;");
        assert_eq!(comment, None);

        Ok(())
    }

    #[test]
    fn split_line_ending_preserves_crlf_lf_and_unterminated_lines()
    -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(split_line_ending("my $x = 1;\r\n"), ("my $x = 1;", "\r\n"));
        assert_eq!(split_line_ending("my $x = 1;\n"), ("my $x = 1;", "\n"));
        assert_eq!(split_line_ending("my $x = 1;"), ("my $x = 1;", ""));

        Ok(())
    }

    #[test]
    fn range_includes_line_treats_zero_width_end_line_as_exclusive()
    -> Result<(), Box<dyn std::error::Error>> {
        let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(3, 0));
        assert!(!range_includes_line(range, 0));
        assert!(range_includes_line(range, 1));
        assert!(range_includes_line(range, 2));
        assert!(!range_includes_line(range, 3));

        let range = TextRange::new(TextPosition::new(1, 0), TextPosition::new(3, 4));
        assert!(range_includes_line(range, 3));

        Ok(())
    }

    #[test]
    fn literal_preserve_region_detects_perl_constructs_that_must_not_be_reflowed()
    -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(literal_preserve_region("=head1 NAME\nDemo\n=cut\n"), Some("POD"));
        assert_eq!(
            literal_preserve_region("my $x = 1;\n__DATA__\nraw\n"),
            Some("DATA/END section")
        );
        assert_eq!(literal_preserve_region("my $text = <<~'EOF';\nbody\nEOF\n"), Some("heredoc"));
        assert_eq!(literal_preserve_region("format STDOUT =\n@<<<<\n$x\n.\n"), Some("format body"));
        assert_eq!(literal_preserve_region("my $x = 1;\n"), None);

        Ok(())
    }
}
