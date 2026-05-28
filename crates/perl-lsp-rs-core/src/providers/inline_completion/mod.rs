//! Inline completions provider with deterministic rules and AI backend support.
//!
//! This crate provides context-aware inline completions that appear as
//! ghost text. Deterministic completions are based on patterns; AI-powered
//! suggestions use the `InlineCompletionBackend` trait for pluggable providers.

use perl_lexer::{PerlLexer, TokenType};
use perl_parser_core::{Parser, RecoverySalvageProfile};
use perl_position_tracking::utf16_line_col_to_offset;
use serde::{Deserialize, Serialize};

const MAX_INLINE_COMPLETION_ITEMS: usize = 5;

/// Prepared context for inline completion suggestions and future AI handoff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedInlineCompletionContext {
    /// Prefix on the current line up to the request position.
    pub prefix: String,
    /// Full current line with trailing newline removed.
    pub current_line: String,
    /// Closest previous non-empty line, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_non_empty_line: Option<String>,
    /// Nearest enclosing subroutine name, if one can be inferred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_function: Option<String>,
    /// Nearest package declaration before the cursor, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_package: Option<String>,
    /// Nearby variables, ordered from closest to farthest.
    pub variables: Vec<String>,
    /// Imported modules or pragmas visible before the cursor.
    pub imports: Vec<String>,
}

