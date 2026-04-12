//! Pull-based diagnostics support (LSP 3.17).

use std::collections::HashMap;

use lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticRelatedInformation,
    DiagnosticSeverity as LspDiagnosticSeverity, DiagnosticTag as LspDiagnosticTag,
    DocumentDiagnosticReport, FullDocumentDiagnosticReport, Location, NumberOrString, Position,
    Range, RelatedFullDocumentDiagnosticReport, RelatedUnchangedDocumentDiagnosticReport,
    UnchangedDocumentDiagnosticReport, Uri, WorkspaceDiagnosticReport,
    WorkspaceDiagnosticReportPartialResult, WorkspaceDocumentDiagnosticReport,
    WorkspaceFullDocumentDiagnosticReport, WorkspaceUnchangedDocumentDiagnosticReport,
};

use serde::{Deserialize, Serialize};

use crate::state::DocumentState;
use crate::util::uri::parse_uri;
use perl_diagnostics_codes::DiagnosticCode;
use perl_lsp_diagnostics::{parse_error_code, parse_error_severity};
use perl_parser::error::ParseError;
use perl_parser::position::offset_to_utf16_line_col;
use perl_parser::util::code_slice;
use perl_parser::Parser;

// Import core diagnostics types from perl-lsp-providers (via parent module re-export)
use super::{
    Diagnostic as InternalDiagnostic, DiagnosticSeverity as InternalDiagnosticSeverity,
    DiagnosticTag as InternalDiagnosticTag, DiagnosticsProvider, RelatedInformation,
};

/// Provider for pull-based diagnostics (LSP 3.17).
pub struct PullDiagnosticsProvider;

impl PullDiagnosticsProvider {
    /// Create a new pull diagnostics provider.
    pub fn new() -> Self {
        Self
    }

    /// Handle textDocument/diagnostic request.
    pub fn get_document_diagnostics(
        &self,
        uri: &Uri,
        content: &str,
        previous_result_id: Option<String>,
    ) -> DocumentDiagnosticReport {
        let result_id = format!("{:x}", md5::compute(content));
        if previous_result_id.as_deref() == Some(&result_id) {
            return self.build_unchanged_report(result_id);
        }

        let diagnostics = self.collect_diagnostics_for_text(uri, content);
        self.build_full_report(result_id, diagnostics)
    }

    /// Handle workspace/diagnostic request.
    pub fn get_workspace_diagnostics(
        &self,
        documents: &HashMap<String, DocumentState>,
        previous_result_ids: Vec<(Uri, String)>,
    ) -> WorkspaceDiagnosticReport {
        let mut items = Vec::new();
        let prev_ids: HashMap<Uri, String> = previous_result_ids.into_iter().collect();

        for (uri_str, doc_state) in documents {
            let uri = parse_uri(uri_str);
            let prev_id = prev_ids.get(&uri).cloned();

            let result_id = format!("{:x}", md5::compute(&doc_state.text));
            let report = if prev_id.as_deref() == Some(&result_id) {
                self.build_unchanged_report(result_id)
            } else {
                let diagnostics = self.collect_diagnostics_for_state(&uri, doc_state);
                self.build_full_report(result_id, diagnostics)
            };

            items.push(self.to_workspace_report(uri, Some(doc_state.version), report));
        }

        WorkspaceDiagnosticReport { items }
    }

    /// Handle workspace/diagnostic partial result.
    pub fn get_workspace_diagnostics_partial(
        &self,
        documents: &[(String, String)],
        batch_size: usize,
    ) -> Vec<WorkspaceDiagnosticReportPartialResult> {
        let mut results = Vec::new();

        for chunk in documents.chunks(batch_size) {
            let mut items = Vec::new();

            for (uri_str, content) in chunk {
                let uri = parse_uri(uri_str);
                let result_id = format!("{:x}", md5::compute(content));
                let diagnostics = self.collect_diagnostics_for_text(&uri, content);
                let report = self.build_full_report(result_id, diagnostics);

                items.push(self.to_workspace_report(uri, None, report));
            }

            results.push(WorkspaceDiagnosticReportPartialResult { items });
        }

        results
    }

