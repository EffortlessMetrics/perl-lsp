//! Scope analyzer issue to diagnostic conversion
//!
//! This module provides functionality for converting scope analyzer issues
//! into diagnostic messages with pragma-aware severity mapping.

use perl_diagnostics::codes::DiagnosticCode;
use perl_semantic_analyzer::scope_analyzer::{IssueKind, ScopeIssue};
use perl_semantic_facts::FileId;
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
/// Preserved for callers that do not have semantic query data. Internally,
/// [`scope_issues_to_diagnostics_with_semantics`] is used with
/// `NullSemanticQueries`, which is functionally equivalent to this function.
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
/// for each `UndeclaredVariable` issue. If the issue's position is covered by
/// dynamic-boundary evidence (e.g., `require $var`, string `eval`, typeglob
/// assignment, or AUTOLOAD scope), the diagnostic is **suppressed** for that
/// specific variable.
///
/// # Suppression policy (Q3 architectural decision)
///
/// - High-confidence normal missing symbol → diagnostic still fires.
/// - Dynamic boundary covering THE specific variable → suppress that exact
///   undefined diagnostic.
/// - Ambiguous but not dynamic → emit the diagnostic (conservative default).
/// - Unavailable semantic path (no shard for file) → fall back to emitting
///   the diagnostic (no false suppression).
///
/// Importantly, a dynamic construct anywhere in the file does **not** suppress
/// unrelated diagnostics: `require $module; print $undeclared_static_var;`
/// still fires for `$undeclared_static_var` because `dynamic_boundary_at`
/// checks the *specific position* of each issue.
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
pub fn scope_issues_to_diagnostics_with_semantics<Q: SemanticQueries>(
    issues: Vec<ScopeIssue>,
    file_id: FileId,
    semantic_queries: &Q,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for issue in issues {
        // For UndeclaredVariable issues, check whether the specific variable
        // position is covered by dynamic-boundary evidence.
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

        // All other issue kinds — and UndeclaredVariable issues not covered by
        // a dynamic boundary — are emitted as diagnostics.
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
        VisibleSymbol,
    };
    use perl_workspace::semantic::queries::{QueryContext, SemanticQueries};

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
}
