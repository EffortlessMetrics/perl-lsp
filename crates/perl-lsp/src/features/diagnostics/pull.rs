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

use crate::state::DocumentState;
use crate::util::uri::parse_uri;
use perl_parser::Parser;
use perl_parser::error::ParseError;
use perl_parser::position::offset_to_utf16_line_col;
use perl_parser::util::code_slice;

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
                provider
                    .get_diagnostics(&ast, &parse_errors, content)
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
            provider
                .get_diagnostics(ast, &doc_state.parse_errors, &doc_state.text)
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
        let tags = to_lsp_tags(&diagnostic.tags);

        LspDiagnostic {
            range,
            severity,
            code,
            code_description: None,
            source: Some("perl-lsp".to_string()),
            message: diagnostic.message,
            related_information,
            tags,
            data: None,
        }
    }

    fn parse_error_to_diagnostic(
        &self,
        uri: &Uri,
        text: &str,
        error: &ParseError,
    ) -> LspDiagnostic {
        let (offset, message) = match error {
            ParseError::UnexpectedToken { location, expected, found } => {
                (*location, format!("Expected {expected}, found {found}"))
            }
            ParseError::SyntaxError { location, message } => (*location, message.clone()),
            ParseError::UnexpectedEof => (text.len(), "Unexpected end of input".to_string()),
            ParseError::LexerError { message } => (0, message.clone()),
            _ => (0, error.to_string()),
        };

        let end_offset = offset.saturating_add(1).min(text.len());
        let range = lsp_range_from_offsets(text, offset, end_offset);

        LspDiagnostic {
            range,
            severity: Some(LspDiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("parse-error".to_string())),
            code_description: None,
            source: Some("perl-lsp".to_string()),
            message,
            related_information: to_lsp_related_information(uri, text, &[]),
            tags: None,
            data: None,
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