    fn collect_diagnostics_for_text(&self, uri: &Uri, content: &str) -> Vec<LspDiagnostic> {
        let code_text = code_slice(content);
        let mut parser = Parser::new(code_text);

        match parser.parse() {
            Ok(ast) => {
                // Retrieve any collected parse errors from error recovery
                let parse_errors: Vec<ParseError> = parser.errors().to_vec();
                let ast = std::sync::Arc::new(ast);
                let provider = DiagnosticsProvider::new(&ast, content.to_string());
                let uri_str = uri.to_string();
                let source_path = url::Url::parse(&uri_str)
                    .map_err(|e| {
                        tracing::warn!(uri = %uri_str, error = %e, "pull diagnostics: failed to parse URI");
                    })
                    .ok()
                    .and_then(|value| {
                        value.to_file_path().map_err(|()| {
                            tracing::warn!(uri = %uri_str, "pull diagnostics: URI is not a file path");
                        }).ok()
                    });
                provider
                    .get_diagnostics_with_path(
                        &ast,
                        &parse_errors,
                        content,
                        None,
                        &[],
                        source_path.as_deref(),
                    )
                    .into_iter()
                    .map(|d| self.to_lsp_diagnostic(uri, content, d))
                    .collect()
            }
            Err(error) => vec![self.parse_error_to_diagnostic(uri, content, &error)],
        }
    }

    fn collect_diagnostics_for_state(
        &self,
        uri: &Uri,
        doc_state: &DocumentState,
    ) -> Vec<LspDiagnostic> {
        if let Some(ast) = &doc_state.ast {
            let provider = DiagnosticsProvider::new(ast, doc_state.text.clone());
            let source_path =
                url::Url::parse(&uri.to_string()).ok().and_then(|value| value.to_file_path().ok());
            provider
                .get_diagnostics_with_path(
                    ast,
                    &doc_state.parse_errors,
                    &doc_state.text,
                    None,
                    &[],
                    source_path.as_deref(),
                )
                .into_iter()
                .map(|d| self.to_lsp_diagnostic(uri, &doc_state.text, d))
                .collect()
        } else if doc_state.parse_errors.is_empty() {
            Vec::new()
        } else {
            doc_state
                .parse_errors
                .iter()
                .map(|error| self.parse_error_to_diagnostic(uri, &doc_state.text, error))
                .collect()
        }
    }

    fn build_unchanged_report(&self, result_id: String) -> DocumentDiagnosticReport {
        DocumentDiagnosticReport::Unchanged(RelatedUnchangedDocumentDiagnosticReport {
            related_documents: None,
            unchanged_document_diagnostic_report: UnchangedDocumentDiagnosticReport { result_id },
        })
    }