/// Request-local facts supplied by the LSP runtime.
///
/// The deterministic provider remains usable with only source text, but the
/// runtime can pass workspace-derived facts here so inline completion can
/// prefer project-aware suggestions without depending on runtime state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InlineCompletionEnvironment {
    /// Modules reachable from the current document's effective `@INC`.
    pub available_modules: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SemanticInlineContext {
    pub(crate) lexical_scope: InlineLexicalScope,
    pub(crate) package: Option<String>,
    pub(crate) enclosing_sub: Option<String>,
    pub(crate) expected_syntax: ExpectedSyntax,
    pub(crate) visible_variables: Vec<VariableFact>,
    pub(crate) receiver_hint: Option<ReceiverHint>,
    pub(crate) dbi_receiver_kind: Option<DbiReceiverKind>,
    pub(crate) imported_modules: Vec<ModuleFact>,
    pub(crate) available_modules: Vec<ModuleFact>,
    pub(crate) current_package_methods: Vec<MethodFact>,
    pub(crate) file_role: FileRole,
    pub(crate) style: InlineStyleContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InlineLexicalScope {
    File,
    Subroutine(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpectedSyntax {
    EmptyStatement,
    UseModule,
    MethodName,
    LexicalVariableName,
    PackageName,
    BlessArguments,
    ReturnExpression,
    LoopBinding,
    TestAssertionArguments,
    ShebangInterpreter,
    SubroutineBody,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VariableFact {
    pub(crate) sigil: VariableSigil,
    pub(crate) name: String,
}

impl VariableFact {
    fn from_perl_variable(variable: &str) -> Option<Self> {
        let mut chars = variable.chars();
        let sigil = VariableSigil::from_char(chars.next()?)?;
        let name: String = chars.collect();
        (!name.is_empty()).then_some(Self { sigil, name })
    }

    fn as_perl_variable(&self) -> String {
        format!("{}{}", self.sigil.as_char(), self.name)
    }

    fn is_scalar_self(&self) -> bool {
        self.sigil == VariableSigil::Scalar && self.name == "self"
    }

    fn is_scalar(&self) -> bool {
        self.sigil == VariableSigil::Scalar
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VariableSigil {
    Scalar,
    Array,
    Hash,
}

impl VariableSigil {
    fn from_char(ch: char) -> Option<Self> {
        match ch {
            '$' => Some(Self::Scalar),
            '@' => Some(Self::Array),
            '%' => Some(Self::Hash),
            _ => None,
        }
    }

    fn as_char(self) -> char {
        match self {
            Self::Scalar => '$',
            Self::Array => '@',
            Self::Hash => '%',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModuleFact {
    pub(crate) name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MethodFact {
    pub(crate) name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReceiverHint {
    SelfReceiver,
    Variable(VariableFact),
    Package(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DbiReceiverKind {
    DatabaseHandle,
    StatementHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileRole {
    Module,
    Script,
    Test,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InlineStyleContext {
    pub(crate) indentation: IndentationStyle,
    pub(crate) language_prelude: LanguagePreludeStyle,
    pub(crate) sub_argument_style: SubArgumentStyle,
    pub(crate) constructor_style: ConstructorStyle,
    pub(crate) test_framework: TestFramework,
}

impl InlineStyleContext {
    fn unknown(context: &PreparedInlineCompletionContext) -> Self {
        Self {
            indentation: indentation_style_from_line(context.current_line.as_str()),
            language_prelude: LanguagePreludeStyle::from_imports(&context.imports),
            sub_argument_style: SubArgumentStyle::Unknown,
            constructor_style: ConstructorStyle::Unknown,
            test_framework: TestFramework::from_imports(&context.imports),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndentationStyle {
    Spaces(usize),
    Tabs,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LanguagePreludeStyle {
    ModernPerl,
    StrictWarnings,
    StrictOnly,
    WarningsOnly,
    Unknown,
}

impl LanguagePreludeStyle {
    fn from_imports(imports: &[String]) -> Self {
        if imports.iter().any(|import| import == "Modern::Perl") {
            return Self::ModernPerl;
        }

        let has_strict = imports.iter().any(|import| import == "strict");
        let has_warnings = imports.iter().any(|import| import == "warnings");
        match (has_strict, has_warnings) {
            (true, true) => Self::StrictWarnings,
            (true, false) => Self::StrictOnly,
            (false, true) => Self::WarningsOnly,
            (false, false) => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubArgumentStyle {
    AtUnderscore,
    Shift,
    Signature,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConstructorStyle {
    BlessHashReturnSelf,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TestFramework {
    Test2V0,
    TestMore,
    Unknown,
}

impl TestFramework {
    fn from_imports(imports: &[String]) -> Self {
        if imports.iter().any(|import| import == "Test2::V0") {
            return Self::Test2V0;
        }
        if imports.iter().any(|import| import == "Test::More") {
            return Self::TestMore;
        }
        Self::Unknown
    }
}

/// Inline completion item (LSP 3.18 preview)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineCompletionItem {
    /// The text to be inserted.
    pub insert_text: String,
    /// The text to be used for filtering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_text: Option<String>,
    /// The range to be replaced by the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<lsp_types::Range>,
    /// An optional command to be executed after the completion is inserted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<lsp_types::Command>,
}

/// Inline completion list (LSP 3.18 preview)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineCompletionList {
    /// The inline completion items.
    pub items: Vec<InlineCompletionItem>,
}

// ── AI backend interface ─────────────────────────────────────────────────────

/// Error type for backend operations.
#[derive(Debug)]
pub enum BackendError {
    /// Network or IO error.
    Transport(String),
    /// Authentication failure (bad key, expired token).
    Auth(String),
    /// Provider returned an error response.
    Provider(String),
    /// Request timed out.
    Timeout,
    /// Rate limit exceeded.
    RateLimited,
    /// Request was cancelled.
    Cancelled,
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(msg) => write!(f, "transport error: {}", msg),
            Self::Auth(msg) => write!(f, "auth error: {}", msg),
            Self::Provider(msg) => write!(f, "provider error: {}", msg),
            Self::Timeout => write!(f, "request timed out"),
            Self::RateLimited => write!(f, "rate limit exceeded"),
            Self::Cancelled => write!(f, "request cancelled"),
        }
    }
}

impl std::error::Error for BackendError {}

/// Request payload sent to an AI completion backend.
#[derive(Debug, Clone)]
pub struct BackendRequest {
    /// Prepared context from the current buffer.
    pub context: PreparedInlineCompletionContext,
    /// Maximum tokens to generate.
    pub max_output_tokens: u32,
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
}

/// A chunk emitted by a streaming backend.
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// Cumulative candidate text so far (NOT a delta).
    pub text: String,
    /// Whether this is the final chunk.
    pub is_final: bool,
}

/// Control signal returned by the stream sink callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamControl {
    /// Continue receiving chunks.
    Continue,
    /// Stop the stream early.
    Stop,
}

/// Trait for AI inline completion backends.
///
/// Implementations provide streaming token generation. The default `complete()`
/// method buffers the stream into a one-shot result, so backends only need to
/// implement `stream()`.
///
/// The trait is sync and callback-based to keep this crate dependency-light
/// and runtime-agnostic. Network I/O happens in the provider crate.
pub trait InlineCompletionBackend: Send + Sync {
    /// One-shot completion: returns the final candidate texts.
    ///
    /// Default implementation buffers the stream.
    fn complete(&self, req: &BackendRequest) -> Result<Vec<String>, BackendError> {
        let mut final_text = String::new();
        self.stream(req, &mut |chunk| {
            final_text = chunk.text.clone();
            if chunk.is_final { StreamControl::Stop } else { StreamControl::Continue }
        })?;
        Ok(if final_text.is_empty() { vec![] } else { vec![final_text] })
    }

    /// Stream completion chunks to a callback sink.
    ///
    /// Each `StreamChunk.text` is **cumulative** — the full candidate so far,
    /// not a delta. The sink returns `StreamControl::Stop` to cancel early.
    fn stream(
        &self,
        req: &BackendRequest,
        sink: &mut dyn FnMut(StreamChunk) -> StreamControl,
    ) -> Result<(), BackendError>;
}

#[derive(Debug)]
struct RankedCompletionItem {
    score: InlineCandidateScore,
    order: usize,
    item: InlineCompletionItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InlineCandidateScore(i16);

impl InlineCandidateScore {
    const LEGACY_PRIORITY_STEP: i16 = 100;

    fn for_candidate(
        source: InlineCandidateSourceKind,
        priority: u8,
        item: &InlineCompletionItem,
        semantic_context: &SemanticInlineContext,
    ) -> Self {
        Self(Self::legacy_base(priority) + semantic_bonus(source, item, semantic_context))
    }

    fn legacy_base(priority: u8) -> i16 {
        10_000 - i16::from(priority) * Self::LEGACY_PRIORITY_STEP
    }

    #[cfg(test)]
    fn from_legacy_priority(priority: u8) -> Self {
        Self(Self::legacy_base(priority))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InlineCandidateSourceKind {
    Receiver,
    Module,
    Syntax,
    Test,
    Shebang,
    ContextualFallback,
}

fn semantic_bonus(
    source: InlineCandidateSourceKind,
    item: &InlineCompletionItem,
    context: &SemanticInlineContext,
) -> i16 {
    match source {
        InlineCandidateSourceKind::Receiver => receiver_candidate_bonus(item, context),
        InlineCandidateSourceKind::Module => module_candidate_bonus(item, context),
        InlineCandidateSourceKind::Syntax => syntax_candidate_bonus(item, context),
        InlineCandidateSourceKind::Test => test_candidate_bonus(context),
        InlineCandidateSourceKind::Shebang => shebang_candidate_bonus(context),
        InlineCandidateSourceKind::ContextualFallback => {
            contextual_fallback_candidate_bonus(item, context)
        }
    }
}

fn module_candidate_bonus(item: &InlineCompletionItem, context: &SemanticInlineContext) -> i16 {
    if context.expected_syntax != ExpectedSyntax::UseModule {
        return 0;
    }

    let module_name = item.insert_text.trim_end_matches(';');
    if context
        .available_modules
        .binary_search_by(|module| module.name.as_str().cmp(module_name))
        .is_ok()
    {
        return 35;
    }

    0
}

fn receiver_candidate_bonus(item: &InlineCompletionItem, context: &SemanticInlineContext) -> i16 {
    if context.expected_syntax != ExpectedSyntax::MethodName {
        return 0;
    }

    let method_name = item.insert_text.trim_end_matches("()");
    if context.current_package_methods.iter().any(|method| method.name == method_name) {
        return 30;
    }

    10
}

fn syntax_candidate_bonus(item: &InlineCompletionItem, context: &SemanticInlineContext) -> i16 {
    match context.expected_syntax {
        ExpectedSyntax::UseModule
            if matches!(
                item.insert_text.as_str(),
                "strict;" | "warnings;" | "feature ':5.36';"
            ) =>
        {
            20
        }
        ExpectedSyntax::ReturnExpression if item.insert_text.ends_with(';') => 20,
        ExpectedSyntax::LexicalVariableName
            if item.insert_text.starts_with("self =")
                && context.visible_variables.iter().any(VariableFact::is_scalar_self) =>
        {
            20
        }
        ExpectedSyntax::PackageName
        | ExpectedSyntax::BlessArguments
        | ExpectedSyntax::LoopBinding => 15,
        ExpectedSyntax::SubroutineBody if item.insert_text.starts_with(" {") => 15,
        _ => 0,
    }
}

fn test_candidate_bonus(context: &SemanticInlineContext) -> i16 {
    match context.expected_syntax {
        ExpectedSyntax::TestAssertionArguments => 30,
        _ if context.file_role == FileRole::Test => 20,
        _ => 0,
    }
}

fn shebang_candidate_bonus(context: &SemanticInlineContext) -> i16 {
    if context.expected_syntax == ExpectedSyntax::ShebangInterpreter { 20 } else { 0 }
}

fn contextual_fallback_candidate_bonus(
    item: &InlineCompletionItem,
    context: &SemanticInlineContext,
) -> i16 {
    if context.file_role == FileRole::Test
        && (item.insert_text.starts_with("is(") || item.insert_text.starts_with("ok("))
    {
        return 25;
    }

    if item.insert_text.starts_with("return ")
        && matches!(context.expected_syntax, ExpectedSyntax::EmptyStatement)
        && !context.visible_variables.is_empty()
    {
        return 15;
    }

    if item.insert_text == "done_testing();" && context.file_role == FileRole::Test {
        return 10;
    }

    0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParseDamage {
    terminated_early: bool,
    error_node_count: usize,
    diagnostics_count: usize,
    recovered_count: usize,
}

impl ParseDamage {
    fn worse_than(&self, baseline: &Self) -> bool {
        (self.terminated_early && !baseline.terminated_early)
            || self.error_node_count > baseline.error_node_count
            || self.diagnostics_count > baseline.diagnostics_count
            || self.recovered_count > baseline.recovered_count
    }
}

fn parse_damage_for_probe(source: &str) -> ParseDamage {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let salvage = RecoverySalvageProfile::from_parse(&output.ast, &output.diagnostics, false);

    ParseDamage {
        terminated_early: output.terminated_early,
        error_node_count: salvage.error_node_count,
        diagnostics_count: output.error_count(),
        recovered_count: output.recovered_count,
    }
}

fn parse_probe_after_item(
    current_line: &str,
    item: &InlineCompletionItem,
    line: u32,
    character: u32,
) -> Option<String> {
    if item.insert_text.contains(['\n', '\r']) {
        return None;
    }

    let (start_character, end_character) = item
        .range
        .as_ref()
        .map(|range| {
            if range.start.line != line || range.end.line != line {
                return None;
            }
            Some((range.start.character, range.end.character))
        })
        .unwrap_or(Some((character, character)))?;

    let start = utf16_line_col_to_offset(current_line, 0, start_character);
    let end = utf16_line_col_to_offset(current_line, 0, end_character);
    if start > end {
        return None;
    }

    let mut probe = String::with_capacity(current_line.len() + item.insert_text.len());
    probe.push_str(&current_line[..start]);
    probe.push_str(item.insert_text.as_str());
    probe.push_str(&current_line[end..]);
    Some(probe)
}

#[derive(Debug)]
struct InlineCandidateSink<'a> {
    semantic_context: &'a SemanticInlineContext,
    items: Vec<RankedCompletionItem>,
    sequence: usize,
}

impl<'a> InlineCandidateSink<'a> {
    fn new(semantic_context: &'a SemanticInlineContext) -> Self {
        Self { semantic_context, items: Vec::new(), sequence: 0 }
    }

    fn push(
        &mut self,
        source: InlineCandidateSourceKind,
        priority: u8,
        item: InlineCompletionItem,
    ) {
        let score =
            InlineCandidateScore::for_candidate(source, priority, &item, self.semantic_context);
        self.items.push(RankedCompletionItem { score, order: self.sequence, item });
        self.sequence += 1;
    }

    fn into_items(self) -> Vec<RankedCompletionItem> {
        self.items
    }
}

trait InlineCandidateSource {
    const SOURCE: InlineCandidateSourceKind;

    fn add_candidates(
        &self,
        provider: &InlineCompletionProvider,
        context: &PreparedInlineCompletionContext,
        semantic_context: &SemanticInlineContext,
        sink: &mut InlineCandidateSink<'_>,
    );
}

#[derive(Debug, Clone, Copy)]
struct ReceiverCandidateSource;

#[derive(Debug, Clone, Copy)]
struct ModuleCandidateSource;

#[derive(Debug, Clone, Copy)]
struct SyntaxCandidateSource;

#[derive(Debug, Clone, Copy)]
struct TestCandidateSource;

#[derive(Debug, Clone, Copy)]
struct ShebangCandidateSource;

#[derive(Debug, Clone, Copy)]
struct ContextualFallbackSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HardRejectZone {
    Comment,
    StringLike,
    HeredocBody,
    Pod,
    RegexLike,
}

/// A provider for inline completions.
pub struct InlineCompletionProvider;

impl Default for InlineCompletionProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl InlineCompletionProvider {
    /// Creates a new `InlineCompletionProvider`.
    pub fn new() -> Self {
        Self
    }

    /// Get inline completions for the given context
    pub fn get_inline_completions(
        &self,
        text: &str,
        line: u32,
        character: u32,
    ) -> InlineCompletionList {
        self.get_inline_completions_with_environment(
            text,
            line,
            character,
            &InlineCompletionEnvironment::default(),
        )
    }

    /// Get inline completions using request-local semantic environment facts.
    pub fn get_inline_completions_with_environment(
        &self,
        text: &str,
        line: u32,
        character: u32,
        environment: &InlineCompletionEnvironment,
    ) -> InlineCompletionList {
        if let Some(context) = self.prepare_context(text, line, character) {
            let semantic_context =
                self.semantic_context_for_source_with_environment(text, &context, environment);
            let items = self.get_completions_for_context(&context, &semantic_context);
            let list = self.apply_replacement_ranges_for_context(
                InlineCompletionList { items },
                &context,
                line,
                character,
            );
            return self.filter_parse_safe_items(list, &context, line, character);
        }

        InlineCompletionList { items: vec![] }
    }

    /// Add an explicit single-line replacement range when the user has already
    /// typed part of the token the completion would finish.
    pub fn apply_replacement_ranges_for_context(
        &self,
        mut list: InlineCompletionList,
        context: &PreparedInlineCompletionContext,
        line: u32,
        character: u32,
    ) -> InlineCompletionList {
        let Some(fragment) = replacement_fragment_at_cursor(context.prefix.as_str()) else {
            return list;
        };
        let Some(range) = replacement_range(context.prefix.as_str(), &fragment, line, character)
        else {
            return list;
        };

        for item in &mut list.items {
            if item.range.is_none() && item_matches_fragment(item, fragment.text) {
                item.range = Some(range);
            }
        }

        list
    }

    fn filter_parse_safe_items(
        &self,
        list: InlineCompletionList,
        context: &PreparedInlineCompletionContext,
        line: u32,
        character: u32,
    ) -> InlineCompletionList {
        let baseline = parse_damage_for_probe(context.current_line.as_str());
        let items = list
            .items
            .into_iter()
            .filter(|item| {
                parse_probe_after_item(context.current_line.as_str(), item, line, character)
                    .map(|probe| {
                        let candidate = parse_damage_for_probe(probe.as_str());
                        !candidate.worse_than(&baseline)
                    })
                    .unwrap_or(true)
            })
            .collect();

        InlineCompletionList { items }
    }

    /// Prepare surrounding code context for deterministic suggestions and
    /// future LLM-backed inline completion.
    pub fn prepare_context(
        &self,
        text: &str,
        line: u32,
        character: u32,
    ) -> Option<PreparedInlineCompletionContext> {
        let line_context = self.line_context_at_position(text, line, character)?;
        let cursor_offset = utf16_line_col_to_offset(text, line, character);
        if hard_reject_zone_at_cursor(text, line_context.prefix, cursor_offset).is_some() {
            return None;
        }

        let lines = self.normalized_lines(text);
        let line_index = usize::try_from(line).ok()?;
        let (current_function, function_start_line) =
            self.current_function_context(&lines, line_index);
        let visible_text = self.visible_text_until_cursor(&lines, line_index, line_context.prefix);
        let variable_scan_text = self.visible_text_since_line(
            &lines,
            function_start_line.unwrap_or(0),
            line_index,
            line_context.prefix,
        );

        Some(PreparedInlineCompletionContext {
            prefix: line_context.prefix.to_string(),
            current_line: line_context.current_line.to_string(),
            previous_non_empty_line: self
                .previous_non_empty_line(&lines, line_index)
                .map(str::to_string),
            current_function,
            current_package: self.current_package(&lines, line_index),
            variables: self.collect_variables(&variable_scan_text),
            imports: self.collect_imports(&visible_text),
        })
    }

    fn line_context_at_position<'a>(
        &self,
        text: &'a str,
        line: u32,
        character: u32,
    ) -> Option<LineContext<'a>> {
        let lines = self.normalized_lines(text);
        let line_index = usize::try_from(line).ok()?;
        let current_line = *lines.get(line_index)?;
        let prefix_end = utf16_line_col_to_offset(current_line, 0, character);

        Some(LineContext { prefix: &current_line[..prefix_end], current_line })
    }

    fn normalized_lines<'a>(&self, text: &'a str) -> Vec<&'a str> {
        if text.is_empty() {
            return vec![""];
        }

        text.split('\n').map(|line| line.strip_suffix('\r').unwrap_or(line)).collect()
    }

    fn get_completions_for_context(
        &self,
        context: &PreparedInlineCompletionContext,
        semantic_context: &SemanticInlineContext,
    ) -> Vec<InlineCompletionItem> {
        let mut sink = InlineCandidateSink::new(semantic_context);
        ReceiverCandidateSource.add_candidates(self, context, semantic_context, &mut sink);
        ModuleCandidateSource.add_candidates(self, context, semantic_context, &mut sink);
        SyntaxCandidateSource.add_candidates(self, context, semantic_context, &mut sink);
        TestCandidateSource.add_candidates(self, context, semantic_context, &mut sink);
        ShebangCandidateSource.add_candidates(self, context, semantic_context, &mut sink);
        ContextualFallbackSource.add_candidates(self, context, semantic_context, &mut sink);

        self.normalize_items(sink.into_items())
    }

    /// Check if we're after a sub declaration without body
    fn match_sub_declaration(&self, prefix: &str) -> Option<String> {
        let idx = last_keyword_index(prefix, "sub ")?;
        let after_sub = &prefix[idx + 4..];
        if after_sub.is_empty() || after_sub.contains('{') || after_sub.contains('(') {
            return None;
        }
        let name = after_sub.trim();
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return None;
        }
        Some(name.to_string())
    }

    /// Check if we're in a constructor context (sub new or BUILD)
    fn is_in_constructor_context(&self, current_function: Option<&str>, prefix: &str) -> bool {
        matches!(current_function, Some("new" | "BUILD"))
            || contains_keyword(prefix, "sub new")
            || contains_keyword(prefix, "sub BUILD")
    }

    /// Generate a smart subroutine body based on naming patterns
    ///
    /// Detects common Perl subroutine naming conventions and generates
    /// appropriate body templates:
    /// - `new`, `BUILD` -> constructor pattern
    /// - `get_*` -> getter pattern
    /// - `set_*` -> setter pattern
    /// - `is_*`, `has_*`, `can_*` -> boolean accessor pattern
    /// - `_*` -> private method placeholder
    /// - default -> simple method template
    fn generate_subroutine_completion(
        &self,
        sub_name: &str,
        semantic_context: &SemanticInlineContext,
    ) -> String {
        if sub_name == "new"
            && semantic_context.style.sub_argument_style == SubArgumentStyle::Signature
        {
            return format!(
                " ($class, %args) {{\n{}\n}}",
                self.generate_smart_body(sub_name, semantic_context)
            );
        }

        format!(" {{\n{}\n}}", self.generate_smart_body(sub_name, semantic_context))
    }

    fn generate_smart_body(
        &self,
        sub_name: &str,
        semantic_context: &SemanticInlineContext,
    ) -> String {
        // Constructor patterns
        if sub_name == "new" || sub_name == "BUILD" {
            return constructor_body(semantic_context.style.sub_argument_style);
        }

        // Getter pattern: get_something or something_getter
        if let Some(field) = sub_name.strip_prefix("get_") {
            // Remove "get_" prefix
            return format!("    my $self = shift;\n    return $self->{{{}}};", field);
        }

        // Setter pattern: set_something or something_setter
        if let Some(field) = sub_name.strip_prefix("set_") {
            // Remove "set_" prefix
            return format!(
                "    my ($self, $value) = @_;\n    $self->{{{}}} = $value;\n    return $self;",
                field
            );
        }

        // Boolean accessor patterns: is_*, has_*, can_*
        if sub_name.starts_with("is_")
            || sub_name.starts_with("has_")
            || sub_name.starts_with("can_")
        {
            let prefix_len = if sub_name.starts_with("is_") { 3 } else { 4 };
            let field = &sub_name[prefix_len..];
            return format!("    my $self = shift;\n    return $self->{{{}}} ? 1 : 0;", field);
        }

        // Private method placeholder
        if sub_name.starts_with('_') {
            return "    my $self = shift;\n    ...".to_string();
        }

        // Default: simple method with shift
        "    my $self = shift;\n    ...".to_string()
    }

    fn current_function_context(
        &self,
        lines: &[&str],
        line_index: usize,
    ) -> (Option<String>, Option<usize>) {
        lines.iter().take(line_index + 1).enumerate().fold(
            (None, None),
            |mut state, (idx, line)| {
                if let Some(name) = self.parse_sub_name(line) {
                    state = (Some(name), Some(idx));
                }
                state
            },
        )
    }

    fn current_package(&self, lines: &[&str], line_index: usize) -> Option<String> {
        let mut scanner = PackageScopeScanner::new();
        for line in lines.iter().take(line_index + 1) {
            scanner.advance(line, self.parse_package_name(line));
        }
        scanner.current_package().map(str::to_string)
    }

    fn previous_non_empty_line<'a>(
        &self,
        lines: &'a [&'a str],
        line_index: usize,
    ) -> Option<&'a str> {
        lines
            .get(..line_index)
            .and_then(|slice| slice.iter().rev().find(|line| !line.trim().is_empty()).copied())
    }

    fn visible_text_until_cursor(&self, lines: &[&str], line_index: usize, prefix: &str) -> String {
        self.visible_text_since_line(lines, 0, line_index, prefix)
    }

    fn visible_text_since_line(
        &self,
        lines: &[&str],
        start_line: usize,
        line_index: usize,
        prefix: &str,
    ) -> String {
        let mut visible_text = String::new();

        for (idx, line) in
            lines.iter().enumerate().skip(start_line).take(line_index.saturating_sub(start_line))
        {
            if idx > start_line {
                visible_text.push('\n');
            }
            visible_text.push_str(line);
        }

        if line_index > start_line || !visible_text.is_empty() {
            visible_text.push('\n');
        }
        visible_text.push_str(prefix);
        visible_text
    }

    fn collect_imports(&self, visible_text: &str) -> Vec<String> {
        let mut imports = Vec::new();

        for line in visible_text.lines() {
            if let Some(import_name) = self.parse_use_name(line) {
                self.push_unique(&mut imports, import_name);
            }
        }

        imports
    }

    fn collect_variables(&self, visible_text: &str) -> Vec<String> {
        let mut variables = Vec::new();
        for declaration_group in collect_declared_variable_groups(visible_text).into_iter().rev() {
            for variable in declaration_group {
                self.push_unique_variable(&mut variables, variable);
            }
        }

        variables
    }

    fn parse_use_name(&self, line: &str) -> Option<String> {
        let trimmed = line.trim_start();
        let rest = trimmed.strip_prefix("use ")?;
        let name: String = rest
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_'))
            .collect();

        (!name.is_empty()).then_some(name)
    }

    fn parse_sub_name(&self, line: &str) -> Option<String> {
        let trimmed = line.trim_start();
        let rest = trimmed.strip_prefix("sub ")?;
        let name: String = rest
            .chars()
            .skip_while(|ch| ch.is_whitespace())
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect();

        (!name.is_empty()).then_some(name)
    }

    fn parse_package_name(&self, line: &str) -> Option<String> {
        let trimmed = line.trim_start();
        let rest = trimmed.strip_prefix("package ")?;
        let name: String = rest
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_'))
            .collect();

        (!name.is_empty()).then_some(name)
    }

    fn semantic_context_for_prepared_context(
        &self,
        context: &PreparedInlineCompletionContext,
    ) -> SemanticInlineContext {
        let lexical_scope = context
            .current_function
            .as_ref()
            .map_or(InlineLexicalScope::File, |name| InlineLexicalScope::Subroutine(name.clone()));
        let visible_variables = context
            .variables
            .iter()
            .filter_map(|variable| VariableFact::from_perl_variable(variable))
            .collect();
        let imported_modules =
            context.imports.iter().map(|name| ModuleFact { name: name.clone() }).collect();

        SemanticInlineContext {
            lexical_scope,
            package: context.current_package.clone(),
            enclosing_sub: context.current_function.clone(),
            expected_syntax: self.expected_syntax(context),
            visible_variables,
            receiver_hint: receiver_hint_from_prefix(context.prefix.as_str()),
            dbi_receiver_kind: None,
            imported_modules,
            available_modules: Vec::new(),
            current_package_methods: Vec::new(),
            file_role: self.file_role(context),
            style: InlineStyleContext::unknown(context),
        }
    }

    #[cfg(test)]
    fn semantic_context_for_source(
        &self,
        text: &str,
        context: &PreparedInlineCompletionContext,
    ) -> SemanticInlineContext {
        self.semantic_context_for_source_with_environment(
            text,
            context,
            &InlineCompletionEnvironment::default(),
        )
    }

    fn semantic_context_for_source_with_environment(
        &self,
        text: &str,
        context: &PreparedInlineCompletionContext,
        environment: &InlineCompletionEnvironment,
    ) -> SemanticInlineContext {
        let mut semantic_context = self.semantic_context_for_prepared_context(context);
        semantic_context.available_modules = available_module_facts(&environment.available_modules);
        semantic_context.file_role = self.file_role_for_source(context, text);
        semantic_context.style = self.style_context_for_source(context, text);
        semantic_context.dbi_receiver_kind = dbi_receiver_kind_for_source(text, &semantic_context);
        semantic_context.current_package_methods = self.current_package_methods_for_source(
            text,
            semantic_context.package.as_deref(),
            semantic_context.enclosing_sub.as_deref(),
        );
        semantic_context
    }

    fn current_package_methods_for_source(
        &self,
        text: &str,
        current_package: Option<&str>,
        enclosing_sub: Option<&str>,
    ) -> Vec<MethodFact> {
        let Some(target_package) = current_package else {
            return Vec::new();
        };

        let mut package_scanner = PackageScopeScanner::new();
        let mut methods = Vec::<MethodFact>::new();
        for line in self.normalized_lines(text) {
            package_scanner.advance(line, self.parse_package_name(line));

            if package_scanner.current_package() != Some(target_package) {
                continue;
            }

            let Some(method_name) = self.parse_sub_name(line) else {
                continue;
            };
            if enclosing_sub == Some(method_name.as_str()) {
                continue;
            }
            if methods.iter().any(|method| method.name == method_name) {
                continue;
            }

            methods.push(MethodFact { name: method_name });
        }

        methods
    }

    fn expected_syntax(&self, context: &PreparedInlineCompletionContext) -> ExpectedSyntax {
        let prefix = context.prefix.as_str();
        if prefix.trim().is_empty() {
            return ExpectedSyntax::EmptyStatement;
        }
        if prefix.trim_end() == "use" || use_completion_fragment(prefix).is_some() {
            return ExpectedSyntax::UseModule;
        }
        if method_arrow_fragment(prefix).is_some() {
            return ExpectedSyntax::MethodName;
        }
        if ends_with_keyword(prefix, "my $") {
            return ExpectedSyntax::LexicalVariableName;
        }
        if ends_with_keyword(prefix, "package ") {
            return ExpectedSyntax::PackageName;
        }
        if ends_with_keyword(prefix, "bless ") {
            return ExpectedSyntax::BlessArguments;
        }
        if ends_with_keyword(prefix, "return ") {
            return ExpectedSyntax::ReturnExpression;
        }
        if ends_with_keyword(prefix, "for ") || ends_with_keyword(prefix, "foreach ") {
            return ExpectedSyntax::LoopBinding;
        }
        if ends_with_keyword(prefix, "ok(") || ends_with_keyword(prefix, "is(") {
            return ExpectedSyntax::TestAssertionArguments;
        }
        if prefix == "#!" || prefix == "#!/" {
            return ExpectedSyntax::ShebangInterpreter;
        }
        if self.match_sub_declaration(prefix).is_some() && !context.current_line.contains('{') {
            return ExpectedSyntax::SubroutineBody;
        }
        ExpectedSyntax::Unknown
    }

    fn file_role(&self, context: &PreparedInlineCompletionContext) -> FileRole {
        if context.imports.iter().any(|import| import == "Test::More" || import == "Test2::V0") {
            return FileRole::Test;
        }
        FileRole::Unknown
    }

    fn file_role_for_source(
        &self,
        context: &PreparedInlineCompletionContext,
        text: &str,
    ) -> FileRole {
        if self.file_role(context) == FileRole::Test {
            return FileRole::Test;
        }
        if text.lines().next().is_some_and(|line| line.starts_with("#!")) {
            return FileRole::Script;
        }
        if context.current_package.is_some() {
            return FileRole::Module;
        }
        FileRole::Unknown
    }

    fn style_context_for_source(
        &self,
        context: &PreparedInlineCompletionContext,
        text: &str,
    ) -> InlineStyleContext {
        InlineStyleContext {
            indentation: indentation_style_from_line(context.current_line.as_str()),
            language_prelude: LanguagePreludeStyle::from_imports(&context.imports),
            sub_argument_style: sub_argument_style(text),
            constructor_style: constructor_style(text),
            test_framework: TestFramework::from_imports(&context.imports),
        }
    }

    fn add_contextual_fallbacks(
        &self,
        context: &PreparedInlineCompletionContext,
        semantic_context: &SemanticInlineContext,
        sink: &mut InlineCandidateSink<'_>,
    ) {
        let prefix = context.prefix.trim();
        let comment_context = context
            .previous_non_empty_line
            .as_deref()
            .map(|line| line.trim_start().starts_with('#'))
            .unwrap_or(false);

        if context.current_line.is_empty()
            && matches!(semantic_context.lexical_scope, InlineLexicalScope::File)
            && context.imports.is_empty()
            && context.variables.is_empty()
            && context.previous_non_empty_line.is_none()
        {
            sink.push(
                InlineCandidateSourceKind::ContextualFallback,
                8,
                InlineCompletionItem {
                    insert_text: "#!/usr/bin/env perl\nuse strict;\nuse warnings;\n\n".into(),
                    filter_text: Some("perl".into()),
                    range: None,
                    command: None,
                },
            );
            sink.push(
                InlineCandidateSourceKind::ContextualFallback,
                9,
                InlineCompletionItem {
                    insert_text: "use strict;\nuse warnings;\n\n".into(),
                    filter_text: Some("strict".into()),
                    range: None,
                    command: None,
                },
            );
        }

        if prefix.is_empty() {
            let mut pushed_test_assertion = false;
            if semantic_context.file_role == FileRole::Test
                && let Some(assertion) = self.preferred_test_statement(semantic_context)
            {
                sink.push(
                    InlineCandidateSourceKind::ContextualFallback,
                    0,
                    InlineCompletionItem {
                        filter_text: Some(test_statement_filter_text(assertion.as_str()).into()),
                        insert_text: assertion,
                        range: None,
                        command: None,
                    },
                );
                pushed_test_assertion = true;
            }

            if let Some(variable) = self.preferred_return_variable(semantic_context) {
                sink.push(
                    InlineCandidateSourceKind::ContextualFallback,
                    0,
                    InlineCompletionItem {
                        insert_text: format!("return {variable};"),
                        filter_text: Some(variable),
                        range: None,
                        command: None,
                    },
                );
            }

            if semantic_context.file_role == FileRole::Test && !pushed_test_assertion {
                sink.push(
                    InlineCandidateSourceKind::ContextualFallback,
                    1,
                    InlineCompletionItem {
                        insert_text: "done_testing();".into(),
                        filter_text: Some("done_testing".into()),
                        range: None,
                        command: None,
                    },
                );
            }

            if comment_context
                && let Some(variable) = self.preferred_assignment_variable(semantic_context)
            {
                sink.push(
                    InlineCandidateSourceKind::ContextualFallback,
                    2,
                    InlineCompletionItem {
                        insert_text: format!("my {variable} = shift;"),
                        filter_text: Some(variable),
                        range: None,
                        command: None,
                    },
                );
            }
        }
    }

    fn normalize_items(&self, mut items: Vec<RankedCompletionItem>) -> Vec<InlineCompletionItem> {
        items.sort_by(|left, right| {
            right.score.0.cmp(&left.score.0).then_with(|| left.order.cmp(&right.order))
        });

        let mut deduped = Vec::new();
        let mut seen = Vec::<String>::new();
        for candidate in items.into_iter() {
            if seen.iter().any(|existing| existing == &candidate.item.insert_text) {
                continue;
            }

            seen.push(candidate.item.insert_text.clone());
            deduped.push(candidate.item);
            if deduped.len() >= MAX_INLINE_COMPLETION_ITEMS {
                break;
            }
        }

        deduped
    }

    fn preferred_return_variable(&self, context: &SemanticInlineContext) -> Option<String> {
        context
            .visible_variables
            .iter()
            .find(|variable| variable.is_scalar_self())
            .map(VariableFact::as_perl_variable)
            .or_else(|| context.visible_variables.first().map(VariableFact::as_perl_variable))
    }

    fn preferred_assignment_variable(&self, context: &SemanticInlineContext) -> Option<String> {
        context
            .visible_variables
            .iter()
            .find(|variable| variable.is_scalar() && !variable.is_scalar_self())
            .map(VariableFact::as_perl_variable)
    }

    fn current_package_method_items(
        &self,
        context: &SemanticInlineContext,
        fragment: &str,
    ) -> Vec<InlineCompletionItem> {
        context
            .current_package_methods
            .iter()
            .filter(|method| {
                completion_matches_fragment(
                    method.name.as_str(),
                    &format!("{}()", method.name),
                    fragment,
                )
            })
            .map(|method| InlineCompletionItem {
                insert_text: format!("{}()", method.name),
                filter_text: Some(method.name.clone()),
                range: None,
                command: None,
            })
            .collect()
    }

    fn preferred_test_statement(&self, context: &SemanticInlineContext) -> Option<String> {
        self.preferred_is_assertion_arguments(context)
            .map(|arguments| format!("is({arguments}"))
            .or_else(|| {
                self.preferred_ok_assertion_arguments(context)
                    .map(|arguments| format!("ok({arguments}"))
            })
    }

    fn preferred_ok_assertion_arguments(&self, context: &SemanticInlineContext) -> Option<String> {
        if !self.supports_test_assertions(context) {
            return None;
        }

        let actual = self.preferred_test_actual_variable(context)?;
        Some(format!("{}, 'test description');", actual.as_perl_variable()))
    }

    fn preferred_is_assertion_arguments(&self, context: &SemanticInlineContext) -> Option<String> {
        if !self.supports_test_assertions(context) {
            return None;
        }

        let actual = self.preferred_test_actual_variable(context)?;
        let expected = context.visible_variables.iter().find(|variable| {
            variable.is_scalar()
                && !variable.is_scalar_self()
                && is_preferred_test_expected_name(variable.name.as_str())
        })?;

        if actual == expected {
            return None;
        }

        Some(format!(
            "{}, {}, 'test description');",
            actual.as_perl_variable(),
            expected.as_perl_variable()
        ))
    }

    fn preferred_test_actual_variable<'a>(
        &self,
        context: &'a SemanticInlineContext,
    ) -> Option<&'a VariableFact> {
        context.visible_variables.iter().find(|variable| {
            variable.is_scalar()
                && !variable.is_scalar_self()
                && is_preferred_test_actual_name(variable.name.as_str())
        })
    }

    fn supports_test_assertions(&self, context: &SemanticInlineContext) -> bool {
        matches!(context.style.test_framework, TestFramework::Test2V0 | TestFramework::TestMore)
    }

    fn push_unique(&self, values: &mut Vec<String>, value: String) {
        if values.iter().any(|existing| existing == &value) {
            return;
        }
        values.push(value);
    }

    fn push_unique_variable(&self, values: &mut Vec<String>, value: String) {
        if values.len() >= 8 {
            return;
        }
        self.push_unique(values, value);
    }
}

impl InlineCandidateSource for ReceiverCandidateSource {
    const SOURCE: InlineCandidateSourceKind = InlineCandidateSourceKind::Receiver;

    fn add_candidates(
        &self,
        provider: &InlineCompletionProvider,
        context: &PreparedInlineCompletionContext,
        semantic_context: &SemanticInlineContext,
        sink: &mut InlineCandidateSink<'_>,
    ) {
        let prefix = context.prefix.as_str();
        if let Some(fragment) = method_arrow_fragment(prefix)
            && semantic_context.expected_syntax == ExpectedSyntax::MethodName
        {
            if semantic_context.receiver_hint == Some(ReceiverHint::SelfReceiver) {
                for method in provider.current_package_method_items(semantic_context, fragment) {
                    sink.push(Self::SOURCE, 0, method);
                }
            } else if let Some(kind) = semantic_context.dbi_receiver_kind {
                for method in dbi_receiver_method_items(kind, fragment) {
                    sink.push(Self::SOURCE, 0, method);
                }
            } else if completion_matches_fragment("new", "new()", fragment) {
                sink.push(
                    Self::SOURCE,
                    0,
                    InlineCompletionItem {
                        insert_text: "new()".into(),
                        filter_text: Some("new".into()),
                        range: None,
                        command: None,
                    },
                );
            }
        }
    }
}

impl InlineCandidateSource for ModuleCandidateSource {
    const SOURCE: InlineCandidateSourceKind = InlineCandidateSourceKind::Module;

    fn add_candidates(
        &self,
        _provider: &InlineCompletionProvider,
        context: &PreparedInlineCompletionContext,
        semantic_context: &SemanticInlineContext,
        sink: &mut InlineCandidateSink<'_>,
    ) {
        if semantic_context.expected_syntax != ExpectedSyntax::UseModule {
            return;
        }

        let Some(fragment) = use_completion_fragment(context.prefix.as_str()) else {
            return;
        };
        if !should_suggest_available_module(fragment) {
            return;
        }

        let mut added = 0usize;
        for module in &semantic_context.available_modules {
            if !completion_matches_fragment(module.name.as_str(), module.name.as_str(), fragment) {
                continue;
            }

            sink.push(
                Self::SOURCE,
                0,
                InlineCompletionItem {
                    insert_text: format!("{};", module.name),
                    filter_text: Some(module.name.clone()),
                    range: None,
                    command: None,
                },
            );
            added += 1;
            if added >= MAX_INLINE_COMPLETION_ITEMS {
                break;
            }
        }
    }
}

impl InlineCandidateSource for SyntaxCandidateSource {
    const SOURCE: InlineCandidateSourceKind = InlineCandidateSourceKind::Syntax;

    fn add_candidates(
        &self,
        provider: &InlineCompletionProvider,
        context: &PreparedInlineCompletionContext,
        semantic_context: &SemanticInlineContext,
        sink: &mut InlineCandidateSink<'_>,
    ) {
        let prefix = context.prefix.as_str();
        let full_line = context.current_line.as_str();

        if prefix.trim_end() == "use" || use_completion_fragment(prefix).is_some() {
            let typed_fragment = use_completion_fragment(prefix).unwrap_or("");
            if completion_matches_fragment("strict", "strict;", typed_fragment) {
                sink.push(
                    Self::SOURCE,
                    0,
                    InlineCompletionItem {
                        insert_text: "strict;".into(),
                        filter_text: Some("strict".into()),
                        range: None,
                        command: None,
                    },
                );
            }

            if completion_matches_fragment("warnings", "warnings;", typed_fragment) {
                sink.push(
                    Self::SOURCE,
                    1,
                    InlineCompletionItem {
                        insert_text: "warnings;".into(),
                        filter_text: Some("warnings".into()),
                        range: None,
                        command: None,
                    },
                );
            }

            if completion_matches_fragment("feature", "feature ':5.36';", typed_fragment) {
                sink.push(
                    Self::SOURCE,
                    2,
                    InlineCompletionItem {
                        insert_text: "feature ':5.36';".into(),
                        filter_text: Some("feature".into()),
                        range: None,
                        command: None,
                    },
                );
            }
        }

        if let Some(sub_name) = provider.match_sub_declaration(prefix)
            && !full_line.contains('{')
        {
            let insert_text = provider.generate_subroutine_completion(&sub_name, semantic_context);
            sink.push(
                Self::SOURCE,
                0,
                InlineCompletionItem {
                    insert_text,
                    filter_text: Some("{".into()),
                    range: None,
                    command: None,
                },
            );
        }

        if ends_with_keyword(prefix, "my $") {
            sink.push(
                Self::SOURCE,
                0,
                InlineCompletionItem {
                    insert_text: "self = shift;".into(),
                    filter_text: Some("self".into()),
                    range: None,
                    command: None,
                },
            );
        }

        if ends_with_keyword(prefix, "package ") {
            sink.push(
                Self::SOURCE,
                0,
                InlineCompletionItem {
                    insert_text: "MyPackage;\n\nuse strict;\nuse warnings;".into(),
                    filter_text: Some("MyPackage".into()),
                    range: None,
                    command: None,
                },
            );
        }

        if ends_with_keyword(prefix, "bless ") {
            sink.push(
                Self::SOURCE,
                0,
                InlineCompletionItem {
                    insert_text: "$self, $class;".into(),
                    filter_text: Some("$self".into()),
                    range: None,
                    command: None,
                },
            );
        }

        if ends_with_keyword(prefix, "return ") {
            if let Some(variable) = provider.preferred_return_variable(semantic_context) {
                sink.push(
                    Self::SOURCE,
                    0,
                    InlineCompletionItem {
                        insert_text: format!("{variable};"),
                        filter_text: Some(variable),
                        range: None,
                        command: None,
                    },
                );
            } else if provider
                .is_in_constructor_context(semantic_context.enclosing_sub.as_deref(), prefix)
            {
                sink.push(
                    Self::SOURCE,
                    1,
                    InlineCompletionItem {
                        insert_text: "$self;".into(),
                        filter_text: Some("$self".into()),
                        range: None,
                        command: None,
                    },
                );
            }
        }

        if ends_with_keyword(prefix, "for ") || ends_with_keyword(prefix, "foreach ") {
            sink.push(
                Self::SOURCE,
                0,
                InlineCompletionItem {
                    insert_text: "my $item (@items) {\n    \n}".into(),
                    filter_text: Some("my".into()),
                    range: None,
                    command: None,
                },
            );
        }
    }
}

impl InlineCandidateSource for TestCandidateSource {
    const SOURCE: InlineCandidateSourceKind = InlineCandidateSourceKind::Test;

    fn add_candidates(
        &self,
        provider: &InlineCompletionProvider,
        context: &PreparedInlineCompletionContext,
        semantic_context: &SemanticInlineContext,
        sink: &mut InlineCandidateSink<'_>,
    ) {
        let prefix = context.prefix.as_str();

        if ends_with_keyword(prefix, "ok(")
            && let Some(arguments) = provider.preferred_ok_assertion_arguments(semantic_context)
        {
            sink.push(
                Self::SOURCE,
                0,
                InlineCompletionItem {
                    filter_text: Some(arguments.clone()),
                    insert_text: arguments,
                    range: None,
                    command: None,
                },
            );
        }

        if ends_with_keyword(prefix, "is(")
            && let Some(arguments) = provider.preferred_is_assertion_arguments(semantic_context)
        {
            sink.push(
                Self::SOURCE,
                0,
                InlineCompletionItem {
                    filter_text: Some(arguments.clone()),
                    insert_text: arguments,
                    range: None,
                    command: None,
                },
            );
        }
    }
}

impl InlineCandidateSource for ShebangCandidateSource {
    const SOURCE: InlineCandidateSourceKind = InlineCandidateSourceKind::Shebang;

    fn add_candidates(
        &self,
        _provider: &InlineCompletionProvider,
        context: &PreparedInlineCompletionContext,
        _semantic_context: &SemanticInlineContext,
        sink: &mut InlineCandidateSink<'_>,
    ) {
        let prefix = context.prefix.as_str();
        if prefix == "#!" || prefix == "#!/" {
            sink.push(
                Self::SOURCE,
                0,
                InlineCompletionItem {
                    insert_text: "/usr/bin/env perl".into(),
                    filter_text: Some("perl".into()),
                    range: None,
                    command: None,
                },
            );
        }
    }
}

impl InlineCandidateSource for ContextualFallbackSource {
    const SOURCE: InlineCandidateSourceKind = InlineCandidateSourceKind::ContextualFallback;

    fn add_candidates(
        &self,
        provider: &InlineCompletionProvider,
        context: &PreparedInlineCompletionContext,
        semantic_context: &SemanticInlineContext,
        sink: &mut InlineCandidateSink<'_>,
    ) {
        provider.add_contextual_fallbacks(context, semantic_context, sink);
    }
}

fn is_preferred_test_actual_name(name: &str) -> bool {
    matches!(name, "actual" | "got" | "result" | "status" | "success" | "value")
}

fn is_preferred_test_expected_name(name: &str) -> bool {
    matches!(name, "expected" | "expected_result" | "want")
}

fn test_statement_filter_text(statement: &str) -> &'static str {
    if statement.starts_with("ok(") { "ok" } else { "is" }
}

struct LineContext<'a> {
    prefix: &'a str,
    current_line: &'a str,
}

struct ReplacementFragment<'a> {
    text: &'a str,
    start_byte: usize,
}

#[derive(Debug, Default)]
struct PackageScopeScanner {
    package: Option<String>,
    block_depth: Option<i32>,
}

impl PackageScopeScanner {
    fn new() -> Self {
        Self::default()
    }

    fn current_package(&self) -> Option<&str> {
        self.package.as_deref()
    }

    fn advance(&mut self, line: &str, parsed_package: Option<String>) {
        if let Some(package) = parsed_package {
            self.package = Some(package);
            self.block_depth = package_line_opens_block(line).then_some(0);
        }

        if let Some(depth) = self.block_depth.as_mut() {
            *depth += brace_delta(line);
            if *depth <= 0 {
                self.package = None;
                self.block_depth = None;
            }
        }
    }
}

fn is_keyword_boundary(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '!' | ';' | '{' | '}' | '(' | ')' | ',')
}

fn ends_with_keyword(prefix: &str, keyword: &str) -> bool {
    if !prefix.ends_with(keyword) {
        return false;
    }
    let before = &prefix[..prefix.len() - keyword.len()];
    before.chars().next_back().is_none_or(is_keyword_boundary)
}

fn last_keyword_index(prefix: &str, keyword: &str) -> Option<usize> {
    let mut search_from = 0;
    let mut last = None;
    while let Some(rel) = prefix[search_from..].find(keyword) {
        let idx = search_from + rel;
        let prev = prefix[..idx].chars().next_back();
        if prev.is_none_or(is_keyword_boundary) {
            last = Some(idx);
        }
        search_from = idx + 1;
    }
    last
}

fn contains_keyword(text: &str, keyword: &str) -> bool {
    last_keyword_index(text, keyword).is_some()
}

fn package_line_opens_block(line: &str) -> bool {
    line.trim_start().starts_with("package ") && code_before_line_comment(line).contains('{')
}

fn brace_delta(line: &str) -> i32 {
    code_before_line_comment(line).chars().fold(0, |depth, ch| match ch {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

fn indentation_style_from_line(line: &str) -> IndentationStyle {
    let mut spaces = 0usize;
    let mut tabs = 0usize;
    for ch in line.chars().take_while(|ch| ch.is_whitespace()) {
        match ch {
            ' ' => spaces += 1,
            '\t' => tabs += 1,
            _ => {}
        }
    }

    match (spaces, tabs) {
        (0, 0) => IndentationStyle::Unknown,
        (_, 0) => IndentationStyle::Spaces(spaces),
        (0, _) => IndentationStyle::Tabs,
        (_, _) => IndentationStyle::Mixed,
    }
}

fn sub_argument_style(text: &str) -> SubArgumentStyle {
    let code_lines: Vec<&str> = non_comment_code_lines(text).collect();
    if code_lines.iter().copied().any(line_declares_signature_sub) {
        return SubArgumentStyle::Signature;
    }
    if code_lines.iter().any(|line| line.contains("= @_;")) {
        return SubArgumentStyle::AtUnderscore;
    }
    if code_lines.iter().any(|line| line.contains("= shift;") || line.trim() == "shift;") {
        return SubArgumentStyle::Shift;
    }
    SubArgumentStyle::Unknown
}

fn line_declares_signature_sub(line: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("sub ") {
        return false;
    }
    let Some(paren_index) = trimmed.find('(') else {
        return false;
    };
    let brace_index = trimmed.find('{').unwrap_or(trimmed.len());
    paren_index < brace_index
}

fn constructor_style(text: &str) -> ConstructorStyle {
    let mut has_bless_hash = false;
    let mut has_return_self = false;
    for line in non_comment_code_lines(text) {
        has_bless_hash |= line.contains("bless {},");
        has_return_self |= line.contains("return $self;");
    }

    if has_bless_hash && has_return_self {
        return ConstructorStyle::BlessHashReturnSelf;
    }
    ConstructorStyle::Unknown
}

fn constructor_body(argument_style: SubArgumentStyle) -> String {
    match argument_style {
        SubArgumentStyle::Signature => {
            "    my $self = bless {}, $class;\n    return $self;".to_string()
        }
        SubArgumentStyle::AtUnderscore => {
            "    my ($class, %args) = @_;\n    my $self = bless {}, $class;\n    return $self;"
                .to_string()
        }
        SubArgumentStyle::Shift | SubArgumentStyle::Unknown => {
            "    my $class = shift;\n    my $self = bless {}, $class;\n    return $self;"
                .to_string()
        }
    }
}

fn non_comment_code_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter(|line| {
        let trimmed = line.trim_start();
        !trimmed.is_empty() && !trimmed.starts_with('#')
    })
}

fn collect_declared_variable_groups(text: &str) -> Vec<Vec<String>> {
    let mut groups = Vec::new();
    for raw_line in non_comment_code_lines(text) {
        let line = code_before_line_comment(raw_line);
        let mut search_start = 0usize;
        while search_start < line.len() {
            let Some((keyword_start, tail_start)) = find_variable_declaration(line, search_start)
            else {
                break;
            };
            let variables = variables_declared_after_keyword(&line[tail_start..]);
            if !variables.is_empty() {
                groups.push(variables);
            }
            search_start = keyword_start + 1;
        }
    }
    groups
}

fn find_variable_declaration(line: &str, search_start: usize) -> Option<(usize, usize)> {
    for (relative_index, _) in line[search_start..].char_indices() {
        let index = search_start + relative_index;
        for keyword in ["my", "our", "state"] {
            if !line[index..].starts_with(keyword) {
                continue;
            }
            if !declaration_keyword_has_boundary(line, index, keyword) {
                continue;
            }
            if is_inside_simple_quoted_text(line, index) {
                continue;
            }
            let after_keyword = index + keyword.len();
            let tail_start = declaration_tail_start(line, after_keyword)?;
            return Some((index, tail_start));
        }
    }
    None
}

fn declaration_keyword_has_boundary(line: &str, index: usize, keyword: &str) -> bool {
    let left_boundary = index == 0
        || line[..index]
            .chars()
            .next_back()
            .is_some_and(|ch| ch.is_whitespace() || is_keyword_boundary(ch));
    if !left_boundary {
        return false;
    }

    let after_keyword = index + keyword.len();
    line[after_keyword..]
        .chars()
        .next()
        .is_some_and(|ch| ch.is_whitespace() || ch == '(' || matches!(ch, '$' | '@' | '%'))
}

fn declaration_tail_start(line: &str, after_keyword: usize) -> Option<usize> {
    let mut tail_start = after_keyword;
    while tail_start < line.len() {
        let ch = line[tail_start..].chars().next()?;
        if !ch.is_whitespace() {
            break;
        }
        tail_start += ch.len_utf8();
    }
    Some(tail_start)
}

fn variables_declared_after_keyword(tail: &str) -> Vec<String> {
    let trimmed = tail.trim_start();
    if let Some(list_tail) = trimmed.strip_prefix('(') {
        let list_end = list_tail.find(')').unwrap_or(list_tail.len());
        return collect_variable_mentions(&list_tail[..list_end]);
    }

    collect_variable_mentions(trimmed).into_iter().take(1).collect()
}

fn code_before_line_comment(line: &str) -> &str {
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;

    for (index, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if single_quoted || double_quoted => escaped = true,
            '\'' if !double_quoted => single_quoted = !single_quoted,
            '"' if !single_quoted => double_quoted = !double_quoted,
            '#' if !single_quoted && !double_quoted => return &line[..index],
            _ => {}
        }
    }

    line
}

fn is_inside_simple_quoted_text(line: &str, byte_index: usize) -> bool {
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;

    for (_, ch) in line[..byte_index].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if single_quoted || double_quoted => escaped = true,
            '\'' if !double_quoted => single_quoted = !single_quoted,
            '"' if !single_quoted => double_quoted = !double_quoted,
            _ => {}
        }
    }

    single_quoted || double_quoted
}

fn collect_variable_mentions(text: &str) -> Vec<String> {
    let mut variables = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'$' || byte == b'@' || byte == b'%' {
            let start = index;
            index += 1;

            if index >= bytes.len() {
                break;
            }

            let first = bytes[index] as char;
            if !(first.is_ascii_alphabetic() || first == '_') {
                continue;
            }

            index += 1;
            while index < bytes.len() {
                let next = bytes[index] as char;
                if next.is_ascii_alphanumeric() || next == '_' {
                    index += 1;
                } else {
                    break;
                }
            }

            variables.push(text[start..index].to_string());
            continue;
        }

        index += 1;
    }

    variables
}

fn receiver_hint_from_prefix(prefix: &str) -> Option<ReceiverHint> {
    let arrow_index = prefix.rfind("->")?;
    let receiver_prefix = prefix[..arrow_index].trim_end();
    let receiver_start = receiver_prefix
        .char_indices()
        .rev()
        .find_map(|(idx, ch)| (!is_receiver_fragment_char(ch)).then_some(idx + ch.len_utf8()))
        .unwrap_or(0);
    let receiver = receiver_prefix[receiver_start..].trim();
    if receiver.is_empty() {
        return None;
    }

    if receiver == "$self" {
        return Some(ReceiverHint::SelfReceiver);
    }
    if let Some(variable) = VariableFact::from_perl_variable(receiver) {
        return Some(ReceiverHint::Variable(variable));
    }
    if receiver.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':')) {
        return Some(ReceiverHint::Package(receiver.to_string()));
    }

    None
}

fn dbi_receiver_kind_for_source(
    text: &str,
    context: &SemanticInlineContext,
) -> Option<DbiReceiverKind> {
    let Some(ReceiverHint::Variable(variable)) = context.receiver_hint.as_ref() else {
        return None;
    };
    if !variable.is_scalar() {
        return None;
    }

    let imported_dbi = context.imported_modules.iter().any(|module| module.name == "DBI");
    let assigned_from_dbi_connect = non_comment_code_lines(text)
        .map(code_before_line_comment)
        .any(|line| line_assigns_variable_from_method(line, variable.name.as_str(), "connect"));
    if is_likely_dbi_database_handle(
        variable.name.as_str(),
        imported_dbi,
        assigned_from_dbi_connect,
    ) {
        return Some(DbiReceiverKind::DatabaseHandle);
    }

    let prepared_statement = non_comment_code_lines(text)
        .map(code_before_line_comment)
        .any(|line| line_assigns_variable_from_method(line, variable.name.as_str(), "prepare"));
    if is_likely_dbi_statement_handle(variable.name.as_str(), imported_dbi, prepared_statement) {
        return Some(DbiReceiverKind::StatementHandle);
    }

    None
}

fn is_likely_dbi_database_handle(name: &str, imported_dbi: bool, has_dbi_connect: bool) -> bool {
    has_dbi_connect || (matches!(name, "dbh" | "db") && imported_dbi)
}

fn is_likely_dbi_statement_handle(
    name: &str,
    imported_dbi: bool,
    prepared_statement: bool,
) -> bool {
    prepared_statement && (name == "sth" || name.ends_with("_sth") || imported_dbi)
}

fn line_assigns_variable_from_method(line: &str, variable_name: &str, method_name: &str) -> bool {
    let Some((left, right)) = line.split_once('=') else {
        return false;
    };
    let variable = format!("${variable_name}");
    let method_call = format!("->{method_name}");
    left.split(|ch: char| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$')))
        .any(|part| part == variable)
        && right.contains(method_call.as_str())
}

fn dbi_receiver_method_items(kind: DbiReceiverKind, fragment: &str) -> Vec<InlineCompletionItem> {
    let method_names: &[&str] = match kind {
        DbiReceiverKind::DatabaseHandle => {
            &["prepare", "do", "selectrow_array", "selectall_arrayref", "disconnect"]
        }
        DbiReceiverKind::StatementHandle => {
            &["execute", "fetchrow_hashref", "fetchrow_array", "finish"]
        }
    };

    method_names
        .iter()
        .filter(|method| completion_matches_fragment(method, &format!("{method}()"), fragment))
        .map(|method| InlineCompletionItem {
            insert_text: format!("{method}()"),
            filter_text: Some((*method).to_string()),
            range: None,
            command: None,
        })
        .collect()
}

fn is_receiver_fragment_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':' | '$' | '@' | '%')
}

fn hard_reject_zone_at_cursor(
    text: &str,
    prefix: &str,
    cursor_offset: usize,
) -> Option<HardRejectZone> {
    if cursor_is_inside_pod(text, cursor_offset) {
        return Some(HardRejectZone::Pod);
    }

    let protected_ranges = protected_token_ranges(text);
    if let Some(zone) = protected_ranges
        .iter()
        .find_map(|range| range.contains_cursor(cursor_offset).then_some(range.zone))
    {
        return Some(zone);
    }

    if cursor_is_inside_line_comment(prefix, cursor_offset, &protected_ranges) {
        return Some(HardRejectZone::Comment);
    }

    if prefix_has_unclosed_match_regex(prefix) {
        return Some(HardRejectZone::RegexLike);
    }

    None
}

#[derive(Debug)]
struct ProtectedRange {
    start: usize,
    end: usize,
    zone: HardRejectZone,
    include_start: bool,
    include_end: bool,
}

impl ProtectedRange {
    fn contains_cursor(&self, cursor_offset: usize) -> bool {
        (self.start < cursor_offset || (self.include_start && self.start == cursor_offset))
            && (cursor_offset < self.end || (self.include_end && self.end == cursor_offset))
    }

    fn contains_byte(&self, byte_offset: usize) -> bool {
        self.start <= byte_offset && byte_offset < self.end
    }
}

fn protected_token_ranges(text: &str) -> Vec<ProtectedRange> {
    let mut lexer = PerlLexer::with_body_tokens(text);
    lexer
        .collect_tokens()
        .into_iter()
        .filter_map(|token| {
            token_hard_reject_zone(&token.token_type).map(|(zone, include_start, include_end)| {
                ProtectedRange {
                    start: token.start,
                    end: token.end,
                    zone,
                    include_start,
                    include_end,
                }
            })
        })
        .collect()
}

fn token_hard_reject_zone(token_type: &TokenType) -> Option<(HardRejectZone, bool, bool)> {
    match token_type {
        TokenType::StringLiteral
        | TokenType::InterpolatedString(_)
        | TokenType::QuoteSingle
        | TokenType::QuoteDouble
        | TokenType::QuoteWords
        | TokenType::QuoteCommand => Some((HardRejectZone::StringLike, false, false)),
        TokenType::RegexMatch
        | TokenType::QuoteRegex
        | TokenType::Substitution
        | TokenType::Transliteration => Some((HardRejectZone::RegexLike, false, false)),
        TokenType::HeredocBody(_) | TokenType::FormatBody(_) | TokenType::DataBody(_) => {
            Some((HardRejectZone::HeredocBody, true, false))
        }
        TokenType::Error(message)
            if message.contains("unterminated string") || message.contains("unclosed") =>
        {
            Some((HardRejectZone::StringLike, false, true))
        }
        _ => None,
    }
}

fn cursor_is_inside_line_comment(
    prefix: &str,
    cursor_offset: usize,
    protected_ranges: &[ProtectedRange],
) -> bool {
    let line_start = cursor_offset.saturating_sub(prefix.len());
    for (idx, ch) in prefix.char_indices() {
        if ch != '#' {
            continue;
        }

        let hash_offset = line_start + idx;
        if protected_ranges.iter().any(|range| range.contains_byte(hash_offset)) {
            continue;
        }
        if is_shebang_completion_prefix(prefix, hash_offset) {
            continue;
        }

        return cursor_offset > hash_offset;
    }

    false
}

fn is_shebang_completion_prefix(prefix: &str, hash_offset: usize) -> bool {
    hash_offset == 0 && matches!(prefix, "#!" | "#!/")
}

fn cursor_is_inside_pod(text: &str, cursor_offset: usize) -> bool {
    let mut pod_start = None;
    for (line_start, line_end, line_text) in line_spans(text) {
        if pod_start.is_none() && is_pod_start_line(line_text) {
            pod_start = Some(line_start);
        }

        if let Some(start) = pod_start {
            if start <= cursor_offset && cursor_offset < line_end {
                return true;
            }
            if is_pod_cut_line(line_text) {
                pod_start = None;
            }
        }
    }

    pod_start.is_some_and(|start| start <= cursor_offset)
}

fn line_spans(text: &str) -> impl Iterator<Item = (usize, usize, &str)> {
    let mut offset = 0usize;
    text.split_inclusive('\n').map(move |line| {
        let start = offset;
        offset += line.len();
        let content = line.strip_suffix('\n').unwrap_or(line);
        let content = content.strip_suffix('\r').unwrap_or(content);
        (start, offset, content)
    })
}

fn is_pod_start_line(line: &str) -> bool {
    if !line.starts_with('=') {
        return false;
    }

    matches!(
        line.split_whitespace().next(),
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
        )
    )
}

fn is_pod_cut_line(line: &str) -> bool {
    if !line.starts_with('=') {
        return false;
    }

    line.split_whitespace().next() == Some("=cut")
}

fn prefix_has_unclosed_match_regex(prefix: &str) -> bool {
    let Some(operator_index) = last_regex_match_operator(prefix) else {
        return false;
    };
    let after_operator = prefix[operator_index + 2..].trim_start();
    let Some(pattern) = after_operator.strip_prefix('/') else {
        return false;
    };

    !contains_unescaped_slash(pattern)
}

fn last_regex_match_operator(prefix: &str) -> Option<usize> {
    let match_index = prefix.rfind("=~");
    let negated_match_index = prefix.rfind("!~");
    match (match_index, negated_match_index) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(index), None) | (None, Some(index)) => Some(index),
        (None, None) => None,
    }
}

