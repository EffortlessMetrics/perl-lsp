//! Native critic rule contract.
//!
//! These types define the Rust-native policy diagnostic surface that future
//! rules should target. They intentionally live beside the existing
//! subprocess-backed Perl::Critic adapter and built-in fallback so callers can
//! migrate rule-by-rule without changing runtime behavior in one large step.

use super::{CriticConfig, Severity, Violation, insertion_range};
use perl_parser_core::Node;
use perl_parser_core::NodeKind;
use perl_parser_core::position::{Position, Range};
use perl_pragma::PragmaTracker;
use perl_semantic_analyzer::scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue};
use serde::{Deserialize, Serialize};

/// Native critic rule category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriticCategory {
    /// Syntax-level policy that can be checked from tokens or AST shape.
    Syntax,
    /// Semantic policy that needs bindings, scopes, or inferred facts.
    Semantic,
    /// Workspace-aware policy that needs cross-file/module facts.
    Workspace,
    /// Style convention policy.
    Style,
    /// Maintainability or complexity policy.
    Maintainability,
    /// Security or unsafe-code policy.
    Security,
    /// Documentation or POD policy.
    Documentation,
}

/// Safety level for an optional native critic fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixSafety {
    /// Safe to apply automatically.
    Safe,
    /// Useful suggestion, but should require user confirmation.
    Suggested,
    /// Diagnostic-only guidance with no automatic edit.
    ManualOnly,
}

/// Text edit attached to a native critic fix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticTextEdit {
    /// Range to replace.
    pub range: Range,
    /// Replacement text.
    pub new_text: String,
}

/// Optional fix attached to a native critic finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticFix {
    /// Human-readable action title.
    pub title: String,
    /// How safe the edit is.
    pub safety: FixSafety,
    /// Edits needed to apply the fix.
    pub edits: Vec<CriticTextEdit>,
}

/// Related span for richer native critic diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticRelatedInformation {
    /// Related source range.
    pub range: Range,
    /// Explanation for the related span.
    pub message: String,
}

/// Native critic finding emitted by a rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticFinding {
    /// Stable native rule ID, such as `native.variables.unused_lexical`.
    pub rule_id: String,
    /// Rule category.
    pub category: CriticCategory,
    /// Finding severity.
    pub severity: Severity,
    /// Precise source span for the finding.
    pub range: Range,
    /// Short diagnostic message.
    pub message: String,
    /// Longer explanation shown in editor/CI details.
    pub explanation: String,
    /// Suppression key accepted by inline or file-level suppression handling.
    pub suppression_key: String,
    /// Related spans, when useful.
    pub related: Vec<CriticRelatedInformation>,
    /// Optional fix.
    pub fix: Option<CriticFix>,
}

impl CriticFinding {
    /// Convert this native finding into the existing violation shape.
    ///
    /// This is the bridge that lets native rules flow through current LSP,
    /// execute-command, and diagnostic consumers while those consumers still
    /// expect `Violation` values.
    #[must_use]
    pub fn to_violation(&self, file: impl Into<String>) -> Violation {
        Violation {
            policy: self.rule_id.clone(),
            description: self.message.clone(),
            explanation: self.explanation.clone(),
            severity: self.severity,
            range: self.range,
            file: file.into(),
        }
    }
}

/// Scope covered by a native critic suppression directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriticSuppressionScope {
    /// Suppression applies to the whole file.
    File,
}

/// Parsed native critic suppression directive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticSuppression {
    /// Suppressed rule ID.
    pub rule_id: String,
    /// Scope covered by this directive.
    pub scope: CriticSuppressionScope,
    /// Zero-based line where the directive appears.
    pub line: usize,
    /// Optional human reason after `--`.
    pub reason: Option<String>,
}

/// Parsed native critic suppressions for a source file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticSuppressionMap {
    suppressions: Vec<CriticSuppression>,
}

impl CriticSuppressionMap {
    /// Parse native critic suppression directives from source text.
    #[must_use]
    pub fn from_source(source: &str) -> Self {
        let suppressions = source
            .lines()
            .enumerate()
            .flat_map(|(line, text)| parse_suppression_line(line, text))
            .collect();

        Self { suppressions }
    }

    /// Parsed suppression records.
    #[must_use]
    pub fn suppressions(&self) -> &[CriticSuppression] {
        &self.suppressions
    }

    /// Whether this map suppresses a native critic finding.
    #[must_use]
    pub fn suppresses(&self, finding: &CriticFinding) -> bool {
        self.suppressions.iter().any(|suppression| {
            suppression.rule_id == finding.rule_id || suppression.rule_id == finding.suppression_key
        })
    }
}

/// Native critic rule context.
pub struct CriticContext<'a> {
    /// Source text being analyzed.
    pub source: &'a str,
    /// Parsed AST for the source.
    pub ast: &'a Node,
    /// Critic configuration.
    pub config: &'a CriticConfig,
}

impl<'a> CriticContext<'a> {
    /// Build a native critic context.
    pub fn new(source: &'a str, ast: &'a Node, config: &'a CriticConfig) -> Self {
        Self { source, ast, config }
    }
}

/// Native critic rule interface.
pub trait CriticRule: Send + Sync {
    /// Stable native rule ID.
    fn id(&self) -> &'static str;

    /// Rule category.
    fn category(&self) -> CriticCategory;

    /// Default severity before profile/config remapping.
    fn default_severity(&self) -> Severity;

    /// Check the source and append findings.
    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>);
}

/// Registry for Rust-native critic rules.
///
/// The registry is intentionally small orchestration: it owns rule instances,
/// runs them against a shared context, and returns their findings in registry
/// order. Runtime diagnostic wiring can build on this without each caller
/// needing to know how native rules are stored or executed.
#[derive(Default)]
pub struct NativeCriticRegistry {
    rules: Vec<Box<dyn CriticRule>>,
}

impl NativeCriticRegistry {
    /// Create an empty native critic registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry from prebuilt rules.
    #[must_use]
    pub fn with_rules(rules: Vec<Box<dyn CriticRule>>) -> Self {
        Self { rules }
    }

    /// Create the default recommended native critic registry.
    ///
    /// This is the opt-in bundle for callers migrating from ad hoc built-in
    /// policy execution to the native rule contract. Keep ordering stable so
    /// diagnostics and receipts are deterministic.
    #[must_use]
    pub fn recommended() -> Self {
        Self::with_rules(vec![
            Box::new(RequireUseStrictRule),
            Box::new(RequireUseWarningsRule),
            Box::new(AssignmentInConditionRule),
            Box::new(PrintfFormatArityRule),
            Box::new(DeprecatedDefinedRule),
            Box::new(UndefComparisonRule),
            Box::new(StaleDollarAtRule),
            Box::new(BarewordFilehandleRule),
            Box::new(TwoArgOpenRule),
            Box::new(PipeOpenRule),
            Box::new(UncheckedOpenCloseRule),
            Box::new(QxReadpipeRule),
            Box::new(BacktickExecRule),
            Box::new(StringEvalRule),
            Box::new(SystemExecRule),
            Box::new(UnusedLexicalVariableRule),
            Box::new(UnusedParameterRule),
            Box::new(DuplicateParameterRule),
            Box::new(ParameterShadowsGlobalRule),
            Box::new(DuplicateLexicalDeclarationRule),
            Box::new(ShadowedLexicalVariableRule),
        ])
    }

    /// Add a rule to the registry.
    pub fn add_rule(&mut self, rule: Box<dyn CriticRule>) {
        self.rules.push(rule);
    }

    /// Number of rules in the registry.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Whether the registry has no rules.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Stable IDs for registered rules, in execution order.
    #[must_use]
    pub fn rule_ids(&self) -> Vec<&'static str> {
        self.rules.iter().map(|rule| rule.id()).collect()
    }

    /// Run all registered rules and return collected findings.
    #[must_use]
    pub fn check(&self, ctx: &CriticContext<'_>) -> Vec<CriticFinding> {
        let mut findings = Vec::new();

        for rule in &self.rules {
            if !rule_enabled(rule.as_ref(), ctx.config) {
                continue;
            }
            rule.check(ctx, &mut findings);
        }

        let suppressions = CriticSuppressionMap::from_source(ctx.source);
        findings
            .into_iter()
            .filter(|finding| severity_enabled(finding.severity, ctx.config))
            .filter(|finding| !suppressions.suppresses(finding))
            .collect()
    }

    /// Run all registered rules and return current legacy violation values.
    ///
    /// This keeps native rule execution single-sourced while callers migrate
    /// from `Violation` consumers to richer native finding/code-action data.
    #[must_use]
    pub fn check_violations(
        &self,
        ctx: &CriticContext<'_>,
        file: impl Into<String>,
    ) -> Vec<Violation> {
        let file = file.into();
        self.check(ctx).into_iter().map(|finding| finding.to_violation(file.clone())).collect()
    }
}

fn rule_enabled(rule: &dyn CriticRule, config: &CriticConfig) -> bool {
    let id = rule.id();
    let included = config.include.is_empty() || config.include.iter().any(|policy| policy == id);
    let excluded = config.exclude.iter().any(|policy| policy == id);

    included && !excluded
}

fn severity_enabled(severity: Severity, config: &CriticConfig) -> bool {
    severity as u8 >= config.severity
}

fn parse_suppression_line(line: usize, text: &str) -> Vec<CriticSuppression> {
    const NO_CRITIC: &str = "## no critic ";
    const NO_NATIVE_CRITIC: &str = "## no perl-lsp-critic ";

    let trimmed = text.trim_start();
    let Some(rest) =
        trimmed.strip_prefix(NO_CRITIC).or_else(|| trimmed.strip_prefix(NO_NATIVE_CRITIC))
    else {
        return Vec::new();
    };

    let (rules, reason) = rest.split_once("--").map_or((rest, None), |(rules, reason)| {
        let reason = reason.trim();
        (rules, (!reason.is_empty()).then(|| reason.to_string()))
    });

    rules
        .split(|ch: char| ch == ',' || ch.is_whitespace())
        .filter_map(|rule_id| {
            let rule_id = rule_id.trim();
            (!rule_id.is_empty()).then(|| CriticSuppression {
                rule_id: rule_id.to_string(),
                scope: CriticSuppressionScope::File,
                line,
                reason: reason.clone(),
            })
        })
        .collect()
}

/// Native rule that requires a file-level `use strict;` pragma.
///
/// This is the first built-in rule expressed through the native critic
/// contract. It deliberately does not replace the existing legacy built-in
/// analyzer yet; callers can opt into it through `NativeCriticRegistry` while
/// runtime diagnostic migration remains incremental.
pub struct RequireUseStrictRule;

impl CriticRule for RequireUseStrictRule {
    fn id(&self) -> &'static str {
        "native.testing.require_use_strict"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Syntax
    }

    fn default_severity(&self) -> Severity {
        Severity::Harsh
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        if has_use_statement(ctx.source, "strict") {
            return;
        }

        let range = insertion_range();
        out.push(CriticFinding {
            rule_id: self.id().to_string(),
            category: self.category(),
            severity: self.default_severity(),
            range,
            message: "Code does not use strict".to_string(),
            explanation: "Always use strict to catch common mistakes".to_string(),
            suppression_key: self.id().to_string(),
            related: Vec::new(),
            fix: Some(CriticFix {
                title: "Add 'use strict'".to_string(),
                safety: FixSafety::Safe,
                edits: vec![CriticTextEdit { range, new_text: "use strict;\n".to_string() }],
            }),
        });
    }
}

/// Native rule that requires a file-level `use warnings;` pragma.
///
/// Like [`RequireUseStrictRule`], this is exposed through the native critic
/// contract without replacing the existing legacy built-in analyzer yet.
pub struct RequireUseWarningsRule;

impl CriticRule for RequireUseWarningsRule {
    fn id(&self) -> &'static str {
        "native.testing.require_use_warnings"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Syntax
    }

    fn default_severity(&self) -> Severity {
        Severity::Harsh
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        if has_use_statement(ctx.source, "warnings") {
            return;
        }

        let range = insertion_range();
        out.push(CriticFinding {
            rule_id: self.id().to_string(),
            category: self.category(),
            severity: self.default_severity(),
            range,
            message: "Code does not use warnings".to_string(),
            explanation: "Always use warnings to catch potential issues".to_string(),
            suppression_key: self.id().to_string(),
            related: Vec::new(),
            fix: Some(CriticFix {
                title: "Add 'use warnings'".to_string(),
                safety: FixSafety::Safe,
                edits: vec![CriticTextEdit { range, new_text: "use warnings;\n".to_string() }],
            }),
        });
    }
}

/// Native rule that reports assignments used directly as conditions.
///
/// This mirrors the existing common-mistake diagnostic through the native
/// critic contract so opt-in native critic users get stable rule IDs,
/// suppressions, severity filtering, and fix metadata.
pub struct AssignmentInConditionRule;

impl CriticRule for AssignmentInConditionRule {
    fn id(&self) -> &'static str {
        "native.common.assignment_in_condition"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Syntax
    }

    fn default_severity(&self) -> Severity {
        Severity::Stern
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        collect_assignment_in_condition_findings(self, ctx.source, ctx.ast, out);
    }
}

/// Native rule that reports static printf/sprintf format arity mismatches.
///
/// This wraps the existing parser-backed PL405 lint through the native critic
/// contract so opt-in native critic users get stable rule IDs, suppression and
/// severity filtering, violation bridge coverage, and LSP parity.
pub struct PrintfFormatArityRule;

impl CriticRule for PrintfFormatArityRule {
    fn id(&self) -> &'static str {
        "native.common.printf_format_arity"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Syntax
    }

    fn default_severity(&self) -> Severity {
        Severity::Stern
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        collect_printf_format_arity_findings(self, ctx.source, ctx.ast, out);
    }
}

/// Native rule that reports deprecated `defined @array` / `defined %hash` use.
///
/// This wraps the existing PL500 deprecated-syntax lint through the native
/// critic contract so opt-in native critic users get stable rule IDs,
/// suppressions, severity filtering, violation bridge coverage, and quick-fix
/// metadata.
pub struct DeprecatedDefinedRule;

impl CriticRule for DeprecatedDefinedRule {
    fn id(&self) -> &'static str {
        "native.common.deprecated_defined"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Syntax
    }

    fn default_severity(&self) -> Severity {
        Severity::Stern
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        collect_deprecated_defined_findings(self, ctx.source, ctx.ast, out);
    }
}

/// Native rule that reports numeric comparison with explicit `undef`.
///
/// This wraps the parser-confirmed PL404 surface through the native critic
/// contract. It intentionally starts with literal `undef` comparisons; broader
/// maybe-undef semantic inference can be added as a separate rule slice.
pub struct UndefComparisonRule;

impl CriticRule for UndefComparisonRule {
    fn id(&self) -> &'static str {
        "native.common.undef_comparison"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Syntax
    }

    fn default_severity(&self) -> Severity {
        Severity::Stern
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        collect_undef_comparison_findings(self, ctx.source, ctx.ast, out);
    }
}

/// Native rule that reports unlocalized `$@` checks after block `eval`.
///
/// Checking `$@` after `eval` without localizing it can observe a stale or
/// clobbered error value. This rule starts with the parser-confirmed
/// `eval { ... }; if ($@) { ... }` family and leaves broader exception-flow
/// analysis for later semantic rule slices.
pub struct StaleDollarAtRule;

impl CriticRule for StaleDollarAtRule {
    fn id(&self) -> &'static str {
        "native.common.stale_dollar_at"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Syntax
    }

    fn default_severity(&self) -> Severity {
        Severity::Stern
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        collect_stale_dollar_at_findings(self, ctx.source, ctx.ast, out);
    }
}