    fn build_full_report(
        &self,
        result_id: String,
        diagnostics: Vec<LspDiagnostic>,
    ) -> DocumentDiagnosticReport {
        DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
            related_documents: None,
            full_document_diagnostic_report: FullDocumentDiagnosticReport {
                result_id: Some(result_id),
                items: diagnostics,
            },
        })
    }

    fn to_workspace_report(
        &self,
        uri: Uri,
        version: Option<i32>,
        report: DocumentDiagnosticReport,
    ) -> WorkspaceDocumentDiagnosticReport {
        let version = version.map(i64::from);

        match report {
            DocumentDiagnosticReport::Full(full) => {
                let RelatedFullDocumentDiagnosticReport { full_document_diagnostic_report, .. } =
                    full;
                WorkspaceDocumentDiagnosticReport::Full(WorkspaceFullDocumentDiagnosticReport {
                    uri,
                    version,
                    full_document_diagnostic_report,
                })
            }
            DocumentDiagnosticReport::Unchanged(unchanged) => {
                let RelatedUnchangedDocumentDiagnosticReport {
                    unchanged_document_diagnostic_report,
                    ..
                } = unchanged;
                WorkspaceDocumentDiagnosticReport::Unchanged(
                    WorkspaceUnchangedDocumentDiagnosticReport {
                        uri,
                        version,
                        unchanged_document_diagnostic_report,
                    },
                )
            }
        }
    }

    fn to_lsp_diagnostic(
        &self,
        uri: &Uri,
        text: &str,
        diagnostic: InternalDiagnostic,
    ) -> LspDiagnostic {
        let range = lsp_range_from_offsets(text, diagnostic.range.0, diagnostic.range.1);
        let severity = Some(to_lsp_severity(diagnostic.severity));
        let code = diagnostic.code.map(NumberOrString::String);
        let related_information =
            to_lsp_related_information(uri, text, &diagnostic.related_information);

        // Collect tag strings before diagnostic is partially moved by the suggestion match
        let tag_strings: Vec<String> = diagnostic
            .tags
            .iter()
            .map(|t| match t {
                InternalDiagnosticTag::Unnecessary => "Unnecessary".to_string(),
                InternalDiagnosticTag::Deprecated => "Deprecated".to_string(),
            })
            .collect();
        let tags = to_lsp_tags(&diagnostic.tags);

        // Append the suggestion to the message when present so users see it inline
        let message = match diagnostic.suggestion {
            Some(ref suggestion) => format!("{}\nSuggestion: {}", diagnostic.message, suggestion),
            None => diagnostic.message,
        };

        let data = code.as_ref().and_then(|c| {
            if let NumberOrString::String(code_str) = c {
                let category = DiagnosticCode::parse_code(code_str)
                    .map(|dc| format!("{:?}", dc.category()))
                    .unwrap_or_else(|| "Other".to_string());
                let fixable = is_fixable_diagnostic(code_str);
                serde_json::to_value(DiagnosticData {
                    code: code_str.clone(),
                    category,
                    fixable,
                    tags: tag_strings,
                })
                .ok()
            } else {
                None
            }
        });

        LspDiagnostic {
            range,
            severity,
            code,
            code_description: None,
            source: Some("perl-lsp".to_string()),
            message,
            related_information,
            tags,
            data,
        }
    }

    fn parse_error_to_diagnostic(
        &self,
        uri: &Uri,
        text: &str,
        error: &ParseError,
    ) -> LspDiagnostic {
        let (offset, base_message) = match error {
            ParseError::UnexpectedToken { location, expected, found } => {
                (*location, format!("Expected {expected}, found {found}"))
            }
            ParseError::SyntaxError { location, message } => (*location, message.clone()),
            ParseError::UnexpectedEof => (text.len(), "Unexpected end of input".to_string()),
            ParseError::LexerError { message } => (0, message.clone()),
            _ => (0, error.to_string()),
        };

        // Append the suggestion inline so users see actionable hints in the fallback path,
        // matching the behaviour of to_lsp_diagnostic for the AST-present path.
        let suggestion = perl_lsp_diagnostics::build_parse_error_hint(error, &base_message);
        let message = match suggestion.as_deref() {
            Some(hint) => format!("{base_message}\nSuggestion: {hint}"),
            None => base_message,
        };

        let end_offset = offset.saturating_add(1).min(text.len());
        let range = lsp_range_from_offsets(text, offset, end_offset);

        let code = parse_error_code(error);
        let code_str = code.as_str();
        let data = serde_json::to_value(DiagnosticData {
            code: code_str.to_string(),
            category: format!("{:?}", code.category()),
            fixable: is_fixable_diagnostic(code_str),
            tags: vec![],
        })
        .map_err(|e| {
            tracing::warn!(error = %e, "pull diagnostics: failed to serialize diagnostic data");
        })
        .ok();

        LspDiagnostic {
            range,
            severity: Some(to_lsp_severity(parse_error_severity(error))),
            code: Some(NumberOrString::String(code_str.to_string())),
            code_description: None,
            source: Some("perl-lsp".to_string()),
            message,
            related_information: to_lsp_related_information(uri, text, &[]),
            tags: None,
            data,
        }
    }
}

fn lsp_range_from_offsets(text: &str, start: usize, end: usize) -> Range {
    let (start, end) = if start <= end { (start, end) } else { (end, start) };
    let (start_line, start_col) = offset_to_utf16_line_col(text, start);
    let (end_line, end_col) = offset_to_utf16_line_col(text, end);
    Range::new(Position::new(start_line, start_col), Position::new(end_line, end_col))
}

fn to_lsp_severity(severity: InternalDiagnosticSeverity) -> LspDiagnosticSeverity {
    match severity {
        InternalDiagnosticSeverity::Error => LspDiagnosticSeverity::ERROR,
        InternalDiagnosticSeverity::Warning => LspDiagnosticSeverity::WARNING,
        InternalDiagnosticSeverity::Information => LspDiagnosticSeverity::INFORMATION,
        InternalDiagnosticSeverity::Hint => LspDiagnosticSeverity::HINT,
    }
}

fn to_lsp_tags(tags: &[InternalDiagnosticTag]) -> Option<Vec<LspDiagnosticTag>> {
    if tags.is_empty() {
        return None;
    }

    Some(
        tags.iter()
            .map(|tag| match tag {
                InternalDiagnosticTag::Unnecessary => LspDiagnosticTag::UNNECESSARY,
                InternalDiagnosticTag::Deprecated => LspDiagnosticTag::DEPRECATED,
            })
            .collect(),
    )
}

