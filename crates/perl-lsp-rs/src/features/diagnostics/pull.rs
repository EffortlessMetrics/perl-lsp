//! Pull-based diagnostics support (LSP 3.17).

use std::collections::HashMap;
use std::path::PathBuf;

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
use perl_diagnostics::codes::DiagnosticCode;
use perl_lsp_rs_core::providers::diagnostics::{parse_error_code, parse_error_severity};
use perl_module::resolution::use_lib::resolve_use_lib_paths_from_source;
use perl_parser::Parser;
use perl_parser::error::ParseError;
use perl_parser::position::offset_to_utf16_line_col;
use perl_parser::util::code_slice;

// Import core diagnostics types from perl-lsp-providers (via parent module re-export)
use super::{
    Diagnostic as InternalDiagnostic, DiagnosticSeverity as InternalDiagnosticSeverity,
    DiagnosticTag as InternalDiagnosticTag, DiagnosticsProvider, RelatedInformation,
};

/// Context for pull diagnostics operations.
///
/// Contains all configuration and state needed to compute diagnostics
/// without direct LspServer dependencies, enabling testability and
/// clean separation of concerns.
#[derive(Clone)]
pub struct PullDiagnosticsContext {
    /// Whether perlcritic is enabled
    pub perlcritic_enabled: bool,
    /// Minimum severity for perlcritic (1-5)
    pub perlcritic_severity: i32,
    /// Optional perlcritic profile path
    pub perlcritic_profile: Option<String>,
    /// Workspace root for .perlcriticrc discovery
    pub workspace_root: Option<PathBuf>,
    /// @INC paths for module resolution
    pub include_paths: Vec<String>,
    /// Whether client supports LSP 3.18 markup messages
    pub markup_message_support: bool,
    /// Optional workspace index for dead code detection
    #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
    pub workspace_index: Option<std::sync::Arc<perl_workspace::workspace_index::WorkspaceIndex>>,
}

impl PullDiagnosticsContext {
    /// Create a new empty context with default values.
    pub fn new() -> Self {
        Self {
            perlcritic_enabled: false,
            perlcritic_severity: 3,
            perlcritic_profile: None,
            workspace_root: None,
            include_paths: Vec::new(),
            markup_message_support: false,
            #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
            workspace_index: None,
        }
    }

    /// Create a context with perlcritic enabled.
    #[cfg(test)]
    pub fn with_perlcritic(severity: i32, profile: Option<String>) -> Self {
        Self {
            perlcritic_enabled: true,
            perlcritic_severity: severity,
            perlcritic_profile: profile,
            workspace_root: None,
            include_paths: Vec::new(),
            markup_message_support: false,
            #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
            workspace_index: None,
        }
    }

    /// Create a context with workspace index for dead code detection.
    #[cfg(all(feature = "workspace", not(target_arch = "wasm32"), test))]
    pub fn with_workspace_index(
        index: std::sync::Arc<perl_workspace::workspace_index::WorkspaceIndex>,
    ) -> Self {
        Self {
            perlcritic_enabled: false,
            perlcritic_severity: 3,
            perlcritic_profile: None,
            workspace_root: None,
            include_paths: Vec::new(),
            markup_message_support: false,
            workspace_index: Some(index),
        }
    }
}

impl std::fmt::Debug for PullDiagnosticsContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PullDiagnosticsContext")
            .field("perlcritic_enabled", &self.perlcritic_enabled)
            .field("perlcritic_severity", &self.perlcritic_severity)
            .field("perlcritic_profile", &self.perlcritic_profile)
            .field("workspace_root", &self.workspace_root)
            .field("include_paths", &self.include_paths)
            .field("markup_message_support", &self.markup_message_support)
            .field("workspace_index", &"<WorkspaceIndex>")
            .finish()
    }
}

/// Provider for pull-based diagnostics (LSP 3.17).
pub struct PullDiagnosticsProvider;

impl PullDiagnosticsProvider {
    /// Create a new pull diagnostics provider.
    pub fn new() -> Self {
        Self
    }

    /// Handle textDocument/diagnostic request.
    ///
    /// The `include_paths` parameter allows specifying @INC search paths for PL701
    /// (ModuleNotFound) diagnostics. When `None`, the context is created with empty
    /// include_paths (backward compatible with existing call sites).
    pub fn get_document_diagnostics(
        &self,
        uri: &Uri,
        content: &str,
        previous_result_id: Option<String>,
        include_paths: Option<Vec<String>>,
    ) -> DocumentDiagnosticReport {
        let mut context = PullDiagnosticsContext::new();
        if let Some(paths) = include_paths {
            context.include_paths = paths;
        }
        self.get_document_diagnostics_with_context(uri, content, previous_result_id, &context, None)
    }