/// Native rule that reports bareword filehandles passed to `open`.
///
/// This mirrors the existing common-mistake and Perl::Critic policy surfaces
/// while giving opt-in native critic users a stable rule ID, precise handle
/// span, suppression key, and fix metadata.
pub struct BarewordFilehandleRule;

impl CriticRule for BarewordFilehandleRule {
    fn id(&self) -> &'static str {
        "native.io.bareword_filehandle"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Syntax
    }

    fn default_severity(&self) -> Severity {
        Severity::Stern
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        collect_bareword_filehandle_findings(self, ctx.source, ctx.ast, out);
    }
}

/// Native rule that reports two-argument `open` calls.
///
/// The rule keeps the current native critic migration pattern: stable native
/// policy ID, suppression/config filtering, violation bridge support, and
/// quick-fix metadata while legacy/default critic behavior remains unchanged.
pub struct TwoArgOpenRule;

impl CriticRule for TwoArgOpenRule {
    fn id(&self) -> &'static str {
        "native.io.two_arg_open"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Security
    }

    fn default_severity(&self) -> Severity {
        Severity::Harsh
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        collect_two_arg_open_findings(self, ctx.source, ctx.ast, out);
    }
}

/// Native rule that reports pipe-open command execution.
///
/// Pipe-open forms execute shell commands through `open`. The rule mirrors the
/// existing security diagnostic through the native critic contract and remains
/// diagnostic-only because rewriting command execution safely needs user
/// intent.
pub struct PipeOpenRule;

impl CriticRule for PipeOpenRule {
    fn id(&self) -> &'static str {
        "native.io.pipe_open"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Security
    }

    fn default_severity(&self) -> Severity {
        Severity::Harsh
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        collect_pipe_open_findings(self, ctx.source, ctx.ast, out);
    }
}

/// Native rule that reports statement-level `open` and `close` calls whose
/// return value is ignored.
///
/// The rule intentionally starts narrow: it catches bare `open(...);` and
/// `close(...);` statements and accepts common checked idioms such as
/// `open(...) or die ...`.
pub struct UncheckedOpenCloseRule;

impl CriticRule for UncheckedOpenCloseRule {
    fn id(&self) -> &'static str {
        "native.io.unchecked_open_close"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Security
    }

    fn default_severity(&self) -> Severity {
        Severity::Stern
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        collect_unchecked_open_close_findings(self, ctx.source, ctx.ast, out);
    }
}

/// Native rule that reports `qx` and `readpipe` command execution.
///
/// This complements the backtick rule with parser-confirmed quote-command and
/// function-call forms. It stays diagnostic-only because replacing shell
/// command execution safely requires user intent.
pub struct QxReadpipeRule;

impl CriticRule for QxReadpipeRule {
    fn id(&self) -> &'static str {
        "native.security.qx_readpipe"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Security
    }

    fn default_severity(&self) -> Severity {
        Severity::Harsh
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        collect_qx_readpipe_findings(self, ctx.source, ctx.ast, out);
    }
}

/// Native rule that reports backtick command execution.
///
/// This mirrors the existing security diagnostic through the native critic
/// contract. The rule intentionally stays diagnostic-only because replacing
/// shell command execution safely requires user intent.
pub struct BacktickExecRule;

impl CriticRule for BacktickExecRule {
    fn id(&self) -> &'static str {
        "native.security.backtick_exec"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Security
    }

    fn default_severity(&self) -> Severity {
        Severity::Harsh
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        collect_backtick_exec_findings(self, ctx.source, ctx.ast, out);
    }
}

/// Native rule that reports string-based `eval`.
///
/// This mirrors the existing security diagnostic and built-in policy through
/// the native critic contract. The rule intentionally does not attach an
/// automatic edit: replacing string eval safely requires user intent.
pub struct StringEvalRule;

impl CriticRule for StringEvalRule {
    fn id(&self) -> &'static str {
        "native.security.string_eval"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Security
    }

    fn default_severity(&self) -> Severity {
        Severity::Harsh
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        collect_string_eval_findings(self, ctx.source, ctx.ast, out);
    }
}

/// Native rule that reports `system` and `exec` command execution.
///
/// This mirrors the existing security diagnostics through the native critic
/// contract. The rule stays diagnostic-only because replacing process
/// execution safely requires user intent.
pub struct SystemExecRule;

impl CriticRule for SystemExecRule {
    fn id(&self) -> &'static str {
        "native.security.system_exec"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Security
    }

    fn default_severity(&self) -> Severity {
        Severity::Harsh
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        collect_system_exec_findings(self, ctx.source, ctx.ast, out);
    }
}

/// Native rule that reports lexical variables declared but never read.
///
/// This rule delegates scope reasoning to the existing semantic analyzer so the
/// native critic path reuses the same declaration/use facts as core diagnostics.
pub struct UnusedLexicalVariableRule;

impl CriticRule for UnusedLexicalVariableRule {
    fn id(&self) -> &'static str {
        "native.variables.unused_lexical"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Semantic
    }

    fn default_severity(&self) -> Severity {
        Severity::Stern
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        let pragma_map = PragmaTracker::build(ctx.ast);
        let issues = ScopeAnalyzer::new().analyze(ctx.ast, ctx.source, &pragma_map);

        out.extend(
            issues
                .into_iter()
                .filter(|issue| issue.kind == IssueKind::UnusedVariable)
                .map(|issue| unused_lexical_finding(self, ctx.source, &issue)),
        );
    }
}

/// Native rule that reports subroutine parameters declared but never read.
///
/// This rule delegates parameter-use reasoning to the semantic scope analyzer
/// so native critic diagnostics reuse the same signature facts as existing
/// PL108 diagnostics.
pub struct UnusedParameterRule;

impl CriticRule for UnusedParameterRule {
    fn id(&self) -> &'static str {
        "native.variables.unused_parameter"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Semantic
    }

    fn default_severity(&self) -> Severity {
        Severity::Stern
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        let pragma_map = PragmaTracker::build(ctx.ast);
        let issues = ScopeAnalyzer::new().analyze(ctx.ast, ctx.source, &pragma_map);

        out.extend(
            issues
                .into_iter()
                .filter(|issue| issue.kind == IssueKind::UnusedParameter)
                .map(|issue| unused_parameter_finding(self, ctx.source, &issue)),
        );
    }
}

/// Native rule that reports subroutine parameters repeated in one signature.
///
/// This rule delegates duplicate-parameter detection to the semantic scope
/// analyzer so native critic diagnostics reuse existing signature facts while
/// exposing a stable native policy ID.
pub struct DuplicateParameterRule;

impl CriticRule for DuplicateParameterRule {
    fn id(&self) -> &'static str {
        "native.variables.duplicate_parameter"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Semantic
    }

    fn default_severity(&self) -> Severity {
        Severity::Gentle
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        let pragma_map = PragmaTracker::build(ctx.ast);
        let issues = ScopeAnalyzer::new().analyze(ctx.ast, ctx.source, &pragma_map);

        out.extend(
            issues
                .into_iter()
                .filter(|issue| issue.kind == IssueKind::DuplicateParameter)
                .map(|issue| duplicate_parameter_finding(self, ctx.source, &issue)),
        );
    }
}

/// Native rule that reports subroutine parameters shadowing outer variables.
///
/// This rule delegates parameter shadowing detection to the semantic scope
/// analyzer so native critic diagnostics reuse existing binding facts while
/// exposing a stable native policy ID.
pub struct ParameterShadowsGlobalRule;

impl CriticRule for ParameterShadowsGlobalRule {
    fn id(&self) -> &'static str {
        "native.variables.parameter_shadows_global"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Semantic
    }

    fn default_severity(&self) -> Severity {
        Severity::Stern
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        let pragma_map = PragmaTracker::build(ctx.ast);
        let issues = ScopeAnalyzer::new().analyze(ctx.ast, ctx.source, &pragma_map);

        out.extend(
            issues
                .into_iter()
                .filter(|issue| issue.kind == IssueKind::ParameterShadowsGlobal)
                .map(|issue| parameter_shadows_global_finding(self, ctx.source, &issue)),
        );
    }
}

/// Native rule that reports lexical variables declared more than once in a scope.
///
/// This rule delegates redeclaration detection to the semantic scope analyzer so
/// native critic diagnostics reuse the same binding facts as existing PL105
/// diagnostics while exposing a stable native policy ID.
pub struct DuplicateLexicalDeclarationRule;

impl CriticRule for DuplicateLexicalDeclarationRule {
    fn id(&self) -> &'static str {
        "native.variables.duplicate_lexical"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Semantic
    }

    fn default_severity(&self) -> Severity {
        Severity::Gentle
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        let pragma_map = PragmaTracker::build(ctx.ast);
        let issues = ScopeAnalyzer::new().analyze(ctx.ast, ctx.source, &pragma_map);

        out.extend(
            issues
                .into_iter()
                .filter(|issue| issue.kind == IssueKind::VariableRedeclaration)
                .map(|issue| duplicate_lexical_finding(self, ctx.source, &issue)),
        );
    }
}

/// Native rule that reports lexical variables that shadow outer declarations.
///
/// This rule delegates shadowing detection to the semantic scope analyzer so
/// native critic diagnostics reuse existing scope facts while exposing a stable
/// native policy ID.
pub struct ShadowedLexicalVariableRule;

impl CriticRule for ShadowedLexicalVariableRule {
    fn id(&self) -> &'static str {
        "native.variables.shadowed_lexical"
    }

    fn category(&self) -> CriticCategory {
        CriticCategory::Semantic
    }

    fn default_severity(&self) -> Severity {
        Severity::Stern
    }

    fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
        let pragma_map = PragmaTracker::build(ctx.ast);
        let issues = ScopeAnalyzer::new().analyze(ctx.ast, ctx.source, &pragma_map);

        out.extend(
            issues
                .into_iter()
                .filter(|issue| issue.kind == IssueKind::VariableShadowing)
                .map(|issue| shadowed_lexical_finding(self, ctx.source, &issue)),
        );
    }
}

fn unused_lexical_finding(
    rule: &UnusedLexicalVariableRule,
    source: &str,
    issue: &ScopeIssue,
) -> CriticFinding {
    let range = range_for_byte_span(source, issue.range.0, issue.range.1);
    let unused_name = prefixed_unused_name(&issue.variable_name);

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: format!("Lexical variable '{}' is declared but never used", issue.variable_name),
        explanation: "Remove the lexical variable, use it, or prefix it with '_' to mark it intentionally unused.".to_string(),
        suppression_key: rule.id().to_string(),
        related: Vec::new(),
        fix: Some(CriticFix {
            title: format!("Rename to '{unused_name}'"),
            safety: FixSafety::Suggested,
            edits: vec![CriticTextEdit { range, new_text: unused_name }],
        }),
    }
}

fn unused_parameter_finding(
    rule: &UnusedParameterRule,
    source: &str,
    issue: &ScopeIssue,
) -> CriticFinding {
    let range = range_for_byte_span(source, issue.range.0, issue.range.1);
    let unused_name = prefixed_unused_name(&issue.variable_name);

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: format!("Parameter '{}' is never used", issue.variable_name),
        explanation: "Use the parameter or prefix it with '_' to mark it intentionally unused."
            .to_string(),
        suppression_key: rule.id().to_string(),
        related: Vec::new(),
        fix: Some(CriticFix {
            title: format!("Rename to '{unused_name}'"),
            safety: FixSafety::Suggested,
            edits: vec![CriticTextEdit { range, new_text: unused_name }],
        }),
    }
}

fn duplicate_parameter_finding(
    rule: &DuplicateParameterRule,
    source: &str,
    issue: &ScopeIssue,
) -> CriticFinding {
    let range = range_for_byte_span(source, issue.range.0, issue.range.1);
    let replacement = numbered_duplicate_name(&issue.variable_name);

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: format!(
            "Parameter '{}' appears more than once in this signature",
            issue.variable_name
        ),
        explanation:
            "Remove the duplicate parameter or rename it so every signature binding is unique."
                .to_string(),
        suppression_key: rule.id().to_string(),
        related: Vec::new(),
        fix: Some(CriticFix {
            title: format!("Rename duplicate parameter to '{replacement}'"),
            safety: FixSafety::Suggested,
            edits: vec![CriticTextEdit { range, new_text: replacement }],
        }),
    }
}

fn parameter_shadows_global_finding(
    rule: &ParameterShadowsGlobalRule,
    source: &str,
    issue: &ScopeIssue,
) -> CriticFinding {
    let range = range_for_byte_span(source, issue.range.0, issue.range.1);
    let replacement = parameter_shadow_name(&issue.variable_name);

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: format!("Parameter '{}' shadows an outer declaration", issue.variable_name),
        explanation: "Rename the parameter or use the outer variable directly to avoid confusing scope shadowing.".to_string(),
        suppression_key: rule.id().to_string(),
        related: Vec::new(),
        fix: Some(CriticFix {
            title: format!("Rename parameter to '{replacement}'"),
            safety: FixSafety::Suggested,
            edits: vec![CriticTextEdit { range, new_text: replacement }],
        }),
    }
}

fn assignment_in_condition_finding(
    rule: &AssignmentInConditionRule,
    source: &str,
    condition: &Node,
) -> CriticFinding {
    let range = range_for_byte_span(source, condition.location.start, condition.location.end);
    let fix = assignment_comparison_fix(source, condition.location.start, condition.location.end);

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: "Assignment in condition - did you mean '=='?".to_string(),
        explanation: "Assignments in conditions are usually accidental. Use '==' for numeric comparison, 'eq' for string comparison, or add parentheses if the assignment is intentional.".to_string(),
        suppression_key: rule.id().to_string(),
        related: vec![
            CriticRelatedInformation {
                range,
                message: "Use '==' for numeric comparison or 'eq' for string comparison.".to_string(),
            },
            CriticRelatedInformation {
                range,
                message: "If the assignment is intentional, wrap it in parentheses.".to_string(),
            },
        ],
        fix,
    }
}

fn printf_format_arity_finding(
    rule: &PrintfFormatArityRule,
    source: &str,
    call_node: &Node,
    format_node: &Node,
    call_name: &str,
    specifier_count: usize,
    arg_count: usize,
) -> CriticFinding {
    let range = range_for_byte_span(source, call_node.location.start, call_node.location.end);
    let format_range =
        range_for_byte_span(source, format_node.location.start, format_node.location.end);

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: format!(
            "`{}` format string has {} specifier{} but {} argument{} supplied",
            call_name,
            specifier_count,
            if specifier_count == 1 { "" } else { "s" },
            arg_count,
            if arg_count == 1 { "" } else { "s" },
        ),
        explanation: format!(
            "Add {} argument{} to match {} format specifier{}, or adjust the format string",
            specifier_count,
            if specifier_count == 1 { "" } else { "s" },
            specifier_count,
            if specifier_count == 1 { "" } else { "s" },
        ),
        suppression_key: rule.id().to_string(),
        related: vec![CriticRelatedInformation {
            range: format_range,
            message: format!(
                "Format string contains {} specifier{}",
                specifier_count,
                if specifier_count == 1 { "" } else { "s" }
            ),
        }],
        fix: None,
    }
}

fn deprecated_defined_finding(
    rule: &DeprecatedDefinedRule,
    source: &str,
    call_node: &Node,
    arg_node: &Node,
    sigil: &str,
    name: &str,
) -> CriticFinding {
    let range = range_for_byte_span(source, call_node.location.start, call_node.location.end);
    let arg_range = range_for_byte_span(source, arg_node.location.start, arg_node.location.end);
    let variable_text = format!("{sigil}{name}");
    let type_name = if sigil == "@" { "array" } else { "hash" };

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: format!("Use of 'defined {variable_text}' is deprecated"),
        explanation: format!(
            "Testing definedness of a whole {type_name} is deprecated because it was rarely useful and often wrong. Use the {type_name} in boolean context instead."
        ),
        suppression_key: rule.id().to_string(),
        related: vec![CriticRelatedInformation {
            range: arg_range,
            message: format!("Use 'if ({variable_text})' instead"),
        }],
        fix: Some(CriticFix {
            title: format!("Replace with '{variable_text}'"),
            safety: FixSafety::Suggested,
            edits: vec![CriticTextEdit { range, new_text: variable_text }],
        }),
    }
}

