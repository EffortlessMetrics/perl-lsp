//! Scope analyzer issue to diagnostic conversion
//!
//! This module provides functionality for converting scope analyzer issues
//! into diagnostic messages with pragma-aware severity mapping.

use perl_diagnostics::codes::DiagnosticCode;
use perl_semantic_analyzer::scope_analyzer::{IssueKind, ScopeIssue};
use perl_semantic_facts::{Confidence, FileId, VisibleSymbol, VisibleSymbolSource};
use perl_workspace::semantic::queries::SemanticQueries;

use super::internal_types::{Diagnostic, DiagnosticTag, RelatedInformation};
use perl_diagnostics::codes::DiagnosticSeverity;

/// Convert scope analyzer issues to diagnostics
///
/// This function processes scope analyzer issues and converts them into
/// appropriate diagnostics with severity levels, codes, and helpful related
/// information based on the issue type.
///
/// # Backward compatibility
///
/// Preserved for callers that do not have semantic query data. This path
/// preserves the original conversion logic directly — no semantic suppression
/// is applied. Callers with workspace semantic data should call
/// [`scope_issues_to_diagnostics_with_semantics`] instead.
#[allow(dead_code)] // Preserved for API backward compatibility
pub fn scope_issues_to_diagnostics(issues: Vec<ScopeIssue>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for issue in issues {
        let severity = match issue.kind {
            IssueKind::UndeclaredVariable
            | IssueKind::VariableRedeclaration
            | IssueKind::DuplicateParameter
            | IssueKind::UnquotedBareword => DiagnosticSeverity::Error,
            IssueKind::VariableShadowing
            | IssueKind::UnusedVariable
            | IssueKind::ParameterShadowsGlobal
            | IssueKind::UnusedParameter
            | IssueKind::UninitializedVariable => DiagnosticSeverity::Warning,
            IssueKind::CaptureVarWithoutRegexMatch => DiagnosticSeverity::Information,
        };

        let code = match issue.kind {
            IssueKind::UndeclaredVariable => DiagnosticCode::UndefinedVariable,
            IssueKind::UnusedVariable => DiagnosticCode::UnusedVariable,
            IssueKind::VariableShadowing => DiagnosticCode::VariableShadowing,
            IssueKind::VariableRedeclaration => DiagnosticCode::VariableRedeclaration,
            IssueKind::DuplicateParameter => DiagnosticCode::DuplicateParameter,
            IssueKind::ParameterShadowsGlobal => DiagnosticCode::ParameterShadowsGlobal,
            IssueKind::UnusedParameter => DiagnosticCode::UnusedParameter,
            IssueKind::UnquotedBareword => DiagnosticCode::UnquotedBareword,
            IssueKind::UninitializedVariable => DiagnosticCode::UninitializedVariable,
            IssueKind::CaptureVarWithoutRegexMatch => DiagnosticCode::CaptureVarWithoutRegexMatch,
        };

        let related_info = build_scope_related_info(&issue);
        let suggestion = build_scope_suggestion(&issue);

        diagnostics.push(Diagnostic {
            range: issue.range,
            severity,
            code: Some(code.as_str().to_string()),
            message: build_enhanced_scope_message(&issue),
            related_information: related_info,
            tags: if matches!(issue.kind, IssueKind::UnusedVariable | IssueKind::UnusedParameter) {
                vec![DiagnosticTag::Unnecessary]
            } else {
                Vec::new()
            },
            suggestion,
        });
    }

    diagnostics
}

