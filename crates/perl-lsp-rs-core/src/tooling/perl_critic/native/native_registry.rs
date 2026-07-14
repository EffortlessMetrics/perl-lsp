//! Native critic rule registry and profile orchestration.

use super::super::{CriticConfig, Severity, Violation};
use super::native_contract::{CriticContext, CriticFinding, CriticRule};
use super::native_suppressions::CriticSuppressionMap;
use super::{
    AssignmentInConditionRule, BacktickExecRule, BarewordFilehandleRule,
    CaptureVarWithoutRegexMatchRule, DeprecatedDefinedRule, DuplicateLexicalDeclarationRule,
    DuplicateParameterRule, ParameterShadowsGlobalRule, PipeOpenRule, PrintfFormatArityRule,
    ProhibitLeadingZerosRule, QxReadpipeRule, RequirePodSectionsRule, RequireUseStrictRule,
    RequireUseWarningsRule, ShadowedLexicalVariableRule, StaleDollarAtRule, StringEvalRule,
    SystemExecRule, TwoArgOpenRule, UncheckedOpenCloseRule, UndeclaredVariableRule,
    UndefComparisonRule, UninitializedVariableRule, UnquotedBarewordRule, UnreachableCodeRule,
    UnusedLexicalVariableRule, UnusedParameterRule,
};

/// Native critic rule bundle used by receipt and readiness tooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCriticProfile {
    /// Lower-noise candidate for eventual default/native recommended use.
    Recommended,
    /// Every registered native rule, useful for strict audits and rule coverage.
    Strict,
}

impl NativeCriticProfile {
    /// Parse a native critic profile token.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "recommended" => Some(Self::Recommended),
            "strict" => Some(Self::Strict),
            _ => None,
        }
    }

    /// Stable profile label for receipts.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Recommended => "recommended",
            Self::Strict => "strict",
        }
    }
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
    /// This is the lower-noise bundle intended for normal editor diagnostics.
    /// Keep ordering stable so diagnostics and receipts are deterministic.
    #[must_use]
    pub fn recommended() -> Self {
        Self::for_profile(NativeCriticProfile::Recommended)
    }

    /// Create a native critic registry for a named profile.
    ///
    /// The explicit profile entry point lets receipts and readiness checks
    /// measure either the lower-noise default or the full strict rule set.
    #[must_use]
    pub fn for_profile(profile: NativeCriticProfile) -> Self {
        match profile {
            NativeCriticProfile::Recommended => Self::recommended_profile(),
            NativeCriticProfile::Strict => Self::strict_profile(),
        }
    }

    fn recommended_profile() -> Self {
        Self::with_rules(vec![
            Box::new(RequireUseStrictRule),
            Box::new(RequireUseWarningsRule),
            Box::new(AssignmentInConditionRule),
            Box::new(PrintfFormatArityRule),
            Box::new(DeprecatedDefinedRule),
            Box::new(UndefComparisonRule),
            Box::new(StaleDollarAtRule),
            Box::new(UnreachableCodeRule),
            Box::new(BarewordFilehandleRule),
            Box::new(TwoArgOpenRule),
            Box::new(PipeOpenRule),
            Box::new(UncheckedOpenCloseRule),
            Box::new(QxReadpipeRule),
            Box::new(BacktickExecRule),
            Box::new(StringEvalRule),
            Box::new(SystemExecRule),
        ])
    }

    fn strict_profile() -> Self {
        Self::with_rules(vec![
            Box::new(RequireUseStrictRule),
            Box::new(RequireUseWarningsRule),
            Box::new(AssignmentInConditionRule),
            Box::new(PrintfFormatArityRule),
            Box::new(DeprecatedDefinedRule),
            Box::new(UndefComparisonRule),
            Box::new(StaleDollarAtRule),
            Box::new(UnreachableCodeRule),
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
            Box::new(CaptureVarWithoutRegexMatchRule),
            Box::new(UndeclaredVariableRule),
            Box::new(UninitializedVariableRule),
            Box::new(UnquotedBarewordRule),
            Box::new(RequirePodSectionsRule),
            Box::new(ProhibitLeadingZerosRule),
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