fn undef_comparison_finding(
    rule: &UndefComparisonRule,
    source: &str,
    comparison_node: &Node,
    op: &str,
    compared_node: &Node,
    undef_node: &Node,
) -> CriticFinding {
    let range =
        range_for_byte_span(source, comparison_node.location.start, comparison_node.location.end);
    let compared_range =
        range_for_byte_span(source, compared_node.location.start, compared_node.location.end);
    let undef_range =
        range_for_byte_span(source, undef_node.location.start, undef_node.location.end);
    let compared_text =
        source[compared_node.location.start..compared_node.location.end].trim().to_string();
    let replacement = match op {
        "==" => format!("!defined({compared_text})"),
        "!=" => format!("defined({compared_text})"),
        _ => compared_text,
    };

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: format!("Using '{op}' with undef -- use defined() to check first"),
        explanation: "Numeric comparison with undef is usually wrong because undef is coerced before comparison. Use defined(...) for definedness checks, or normalize the value before comparing.".to_string(),
        suppression_key: rule.id().to_string(),
        related: vec![
            CriticRelatedInformation {
                range: undef_range,
                message: "This undef literal should be checked with defined(...) instead.".to_string(),
            },
            CriticRelatedInformation {
                range: compared_range,
                message: "Check this expression with defined(...) before comparing values.".to_string(),
            },
        ],
        fix: Some(CriticFix {
            title: "Use defined() check".to_string(),
            safety: FixSafety::Suggested,
            edits: vec![CriticTextEdit { range, new_text: replacement }],
        }),
    }
}

fn stale_dollar_at_finding(
    rule: &StaleDollarAtRule,
    source: &str,
    eval_node: &Node,
    dollar_at_node: &Node,
) -> CriticFinding {
    let range =
        range_for_byte_span(source, dollar_at_node.location.start, dollar_at_node.location.end);
    let eval_range = range_for_byte_span(source, eval_node.location.start, eval_node.location.end);

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: "Checking $@ after eval can observe a stale error".to_string(),
        explanation: "The $@ variable is global and can retain or be clobbered by unrelated exception handling. Localize $@ around eval, or check the eval return value before inspecting the error.".to_string(),
        suppression_key: rule.id().to_string(),
        related: vec![CriticRelatedInformation {
            range: eval_range,
            message: "This eval should localize $@ or have its return value checked.".to_string(),
        }],
        fix: None,
    }
}

fn bareword_filehandle_finding(
    rule: &BarewordFilehandleRule,
    source: &str,
    handle: &Node,
    handle_name: &str,
) -> CriticFinding {
    let range = range_for_byte_span(source, handle.location.start, handle.location.end);
    let lexical_name = bareword_filehandle_lexical_name(handle_name);

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: format!("Bareword filehandle '{handle_name}' should be lexical"),
        explanation: "Bareword filehandles are package globals and can be accidentally reused across scopes. Use lexical filehandles for safer IO.".to_string(),
        suppression_key: rule.id().to_string(),
        related: Vec::new(),
        fix: Some(CriticFix {
            title: format!(
                "Replace bareword filehandle '{handle_name}' with lexical '{lexical_name}'"
            ),
            safety: FixSafety::Suggested,
            edits: vec![CriticTextEdit {
                range,
                new_text: format!("my {lexical_name}"),
            }],
        }),
    }
}

fn two_arg_open_finding(
    rule: &TwoArgOpenRule,
    source: &str,
    call: &Node,
    open_args: &[Node],
) -> CriticFinding {
    let range = range_for_byte_span(source, call.location.start, call.location.end);
    let fix = two_arg_open_fix_text(source, open_args).map(|new_text| CriticFix {
        title: "Convert to three-argument open() for safety".to_string(),
        safety: FixSafety::Suggested,
        edits: vec![CriticTextEdit { range, new_text }],
    });

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: "Two-argument open should use an explicit mode".to_string(),
        explanation: "Two-argument open combines mode and filename, which can allow shell interpretation when the filename is derived from input. Use three-argument open with a separate mode.".to_string(),
        suppression_key: rule.id().to_string(),
        related: Vec::new(),
        fix,
    }
}

fn two_arg_open_fix_text(source: &str, open_args: &[Node]) -> Option<String> {
    let [handle, path] = open_args else {
        return None;
    };

    let handle_text = source.get(handle.location.start..handle.location.end)?.trim();
    let path_text = source.get(path.location.start..path.location.end)?.trim();

    if handle_text.is_empty() || path_text.is_empty() {
        return None;
    }

    Some(format!("open({handle_text}, '<', {path_text})"))
}

fn pipe_open_finding(rule: &PipeOpenRule, source: &str, open_node: &Node) -> CriticFinding {
    let range = range_for_byte_span(source, open_node.location.start, open_node.location.end);

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: "Pipe-open executes a shell command".to_string(),
        explanation: "Pipe-open forms run a command through the shell. Prefer explicit command argument lists or IPC modules when command execution is required.".to_string(),
        suppression_key: rule.id().to_string(),
        related: vec![CriticRelatedInformation {
            range,
            message: "Validate command arguments and avoid string command execution when input may be user-controlled.".to_string(),
        }],
        fix: None,
    }
}

fn unchecked_open_close_finding(
    rule: &UncheckedOpenCloseRule,
    source: &str,
    call_node: &Node,
    name: &str,
) -> CriticFinding {
    let range = range_for_byte_span(source, call_node.location.start, call_node.location.end);
    let message = match name {
        "close" => "close() return value should be checked",
        _ => "open() return value should be checked",
    };

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: message.to_string(),
        explanation: "open and close report I/O failures through their return value. Check the result with an explicit error path such as `or die` so failures are not silently ignored.".to_string(),
        suppression_key: rule.id().to_string(),
        related: vec![CriticRelatedInformation {
            range,
            message: "Unchecked I/O calls can hide missing files, permission failures, and failed flushes.".to_string(),
        }],
        fix: None,
    }
}

fn backtick_exec_finding(
    rule: &BacktickExecRule,
    source: &str,
    string_node: &Node,
) -> CriticFinding {
    let range = range_for_byte_span(source, string_node.location.start, string_node.location.end);

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: "Command execution detected".to_string(),
        explanation: "Backticks execute shell commands. Prefer explicit command argument lists or IPC modules when command execution is required.".to_string(),
        suppression_key: rule.id().to_string(),
        related: vec![CriticRelatedInformation {
            range,
            message: "Validate command arguments and avoid shell command execution when input may be user-controlled.".to_string(),
        }],
        fix: None,
    }
}

fn qx_readpipe_finding(rule: &QxReadpipeRule, source: &str, command_node: &Node) -> CriticFinding {
    let range = range_for_byte_span(source, command_node.location.start, command_node.location.end);

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: "qx/readpipe command execution detected".to_string(),
        explanation: "qx and readpipe execute shell commands. Prefer explicit command argument lists or IPC modules when command execution is required.".to_string(),
        suppression_key: rule.id().to_string(),
        related: vec![CriticRelatedInformation {
            range,
            message: "Validate command arguments and avoid shell command execution when input may be user-controlled.".to_string(),
        }],
        fix: None,
    }
}

fn string_eval_finding(rule: &StringEvalRule, source: &str, eval_node: &Node) -> CriticFinding {
    let range = range_for_byte_span(source, eval_node.location.start, eval_node.location.end);

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: "String eval is a security risk".to_string(),
        explanation: "String eval executes dynamically generated Perl code and is difficult to analyze safely. Prefer block eval for exception handling or a safer dispatch mechanism.".to_string(),
        suppression_key: rule.id().to_string(),
        related: vec![CriticRelatedInformation {
            range,
            message: "String eval executes arbitrary Perl code at runtime when the string contains user-controlled input.".to_string(),
        }],
        fix: None,
    }
}

fn system_exec_finding(
    rule: &SystemExecRule,
    source: &str,
    call_node: &Node,
    name: &str,
) -> CriticFinding {
    let range = range_for_byte_span(source, call_node.location.start, call_node.location.end);
    let message = match name {
        "exec" => "exec() replaces the current process with a shell command",
        _ => "system() executes a shell command",
    };

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: message.to_string(),
        explanation: "system and exec run external commands. Prefer explicit argument lists or IPC modules when command execution is required, and validate any user-controlled input.".to_string(),
        suppression_key: rule.id().to_string(),
        related: vec![CriticRelatedInformation {
            range,
            message: "List form avoids shell interpolation, but command execution still needs an explicit security review.".to_string(),
        }],
        fix: None,
    }
}

fn duplicate_lexical_finding(
    rule: &DuplicateLexicalDeclarationRule,
    source: &str,
    issue: &ScopeIssue,
) -> CriticFinding {
    let range = range_for_byte_span(source, issue.range.0, issue.range.1);
    let fix = duplicate_my_fix(source, issue.range.0);

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: format!(
            "Lexical variable '{}' is declared more than once in the same scope",
            issue.variable_name
        ),
        explanation:
            "Remove the duplicate lexical declarator or assign to the existing lexical variable."
                .to_string(),
        suppression_key: rule.id().to_string(),
        related: Vec::new(),
        fix,
    }
}

fn shadowed_lexical_finding(
    rule: &ShadowedLexicalVariableRule,
    source: &str,
    issue: &ScopeIssue,
) -> CriticFinding {
    let range = range_for_byte_span(source, issue.range.0, issue.range.1);
    let replacement = shadowed_lexical_name(&issue.variable_name);

    CriticFinding {
        rule_id: rule.id().to_string(),
        category: rule.category(),
        severity: rule.default_severity(),
        range,
        message: format!("Lexical variable '{}' shadows an outer declaration", issue.variable_name),
        explanation: "Rename the inner lexical variable or use the outer variable directly to avoid confusing scope shadowing.".to_string(),
        suppression_key: rule.id().to_string(),
        related: Vec::new(),
        fix: Some(CriticFix {
            title: format!("Rename to '{replacement}'"),
            safety: FixSafety::Suggested,
            edits: vec![CriticTextEdit { range, new_text: replacement }],
        }),
    }
}

fn collect_assignment_in_condition_findings(
    rule: &AssignmentInConditionRule,
    source: &str,
    node: &Node,
    out: &mut Vec<CriticFinding>,
) {
    match &node.kind {
        NodeKind::If { condition, elsif_branches, .. } => {
            push_assignment_condition_finding(rule, source, condition, out);
            for (elsif_condition, _) in elsif_branches {
                push_assignment_condition_finding(rule, source, elsif_condition, out);
            }
        }
        NodeKind::While { condition, .. } => {
            push_assignment_condition_finding(rule, source, condition, out);
        }
        NodeKind::For { condition: Some(condition), .. } => {
            push_assignment_condition_finding(rule, source, condition, out);
        }
        NodeKind::StatementModifier { modifier, condition, .. }
            if matches!(modifier.as_str(), "if" | "unless" | "while" | "until") =>
        {
            push_assignment_condition_finding(rule, source, condition, out);
        }
        _ => {}
    }

    for child in node.children() {
        collect_assignment_in_condition_findings(rule, source, child, out);
    }
}

fn collect_printf_format_arity_findings(
    rule: &PrintfFormatArityRule,
    source: &str,
    node: &Node,
    out: &mut Vec<CriticFinding>,
) {
    match &node.kind {
        NodeKind::FunctionCall { name, args } if name == "printf" || name == "sprintf" => {
            push_printf_format_arity_finding(rule, source, node, name, args, out);
        }
        NodeKind::IndirectCall { method, args, .. } if method == "printf" => {
            push_printf_format_arity_finding(rule, source, node, method, args, out);
        }
        _ => {}
    }

    for child in node.children() {
        collect_printf_format_arity_findings(rule, source, child, out);
    }
}

fn collect_deprecated_defined_findings(
    rule: &DeprecatedDefinedRule,
    source: &str,
    node: &Node,
    out: &mut Vec<CriticFinding>,
) {
    if let NodeKind::FunctionCall { name, args } = &node.kind
        && name == "defined"
    {
        let effective_args = effective_call_args(args);
        if let Some(arg) = effective_args.first()
            && let NodeKind::Variable { sigil, name } = &arg.kind
            && matches!(sigil.as_str(), "@" | "%")
        {
            out.push(deprecated_defined_finding(rule, source, node, arg, sigil, name));
        }
    }

    for child in node.children() {
        collect_deprecated_defined_findings(rule, source, child, out);
    }
}

fn collect_undef_comparison_findings(
    rule: &UndefComparisonRule,
    source: &str,
    node: &Node,
    out: &mut Vec<CriticFinding>,
) {
    if let NodeKind::Binary { op, left, right } = &node.kind
        && matches!(op.as_str(), "==" | "!=")
    {
        match (&left.kind, &right.kind) {
            (NodeKind::Undef, _) => {
                out.push(undef_comparison_finding(rule, source, node, op, right, left));
            }
            (_, NodeKind::Undef) => {
                out.push(undef_comparison_finding(rule, source, node, op, left, right));
            }
            _ => {}
        }
    }

    for child in node.children() {
        collect_undef_comparison_findings(rule, source, child, out);
    }
}

fn collect_stale_dollar_at_findings(
    rule: &StaleDollarAtRule,
    source: &str,
    node: &Node,
    out: &mut Vec<CriticFinding>,
) {
    match &node.kind {
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            collect_stale_dollar_at_in_statements(rule, source, statements, out);
        }
        _ => {}
    }

    for child in node.children() {
        collect_stale_dollar_at_findings(rule, source, child, out);
    }
}

fn collect_stale_dollar_at_in_statements(
    rule: &StaleDollarAtRule,
    source: &str,
    statements: &[Node],
    out: &mut Vec<CriticFinding>,
) {
    for (idx, statement) in statements.iter().enumerate() {
        let Some(eval_node) = eval_statement_node(statement) else {
            continue;
        };
        if idx > 0 && localizes_dollar_at(&statements[idx - 1]) {
            continue;
        }
        let Some(next_statement) = statements.get(idx + 1) else {
            continue;
        };
        if let Some(dollar_at) = condition_dollar_at_read(next_statement) {
            out.push(stale_dollar_at_finding(rule, source, eval_node, dollar_at));
        }
    }
}

fn eval_statement_node(statement: &Node) -> Option<&Node> {
    match &statement.kind {
        NodeKind::ExpressionStatement { expression }
            if matches!(&expression.kind, NodeKind::Eval { .. }) =>
        {
            Some(expression)
        }
        NodeKind::Eval { .. } => Some(statement),
        _ => None,
    }
}

fn localizes_dollar_at(statement: &Node) -> bool {
    match &statement.kind {
        NodeKind::VariableDeclaration { declarator, variable, .. } if declarator == "local" => {
            is_dollar_at_variable(variable)
        }
        NodeKind::ExpressionStatement { expression } => localizes_dollar_at(expression),
        _ => false,
    }
}

fn condition_dollar_at_read(statement: &Node) -> Option<&Node> {
    match &statement.kind {
        NodeKind::If { condition, .. } | NodeKind::While { condition, .. } => {
            first_dollar_at_variable(condition)
        }
        NodeKind::StatementModifier { condition, .. } => first_dollar_at_variable(condition),
        _ => None,
    }
}

fn first_dollar_at_variable(node: &Node) -> Option<&Node> {
    if is_dollar_at_variable(node) {
        return Some(node);
    }

    node.children().into_iter().find_map(first_dollar_at_variable)
}