fn to_lsp_related_information(
    uri: &Uri,
    text: &str,
    infos: &[RelatedInformation],
) -> Option<Vec<DiagnosticRelatedInformation>> {
    if infos.is_empty() {
        return None;
    }

    Some(
        infos
            .iter()
            .map(|info| DiagnosticRelatedInformation {
                location: Location {
                    uri: uri.clone(),
                    range: lsp_range_from_offsets(text, info.location.0, info.location.1),
                },
                message: info.message.clone(),
            })
            .collect(),
    )
}

/// Structured data attached to each LSP diagnostic for client integration.
///
/// Serialized into the `data` field of `lsp_types::Diagnostic` so that clients can
/// identify fixable diagnostics, filter by category, and integrate with code actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticData {
    /// The diagnostic code string (e.g., "PL001")
    pub code: String,
    /// Category name derived from `DiagnosticCode::category()` (e.g., "Parser", "StrictWarnings")
    pub category: String,
    /// Whether a quick-fix code action is currently available for this diagnostic
    pub fixable: bool,
    /// Tag names (e.g., ["Unnecessary"], ["Deprecated"])
    pub tags: Vec<String>,
}

/// Returns `true` when a quick-fix code action exists for the given diagnostic code.
///
/// The authoritative source is `crates/perl-lsp-code-actions/src/code_actions.rs`.
fn is_fixable_diagnostic(code: &str) -> bool {
    if matches!(
        code,
        "TestingAndDebugging::RequireUseStrict"
            | "TestingAndDebugging::RequireUseWarnings"
            | "InputOutput::ProhibitBarewordFileHandles"
            | "InputOutput::RequireBriefOpen"
            | "InputOutput::RequireThreeArgOpen"
            | "Variables::ProhibitUnusedVariables"
    ) {
        return true;
    }

    matches!(
        DiagnosticCode::parse_code(code),
        Some(
            DiagnosticCode::ParseError
                | DiagnosticCode::MissingStrict
                | DiagnosticCode::MissingWarnings
                | DiagnosticCode::PhaseScopedStrictPragma
                | DiagnosticCode::PhaseScopedWarningsPragma
                | DiagnosticCode::UnusedVariable
                | DiagnosticCode::UndefinedVariable
                | DiagnosticCode::VariableShadowing
                | DiagnosticCode::UnusedParameter
                | DiagnosticCode::UnquotedBareword
                | DiagnosticCode::BarewordFilehandle
                | DiagnosticCode::TwoArgOpen
                | DiagnosticCode::AssignmentInCondition
                | DiagnosticCode::NumericComparisonWithUndef
                | DiagnosticCode::DeprecatedDefined
                | DiagnosticCode::MissingPackageDeclaration
                | DiagnosticCode::VariableRedeclaration
                | DiagnosticCode::MisspelledPragma
                | DiagnosticCode::UnreachableCode
                | DiagnosticCode::DuplicateSubroutine
                | DiagnosticCode::MissingReturn
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{DocumentDiagnosticReport, NumberOrString};

    fn get_full_items(report: DocumentDiagnosticReport) -> Vec<lsp_types::Diagnostic> {
        match report {
            DocumentDiagnosticReport::Full(full) => full.full_document_diagnostic_report.items,
            _ => vec![],
        }
    }

    #[test]
    fn diagnostic_data_for_parse_error() -> Result<(), Box<dyn std::error::Error>> {
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse()?;
        let items = get_full_items(provider.get_document_diagnostics(&uri, "my $x = ;", None));
        assert!(!items.is_empty());
        // Find the PL001 (ParseError) diagnostic — ordering may vary depending on
        // which lints run first (e.g., PL100 MissingStrict may precede PL001).
        let diag = items
            .iter()
            .find(|d| d.data.as_ref().and_then(|v| v["code"].as_str()) == Some("PL001"))
            .ok_or("expected a PL001 ParseError diagnostic in the results")?;
        let data = diag.data.as_ref().ok_or("data should be populated")?;
        assert_eq!(data["code"], "PL001");
        assert_eq!(data["category"], "Parser");
        assert_eq!(data["fixable"], true);
        let tags = data["tags"].as_array().ok_or("tags should be an array")?;
        assert!(tags.is_empty());
        Ok(())
    }

    #[test]
    fn diagnostic_data_none_when_no_code() -> Result<(), Box<dyn std::error::Error>> {
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse()?;
        let report = provider.get_document_diagnostics(&uri, "my $x = 1;\n", None);
        let items = get_full_items(report);
        // Any diagnostic without a code must also have data: None
        assert!(items.iter().all(|d| d.code.is_some() || d.data.is_none()));
        Ok(())
    }

    #[test]
    fn diagnostic_data_for_missing_strict() -> Result<(), Box<dyn std::error::Error>> {
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse()?;
        let code = "print 'hello';\n";
        let items = get_full_items(provider.get_document_diagnostics(&uri, code, None));
        let diag = items
            .iter()
            .find(|d| {
                d.code.as_ref().map(|c| matches!(c, NumberOrString::String(s) if s == "PL100"))
                    == Some(true)
            })
            .ok_or("expected PL100 (missing strict) diagnostic for bare print statement")?;
        let data = diag.data.as_ref().ok_or("data should be Some for PL100")?;
        assert_eq!(data["code"], "PL100");
        assert_eq!(data["category"], "StrictWarnings");
        assert_eq!(data["fixable"], true);
        Ok(())
    }

    #[test]
    fn diagnostic_data_fixable_true_for_variable_redeclaration(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // PL105 (VariableRedeclaration) offers a quick-fix that removes the duplicate `my`,
        // so the enriched diagnostic data must advertise it as fixable.
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse()?;
        // Redeclare $x in the same scope to trigger PL105
        let code = "use strict; use warnings; my $x = 1; my $x = 2;\n";
        let items = get_full_items(provider.get_document_diagnostics(&uri, code, None));
        if let Some(diag) = items.iter().find(|d| {
            d.code.as_ref().map(|c| matches!(c, NumberOrString::String(s) if s == "PL105"))
                == Some(true)
        }) {
            let data = diag.data.as_ref().ok_or("data should be Some for PL105")?;
            assert_eq!(data["code"], "PL105");
            assert_eq!(data["fixable"], true, "PL105 now has a quick-fix; fixable must stay true");
        }
        // Also verify that every diagnostic with a code has a valid data object
        for d in &items {
            if d.code.is_some() {
                let data = d.data.as_ref().ok_or("data must be Some when code is Some")?;
                assert!(data["fixable"].is_boolean(), "fixable must always be a boolean");
            }
        }
        Ok(())
    }

    #[test]
    fn diagnostic_data_is_valid_json_object() -> Result<(), Box<dyn std::error::Error>> {
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse()?;
        let items = get_full_items(provider.get_document_diagnostics(&uri, "my $x = ;", None));
        for diag in &items {
            if diag.code.is_some() {
                let data = diag.data.as_ref().ok_or("data must be Some when code is Some")?;
                assert!(data.is_object(), "data must be a JSON object");
                assert!(data["code"].is_string());
                assert!(data["category"].is_string());
                assert!(data["fixable"].is_boolean());
                assert!(data["tags"].is_array());
            }
        }
        Ok(())
    }

    #[test]
    fn invalid_prototype_syntax_error_maps_to_pl302_warning(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse()?;
        let diagnostic = provider.parse_error_to_diagnostic(
            &uri,
            "sub foo (XYZ) {}",
            &ParseError::SyntaxError {
                location: 8,
                message: "Invalid prototype character(s) 'X'".to_string(),
            },
        );

        assert_eq!(diagnostic.code, Some(NumberOrString::String("PL302".to_string())));
        assert_eq!(diagnostic.severity, Some(LspDiagnosticSeverity::WARNING));
        let data = diagnostic.data.as_ref().ok_or("data should be populated")?;
        assert_eq!(data["code"], "PL302");
        Ok(())
    }

    #[test]
    fn perlcritic_policy_codes_are_marked_fixable_in_diagnostic_data() {
        assert!(is_fixable_diagnostic("PL502"));
        assert!(is_fixable_diagnostic("PL503"));
        assert!(is_fixable_diagnostic("TestingAndDebugging::RequireUseStrict"));
        assert!(is_fixable_diagnostic("TestingAndDebugging::RequireUseWarnings"));
        assert!(is_fixable_diagnostic("InputOutput::RequireThreeArgOpen"));
        assert!(is_fixable_diagnostic("Variables::ProhibitUnusedVariables"));
    }

    #[test]
    fn unknown_subroutine_attribute_syntax_error_stays_warning(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse()?;
        let diagnostic = provider.parse_error_to_diagnostic(
            &uri,
            "sub foo :wat {}",
            &ParseError::SyntaxError {
                location: 8,
                message: "unknown subroutine attribute ':wat'".to_string(),
            },
        );

        assert_eq!(diagnostic.code, Some(NumberOrString::String("PL002".to_string())));
        assert_eq!(diagnostic.severity, Some(LspDiagnosticSeverity::WARNING));
        let data = diagnostic.data.as_ref().ok_or("data should be populated")?;
        assert_eq!(data["code"], "PL002");
        Ok(())
    }
}