fn contains_unescaped_slash(text: &str) -> bool {
    let mut escaped = false;
    for ch in text.chars() {
        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if ch == '/' {
            return true;
        }
    }

    false
}

fn method_arrow_fragment(prefix: &str) -> Option<&str> {
    let arrow_index = prefix.rfind("->")?;
    let fragment = &prefix[arrow_index + 2..];
    fragment.chars().all(is_identifier_fragment_char).then_some(fragment)
}

fn use_completion_fragment(prefix: &str) -> Option<&str> {
    let use_index = last_keyword_index(prefix, "use ")?;
    let fragment = &prefix[use_index + 4..];
    fragment.chars().all(is_module_fragment_char).then_some(fragment)
}

fn should_suggest_available_module(fragment: &str) -> bool {
    !fragment.is_empty()
        && (fragment.contains("::")
            || fragment.chars().next().is_some_and(|ch| ch.is_ascii_uppercase()))
}

fn available_module_facts(modules: &[String]) -> Vec<ModuleFact> {
    let mut facts: Vec<ModuleFact> = modules
        .iter()
        .filter(|module| !module.is_empty())
        .map(|module| ModuleFact { name: module.clone() })
        .collect();
    facts.sort_by(|left, right| left.name.cmp(&right.name));
    facts.dedup_by(|left, right| left.name == right.name);
    facts
}