fn is_dollar_at_variable(node: &Node) -> bool {
    matches!(
        &node.kind,
        NodeKind::Variable { sigil, name } if sigil == "$" && name == "@"
    )
}

fn push_printf_format_arity_finding(
    rule: &PrintfFormatArityRule,
    source: &str,
    call_node: &Node,
    call_name: &str,
    args: &[Node],
    out: &mut Vec<CriticFinding>,
) {
    let Some(format_node) = args.first() else {
        return;
    };
    let NodeKind::String { value, .. } = &format_node.kind else {
        return;
    };

    let format_content = unquote_string(value);
    if format_content.contains('$') {
        return;
    }

    let specifier_count = count_format_specifiers(format_content);
    let arg_count = args.len().saturating_sub(1);
    if specifier_count != arg_count {
        out.push(printf_format_arity_finding(
            rule,
            source,
            call_node,
            format_node,
            call_name,
            specifier_count,
            arg_count,
        ));
    }
}

fn collect_bareword_filehandle_findings(
    rule: &BarewordFilehandleRule,
    source: &str,
    node: &Node,
    out: &mut Vec<CriticFinding>,
) {
    if let NodeKind::FunctionCall { name, args } = &node.kind {
        push_bareword_filehandle_finding(rule, source, name, args, out);
    }

    for child in node.children() {
        collect_bareword_filehandle_findings(rule, source, child, out);
    }
}

fn push_bareword_filehandle_finding(
    rule: &BarewordFilehandleRule,
    source: &str,
    function_name: &str,
    args: &[Node],
    out: &mut Vec<CriticFinding>,
) {
    if function_name != "open" {
        return;
    }

    let open_args = effective_call_args(args);

    let Some(handle) = open_args.first() else {
        return;
    };
    let NodeKind::Identifier { name } = &handle.kind else {
        return;
    };
    if open_args.len() < 2 || is_standard_filehandle(name) {
        return;
    }

    out.push(bareword_filehandle_finding(rule, source, handle, name));
}

fn collect_two_arg_open_findings(
    rule: &TwoArgOpenRule,
    source: &str,
    node: &Node,
    out: &mut Vec<CriticFinding>,
) {
    if let NodeKind::FunctionCall { name, args } = &node.kind
        && name == "open"
    {
        let open_args = effective_call_args(args);
        if open_args.len() == 2 {
            out.push(two_arg_open_finding(rule, source, node, open_args));
        }
    }

    for child in node.children() {
        collect_two_arg_open_findings(rule, source, child, out);
    }
}

fn collect_pipe_open_findings(
    rule: &PipeOpenRule,
    source: &str,
    node: &Node,
    out: &mut Vec<CriticFinding>,
) {
    if let NodeKind::FunctionCall { name, args } = &node.kind
        && name == "open"
    {
        let open_args = effective_call_args(args);
        if is_pipe_open_args(open_args) {
            out.push(pipe_open_finding(rule, source, node));
        }
    }

    for child in node.children() {
        collect_pipe_open_findings(rule, source, child, out);
    }
}

fn collect_unchecked_open_close_findings(
    rule: &UncheckedOpenCloseRule,
    source: &str,
    node: &Node,
    out: &mut Vec<CriticFinding>,
) {
    if let NodeKind::ExpressionStatement { expression } = &node.kind {
        push_unchecked_open_close_statement_finding(rule, source, expression, out);
        return;
    }

    for child in node.children() {
        collect_unchecked_open_close_findings(rule, source, child, out);
    }
}

fn push_unchecked_open_close_statement_finding(
    rule: &UncheckedOpenCloseRule,
    source: &str,
    expression: &Node,
    out: &mut Vec<CriticFinding>,
) {
    match &expression.kind {
        NodeKind::FunctionCall { name, .. } if is_open_close_call(name) => {
            if !has_trailing_error_check(source, expression) {
                out.push(unchecked_open_close_finding(rule, source, expression, name));
            }
        }
        _ => {}
    }
}

fn is_open_close_call(name: &str) -> bool {
    matches!(name, "open" | "close")
}

fn has_trailing_error_check(source: &str, call_node: &Node) -> bool {
    let Some(call_text) = source.get(call_node.location.start..call_node.location.end) else {
        return false;
    };
    let Some(close_paren) = call_text.trim_end().rfind(')') else {
        return false;
    };
    let trailing = call_text[close_paren + 1..].trim_start();
    starts_with_error_check_operator(trailing)
}

fn starts_with_error_check_operator(text: &str) -> bool {
    text.strip_prefix("||").is_some()
        || text
            .strip_prefix("or")
            .is_some_and(|rest| rest.chars().next().is_none_or(|ch| !is_identifier_continue(ch)))
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn is_pipe_open_args(open_args: &[Node]) -> bool {
    match open_args.len() {
        // 3+ arg form: open(my $fh, "|-", "cmd") or open(my $fh, "-|", "cmd")
        n if n >= 3 => open_args.get(1).is_some_and(is_pipe_mode_string),
        // 2-arg form: open(FH, "|cmd")
        2 => open_args.get(1).is_some_and(is_pipe_two_arg_string),
        _ => false,
    }
}

fn is_pipe_mode_string(node: &Node) -> bool {
    match &node.kind {
        NodeKind::String { value, .. } => {
            let trimmed = value.trim_matches(['"', '\'']);
            trimmed == "|-" || trimmed == "-|"
        }
        _ => false,
    }
}

fn is_pipe_two_arg_string(node: &Node) -> bool {
    match &node.kind {
        NodeKind::String { value, .. } => {
            let trimmed = value.trim_matches(['"', '\'']);
            trimmed.starts_with('|') || trimmed.ends_with('|')
        }
        _ => false,
    }
}

fn collect_qx_readpipe_findings(
    rule: &QxReadpipeRule,
    source: &str,
    node: &Node,
    out: &mut Vec<CriticFinding>,
) {
    match &node.kind {
        NodeKind::String { value, interpolated: true } if is_qx_string(value) => {
            out.push(qx_readpipe_finding(rule, source, node));
        }
        NodeKind::FunctionCall { name, .. } if name == "readpipe" => {
            out.push(qx_readpipe_finding(rule, source, node));
        }
        _ => {}
    }

    for child in node.children() {
        collect_qx_readpipe_findings(rule, source, child, out);
    }
}

fn is_qx_string(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("qx") else {
        return false;
    };
    rest.chars().find(|ch| !ch.is_whitespace()).is_some_and(is_quote_like_delimiter)
}

fn is_quote_like_delimiter(delimiter: char) -> bool {
    !delimiter.is_ascii_alphanumeric() && !delimiter.is_whitespace()
}

fn collect_backtick_exec_findings(
    rule: &BacktickExecRule,
    source: &str,
    node: &Node,
    out: &mut Vec<CriticFinding>,
) {
    if let NodeKind::String { value, interpolated: true } = &node.kind
        && is_backtick_string(value)
    {
        out.push(backtick_exec_finding(rule, source, node));
    }

    for child in node.children() {
        collect_backtick_exec_findings(rule, source, child, out);
    }
}

fn is_backtick_string(value: &str) -> bool {
    value.starts_with('`') && value.ends_with('`') && value.len() >= 2
}

fn collect_string_eval_findings(
    rule: &StringEvalRule,
    source: &str,
    node: &Node,
    out: &mut Vec<CriticFinding>,
) {
    match &node.kind {
        NodeKind::Eval { block } if is_string_eval_expression(block) => {
            out.push(string_eval_finding(rule, source, node));
        }
        NodeKind::FunctionCall { name, args } if name == "eval" => {
            let eval_args = effective_call_args(args);
            if eval_args.first().is_some_and(is_string_eval_expression) {
                out.push(string_eval_finding(rule, source, node));
            }
        }
        _ => {}
    }

    for child in node.children() {
        collect_string_eval_findings(rule, source, child, out);
    }
}

fn collect_system_exec_findings(
    rule: &SystemExecRule,
    source: &str,
    node: &Node,
    out: &mut Vec<CriticFinding>,
) {
    if let NodeKind::FunctionCall { name, .. } = &node.kind
        && matches!(name.as_str(), "system" | "exec")
    {
        out.push(system_exec_finding(rule, source, node, name));
    }

    for child in node.children() {
        collect_system_exec_findings(rule, source, child, out);
    }
}

fn is_string_eval_expression(node: &Node) -> bool {
    match &node.kind {
        NodeKind::String { .. } | NodeKind::Variable { .. } => true,
        NodeKind::Binary { op, .. } => op == ".",
        _ => false,
    }
}

fn effective_call_args(args: &[Node]) -> &[Node] {
    if args.len() == 1
        && let NodeKind::ArrayLiteral { elements } = &args[0].kind
    {
        return elements;
    }

    args
}

fn is_standard_filehandle(name: &str) -> bool {
    matches!(name, "STDIN" | "STDOUT" | "STDERR" | "ARGV" | "ARGVOUT" | "DATA")
}

fn push_assignment_condition_finding(
    rule: &AssignmentInConditionRule,
    source: &str,
    condition: &Node,
    out: &mut Vec<CriticFinding>,
) {
    if is_assignment_condition(source, condition) {
        out.push(assignment_in_condition_finding(rule, source, condition));
    }
}

fn is_assignment_condition(source: &str, condition: &Node) -> bool {
    let is_assignment = matches!(
        &condition.kind,
        NodeKind::Binary { op, .. } if op == "="
    ) || matches!(&condition.kind, NodeKind::Assignment { .. });

    is_assignment
        && !has_extra_condition_parentheses(
            source,
            condition.location.start,
            condition.location.end,
        )
}

fn has_extra_condition_parentheses(source: &str, start: usize, end: usize) -> bool {
    preceding_open_parens(source, start) >= 2 && following_close_parens(source, end) >= 2
}

fn preceding_open_parens(source: &str, start: usize) -> usize {
    let mut count = 0;
    let mut cursor = start.min(source.len());
    while cursor > 0 {
        let Some((idx, ch)) = source[..cursor].char_indices().next_back() else {
            break;
        };
        if ch.is_whitespace() {
            cursor = idx;
            continue;
        }
        if ch == '(' {
            count += 1;
            cursor = idx;
            continue;
        }
        break;
    }
    count
}

fn following_close_parens(source: &str, end: usize) -> usize {
    let mut count = 0;
    let mut cursor = end.min(source.len());
    while cursor < source.len() {
        let Some(ch) = source[cursor..].chars().next() else {
            break;
        };
        if ch.is_whitespace() {
            cursor += ch.len_utf8();
            continue;
        }
        if ch == ')' {
            count += 1;
            cursor += ch.len_utf8();
            continue;
        }
        break;
    }
    count
}

fn assignment_comparison_fix(source: &str, start: usize, end: usize) -> Option<CriticFix> {
    let start = start.min(source.len());
    let end = end.min(source.len()).max(start);
    let equals_offset = source[start..end].find('=')?;
    let equals_start = start + equals_offset;
    let range = range_for_byte_span(source, equals_start, equals_start + 1);

    Some(CriticFix {
        title: "Change to comparison (==)".to_string(),
        safety: FixSafety::Suggested,
        edits: vec![CriticTextEdit { range, new_text: "==".to_string() }],
    })
}

fn count_format_specifiers(format: &str) -> usize {
    let bytes = format.as_bytes();
    let mut count = 0;
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'%' {
            index += 1;
            continue;
        }

        index += 1;
        if index >= bytes.len() {
            break;
        }
        if bytes[index] == b'%' {
            index += 1;
            continue;
        }

        while index < bytes.len() && matches!(bytes[index], b'-' | b'+' | b' ' | b'0' | b'#') {
            index += 1;
        }
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index < bytes.len() && bytes[index] == b'*' {
            index += 1;
        }
        if index < bytes.len() && bytes[index] == b'.' {
            index += 1;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            if index < bytes.len() && bytes[index] == b'*' {
                index += 1;
            }
        }
        if index < bytes.len()
            && matches!(bytes[index], b'h' | b'l' | b'L' | b'q' | b'v' | b'z' | b't')
        {
            index += 1;
            if index < bytes.len() && matches!(bytes[index], b'h' | b'l') {
                index += 1;
            }
        }
        if index < bytes.len()
            && matches!(
                bytes[index],
                b's' | b'd'
                    | b'i'
                    | b'u'
                    | b'o'
                    | b'x'
                    | b'X'
                    | b'e'
                    | b'E'
                    | b'f'
                    | b'F'
                    | b'g'
                    | b'G'
                    | b'c'
                    | b'p'
                    | b'n'
                    | b'b'
            )
        {
            count += 1;
        }
        index += 1;
    }

    count
}

fn unquote_string(raw: &str) -> &str {
    if raw.len() >= 2 {
        let bytes = raw.as_bytes();
        let first = bytes[0];
        let last = bytes[raw.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &raw[1..raw.len() - 1];
        }
    }

    raw
}

fn duplicate_my_fix(source: &str, variable_start: usize) -> Option<CriticFix> {
    let (start, end) = duplicate_my_span(source, variable_start)?;

    Some(CriticFix {
        title: "Remove duplicate 'my' declaration".to_string(),
        safety: FixSafety::Safe,
        edits: vec![CriticTextEdit {
            range: range_for_byte_span(source, start, end),
            new_text: String::new(),
        }],
    })
}

fn duplicate_my_span(source: &str, variable_start: usize) -> Option<(usize, usize)> {
    let variable_start = variable_start.min(source.len());
    let line_start = source[..variable_start].rfind('\n').map_or(0, |pos| pos + 1);
    let before_var = &source[line_start..variable_start];
    let my_offset = before_var.rfind("my ")?;

    if before_var[my_offset + 3..].chars().all(char::is_whitespace) {
        let start = line_start + my_offset;
        Some((start, start + 3))
    } else {
        None
    }
}

fn shadowed_lexical_name(name: &str) -> String {
    let (sigil, base_name) = split_sigil(name);
    format!("{sigil}inner_{base_name}")
}

fn numbered_duplicate_name(name: &str) -> String {
    let (sigil, base_name) = split_sigil(name);
    format!("{sigil}{base_name}_2")
}

fn parameter_shadow_name(name: &str) -> String {
    let (sigil, base_name) = split_sigil(name);
    format!("{sigil}p_{base_name}")
}

fn prefixed_unused_name(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(sigil @ ('$' | '@' | '%' | '&' | '*')) => {
            let rest = chars.as_str();
            format!("{sigil}_{rest}")
        }
        _ => format!("_{name}"),
    }
}

fn bareword_filehandle_lexical_name(name: &str) -> String {
    format!("${}_fh", name.to_lowercase())
}

fn split_sigil(name: &str) -> (&str, &str) {
    let bare = name.trim_start_matches(['$', '@', '%', '&', '*']);
    let sigil_len = name.len() - bare.len();
    (&name[..sigil_len], bare)
}

fn has_use_statement(content: &str, feature: &str) -> bool {
    content.lines().any(|line| has_use_statement_line(line, feature))
}

fn has_use_statement_line(line: &str, feature: &str) -> bool {
    let code_portion = line.split('#').next().unwrap_or_default();
    let mut tokens = code_portion.split_whitespace();
    let Some(first) = tokens.next() else {
        return false;
    };
    if first != "use" {
        return false;
    }
    let Some(module) = tokens.next() else {
        return false;
    };
    module.trim_end_matches(';') == feature
}

fn range_for_byte_span(content: &str, start: usize, end: usize) -> Range {
    let start = start.min(content.len());
    let end = end.min(content.len()).max(start);
    let start_position = position_for_byte_offset(content, start);
    let end_position = position_for_byte_offset(content, end);

    Range { start: start_position, end: end_position }
}

