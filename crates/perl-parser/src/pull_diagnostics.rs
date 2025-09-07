use lsp_types::{Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, DocumentDiagnosticReport, DocumentDiagnosticReportKind, FullDocumentDiagnosticReport, RelatedFullDocumentDiagnosticReport, Uri, WorkspaceDocumentDiagnosticReport, WorkspaceDiagnosticReport, WorkspaceDiagnosticReportPartialResult};
use std::collections::HashMap;
use std::sync::Arc;

use crate::diagnostics::DiagnosticsProvider;
use crate::document_store::DocumentStore;

/// Provider for pull-based diagnostics (LSP 3.17)
pub struct PullDiagnosticsProvider {
    diagnostics_provider: Arc<DiagnosticsProvider>,
}

impl PullDiagnosticsProvider {
    pub fn new(diagnostics_provider: Arc<DiagnosticsProvider>) -> Self {
        Self {
            diagnostics_provider,
        }
    }

    /// Handle textDocument/diagnostic request
    pub fn get_document_diagnostics(
        &self,
        uri: &Uri,
        content: &str,
        previous_result_id: Option<String>,
    ) -> DocumentDiagnosticReport {
        // Get diagnostics from the existing provider
        let diagnostics = self.diagnostics_provider.analyze(content);
        
        // Generate a result ID based on content hash
        let result_id = format!("{:x}", md5::compute(content));
        
        // If the result ID matches the previous one, return unchanged
        if let Some(prev_id) = previous_result_id {
            if prev_id == result_id {
                return DocumentDiagnosticReport::Unchanged(
                    lsp_types::RelatedUnchangedDocumentDiagnosticReport {
                        kind: DocumentDiagnosticReportKind::Unchanged,
                        result_id: prev_id,
                    }
                );
            }
        }
        
        // Convert to LSP diagnostics
        let lsp_diagnostics: Vec<Diagnostic> = diagnostics
            .into_iter()
            .map(|d| Diagnostic {
                range: d.range,
                severity: Some(d.severity),
                code: d.code.map(|c| lsp_types::NumberOrString::String(c)),
                code_description: None,
                source: Some("perl-lsp".to_string()),
                message: d.message,
                related_information: d.related_information,
                tags: d.tags,
                data: None,
            })
            .collect();
        
        DocumentDiagnosticReport::Full(
            RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    kind: DocumentDiagnosticReportKind::Full,
                    result_id: Some(result_id),
                    items: lsp_diagnostics,
                },
            }
        )
    }

    /// Handle workspace/diagnostic request
    pub fn get_workspace_diagnostics(
        &self,
        documents: &HashMap<String, crate::lsp_server::DocumentState>,
        previous_result_ids: Vec<(Uri, String)>,
    ) -> WorkspaceDiagnosticReport {
        let mut items = Vec::new();
        let prev_ids: HashMap<Uri, String> = previous_result_ids.into_iter().collect();
        
        for (uri_str, doc_state) in documents {
            let uri = Uri::parse(uri_str).unwrap_or_else(|_| Uri::from_file_path("/invalid").unwrap());
            let prev_id = prev_ids.get(&uri).cloned();
            
            let report = self.get_document_diagnostics(&uri, &doc_state.text, prev_id);
            
            items.push(WorkspaceDocumentDiagnosticReport {
                uri,
                version: Some(doc_state._version),
                report,
            });
        }
        
        WorkspaceDiagnosticReport { items }
    }

    /// Handle workspace/diagnostic partial result
    pub fn get_workspace_diagnostics_partial(
        &self,
        documents: &[(String, String)], // (uri, content) pairs
        batch_size: usize,
    ) -> Vec<WorkspaceDiagnosticReportPartialResult> {
        let mut results = Vec::new();
        
        for chunk in documents.chunks(batch_size) {
            let mut items = Vec::new();
            
            for (uri_str, content) in chunk {
                let uri = Uri::parse(uri_str).unwrap_or_else(|_| Uri::from_file_path("/invalid").unwrap());
                let report = self.get_document_diagnostics(&uri, content, None);
                
                items.push(WorkspaceDocumentDiagnosticReport {
                    uri,
                    version: None,
                    report,
                });
            }
            
            results.push(WorkspaceDiagnosticReportPartialResult { items });
        }
        
        results
    }
}