fn completion_matches_fragment(filter_text: &str, insert_text: &str, fragment: &str) -> bool {
    fragment.is_empty() || filter_text.starts_with(fragment) || insert_text.starts_with(fragment)
}

fn item_matches_fragment(item: &InlineCompletionItem, fragment: &str) -> bool {
    item.filter_text.as_deref().is_some_and(|filter_text| filter_text.starts_with(fragment))
        || item.insert_text.starts_with(fragment)
}

fn replacement_fragment_at_cursor(prefix: &str) -> Option<ReplacementFragment<'_>> {
    let mut start_byte = prefix.len();
    for (idx, ch) in prefix.char_indices().rev() {
        if is_replacement_fragment_char(ch) {
            start_byte = idx;
        } else {
            break;
        }
    }

    (start_byte < prefix.len())
        .then_some(ReplacementFragment { text: &prefix[start_byte..], start_byte })
}

fn replacement_range(
    prefix: &str,
    fragment: &ReplacementFragment<'_>,
    line: u32,
    character: u32,
) -> Option<lsp_types::Range> {
    if fragment.text.is_empty() {
        return None;
    }

    let start_character =
        u32::try_from(prefix[..fragment.start_byte].encode_utf16().count()).ok()?;
    if start_character > character {
        return None;
    }

    Some(lsp_types::Range {
        start: lsp_types::Position::new(line, start_character),
        end: lsp_types::Position::new(line, character),
    })
}