    /// Handle textDocument/diagnostic request with full context.
    ///
    /// This is the production entry point that includes all diagnostic sources:
    /// - Parse errors and AST-based diagnostics
    /// - External perlcritic integration (if enabled in context)
    /// - Dead code detection (if workspace index available)
    /// - Built-in Perl::Critic policy analysis
    /// - @INC-aware module resolution diagnostics
    pub fn get_document_diagnostics_with_context(
        &self,
        uri: &Uri,
        content: &str,
        previous_result_id: Option<String>,
        context: &PullDiagnosticsContext,
        doc_state: Option<&DocumentState>,
    ) -> DocumentDiagnosticReport {
        let result_id = format!("{:x}", md5::compute(content));
        if previous_result_id.as_deref() == Some(&result_id) {
            return self.build_unchanged_report(result_id);
        }

        let diagnostics =
            self.collect_diagnostics_for_text_with_context(uri, content, context, doc_state);
        self.build_full_report(result_id, diagnostics)
    }

    /// Handle workspace/diagnostic request.
    pub fn get_workspace_diagnostics(
        &self,
        documents: &HashMap<String, DocumentState>,
        previous_result_ids: Vec<(Uri, String)>,
    ) -> WorkspaceDiagnosticReport {
        let context = PullDiagnosticsContext::new();
        self.get_workspace_diagnostics_with_context(documents, previous_result_ids, &context)
    }

    /// Handle workspace/diagnostic request with full context.
    pub fn get_workspace_diagnostics_with_context(
        &self,
        documents: &HashMap<String, DocumentState>,
        previous_result_ids: Vec<(Uri, String)>,
        context: &PullDiagnosticsContext,
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
                let diagnostics =
                    self.collect_diagnostics_for_state_with_context(&uri, doc_state, context);
                self.build_full_report(result_id, diagnostics)
            };