fn position_for_byte_offset(content: &str, offset: usize) -> Position {
    let offset = offset.min(content.len());
    let prefix = &content[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let line_start = prefix.rfind('\n').map_or(0, |idx| idx + 1);
    let column = content[line_start..offset].chars().count();

    Position { byte: offset, line: usize_to_u32(line), column: usize_to_u32(column) }
}

fn usize_to_u32(value: usize) -> u32 {
    value.min(u32::MAX as usize) as u32
}

/// Build an empty AST node for tests that only exercise rule contract plumbing.
#[cfg(test)]
fn empty_program_node() -> Node {
    use perl_parser_core::{NodeKind, SourceLocation};

    Node::new(NodeKind::Program { statements: Vec::new() }, SourceLocation { start: 0, end: 0 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser::Parser;
    use perl_parser_core::position::{Position, Range};

    struct DummyRule;

    impl CriticRule for DummyRule {
        fn id(&self) -> &'static str {
            "native.test.dummy"
        }

        fn category(&self) -> CriticCategory {
            CriticCategory::Syntax
        }

        fn default_severity(&self) -> Severity {
            Severity::Harsh
        }

        fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
            if ctx.source.contains("dummy") {
                out.push(CriticFinding {
                    rule_id: self.id().to_string(),
                    category: self.category(),
                    severity: self.default_severity(),
                    range: Range {
                        start: Position { byte: 0, line: 0, column: 0 },
                        end: Position { byte: 5, line: 0, column: 5 },
                    },
                    message: "dummy finding".to_string(),
                    explanation: "dummy explanation".to_string(),
                    suppression_key: self.id().to_string(),
                    related: Vec::new(),
                    fix: None,
                });
            }
        }
    }

    struct SecondDummyRule;

    impl CriticRule for SecondDummyRule {
        fn id(&self) -> &'static str {
            "native.test.second"
        }

        fn category(&self) -> CriticCategory {
            CriticCategory::Maintainability
        }

        fn default_severity(&self) -> Severity {
            Severity::Cruel
        }

        fn check(&self, ctx: &CriticContext<'_>, out: &mut Vec<CriticFinding>) {
            if ctx.source.contains("second") {
                out.push(CriticFinding {
                    rule_id: self.id().to_string(),
                    category: self.category(),
                    severity: self.default_severity(),
                    range: Range {
                        start: Position { byte: 6, line: 0, column: 6 },
                        end: Position { byte: 12, line: 0, column: 12 },
                    },
                    message: "second finding".to_string(),
                    explanation: "second explanation".to_string(),
                    suppression_key: self.id().to_string(),
                    related: Vec::new(),
                    fix: None,
                });
            }
        }
    }

    fn config_with_minimum_severity(severity: u8) -> CriticConfig {
        CriticConfig { severity, ..Default::default() }
    }

    fn parse_source(source: &str) -> Node {
        let mut parser = Parser::new(source);
        parser.parse().expect("test source should parse")
    }

    #[test]
    fn native_critic_rule_contract_emits_stable_finding_shape() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let ctx = CriticContext::new("dummy", &ast, &config);
        let mut findings = Vec::new();

        DummyRule.check(&ctx, &mut findings);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "native.test.dummy");
        assert_eq!(findings[0].suppression_key, "native.test.dummy");
        assert_eq!(findings[0].category, CriticCategory::Syntax);
        assert_eq!(findings[0].severity, Severity::Harsh);
    }

    #[test]
    fn native_critic_finding_serializes_agent_friendly_fields() {
        let finding = CriticFinding {
            rule_id: "native.test.fixable".to_string(),
            category: CriticCategory::Style,
            severity: Severity::Cruel,
            range: Range {
                start: Position { byte: 0, line: 0, column: 0 },
                end: Position { byte: 1, line: 0, column: 1 },
            },
            message: "style issue".to_string(),
            explanation: "style explanation".to_string(),
            suppression_key: "native.test.fixable".to_string(),
            related: Vec::new(),
            fix: Some(CriticFix {
                title: "Apply style fix".to_string(),
                safety: FixSafety::Safe,
                edits: vec![CriticTextEdit {
                    range: Range {
                        start: Position { byte: 0, line: 0, column: 0 },
                        end: Position { byte: 1, line: 0, column: 1 },
                    },
                    new_text: "x".to_string(),
                }],
            }),
        };

        let value = serde_json::to_value(&finding).expect("serialize native critic finding");

        assert_eq!(value["rule_id"], "native.test.fixable");
        assert_eq!(value["category"], "style");
        assert_eq!(value["fix"]["safety"], "safe");
        assert_eq!(value["fix"]["edits"][0]["new_text"], "x");
    }

    #[test]
    fn native_critic_finding_converts_to_legacy_violation_shape() {
        let finding = CriticFinding {
            rule_id: "native.variables.unused_lexical".to_string(),
            category: CriticCategory::Semantic,
            severity: Severity::Stern,
            range: Range {
                start: Position { byte: 10, line: 1, column: 4 },
                end: Position { byte: 12, line: 1, column: 6 },
            },
            message: "unused lexical variable".to_string(),
            explanation: "remove or use the lexical variable".to_string(),
            suppression_key: "native.variables.unused_lexical".to_string(),
            related: Vec::new(),
            fix: None,
        };

        let violation = finding.to_violation("lib/App.pm");

        assert_eq!(violation.policy, "native.variables.unused_lexical");
        assert_eq!(violation.description, "unused lexical variable");
        assert_eq!(violation.explanation, "remove or use the lexical variable");
        assert_eq!(violation.severity, Severity::Stern);
        assert_eq!(violation.range, finding.range);
        assert_eq!(violation.file, "lib/App.pm");
    }

    #[test]
    fn native_critic_registry_runs_rules_in_order() {
        let ast = empty_program_node();
        let config = config_with_minimum_severity(1);
        let ctx = CriticContext::new("dummy second", &ast, &config);
        let registry =
            NativeCriticRegistry::with_rules(vec![Box::new(DummyRule), Box::new(SecondDummyRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());
        assert_eq!(registry.rule_ids(), vec!["native.test.dummy", "native.test.second"]);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].rule_id, "native.test.dummy");
        assert_eq!(findings[1].rule_id, "native.test.second");
    }

    #[test]
    fn native_critic_registry_can_be_extended_incrementally() {
        let ast = empty_program_node();
        let config = config_with_minimum_severity(1);
        let ctx = CriticContext::new("second", &ast, &config);
        let mut registry = NativeCriticRegistry::new();

        assert!(registry.is_empty());
        registry.add_rule(Box::new(SecondDummyRule));

        let findings = registry.check(&ctx);

        assert_eq!(registry.rule_ids(), vec!["native.test.second"]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].category, CriticCategory::Maintainability);
    }

    #[test]
    fn native_require_use_strict_rule_emits_safe_fix_when_missing() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let ctx = CriticContext::new("my $x = 1;\n", &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(RequireUseStrictRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.testing.require_use_strict");
        assert_eq!(finding.category, CriticCategory::Syntax);
        assert_eq!(finding.severity, Severity::Harsh);
        assert_eq!(finding.message, "Code does not use strict");
        assert_eq!(finding.suppression_key, "native.testing.require_use_strict");

        let fix = finding.fix.as_ref().expect("missing strict should have a safe fix");
        assert_eq!(fix.title, "Add 'use strict'");
        assert_eq!(fix.safety, FixSafety::Safe);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].range, insertion_range());
        assert_eq!(fix.edits[0].new_text, "use strict;\n");
    }

    #[test]
    fn native_require_use_strict_rule_accepts_exact_pragma_only() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let exact_ctx = CriticContext::new("use strict;\nmy $x = 1;\n", &ast, &config);
        let similar_ctx = CriticContext::new("use strictures;\nmy $x = 1;\n", &ast, &config);
        let commented_ctx = CriticContext::new("# use strict;\nmy $x = 1;\n", &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(RequireUseStrictRule)]);

        assert!(registry.check(&exact_ctx).is_empty());
        assert_eq!(registry.check(&similar_ctx).len(), 1);
        assert_eq!(registry.check(&commented_ctx).len(), 1);
    }

    #[test]
    fn native_require_use_warnings_rule_emits_safe_fix_when_missing() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let ctx = CriticContext::new("use strict;\nmy $x = 1;\n", &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(RequireUseWarningsRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.testing.require_use_warnings");
        assert_eq!(finding.category, CriticCategory::Syntax);
        assert_eq!(finding.severity, Severity::Harsh);
        assert_eq!(finding.message, "Code does not use warnings");
        assert_eq!(finding.suppression_key, "native.testing.require_use_warnings");

        let fix = finding.fix.as_ref().expect("missing warnings should have a safe fix");
        assert_eq!(fix.title, "Add 'use warnings'");
        assert_eq!(fix.safety, FixSafety::Safe);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].range, insertion_range());
        assert_eq!(fix.edits[0].new_text, "use warnings;\n");
    }

    #[test]
    fn native_require_use_warnings_rule_accepts_exact_pragma_only() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let exact_ctx = CriticContext::new("use warnings;\nmy $x = 1;\n", &ast, &config);
        let similar_ctx = CriticContext::new("use warningsx;\nmy $x = 1;\n", &ast, &config);
        let commented_ctx = CriticContext::new("# use warnings;\nmy $x = 1;\n", &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(RequireUseWarningsRule)]);

        assert!(registry.check(&exact_ctx).is_empty());
        assert_eq!(registry.check(&similar_ctx).len(), 1);
        assert_eq!(registry.check(&commented_ctx).len(), 1);
    }

    #[test]
    fn native_strict_and_warnings_rules_run_together_in_order() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let ctx = CriticContext::new("my $x = 1;\n", &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![
            Box::new(RequireUseStrictRule),
            Box::new(RequireUseWarningsRule),
        ]);

        let findings = registry.check(&ctx);

        assert_eq!(
            registry.rule_ids(),
            vec!["native.testing.require_use_strict", "native.testing.require_use_warnings"]
        );
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].rule_id, "native.testing.require_use_strict");
        assert_eq!(findings[1].rule_id, "native.testing.require_use_warnings");
    }

    #[test]
    fn native_assignment_in_condition_rule_reports_if_assignment() {
        let source = "use strict;\nuse warnings;\nmy $x = 0;\nif ($x = 5) { print $x; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(AssignmentInConditionRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.common.assignment_in_condition");
        assert_eq!(finding.category, CriticCategory::Syntax);
        assert_eq!(finding.severity, Severity::Stern);
        assert_eq!(finding.message, "Assignment in condition - did you mean '=='?");
        assert_eq!(finding.suppression_key, "native.common.assignment_in_condition");
        assert_eq!(&source[finding.range.start.byte..finding.range.end.byte], "$x = 5");

        let fix = finding.fix.as_ref().expect("assignment condition should offer comparison fix");
        assert_eq!(fix.title, "Change to comparison (==)");
        assert_eq!(fix.safety, FixSafety::Suggested);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(&source[fix.edits[0].range.start.byte..fix.edits[0].range.end.byte], "=");
        assert_eq!(fix.edits[0].new_text, "==");
        assert_eq!(finding.related.len(), 2);
    }

    #[test]
    fn native_assignment_in_condition_rule_reports_statement_modifier_assignment() {
        let source = "use strict;\nuse warnings;\nmy $x = 0;\nprint $x if $x = 5;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(AssignmentInConditionRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "native.common.assignment_in_condition");
        assert_eq!(&source[findings[0].range.start.byte..findings[0].range.end.byte], "$x = 5");
    }

    #[test]
    fn native_assignment_in_condition_rule_reports_while_assignment() {
        let source =
            "use strict;\nuse warnings;\nmy $x = 0;\nwhile ($x = next_value()) { print $x; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(AssignmentInConditionRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "native.common.assignment_in_condition");
        assert_eq!(
            &source[findings[0].range.start.byte..findings[0].range.end.byte],
            "$x = next_value()"
        );
    }

    #[test]
    fn native_assignment_in_condition_rule_accepts_comparisons() {
        let source = "use strict;\nuse warnings;\nmy $x = 0;\nif ($x == 5) { print $x; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(AssignmentInConditionRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "comparison conditions should be accepted");
    }

    #[test]
    fn native_assignment_in_condition_rule_accepts_explicitly_parenthesized_assignments() {
        let source =
            "use strict;\nuse warnings;\nmy $x = 0;\nif (($x = next_value())) { print $x; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(AssignmentInConditionRule)]);

        let findings = registry.check(&ctx);

        assert!(
            findings.is_empty(),
            "double-parenthesized assignment conditions are treated as intentional"
        );
    }

    #[test]
    fn native_assignment_in_condition_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\nmy $x = 0;\nif ($x = 5) { print $x; }\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.common.assignment_in_condition".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(AssignmentInConditionRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no critic native.common.assignment_in_condition -- intentional assignment\nuse strict;\nuse warnings;\nmy $x = 0;\nif ($x = 5) { print $x; }\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());
    }

    #[test]
    fn native_assignment_in_condition_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nmy $x = 0;\nif ($x = 5) { print $x; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(AssignmentInConditionRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.common.assignment_in_condition");
        assert_eq!(violations[0].description, "Assignment in condition - did you mean '=='?");
        assert_eq!(
            violations[0].explanation,
            "Assignments in conditions are usually accidental. Use '==' for numeric comparison, 'eq' for string comparison, or add parentheses if the assignment is intentional."
        );
        assert_eq!(violations[0].severity, Severity::Stern);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_printf_format_arity_rule_reports_static_mismatch() {
        let source = "use strict;\nuse warnings;\nprintf \"%s %s\", $name;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(PrintfFormatArityRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.common.printf_format_arity");
        assert_eq!(finding.category, CriticCategory::Syntax);
        assert_eq!(finding.severity, Severity::Stern);
        assert_eq!(
            finding.message,
            "`printf` format string has 2 specifiers but 1 argument supplied"
        );
        assert_eq!(finding.suppression_key, "native.common.printf_format_arity");
        assert_eq!(
            &source[finding.range.start.byte..finding.range.end.byte],
            "printf \"%s %s\", $name"
        );
        assert_eq!(finding.related.len(), 1);
        assert_eq!(finding.related[0].message, "Format string contains 2 specifiers");
        assert!(finding.fix.is_none());
    }

    #[test]
    fn native_printf_format_arity_rule_accepts_matching_and_dynamic_formats() {
        let source = "use strict;\nuse warnings;\nmy $fmt = \"%s\";\nprintf \"%s\", $name;\nprintf $fmt, $name;\nsprintf \"%s %d\", $name, $count;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(PrintfFormatArityRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "matching and dynamic formats should be accepted");
    }

    #[test]
    fn native_printf_format_arity_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\nprintf \"%s %s\", $name;\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.common.printf_format_arity".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(PrintfFormatArityRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no critic native.common.printf_format_arity -- generated format\nuse strict;\nuse warnings;\nprintf \"%s %s\", $name;\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());
    }

    #[test]
    fn native_printf_format_arity_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nsprintf \"%s %s\", $name;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(PrintfFormatArityRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.common.printf_format_arity");
        assert_eq!(
            violations[0].description,
            "`sprintf` format string has 2 specifiers but 1 argument supplied"
        );
        assert_eq!(
            violations[0].explanation,
            "Add 2 arguments to match 2 format specifiers, or adjust the format string"
        );
        assert_eq!(violations[0].severity, Severity::Stern);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_deprecated_defined_rule_reports_array_defined() {
        let source = "use strict;\nuse warnings;\nmy @items = (1, 2);\nif (defined @items) { print @items; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(DeprecatedDefinedRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.common.deprecated_defined");
        assert_eq!(finding.category, CriticCategory::Syntax);
        assert_eq!(finding.severity, Severity::Stern);
        assert_eq!(finding.message, "Use of 'defined @items' is deprecated");
        assert_eq!(finding.suppression_key, "native.common.deprecated_defined");
        assert_eq!(&source[finding.range.start.byte..finding.range.end.byte], "defined @items");
        assert_eq!(finding.related.len(), 1);

        let fix = finding.fix.as_ref().expect("deprecated defined should offer direct fix");
        assert_eq!(fix.title, "Replace with '@items'");
        assert_eq!(fix.safety, FixSafety::Suggested);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].new_text, "@items");
    }

    #[test]
    fn native_deprecated_defined_rule_reports_parenthesized_hash_defined() {
        let source = "use strict;\nuse warnings;\nmy %seen = (a => 1);\nif (defined(%seen)) { print keys %seen; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(DeprecatedDefinedRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "native.common.deprecated_defined");
        assert_eq!(findings[0].message, "Use of 'defined %seen' is deprecated");
    }

    #[test]
    fn native_deprecated_defined_rule_accepts_scalar_defined() {
        let source =
            "use strict;\nuse warnings;\nmy $item = 1;\nif (defined $item) { print $item; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(DeprecatedDefinedRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "scalar defined checks should be accepted");
    }

    #[test]
    fn native_deprecated_defined_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\nmy @items = (1, 2);\nif (defined @items) { print @items; }\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.common.deprecated_defined".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(DeprecatedDefinedRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no critic native.common.deprecated_defined -- legacy code\nuse strict;\nuse warnings;\nmy @items = (1, 2);\nif (defined @items) { print @items; }\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());
    }

    #[test]
    fn native_deprecated_defined_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nmy @items = (1, 2);\ndefined @items;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(DeprecatedDefinedRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.common.deprecated_defined");
        assert_eq!(violations[0].description, "Use of 'defined @items' is deprecated");
        assert_eq!(
            violations[0].explanation,
            "Testing definedness of a whole array is deprecated because it was rarely useful and often wrong. Use the array in boolean context instead."
        );
        assert_eq!(violations[0].severity, Severity::Stern);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_undef_comparison_rule_reports_numeric_comparison_with_undef() {
        let source = "use strict;\nuse warnings;\nmy $value = maybe();\nif ($value == undef) { print $value; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UndefComparisonRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.common.undef_comparison");
        assert_eq!(finding.category, CriticCategory::Syntax);
        assert_eq!(finding.severity, Severity::Stern);
        assert_eq!(finding.message, "Using '==' with undef -- use defined() to check first");
        assert_eq!(finding.suppression_key, "native.common.undef_comparison");
        assert_eq!(&source[finding.range.start.byte..finding.range.end.byte], "$value == undef");
        assert_eq!(finding.related.len(), 2);

        let fix = finding.fix.as_ref().expect("undef comparison should offer defined() fix");
        assert_eq!(fix.title, "Use defined() check");
        assert_eq!(fix.safety, FixSafety::Suggested);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].new_text, "!defined($value)");
    }

    #[test]
    fn native_undef_comparison_rule_reports_reversed_not_equal_comparison() {
        let source = "use strict;\nuse warnings;\nmy $value = maybe();\nif (undef != $value) { print $value; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UndefComparisonRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "native.common.undef_comparison");
        assert_eq!(
            &source[findings[0].range.start.byte..findings[0].range.end.byte],
            "undef != $value"
        );
        assert_eq!(
            findings[0].fix.as_ref().expect("defined fix").edits[0].new_text,
            "defined($value)"
        );
    }

    #[test]
    fn native_undef_comparison_rule_accepts_defined_checks_and_other_comparisons() {
        let source = "use strict;\nuse warnings;\nmy $value = maybe();\nif (defined $value) { print $value; }\nif ($value == 0) { print $value; }\nif ($value eq undef) { print $value; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UndefComparisonRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "only numeric ==/!= comparisons with undef should report");
    }

    #[test]
    fn native_undef_comparison_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\nmy $value = maybe();\nif ($value == undef) { print $value; }\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.common.undef_comparison".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UndefComparisonRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no critic native.common.undef_comparison -- legacy check\nuse strict;\nuse warnings;\nmy $value = maybe();\nif ($value == undef) { print $value; }\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());
    }

    #[test]
    fn native_undef_comparison_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nmy $value = maybe();\n$value == undef;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UndefComparisonRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.common.undef_comparison");
        assert_eq!(
            violations[0].description,
            "Using '==' with undef -- use defined() to check first"
        );
        assert_eq!(
            violations[0].explanation,
            "Numeric comparison with undef is usually wrong because undef is coerced before comparison. Use defined(...) for definedness checks, or normalize the value before comparing."
        );
        assert_eq!(violations[0].severity, Severity::Stern);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_stale_dollar_at_rule_reports_if_check_after_unlocalized_eval() {
        let source = "use strict;\nuse warnings;\neval { risky_call(); };\nif ($@) { warn $@; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(StaleDollarAtRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.common.stale_dollar_at");
        assert_eq!(finding.category, CriticCategory::Syntax);
        assert_eq!(finding.severity, Severity::Stern);
        assert_eq!(finding.message, "Checking $@ after eval can observe a stale error");
        assert_eq!(finding.suppression_key, "native.common.stale_dollar_at");
        assert_eq!(&source[finding.range.start.byte..finding.range.end.byte], "$@");
        assert_eq!(finding.related.len(), 1);
        assert!(finding.fix.is_none());
    }

    #[test]
    fn native_stale_dollar_at_rule_reports_statement_modifier_after_unlocalized_eval() {
        let source = "use strict;\nuse warnings;\neval { risky_call(); };\nwarn $@ if $@;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(StaleDollarAtRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "native.common.stale_dollar_at");
        assert_eq!(&source[findings[0].range.start.byte..findings[0].range.end.byte], "$@");
    }

    #[test]
    fn native_stale_dollar_at_rule_accepts_localized_or_return_checked_eval() {
        let source = "use strict;\nuse warnings;\n{\nlocal $@;\neval { risky_call(); };\nif ($@) { warn $@; }\n}\nmy $ok = eval { risky_call(); };\nif (!$ok) { warn 'failed'; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(StaleDollarAtRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "localized $@ and eval return checks should be accepted");
    }

    #[test]
    fn native_stale_dollar_at_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\neval { risky_call(); };\nif ($@) { warn $@; }\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.common.stale_dollar_at".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(StaleDollarAtRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no critic native.common.stale_dollar_at -- legacy eval\nuse strict;\nuse warnings;\neval { risky_call(); };\nif ($@) { warn $@; }\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());
    }

    #[test]
    fn native_stale_dollar_at_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\neval { risky_call(); };\nif ($@) { warn $@; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(StaleDollarAtRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.common.stale_dollar_at");
        assert_eq!(violations[0].description, "Checking $@ after eval can observe a stale error");
        assert_eq!(
            violations[0].explanation,
            "The $@ variable is global and can retain or be clobbered by unrelated exception handling. Localize $@ around eval, or check the eval return value before inspecting the error."
        );
        assert_eq!(violations[0].severity, Severity::Stern);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_bareword_filehandle_rule_reports_open_bareword() {
        let source = "use strict;\nuse warnings;\nopen(FH, '<', 'file.txt');\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(BarewordFilehandleRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.io.bareword_filehandle");
        assert_eq!(finding.category, CriticCategory::Syntax);
        assert_eq!(finding.severity, Severity::Stern);
        assert_eq!(finding.message, "Bareword filehandle 'FH' should be lexical");
        assert_eq!(finding.suppression_key, "native.io.bareword_filehandle");
        assert_eq!(&source[finding.range.start.byte..finding.range.end.byte], "FH");

        let fix = finding.fix.as_ref().expect("bareword filehandle should offer lexical fix");
        assert_eq!(fix.title, "Replace bareword filehandle 'FH' with lexical '$fh_fh'");
        assert_eq!(fix.safety, FixSafety::Suggested);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].range, finding.range);
        assert_eq!(fix.edits[0].new_text, "my $fh_fh");
    }

    #[test]
    fn native_bareword_filehandle_rule_accepts_lexical_and_standard_filehandles() {
        let source = "use strict;\nuse warnings;\nopen(my $fh, '<', 'file.txt');\nopen(STDOUT, '>', 'out.txt');\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(BarewordFilehandleRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "lexical and standard filehandles should be accepted");
    }

    #[test]
    fn native_bareword_filehandle_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\nopen(FH, '<', 'file.txt');\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.io.bareword_filehandle".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(BarewordFilehandleRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no critic native.io.bareword_filehandle -- legacy handle\nuse strict;\nuse warnings;\nopen(FH, '<', 'file.txt');\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());
    }

    #[test]
    fn native_bareword_filehandle_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nopen(FH, '<', 'file.txt');\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(BarewordFilehandleRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.io.bareword_filehandle");
        assert_eq!(violations[0].description, "Bareword filehandle 'FH' should be lexical");
        assert_eq!(
            violations[0].explanation,
            "Bareword filehandles are package globals and can be accidentally reused across scopes. Use lexical filehandles for safer IO."
        );
        assert_eq!(violations[0].severity, Severity::Stern);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_two_arg_open_rule_reports_two_arg_open() {
        let source = "use strict;\nuse warnings;\nmy $path = 'file.txt';\nopen(my $fh, $path);\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(TwoArgOpenRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.io.two_arg_open");
        assert_eq!(finding.category, CriticCategory::Security);
        assert_eq!(finding.severity, Severity::Harsh);
        assert_eq!(finding.message, "Two-argument open should use an explicit mode");
        assert_eq!(finding.suppression_key, "native.io.two_arg_open");
        assert_eq!(
            &source[finding.range.start.byte..finding.range.end.byte],
            "open(my $fh, $path)"
        );

        let fix = finding.fix.as_ref().expect("two-arg open should offer a safety fix");
        assert_eq!(fix.title, "Convert to three-argument open() for safety");
        assert_eq!(fix.safety, FixSafety::Suggested);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].range, finding.range);
        assert_eq!(fix.edits[0].new_text, "open(my $fh, '<', $path)");
    }

    #[test]
    fn native_two_arg_open_rule_accepts_three_arg_open() {
        let source =
            "use strict;\nuse warnings;\nmy $path = 'file.txt';\nopen(my $fh, '<', $path);\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(TwoArgOpenRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "three-argument open should be accepted");
    }

    #[test]
    fn native_two_arg_open_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\nmy $path = 'file.txt';\nopen(my $fh, $path);\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.io.two_arg_open".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(TwoArgOpenRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no critic native.io.two_arg_open -- trusted legacy filename\nuse strict;\nuse warnings;\nmy $path = 'file.txt';\nopen(my $fh, $path);\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());

        let severity_config =
            CriticConfig { severity: Severity::Stern as u8, ..Default::default() };
        let severity_ctx = CriticContext::new(source, &ast, &severity_config);
        assert!(registry.check(&severity_ctx).is_empty());
    }

    #[test]
    fn native_two_arg_open_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nmy $path = 'file.txt';\nopen(my $fh, $path);\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(TwoArgOpenRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.io.two_arg_open");
        assert_eq!(violations[0].description, "Two-argument open should use an explicit mode");
        assert_eq!(
            violations[0].explanation,
            "Two-argument open combines mode and filename, which can allow shell interpretation when the filename is derived from input. Use three-argument open with a separate mode."
        );
        assert_eq!(violations[0].severity, Severity::Harsh);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_pipe_open_rule_reports_pipe_open_forms() {
        let source = "use strict;\nuse warnings;\nopen(my $read_fh, '-|', 'ls');\nopen(my $write_fh, '|-', 'cat');\nopen(FH, '|cmd');\nopen(my $legacy_read_fh, 'cat |');\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(PipeOpenRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 4);
        for finding in &findings {
            assert_eq!(finding.rule_id, "native.io.pipe_open");
            assert_eq!(finding.category, CriticCategory::Security);
            assert_eq!(finding.severity, Severity::Harsh);
            assert_eq!(finding.message, "Pipe-open executes a shell command");
            assert_eq!(finding.suppression_key, "native.io.pipe_open");
            assert!(finding.fix.is_none(), "pipe-open replacement is not safe to automate");
        }
        assert_eq!(
            &source[findings[0].range.start.byte..findings[0].range.end.byte],
            "open(my $read_fh, '-|', 'ls')"
        );
        assert_eq!(
            &source[findings[1].range.start.byte..findings[1].range.end.byte],
            "open(my $write_fh, '|-', 'cat')"
        );
        assert_eq!(
            &source[findings[2].range.start.byte..findings[2].range.end.byte],
            "open(FH, '|cmd')"
        );
        assert_eq!(
            &source[findings[3].range.start.byte..findings[3].range.end.byte],
            "open(my $legacy_read_fh, 'cat |')"
        );
    }

    #[test]
    fn native_pipe_open_rule_accepts_normal_open() {
        let source =
            "use strict;\nuse warnings;\nmy $path = 'file.txt';\nopen(my $fh, '<', $path);\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(PipeOpenRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "normal three-argument open should be accepted");
    }

    #[test]
    fn native_pipe_open_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\nopen(my $fh, '-|', 'ls');\n";
        let ast = parse_source(source);
        let excluded_config =
            CriticConfig { exclude: vec!["native.io.pipe_open".to_string()], ..Default::default() };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(PipeOpenRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no critic native.io.pipe_open -- trusted command\nuse strict;\nuse warnings;\nopen(my $fh, '-|', 'ls');\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());

        let severity_config =
            CriticConfig { severity: Severity::Stern as u8, ..Default::default() };
        let severity_ctx = CriticContext::new(source, &ast, &severity_config);
        assert!(registry.check(&severity_ctx).is_empty());
    }

    #[test]
    fn native_pipe_open_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nopen(my $fh, '-|', 'ls');\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(PipeOpenRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.io.pipe_open");
        assert_eq!(violations[0].description, "Pipe-open executes a shell command");
        assert_eq!(
            violations[0].explanation,
            "Pipe-open forms run a command through the shell. Prefer explicit command argument lists or IPC modules when command execution is required."
        );
        assert_eq!(violations[0].severity, Severity::Harsh);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_unchecked_open_close_rule_reports_bare_statement_calls() {
        let source = "use strict;\nuse warnings;\nmy $path = 'file.txt';\nopen(my $fh, '<', $path);\nclose($fh);\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UncheckedOpenCloseRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 2);
        for finding in &findings {
            assert_eq!(finding.rule_id, "native.io.unchecked_open_close");
            assert_eq!(finding.category, CriticCategory::Security);
            assert_eq!(finding.severity, Severity::Stern);
            assert_eq!(finding.suppression_key, "native.io.unchecked_open_close");
            assert!(finding.fix.is_none(), "unchecked I/O fix needs caller-specific error text");
        }
        assert_eq!(findings[0].message, "open() return value should be checked");
        assert_eq!(findings[1].message, "close() return value should be checked");
        assert_eq!(
            &source[findings[0].range.start.byte..findings[0].range.end.byte],
            "open(my $fh, '<', $path)"
        );
        assert_eq!(&source[findings[1].range.start.byte..findings[1].range.end.byte], "close($fh)");
    }

    #[test]
    fn native_unchecked_open_close_rule_accepts_or_die_checks() {
        let source = "use strict;\nuse warnings;\nmy $path = 'file.txt';\nopen(my $fh, '<', $path) or die $!;\nclose($fh) || die $!;\nopen(my $compact_fh, '<', $path)||die $!;\nclose($compact_fh)||die $!;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UncheckedOpenCloseRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "open/close guarded by error paths should be accepted");
    }

    #[test]
    fn native_unchecked_open_close_rule_reports_argument_level_error_checks() {
        let source = "use strict;\nuse warnings;\nmy $path = 'file.txt';\nopen(my $fh, '<', $path || die 'missing path');\nclose($fh || die 'missing handle');\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UncheckedOpenCloseRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].message, "open() return value should be checked");
        assert_eq!(findings[1].message, "close() return value should be checked");
    }

    #[test]
    fn native_unchecked_open_close_rule_composes_with_config_and_suppressions() {
        let source =
            "use strict;\nuse warnings;\nmy $path = 'file.txt';\nopen(my $fh, '<', $path);\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.io.unchecked_open_close".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UncheckedOpenCloseRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let included_config = CriticConfig {
            include: vec!["native.io.unchecked_open_close".to_string()],
            ..Default::default()
        };
        let included_ctx = CriticContext::new(source, &ast, &included_config);
        assert_eq!(registry.check(&included_ctx).len(), 1);

        let other_include_config = CriticConfig {
            include: vec!["native.security.string_eval".to_string()],
            ..Default::default()
        };
        let other_include_ctx = CriticContext::new(source, &ast, &other_include_config);
        assert!(registry.check(&other_include_ctx).is_empty());

        let config = CriticConfig::default();
        let unsuppressed_ctx = CriticContext::new(source, &ast, &config);
        assert_eq!(registry.check(&unsuppressed_ctx).len(), 1);

        let suppressed_source = "## no critic native.io.unchecked_open_close -- handled by caller\nuse strict;\nuse warnings;\nmy $path = 'file.txt';\nopen(my $fh, '<', $path);\n";
        let suppressed_ast = parse_source(suppressed_source);
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());

        let severity_config =
            CriticConfig { severity: Severity::Gentle as u8, ..Default::default() };
        let severity_ctx = CriticContext::new(source, &ast, &severity_config);
        assert!(registry.check(&severity_ctx).is_empty());
    }

    #[test]
    fn native_unchecked_open_close_rule_flows_through_violation_bridge() {
        let source =
            "use strict;\nuse warnings;\nmy $path = 'file.txt';\nopen(my $fh, '<', $path);\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UncheckedOpenCloseRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.io.unchecked_open_close");
        assert_eq!(violations[0].description, "open() return value should be checked");
        assert_eq!(
            violations[0].explanation,
            "open and close report I/O failures through their return value. Check the result with an explicit error path such as `or die` so failures are not silently ignored."
        );
        assert_eq!(violations[0].severity, Severity::Stern);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_qx_readpipe_rule_reports_qx_and_readpipe_forms() {
        let source = "use strict;\nuse warnings;\nmy $date = qx(date);\nmy $listing = qx/ls -la/;\nmy $hash = qx#whoami#;\nmy $pipe_delim = qx|id|;\nmy $tick_delim = qx`uname`;\nmy $pipe = readpipe($date);\nprint $listing . $pipe . $hash . $pipe_delim . $tick_delim;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(QxReadpipeRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 6);
        for finding in &findings {
            assert_eq!(finding.rule_id, "native.security.qx_readpipe");
            assert_eq!(finding.category, CriticCategory::Security);
            assert_eq!(finding.severity, Severity::Harsh);
            assert_eq!(finding.message, "qx/readpipe command execution detected");
            assert_eq!(finding.suppression_key, "native.security.qx_readpipe");
            assert!(finding.fix.is_none(), "qx/readpipe replacement is not safe to automate");
        }
        assert_eq!(&source[findings[0].range.start.byte..findings[0].range.end.byte], "qx(date)");
        assert_eq!(&source[findings[1].range.start.byte..findings[1].range.end.byte], "qx/ls -la/");
        assert_eq!(&source[findings[2].range.start.byte..findings[2].range.end.byte], "qx#whoami#");
        assert_eq!(&source[findings[3].range.start.byte..findings[3].range.end.byte], "qx|id|");
        assert_eq!(&source[findings[4].range.start.byte..findings[4].range.end.byte], "qx`uname`");
        assert_eq!(
            &source[findings[5].range.start.byte..findings[5].range.end.byte],
            "readpipe($date)"
        );
    }

    #[test]
    fn native_qx_readpipe_rule_accepts_non_command_strings_and_calls() {
        let source = "use strict;\nuse warnings;\nmy $text = 'qx(date)';\nmy $value = read_line($text);\nprint $value;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(QxReadpipeRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "ordinary strings and non-readpipe calls should be accepted");
    }

    #[test]
    fn native_qx_readpipe_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\nmy $out = qx(date);\nprint $out;\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.security.qx_readpipe".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(QxReadpipeRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no critic native.security.qx_readpipe -- trusted command\nuse strict;\nuse warnings;\nmy $out = readpipe('date');\nprint $out;\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());

        let severity_config =
            CriticConfig { severity: Severity::Stern as u8, ..Default::default() };
        let severity_ctx = CriticContext::new(source, &ast, &severity_config);
        assert!(registry.check(&severity_ctx).is_empty());
    }

    #[test]
    fn native_qx_readpipe_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nmy $out = qx(date);\nprint $out;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(QxReadpipeRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.security.qx_readpipe");
        assert_eq!(violations[0].description, "qx/readpipe command execution detected");
        assert_eq!(
            violations[0].explanation,
            "qx and readpipe execute shell commands. Prefer explicit command argument lists or IPC modules when command execution is required."
        );
        assert_eq!(violations[0].severity, Severity::Harsh);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_backtick_exec_rule_reports_backtick_strings() {
        let source = "use strict;\nuse warnings;\nmy $out = `ls -la`;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(BacktickExecRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        for finding in &findings {
            assert_eq!(finding.rule_id, "native.security.backtick_exec");
            assert_eq!(finding.category, CriticCategory::Security);
            assert_eq!(finding.severity, Severity::Harsh);
            assert_eq!(finding.message, "Command execution detected");
            assert_eq!(finding.suppression_key, "native.security.backtick_exec");
            assert!(finding.fix.is_none(), "backtick command replacement is not safe to automate");
        }
        assert_eq!(&source[findings[0].range.start.byte..findings[0].range.end.byte], "`ls -la`");
    }

    #[test]
    fn native_backtick_exec_rule_accepts_normal_strings() {
        let source = "use strict;\nuse warnings;\nmy $text = 'not a command';\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(BacktickExecRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "ordinary strings should be accepted");
    }

    #[test]
    fn native_backtick_exec_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\nmy $out = `ls -la`;\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.security.backtick_exec".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(BacktickExecRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no critic native.security.backtick_exec -- trusted command\nuse strict;\nuse warnings;\nmy $out = `ls -la`;\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());

        let severity_config =
            CriticConfig { severity: Severity::Stern as u8, ..Default::default() };
        let severity_ctx = CriticContext::new(source, &ast, &severity_config);
        assert!(registry.check(&severity_ctx).is_empty());
    }

    #[test]
    fn native_backtick_exec_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nmy $out = `ls -la`;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(BacktickExecRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.security.backtick_exec");
        assert_eq!(violations[0].description, "Command execution detected");
        assert_eq!(
            violations[0].explanation,
            "Backticks execute shell commands. Prefer explicit command argument lists or IPC modules when command execution is required."
        );
        assert_eq!(violations[0].severity, Severity::Harsh);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_string_eval_rule_reports_literal_and_variable_eval() {
        let source =
            "use strict;\nuse warnings;\nmy $code = 'print 1';\neval $code;\neval 'print 2';\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(StringEvalRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 2);
        for finding in &findings {
            assert_eq!(finding.rule_id, "native.security.string_eval");
            assert_eq!(finding.category, CriticCategory::Security);
            assert_eq!(finding.severity, Severity::Harsh);
            assert_eq!(finding.message, "String eval is a security risk");
            assert_eq!(finding.suppression_key, "native.security.string_eval");
            assert!(finding.fix.is_none(), "string eval replacement is not safe to automate");
        }
        assert_eq!(&source[findings[0].range.start.byte..findings[0].range.end.byte], "eval $code");
        assert_eq!(
            &source[findings[1].range.start.byte..findings[1].range.end.byte],
            "eval 'print 2'"
        );
    }

    #[test]
    fn native_string_eval_rule_accepts_block_eval() {
        let source = "use strict;\nuse warnings;\neval { my $x = 1; };\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(StringEvalRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "block eval should be accepted");
    }

    #[test]
    fn native_string_eval_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\nmy $code = 'print 1';\neval $code;\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.security.string_eval".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(StringEvalRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no critic native.security.string_eval -- generated DSL\nuse strict;\nuse warnings;\nmy $code = 'print 1';\neval $code;\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());

        let severity_config =
            CriticConfig { severity: Severity::Stern as u8, ..Default::default() };
        let severity_ctx = CriticContext::new(source, &ast, &severity_config);
        assert!(registry.check(&severity_ctx).is_empty());
    }

    #[test]
    fn native_string_eval_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nmy $code = 'print 1';\neval $code;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(StringEvalRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.security.string_eval");
        assert_eq!(violations[0].description, "String eval is a security risk");
        assert_eq!(
            violations[0].explanation,
            "String eval executes dynamically generated Perl code and is difficult to analyze safely. Prefer block eval for exception handling or a safer dispatch mechanism."
        );
        assert_eq!(violations[0].severity, Severity::Harsh);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_system_exec_rule_reports_system_and_exec_forms() {
        let source = "use strict;\nuse warnings;\nmy $cmd = 'ls';\nsystem($cmd);\nsystem('ls', '-la');\nexec($cmd);\nexec('ls', '-la');\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(SystemExecRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 4);
        for finding in &findings {
            assert_eq!(finding.rule_id, "native.security.system_exec");
            assert_eq!(finding.category, CriticCategory::Security);
            assert_eq!(finding.severity, Severity::Harsh);
            assert_eq!(finding.suppression_key, "native.security.system_exec");
            assert!(finding.fix.is_none(), "system/exec replacement is not safe to automate");
        }
        assert_eq!(findings[0].message, "system() executes a shell command");
        assert_eq!(findings[1].message, "system() executes a shell command");
        assert_eq!(findings[2].message, "exec() replaces the current process with a shell command");
        assert_eq!(findings[3].message, "exec() replaces the current process with a shell command");
        assert_eq!(
            &source[findings[0].range.start.byte..findings[0].range.end.byte],
            "system($cmd)"
        );
        assert_eq!(
            &source[findings[1].range.start.byte..findings[1].range.end.byte],
            "system('ls', '-la')"
        );
        assert_eq!(&source[findings[2].range.start.byte..findings[2].range.end.byte], "exec($cmd)");
        assert_eq!(
            &source[findings[3].range.start.byte..findings[3].range.end.byte],
            "exec('ls', '-la')"
        );
    }

    #[test]
    fn native_system_exec_rule_accepts_non_command_calls() {
        let source = "use strict;\nuse warnings;\nmy $system = 'name';\nmy $exec_ok = run_exec($system);\nprint $exec_ok;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(SystemExecRule)]);

        let findings = registry.check(&ctx);

        assert!(
            findings.is_empty(),
            "ordinary variables and non-system/exec calls should be accepted"
        );
    }

    #[test]
    fn native_system_exec_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\nmy $cmd = 'ls';\nsystem($cmd);\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.security.system_exec".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(SystemExecRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no critic native.security.system_exec -- trusted command\nuse strict;\nuse warnings;\nmy $cmd = 'ls';\nexec($cmd);\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());

        let severity_config =
            CriticConfig { severity: Severity::Stern as u8, ..Default::default() };
        let severity_ctx = CriticContext::new(source, &ast, &severity_config);
        assert!(registry.check(&severity_ctx).is_empty());
    }

    #[test]
    fn native_system_exec_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nmy $cmd = 'ls';\nsystem($cmd);\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(SystemExecRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.security.system_exec");
        assert_eq!(violations[0].description, "system() executes a shell command");
        assert_eq!(
            violations[0].explanation,
            "system and exec run external commands. Prefer explicit argument lists or IPC modules when command execution is required, and validate any user-controlled input."
        );
        assert_eq!(violations[0].severity, Severity::Harsh);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_recommended_registry_contains_initial_policy_bundle() {
        let source = "print 1;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::recommended();

        let findings = registry.check(&ctx);

        assert_eq!(
            registry.rule_ids(),
            vec![
                "native.testing.require_use_strict",
                "native.testing.require_use_warnings",
                "native.common.assignment_in_condition",
                "native.common.printf_format_arity",
                "native.common.deprecated_defined",
                "native.common.undef_comparison",
                "native.common.stale_dollar_at",
                "native.io.bareword_filehandle",
                "native.io.two_arg_open",
                "native.io.pipe_open",
                "native.io.unchecked_open_close",
                "native.security.qx_readpipe",
                "native.security.backtick_exec",
                "native.security.string_eval",
                "native.security.system_exec",
                "native.variables.unused_lexical",
                "native.variables.unused_parameter",
                "native.variables.duplicate_parameter",
                "native.variables.parameter_shadows_global",
                "native.variables.duplicate_lexical",
                "native.variables.shadowed_lexical"
            ]
        );
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].suppression_key, "native.testing.require_use_strict");
        assert_eq!(findings[1].suppression_key, "native.testing.require_use_warnings");
    }

    #[test]
    fn native_unused_lexical_rule_reports_declared_but_unread_variable() {
        let source = "use strict;\nuse warnings;\nmy $unused = 1;\nprint 1;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UnusedLexicalVariableRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.variables.unused_lexical");
        assert_eq!(finding.category, CriticCategory::Semantic);
        assert_eq!(finding.severity, Severity::Stern);
        assert_eq!(finding.message, "Lexical variable '$unused' is declared but never used");
        assert_eq!(finding.suppression_key, "native.variables.unused_lexical");
        assert_eq!(&source[finding.range.start.byte..finding.range.end.byte], "$unused");

        let fix = finding.fix.as_ref().expect("unused lexical should offer an intent marker");
        assert_eq!(fix.title, "Rename to '$_unused'");
        assert_eq!(fix.safety, FixSafety::Suggested);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].range, finding.range);
        assert_eq!(fix.edits[0].new_text, "$_unused");
    }

    #[test]
    fn native_unused_lexical_rule_accepts_used_and_intentionally_unused_variables() {
        let source = "use strict;\nuse warnings;\nmy $used = 1;\nmy $_ignored = 2;\nprint $used;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UnusedLexicalVariableRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "used and underscore-prefixed variables should be accepted");
    }

    #[test]
    fn native_unused_lexical_rule_reports_multiple_sigils() {
        let source = "use strict;\nuse warnings;\nmy @items = (1, 2);\nmy %seen = ();\nprint 1;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UnusedLexicalVariableRule)]);

        let findings = registry.check(&ctx);
        let names = findings
            .iter()
            .map(|finding| &source[finding.range.start.byte..finding.range.end.byte])
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["@items", "%seen"]);
        assert_eq!(
            findings
                .iter()
                .map(|finding| finding.fix.as_ref().expect("fix").edits[0].new_text.as_str())
                .collect::<Vec<_>>(),
            vec!["@_items", "%_seen"]
        );
    }

    #[test]
    fn native_unused_lexical_rule_composes_with_config_and_suppressions() {
        let ast = parse_source("use strict;\nuse warnings;\nmy $unused = 1;\n");
        let excluded_config = CriticConfig {
            exclude: vec!["native.variables.unused_lexical".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(
            "use strict;\nuse warnings;\nmy $unused = 1;\n",
            &ast,
            &excluded_config,
        );
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UnusedLexicalVariableRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no critic native.variables.unused_lexical -- legacy fixture\nuse strict;\nuse warnings;\nmy $unused = 1;\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());
    }

    #[test]
    fn native_unused_lexical_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nmy $unused = 1;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UnusedLexicalVariableRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.variables.unused_lexical");
        assert_eq!(
            violations[0].description,
            "Lexical variable '$unused' is declared but never used"
        );
        assert_eq!(
            violations[0].explanation,
            "Remove the lexical variable, use it, or prefix it with '_' to mark it intentionally unused."
        );
        assert_eq!(violations[0].severity, Severity::Stern);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_unused_parameter_rule_reports_unread_signature_parameter() {
        let source = "use strict;\nuse warnings;\nsub helper($used, $unused) { return $used; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UnusedParameterRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.variables.unused_parameter");
        assert_eq!(finding.category, CriticCategory::Semantic);
        assert_eq!(finding.severity, Severity::Stern);
        assert_eq!(finding.message, "Parameter '$unused' is never used");
        assert_eq!(finding.suppression_key, "native.variables.unused_parameter");
        assert_eq!(&source[finding.range.start.byte..finding.range.end.byte], "$unused");

        let fix = finding.fix.as_ref().expect("unused parameter should offer an intent marker");
        assert_eq!(fix.title, "Rename to '$_unused'");
        assert_eq!(fix.safety, FixSafety::Suggested);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].range, finding.range);
        assert_eq!(fix.edits[0].new_text, "$_unused");
    }

    #[test]
    fn native_unused_parameter_rule_accepts_used_and_intentionally_unused_parameters() {
        let source = "use strict;\nuse warnings;\nsub helper($used, $_ignored) { return $used; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UnusedParameterRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "used and underscore-prefixed parameters should be accepted");
    }

    #[test]
    fn native_unused_parameter_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\nsub helper($used, $unused) { return $used; }\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.variables.unused_parameter".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UnusedParameterRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no critic native.variables.unused_parameter -- fixture\nuse strict;\nuse warnings;\nsub helper($used, $unused) { return $used; }\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());
    }

    #[test]
    fn native_unused_parameter_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nsub helper($used, $unused) { return $used; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(UnusedParameterRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.variables.unused_parameter");
        assert_eq!(violations[0].description, "Parameter '$unused' is never used");
        assert_eq!(
            violations[0].explanation,
            "Use the parameter or prefix it with '_' to mark it intentionally unused."
        );
        assert_eq!(violations[0].severity, Severity::Stern);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_duplicate_parameter_rule_reports_repeated_signature_parameter() {
        let source = "use strict;\nuse warnings;\nsub helper($arg, $arg) { return $arg; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(DuplicateParameterRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.variables.duplicate_parameter");
        assert_eq!(finding.category, CriticCategory::Semantic);
        assert_eq!(finding.severity, Severity::Gentle);
        assert_eq!(finding.message, "Parameter '$arg' appears more than once in this signature");
        assert_eq!(finding.suppression_key, "native.variables.duplicate_parameter");
        assert_eq!(&source[finding.range.start.byte..finding.range.end.byte], "$arg");

        let fix = finding.fix.as_ref().expect("duplicate parameter should offer rename");
        assert_eq!(fix.title, "Rename duplicate parameter to '$arg_2'");
        assert_eq!(fix.safety, FixSafety::Suggested);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].range, finding.range);
        assert_eq!(fix.edits[0].new_text, "$arg_2");
    }

    #[test]
    fn native_duplicate_parameter_rule_accepts_unique_parameters() {
        let source =
            "use strict;\nuse warnings;\nsub helper($left, $right) { return $left + $right; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(DuplicateParameterRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "unique parameters should be accepted");
    }

    #[test]
    fn native_duplicate_parameter_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\nsub helper($arg, $arg) { return $arg; }\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.variables.duplicate_parameter".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(DuplicateParameterRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no perl-lsp-critic native.variables.duplicate_parameter -- fixture\nuse strict;\nuse warnings;\nsub helper($arg, $arg) { return $arg; }\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());
    }

    #[test]
    fn native_duplicate_parameter_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nsub helper($arg, $arg) { return $arg; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(DuplicateParameterRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.variables.duplicate_parameter");
        assert_eq!(
            violations[0].description,
            "Parameter '$arg' appears more than once in this signature"
        );
        assert_eq!(
            violations[0].explanation,
            "Remove the duplicate parameter or rename it so every signature binding is unique."
        );
        assert_eq!(violations[0].severity, Severity::Gentle);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_parameter_shadows_global_rule_reports_parameter_shadowing() {
        let source = "use strict;\nuse warnings;\nmy $name = 'outer';\nsub helper($name) { return $name; }\nprint $name;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(ParameterShadowsGlobalRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.variables.parameter_shadows_global");
        assert_eq!(finding.category, CriticCategory::Semantic);
        assert_eq!(finding.severity, Severity::Stern);
        assert_eq!(finding.message, "Parameter '$name' shadows an outer declaration");
        assert_eq!(finding.suppression_key, "native.variables.parameter_shadows_global");
        assert_eq!(&source[finding.range.start.byte..finding.range.end.byte], "$name");

        let fix = finding.fix.as_ref().expect("shadowing parameter should offer rename");
        assert_eq!(fix.title, "Rename parameter to '$p_name'");
        assert_eq!(fix.safety, FixSafety::Suggested);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].range, finding.range);
        assert_eq!(fix.edits[0].new_text, "$p_name");
    }

    #[test]
    fn native_parameter_shadows_global_rule_accepts_unique_parameters() {
        let source =
            "use strict;\nuse warnings;\nmy $outer = 1;\nsub helper($inner) { return $inner; }\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(ParameterShadowsGlobalRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "non-shadowing parameters should be accepted");
    }

    #[test]
    fn native_parameter_shadows_global_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\nmy $name = 'outer';\nsub helper($name) { return $name; }\nprint $name;\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.variables.parameter_shadows_global".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(ParameterShadowsGlobalRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no critic native.variables.parameter_shadows_global -- fixture\nuse strict;\nuse warnings;\nmy $name = 'outer';\nsub helper($name) { return $name; }\nprint $name;\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());
    }

    #[test]
    fn native_parameter_shadows_global_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nmy $name = 'outer';\nsub helper($name) { return $name; }\nprint $name;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry = NativeCriticRegistry::with_rules(vec![Box::new(ParameterShadowsGlobalRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.variables.parameter_shadows_global");
        assert_eq!(violations[0].description, "Parameter '$name' shadows an outer declaration");
        assert_eq!(
            violations[0].explanation,
            "Rename the parameter or use the outer variable directly to avoid confusing scope shadowing."
        );
        assert_eq!(violations[0].severity, Severity::Stern);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_duplicate_lexical_rule_reports_same_scope_redeclaration() {
        let source = "use strict;\nuse warnings;\nmy $dup = 1;\nmy $dup = 2;\nprint $dup;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry =
            NativeCriticRegistry::with_rules(vec![Box::new(DuplicateLexicalDeclarationRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.variables.duplicate_lexical");
        assert_eq!(finding.category, CriticCategory::Semantic);
        assert_eq!(finding.severity, Severity::Gentle);
        assert_eq!(
            finding.message,
            "Lexical variable '$dup' is declared more than once in the same scope"
        );
        assert_eq!(finding.suppression_key, "native.variables.duplicate_lexical");
        assert_eq!(&source[finding.range.start.byte..finding.range.end.byte], "$dup");

        let fix = finding.fix.as_ref().expect("duplicate my should offer a safe fix");
        assert_eq!(fix.title, "Remove duplicate 'my' declaration");
        assert_eq!(fix.safety, FixSafety::Safe);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(&source[fix.edits[0].range.start.byte..fix.edits[0].range.end.byte], "my ");
        assert_eq!(fix.edits[0].new_text, "");
    }

    #[test]
    fn native_duplicate_lexical_rule_accepts_nested_shadowing() {
        let source = "use strict;\nuse warnings;\nmy $value = 1;\n{ my $value = 2; print $value; }\nprint $value;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry =
            NativeCriticRegistry::with_rules(vec![Box::new(DuplicateLexicalDeclarationRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "nested lexical shadowing is not same-scope duplication");
    }

    #[test]
    fn native_duplicate_lexical_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\nmy $dup = 1;\nmy $dup = 2;\nprint $dup;\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.variables.duplicate_lexical".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry =
            NativeCriticRegistry::with_rules(vec![Box::new(DuplicateLexicalDeclarationRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no perl-lsp-critic native.variables.duplicate_lexical -- fixture\nuse strict;\nuse warnings;\nmy $dup = 1;\nmy $dup = 2;\nprint $dup;\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());
    }

    #[test]
    fn native_duplicate_lexical_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nmy $dup = 1;\nmy $dup = 2;\nprint $dup;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry =
            NativeCriticRegistry::with_rules(vec![Box::new(DuplicateLexicalDeclarationRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.variables.duplicate_lexical");
        assert_eq!(
            violations[0].description,
            "Lexical variable '$dup' is declared more than once in the same scope"
        );
        assert_eq!(
            violations[0].explanation,
            "Remove the duplicate lexical declarator or assign to the existing lexical variable."
        );
        assert_eq!(violations[0].severity, Severity::Gentle);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_shadowed_lexical_rule_reports_inner_shadowing() {
        let source = "use strict;\nuse warnings;\nmy $value = 1;\n{ my $value = 2; print $value; }\nprint $value;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry =
            NativeCriticRegistry::with_rules(vec![Box::new(ShadowedLexicalVariableRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.rule_id, "native.variables.shadowed_lexical");
        assert_eq!(finding.category, CriticCategory::Semantic);
        assert_eq!(finding.severity, Severity::Stern);
        assert_eq!(finding.message, "Lexical variable '$value' shadows an outer declaration");
        assert_eq!(finding.suppression_key, "native.variables.shadowed_lexical");
        assert_eq!(&source[finding.range.start.byte..finding.range.end.byte], "$value");

        let fix = finding.fix.as_ref().expect("shadowed lexical should offer a rename");
        assert_eq!(fix.title, "Rename to '$inner_value'");
        assert_eq!(fix.safety, FixSafety::Suggested);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].range, finding.range);
        assert_eq!(fix.edits[0].new_text, "$inner_value");
    }

    #[test]
    fn native_shadowed_lexical_rule_accepts_unique_nested_lexicals() {
        let source = "use strict;\nuse warnings;\nmy $outer = 1;\n{ my $inner = 2; print $inner; }\nprint $outer;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry =
            NativeCriticRegistry::with_rules(vec![Box::new(ShadowedLexicalVariableRule)]);

        let findings = registry.check(&ctx);

        assert!(findings.is_empty(), "unique nested lexicals should not be shadowing findings");
    }

    #[test]
    fn native_shadowed_lexical_rule_composes_with_config_and_suppressions() {
        let source = "use strict;\nuse warnings;\nmy $value = 1;\n{ my $value = 2; print $value; }\nprint $value;\n";
        let ast = parse_source(source);
        let excluded_config = CriticConfig {
            exclude: vec!["native.variables.shadowed_lexical".to_string()],
            ..Default::default()
        };
        let excluded_ctx = CriticContext::new(source, &ast, &excluded_config);
        let registry =
            NativeCriticRegistry::with_rules(vec![Box::new(ShadowedLexicalVariableRule)]);

        assert!(registry.check(&excluded_ctx).is_empty());

        let suppressed_source = "## no critic native.variables.shadowed_lexical -- fixture\nuse strict;\nuse warnings;\nmy $value = 1;\n{ my $value = 2; print $value; }\nprint $value;\n";
        let suppressed_ast = parse_source(suppressed_source);
        let config = CriticConfig::default();
        let suppressed_ctx = CriticContext::new(suppressed_source, &suppressed_ast, &config);

        assert!(registry.check(&suppressed_ctx).is_empty());
    }

    #[test]
    fn native_shadowed_lexical_rule_flows_through_violation_bridge() {
        let source = "use strict;\nuse warnings;\nmy $value = 1;\n{ my $value = 2; print $value; }\nprint $value;\n";
        let ast = parse_source(source);
        let config = CriticConfig::default();
        let ctx = CriticContext::new(source, &ast, &config);
        let registry =
            NativeCriticRegistry::with_rules(vec![Box::new(ShadowedLexicalVariableRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.variables.shadowed_lexical");
        assert_eq!(
            violations[0].description,
            "Lexical variable '$value' shadows an outer declaration"
        );
        assert_eq!(
            violations[0].explanation,
            "Rename the inner lexical variable or use the outer variable directly to avoid confusing scope shadowing."
        );
        assert_eq!(violations[0].severity, Severity::Stern);
        assert_eq!(violations[0].file, "lib/App.pm");
    }

    #[test]
    fn native_critic_registry_maps_findings_to_legacy_violations() {
        let ast = empty_program_node();
        let config = config_with_minimum_severity(1);
        let ctx = CriticContext::new("dummy second", &ast, &config);
        let registry =
            NativeCriticRegistry::with_rules(vec![Box::new(DummyRule), Box::new(SecondDummyRule)]);

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 2);
        assert_eq!(violations[0].policy, "native.test.dummy");
        assert_eq!(violations[0].description, "dummy finding");
        assert_eq!(violations[0].file, "lib/App.pm");
        assert_eq!(violations[1].policy, "native.test.second");
        assert_eq!(violations[1].description, "second finding");
        assert_eq!(violations[1].file, "lib/App.pm");
    }

    #[test]
    fn native_critic_registry_honors_include_and_exclude_config() {
        let ast = empty_program_node();
        let config = CriticConfig {
            severity: 1,
            include: vec!["native.test.dummy".to_string()],
            exclude: vec!["native.test.second".to_string()],
            ..Default::default()
        };
        let ctx = CriticContext::new("dummy second", &ast, &config);
        let registry =
            NativeCriticRegistry::with_rules(vec![Box::new(DummyRule), Box::new(SecondDummyRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "native.test.dummy");
    }

    #[test]
    fn native_critic_registry_honors_minimum_severity_config() {
        let ast = empty_program_node();
        let config = CriticConfig { severity: 3, ..Default::default() };
        let ctx = CriticContext::new("dummy second", &ast, &config);
        let registry =
            NativeCriticRegistry::with_rules(vec![Box::new(DummyRule), Box::new(SecondDummyRule)]);

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "native.test.dummy");
        assert_eq!(findings[0].severity, Severity::Harsh);
    }

    #[test]
    fn native_critic_suppression_map_parses_directives_and_reasons() {
        let source = "\
## no critic native.testing.require_use_strict -- generated legacy file
## no perl-lsp-critic native.testing.require_use_warnings,native.test.second
my $x = 1;
";

        let suppressions = CriticSuppressionMap::from_source(source);

        assert_eq!(suppressions.suppressions().len(), 3);
        assert_eq!(suppressions.suppressions()[0].rule_id, "native.testing.require_use_strict");
        assert_eq!(suppressions.suppressions()[0].scope, CriticSuppressionScope::File);
        assert_eq!(suppressions.suppressions()[0].line, 0);
        assert_eq!(suppressions.suppressions()[0].reason.as_deref(), Some("generated legacy file"));
        assert_eq!(suppressions.suppressions()[1].rule_id, "native.testing.require_use_warnings");
        assert_eq!(suppressions.suppressions()[2].rule_id, "native.test.second");
    }

    #[test]
    fn native_critic_registry_filters_suppressed_findings() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let ctx = CriticContext::new(
            "## no critic native.testing.require_use_strict -- legacy file\nmy $x = 1;\n",
            &ast,
            &config,
        );
        let registry = NativeCriticRegistry::recommended();

        let findings = registry.check(&ctx);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "native.testing.require_use_warnings");
    }

    #[test]
    fn native_critic_registry_filters_suppressed_violations() {
        let ast = empty_program_node();
        let config = CriticConfig::default();
        let ctx = CriticContext::new(
            "## no perl-lsp-critic native.testing.require_use_warnings\nmy $x = 1;\n",
            &ast,
            &config,
        );
        let registry = NativeCriticRegistry::recommended();

        let violations = registry.check_violations(&ctx, "lib/App.pm");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy, "native.testing.require_use_strict");
        assert_eq!(violations[0].file, "lib/App.pm");
    }
}