fn is_replacement_fragment_char(ch: char) -> bool {
    is_identifier_fragment_char(ch) || matches!(ch, '$' | '@' | '%' | ':')
}

fn is_identifier_fragment_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_module_fragment_char(ch: char) -> bool {
    is_identifier_fragment_char(ch) || ch == ':'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_after_arrow() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("$obj->", 0, 6);
        assert!(!completions.items.is_empty());
        assert_eq!(completions.items[0].insert_text, "new()");
    }

    #[test]
    fn test_after_use() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("use ", 0, 4);
        assert!(!completions.items.is_empty());
        assert!(completions.items.iter().any(|i| i.insert_text == "strict;"));
    }

    #[test]
    fn use_namespace_suggests_available_module_from_environment()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let environment = InlineCompletionEnvironment {
            available_modules: vec![
                "A::First".to_string(),
                "B::Second".to_string(),
                "C::Third".to_string(),
                "D::Fourth".to_string(),
                "E::Fifth".to_string(),
                "F::Sixth".to_string(),
                "Other::Tool".to_string(),
                "My::App".to_string(),
                "My::App::Config".to_string(),
            ],
        };
        let completions =
            provider.get_inline_completions_with_environment("use My::", 0, 8, &environment);

        let module = completions
            .items
            .iter()
            .find(|item| item.insert_text == "My::App;")
            .ok_or("expected My::App module inline completion")?;
        assert_eq!(module.filter_text.as_deref(), Some("My::App"));
        let range = module.range.as_ref().ok_or("module completion should replace typed prefix")?;
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 4);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 8);
        assert!(completions.items.iter().all(|item| item.insert_text != "Other::Tool;"));
        Ok(())
    }

    #[test]
    fn use_partial_token_replaces_typed_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("use str", 0, 7);
        let item = completions
            .items
            .iter()
            .find(|item| item.insert_text == "strict;")
            .ok_or("expected strict; completion for use str")?;
        let range = item.range.as_ref().ok_or("partial token completion must carry a range")?;

        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 4);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 7);
        assert!(completions.items.iter().all(|item| item.insert_text != "warnings;"));
        Ok(())
    }

    #[test]
    fn parse_safety_keeps_partial_token_replacement() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("use str", 0, 7);
        let item = completions
            .items
            .iter()
            .find(|item| item.insert_text == "strict;")
            .ok_or("expected parse-safe strict; completion")?;

        assert_eq!(item.filter_text.as_deref(), Some("strict"));
        assert!(item.range.is_some());
        Ok(())
    }

    #[test]
    fn parse_safety_rejects_candidate_that_adds_error() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let context = provider.prepare_context("", 0, 0).ok_or("expected prepared context")?;
        let list = InlineCompletionList {
            items: vec![
                InlineCompletionItem {
                    insert_text: "my $value = 1;".into(),
                    filter_text: Some("$value".into()),
                    range: None,
                    command: None,
                },
                InlineCompletionItem {
                    insert_text: "my $value = ;".into(),
                    filter_text: Some("$value".into()),
                    range: None,
                    command: None,
                },
            ],
        };

        let filtered = provider.filter_parse_safe_items(list, &context, 0, 0);

        assert!(filtered.items.iter().any(|item| item.insert_text == "my $value = 1;"));
        assert!(filtered.items.iter().all(|item| item.insert_text != "my $value = ;"));
        Ok(())
    }

    #[test]
    fn parse_safety_does_not_drop_incomplete_baseline_improvements()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "my $value = ";
        let context = provider.prepare_context(source, 0, 12).ok_or("expected prepared context")?;
        let list = InlineCompletionList {
            items: vec![InlineCompletionItem {
                insert_text: "1;".into(),
                filter_text: Some("1".into()),
                range: None,
                command: None,
            }],
        };

        let filtered = provider.filter_parse_safe_items(list, &context, 0, 12);

        assert!(filtered.items.iter().any(|item| item.insert_text == "1;"));
        Ok(())
    }

    #[test]
    fn method_arrow_partial_token_replaces_only_method_fragment()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("$obj->n", 0, 7);
        let item = completions
            .items
            .iter()
            .find(|item| item.insert_text == "new()")
            .ok_or("expected new() completion for $obj->n")?;
        let range = item.range.as_ref().ok_or("method fragment completion must carry a range")?;

        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 6);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 7);
        Ok(())
    }

    #[test]
    fn partial_token_range_uses_utf16_wire_positions() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "my $emoji = \"😀\"; use str";
        let character = u32::try_from(source.encode_utf16().count())?;
        let completions = provider.get_inline_completions(source, 0, character);
        let item = completions
            .items
            .iter()
            .find(|item| item.insert_text == "strict;")
            .ok_or("expected strict; completion after UTF-16 prefix")?;
        let range = item.range.as_ref().ok_or("UTF-16 partial token must carry a range")?;

        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, character - 3);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, character);
        Ok(())
    }

    #[test]
    fn test_after_sub() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("sub hello", 0, 9);
        assert!(!completions.items.is_empty());
        // Default method generates simple template with shift
        assert!(completions.items[0].insert_text.contains("my $self = shift"));
    }

    #[test]
    fn test_sub_new_constructor() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("sub new", 0, 7);
        assert!(!completions.items.is_empty());
        // Constructor generates bless pattern
        assert!(completions.items[0].insert_text.contains("bless"));
        assert!(completions.items[0].insert_text.contains("my $class = shift"));
    }

    #[test]
    fn test_sub_getter() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("sub get_name", 0, 12);
        assert!(!completions.items.is_empty());
        // Getter generates accessor pattern
        assert!(completions.items[0].insert_text.contains("return $self->{name}"));
    }

    #[test]
    fn test_sub_setter() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("sub set_name", 0, 12);
        assert!(!completions.items.is_empty());
        // Setter generates mutator pattern
        assert!(completions.items[0].insert_text.contains("$self->{name} = $value"));
    }

    #[test]
    fn test_sub_is_predicate() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("sub is_active", 0, 13);
        assert!(!completions.items.is_empty());
        // Boolean accessor returns 1/0
        assert!(completions.items[0].insert_text.contains("? 1 : 0"));
    }

    #[test]
    fn test_sub_has_predicate() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("sub has_items", 0, 13);
        assert!(!completions.items.is_empty());
        // Boolean accessor returns 1/0
        assert!(completions.items[0].insert_text.contains("? 1 : 0"));
    }

    #[test]
    fn test_no_completion_when_brace_exists() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("sub hello {", 0, 9);
        // Should not suggest brace when one exists
        assert!(completions.items.is_empty() || !completions.items[0].insert_text.contains('{'));
    }

    #[test]
    fn test_shebang_completion() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("#!/", 0, 3);
        assert!(!completions.items.is_empty());
        assert_eq!(completions.items[0].insert_text, "/usr/bin/env perl");
    }

    #[test]
    fn test_after_arrow_with_unicode_prefix_uses_utf16_position() {
        let provider = InlineCompletionProvider::new();
        let source = "my $emoji = \"😀\"; my $obj = Package->";
        let character = source.encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 0, character);

        assert!(!completions.items.is_empty());
        assert_eq!(completions.items[0].insert_text, "new()");
    }

    #[test]
    fn self_receiver_prefers_current_package_methods() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source =
            "package Demo;\nsub save {}\nsub display_name {}\nsub caller {\n    $self->\n}\n";
        let character = "    $self->".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 4, character);

        let first = completions.items.first().ok_or("expected self method completion")?;
        assert_eq!(first.insert_text, "save()");
        assert!(
            completions.items.iter().any(|item| item.insert_text == "display_name()"),
            "expected current package method suggestions, got {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
        assert!(
            completions.items.iter().all(|item| item.insert_text != "new()"),
            "$self-> should not fall back to generic constructor guesses when methods are known"
        );
        Ok(())
    }

    #[test]
    fn dbi_database_handle_receiver_suggests_common_methods() {
        let provider = InlineCompletionProvider::new();
        let source = "use DBI;\nmy $dbh = DBI->connect($dsn);\n$dbh->\n";
        let character = "$dbh->".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);
        let insert_texts: Vec<&str> =
            completions.items.iter().map(|item| item.insert_text.as_str()).collect();

        assert_eq!(insert_texts.first(), Some(&"prepare()"));
        assert!(insert_texts.contains(&"do()"));
        assert!(insert_texts.contains(&"disconnect()"));
        assert!(
            !insert_texts.contains(&"new()"),
            "DBI database handle must not fall back to generic constructor guesses"
        );
    }

    #[test]
    fn dbi_statement_handle_partial_receiver_suggests_fetch_methods_with_range()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source =
            "use DBI;\nmy $dbh = DBI->connect($dsn);\nmy $sth = $dbh->prepare($sql);\n$sth->f\n";
        let character = "$sth->f".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 3, character);
        let item = completions
            .items
            .iter()
            .find(|item| item.insert_text == "fetchrow_hashref()")
            .ok_or("expected DBI statement handle fetchrow_hashref() completion")?;
        let range =
            item.range.as_ref().ok_or("partial DBI method completion must carry a range")?;

        assert_eq!(range.start.line, 3);
        assert_eq!(range.start.character, "$sth->".encode_utf16().count() as u32);
        assert_eq!(range.end.line, 3);
        assert_eq!(range.end.character, character);
        assert!(
            completions.items.iter().all(|item| item.insert_text != "new()"),
            "DBI statement handle must not fall back to generic constructor guesses"
        );
        Ok(())
    }

    #[test]
    fn self_receiver_partial_method_replaces_typed_fragment()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source =
            "package Demo;\nsub save {}\nsub display_name {}\nsub caller {\n    $self->dis\n}\n";
        let character = "    $self->dis".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 4, character);
        let item = completions
            .items
            .iter()
            .find(|item| item.insert_text == "display_name()")
            .ok_or("expected display_name() completion")?;
        let range = item.range.as_ref().ok_or("partial method completion must carry a range")?;

        assert_eq!(range.start.line, 4);
        assert_eq!(range.start.character, "    $self->".encode_utf16().count() as u32);
        assert_eq!(range.end.line, 4);
        assert_eq!(range.end.character, character);
        Ok(())
    }

    #[test]
    fn self_receiver_does_not_suggest_other_package_methods() {
        let provider = InlineCompletionProvider::new();
        let source =
            "package Other;\nsub external {}\npackage Demo;\nsub caller {\n    $self->\n}\n";
        let character = "    $self->".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 4, character);

        assert!(
            completions.items.iter().all(|item| item.insert_text != "external()"),
            "other package methods should not be suggested for current-package self receiver"
        );
        assert!(
            completions.items.iter().all(|item| item.insert_text != "new()"),
            "$self-> with no known current-package methods should stay quiet"
        );
    }

    #[test]
    fn self_receiver_does_not_leak_methods_after_block_scoped_package() {
        let provider = InlineCompletionProvider::new();
        let source =
            "package Demo {\nsub save {}\nsub caller {\n    $self->\n}\n}\nsub external {}\n";
        let character = "    $self->".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 3, character);

        assert!(completions.items.iter().any(|item| item.insert_text == "save()"));
        assert!(
            completions.items.iter().all(|item| item.insert_text != "external()"),
            "methods after a block-scoped package must not leak into that package"
        );
    }

    #[test]
    fn test_prepare_context_collects_function_variables_and_imports()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\npackage Demo;\n\nsub helper {\n    my $result = 1;\n    my $status = $result;\n    \n}\n";
        let line = 6;
        let character = 4;
        let context =
            provider.prepare_context(source, line, character).ok_or("expected prepared context")?;

        assert_eq!(context.current_function.as_deref(), Some("helper"));
        assert_eq!(context.current_package.as_deref(), Some("Demo"));
        assert_eq!(context.previous_non_empty_line.as_deref(), Some("    my $status = $result;"));
        assert!(context.imports.iter().any(|import_name| import_name == "Test::More"));
        assert!(context.variables.iter().any(|variable| variable == "$status"));
        assert!(context.variables.iter().any(|variable| variable == "$result"));
        Ok(())
    }

    #[test]
    fn semantic_context_scaffold_derives_existing_perl_facts()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\npackage Demo;\n\nsub helper {\n    my @items = fetch_items();\n    my $status = $items[0];\n    \n}\n";
        let prepared =
            provider.prepare_context(source, 6, 4).ok_or("expected prepared inline context")?;
        let semantic = provider.semantic_context_for_prepared_context(&prepared);

        assert_eq!(semantic.lexical_scope, InlineLexicalScope::Subroutine("helper".into()));
        assert_eq!(semantic.package.as_deref(), Some("Demo"));
        assert_eq!(semantic.enclosing_sub.as_deref(), Some("helper"));
        assert_eq!(semantic.expected_syntax, ExpectedSyntax::EmptyStatement);
        assert_eq!(semantic.file_role, FileRole::Test);
        assert!(
            semantic.imported_modules.iter().any(|module| module.name == "Test::More"),
            "expected Test::More module fact, got {:?}",
            semantic.imported_modules
        );
        assert!(
            semantic
                .visible_variables
                .iter()
                .any(|variable| variable.as_perl_variable() == "$status"),
            "expected nearby scalar variable fact, got {:?}",
            semantic.visible_variables
        );
        assert!(
            semantic
                .visible_variables
                .iter()
                .any(|variable| variable.as_perl_variable() == "@items"),
            "expected array variable fact, got {:?}",
            semantic.visible_variables
        );
        Ok(())
    }

    #[test]
    fn semantic_context_scaffold_detects_method_receiver() -> Result<(), Box<dyn std::error::Error>>
    {
        let provider = InlineCompletionProvider::new();
        let source = "package Demo;\nsub helper {\n    $self->n\n}\n";
        let character = "    $self->n".encode_utf16().count() as u32;
        let prepared = provider
            .prepare_context(source, 2, character)
            .ok_or("expected prepared inline context")?;
        let semantic = provider.semantic_context_for_prepared_context(&prepared);

        assert_eq!(semantic.expected_syntax, ExpectedSyntax::MethodName);
        assert_eq!(semantic.receiver_hint, Some(ReceiverHint::SelfReceiver));
        assert_eq!(semantic.file_role, FileRole::Unknown);
        Ok(())
    }

    #[test]
    fn semantic_context_source_collects_current_package_methods()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "package Other;\nsub external {}\npackage Demo;\nsub save {}\nsub display_name {}\nsub caller {\n    $self->\n}\n";
        let character = "    $self->".encode_utf16().count() as u32;
        let prepared = provider
            .prepare_context(source, 6, character)
            .ok_or("expected prepared inline context")?;
        let semantic = provider.semantic_context_for_source(source, &prepared);

        let methods: Vec<&str> =
            semantic.current_package_methods.iter().map(|method| method.name.as_str()).collect();
        assert_eq!(methods, vec!["save", "display_name"]);
        Ok(())
    }

    #[test]
    fn semantic_context_source_resets_after_block_scoped_package()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "package Demo {\nsub save {}\n}\n\nmy $value = 1;\n";
        let prepared = provider.prepare_context(source, 4, 0).ok_or("expected context")?;
        let semantic = provider.semantic_context_for_source(source, &prepared);

        assert_eq!(semantic.package, None);
        assert!(semantic.current_package_methods.is_empty());
        Ok(())
    }

    #[test]
    fn semantic_context_scaffold_detects_use_context() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "package Demo;\nuse My::";
        let character = "use My::".encode_utf16().count() as u32;
        let prepared = provider
            .prepare_context(source, 1, character)
            .ok_or("expected prepared inline context")?;
        let semantic = provider.semantic_context_for_prepared_context(&prepared);

        assert_eq!(semantic.expected_syntax, ExpectedSyntax::UseModule);
        assert_eq!(semantic.file_role, FileRole::Unknown);
        assert_eq!(semantic.package.as_deref(), Some("Demo"));
        Ok(())
    }

    #[test]
    fn semantic_context_scaffold_detects_package_receiver() -> Result<(), Box<dyn std::error::Error>>
    {
        let provider = InlineCompletionProvider::new();
        let source = "Demo::Widget->";
        let character = source.encode_utf16().count() as u32;
        let prepared =
            provider.prepare_context(source, 0, character).ok_or("expected prepared context")?;
        let semantic = provider.semantic_context_for_prepared_context(&prepared);

        assert_eq!(semantic.expected_syntax, ExpectedSyntax::MethodName);
        assert_eq!(semantic.receiver_hint, Some(ReceiverHint::Package("Demo::Widget".into())));
        Ok(())
    }

    #[test]
    fn semantic_context_scaffold_classifies_existing_trigger_prefixes()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let cases = [
            ("use ", ExpectedSyntax::UseModule),
            ("$obj->", ExpectedSyntax::MethodName),
            ("sub helper", ExpectedSyntax::SubroutineBody),
            ("my $", ExpectedSyntax::LexicalVariableName),
            ("package ", ExpectedSyntax::PackageName),
            ("bless ", ExpectedSyntax::BlessArguments),
            ("return ", ExpectedSyntax::ReturnExpression),
            ("for ", ExpectedSyntax::LoopBinding),
            ("ok(", ExpectedSyntax::TestAssertionArguments),
            ("is(", ExpectedSyntax::TestAssertionArguments),
            ("#!", ExpectedSyntax::ShebangInterpreter),
        ];

        for (source, expected) in cases {
            let character = source.encode_utf16().count() as u32;
            let prepared = provider
                .prepare_context(source, 0, character)
                .ok_or("expected prepared context")?;
            let semantic = provider.semantic_context_for_prepared_context(&prepared);
            assert_eq!(semantic.expected_syntax, expected, "prefix {source:?}");
        }
        Ok(())
    }

    #[test]
    fn semantic_context_scaffold_keeps_neutral_context_unknown()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "my $value = 42;";
        let character = source.encode_utf16().count() as u32;
        let prepared =
            provider.prepare_context(source, 0, character).ok_or("expected prepared context")?;
        let semantic = provider.semantic_context_for_prepared_context(&prepared);

        assert_eq!(semantic.file_role, FileRole::Unknown);
        assert_eq!(semantic.receiver_hint, None);
        assert_eq!(semantic.expected_syntax, ExpectedSyntax::Unknown);
        assert_eq!(semantic.style.indentation, IndentationStyle::Unknown);
        assert_eq!(semantic.style.language_prelude, LanguagePreludeStyle::Unknown);
        assert_eq!(semantic.style.sub_argument_style, SubArgumentStyle::Unknown);
        assert_eq!(semantic.style.constructor_style, ConstructorStyle::Unknown);
        assert_eq!(semantic.style.test_framework, TestFramework::Unknown);
        Ok(())
    }

    #[test]
    fn semantic_context_source_detects_file_roles() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();

        let module_source = "package Demo;\n\n";
        let module_prepared =
            provider.prepare_context(module_source, 1, 0).ok_or("expected module context")?;
        let module_semantic = provider.semantic_context_for_source(module_source, &module_prepared);
        assert_eq!(module_semantic.file_role, FileRole::Module);

        let script_source = "#!/usr/bin/env perl\n\n";
        let script_prepared =
            provider.prepare_context(script_source, 1, 0).ok_or("expected script context")?;
        let script_semantic = provider.semantic_context_for_source(script_source, &script_prepared);
        assert_eq!(script_semantic.file_role, FileRole::Script);

        let test_source = "#!/usr/bin/env perl\nuse Test2::V0;\npackage Demo;\n\n";
        let test_prepared =
            provider.prepare_context(test_source, 3, 0).ok_or("expected test context")?;
        let test_semantic = provider.semantic_context_for_source(test_source, &test_prepared);
        assert_eq!(test_semantic.file_role, FileRole::Test);
        Ok(())
    }

    #[test]
    fn semantic_context_source_detects_test2_signature_constructor_style()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "use Test2::V0;\n\nsub new ($class, %args) {\n    my $self = bless {}, $class;\n    return $self;\n    \n}\n";
        let prepared = provider.prepare_context(source, 5, 4).ok_or("expected style context")?;
        let semantic = provider.semantic_context_for_source(source, &prepared);

        assert_eq!(semantic.file_role, FileRole::Test);
        assert_eq!(semantic.style.indentation, IndentationStyle::Spaces(4));
        assert_eq!(semantic.style.language_prelude, LanguagePreludeStyle::Unknown);
        assert_eq!(semantic.style.sub_argument_style, SubArgumentStyle::Signature);
        assert_eq!(semantic.style.constructor_style, ConstructorStyle::BlessHashReturnSelf);
        assert_eq!(semantic.style.test_framework, TestFramework::Test2V0);
        Ok(())
    }

    #[test]
    fn semantic_context_source_detects_language_prelude_and_indentation_style()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();

        let strict_source = "use strict;\nuse warnings;\nsub helper {\n    \n}\n";
        let strict_prepared =
            provider.prepare_context(strict_source, 3, 4).ok_or("expected strict style context")?;
        let strict_semantic = provider.semantic_context_for_source(strict_source, &strict_prepared);
        assert_eq!(strict_semantic.style.indentation, IndentationStyle::Spaces(4));
        assert_eq!(strict_semantic.style.language_prelude, LanguagePreludeStyle::StrictWarnings);

        let modern_source = "use Modern::Perl;\nsub helper {\n\t\n}\n";
        let modern_prepared =
            provider.prepare_context(modern_source, 2, 1).ok_or("expected modern style context")?;
        let modern_semantic = provider.semantic_context_for_source(modern_source, &modern_prepared);
        assert_eq!(modern_semantic.style.indentation, IndentationStyle::Tabs);
        assert_eq!(modern_semantic.style.language_prelude, LanguagePreludeStyle::ModernPerl);
        Ok(())
    }

    #[test]
    fn semantic_context_source_detects_shift_and_at_underscore_styles()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();

        let shift_source = "sub helper {\n    my $self = shift;\n    \n}\n";
        let shift_prepared =
            provider.prepare_context(shift_source, 2, 4).ok_or("expected shift style context")?;
        let shift_semantic = provider.semantic_context_for_source(shift_source, &shift_prepared);
        assert_eq!(shift_semantic.style.sub_argument_style, SubArgumentStyle::Shift);

        let at_underscore_source = "sub helper {\n    my ($self, %args) = @_;\n    \n}\n";
        let at_underscore_prepared = provider
            .prepare_context(at_underscore_source, 2, 4)
            .ok_or("expected @_ style context")?;
        let at_underscore_semantic =
            provider.semantic_context_for_source(at_underscore_source, &at_underscore_prepared);
        assert_eq!(at_underscore_semantic.style.sub_argument_style, SubArgumentStyle::AtUnderscore);
        Ok(())
    }

    #[test]
    fn semantic_context_source_ignores_commented_style_examples()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    # my $self = shift;\n    # my $self = bless {}, $class;\n    # return $self;\n    \n}\n";
        let prepared =
            provider.prepare_context(source, 4, 4).ok_or("expected commented style context")?;
        let semantic = provider.semantic_context_for_source(source, &prepared);

        assert_eq!(semantic.style.sub_argument_style, SubArgumentStyle::Unknown);
        assert_eq!(semantic.style.constructor_style, ConstructorStyle::Unknown);
        Ok(())
    }

    #[test]
    fn prepared_inline_context_serialization_shape_stays_stable()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source =
            "use Test::More;\npackage Demo;\n\nsub helper {\n    my $result = 1;\n    \n}\n";
        let prepared = provider.prepare_context(source, 5, 4).ok_or("expected prepared context")?;
        let value = serde_json::to_value(&prepared)?;

        assert!(value.get("fileRole").is_none(), "prepared context leaked fileRole: {value:?}");
        assert!(value.get("style").is_none(), "prepared context leaked style: {value:?}");
        assert!(value.get("localStyle").is_none(), "prepared context leaked localStyle: {value:?}");
        assert!(
            value.get("semanticContext").is_none(),
            "prepared context leaked semanticContext: {value:?}"
        );

        let legacy = r#"{
            "prefix": "    ",
            "currentLine": "    ",
            "previousNonEmptyLine": "    my $result = 1;",
            "currentFunction": "helper",
            "currentPackage": "Demo",
            "variables": ["$result"],
            "imports": ["Test::More"]
        }"#;
        let decoded: PreparedInlineCompletionContext = serde_json::from_str(legacy)?;
        assert_eq!(decoded.current_function.as_deref(), Some("helper"));
        assert_eq!(decoded.variables, vec!["$result"]);
        assert_eq!(decoded.imports, vec!["Test::More"]);
        Ok(())
    }

    #[test]
    fn prepared_context_prefers_declared_variables_over_rhs_mentions()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source =
            "sub helper {\n    my $input = seed();\n    my $result = compute($input);\n    \n}\n";
        let prepared = provider.prepare_context(source, 3, 4).ok_or("expected prepared context")?;

        let result_position = prepared
            .variables
            .iter()
            .position(|variable| variable == "$result")
            .ok_or("expected declared $result to be collected")?;
        let input_position = prepared
            .variables
            .iter()
            .position(|variable| variable == "$input")
            .ok_or("expected $input to be collected")?;
        assert!(
            result_position < input_position,
            "declared $result should outrank RHS mention $input: {:?}",
            prepared.variables
        );
        Ok(())
    }

    #[test]
    fn prepared_context_collects_parenthesized_variable_declarations()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my ($self, %args) = @_;\n    \n}\n";
        let prepared = provider.prepare_context(source, 2, 4).ok_or("expected prepared context")?;

        let self_position = prepared
            .variables
            .iter()
            .position(|variable| variable == "$self")
            .ok_or("expected declared $self to be collected")?;
        let args_position = prepared
            .variables
            .iter()
            .position(|variable| variable == "%args")
            .ok_or("expected declared %args to be collected")?;
        assert!(
            self_position < args_position,
            "nearest declaration order should stay stable: {:?}",
            prepared.variables
        );
        assert!(
            prepared.variables.iter().all(|variable| variable != "@_"),
            "assignment source @_ should not be treated as a visible lexical: {:?}",
            prepared.variables
        );
        Ok(())
    }

    #[test]
    fn prepared_context_collects_loop_variable_without_iterable_mentions()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    for my $user (@users) {\n        \n    }\n}\n";
        let prepared = provider.prepare_context(source, 2, 8).ok_or("expected prepared context")?;

        assert!(prepared.variables.iter().any(|variable| variable == "$user"));
        assert!(
            prepared.variables.iter().all(|variable| variable != "@users"),
            "iterable mention should not be treated as a loop lexical: {:?}",
            prepared.variables
        );
        Ok(())
    }

    #[test]
    fn prepared_context_ignores_undeclared_mentions_for_return_context()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    $result = compute();\n    \n}\n";
        let prepared = provider.prepare_context(source, 2, 4).ok_or("expected prepared context")?;

        assert!(
            prepared.variables.iter().all(|variable| variable != "$result"),
            "undeclared mention should not be treated as a visible lexical: {:?}",
            prepared.variables
        );

        let completions = provider.get_inline_completions(source, 2, 4);
        assert!(completions.items.iter().all(|item| item.insert_text != "return $result;"));
        Ok(())
    }

    #[test]
    fn prepared_context_ignores_decl_like_text_in_comments_and_strings()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    # my $comment_var = 1;\n    my $text = \"prefix my $string_var\";\n    \n}\n";
        let prepared = provider.prepare_context(source, 3, 4).ok_or("expected prepared context")?;

        assert!(prepared.variables.iter().any(|variable| variable == "$text"));
        assert!(
            prepared.variables.iter().all(|variable| variable != "$comment_var"),
            "comment declaration should not be collected: {:?}",
            prepared.variables
        );
        assert!(
            prepared.variables.iter().all(|variable| variable != "$string_var"),
            "quoted declaration text should not be collected: {:?}",
            prepared.variables
        );
        Ok(())
    }

    #[test]
    fn non_empty_file_without_declarations_does_not_get_empty_file_scaffold() {
        let provider = InlineCompletionProvider::new();
        let source = "$ghost = compute();\n";
        let completions = provider.get_inline_completions(source, 1, 0);

        assert!(completions.items.iter().all(|item| !item.insert_text.contains("use strict;")));
    }

    #[test]
    fn test_empty_file_gets_scaffold_suggestions() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("", 0, 0);

        assert!(!completions.items.is_empty());
        assert!(completions.items.iter().any(|item| item.insert_text.contains("use strict;")));
    }

    #[test]
    fn test_blank_line_in_function_prefers_nearby_variable() {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $result = compute();\n    \n}\n";
        let completions = provider.get_inline_completions(source, 2, 4);

        assert!(!completions.items.is_empty());
        assert!(completions.items.iter().any(|item| item.insert_text == "return $result;"));
    }

    #[test]
    fn test_file_blank_line_suggests_test_more_assertion_from_declared_variables()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\n\nmy $got = compute();\nmy $expected = 42;\n\n";
        let completions = provider.get_inline_completions(source, 4, 0);

        let first = completions.items.first().ok_or("expected inline completion")?;
        assert_eq!(first.insert_text, "is($got, $expected, 'test description');");
        assert!(
            completions.items.iter().all(|item| item.insert_text != "done_testing();"),
            "done_testing should not be suggested ahead of a concrete assertion: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn test_file_blank_line_suggests_test2_assertion_from_declared_variables()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "use Test2::V0;\n\nmy $result = compute();\n\n";
        let completions = provider.get_inline_completions(source, 3, 0);

        let first = completions.items.first().ok_or("expected inline completion")?;
        assert_eq!(first.insert_text, "ok($result, 'test description');");
        Ok(())
    }

    #[test]
    fn test_file_blank_line_without_test_import_does_not_suggest_assertion() {
        let provider = InlineCompletionProvider::new();
        let source = "my $got = compute();\nmy $expected = 42;\n\n";
        let completions = provider.get_inline_completions(source, 2, 0);

        assert!(
            completions
                .items
                .iter()
                .all(|item| !item.insert_text.starts_with("is(")
                    && !item.insert_text.starts_with("ok(")),
            "non-test files should not get test assertion statements: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
    }

    #[test]
    fn ok_paren_in_test_file_uses_declared_scalar_argument()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\nmy $status = compute();\n!ok(";
        let completions = provider.get_inline_completions(source, 2, 4);

        let item = completions
            .items
            .iter()
            .find(|item| item.insert_text.starts_with("$status,"))
            .ok_or("expected ok arguments from declared $status")?;
        assert_eq!(item.insert_text, "$status, 'test description');");
        Ok(())
    }

    #[test]
    fn is_paren_in_test_file_uses_declared_actual_expected_pair()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\nmy $actual = compute();\nmy $expected = 42;\nis(";
        let completions = provider.get_inline_completions(source, 3, 3);

        let item = completions
            .items
            .iter()
            .find(|item| item.insert_text.starts_with("$actual,"))
            .ok_or("expected is arguments from declared actual/expected variables")?;
        assert_eq!(item.insert_text, "$actual, $expected, 'test description');");
        Ok(())
    }

    #[test]
    fn test_assertion_requires_declared_actual_and_expected_variables() {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\n\n$got = compute();\nmy $expected = 42;\n\n";
        let completions = provider.get_inline_completions(source, 4, 0);

        assert!(
            completions
                .items
                .iter()
                .all(|item| !item.insert_text.starts_with("is(")
                    && !item.insert_text.starts_with("ok(")),
            "undeclared actual variable should not drive assertion suggestion: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
        assert!(completions.items.iter().any(|item| item.insert_text == "done_testing();"));
    }

    #[test]
    fn test_blank_line_after_comment_still_has_contextual_suggestions() {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\n\nsub helper {\n    my $result = 1;\n    # explain next step\n    \n}\n";
        let completions = provider.get_inline_completions(source, 5, 4);

        assert!(!completions.items.is_empty());
        assert!(completions.items.iter().any(|item| item.insert_text == "return $result;"));
        let has_ok_assertion = completions
            .items
            .iter()
            .any(|item| item.insert_text == "ok($result, 'test description');");
        assert!(has_ok_assertion);
        assert!(completions.items.iter().all(|item| item.insert_text != "done_testing();"));
    }

    #[test]
    fn inline_completion_is_suppressed_inside_line_comment() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("# use ", 0, 6);

        assert!(completions.items.is_empty());
    }

    #[test]
    fn inline_completion_is_suppressed_inside_trailing_comment() {
        let provider = InlineCompletionProvider::new();
        let source = "my $value = '#'; # use ";
        let character = source.encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 0, character);

        assert!(completions.items.is_empty());
    }

    #[test]
    fn inline_completion_is_suppressed_inside_string_literal() {
        let provider = InlineCompletionProvider::new();
        let source = "my $text = \"use \";";
        let character = "my $text = \"use ".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 0, character);

        assert!(completions.items.is_empty());
    }

    #[test]
    fn inline_completion_is_suppressed_inside_heredoc_body() {
        let provider = InlineCompletionProvider::new();
        let source = "print <<'EOF';\nuse \nEOF\n";
        let completions = provider.get_inline_completions(source, 1, 4);

        assert!(completions.items.is_empty());
    }

    #[test]
    fn inline_completion_is_suppressed_at_heredoc_body_start() {
        let provider = InlineCompletionProvider::new();
        let source = "print <<'EOF';\nuse \nEOF\n";
        let completions = provider.get_inline_completions(source, 1, 0);

        assert!(completions.items.is_empty());
    }

    #[test]
    fn inline_completion_is_suppressed_inside_pod() {
        let provider = InlineCompletionProvider::new();
        let source = "=pod\nuse \n=cut\nuse ";
        let completions = provider.get_inline_completions(source, 1, 4);

        assert!(completions.items.is_empty());
    }

    #[test]
    fn inline_completion_resumes_after_pod_cut() {
        let provider = InlineCompletionProvider::new();
        let source = "=pod\nwords\n=cut\nuse ";
        let completions = provider.get_inline_completions(source, 3, 4);

        assert!(completions.items.iter().any(|item| item.insert_text == "strict;"));
    }

    #[test]
    fn indented_equals_text_is_not_treated_as_pod() {
        let provider = InlineCompletionProvider::new();
        let source = " =pod\nuse ";
        let completions = provider.get_inline_completions(source, 1, 4);

        assert!(completions.items.iter().any(|item| item.insert_text == "strict;"));
    }

    #[test]
    fn inline_completion_is_suppressed_inside_regex_literal() {
        let provider = InlineCompletionProvider::new();
        let source = "if ($name =~ /use /) {}";
        let character = "if ($name =~ /use ".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 0, character);

        assert!(completions.items.is_empty());
    }

    #[test]
    fn inline_completion_is_suppressed_inside_unclosed_match_regex() {
        let provider = InlineCompletionProvider::new();
        let source = "if ($name =~ /use ";
        let character = source.encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 0, character);

        assert!(completions.items.is_empty());
    }

    #[test]
    fn inline_completion_is_suppressed_inside_unclosed_string_at_eof() {
        let provider = InlineCompletionProvider::new();
        let source = "my $text = \"use ";
        let character = source.encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 0, character);

        assert!(completions.items.is_empty());
    }

    #[test]
    fn use_trigger_requires_word_boundary() {
        let provider = InlineCompletionProvider::new();
        // "refuse " ends with "use " but "use" is not at a token boundary.
        let completions = provider.get_inline_completions("refuse ", 0, 7);
        assert!(
            completions.items.iter().all(|i| i.insert_text != "strict;"),
            "should not suggest `use strict;` inside an identifier; got {:?}",
            completions.items.iter().map(|i| &i.insert_text).collect::<Vec<_>>()
        );
    }

    #[test]
    fn use_trigger_fires_after_semicolon_no_space() {
        let provider = InlineCompletionProvider::new();
        // `;use ` is a legitimate boundary even without an intervening space.
        let completions = provider.get_inline_completions(";use ", 0, 5);
        assert!(completions.items.iter().any(|i| i.insert_text == "strict;"));
    }

    #[test]
    fn sub_trigger_requires_word_boundary() {
        let provider = InlineCompletionProvider::new();
        // "absub foo" contains "sub " but "sub" is not at a token boundary.
        let completions = provider.get_inline_completions("absub foo", 0, 9);
        assert!(
            completions.items.iter().all(|i| !i.insert_text.contains("my $self = shift")),
            "should not generate a body for a sub buried inside an identifier"
        );
    }

    #[test]
    fn my_dollar_trigger_requires_word_boundary() {
        let provider = InlineCompletionProvider::new();
        // "army $" ends with "my $" but "my" is not at a token boundary.
        let completions = provider.get_inline_completions("army $", 0, 6);
        assert!(completions.items.iter().all(|i| i.insert_text != "self = shift;"));
    }

    #[test]
    fn package_trigger_requires_word_boundary() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("unpackage ", 0, 10);
        assert!(completions.items.iter().all(|i| !i.insert_text.starts_with("MyPackage;")));
    }

    #[test]
    fn bless_trigger_requires_word_boundary() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("unbless ", 0, 8);
        assert!(completions.items.iter().all(|i| i.insert_text != "$self, $class;"));
    }

    #[test]
    fn return_trigger_requires_word_boundary() {
        let provider = InlineCompletionProvider::new();
        // No surrounding scope; the only path to `$self;` is the return rule.
        let completions = provider.get_inline_completions("unreturn ", 0, 9);
        assert!(completions.items.iter().all(|i| i.insert_text != "$self;"));
    }

    #[test]
    fn for_trigger_requires_word_boundary() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("sufor ", 0, 6);
        assert!(completions.items.iter().all(|i| !i.insert_text.contains("(@items)")));
    }

    #[test]
    fn ok_paren_trigger_requires_word_boundary() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("hook(", 0, 5);
        assert!(completions.items.iter().all(|i| !i.insert_text.starts_with("$result,")));
    }

    #[test]
    fn ok_paren_trigger_fires_after_negation_operator() {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\nmy $result = compute();\n!ok(";
        let completions = provider.get_inline_completions(source, 2, 4);
        assert!(completions.items.iter().any(|i| i.insert_text.starts_with("$result,")));
    }

    #[test]
    fn is_paren_trigger_requires_word_boundary() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("basis(", 0, 6);
        assert!(completions.items.iter().all(|i| !i.insert_text.starts_with("$got,")));
    }

    #[test]
    fn sub_declaration_in_for_loop_parens_still_triggers() {
        // Boundary chars like `(` should still allow keyword detection.
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("for (my $", 0, 9);
        assert!(
            completions.items.iter().any(|i| i.insert_text == "self = shift;"),
            "`my $` after `(` should still trigger the my-dollar rule"
        );
    }

    #[test]
    fn test_normalize_items_orders_deduplicates_and_limits() {
        let provider = InlineCompletionProvider::new();
        let items = vec![
            RankedCompletionItem {
                score: InlineCandidateScore::from_legacy_priority(2),
                order: 0,
                item: InlineCompletionItem {
                    insert_text: "late".into(),
                    filter_text: None,
                    range: None,
                    command: None,
                },
            },
            RankedCompletionItem {
                score: InlineCandidateScore::from_legacy_priority(0),
                order: 1,
                item: InlineCompletionItem {
                    insert_text: "first".into(),
                    filter_text: None,
                    range: None,
                    command: None,
                },
            },
            RankedCompletionItem {
                score: InlineCandidateScore::from_legacy_priority(0),
                order: 2,
                item: InlineCompletionItem {
                    insert_text: "first".into(),
                    filter_text: Some("duplicate".into()),
                    range: None,
                    command: None,
                },
            },
            RankedCompletionItem {
                score: InlineCandidateScore::from_legacy_priority(1),
                order: 3,
                item: InlineCompletionItem {
                    insert_text: "second".into(),
                    filter_text: None,
                    range: None,
                    command: None,
                },
            },
            RankedCompletionItem {
                score: InlineCandidateScore::from_legacy_priority(3),
                order: 4,
                item: InlineCompletionItem {
                    insert_text: "third".into(),
                    filter_text: None,
                    range: None,
                    command: None,
                },
            },
            RankedCompletionItem {
                score: InlineCandidateScore::from_legacy_priority(4),
                order: 5,
                item: InlineCompletionItem {
                    insert_text: "fourth".into(),
                    filter_text: None,
                    range: None,
                    command: None,
                },
            },
            RankedCompletionItem {
                score: InlineCandidateScore::from_legacy_priority(5),
                order: 6,
                item: InlineCompletionItem {
                    insert_text: "fifth".into(),
                    filter_text: None,
                    range: None,
                    command: None,
                },
            },
        ];

        let normalized = provider.normalize_items(items);

        assert_eq!(normalized.len(), MAX_INLINE_COMPLETION_ITEMS);
        assert_eq!(normalized[0].insert_text, "first");
        assert_eq!(normalized[1].insert_text, "second");
        assert_eq!(normalized[2].insert_text, "late");
        assert_eq!(normalized[3].insert_text, "third");
        assert_eq!(normalized[4].insert_text, "fourth");
    }

    #[test]
    fn test_normalize_items_prefers_semantic_score_before_sequence() {
        let provider = InlineCompletionProvider::new();
        let items = vec![
            RankedCompletionItem {
                score: InlineCandidateScore::from_legacy_priority(0),
                order: 0,
                item: InlineCompletionItem {
                    insert_text: "return $result;".into(),
                    filter_text: Some("$result".into()),
                    range: None,
                    command: None,
                },
            },
            RankedCompletionItem {
                score: InlineCandidateScore(InlineCandidateScore::legacy_base(0) + 25),
                order: 1,
                item: InlineCompletionItem {
                    insert_text: "is($got, $expected, 'test description');".into(),
                    filter_text: Some("is".into()),
                    range: None,
                    command: None,
                },
            },
        ];

        let normalized = provider.normalize_items(items);

        assert_eq!(normalized[0].insert_text, "is($got, $expected, 'test description');");
        assert_eq!(normalized[1].insert_text, "return $result;");
    }
}