/// Convert scope analyzer issues to diagnostics with dynamic-boundary suppression.
///
/// Extends [`scope_issues_to_diagnostics`] by consulting `semantic_queries`
/// for `UndeclaredVariable` and `UnquotedBareword` issues.
///
/// # Suppression policy
///
/// ## `UndeclaredVariable` (position-scoped)
///
/// Calls [`SemanticQueries::dynamic_boundary_at`] at the specific byte
/// position of the issue. If the position is covered by dynamic-boundary
/// evidence (e.g. `require $var` at that offset), the diagnostic is suppressed
/// for that specific variable.
///
/// A dynamic construct **elsewhere** in the file does NOT suppress unrelated
/// variables: `require $module; print $undeclared_static_var;` still fires for
/// `$undeclared_static_var` because `dynamic_boundary_at` is position-scoped.
///
/// ## `UnquotedBareword` (file-wide dynamic callable)
///
/// Calls [`SemanticQueries::dynamic_callable_may_be_visible_at`] for the
/// bareword name. Returns `Some` when:
/// - The file has at least one `ImportSpec` with `ImportSymbols::Dynamic`
///   (e.g. `Foo->import(@names)`) — any bareword in the file might be imported.
/// - The file has a `DynamicBoundary` occurrence whose entity name matches the
///   bareword (e.g. `eval "sub NAME { ... }"` — only `NAME` is suppressed).
/// - The symbol is visible from exactly one high-confidence imported,
///   exported, or generated compiler fact. Low-confidence, ambiguous, or
///   dynamic visibility remains diagnosed.
///
/// Non-literal evals, non-Dynamic imports, and genuinely missing subs still fire.
///
/// ## All other issue kinds
///
/// Always emitted as diagnostics (no suppression).
///
/// # Backward compatibility
///
/// The original [`scope_issues_to_diagnostics`] is preserved unchanged.
/// Callers that cannot provide `FileId` or semantic queries should continue
/// using the original function.
///
/// # Requirements
///
/// - **Req 7.4**: Suppress undefined-symbol diagnostics for references within
///   dynamic boundary scopes.
/// - **Req 7.5**: Suppress `UnquotedBareword` diagnostics for barewords
///   plausibly provided by a dynamic import or string-eval sub declaration.
pub fn scope_issues_to_diagnostics_with_semantics<Q: SemanticQueries>(
    issues: Vec<ScopeIssue>,
    file_id: FileId,
    semantic_queries: &Q,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for issue in issues {
        // ── UndeclaredVariable suppression (position-scoped) ──
        //
        // Check whether the specific variable position is covered by a
        // dynamic-boundary occurrence (e.g. `require $var` at that offset).
        // This is deliberately narrow: only positions within the dynamic
        // construct's span are suppressed.
        if issue.kind == IssueKind::UndeclaredVariable {
            let byte_offset = issue.range.0 as u32;
            // Strip the sigil from the variable name to get the bare symbol.
            let bare_symbol = issue.variable_name.trim_start_matches(['$', '@', '%', '&', '*']);
            // Use the full variable name (with sigil) as a fallback too.
            let symbol_to_check =
                if bare_symbol.is_empty() { issue.variable_name.as_str() } else { bare_symbol };

            // Query for dynamic boundary at the specific position for this symbol.
            let is_covered = semantic_queries
                .dynamic_boundary_at(file_id, byte_offset, Some(symbol_to_check))
                .is_some();

            if is_covered {
                // The specific variable at this position is covered by a
                // dynamic boundary — suppress the undefined-symbol diagnostic.
                tracing::debug!(
                    variable = %issue.variable_name,
                    byte_offset,
                    "suppressed UndeclaredVariable diagnostic: covered by dynamic boundary"
                );
                continue;
            }
        }

        // ── UnquotedBareword suppression (file-wide dynamic callable) ──
        //
        // Check whether a dynamic import or eval-sub evidence exists in the
        // file that makes this bareword plausibly visible.
        //
        // Policy differences from UndeclaredVariable:
        // - Coverage is file-wide when a Dynamic import exists (any bareword
        //   in the file might come from an import with unknown symbol list).
        // - Coverage is name-scoped when an eval-sub boundary exists (only
        //   the sub named in the eval string is suppressed).
        //
        // This is intentionally conservative — non-literal evals, non-Dynamic
        // imports, and barewords with genuinely no evidence are still flagged.
        if issue.kind == IssueKind::UnquotedBareword {
            let byte_offset = issue.range.0 as u32;
            // Barewords have no sigil; use the name directly.
            let symbol = issue.variable_name.as_str();

            let is_callable_visible = semantic_queries
                .dynamic_callable_may_be_visible_at(file_id, byte_offset, symbol)
                .is_some();

            if is_callable_visible {
                tracing::debug!(
                    bareword = %issue.variable_name,
                    "suppressed UnquotedBareword diagnostic: dynamic callable evidence present"
                );
                continue;
            }

            let visible_trust =
                visible_symbol_diagnostic_trust(semantic_queries, file_id, byte_offset, symbol);

            if visible_trust == VisibleSymbolDiagnosticTrust::Proven {
                tracing::debug!(
                    bareword = %issue.variable_name,
                    "suppressed UnquotedBareword diagnostic: compiler fact proves imported/generated symbol"
                );
                continue;
            }
        }

        // ── All other issue kinds ── (and unsuppressed UndeclaredVariable /
        // UnquotedBareword) — always emit the diagnostic.
        //
        // This includes: VariableRedeclaration, DuplicateParameter,
        // VariableShadowing, UnusedVariable, ParameterShadowsGlobal,
        // UnusedParameter, UninitializedVariable, CaptureVarWithoutRegexMatch.
        let severity = match issue.kind {
            IssueKind::UndeclaredVariable
            | IssueKind::VariableRedeclaration
            | IssueKind::DuplicateParameter
            | IssueKind::UnquotedBareword => DiagnosticSeverity::Error,
            IssueKind::VariableShadowing
            | IssueKind::UnusedVariable
            | IssueKind::ParameterShadowsGlobal
            | IssueKind::UnusedParameter
            | IssueKind::UninitializedVariable => DiagnosticSeverity::Warning,
            IssueKind::CaptureVarWithoutRegexMatch => DiagnosticSeverity::Information,
        };

        let code = match issue.kind {
            IssueKind::UndeclaredVariable => DiagnosticCode::UndefinedVariable,
            IssueKind::UnusedVariable => DiagnosticCode::UnusedVariable,
            IssueKind::VariableShadowing => DiagnosticCode::VariableShadowing,
            IssueKind::VariableRedeclaration => DiagnosticCode::VariableRedeclaration,
            IssueKind::DuplicateParameter => DiagnosticCode::DuplicateParameter,
            IssueKind::ParameterShadowsGlobal => DiagnosticCode::ParameterShadowsGlobal,
            IssueKind::UnusedParameter => DiagnosticCode::UnusedParameter,
            IssueKind::UnquotedBareword => DiagnosticCode::UnquotedBareword,
            IssueKind::UninitializedVariable => DiagnosticCode::UninitializedVariable,
            IssueKind::CaptureVarWithoutRegexMatch => DiagnosticCode::CaptureVarWithoutRegexMatch,
        };

        let mut related_info = build_scope_related_info(&issue);
        if issue.kind == IssueKind::UnquotedBareword {
            add_visible_symbol_trust_boundary_related_info(
                &issue,
                &mut related_info,
                visible_symbol_diagnostic_trust(
                    semantic_queries,
                    file_id,
                    issue.range.0 as u32,
                    issue.variable_name.as_str(),
                ),
            );
        }
        let suggestion = build_scope_suggestion(&issue);

        diagnostics.push(Diagnostic {
            range: issue.range,
            severity,
            code: Some(code.as_str().to_string()),
            message: build_enhanced_scope_message(&issue),
            related_information: related_info,
            tags: if matches!(issue.kind, IssueKind::UnusedVariable | IssueKind::UnusedParameter) {
                vec![DiagnosticTag::Unnecessary]
            } else {
                Vec::new()
            },
            suggestion,
        });
    }

    diagnostics
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisibleSymbolDiagnosticTrust {
    NoEvidence,
    Proven,
    LowConfidence,
    DynamicBoundary,
    Ambiguous,
}

fn visible_symbol_diagnostic_trust<Q: SemanticQueries>(
    semantic_queries: &Q,
    file_id: FileId,
    byte_offset: u32,
    symbol: &str,
) -> VisibleSymbolDiagnosticTrust {
    let visible = semantic_queries.visible_symbols_at(file_id, byte_offset, None);
    classify_visible_symbol_diagnostic_trust(&visible, symbol)
}

fn classify_visible_symbol_diagnostic_trust(
    visible: &[VisibleSymbol],
    symbol: &str,
) -> VisibleSymbolDiagnosticTrust {
    let matching: Vec<_> = visible.iter().filter(|candidate| candidate.name == symbol).collect();
    if matching.is_empty() {
        return VisibleSymbolDiagnosticTrust::NoEvidence;
    }

    if matching.iter().any(|candidate| candidate.source == VisibleSymbolSource::DynamicUnknown) {
        return VisibleSymbolDiagnosticTrust::DynamicBoundary;
    }

    if matching.iter().any(|candidate| candidate.confidence == Confidence::Low) {
        return VisibleSymbolDiagnosticTrust::LowConfidence;
    }

    if matching.len() > 1 {
        return VisibleSymbolDiagnosticTrust::Ambiguous;
    }

    if is_proven_compiler_visible_symbol(matching[0]) {
        return VisibleSymbolDiagnosticTrust::Proven;
    }

    VisibleSymbolDiagnosticTrust::NoEvidence
}

fn is_proven_compiler_visible_symbol(candidate: &VisibleSymbol) -> bool {
    matches!(
        candidate.source,
        VisibleSymbolSource::ExplicitImport
            | VisibleSymbolSource::DefaultExport
            | VisibleSymbolSource::ExportTag
            | VisibleSymbolSource::Generated
    ) && candidate.confidence == Confidence::High
}

fn add_visible_symbol_trust_boundary_related_info(
    issue: &ScopeIssue,
    related_info: &mut Vec<RelatedInformation>,
    trust: VisibleSymbolDiagnosticTrust,
) {
    let message = match trust {
        VisibleSymbolDiagnosticTrust::NoEvidence | VisibleSymbolDiagnosticTrust::Proven => return,
        VisibleSymbolDiagnosticTrust::LowConfidence => {
            "Trust boundary: only low-confidence symbol evidence exists, so perl-lsp keeps this diagnostic instead of suppressing it."
        }
        VisibleSymbolDiagnosticTrust::DynamicBoundary => {
            "Trust boundary: dynamic Perl evidence exists for this name, so perl-lsp does not treat it as an exact static fact."
        }
        VisibleSymbolDiagnosticTrust::Ambiguous => {
            "Trust boundary: multiple possible symbol facts matched, so perl-lsp keeps this diagnostic instead of guessing."
        }
    };

    related_info.push(RelatedInformation { location: issue.range, message: message.to_string() });
}

/// Build related information for a scope issue (extracted for reuse).
fn build_scope_related_info(issue: &ScopeIssue) -> Vec<RelatedInformation> {
    match issue.kind {
        IssueKind::UndeclaredVariable => vec![
            RelatedInformation {
                location: issue.range,
                message: "💡 Declare the variable with 'my', 'our', 'local', or 'state'".to_string(),
            },
            RelatedInformation {
                location: issue.range,
                message: "ℹ️ Under 'use strict', all variables must be declared before use. Use 'my' for lexical scope or 'our' for package variables.".to_string(),
            }
        ],
        IssueKind::UnusedVariable => vec![
            RelatedInformation {
                location: issue.range,
                message: "💡 Remove the unused variable or prefix with '_' to indicate it's intentionally unused".to_string(),
            }
        ],
        IssueKind::UnusedParameter => vec![
            RelatedInformation {
                location: issue.range,
                message: "💡 Remove the unused parameter or prefix with '_' (e.g., $_unused) to indicate it's intentionally unused".to_string(),
            }
        ],
        IssueKind::VariableShadowing => vec![
            RelatedInformation {
                location: issue.range,
                message: "💡 Rename this variable or use the outer scope variable instead".to_string(),
            },
            RelatedInformation {
                location: issue.range,
                message: "ℹ️ Variable shadowing can make code harder to understand and may hide bugs.".to_string(),
            }
        ],
        IssueKind::VariableRedeclaration => vec![
            RelatedInformation {
                location: issue.range,
                message: "💡 Remove the duplicate 'my' declaration - just assign to the existing variable".to_string(),
            }
        ],
        IssueKind::DuplicateParameter => vec![
            RelatedInformation {
                location: issue.range,
                message: "💡 Remove the duplicate parameter or use a different name".to_string(),
            }
        ],
        IssueKind::ParameterShadowsGlobal => vec![
            RelatedInformation {
                location: issue.range,
                message: "💡 Rename the parameter to avoid shadowing the global variable".to_string(),
            }
        ],
        IssueKind::UninitializedVariable => vec![
            RelatedInformation {
                location: issue.range,
                message: "💡 Initialize the variable when declaring it: my $var = value;".to_string(),
            },
            RelatedInformation {
                location: issue.range,
                message: "ℹ️ Using uninitialized variables may cause warnings and unexpected behavior.".to_string(),
            }
        ],
        IssueKind::UnquotedBareword => vec![
            RelatedInformation {
                location: issue.range,
                message: "💡 Quote the bareword as a string: 'word' or \"word\"".to_string(),
            },
            RelatedInformation {
                location: issue.range,
                message: "ℹ️ Under 'use strict', barewords are not allowed unless they're subroutine calls or hash keys.".to_string(),
            }
        ],
        IssueKind::CaptureVarWithoutRegexMatch => vec![
            RelatedInformation {
                location: issue.range,
                message: "💡 Perform a regex match before using this capture variable: if ($str =~ /(...)/){ ... }".to_string(),
            },
            RelatedInformation {
                location: issue.range,
                message: "ℹ️ Capture variables ($1, $2, etc.) hold the last successful match and may be undef if no match has occurred.".to_string(),
            }
        ],
    }
}

/// Build an enhanced, more helpful message for a scope issue.
///
/// Augments the analyzer's raw description with the variable name and
/// actionable context so users immediately understand what went wrong.
fn build_enhanced_scope_message(issue: &ScopeIssue) -> String {
    let name = &issue.variable_name;
    match issue.kind {
        IssueKind::UndeclaredVariable => {
            format!(
                "Variable '{}' is used but not declared -- add 'my {}' to declare it in this scope",
                name, name
            )
        }
        IssueKind::UnusedVariable => {
            format!(
                "Variable '{}' is declared but never used -- prefix with '_' or remove it",
                name
            )
        }
        IssueKind::UnusedParameter => {
            format!(
                "Parameter '{}' is never used -- prefix with '_' (e.g., $_{}) to suppress this warning",
                name,
                name.trim_start_matches('$')
            )
        }
        IssueKind::VariableShadowing => {
            format!(
                "Variable '{}' shadows an outer declaration -- consider renaming to avoid confusion",
                name
            )
        }
        IssueKind::VariableRedeclaration => {
            format!(
                "Variable '{}' is declared again in the same scope -- remove the duplicate 'my'",
                name
            )
        }
        IssueKind::UninitializedVariable => {
            format!(
                "Variable '{}' is used before being initialized -- assign a value when declaring it",
                name
            )
        }
        IssueKind::UnquotedBareword => {
            format!(
                "Bareword '{}' is not allowed under 'use strict' -- quote it as '{}' or use it as a subroutine call",
                name, name
            )
        }
        // Fall back to the analyzer's original description for other kinds
        _ => issue.description.clone(),
    }
}

/// Build a short actionable fix suggestion for a scope issue.
fn build_scope_suggestion(issue: &ScopeIssue) -> Option<String> {
    let name = &issue.variable_name;
    match issue.kind {
        IssueKind::UndeclaredVariable => Some(format!("Add 'my {};' before this line", name)),
        IssueKind::UnusedVariable => Some(format!("Prefix as '_{}'", name.trim_start_matches('$'))),
        IssueKind::UnusedParameter => {
            Some(format!("Rename to '$_{}'", name.trim_start_matches('$')))
        }
        IssueKind::VariableRedeclaration => Some("Remove the duplicate 'my' keyword".to_string()),
        IssueKind::UninitializedVariable => Some(format!("Initialize: my {} = ...;", name)),
        IssueKind::UnquotedBareword => {
            Some(format!("Quote as '{}' or use qw({}) for lists", name, name))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_semantic_facts::{
        AnchorId, Confidence, DefinitionCandidate, EntityFact, EntityId, FileId, OccurrenceFact,
        OccurrenceId, OccurrenceKind, Provenance, RenamePlan, SafeDeletePlan, ScopeId,
        VisibleSymbol, VisibleSymbolSource,
    };
    use perl_workspace::semantic::queries::{
        DynamicCallableEvidence, QueryContext, SemanticQueries,
    };

    // ── Stub that simulates dynamic boundary coverage at any position ──

    struct DynamicBoundaryStubQueries;

    impl SemanticQueries for DynamicBoundaryStubQueries {
        fn symbol_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
        ) -> Option<(EntityFact, OccurrenceFact)> {
            None
        }

        fn definitions(&self, _symbol: &str, _context: &QueryContext) -> Vec<DefinitionCandidate> {
            Vec::new()
        }

        fn references(&self, _entity_id: EntityId) -> Vec<OccurrenceFact> {
            Vec::new()
        }

        fn visible_symbols_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _scope_id: Option<ScopeId>,
        ) -> Vec<VisibleSymbol> {
            Vec::new()
        }

        fn method_candidates(
            &self,
            _receiver_package: &str,
            _method_name: &str,
        ) -> Vec<DefinitionCandidate> {
            Vec::new()
        }

        fn rename_plan(&self, entity_id: EntityId, new_name: &str) -> RenamePlan {
            RenamePlan::new(entity_id, String::new(), new_name.to_string(), vec![], vec![], vec![])
        }

        fn safe_delete_plan(&self, entity_id: EntityId) -> SafeDeletePlan {
            SafeDeletePlan::new(entity_id, String::new(), vec![], vec![])
        }

        fn dynamic_boundary_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _symbol: Option<&str>,
        ) -> Option<OccurrenceFact> {
            // Simulates full file dynamic coverage: any position is covered.
            Some(OccurrenceFact {
                id: OccurrenceId(8888),
                kind: OccurrenceKind::DynamicBoundary,
                entity_id: None,
                anchor_id: AnchorId(8888),
                scope_id: None,
                provenance: Provenance::DynamicBoundary,
                confidence: Confidence::Low,
            })
        }

        fn dynamic_callable_may_be_visible_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _symbol: &str,
        ) -> Option<DynamicCallableEvidence> {
            None
        }
    }

    /// No-op stub — no dynamic boundary coverage anywhere.
    struct NullStubQueries;

    impl SemanticQueries for NullStubQueries {
        fn symbol_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
        ) -> Option<(EntityFact, OccurrenceFact)> {
            None
        }

        fn definitions(&self, _symbol: &str, _context: &QueryContext) -> Vec<DefinitionCandidate> {
            Vec::new()
        }

        fn references(&self, _entity_id: EntityId) -> Vec<OccurrenceFact> {
            Vec::new()
        }

        fn visible_symbols_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _scope_id: Option<ScopeId>,
        ) -> Vec<VisibleSymbol> {
            Vec::new()
        }

        fn method_candidates(
            &self,
            _receiver_package: &str,
            _method_name: &str,
        ) -> Vec<DefinitionCandidate> {
            Vec::new()
        }

        fn rename_plan(&self, entity_id: EntityId, new_name: &str) -> RenamePlan {
            RenamePlan::new(entity_id, String::new(), new_name.to_string(), vec![], vec![], vec![])
        }

        fn safe_delete_plan(&self, entity_id: EntityId) -> SafeDeletePlan {
            SafeDeletePlan::new(entity_id, String::new(), vec![], vec![])
        }

        fn dynamic_boundary_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _symbol: Option<&str>,
        ) -> Option<OccurrenceFact> {
            None
        }

        fn dynamic_callable_may_be_visible_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _symbol: &str,
        ) -> Option<DynamicCallableEvidence> {
            None
        }
    }

    // ── Helper ──

    fn undeclared_issue(name: &str, range: (usize, usize)) -> ScopeIssue {
        ScopeIssue {
            kind: IssueKind::UndeclaredVariable,
            variable_name: name.to_string(),
            line: 1,
            range,
            description: format!("Variable '{}' not declared", name),
        }
    }

    fn unused_issue(name: &str, range: (usize, usize)) -> ScopeIssue {
        ScopeIssue {
            kind: IssueKind::UnusedVariable,
            variable_name: name.to_string(),
            line: 1,
            range,
            description: format!("Variable '{}' unused", name),
        }
    }

    fn bareword_issue(name: &str, range: (usize, usize)) -> ScopeIssue {
        ScopeIssue {
            kind: IssueKind::UnquotedBareword,
            variable_name: name.to_string(),
            line: 1,
            range,
            description: format!("Bareword '{}' not allowed under 'use strict'", name),
        }
    }

    /// Stub that returns `Some` from `dynamic_callable_may_be_visible_at`
    /// for any symbol (simulates a file with a dynamic import or eval sub).
    struct DynamicCallableStubQueries;

    impl SemanticQueries for DynamicCallableStubQueries {
        fn symbol_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
        ) -> Option<(EntityFact, OccurrenceFact)> {
            None
        }

        fn definitions(&self, _symbol: &str, _context: &QueryContext) -> Vec<DefinitionCandidate> {
            Vec::new()
        }

        fn references(&self, _entity_id: EntityId) -> Vec<OccurrenceFact> {
            Vec::new()
        }

        fn visible_symbols_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _scope_id: Option<ScopeId>,
        ) -> Vec<VisibleSymbol> {
            Vec::new()
        }

        fn method_candidates(
            &self,
            _receiver_package: &str,
            _method_name: &str,
        ) -> Vec<DefinitionCandidate> {
            Vec::new()
        }

        fn rename_plan(&self, entity_id: EntityId, new_name: &str) -> RenamePlan {
            RenamePlan::new(entity_id, String::new(), new_name.to_string(), vec![], vec![], vec![])
        }

        fn safe_delete_plan(&self, entity_id: EntityId) -> SafeDeletePlan {
            SafeDeletePlan::new(entity_id, String::new(), vec![], vec![])
        }

        fn dynamic_boundary_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _symbol: Option<&str>,
        ) -> Option<OccurrenceFact> {
            // No variable boundary coverage (this stub is for callables only).
            None
        }

        fn dynamic_callable_may_be_visible_at(
            &self,
            file_id: FileId,
            _byte_offset: u32,
            _symbol: &str,
        ) -> Option<DynamicCallableEvidence> {
            // Simulates a file with a dynamic import: any bareword might be visible.
            Some(DynamicCallableEvidence::DynamicImport {
                file_id,
                anchor_id: Some(AnchorId(6666)),
                module: "StubModule".to_string(),
            })
        }
    }

    struct VisibleSymbolsStubQueries {
        visible_symbols: Vec<VisibleSymbol>,
    }

    impl SemanticQueries for VisibleSymbolsStubQueries {
        fn symbol_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
        ) -> Option<(EntityFact, OccurrenceFact)> {
            None
        }

        fn definitions(&self, _symbol: &str, _context: &QueryContext) -> Vec<DefinitionCandidate> {
            Vec::new()
        }

        fn references(&self, _entity_id: EntityId) -> Vec<OccurrenceFact> {
            Vec::new()
        }

        fn visible_symbols_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _scope_id: Option<ScopeId>,
        ) -> Vec<VisibleSymbol> {
            self.visible_symbols.clone()
        }

        fn method_candidates(
            &self,
            _receiver_package: &str,
            _method_name: &str,
        ) -> Vec<DefinitionCandidate> {
            Vec::new()
        }

        fn rename_plan(&self, entity_id: EntityId, new_name: &str) -> RenamePlan {
            RenamePlan::new(entity_id, String::new(), new_name.to_string(), vec![], vec![], vec![])
        }

        fn safe_delete_plan(&self, entity_id: EntityId) -> SafeDeletePlan {
            SafeDeletePlan::new(entity_id, String::new(), vec![], vec![])
        }

        fn dynamic_boundary_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _symbol: Option<&str>,
        ) -> Option<OccurrenceFact> {
            None
        }

        fn dynamic_callable_may_be_visible_at(
            &self,
            _file_id: FileId,
            _byte_offset: u32,
            _symbol: &str,
        ) -> Option<DynamicCallableEvidence> {
            None
        }
    }

    fn visible_symbol(
        name: &str,
        source: VisibleSymbolSource,
        confidence: Confidence,
    ) -> VisibleSymbol {
        VisibleSymbol {
            name: name.to_string(),
            entity_id: Some(EntityId(9001)),
            source,
            confidence,
            context: None,
        }
    }

    fn has_related_info_containing(diagnostic: &Diagnostic, needle: &str) -> bool {
        diagnostic.related_information.iter().any(|info| info.message.contains(needle))
    }

    // ── Tests ──

    #[test]
    fn suppresses_undeclared_variable_when_covered_by_dynamic_boundary()
    -> Result<(), Box<dyn std::error::Error>> {
        let issues = vec![undeclared_issue("$foo", (10, 14))];
        let queries = DynamicBoundaryStubQueries;

        let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, FileId(1), &queries);

        assert!(
            diagnostics.is_empty(),
            "UndeclaredVariable covered by dynamic boundary should be suppressed"
        );
        Ok(())
    }

    #[test]
    fn does_not_suppress_undeclared_variable_when_not_covered()
    -> Result<(), Box<dyn std::error::Error>> {
        let issues = vec![undeclared_issue("$foo", (10, 14))];
        let queries = NullStubQueries;

        let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, FileId(1), &queries);

        assert_eq!(
            diagnostics.len(),
            1,
            "UndeclaredVariable NOT covered by dynamic boundary should still fire"
        );
        Ok(())
    }

    #[test]
    fn does_not_suppress_unused_variable_even_in_dynamic_scope()
    -> Result<(), Box<dyn std::error::Error>> {
        // Non-UndeclaredVariable issues are never suppressed by dynamic boundary.
        let issues = vec![unused_issue("$bar", (20, 24))];
        let queries = DynamicBoundaryStubQueries;

        let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, FileId(1), &queries);

        assert_eq!(
            diagnostics.len(),
            1,
            "UnusedVariable should NOT be suppressed by dynamic boundary"
        );
        Ok(())
    }

    #[test]
    fn suppresses_only_dynamic_not_static_variable_in_same_file()
    -> Result<(), Box<dyn std::error::Error>> {
        // This tests the issue-local suppression contract (Q1):
        // when NullStubQueries is used, nothing is suppressed even if a
        // dynamic construct exists "nearby" in the file.
        // DynamicBoundaryStubQueries suppresses ALL positions — to test
        // selective suppression, we use a position-aware stub.
        struct PositionAwareStub;
        impl SemanticQueries for PositionAwareStub {
            fn symbol_at(&self, _: FileId, _: u32) -> Option<(EntityFact, OccurrenceFact)> {
                None
            }
            fn definitions(&self, _: &str, _: &QueryContext) -> Vec<DefinitionCandidate> {
                Vec::new()
            }
            fn references(&self, _: EntityId) -> Vec<OccurrenceFact> {
                Vec::new()
            }
            fn visible_symbols_at(
                &self,
                _: FileId,
                _: u32,
                _: Option<ScopeId>,
            ) -> Vec<VisibleSymbol> {
                Vec::new()
            }
            fn method_candidates(&self, _: &str, _: &str) -> Vec<DefinitionCandidate> {
                Vec::new()
            }
            fn rename_plan(&self, id: EntityId, n: &str) -> RenamePlan {
                RenamePlan::new(id, String::new(), n.to_string(), vec![], vec![], vec![])
            }
            fn safe_delete_plan(&self, id: EntityId) -> SafeDeletePlan {
                SafeDeletePlan::new(id, String::new(), vec![], vec![])
            }
            fn dynamic_boundary_at(
                &self,
                _: FileId,
                byte_offset: u32,
                _: Option<&str>,
            ) -> Option<OccurrenceFact> {
                // Only cover positions 10..30.
                if byte_offset >= 10 && byte_offset < 30 {
                    Some(OccurrenceFact {
                        id: OccurrenceId(7777),
                        kind: OccurrenceKind::DynamicBoundary,
                        entity_id: None,
                        anchor_id: AnchorId(7777),
                        scope_id: None,
                        provenance: Provenance::DynamicBoundary,
                        confidence: Confidence::Low,
                    })
                } else {
                    None
                }
            }

            fn dynamic_callable_may_be_visible_at(
                &self,
                _: FileId,
                _: u32,
                _: &str,
            ) -> Option<DynamicCallableEvidence> {
                None
            }
        }

        let dynamic_var = undeclared_issue("$dynamic_var", (15, 27)); // covered (15 < 30)
        let static_var = undeclared_issue("$static_var", (50, 61)); // NOT covered (50 >= 30)
        let issues = vec![dynamic_var, static_var];

        let diagnostics =
            scope_issues_to_diagnostics_with_semantics(issues, FileId(1), &PositionAwareStub);

        assert_eq!(
            diagnostics.len(),
            1,
            "Only the static_var diagnostic should fire; dynamic_var should be suppressed"
        );
        assert!(
            diagnostics[0].message.contains("static_var"),
            "The remaining diagnostic should be for $static_var, got: {:?}",
            diagnostics[0].message
        );
        Ok(())
    }

    // ── UnquotedBareword suppression tests (PR-B) ──
    //
    // Cases 2 and 3 from the spec:
    //   2. `Foo->import(@names); bar()` — dynamic import, bareword should be suppressed
    //   3. `eval "sub generated_from_string { 1 }"; generated_from_string()` — suppressed
    //
    // Controls:
    //   4. `$undeclared_static_var` — UndeclaredVariable must still fire
    //   5. `truly_undefined_sub()` — UnquotedBareword with no dynamic evidence must still fire
    //   6. eval defines NAME but other bareword is unrelated — only NAME suppressed

    #[test]
    fn suppresses_unquoted_bareword_when_dynamic_callable_may_be_visible()
    -> Result<(), Box<dyn std::error::Error>> {
        // Case 2/3: dynamic import or eval-sub evidence → suppress UnquotedBareword.
        // DynamicCallableStubQueries returns Some for dynamic_callable_may_be_visible_at.
        let issues = vec![bareword_issue("bar", (20, 23))];
        let queries = DynamicCallableStubQueries;

        let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, FileId(1), &queries);

        assert!(
            diagnostics.is_empty(),
            "UnquotedBareword covered by dynamic callable evidence should be suppressed"
        );
        Ok(())
    }

    #[test]
    fn does_not_suppress_unquoted_bareword_when_no_dynamic_evidence()
    -> Result<(), Box<dyn std::error::Error>> {
        // Case 5: no dynamic evidence → UnquotedBareword must still fire.
        // NullStubQueries returns None for all queries.
        let issues = vec![bareword_issue("truly_undefined_sub", (10, 29))];
        let queries = NullStubQueries;

        let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, FileId(1), &queries);

        assert_eq!(
            diagnostics.len(),
            1,
            "UnquotedBareword with no dynamic evidence must still fire"
        );
        Ok(())
    }

    #[test]
    fn suppresses_unquoted_bareword_when_high_confidence_import_fact_exists()
    -> Result<(), Box<dyn std::error::Error>> {
        let issues = vec![bareword_issue("imported_func", (10, 23))];
        let queries = VisibleSymbolsStubQueries {
            visible_symbols: vec![visible_symbol(
                "imported_func",
                VisibleSymbolSource::ExplicitImport,
                Confidence::High,
            )],
        };

        let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, FileId(1), &queries);

        assert!(
            diagnostics.is_empty(),
            "high-confidence import/export compiler fact should suppress imported bareword"
        );
        Ok(())
    }

    #[test]
    fn suppresses_unquoted_bareword_when_high_confidence_export_visibility_exists()
    -> Result<(), Box<dyn std::error::Error>> {
        for source in [VisibleSymbolSource::DefaultExport, VisibleSymbolSource::ExportTag] {
            let issues = vec![bareword_issue("exported_func", (10, 23))];
            let queries = VisibleSymbolsStubQueries {
                visible_symbols: vec![visible_symbol(
                    "exported_func",
                    source.clone(),
                    Confidence::High,
                )],
            };

            let diagnostics =
                scope_issues_to_diagnostics_with_semantics(issues, FileId(1), &queries);

            assert!(
                diagnostics.is_empty(),
                "high-confidence {source:?} visibility should suppress exported bareword"
            );
        }
        Ok(())
    }

    #[test]
    fn suppresses_unquoted_bareword_when_high_confidence_framework_fact_exists()
    -> Result<(), Box<dyn std::error::Error>> {
        let issues = vec![bareword_issue("generated_member", (10, 26))];
        let queries = VisibleSymbolsStubQueries {
            visible_symbols: vec![visible_symbol(
                "generated_member",
                VisibleSymbolSource::Generated,
                Confidence::High,
            )],
        };

        let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, FileId(1), &queries);

        assert!(
            diagnostics.is_empty(),
            "high-confidence framework compiler fact should suppress generated bareword"
        );
        Ok(())
    }

    #[test]
    fn low_confidence_import_fact_does_not_suppress_unquoted_bareword()
    -> Result<(), Box<dyn std::error::Error>> {
        let issues = vec![bareword_issue("maybe_imported_func", (10, 29))];
        let queries = VisibleSymbolsStubQueries {
            visible_symbols: vec![visible_symbol(
                "maybe_imported_func",
                VisibleSymbolSource::ExplicitImport,
                Confidence::Low,
            )],
        };

        let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, FileId(1), &queries);

        assert_eq!(
            diagnostics.len(),
            1,
            "low-confidence import/export fact should not silently suppress diagnostics"
        );
        assert!(
            has_related_info_containing(&diagnostics[0], "low-confidence symbol evidence"),
            "low-confidence diagnostic should carry an explicit trust-boundary label: {diagnostics:?}"
        );
        Ok(())
    }

    #[test]
    fn dynamic_boundary_candidate_blocks_compiler_fact_suppression()
    -> Result<(), Box<dyn std::error::Error>> {
        let issues = vec![bareword_issue("dynamic_symbol", (10, 24))];
        let queries = VisibleSymbolsStubQueries {
            visible_symbols: vec![
                visible_symbol(
                    "dynamic_symbol",
                    VisibleSymbolSource::DynamicUnknown,
                    Confidence::Low,
                ),
                visible_symbol(
                    "dynamic_symbol",
                    VisibleSymbolSource::ExplicitImport,
                    Confidence::High,
                ),
            ],
        };

        let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, FileId(1), &queries);

        assert_eq!(
            diagnostics.len(),
            1,
            "dynamic-boundary candidate should block imported/generated fact suppression"
        );
        assert!(
            has_related_info_containing(&diagnostics[0], "dynamic Perl evidence"),
            "dynamic-boundary diagnostic should carry an explicit trust-boundary label: {diagnostics:?}"
        );
        Ok(())
    }

    #[test]
    fn ambiguous_import_visibility_does_not_expand_live_bareword_cutover()
    -> Result<(), Box<dyn std::error::Error>> {
        let issues = vec![bareword_issue("ambiguous_sub", (10, 23))];
        let queries = VisibleSymbolsStubQueries {
            visible_symbols: vec![
                visible_symbol(
                    "ambiguous_sub",
                    VisibleSymbolSource::ExplicitImport,
                    Confidence::High,
                ),
                visible_symbol(
                    "ambiguous_sub",
                    VisibleSymbolSource::DefaultExport,
                    Confidence::High,
                ),
            ],
        };

        let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, FileId(1), &queries);

        assert_eq!(
            diagnostics.len(),
            1,
            "ambiguous imported/generated visibility should not silently suppress diagnostics"
        );
        assert!(
            has_related_info_containing(&diagnostics[0], "multiple possible symbol facts"),
            "ambiguous diagnostic should carry an explicit trust-boundary label: {diagnostics:?}"
        );
        Ok(())
    }

    #[test]
    fn undeclared_variable_still_fires_when_only_callable_evidence_present()
    -> Result<(), Box<dyn std::error::Error>> {
        // Case 4: UndeclaredVariable is suppressed by dynamic_boundary_at (position-based),
        // NOT by dynamic_callable_may_be_visible_at. When only callable evidence is
        // present (DynamicCallableStubQueries returns None from dynamic_boundary_at),
        // UndeclaredVariable must still fire.
        let issues = vec![undeclared_issue("$undeclared_static_var", (10, 32))];
        let queries = DynamicCallableStubQueries;

        let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, FileId(1), &queries);

        assert_eq!(
            diagnostics.len(),
            1,
            "UndeclaredVariable must not be suppressed by callable evidence (wrong query)"
        );
        Ok(())
    }

    #[test]
    fn eval_sub_boundary_suppresses_named_sub_not_other_barewords()
    -> Result<(), Box<dyn std::error::Error>> {
        // Case 6: eval defines NAME → NAME suppressed, other bareword still fires.
        // Use a stub that suppresses "generated_from_string" but not "truly_undefined_sub".
        struct NamedEvalSubStub;
        impl SemanticQueries for NamedEvalSubStub {
            fn symbol_at(&self, _: FileId, _: u32) -> Option<(EntityFact, OccurrenceFact)> {
                None
            }
            fn definitions(&self, _: &str, _: &QueryContext) -> Vec<DefinitionCandidate> {
                Vec::new()
            }
            fn references(&self, _: EntityId) -> Vec<OccurrenceFact> {
                Vec::new()
            }
            fn visible_symbols_at(
                &self,
                _: FileId,
                _: u32,
                _: Option<ScopeId>,
            ) -> Vec<VisibleSymbol> {
                Vec::new()
            }
            fn method_candidates(&self, _: &str, _: &str) -> Vec<DefinitionCandidate> {
                Vec::new()
            }
            fn rename_plan(&self, id: EntityId, n: &str) -> RenamePlan {
                RenamePlan::new(id, String::new(), n.to_string(), vec![], vec![], vec![])
            }
            fn safe_delete_plan(&self, id: EntityId) -> SafeDeletePlan {
                SafeDeletePlan::new(id, String::new(), vec![], vec![])
            }
            fn dynamic_boundary_at(
                &self,
                _: FileId,
                _: u32,
                _: Option<&str>,
            ) -> Option<OccurrenceFact> {
                None
            }
            fn dynamic_callable_may_be_visible_at(
                &self,
                _: FileId,
                _: u32,
                symbol: &str,
            ) -> Option<DynamicCallableEvidence> {
                // Only suppress the named sub from the eval string.
                if symbol == "generated_from_string" {
                    Some(DynamicCallableEvidence::EvalSub {
                        occurrence: OccurrenceFact {
                            id: OccurrenceId(5555),
                            kind: OccurrenceKind::DynamicBoundary,
                            entity_id: None,
                            anchor_id: AnchorId(5555),
                            scope_id: None,
                            provenance: Provenance::DynamicBoundary,
                            confidence: Confidence::Low,
                        },
                    })
                } else {
                    None
                }
            }
        }

        let generated = bareword_issue("generated_from_string", (10, 31));
        let unrelated = bareword_issue("truly_undefined_sub", (40, 59));
        let issues = vec![generated, unrelated];

        let diagnostics =
            scope_issues_to_diagnostics_with_semantics(issues, FileId(1), &NamedEvalSubStub);

        assert_eq!(
            diagnostics.len(),
            1,
            "Only 'truly_undefined_sub' should fire; 'generated_from_string' should be suppressed"
        );
        assert!(
            diagnostics[0].message.contains("truly_undefined_sub"),
            "The remaining diagnostic should be for 'truly_undefined_sub', got: {:?}",
            diagnostics[0].message
        );
        Ok(())
    }

    #[test]
    fn other_issue_kinds_never_suppressed_by_dynamic_callable_evidence()
    -> Result<(), Box<dyn std::error::Error>> {
        // Non-UndeclaredVariable, non-UnquotedBareword issues are never suppressed.
        let issues = vec![unused_issue("$bar", (20, 24))];
        let queries = DynamicCallableStubQueries;

        let diagnostics = scope_issues_to_diagnostics_with_semantics(issues, FileId(1), &queries);

        assert_eq!(
            diagnostics.len(),
            1,
            "UnusedVariable should NOT be suppressed by dynamic callable evidence"
        );
        Ok(())
    }
}
