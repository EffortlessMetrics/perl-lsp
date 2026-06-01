//! Inline completions provider with deterministic rules and AI backend support.
//!
//! This crate provides context-aware inline completions that appear as
//! ghost text. Deterministic completions are based on patterns; AI-powered
//! suggestions use the `InlineCompletionBackend` trait for pluggable providers.

use perl_lexer::{PerlLexer, TokenType};
use perl_parser_core::{Parser, RecoverySalvageProfile};
use perl_position_tracking::utf16_line_col_to_offset;
use serde::{Deserialize, Serialize};

pub mod next_edit;
pub use next_edit::{
    MissingImportNextEditCandidate, MissingImportNextEditProof, MissingImportNextEditRequest,
    NextEditCandidateFamily, NextEditFeatureGate, NextEditGateSource, NextEditProvider,
    NextEditRejectionReason, NextEditRequest, NextEditResponse, NextEditSafetyPolicy,
    NextEditStatus, NextEditSuggestion, NextEditTextEdit, TestAssertionNextEditCandidate,
    TestAssertionNextEditFramework, TestAssertionNextEditProof, TestAssertionNextEditRequest,
};

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
    pub(crate) has_done_testing_call: bool,
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
    GuardCondition,
    ConditionExpression,
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

    fn is_array(&self) -> bool {
        self.sigil == VariableSigil::Array
    }

    fn is_hash(&self) -> bool {
        self.sigil == VariableSigil::Hash
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
    metadata: InlineCandidateMetadata,
    item: InlineCompletionItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InlineCandidateMetadata {
    source: InlineCandidateSourceKind,
    reason: InlineCandidateReason,
    confidence: InlineCandidateConfidence,
}

impl InlineCandidateMetadata {
    fn for_candidate(
        source: InlineCandidateSourceKind,
        item: &InlineCompletionItem,
        semantic_context: &SemanticInlineContext,
    ) -> Self {
        let reason = InlineCandidateReason::for_candidate(source, item, semantic_context);
        let confidence = InlineCandidateConfidence::for_reason(reason);
        Self { source, reason, confidence }
    }

    fn stable_tiebreak(self) -> u8 {
        self.source.stable_rank() * 32
            + self.reason.stable_rank() * 4
            + self.confidence.stable_rank()
    }

    #[cfg(test)]
    fn test_fixture() -> Self {
        Self {
            source: InlineCandidateSourceKind::Syntax,
            reason: InlineCandidateReason::SourceSyntax,
            confidence: InlineCandidateConfidence::Medium,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InlineCandidateReason {
    CurrentPackageMethod,
    DbiReceiverMethod,
    EffectiveIncModule,
    VisibleLexical,
    SourceReceiver,
    SourceModule,
    SourceSyntax,
    SourceTest,
    SourceShebang,
    SourceContextualFallback,
}

impl InlineCandidateReason {
    fn for_candidate(
        source: InlineCandidateSourceKind,
        item: &InlineCompletionItem,
        semantic_context: &SemanticInlineContext,
    ) -> Self {
        match source {
            InlineCandidateSourceKind::Receiver => {
                receiver_candidate_reason(item, semantic_context)
            }
            InlineCandidateSourceKind::Module => module_candidate_reason(item, semantic_context),
            InlineCandidateSourceKind::Syntax => syntax_candidate_reason(semantic_context),
            InlineCandidateSourceKind::Test => Self::SourceTest,
            InlineCandidateSourceKind::Shebang => Self::SourceShebang,
            InlineCandidateSourceKind::ContextualFallback => Self::SourceContextualFallback,
        }
    }

    fn stable_rank(self) -> u8 {
        match self {
            Self::CurrentPackageMethod => 0,
            Self::DbiReceiverMethod => 1,
            Self::EffectiveIncModule => 2,
            Self::VisibleLexical => 3,
            Self::SourceReceiver => 4,
            Self::SourceModule => 5,
            Self::SourceSyntax => 6,
            Self::SourceTest => 7,
            Self::SourceShebang => 8,
            Self::SourceContextualFallback => 9,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InlineCandidateConfidence {
    High,
    Medium,
    Low,
}

impl InlineCandidateConfidence {
    fn for_reason(reason: InlineCandidateReason) -> Self {
        match reason {
            InlineCandidateReason::CurrentPackageMethod
            | InlineCandidateReason::DbiReceiverMethod
            | InlineCandidateReason::EffectiveIncModule
            | InlineCandidateReason::VisibleLexical
            | InlineCandidateReason::SourceTest => Self::High,
            InlineCandidateReason::SourceSyntax | InlineCandidateReason::SourceShebang => {
                Self::Medium
            }
            InlineCandidateReason::SourceReceiver
            | InlineCandidateReason::SourceModule
            | InlineCandidateReason::SourceContextualFallback => Self::Low,
        }
    }

    fn stable_rank(self) -> u8 {
        match self {
            Self::High => 0,
            Self::Medium => 1,
            Self::Low => 2,
        }
    }
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

impl InlineCandidateSourceKind {
    fn stable_rank(self) -> u8 {
        match self {
            Self::Receiver => 0,
            Self::Module => 1,
            Self::Syntax => 2,
            Self::Test => 3,
            Self::Shebang => 4,
            Self::ContextualFallback => 5,
        }
    }
}

fn receiver_candidate_reason(
    item: &InlineCompletionItem,
    context: &SemanticInlineContext,
) -> InlineCandidateReason {
    let method_name = item.insert_text.trim_end_matches("()");
    if receiver_targets_current_package(context)
        && context.current_package_methods.iter().any(|method| method.name == method_name)
    {
        return InlineCandidateReason::CurrentPackageMethod;
    }

    if context.dbi_receiver_kind.is_some() {
        return InlineCandidateReason::DbiReceiverMethod;
    }

    InlineCandidateReason::SourceReceiver
}

fn receiver_targets_current_package(context: &SemanticInlineContext) -> bool {
    match context.receiver_hint.as_ref() {
        Some(ReceiverHint::SelfReceiver) => true,
        Some(ReceiverHint::Package(package)) => context
            .package
            .as_deref()
            .is_some_and(|current_package| package == "__PACKAGE__" || package == current_package),
        _ => false,
    }
}

fn module_candidate_reason(
    item: &InlineCompletionItem,
    context: &SemanticInlineContext,
) -> InlineCandidateReason {
    let module_name = item.insert_text.trim_end_matches(';');
    if context.available_modules.iter().any(|module| module.name == module_name) {
        return InlineCandidateReason::EffectiveIncModule;
    }

    InlineCandidateReason::SourceModule
}

fn syntax_candidate_reason(context: &SemanticInlineContext) -> InlineCandidateReason {
    match context.expected_syntax {
        ExpectedSyntax::ReturnExpression
        | ExpectedSyntax::GuardCondition
        | ExpectedSyntax::ConditionExpression
        | ExpectedSyntax::LoopBinding => InlineCandidateReason::VisibleLexical,
        _ => InlineCandidateReason::SourceSyntax,
    }
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
        ExpectedSyntax::ReturnExpression | ExpectedSyntax::GuardCondition
            if item.insert_text.ends_with(';') =>
        {
            20
        }
        ExpectedSyntax::ConditionExpression if item.insert_text.ends_with(") {\n    \n}") => 20,
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
        let metadata = InlineCandidateMetadata::for_candidate(source, &item, self.semantic_context);
        self.items.push(RankedCompletionItem { score, order: self.sequence, metadata, item });
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
                self.semantic_context_for_request(text, line, &context, environment);
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
        let mut scope = None::<FunctionScope>;
        for (idx, line) in lines.iter().take(line_index + 1).enumerate() {
            if let Some(name) = self.parse_sub_name(line) {
                scope = Some(FunctionScope {
                    name,
                    start_line: idx,
                    block_depth: 0,
                    opened_block: false,
                });
            }

            if let Some(active) = scope.as_mut() {
                let delta = brace_delta(line);
                active.opened_block |= line_opens_block(line);
                active.block_depth += delta;
                if idx < line_index && active.opened_block && active.block_depth <= 0 {
                    scope = None;
                }
            }
        }

        scope.map(|active| (Some(active.name), Some(active.start_line))).unwrap_or((None, None))
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
        for declaration_group in
            collect_live_declared_variable_groups(visible_text).into_iter().rev()
        {
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
            has_done_testing_call: false,
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

    #[cfg(test)]
    fn semantic_context_for_source_with_environment(
        &self,
        text: &str,
        context: &PreparedInlineCompletionContext,
        environment: &InlineCompletionEnvironment,
    ) -> SemanticInlineContext {
        self.semantic_context_for_source_with_environment_and_dbi_text(
            text,
            text,
            context,
            environment,
        )
    }

    fn semantic_context_for_request(
        &self,
        text: &str,
        line: u32,
        context: &PreparedInlineCompletionContext,
        environment: &InlineCompletionEnvironment,
    ) -> SemanticInlineContext {
        let lines = self.normalized_lines(text);
        let line_index = usize::try_from(line).unwrap_or(usize::MAX);
        let visible_text = if line_index < lines.len() {
            self.visible_text_until_cursor(&lines, line_index, context.prefix.as_str())
        } else {
            text.to_string()
        };

        self.semantic_context_for_source_with_environment_and_dbi_text(
            text,
            visible_text.as_str(),
            context,
            environment,
        )
    }

    fn semantic_context_for_source_with_environment_and_dbi_text(
        &self,
        text: &str,
        dbi_visible_text: &str,
        context: &PreparedInlineCompletionContext,
        environment: &InlineCompletionEnvironment,
    ) -> SemanticInlineContext {
        let mut semantic_context = self.semantic_context_for_prepared_context(context);
        semantic_context.available_modules = available_module_facts(&environment.available_modules);
        semantic_context.file_role = self.file_role_for_source(context, text);
        semantic_context.style = self.style_context_for_source(context, text);
        semantic_context.has_done_testing_call = source_has_done_testing_call(text);
        semantic_context.dbi_receiver_kind =
            dbi_receiver_kind_for_source(dbi_visible_text, &semantic_context);
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
        if prefix.trim_end() == "use"
            || prefix.trim_end() == "require"
            || module_statement_fragment(prefix).is_some()
        {
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
        if return_expression_fragment(prefix).is_some() {
            return ExpectedSyntax::ReturnExpression;
        }
        if is_guard_condition_prefix(prefix) {
            return ExpectedSyntax::GuardCondition;
        }
        if condition_expression_prefix(prefix).is_some() {
            return ExpectedSyntax::ConditionExpression;
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
            let mut suppress_return_candidate = false;
            if semantic_context.file_role == FileRole::Test
                && let Some(assertion) = self.preferred_test_statement(semantic_context)
            {
                suppress_return_candidate = assertion.starts_with("is(");
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

            if !suppress_return_candidate
                && let Some(variable) = self.preferred_return_variable(semantic_context)
            {
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

            if semantic_context.file_role == FileRole::Test
                && !pushed_test_assertion
                && !semantic_context.has_done_testing_call
            {
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
            right.score.0.cmp(&left.score.0).then_with(|| left.order.cmp(&right.order)).then_with(
                || left.metadata.stable_tiebreak().cmp(&right.metadata.stable_tiebreak()),
            )
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
        let self_variable = context
            .visible_variables
            .iter()
            .find(|variable| variable.is_scalar_self())
            .map(VariableFact::as_perl_variable);

        if is_constructor_sub(context.enclosing_sub.as_deref()) {
            return self_variable
                .or_else(|| context.visible_variables.first().map(VariableFact::as_perl_variable));
        }

        context
            .visible_variables
            .iter()
            .find(|variable| variable.is_scalar() && !variable.is_scalar_self())
            .map(VariableFact::as_perl_variable)
            .or(self_variable)
            .or_else(|| context.visible_variables.first().map(VariableFact::as_perl_variable))
    }

    fn return_variable_items(
        &self,
        context: &SemanticInlineContext,
        fragment: &str,
    ) -> Vec<InlineCompletionItem> {
        context
            .visible_variables
            .iter()
            .map(VariableFact::as_perl_variable)
            .filter(|variable| {
                completion_matches_fragment(variable.as_str(), &format!("{variable};"), fragment)
            })
            .map(|variable| InlineCompletionItem {
                insert_text: format!("{variable};"),
                filter_text: Some(variable),
                range: None,
                command: None,
            })
            .collect()
    }

    fn preferred_guard_condition(&self, context: &SemanticInlineContext) -> Option<String> {
        context
            .visible_variables
            .iter()
            .find(|variable| {
                variable.is_scalar()
                    && !variable.is_scalar_self()
                    && is_preferred_guard_condition_name(variable.name.as_str())
            })
            .or_else(|| {
                context
                    .visible_variables
                    .iter()
                    .find(|variable| variable.is_scalar() && !variable.is_scalar_self())
            })
            .map(VariableFact::as_perl_variable)
    }

    fn preferred_assignment_variable(&self, context: &SemanticInlineContext) -> Option<String> {
        context
            .visible_variables
            .iter()
            .find(|variable| variable.is_scalar() && !variable.is_scalar_self())
            .map(VariableFact::as_perl_variable)
    }

    fn preferred_loop_binding(&self, context: &SemanticInlineContext) -> Option<String> {
        if let Some(array) = context.visible_variables.iter().find(|variable| variable.is_array()) {
            let item_name = singular_loop_variable_name(array.name.as_str());
            return Some(format!("my ${item_name} ({}) {{\n    \n}}", array.as_perl_variable()));
        }

        let hash = context.visible_variables.iter().find(|variable| variable.is_hash())?;
        let key_name = hash_key_loop_variable_name(hash.name.as_str());
        Some(format!("my ${key_name} (keys {}) {{\n    \n}}", hash.as_perl_variable()))
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

    fn preferred_subtest_block(&self, context: &SemanticInlineContext) -> Option<String> {
        self.supports_test_assertions(context)
            .then(|| "'test description' => sub {\n    \n};".to_string())
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
            if receiver_targets_current_package(semantic_context) {
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

        let Some(fragment) = module_statement_fragment(context.prefix.as_str()) else {
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

        if let Some(fragment) = return_expression_fragment(prefix) {
            let constructor_self_matches = provider
                .is_in_constructor_context(semantic_context.enclosing_sub.as_deref(), prefix)
                && completion_matches_fragment("$self", "$self;", fragment);

            if constructor_self_matches {
                sink.push(
                    Self::SOURCE,
                    0,
                    InlineCompletionItem {
                        insert_text: "$self;".into(),
                        filter_text: Some("$self".into()),
                        range: None,
                        command: None,
                    },
                );
            }

            for variable in provider.return_variable_items(semantic_context, fragment) {
                if constructor_self_matches && variable.insert_text == "$self;" {
                    continue;
                }
                sink.push(Self::SOURCE, 0, variable);
            }
        }

        if is_guard_condition_prefix(prefix)
            && let Some(condition) = provider.preferred_guard_condition(semantic_context)
        {
            sink.push(
                Self::SOURCE,
                0,
                InlineCompletionItem {
                    insert_text: format!("{condition};"),
                    filter_text: Some(condition),
                    range: None,
                    command: None,
                },
            );
        } else if condition_expression_prefix(prefix).is_some()
            && let Some(condition) = provider.preferred_guard_condition(semantic_context)
        {
            sink.push(
                Self::SOURCE,
                0,
                InlineCompletionItem {
                    insert_text: condition_expression_insert_text(prefix, condition.as_str()),
                    filter_text: Some(condition),
                    range: None,
                    command: None,
                },
            );
        }

        if let Some((assigned_sigil, assigned_name)) = lexical_assignment_rhs_prefix(prefix)
            && let Some(variable) = semantic_context
                .visible_variables
                .iter()
                .find(|variable| {
                    variable.sigil == assigned_sigil
                        && !variable.is_scalar_self()
                        && variable.name != assigned_name
                })
                .map(VariableFact::as_perl_variable)
        {
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
        }

        if ends_with_keyword(prefix, "for ") || ends_with_keyword(prefix, "foreach ") {
            if let Some(binding) = provider.preferred_loop_binding(semantic_context) {
                sink.push(
                    Self::SOURCE,
                    0,
                    InlineCompletionItem {
                        insert_text: binding,
                        filter_text: Some("my".into()),
                        range: None,
                        command: None,
                    },
                );
            }
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

        if ends_with_keyword(prefix, "subtest ")
            && let Some(block) = provider.preferred_subtest_block(semantic_context)
        {
            sink.push(
                Self::SOURCE,
                0,
                InlineCompletionItem {
                    filter_text: Some("subtest".into()),
                    insert_text: block,
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

fn is_constructor_sub(name: Option<&str>) -> bool {
    matches!(name, Some("new" | "BUILD"))
}

fn is_preferred_guard_condition_name(name: &str) -> bool {
    name == "ok"
        || name == "valid"
        || name == "ready"
        || name.starts_with("is_")
        || name.starts_with("has_")
        || name.starts_with("can_")
        || name.starts_with("should_")
        || name.ends_with("_ok")
}

fn return_expression_fragment(prefix: &str) -> Option<&str> {
    let return_index = last_keyword_index(prefix, "return ")?;
    let fragment = &prefix[return_index + 7..];
    fragment.chars().all(is_return_expression_fragment_char).then_some(fragment)
}

fn is_return_expression_fragment_char(ch: char) -> bool {
    is_identifier_fragment_char(ch) || matches!(ch, '$' | '@' | '%')
}

fn is_guard_condition_prefix(prefix: &str) -> bool {
    ends_with_keyword(prefix, "return unless ")
        || ends_with_keyword(prefix, "return if ")
        || ends_with_keyword(prefix, "next if ")
        || ends_with_keyword(prefix, "next unless ")
        || ends_with_keyword(prefix, "last if ")
        || ends_with_keyword(prefix, "last unless ")
}

fn condition_expression_prefix(prefix: &str) -> Option<&'static str> {
    if ends_with_keyword(prefix, "if (") || ends_with_keyword(prefix, "if ") {
        return Some("if");
    }
    if ends_with_keyword(prefix, "unless (") || ends_with_keyword(prefix, "unless ") {
        return Some("unless");
    }
    if ends_with_keyword(prefix, "while (") || ends_with_keyword(prefix, "while ") {
        return Some("while");
    }
    None
}

fn condition_expression_insert_text(prefix: &str, condition: &str) -> String {
    if prefix.ends_with('(') {
        format!("{condition}) {{\n    \n}}")
    } else {
        format!("({condition}) {{\n    \n}}")
    }
}

fn lexical_assignment_rhs_prefix(prefix: &str) -> Option<(VariableSigil, &str)> {
    let lhs = prefix.trim_end().strip_suffix('=')?.trim_end();
    let (variable_start, sigil) = lhs
        .char_indices()
        .rev()
        .find_map(|(idx, ch)| VariableSigil::from_char(ch).map(|sigil| (idx, sigil)))?;
    let declaration = lhs[..variable_start].trim_end();
    let variable_name = &lhs[variable_start + 1..];

    match declaration.split_whitespace().last() {
        Some("my") => {}
        _ => return None,
    }

    if variable_name.is_empty() || !variable_name.chars().all(is_identifier_fragment_char) {
        return None;
    }

    Some((sigil, variable_name))
}

fn test_statement_filter_text(statement: &str) -> &'static str {
    if statement.starts_with("ok(") { "ok" } else { "is" }
}

fn source_has_done_testing_call(text: &str) -> bool {
    text.lines().any(line_has_done_testing_call)
}

fn line_has_done_testing_call(line: &str) -> bool {
    let mut quote = None;
    let mut escaped = false;

    for (idx, ch) in line.char_indices() {
        if let Some(quote_char) = quote {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == quote_char {
                quote = None;
            }
            continue;
        }

        match ch {
            '#' => return false,
            '\'' | '"' => {
                quote = Some(ch);
                continue;
            }
            _ => {}
        }

        if line[idx..].starts_with("done_testing") && done_testing_call_boundaries(line, idx) {
            return true;
        }
    }

    false
}

fn done_testing_call_boundaries(line: &str, start: usize) -> bool {
    let before_ok =
        line[..start].chars().next_back().is_none_or(|ch| !is_perl_qualified_identifier_char(ch));
    if !before_ok {
        return false;
    }

    let after = &line[start + "done_testing".len()..];
    let after_ok = after.chars().next().is_none_or(|ch| !is_perl_qualified_identifier_char(ch));
    after_ok && after.trim_start().starts_with('(')
}

fn is_perl_qualified_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':')
}

fn singular_loop_variable_name(array_name: &str) -> String {
    match array_name {
        "children" => "child".into(),
        "entries" => "entry".into(),
        "items" => "item".into(),
        "people" => "person".into(),
        "statuses" => "status".into(),
        name if name.ends_with("ies") && name.len() > 3 => {
            format!("{}y", &name[..name.len() - 3])
        }
        name if name.ends_with("ches")
            || name.ends_with("shes")
            || name.ends_with("sses")
            || name.ends_with("xes")
            || name.ends_with("zes") =>
        {
            name[..name.len() - 2].to_string()
        }
        name if name.ends_with('s')
            && name.len() > 1
            && !name.ends_with("is")
            && !name.ends_with("ss")
            && !name.ends_with("us") =>
        {
            name[..name.len() - 1].to_string()
        }
        _ => "item".into(),
    }
}

fn hash_key_loop_variable_name(hash_name: &str) -> String {
    hash_name
        .strip_suffix("_by_id")
        .map(|_| "id".into())
        .or_else(|| hash_name.strip_suffix("_by_name").map(|_| "name".into()))
        .or_else(|| hash_name.strip_suffix("_by_key").map(|_| "key".into()))
        .unwrap_or_else(|| "key".into())
}

struct LineContext<'a> {
    prefix: &'a str,
    current_line: &'a str,
}

struct ReplacementFragment<'a> {
    text: &'a str,
    start_byte: usize,
}

#[derive(Debug)]
struct FunctionScope {
    name: String,
    start_line: usize,
    block_depth: i32,
    opened_block: bool,
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
    line.trim_start().starts_with("package ") && line_opens_block(line)
}

fn line_opens_block(line: &str) -> bool {
    structural_brace_scan(line).opens_block
}

fn brace_delta(line: &str) -> i32 {
    structural_brace_scan(line).delta
}

#[derive(Debug, Default)]
struct StructuralBraceScan {
    opens_block: bool,
    delta: i32,
}

fn structural_brace_scan(line: &str) -> StructuralBraceScan {
    let mut scan = StructuralBraceScan::default();
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;

    for ch in line.chars() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if single_quoted || double_quoted => escaped = true,
            '\'' if !double_quoted => single_quoted = !single_quoted,
            '"' if !single_quoted => double_quoted = !double_quoted,
            '#' if !single_quoted && !double_quoted => break,
            '{' if !single_quoted && !double_quoted => {
                scan.opens_block = true;
                scan.delta += 1;
            }
            '}' if !single_quoted && !double_quoted => scan.delta -= 1,
            _ => {}
        }
    }

    scan
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

fn collect_live_declared_variable_groups(text: &str) -> Vec<Vec<String>> {
    let (groups, final_depth) = collect_scoped_declared_variable_groups_with_final_depth(text);
    groups
        .into_iter()
        .filter(|group| group.depth <= final_depth)
        .map(|group| group.variables)
        .collect()
}

fn collect_scoped_declared_variable_groups_with_final_depth(
    text: &str,
) -> (Vec<ScopedVariableGroup>, i32) {
    let mut groups = Vec::new();
    let mut depth = 0;
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
                groups.push(ScopedVariableGroup {
                    depth: declaration_scope_depth(line, keyword_start, depth),
                    variables,
                });
            }
            search_start = keyword_start + 1;
        }
        depth += brace_delta(line);
    }
    (groups, depth)
}

#[derive(Debug)]
struct ScopedVariableGroup {
    depth: i32,
    variables: Vec<String>,
}

fn declaration_scope_depth(line: &str, keyword_start: usize, line_depth: i32) -> i32 {
    let depth_before_declaration = line_depth + brace_delta(&line[..keyword_start]);
    if declaration_is_for_loop_binding(line, keyword_start) {
        depth_before_declaration + 1
    } else {
        depth_before_declaration
    }
}

fn declaration_is_for_loop_binding(line: &str, keyword_start: usize) -> bool {
    line[..keyword_start]
        .split_whitespace()
        .next_back()
        .is_some_and(|keyword| matches!(keyword, "for" | "foreach"))
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
        .any(|line| line_assigns_variable_from_dbi_connect(line, variable.name.as_str()));
    if is_likely_dbi_database_handle(
        variable.name.as_str(),
        imported_dbi,
        assigned_from_dbi_connect,
    ) {
        return Some(DbiReceiverKind::DatabaseHandle);
    }

    let prepared_statement = non_comment_code_lines(text)
        .map(code_before_line_comment)
        .any(|line| line_assigns_variable_from_dbi_prepare(line, variable.name.as_str()));
    if is_likely_dbi_statement_handle(variable.name.as_str(), imported_dbi, prepared_statement) {
        return Some(DbiReceiverKind::StatementHandle);
    }

    None
}

fn is_likely_dbi_database_handle(name: &str, imported_dbi: bool, has_dbi_connect: bool) -> bool {
    has_dbi_connect || (matches!(name, "dbh" | "db") || name.ends_with("_dbh")) && imported_dbi
}

fn is_likely_dbi_statement_handle(
    name: &str,
    imported_dbi: bool,
    prepared_statement: bool,
) -> bool {
    prepared_statement && (name == "sth" || name.ends_with("_sth") || imported_dbi)
}

fn line_assigns_variable_from_dbi_connect(line: &str, variable_name: &str) -> bool {
    line_assigns_variable_from_method_receiver(line, variable_name, "connect")
        .is_some_and(|receiver| receiver == "DBI")
        || line_assigns_variable_from_method_receiver(line, variable_name, "connect_cached")
            .is_some_and(|receiver| receiver == "DBI")
}

fn line_assigns_variable_from_dbi_prepare(line: &str, variable_name: &str) -> bool {
    line_assigns_variable_from_method_receiver(line, variable_name, "prepare")
        .is_some_and(is_likely_dbi_database_receiver)
}

fn is_likely_dbi_database_receiver(receiver: &str) -> bool {
    receiver == "$dbh" || receiver == "$db" || receiver.ends_with("_dbh")
}

fn line_assigns_variable_from_method_receiver<'line>(
    line: &'line str,
    variable_name: &str,
    method_name: &str,
) -> Option<&'line str> {
    let (left, right) = line.split_once('=')?;
    let variable = format!("${variable_name}");
    if !left
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$')))
        .any(|part| part == variable)
    {
        return None;
    }

    let method_call = format!("->{method_name}");
    let method_start = right.find(method_call.as_str())?;
    let receiver_prefix = right[..method_start].trim_end();
    let receiver_start = receiver_prefix
        .char_indices()
        .rev()
        .find_map(|(idx, ch)| (!is_receiver_fragment_char(ch)).then_some(idx + ch.len_utf8()))
        .unwrap_or(0);
    let receiver = receiver_prefix[receiver_start..].trim();
    (!receiver.is_empty()).then_some(receiver)
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

    if cursor_is_inside_format_body(text, cursor_offset) {
        return Some(HardRejectZone::HeredocBody);
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

fn cursor_is_inside_format_body(text: &str, cursor_offset: usize) -> bool {
    let mut body_start = None;
    for (line_start, line_end, line_text) in line_spans(text) {
        if body_start.is_none() {
            if is_format_declaration_line(line_text) {
                body_start = Some(line_end);
            }
            continue;
        }

        if is_format_terminator_line(line_text) {
            body_start = None;
            continue;
        }

        if body_start.is_some_and(|start| start <= cursor_offset && cursor_offset < line_end) {
            return true;
        }

        if cursor_offset < line_start {
            return false;
        }
    }

    body_start.is_some_and(|start| start <= cursor_offset)
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

fn is_format_declaration_line(line: &str) -> bool {
    let code = code_before_line_comment(line).trim();
    let Some(rest) = code.strip_prefix("format") else {
        return false;
    };

    rest.chars().next().is_some_and(is_keyword_boundary) && code.ends_with('=')
}

fn is_format_terminator_line(line: &str) -> bool {
    line.trim() == "."
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
    module_fragment(fragment)
}

fn require_completion_fragment(prefix: &str) -> Option<&str> {
    let require_index = last_keyword_index(prefix, "require ")?;
    let fragment = &prefix[require_index + 8..];
    module_fragment(fragment)
}

fn module_statement_fragment(prefix: &str) -> Option<&str> {
    use_completion_fragment(prefix).or_else(|| require_completion_fragment(prefix))
}

fn module_fragment(fragment: &str) -> Option<&str> {
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
    fn require_namespace_suggests_available_module_from_environment()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let environment = InlineCompletionEnvironment {
            available_modules: vec!["My::App".to_string(), "Other::Tool".to_string()],
        };
        let completions =
            provider.get_inline_completions_with_environment("require My::", 0, 12, &environment);

        let module = completions
            .items
            .iter()
            .find(|item| item.insert_text == "My::App;")
            .ok_or("expected require module inline completion")?;
        assert_eq!(module.filter_text.as_deref(), Some("My::App"));
        let range =
            module.range.as_ref().ok_or("require completion should replace typed prefix")?;
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 8);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 12);
        assert!(completions.items.iter().all(|item| item.insert_text != "strict;"));
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
    fn parse_safety_rejects_multiline_candidate_that_adds_error()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let context = provider.prepare_context("", 0, 0).ok_or("expected prepared context")?;
        let list = InlineCompletionList {
            items: vec![
                InlineCompletionItem {
                    insert_text: "my $value = 1;\nreturn $value;".into(),
                    filter_text: Some("$value".into()),
                    range: None,
                    command: None,
                },
                InlineCompletionItem {
                    insert_text: "my $value = ;\nreturn $value;".into(),
                    filter_text: Some("$value".into()),
                    range: None,
                    command: None,
                },
            ],
        };

        let filtered = provider.filter_parse_safe_items(list, &context, 0, 0);

        assert!(
            filtered.items.iter().any(|item| item.insert_text == "my $value = 1;\nreturn $value;")
        );
        assert!(
            filtered.items.iter().all(|item| item.insert_text != "my $value = ;\nreturn $value;")
        );
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
    fn lexical_assignment_rhs_uses_visible_source_scalar() -> Result<(), Box<dyn std::error::Error>>
    {
        let provider = InlineCompletionProvider::new();
        let source = "sub copy {\n    my $result = compute();\n    my $copy = ";
        let line = 2;
        let character = "    my $copy = ".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, line, character);

        assert_eq!(
            completions.items.first().map(|item| item.insert_text.as_str()),
            Some("$result;")
        );
        assert!(completions.items.iter().all(|item| item.insert_text != "$copy;"));
        Ok(())
    }

    #[test]
    fn lexical_assignment_rhs_uses_matching_aggregate_sigil()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let array_source = "sub copy {\n    my @users = fetch_users();\n    my @copy = ";
        let array_line = 2;
        let array_character = "    my @copy = ".encode_utf16().count() as u32;
        let array_completions =
            provider.get_inline_completions(array_source, array_line, array_character);

        assert_eq!(
            array_completions.items.first().map(|item| item.insert_text.as_str()),
            Some("@users;")
        );
        assert!(array_completions.items.iter().all(|item| item.insert_text != "@copy;"));
        assert!(array_completions.items.iter().all(|item| item.insert_text != "$users;"));

        let hash_source = "sub copy {\n    my %users_by_id = load_users();\n    my %copy = ";
        let hash_line = 2;
        let hash_character = "    my %copy = ".encode_utf16().count() as u32;
        let hash_completions =
            provider.get_inline_completions(hash_source, hash_line, hash_character);

        assert_eq!(
            hash_completions.items.first().map(|item| item.insert_text.as_str()),
            Some("%users_by_id;")
        );
        assert!(hash_completions.items.iter().all(|item| item.insert_text != "%copy;"));
        assert!(hash_completions.items.iter().all(|item| item.insert_text != "$users_by_id;"));
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
    fn non_dbi_connect_receiver_does_not_get_dbi_methods() {
        let provider = InlineCompletionProvider::new();
        let source = "my $socket = Client->connect($dsn);\n$socket->\n";
        let character = "$socket->".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 1, character);
        let insert_texts: Vec<&str> =
            completions.items.iter().map(|item| item.insert_text.as_str()).collect();

        for unexpected in ["prepare()", "do()", "disconnect()"] {
            assert!(
                !insert_texts.contains(&unexpected),
                "non-DBI connect receivers must not receive DBI methods: {insert_texts:?}"
            );
        }
    }

    #[test]
    fn imported_dbi_unrelated_prepare_receiver_does_not_get_statement_methods() {
        let provider = InlineCompletionProvider::new();
        let source = "use DBI;\nmy $query = $builder->prepare($sql);\n$query->f\n";
        let character = "$query->f".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);

        assert!(
            completions.items.iter().all(|item| item.insert_text != "fetchrow_hashref()"),
            "non-DBI prepare receivers must not receive statement-handle methods: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
    }

    #[test]
    fn future_dbi_assignment_does_not_affect_current_receiver_completion() {
        let provider = InlineCompletionProvider::new();
        let source = "use DBI;\n$conn->\nmy $conn = DBI->connect($dsn);\n";
        let character = "$conn->".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 1, character);
        let insert_texts: Vec<&str> =
            completions.items.iter().map(|item| item.insert_text.as_str()).collect();

        for unexpected in ["prepare()", "do()", "disconnect()"] {
            assert!(
                !insert_texts.contains(&unexpected),
                "future DBI assignments must not shape current receiver completions: {insert_texts:?}"
            );
        }
    }

    #[test]
    fn guard_condition_prefers_boolean_named_visible_scalar() {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $result = compute();\n    my $is_valid = validate($result);\n    return unless ";
        let character = "    return unless ".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 3, character);
        let insert_texts: Vec<&str> =
            completions.items.iter().map(|item| item.insert_text.as_str()).collect();

        assert!(insert_texts.contains(&"$is_valid;"));
        assert!(!insert_texts.contains(&"$result;"));
    }

    #[test]
    fn guard_condition_stays_quiet_without_visible_scalar() {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my @users = fetch_users();\n    return unless ";
        let character = "    return unless ".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);

        assert!(completions.items.is_empty());
    }

    #[test]
    fn guard_condition_prefers_valid_scalar_name() {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $valid = validate();\n    return unless ";
        let character = "    return unless ".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);

        assert_eq!(
            completions.items.first().map(|item| item.insert_text.as_str()),
            Some("$valid;")
        );
    }

    #[test]
    fn guard_condition_prefers_ready_scalar_name() {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $ready = is_ready();\n    return if ";
        let character = "    return if ".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);

        assert_eq!(
            completions.items.first().map(|item| item.insert_text.as_str()),
            Some("$ready;")
        );
    }

    #[test]
    fn guard_condition_prefers_ok_scalar_name() {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $ok = check_status();\n    last if ";
        let character = "    last if ".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);

        assert_eq!(completions.items.first().map(|item| item.insert_text.as_str()), Some("$ok;"));
    }

    #[test]
    fn guard_condition_prefers_has_prefix_scalar_name() {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $has_value = load_value();\n    return unless ";
        let character = "    return unless ".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);

        assert_eq!(
            completions.items.first().map(|item| item.insert_text.as_str()),
            Some("$has_value;")
        );
    }

    #[test]
    fn guard_condition_prefers_can_prefix_scalar_name() {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $can_retry = should_retry();\n    return if ";
        let character = "    return if ".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);

        assert_eq!(
            completions.items.first().map(|item| item.insert_text.as_str()),
            Some("$can_retry;")
        );
    }

    #[test]
    fn guard_condition_prefers_ok_suffix_scalar_name() {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $status_ok = check_status();\n    next if ";
        let character = "    next if ".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);

        assert_eq!(
            completions.items.first().map(|item| item.insert_text.as_str()),
            Some("$status_ok;")
        );
    }

    #[test]
    fn loop_guard_condition_handles_next_unless_with_visible_scalar() {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $should_skip = should_skip();\n    next unless ";
        let character = "    next unless ".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);

        assert_eq!(
            completions.items.first().map(|item| item.insert_text.as_str()),
            Some("$should_skip;")
        );
    }

    #[test]
    fn loop_guard_condition_handles_last_unless_with_visible_scalar() {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $has_more = iterator_has_more();\n    last unless ";
        let character = "    last unless ".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);

        assert_eq!(
            completions.items.first().map(|item| item.insert_text.as_str()),
            Some("$has_more;")
        );
    }

    #[test]
    fn guard_condition_does_not_emit_condition_expression_block()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $ready = is_ready();\n    return if ";
        let character = "    return if ".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);

        // The guard completion ($ready;) must be present.
        let guard_present = completions.items.iter().any(|item| item.insert_text == "$ready;");
        assert!(guard_present);

        // Guard contexts do not emit condition blocks.
        assert!(completions.items.iter().all(|item| !item.insert_text.contains("{\n")));

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
    fn lexical_assignment_rhs_prefix_requires_my_declaration()
    -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            lexical_assignment_rhs_prefix("my $copy = "),
            Some((VariableSigil::Scalar, "copy"))
        );
        assert_eq!(
            lexical_assignment_rhs_prefix("my @copy = "),
            Some((VariableSigil::Array, "copy"))
        );
        assert_eq!(
            lexical_assignment_rhs_prefix("my %copy = "),
            Some((VariableSigil::Hash, "copy"))
        );
        assert_eq!(lexical_assignment_rhs_prefix("dummy $copy = "), None);
        assert_eq!(lexical_assignment_rhs_prefix("myself $copy = "), None);
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
    fn prepared_context_excludes_variables_from_closed_blocks()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "{\n    my @users = fetch_users();\n}\nfor ";
        let prepared = provider.prepare_context(source, 3, 4).ok_or("expected prepared context")?;
        let completions = provider.get_inline_completions(source, 3, 4);

        assert!(
            prepared.variables.iter().all(|variable| variable != "@users"),
            "closed block array should not be visible at the cursor: {:?}",
            prepared.variables
        );
        assert!(
            completions.items.is_empty(),
            "loop binding should stay silent when the only collection is out of scope: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn prepared_context_excludes_variables_from_closed_subroutines()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $result = compute();\n}\n\nreturn ";
        let prepared = provider.prepare_context(source, 4, 7).ok_or("expected prepared context")?;
        let completions = provider.get_inline_completions(source, 4, 7);

        assert_eq!(prepared.current_function, None);
        assert!(
            prepared.variables.iter().all(|variable| variable != "$result"),
            "closed subroutine scalar should not be visible at the cursor: {:?}",
            prepared.variables
        );
        assert!(
            completions.items.iter().all(|item| item.insert_text != "$result;"),
            "return completion should not use a closed subroutine lexical: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn prepared_context_excludes_variables_from_single_line_closed_subroutines()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper { my $result = compute(); }\n\nreturn ";
        let prepared = provider.prepare_context(source, 2, 7).ok_or("expected prepared context")?;
        let completions = provider.get_inline_completions(source, 2, 7);

        assert_eq!(prepared.current_function, None);
        assert!(
            prepared.variables.iter().all(|variable| variable != "$result"),
            "single-line closed subroutine scalar should not be visible at the cursor: {:?}",
            prepared.variables
        );
        assert!(
            completions.items.iter().all(|item| item.insert_text != "$result;"),
            "return completion should not use a single-line closed subroutine lexical: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn structural_brace_scan_ignores_simple_quoted_braces() {
        assert!(!line_opens_block("my $text = \"{\";"));
        assert!(!line_opens_block("my $text = '{';"));
        assert_eq!(brace_delta("my $text = \"{\";"), 0);
        assert_eq!(brace_delta("my $text = \"}\";"), 0);
        assert_eq!(brace_delta("my $text = \"\\\"{\";"), 0);
        assert_eq!(brace_delta("sub helper {"), 1);
        assert_eq!(brace_delta("}"), -1);
    }

    #[test]
    fn prepared_context_excludes_variables_when_closed_sub_contains_open_brace_string()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $result = \"{\";\n}\n\nreturn ";
        let prepared = provider.prepare_context(source, 4, 7).ok_or("expected prepared context")?;
        let completions = provider.get_inline_completions(source, 4, 7);

        assert_eq!(prepared.current_function, None);
        assert!(
            prepared.variables.iter().all(|variable| variable != "$result"),
            "closed subroutine scalar should not stay visible because a string contains '{{': {:?}",
            prepared.variables
        );
        assert!(
            completions.items.iter().all(|item| item.insert_text != "$result;"),
            "return completion should not use a closed subroutine lexical after quoted '{{': {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn prepared_context_keeps_function_scope_when_string_contains_close_brace()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $text = \"}\";\n    return ";
        let prepared =
            provider.prepare_context(source, 2, 11).ok_or("expected prepared context")?;
        let completions = provider.get_inline_completions(source, 2, 11);

        assert_eq!(prepared.current_function.as_deref(), Some("helper"));
        assert!(
            prepared.variables.iter().any(|variable| variable == "$text"),
            "lexical declared inside the active subroutine should remain visible: {:?}",
            prepared.variables
        );
        assert!(
            completions.items.iter().any(|item| item.insert_text == "$text;"),
            "return completion should still use active subroutine lexical after quoted '}}': {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
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
    fn return_partial_variable_replaces_typed_fragment() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $result = compute();\n    return $res\n}\n";
        let character = "    return $res".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);
        let item = completions
            .items
            .iter()
            .find(|item| item.insert_text == "$result;")
            .ok_or("expected partial return variable completion")?;
        let range = item.range.as_ref().ok_or("partial return must carry a range")?;

        assert_eq!(range.start.line, 2);
        assert_eq!(range.start.character, "    return ".encode_utf16().count() as u32);
        assert_eq!(range.end.line, 2);
        assert_eq!(range.end.character, character);
        Ok(())
    }

    #[test]
    fn return_context_keeps_all_matching_visible_variables_ranked_by_recency()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $result = compute();\n    my $status = check($result);\n    return $\n}\n";
        let character = "    return $".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 3, character);

        assert_eq!(
            completions.items.first().map(|item| item.insert_text.as_str()),
            Some("$status;")
        );
        assert!(completions.items.iter().any(|item| item.insert_text == "$result;"));
        Ok(())
    }

    #[test]
    fn method_blank_line_prefers_domain_scalar_over_self() -> Result<(), Box<dyn std::error::Error>>
    {
        let provider = InlineCompletionProvider::new();
        let source =
            "sub render {\n    my $self = shift;\n    my $result = $self->build_result;\n    \n}\n";
        let completions = provider.get_inline_completions(source, 3, 4);
        let first = completions.items.first().ok_or("expected return completion")?;

        assert_eq!(first.insert_text, "return $result;");
        assert!(
            completions.items.iter().all(|item| item.insert_text != "return $self;"),
            "non-constructor methods should not prefer returning receiver state over a closer scalar: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn constructor_return_context_still_prefers_self() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();

        for constructor in ["new", "BUILD"] {
            let source = format!(
                "sub {constructor} {{\n    my $class = shift;\n    my $self = bless {{}}, $class;\n    my $result = $self;\n    return \n}}\n"
            );
            let character = "    return ".encode_utf16().count() as u32;
            let completions = provider.get_inline_completions(&source, 4, character);
            let first =
                completions.items.first().ok_or("expected constructor return completion")?;

            assert_eq!(
                first.insert_text, "$self;",
                "{constructor} should keep preferring the constructed receiver"
            );
        }
        Ok(())
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
    fn subtest_in_test_file_suggests_block() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "use Test2::V0;\n\nsubtest ";
        let character = "subtest ".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);

        let item = completions
            .items
            .iter()
            .find(|item| item.insert_text.starts_with("'test description' => sub"))
            .ok_or("expected subtest block completion")?;
        assert_eq!(item.insert_text, "'test description' => sub {\n    \n};");
        assert_eq!(item.filter_text.as_deref(), Some("subtest"));
        Ok(())
    }

    #[test]
    fn subtest_in_test_more_file_suggests_block() {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\n\nsubtest ";
        let character = "subtest ".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);

        assert!(
            completions
                .items
                .iter()
                .any(|item| item.insert_text == "'test description' => sub {\n    \n};"),
            "Test::More files should get subtest block completions: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
    }

    #[test]
    fn subtest_without_test_import_stays_quiet() {
        let provider = InlineCompletionProvider::new();
        let source = "subtest ";
        let character = source.encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 0, character);

        assert!(
            completions
                .items
                .iter()
                .all(|item| !item.insert_text.starts_with("'test description' => sub")),
            "non-test files should not get subtest block completions: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
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
    fn test_blank_line_suggests_done_testing_when_missing() -> Result<(), Box<dyn std::error::Error>>
    {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\nok(1, 'works');\n\n";
        let completions = provider.get_inline_completions(source, 2, 0);

        assert!(
            completions.items.iter().any(|item| item.insert_text == "done_testing();"),
            "test files without done_testing should keep the fallback: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn test_blank_line_does_not_suggest_duplicate_done_testing()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\nok(1, 'works');\ndone_testing();\n\n";
        let completions = provider.get_inline_completions(source, 3, 0);

        assert!(
            completions.items.iter().all(|item| item.insert_text != "done_testing();"),
            "test files with done_testing should not duplicate it: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn test_commented_done_testing_does_not_suppress_fallback()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\n# done_testing();\n\n";
        let completions = provider.get_inline_completions(source, 2, 0);

        assert!(
            completions.items.iter().any(|item| item.insert_text == "done_testing();"),
            "commented done_testing should not suppress the fallback: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn test_string_and_identifier_done_testing_mentions_do_not_suppress_fallback()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\nmy $not_done_testing = 'done_testing(); is mentioned';\n\n";
        let completions = provider.get_inline_completions(source, 2, 0);

        assert!(
            completions.items.iter().any(|item| item.insert_text == "done_testing();"),
            "non-call done_testing mentions should not suppress the fallback: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn test_done_testing_detector_skips_escaped_quote_mention_before_real_call()
    -> Result<(), Box<dyn std::error::Error>> {
        let line = r#"my $escaped = "escaped \" done_testing(); still string"; done_testing();"#;

        assert!(line_has_done_testing_call(line));
        Ok(())
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
    fn if_condition_uses_visible_boolean_scalar() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "my $is_ready = check_ready();\nif ";
        let completions = provider.get_inline_completions(source, 1, 3);
        let first = completions.items.first().ok_or("expected if condition completion")?;

        assert_eq!(first.insert_text, "($is_ready) {\n    \n}");
        Ok(())
    }

    #[test]
    fn while_open_paren_condition_closes_existing_paren() -> Result<(), Box<dyn std::error::Error>>
    {
        let provider = InlineCompletionProvider::new();
        let source = "my $ok = keep_going();\nwhile (";
        let completions = provider.get_inline_completions(source, 1, 7);
        let first = completions.items.first().ok_or("expected while condition completion")?;

        assert_eq!(first.insert_text, "$ok) {\n    \n}");
        Ok(())
    }

    #[test]
    fn control_condition_without_visible_scalar_stays_silent()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("if ", 0, 3);

        assert_eq!(completions.items.len(), 0);
        Ok(())
    }

    #[test]
    fn for_loop_uses_visible_array_for_binding() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "my @users = fetch_users();\nfor ";
        let completions = provider.get_inline_completions(source, 1, 4);
        let first = completions.items.first().ok_or("expected loop binding completion")?;

        assert_eq!(first.insert_text, "my $user (@users) {\n    \n}");
        assert!(
            completions.items.iter().all(|item| !item.insert_text.contains("(@items)")),
            "loop completion must use visible arrays instead of snippet placeholders: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn foreach_loop_singularizes_visible_array_name() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "my @entries = read_entries();\nforeach ";
        let completions = provider.get_inline_completions(source, 1, 8);
        let first = completions.items.first().ok_or("expected foreach binding completion")?;

        assert_eq!(first.insert_text, "my $entry (@entries) {\n    \n}");
        Ok(())
    }

    #[test]
    fn loop_binding_does_not_blindly_trim_non_plural_s_suffixes()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "my @status = fetch_status();\nfor ";
        let completions = provider.get_inline_completions(source, 1, 4);
        let first = completions.items.first().ok_or("expected loop binding completion")?;

        assert_eq!(first.insert_text, "my $item (@status) {\n    \n}");
        assert!(
            completions.items.iter().all(|item| !item.insert_text.contains("$statu")),
            "loop binding must not trim singular-looking names ending in s: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn loop_binding_handles_statuses_plural() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "my @statuses = fetch_statuses();\nfor ";
        let completions = provider.get_inline_completions(source, 1, 4);
        let first = completions.items.first().ok_or("expected loop binding completion")?;

        assert_eq!(first.insert_text, "my $status (@statuses) {\n    \n}");
        Ok(())
    }

    #[test]
    fn for_loop_uses_visible_hash_keys_when_no_array_is_available()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "my %users_by_id = load_users();\nfor ";
        let completions = provider.get_inline_completions(source, 1, 4);
        let first = completions.items.first().ok_or("expected hash key loop completion")?;

        assert_eq!(first.insert_text, "my $id (keys %users_by_id) {\n    \n}");
        assert!(
            completions.items.iter().all(|item| !item.insert_text.contains("(@items)")),
            "loop completion must not fall back to snippet placeholders: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn for_loop_prefers_visible_array_over_hash() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "my %users_by_id = load_users();\nmy @users = values %users_by_id;\nfor ";
        let completions = provider.get_inline_completions(source, 2, 4);
        let first = completions.items.first().ok_or("expected array loop completion")?;

        assert_eq!(first.insert_text, "my $user (@users) {\n    \n}");
        assert!(
            completions.items.iter().all(|item| !item.insert_text.contains("keys %users_by_id")),
            "visible arrays should stay preferred over hash key loops: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn loop_binding_without_visible_collection_stays_silent() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("for ", 0, 4);

        assert!(
            completions.items.is_empty(),
            "loop binding should not invent placeholder collections: {:?}",
            completions.items.iter().map(|item| &item.insert_text).collect::<Vec<_>>()
        );
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
    fn package_receiver_suggests_current_package_methods() -> Result<(), Box<dyn std::error::Error>>
    {
        let provider = InlineCompletionProvider::new();
        let source = "package Demo::Widget;\nsub save {}\nsub render {}\nDemo::Widget->sa";
        let character = "Demo::Widget->sa".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 3, character);
        let item = completions
            .items
            .iter()
            .find(|item| item.insert_text == "save()")
            .ok_or("expected current package method completion for package receiver")?;

        assert_eq!(item.filter_text.as_deref(), Some("save"));
        let range = item.range.as_ref().ok_or("typed method fragment should be replaced")?;
        assert_eq!(range.start.line, 3);
        assert_eq!(range.start.character, 14);
        assert_eq!(range.end.line, 3);
        assert_eq!(range.end.character, character);
        assert!(completions.items.iter().all(|item| item.insert_text != "render()"));
        Ok(())
    }

    #[test]
    fn package_magic_receiver_suggests_current_package_methods()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "package Demo::Widget;\nsub save {}\n__PACKAGE__->";
        let character = "__PACKAGE__->".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);

        assert!(
            completions.items.iter().any(|item| item.insert_text == "save()"),
            "__PACKAGE__ receiver should use current package methods: {:?}",
            completions.items
        );
        Ok(())
    }

    #[test]
    fn different_package_receiver_does_not_suggest_current_package_methods()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "package Demo::Widget;\nsub save {}\nOther::Widget->sa";
        let character = "Other::Widget->sa".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 2, character);

        assert!(completions.items.iter().all(|item| item.insert_text != "save()"));
        Ok(())
    }

    #[test]
    fn candidate_metadata_explains_semantic_sources() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();

        let module_source = "package Demo;\nuse My::";
        let module_character = "use My::".encode_utf16().count() as u32;
        let module_prepared = provider
            .prepare_context(module_source, 1, module_character)
            .ok_or("expected module context")?;
        let mut module_context = provider.semantic_context_for_prepared_context(&module_prepared);
        module_context.available_modules = vec![ModuleFact { name: "My::App".into() }];
        let module_item = InlineCompletionItem {
            insert_text: "My::App;".into(),
            filter_text: Some("My::App".into()),
            range: None,
            command: None,
        };
        let module_metadata = InlineCandidateMetadata::for_candidate(
            InlineCandidateSourceKind::Module,
            &module_item,
            &module_context,
        );
        assert_eq!(module_metadata.source, InlineCandidateSourceKind::Module);
        assert_eq!(module_metadata.reason, InlineCandidateReason::EffectiveIncModule);
        assert_eq!(module_metadata.confidence, InlineCandidateConfidence::High);

        let receiver_source = "package Demo;\nsub save {}\nsub caller {\n    $self->\n}\n";
        let receiver_character = "    $self->".encode_utf16().count() as u32;
        let receiver_prepared = provider
            .prepare_context(receiver_source, 3, receiver_character)
            .ok_or("expected receiver context")?;
        let receiver_context =
            provider.semantic_context_for_source(receiver_source, &receiver_prepared);
        let receiver_item = InlineCompletionItem {
            insert_text: "save()".into(),
            filter_text: Some("save".into()),
            range: None,
            command: None,
        };
        let receiver_metadata = InlineCandidateMetadata::for_candidate(
            InlineCandidateSourceKind::Receiver,
            &receiver_item,
            &receiver_context,
        );
        assert_eq!(receiver_metadata.reason, InlineCandidateReason::CurrentPackageMethod);
        assert_eq!(receiver_metadata.confidence, InlineCandidateConfidence::High);

        Ok(())
    }

    #[test]
    fn candidate_metadata_explains_fallback_sources() -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let prepared = provider.prepare_context("return ", 0, 7).ok_or("expected context")?;
        let mut semantic = provider.semantic_context_for_prepared_context(&prepared);
        let lexical_item = InlineCompletionItem {
            insert_text: "$result;".into(),
            filter_text: Some("$result".into()),
            range: None,
            command: None,
        };
        let lexical_metadata = InlineCandidateMetadata::for_candidate(
            InlineCandidateSourceKind::Syntax,
            &lexical_item,
            &semantic,
        );
        assert_eq!(lexical_metadata.reason, InlineCandidateReason::VisibleLexical);
        assert_eq!(lexical_metadata.confidence, InlineCandidateConfidence::High);

        semantic.expected_syntax = ExpectedSyntax::Unknown;
        semantic.receiver_hint = Some(ReceiverHint::Variable(VariableFact {
            sigil: VariableSigil::Scalar,
            name: "dbh".into(),
        }));
        semantic.dbi_receiver_kind = Some(DbiReceiverKind::DatabaseHandle);
        let dbi_item = InlineCompletionItem {
            insert_text: "prepare()".into(),
            filter_text: Some("prepare".into()),
            range: None,
            command: None,
        };
        let dbi_metadata = InlineCandidateMetadata::for_candidate(
            InlineCandidateSourceKind::Receiver,
            &dbi_item,
            &semantic,
        );
        assert_eq!(dbi_metadata.reason, InlineCandidateReason::DbiReceiverMethod);

        let source_reason_cases = [
            (InlineCandidateSourceKind::Module, InlineCandidateReason::SourceModule),
            (InlineCandidateSourceKind::Syntax, InlineCandidateReason::SourceSyntax),
            (InlineCandidateSourceKind::Test, InlineCandidateReason::SourceTest),
            (InlineCandidateSourceKind::Shebang, InlineCandidateReason::SourceShebang),
            (
                InlineCandidateSourceKind::ContextualFallback,
                InlineCandidateReason::SourceContextualFallback,
            ),
        ];
        for (source, reason) in source_reason_cases {
            let metadata = InlineCandidateMetadata::for_candidate(source, &dbi_item, &semantic);
            assert_eq!(metadata.reason, reason, "source {source:?}");
        }

        semantic.dbi_receiver_kind = None;
        let receiver_metadata = InlineCandidateMetadata::for_candidate(
            InlineCandidateSourceKind::Receiver,
            &dbi_item,
            &semantic,
        );
        assert_eq!(receiver_metadata.reason, InlineCandidateReason::SourceReceiver);
        assert_eq!(receiver_metadata.confidence, InlineCandidateConfidence::Low);

        Ok(())
    }

    #[test]
    fn candidate_metadata_tiebreak_ranks_are_stable() -> Result<(), Box<dyn std::error::Error>> {
        let source_ranks: Vec<_> = [
            InlineCandidateSourceKind::Receiver,
            InlineCandidateSourceKind::Module,
            InlineCandidateSourceKind::Syntax,
            InlineCandidateSourceKind::Test,
            InlineCandidateSourceKind::Shebang,
            InlineCandidateSourceKind::ContextualFallback,
        ]
        .into_iter()
        .map(|source| source.stable_rank())
        .collect();
        assert_eq!(source_ranks, vec![0, 1, 2, 3, 4, 5]);

        let reason_ranks: Vec<_> = [
            InlineCandidateReason::CurrentPackageMethod,
            InlineCandidateReason::DbiReceiverMethod,
            InlineCandidateReason::EffectiveIncModule,
            InlineCandidateReason::VisibleLexical,
            InlineCandidateReason::SourceReceiver,
            InlineCandidateReason::SourceModule,
            InlineCandidateReason::SourceSyntax,
            InlineCandidateReason::SourceTest,
            InlineCandidateReason::SourceShebang,
            InlineCandidateReason::SourceContextualFallback,
        ]
        .into_iter()
        .map(|reason| reason.stable_rank())
        .collect();
        assert_eq!(reason_ranks, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

        let confidence_ranks: Vec<_> = [
            InlineCandidateConfidence::High,
            InlineCandidateConfidence::Medium,
            InlineCandidateConfidence::Low,
        ]
        .into_iter()
        .map(|confidence| confidence.stable_rank())
        .collect();
        assert_eq!(confidence_ranks, vec![0, 1, 2]);

        let high_confidence = InlineCandidateMetadata {
            source: InlineCandidateSourceKind::Receiver,
            reason: InlineCandidateReason::CurrentPackageMethod,
            confidence: InlineCandidateConfidence::High,
        };
        let low_confidence = InlineCandidateMetadata {
            source: InlineCandidateSourceKind::ContextualFallback,
            reason: InlineCandidateReason::SourceContextualFallback,
            confidence: InlineCandidateConfidence::Low,
        };
        assert!(high_confidence.stable_tiebreak() < low_confidence.stable_tiebreak());

        Ok(())
    }

    fn ranked_candidate(
        source: InlineCandidateSourceKind,
        priority: u8,
        order: usize,
        insert_text: &str,
        filter_text: Option<&str>,
        semantic_context: &SemanticInlineContext,
    ) -> RankedCompletionItem {
        let item = InlineCompletionItem {
            insert_text: insert_text.into(),
            filter_text: filter_text.map(str::to_string),
            range: None,
            command: None,
        };
        let score = InlineCandidateScore::for_candidate(source, priority, &item, semantic_context);
        let metadata = InlineCandidateMetadata::for_candidate(source, &item, semantic_context);
        RankedCompletionItem { score, order, metadata, item }
    }

    #[test]
    fn ranking_calibration_prefers_effective_inc_module_over_generic_use_suggestion()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "package Demo;\nuse My::";
        let character = "use My::".encode_utf16().count() as u32;
        let prepared =
            provider.prepare_context(source, 1, character).ok_or("expected use-module context")?;
        let mut semantic = provider.semantic_context_for_prepared_context(&prepared);
        semantic.available_modules = vec![ModuleFact { name: "My::App".into() }];

        let normalized = provider.normalize_items(vec![
            ranked_candidate(
                InlineCandidateSourceKind::Syntax,
                0,
                0,
                "strict;",
                Some("strict"),
                &semantic,
            ),
            ranked_candidate(
                InlineCandidateSourceKind::Module,
                0,
                1,
                "My::App;",
                Some("My::App"),
                &semantic,
            ),
        ]);

        assert_eq!(normalized[0].insert_text, "My::App;");
        assert_eq!(normalized[1].insert_text, "strict;");
        Ok(())
    }

    #[test]
    fn ranking_calibration_prefers_current_package_method_over_generic_receiver()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "package Demo;\nsub save {}\nsub caller {\n    $self->\n}\n";
        let character = "    $self->".encode_utf16().count() as u32;
        let prepared =
            provider.prepare_context(source, 3, character).ok_or("expected receiver context")?;
        let semantic = provider.semantic_context_for_source(source, &prepared);

        let normalized = provider.normalize_items(vec![
            ranked_candidate(
                InlineCandidateSourceKind::Receiver,
                0,
                0,
                "new()",
                Some("new"),
                &semantic,
            ),
            ranked_candidate(
                InlineCandidateSourceKind::Receiver,
                0,
                1,
                "save()",
                Some("save"),
                &semantic,
            ),
        ]);

        assert_eq!(normalized[0].insert_text, "save()");
        assert_eq!(normalized[1].insert_text, "new()");
        Ok(())
    }

    #[test]
    fn ranking_calibration_prefers_test_assertion_over_generic_return()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\nmy $got = compute();\n\n";
        let prepared = provider.prepare_context(source, 2, 0).ok_or("expected test context")?;
        let semantic = provider.semantic_context_for_source(source, &prepared);

        let normalized = provider.normalize_items(vec![
            ranked_candidate(
                InlineCandidateSourceKind::ContextualFallback,
                0,
                0,
                "return $got;",
                Some("$got"),
                &semantic,
            ),
            ranked_candidate(
                InlineCandidateSourceKind::ContextualFallback,
                0,
                1,
                "is($got, $expected, 'test description');",
                Some("is"),
                &semantic,
            ),
        ]);

        assert_eq!(normalized[0].insert_text, "is($got, $expected, 'test description');");
        assert_eq!(normalized[1].insert_text, "return $got;");
        Ok(())
    }

    #[test]
    fn test_assertion_context_suppresses_generic_return_candidate()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\nmy $got = compute();\nmy $expected = 42;\n\n";

        let completions = provider.get_inline_completions(source, 4, 0);

        assert!(
            completions
                .items
                .iter()
                .any(|item| item.insert_text == "is($got, $expected, 'test description');")
        );
        assert!(
            completions.items.iter().all(|item| !item.insert_text.starts_with("return ")),
            "test assertion slots should not include generic return candidates: {:?}",
            completions.items
        );
        Ok(())
    }

    #[test]
    fn ok_assertion_context_keeps_generic_return_candidate()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\nmy $got = compute();\n\n";

        let completions = provider.get_inline_completions(source, 3, 0);

        assert!(
            completions
                .items
                .iter()
                .any(|item| item.insert_text == "ok($got, 'test description');")
        );
        assert!(
            completions.items.iter().any(|item| item.insert_text == "return $got;"),
            "weaker ok(...) assertion slots should keep generic return fallback: {:?}",
            completions.items
        );
        Ok(())
    }

    #[test]
    fn ranking_calibration_prefers_visible_guard_scalar_over_generic_condition()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $is_valid = validate();\n    return unless ";
        let character = "    return unless ".encode_utf16().count() as u32;
        let prepared =
            provider.prepare_context(source, 2, character).ok_or("expected guard context")?;
        let semantic = provider.semantic_context_for_source(source, &prepared);

        let normalized = provider.normalize_items(vec![
            ranked_candidate(
                InlineCandidateSourceKind::ContextualFallback,
                0,
                0,
                "$condition;",
                Some("$condition"),
                &semantic,
            ),
            ranked_candidate(
                InlineCandidateSourceKind::Syntax,
                0,
                1,
                "$is_valid;",
                Some("$is_valid"),
                &semantic,
            ),
        ]);

        assert_eq!(normalized[0].insert_text, "$is_valid;");
        assert_eq!(normalized[1].insert_text, "$condition;");
        Ok(())
    }

    #[test]
    fn ranking_calibration_keeps_signature_constructor_style_first()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let source = "sub existing ($class, %args) {\n    my $self = bless {}, $class;\n    return $self;\n}\n\nsub new";
        let character = "sub new".encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 5, character);
        let first = completions.items.first().ok_or("expected constructor completion")?;

        assert!(
            first.insert_text.starts_with(" ($class, %args) {"),
            "signature-style constructor should keep signature arguments first: {}",
            first.insert_text
        );
        assert!(
            !first.insert_text.contains("my $class = shift;"),
            "signature-style constructor should not fall back to shift style: {}",
            first.insert_text
        );
        Ok(())
    }

    #[test]
    fn test_normalize_items_orders_deduplicates_and_limits() {
        let provider = InlineCompletionProvider::new();
        let items = vec![
            RankedCompletionItem {
                score: InlineCandidateScore::from_legacy_priority(2),
                order: 0,
                metadata: InlineCandidateMetadata::test_fixture(),
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
                metadata: InlineCandidateMetadata::test_fixture(),
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
                metadata: InlineCandidateMetadata::test_fixture(),
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
                metadata: InlineCandidateMetadata::test_fixture(),
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
                metadata: InlineCandidateMetadata::test_fixture(),
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
                metadata: InlineCandidateMetadata::test_fixture(),
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
                metadata: InlineCandidateMetadata::test_fixture(),
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
    fn test_normalize_items_uses_metadata_tiebreak_after_score_and_sequence()
    -> Result<(), Box<dyn std::error::Error>> {
        let provider = InlineCompletionProvider::new();
        let items = vec![
            RankedCompletionItem {
                score: InlineCandidateScore::from_legacy_priority(0),
                order: 0,
                metadata: InlineCandidateMetadata {
                    source: InlineCandidateSourceKind::ContextualFallback,
                    reason: InlineCandidateReason::SourceContextualFallback,
                    confidence: InlineCandidateConfidence::Low,
                },
                item: InlineCompletionItem {
                    insert_text: "fallback".into(),
                    filter_text: None,
                    range: None,
                    command: None,
                },
            },
            RankedCompletionItem {
                score: InlineCandidateScore::from_legacy_priority(0),
                order: 0,
                metadata: InlineCandidateMetadata {
                    source: InlineCandidateSourceKind::Receiver,
                    reason: InlineCandidateReason::CurrentPackageMethod,
                    confidence: InlineCandidateConfidence::High,
                },
                item: InlineCompletionItem {
                    insert_text: "save()".into(),
                    filter_text: None,
                    range: None,
                    command: None,
                },
            },
        ];

        let normalized = provider.normalize_items(items);

        assert_eq!(normalized[0].insert_text, "save()");
        assert_eq!(normalized[1].insert_text, "fallback");

        Ok(())
    }

    #[test]
    fn test_normalize_items_prefers_semantic_score_before_sequence() {
        let provider = InlineCompletionProvider::new();
        let items = vec![
            RankedCompletionItem {
                score: InlineCandidateScore::from_legacy_priority(0),
                order: 0,
                metadata: InlineCandidateMetadata::test_fixture(),
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
                metadata: InlineCandidateMetadata::test_fixture(),
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