            items.push(self.to_workspace_report(uri, Some(doc_state.version), report));
        }

        WorkspaceDiagnosticReport { items }
    }

    /// Handle workspace/diagnostic partial result with context.
    pub fn get_workspace_diagnostics_partial_with_context(
        &self,
        documents: &[(String, String)],
        batch_size: usize,
        context: &PullDiagnosticsContext,
    ) -> Vec<WorkspaceDiagnosticReportPartialResult> {
        let mut results = Vec::new();

        for chunk in documents.chunks(batch_size) {
            let mut items = Vec::new();

            for (uri_str, content) in chunk {
                let uri = parse_uri(uri_str);
                let result_id = format!("{:x}", md5::compute(content));
                // For partial results, we need to parse the content
                let diagnostics =
                    self.collect_diagnostics_for_text_with_context(&uri, content, context, None);
                let report = self.build_full_report(result_id, diagnostics);

                items.push(self.to_workspace_report(uri, None, report));
            }

            results.push(WorkspaceDiagnosticReportPartialResult { items });
        }

        results
    }

    fn collect_diagnostics_for_text_with_context(
        &self,
        uri: &Uri,
        content: &str,
        context: &PullDiagnosticsContext,
        _doc_state: Option<&DocumentState>,
    ) -> Vec<LspDiagnostic> {
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
                let include_paths = self.effective_include_paths(
                    &context.include_paths,
                    content,
                    source_path.as_deref(),
                    context,
                );

                // Build module resolver using context include_paths
                let resolver = |module: &str| {
                    self.resolve_module_with_paths(module, &include_paths, source_path.as_deref())
                };

                let search_paths: Vec<String> = include_paths.clone();

                let mut diagnostics = provider
                    .get_diagnostics_with_path(
                        &ast,
                        &parse_errors,
                        content,
                        Some(&resolver),
                        &search_paths,
                        source_path.as_deref(),
                    )
                    .into_iter()
                    .map(|d| self.to_lsp_diagnostic_with_context(uri, content, d, context))
                    .collect::<Vec<_>>();

                // Add built-in Perl::Critic policy violations
                self.add_builtin_critic_diagnostics(uri, &ast, content, &mut diagnostics);

                diagnostics
            }
            Err(error) => {
                vec![self.parse_error_to_diagnostic_with_context(uri, content, &error, context)]
            }
        }
    }

    /// Resolve a module to a path using the provided include paths.
    fn resolve_module_with_paths(
        &self,
        module: &str,
        include_paths: &[String],
        source_path: Option<&std::path::Path>,
    ) -> bool {
        // Convert module name to path
        let module_path = module.replace("::", "/") + ".pm";

        // Check include paths
        for path in include_paths {
            let include_root = {
                let include_path = std::path::Path::new(path);
                if include_path.is_absolute() {
                    include_path.to_path_buf()
                } else if let Some(source_parent) = source_path.and_then(std::path::Path::parent) {
                    source_parent.join(include_path)
                } else {
                    include_path.to_path_buf()
                }
            };
            let full_path = include_root.join(&module_path);
            if full_path.exists() {
                return true;
            }
        }

        // Check relative to source file
        if let Some(source) = source_path {
            if let Some(parent) = source.parent() {
                let relative_path = parent.join(&module_path);
                if relative_path.exists() {
                    return true;
                }
            }
        }

        false
    }

    fn effective_include_paths(
        &self,
        include_paths: &[String],
        content: &str,
        source_path: Option<&std::path::Path>,
        context: &PullDiagnosticsContext,
    ) -> Vec<String> {
        let mut effective_paths = include_paths.to_vec();
        let workspace_root = context
            .workspace_root
            .as_deref()
            .or_else(|| source_path.and_then(std::path::Path::parent))
            .unwrap_or(std::path::Path::new("."));
        let file_dir = source_path.and_then(std::path::Path::parent);

        let dynamic_paths = resolve_use_lib_paths_from_source(content, workspace_root, file_dir);
        for path in dynamic_paths.into_iter().rev() {
            effective_paths.retain(|existing| existing != &path);
            effective_paths.insert(0, path);
        }

        effective_paths
    }

    /// Add built-in Perl::Critic policy diagnostics.
    fn add_builtin_critic_diagnostics(
        &self,
        uri: &Uri,
        ast: &std::sync::Arc<perl_parser::ast::Node>,
        content: &str,
        diagnostics: &mut Vec<LspDiagnostic>,
    ) {
        use perl_lsp_rs_core::tooling::perl_critic::BuiltInAnalyzer;

        let built_in_analyzer = BuiltInAnalyzer::new();
        let violations = built_in_analyzer.analyze(ast, content);

        for violation in violations {
            let lsp_severity = violation.severity.to_diagnostic_severity();
            let internal_severity = match lsp_severity {
                lsp_types::DiagnosticSeverity::ERROR => InternalDiagnosticSeverity::Error,
                lsp_types::DiagnosticSeverity::WARNING => InternalDiagnosticSeverity::Warning,
                lsp_types::DiagnosticSeverity::INFORMATION => {
                    InternalDiagnosticSeverity::Information
                }
                lsp_types::DiagnosticSeverity::HINT => InternalDiagnosticSeverity::Hint,
                _ => InternalDiagnosticSeverity::Hint,
            };

            let internal_diag = InternalDiagnostic {
                range: (violation.range.start.byte, violation.range.end.byte),
                severity: internal_severity,
                code: Some(violation.policy.clone()),
                message: violation.description.clone(),
                related_information: Vec::new(),
                tags: Vec::new(),
                suggestion: None,
            };

            diagnostics.push(self.to_lsp_diagnostic(uri, content, internal_diag));
        }
    }

    fn collect_diagnostics_for_state_with_context(
        &self,
        uri: &Uri,
        doc_state: &DocumentState,
        context: &PullDiagnosticsContext,
    ) -> Vec<LspDiagnostic> {
        if let Some(ast) = &doc_state.ast {
            let provider = DiagnosticsProvider::new(ast, doc_state.text.clone());
            let source_path =
                url::Url::parse(&uri.to_string()).ok().and_then(|value| value.to_file_path().ok());
            let include_paths = self.effective_include_paths(
                &context.include_paths,
                &doc_state.text,
                source_path.as_deref(),
                context,
            );

            // Build module resolver using context include_paths
            let resolver = |module: &str| {
                self.resolve_module_with_paths(module, &include_paths, source_path.as_deref())
            };

            let search_paths: Vec<String> = include_paths.clone();

            let mut diagnostics = provider
                .get_diagnostics_with_path(
                    ast,
                    &doc_state.parse_errors,
                    &doc_state.text,
                    Some(&resolver),
                    &search_paths,
                    source_path.as_deref(),
                )
                .into_iter()
                .map(|d| self.to_lsp_diagnostic_with_context(uri, &doc_state.text, d, context))
                .collect::<Vec<_>>();

            // Add built-in Perl::Critic policy violations
            self.add_builtin_critic_diagnostics(uri, ast, &doc_state.text, &mut diagnostics);

            // Add dead code diagnostics from workspace-wide symbol analysis
            #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
            {
                if let Some(ref workspace_index) = context.workspace_index {
                    let dead_code_diags =
                        perl_lsp_rs_core::providers::diagnostics::detect_dead_code(
                            workspace_index,
                            &uri.to_string(),
                            &doc_state.text,
                            &doc_state.line_starts,
                        );
                    // Convert dead code diagnostics to LSP format
                    for d in dead_code_diags {
                        diagnostics.push(self.internal_to_lsp_diagnostic(
                            uri,
                            &doc_state.text,
                            d,
                            context,
                        ));
                    }
                }
            }

            diagnostics
        } else if doc_state.parse_errors.is_empty() {
            Vec::new()
        } else {
            doc_state
                .parse_errors
                .iter()
                .map(|error| {
                    self.parse_error_to_diagnostic_with_context(
                        uri,
                        &doc_state.text,
                        error,
                        context,
                    )
                })
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

    /// Convert internal diagnostic to LSP diagnostic with context support.
    fn to_lsp_diagnostic_with_context(
        &self,
        uri: &Uri,
        text: &str,
        diagnostic: InternalDiagnostic,
        context: &PullDiagnosticsContext,
    ) -> LspDiagnostic {
        let range = lsp_range_from_offsets(text, diagnostic.range.0, diagnostic.range.1);
        let severity = Some(to_lsp_severity(diagnostic.severity));
        let code = diagnostic.code.map(NumberOrString::String);
        let code_for_source = code.clone();
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
            None => diagnostic.message.clone(),
        };

        let data = code.as_ref().and_then(|c| {
            if let NumberOrString::String(code_str) = c {
                let category = DiagnosticCode::parse_code(code_str)
                    .map(|dc| format!("{:?}", dc.category()))
                    .unwrap_or_else(|| {
                        // Check if it's a perlcritic policy
                        if code_str.contains("::") {
                            "PerlCritic".to_string()
                        } else {
                            "Other".to_string()
                        }
                    });
                let fixable = is_fixable_diagnostic(code_str);
                let data_obj = DiagnosticData {
                    code: code_str.clone(),
                    category,
                    fixable,
                    tags: tag_strings.clone(),
                };

                // Add LSP 3.18 markup message support if enabled
                if context.markup_message_support {
                    let markdown = format!("**{}**: {}", code_str, diagnostic.message);
                    return serde_json::to_value(data_obj).ok().map(|mut v| {
                        v["messageMarkup"] = serde_json::json!({
                            "kind": "markdown",
                            "value": markdown
                        });
                        v
                    });
                }

                serde_json::to_value(data_obj).ok()
            } else {
                None
            }
        });

        LspDiagnostic {
            range,
            severity,
            code,
            code_description: None,
            source: diagnostic_source(code_for_source.as_ref()),
            message,
            related_information,
            tags,
            data,
        }
    }

    /// Convert internal diagnostic from perl-lsp-diagnostics crate to LSP diagnostic.
    fn internal_to_lsp_diagnostic(
        &self,
        _uri: &Uri,
        text: &str,
        diagnostic: perl_lsp_rs_core::providers::diagnostics::Diagnostic,
        context: &PullDiagnosticsContext,
    ) -> LspDiagnostic {
        let range = lsp_range_from_offsets(text, diagnostic.range.0, diagnostic.range.1);
        let severity = Some(to_lsp_severity(diagnostic.severity));
        let code = diagnostic.code.map(NumberOrString::String);
        let code_for_source = code.clone();
        let tags = to_lsp_tags(&diagnostic.tags);

        // Collect tag strings
        let tag_strings: Vec<String> = diagnostic
            .tags
            .iter()
            .map(|t| match t {
                perl_lsp_rs_core::providers::diagnostics::DiagnosticTag::Unnecessary => {
                    "Unnecessary".to_string()
                }
                perl_lsp_rs_core::providers::diagnostics::DiagnosticTag::Deprecated => {
                    "Deprecated".to_string()
                }
            })
            .collect();

        let message = match diagnostic.suggestion {
            Some(ref suggestion) => format!("{}\nSuggestion: {}", diagnostic.message, suggestion),
            None => diagnostic.message.clone(),
        };

        let data = code.as_ref().and_then(|c| {
            if let NumberOrString::String(code_str) = c {
                let category = DiagnosticCode::parse_code(code_str)
                    .map(|dc| format!("{:?}", dc.category()))
                    .unwrap_or_else(|| "Other".to_string());
                let fixable = is_fixable_diagnostic(code_str);
                let data_obj = DiagnosticData {
                    code: code_str.clone(),
                    category,
                    fixable,
                    tags: tag_strings.clone(),
                };

                // Add LSP 3.18 markup message support if enabled
                if context.markup_message_support {
                    let markdown = format!("**{}**: {}", code_str, diagnostic.message);
                    return serde_json::to_value(data_obj).ok().map(|mut v| {
                        v["messageMarkup"] = serde_json::json!({
                            "kind": "markdown",
                            "value": markdown
                        });
                        v
                    });
                }

                serde_json::to_value(data_obj).ok()
            } else {
                None
            }
        });

        LspDiagnostic {
            range,
            severity,
            code,
            code_description: None,
            source: diagnostic_source(code_for_source.as_ref()),
            message,
            related_information: None,
            tags,
            data,
        }
    }

    #[cfg(test)]
    fn parse_error_to_diagnostic(
        &self,
        uri: &Uri,
        text: &str,
        error: &ParseError,
    ) -> LspDiagnostic {
        let context = PullDiagnosticsContext::new();
        self.parse_error_to_diagnostic_with_context(uri, text, error, &context)
    }

    fn parse_error_to_diagnostic_with_context(
        &self,
        uri: &Uri,
        text: &str,
        error: &ParseError,
        context: &PullDiagnosticsContext,
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
        let suggestion =
            perl_lsp_rs_core::providers::diagnostics::build_parse_error_hint(error, &base_message);
        let message = match suggestion.as_deref() {
            Some(hint) => format!("{base_message}\nSuggestion: {hint}"),
            None => base_message,
        };

        let end_offset = offset.saturating_add(1).min(text.len());
        let range = lsp_range_from_offsets(text, offset, end_offset);

        let code = parse_error_code(error);
        let code_str = code.as_str();

        let data_obj = DiagnosticData {
            code: code_str.to_string(),
            category: format!("{:?}", code.category()),
            fixable: is_fixable_diagnostic(code_str),
            tags: vec![],
        };

        // Add LSP 3.18 markup message support if enabled
        let data = if context.markup_message_support {
            let markdown = format!("**{}**: {}", code_str, message);
            serde_json::to_value(data_obj).ok().map(|mut v| {
                v["messageMarkup"] = serde_json::json!({
                    "kind": "markdown",
                    "value": markdown
                });
                v
            })
        } else {
            serde_json::to_value(data_obj).ok()
        };

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

/// Determine the diagnostic source based on the code.
fn diagnostic_source(code: Option<&NumberOrString>) -> Option<String> {
    match code {
        Some(NumberOrString::String(code_str)) => {
            // Perl::Critic policies contain "::" and are not in our DiagnosticCode enum
            if code_str.contains("::") && DiagnosticCode::parse_code(code_str).is_none() {
                Some("perlcritic".to_string())
            } else {
                Some("perl-lsp".to_string())
            }
        }
        _ => Some("perl-lsp".to_string()),
    }
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
        let items =
            get_full_items(provider.get_document_diagnostics(&uri, "my $x = ;", None, None));
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
        let report = provider.get_document_diagnostics(&uri, "my $x = 1;\n", None, None);
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
        let items = get_full_items(provider.get_document_diagnostics(&uri, code, None, None));
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
    fn diagnostic_data_fixable_true_for_variable_redeclaration()
    -> Result<(), Box<dyn std::error::Error>> {
        // PL105 (VariableRedeclaration) offers a quick-fix that removes the duplicate `my`,
        // so the enriched diagnostic data must advertise it as fixable.
        let provider = PullDiagnosticsProvider::new();
        let uri: Uri = "file:///test.pl".parse()?;
        // Redeclare $x in the same scope to trigger PL105
        let code = "use strict; use warnings; my $x = 1; my $x = 2;\n";
        let items = get_full_items(provider.get_document_diagnostics(&uri, code, None, None));
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
        let items =
            get_full_items(provider.get_document_diagnostics(&uri, "my $x = ;", None, None));
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
    fn invalid_prototype_syntax_error_maps_to_pl302_warning()
    -> Result<(), Box<dyn std::error::Error>> {
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
    fn unknown_subroutine_attribute_syntax_error_stays_warning()
    -> Result<(), Box<dyn std::error::Error>> {
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
