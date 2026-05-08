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
        .or_else(|| format_simple_statement_line(line))
        .or_else(|| format_simple_lexical_line(line))
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

fn format_simple_subroutine_line(line: &str, config: &FormatConfig) -> Option<String> {
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

    let formatted = format_simple_subroutine_tokens(&tokens, indent, config)?;
    Some(formatted)
}

fn format_simple_control_block_line(line: &str, config: &FormatConfig) -> Option<String> {
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

    format_simple_control_block_tokens(&tokens, indent, config)
}

fn format_simple_statement_line(line: &str) -> Option<String> {
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

    let formatted = format_simple_statement_tokens(&tokens)?;
    Some(format!("{indent}{formatted}"))
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
    let statements = format_simple_statement_block(body_tokens)?;
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

    let (condition, next_index) = format_simple_condition_tokens(tokens, 2)?;
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
    let statements = format_simple_statement_block(body_tokens)?;
    let else_statements = format_simple_else_branch(tokens, body_end, keyword)?;

    if else_statements.is_none() && body_end + 1 != tokens.len() {
        return None;
    }

    let body_indent = format!("{indent}{}", indent_unit(config));
    let mut formatted = render_simple_block_doc(
        format!("{indent}{keyword} ({condition}) {{"),
        &statements,
        indent,
        &body_indent,
        config,
    );

    if let Some(else_statements) = else_statements {
        formatted.push_str(&render_simple_else_doc(&else_statements, indent, &body_indent, config));
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
    let mut parts = vec![FormatDoc::text(header)];
    push_simple_block_body_docs(&mut parts, statements, indent, body_indent);
    FormatDoc::group(parts).render(config)
}

fn render_simple_else_doc(
    statements: &[String],
    indent: &str,
    body_indent: &str,
    config: &FormatConfig,
) -> String {
    let mut parts = vec![FormatDoc::text(" else {")];
    push_simple_block_body_docs(&mut parts, statements, indent, body_indent);
    FormatDoc::group(parts).render(config)
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

fn format_simple_else_branch(
    tokens: &[perl_parser_core::Token],
    body_end: usize,
    keyword: &str,
) -> Option<Option<Vec<String>>> {
    use perl_parser_core::TokenKind;

    let next = body_end + 1;
    if next == tokens.len() {
        return Some(None);
    }

    if !matches!(keyword, "if" | "unless") {
        return None;
    }
    if tokens.get(next)?.kind != TokenKind::Else
        || tokens.get(next + 1)?.kind != TokenKind::LeftBrace
        || tokens.last()?.kind != TokenKind::RightBrace
    {
        return None;
    }

    let else_body_start = next + 2;
    let else_body_tokens = &tokens[else_body_start..tokens.len() - 1];
    let statements = format_simple_statement_block(else_body_tokens)?;
    Some(Some(statements))
}

fn format_simple_statement_block(tokens: &[perl_parser_core::Token]) -> Option<Vec<String>> {
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
        statements.push(format_simple_statement_tokens(statement_tokens)?);
        start = idx + 1;
    }

    (start == tokens.len()).then_some(statements)
}

fn format_simple_statement_tokens(tokens: &[perl_parser_core::Token]) -> Option<String> {
    format_simple_lexical_tokens(tokens)
        .or_else(|| format_simple_return_tokens(tokens))
        .or_else(|| format_simple_assignment_tokens(tokens))
        .or_else(|| format_simple_expression_statement_tokens(tokens))
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
    } else if tokens[next_index].kind == TokenKind::Assign {
        let value = format_simple_expression_tokens(tokens, next_index + 1, semicolon_index)?;
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

fn format_simple_return_tokens(tokens: &[perl_parser_core::Token]) -> Option<String> {
    use perl_parser_core::TokenKind;

    if tokens.first()?.kind != TokenKind::Return || tokens.last()?.kind != TokenKind::Semicolon {
        return None;
    }

    let semicolon_index = tokens.len() - 1;
    if semicolon_index == 1 {
        return Some("return;".to_string());
    }

    let value = format_simple_expression_tokens(tokens, 1, semicolon_index)?;
    Some(format!("return {value};"))
}

fn format_simple_assignment_tokens(tokens: &[perl_parser_core::Token]) -> Option<String> {
    use perl_parser_core::TokenKind;

    if tokens.last()?.kind != TokenKind::Semicolon {
        return None;
    }

    let (variable, next_index) = format_variable_tokens(tokens, 0)?;
    let semicolon_index = tokens.len() - 1;
    if tokens.get(next_index)?.kind != TokenKind::Assign {
        return None;
    }

    let value = format_simple_expression_tokens(tokens, next_index + 1, semicolon_index)?;
    Some(format!("{variable} = {value};"))
}

fn format_simple_expression_statement_tokens(tokens: &[perl_parser_core::Token]) -> Option<String> {
    use perl_parser_core::TokenKind;

    if tokens.last()?.kind != TokenKind::Semicolon {
        return None;
    }

    let semicolon_index = tokens.len() - 1;
    let (call, next_index) = format_simple_call_tokens(tokens, 0)?;
    (next_index == semicolon_index).then(|| format!("{call};"))
}

fn format_simple_condition_tokens(
    tokens: &[perl_parser_core::Token],
    start: usize,
) -> Option<(String, usize)> {
    use perl_parser_core::TokenKind;

    let end = tokens[start..]
        .iter()
        .position(|token| token.kind == TokenKind::RightParen)
        .map(|offset| start + offset)?;
    let condition = format_simple_expression_tokens(tokens, start, end)?;
    Some((condition, end))
}

fn format_simple_expression_tokens(
    tokens: &[perl_parser_core::Token],
    start: usize,
    end: usize,
) -> Option<String> {
    let (left, next_index) = format_simple_atom_tokens(tokens, start)?;
    if next_index == end {
        return Some(left);
    }

    let operator = simple_binary_operator_text(tokens.get(next_index)?)?;
    let (right, final_index) = format_simple_atom_tokens(tokens, next_index + 1)?;
    (final_index == end).then(|| format!("{left} {operator} {right}"))
}

fn format_simple_atom_tokens(
    tokens: &[perl_parser_core::Token],
    start: usize,
) -> Option<(String, usize)> {
    if let Some((variable, next_index)) = format_variable_tokens(tokens, start) {
        return Some((variable, next_index));
    }

    if let Some((call, next_index)) = format_simple_call_tokens(tokens, start) {
        return Some((call, next_index));
    }

    let token = tokens.get(start)?;
    let value = simple_value_text(token)?;
    Some((value.to_string(), start + 1))
}

fn format_simple_call_tokens(
    tokens: &[perl_parser_core::Token],
    start: usize,
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

    loop {
        let (arg, next_index) = format_simple_atom_tokens(tokens, index)?;
        args.push(arg);
        index = next_index;

        match tokens.get(index)?.kind {
            TokenKind::Comma => index += 1,
            TokenKind::RightParen => {
                return Some((format!("{}({})", name.text, args.join(", ")), index + 1));
            }
            _ => return None,
        }
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
