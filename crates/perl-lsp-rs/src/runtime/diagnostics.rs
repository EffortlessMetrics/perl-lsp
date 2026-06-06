//! Diagnostic publishing and handling
//!
//! Handles both push and pull diagnostics for the LSP server.
//! - Push diagnostics: Server-initiated via `textDocument/publishDiagnostics`
//! - Pull diagnostics: Client-initiated via `textDocument/diagnostic` and `workspace/diagnostic`

use super::*;
use crate::features::diagnostics::{
    Diagnostic as InternalDiagnostic, DiagnosticTag as InternalDiagnosticTag,
    PullDiagnosticsContext,
};
use perl_diagnostics::codes::DiagnosticCode;

#[cfg(not(target_arch = "wasm32"))]
fn resolve_configured_profile_path(
    configured_profile: &str,
    workspace_root: Option<&std::path::Path>,
    file_path: &std::path::Path,
) -> Option<std::path::PathBuf> {
    let profile_path = std::path::Path::new(configured_profile);
    if profile_path.is_absolute() {
        return profile_path.exists().then(|| profile_path.to_path_buf());
    }

    let file_dir = file_path.parent();
    [
        Some(profile_path.to_path_buf()),
        workspace_root.map(|root| root.join(profile_path)),
        file_dir.map(|dir| dir.join(profile_path)),
    ]
    .into_iter()
    .flatten()
    .find(|candidate| candidate.exists())
}

#[cfg(not(target_arch = "wasm32"))]
fn workspace_root_for_doc(server: &LspServer, uri: &str) -> Option<std::path::PathBuf> {
    server
        .folder_for_doc_uri(uri)
        .and_then(|folder| folder.path.or_else(|| source_path_from_uri(&folder.uri)))
        .or_else(|| server.root_path.lock().clone())
}

#[cfg(not(target_arch = "wasm32"))]
fn find_workspace_perlcritic_profile(
    workspace_root: Option<&std::path::Path>,
    file_path: &std::path::Path,
) -> Option<String> {
    let mut dir = file_path.parent().map(|p| p.to_path_buf());
    while let Some(current) = dir {
        for profile_name in [".perlcriticrc", "perlcriticrc"] {
            let candidate = current.join(profile_name);
            if candidate.exists() {
                return candidate.to_str().map(|s| s.to_string());
            }
        }

        if workspace_root == Some(current.as_path()) || current.parent().is_none() {
            break;
        }
        dir = current.parent().map(|p| p.to_path_buf());
    }
    None
}

/// Orchestrator for pull diagnostics operations.
///
/// Coordinates between LspServer state and the pure-logic PullDiagnosticsProvider.
/// Handles:
/// - Building context from server state (config, workspace index, capabilities)
/// - Managing the cached CriticAnalyzer for external perlcritic integration
/// - Emitting workspace-scoped warnings (with deduplication)
/// - @INC path resolution for module diagnostics
pub struct PullDiagnosticsOrchestrator {
    /// Cached CriticAnalyzer for external perlcritic
    #[cfg(not(target_arch = "wasm32"))]
    critic_analyzer: Mutex<Option<perl_lsp_rs_core::tooling::perl_critic::CriticAnalyzer>>,
    /// Track warnings already emitted (deduplication)
    #[cfg(not(target_arch = "wasm32"))]
    warnings_sent: Mutex<std::collections::HashSet<String>>,
}

impl PullDiagnosticsOrchestrator {
    /// Create a new orchestrator.
    pub fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            critic_analyzer: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            warnings_sent: Mutex::new(std::collections::HashSet::new()),
        }
    }

    /// Build context from LspServer state.
    pub fn build_context(&self, server: &LspServer, uri: &str) -> PullDiagnosticsContext {
        // Get config values
        let (
            perlcritic_enabled,
            perlcritic_severity,
            perlcritic_profile,
            critic_engine,
            native_critic_profile,
            native_critic_include,
            native_critic_exclude,
        ) = {
            let cfg = server.config.lock();
            (
                cfg.perlcritic_enabled,
                cfg.perlcritic_severity,
                cfg.perlcritic_profile.clone(),
                cfg.critic_engine,
                cfg.native_critic_profile.clone(),
                cfg.native_critic_include.clone(),
                cfg.native_critic_exclude.clone(),
            )
        };

        let profile =
            perlcritic_profile.and_then(|p| if p.trim().is_empty() { None } else { Some(p) });

        // Get workspace root for this document's containing folder (multi-root aware).
        // Falls back to the global root_path when no specific folder matches.
        //
        // Note: we inline the resolution here rather than calling `workspace_root_for_doc`
        // because `build_context` runs on all targets (including wasm32), while
        // `workspace_root_for_doc` is `#[cfg(not(target_arch = "wasm32"))]` since it is
        // only needed from the native perlcritic diagnostic paths.
        let workspace_root = server
            .folder_for_doc_uri(uri)
            .and_then(|folder| folder.path.or_else(|| source_path_from_uri(&folder.uri)))
            .or_else(|| server.root_path.lock().clone());

        // Get include paths for the document
        let include_paths: Vec<String> = server
            .include_paths_for_doc(uri)
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();

        // Get client capabilities
        let markup_message_support = server.client_capabilities.lock().markup_message_support;

        // Build context
        PullDiagnosticsContext {
            perlcritic_enabled,
            perlcritic_severity: perlcritic_severity.into(),
            perlcritic_profile: profile,
            critic_engine,
            native_critic_profile,
            native_critic_include,
            native_critic_exclude,
            workspace_root,
            include_paths,
            markup_message_support,
            #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
            workspace_index: server.workspace_index(),
        }
    }

    /// Collect external perlcritic diagnostics.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn collect_perlcritic_diagnostics(
        &self,
        server: &LspServer,
        uri: &str,
        doc_text: &str,
        diagnostics: &mut Vec<InternalDiagnostic>,
    ) {
        use perl_lsp_rs_core::tooling::perl_critic::{CriticAnalyzer, CriticConfig};

        // Check config
        let (enabled, severity, profile, theme, critic_engine) = {
            let cfg = server.config.lock();
            (
                cfg.perlcritic_enabled,
                cfg.perlcritic_severity,
                cfg.perlcritic_profile.clone(),
                cfg.perlcritic_theme.clone(),
                cfg.critic_engine,
            )
        };

        if !enabled || critic_engine == perl_lsp_rs_core::config::CriticEngine::Native {
            return;
        }

        let profile = profile.and_then(|p| if p.trim().is_empty() { None } else { Some(p) });

        // Convert URI to file path
        let file_path = match url::Url::parse(uri) {
            Ok(u) => match u.to_file_path() {
                Ok(p) => p,
                Err(()) => {
                    tracing::warn!(uri, "perlcritic: URI is not a file path");
                    return;
                }
            },
            Err(e) => {
                tracing::warn!(uri, error = %e, "perlcritic: failed to parse URI");
                return;
            }
        };

        // Check if perlcritic is available (unless bypassed for tests)
        let skip_check = server.skip_perlcritic_command_check.load(Ordering::Relaxed);
        if !skip_check && !crate::execute_command::command_exists("perlcritic") {
            self.emit_warning(
                server,
                "missing-binary".to_string(),
                "Perl::Critic is enabled but `perlcritic` was not found on PATH. Install Perl::Critic (for example: `cpanm Perl::Critic`) or disable perl.perlcritic.enabled.",
            );
            return;
        }

        let workspace_root = workspace_root_for_doc(server, uri);

        // Validate configured profile if present.
        let resolved_configured_profile = if let Some(ref configured_profile) = profile {
            let resolved = resolve_configured_profile_path(
                configured_profile,
                workspace_root.as_deref(),
                &file_path,
            );
            if resolved.is_none() {
                self.emit_warning(
                    server,
                    format!("missing-profile:{configured_profile}"),
                    &format!(
                        "Perl::Critic profile path does not exist: {configured_profile}. Update perl.perlcritic.profile or create the profile file."
                    ),
                );
                return;
            }
            resolved
        } else {
            None
        };

        // Lazy-init the CriticAnalyzer
        {
            let mut guard = self.critic_analyzer.lock();
            if guard.is_none() {
                // Walk up directory tree looking for .perlcriticrc / perlcriticrc.
                let resolved_profile = resolved_configured_profile
                    .as_ref()
                    .and_then(|p| p.to_str().map(|s| s.to_string()))
                    .or_else(|| {
                        find_workspace_perlcritic_profile(workspace_root.as_deref(), &file_path)
                    });

                let critic_config = CriticConfig {
                    severity,
                    profile: resolved_profile,
                    theme: theme.clone(),
                    ..Default::default()
                };

                // Use injected test runtime if present, otherwise OS runtime
                let analyzer = {
                    let rt_guard = server.critic_runtime_override.lock();
                    if let Some(ref rt) = *rt_guard {
                        CriticAnalyzer::new(critic_config, std::sync::Arc::clone(rt))
                    } else {
                        CriticAnalyzer::with_os_runtime(critic_config)
                    }
                };

                *guard = Some(analyzer);
            }
        }

        // Compute content hash for cache validation (detects stale entries from
        // external file changes that bypass the LSP didChange notification).
        let content_hash = perl_lsp_rs_core::tooling::perl_critic::hash_content(doc_text);

        // Run analysis
        let result = {
            let mut guard = self.critic_analyzer.lock();
            guard.as_mut().map(|a| a.analyze_file_with_hash(&file_path, content_hash))
        };

        match result {
            Some(Ok(violations)) => {
                for v in violations {
                    // Map Perl::Critic severity to LSP severity
                    let internal_severity = match v.severity {
                        perl_lsp_rs_core::tooling::perl_critic::Severity::Gentle => {
                            InternalDiagnosticSeverity::Error
                        }
                        perl_lsp_rs_core::tooling::perl_critic::Severity::Stern
                        | perl_lsp_rs_core::tooling::perl_critic::Severity::Harsh => {
                            InternalDiagnosticSeverity::Warning
                        }
                        perl_lsp_rs_core::tooling::perl_critic::Severity::Cruel => {
                            InternalDiagnosticSeverity::Information
                        }
                        perl_lsp_rs_core::tooling::perl_critic::Severity::Brutal => {
                            InternalDiagnosticSeverity::Hint
                        }
                    };

                    // Convert line/column to byte offset
                    let start_byte = crate::util::position_to_offset(
                        doc_text,
                        v.range.start.line,
                        v.range.start.column,
                    )
                    .unwrap_or(0);
                    let end_byte = crate::util::position_to_offset(
                        doc_text,
                        v.range.end.line,
                        v.range.end.column,
                    )
                    .unwrap_or(start_byte.saturating_add(1));

                    diagnostics.push(InternalDiagnostic {
                        range: (start_byte, end_byte),
                        severity: internal_severity,
                        code: Some(v.policy),
                        message: v.description,
                        related_information: Vec::new(),
                        tags: Vec::new(),
                        suggestion: None,
                    });
                }
            }
            Some(Err(e)) => {
                self.emit_warning(
                    server,
                    format!("execution-failed:{e}"),
                    &format!("Perl::Critic execution failed: {e}"),
                );
                tracing::warn!(uri, error = %e, "perlcritic failed");
            }
            None => {}
        }
    }

    /// No-op stub for WASM targets.
    #[cfg(target_arch = "wasm32")]
    pub fn collect_perlcritic_diagnostics(
        &self,
        _server: &LspServer,
        _uri: &str,
        _doc_text: &str,
        _diagnostics: &mut Vec<InternalDiagnostic>,
    ) {
    }

    /// Emit a workspace-scoped warning (with deduplication).
    #[cfg(not(target_arch = "wasm32"))]
    fn emit_warning(&self, server: &LspServer, key: String, message: &str) {
        let mut sent = self.warnings_sent.lock();
        if sent.insert(key) {
            let _ = server.show_message(super::window::MessageType::Warning, message);
        }
    }

    /// No-op stub for WASM targets.
    #[cfg(target_arch = "wasm32")]
    fn emit_warning(&self, _server: &LspServer, _key: String, _message: &str) {}

    /// Reset the orchestrator state (e.g., on configuration change).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reset(&self) {
        *self.critic_analyzer.lock() = None;
        self.warnings_sent.lock().clear();
    }

    /// No-op stub for WASM targets.
    #[cfg(target_arch = "wasm32")]
    pub fn reset(&self) {}

    /// Invalidate cached perlcritic violations for a single file path.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn invalidate_file_cache(&self, file_path: &std::path::Path) {
        let path_str = file_path.to_string_lossy().to_string();
        if let Some(ref mut analyzer) = *self.critic_analyzer.lock() {
            analyzer.invalidate_cache(&path_str);
        }
    }

    /// No-op stub for WASM targets.
    #[cfg(target_arch = "wasm32")]
    pub fn invalidate_file_cache(&self, _file_path: &std::path::Path) {}
}

impl Default for PullDiagnosticsOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl LspServer {
    /// Convert internal diagnostic tags to LSP tag values
    ///
    /// Maps internal `DiagnosticTag` variants to their LSP numeric equivalents:
    /// - Unnecessary â†’ 1
    /// - Deprecated â†’ 2
    fn diagnostic_tags_to_lsp(tags: &[InternalDiagnosticTag]) -> Vec<i32> {
        tags.iter()
            .map(|t| match t {
                InternalDiagnosticTag::Unnecessary => 1,
                InternalDiagnosticTag::Deprecated => 2,
            })
            .collect()
    }

    /// Publish diagnostics for a document (push diagnostics)
    ///
    /// Computes and publishes diagnostics for a Perl document including syntax
    /// errors, semantic issues, and Perl::Critic-style code quality checks.
    /// Uses push-based notification model for backward compatibility with LSP 3.16 clients.
    ///
    /// # LSP Protocol
    ///
    /// Notification: `textDocument/publishDiagnostics`
    /// Capability: `textDocument.publishDiagnostics`
    ///
    /// # Arguments
    ///
    /// * `uri` - Document URI to compute diagnostics for
    ///
    /// # Diagnostics Sources
    ///
    /// - Parse errors from Perl parser
    /// - Unused variable warnings from scope analysis
    /// - Perl::Critic built-in policy violations
    /// - External perlcritic violations (opt-in via config)
    /// - Semantic errors from type inference
    ///
    /// # Performance
    ///
    /// Only publishes if client doesn't support pull diagnostics to avoid
    /// double-flow for modern LSP 3.17+ clients.
    pub(crate) fn publish_diagnostics(&self, uri: &str) {
        if self.client_supports_pull_diags.load(Ordering::Relaxed) {
            return;
        }

        // Syntax-only mode: report parse errors only and skip the full
        // semantic / critic / module-resolution / dead-code stack. Latency
        // harnesses use this so "first useful answer" measurements don't
        // include the background analysis cost on every keystroke.
        if self.runtime_tuning.diagnostic_mode
            == perl_lsp_rs_core::runtime::tuning::DiagnosticMode::SyntaxOnly
        {
            self.publish_syntax_only_diagnostics(uri);
            return;
        }

        let normalized_uri = self.normalize_uri_key(uri);

        // Snapshot all fields needed from DocumentState while holding the lock briefly,
        // then drop the lock before calling resolve_module_to_path which also acquires
        // documents.lock().  Holding the lock across that call causes a reentrant deadlock
        // because parking_lot::Mutex is not reentrant.
        let snapshot = {
            let documents = self.documents.lock();
            documents.get(&normalized_uri).or_else(|| documents.get(uri)).map(|doc| {
                (
                    doc.ast.clone(),
                    doc.text.clone(),
                    doc.parse_errors.clone(),
                    doc.version,
                    doc.degradation_tier,
                    doc.line_starts.clone(),
                    doc.rope.clone(),
                    Arc::clone(&doc.generation),
                    doc.generation.load(Ordering::SeqCst),
                )
            })
            // lock is released here
        };

        let Some((
            ast_opt,
            text,
            parse_errors,
            version,
            degradation_tier,
            line_starts,
            rope,
            generation,
            gen_at_snapshot,
        )) = snapshot
        else {
            return;
        };

        // Position helper that works on the snapshotted line_starts + rope.
        let pos16 = |offset: usize| line_starts.offset_to_position_rope(&rope, offset);

        let lsp_diagnostics: Vec<Value> = if let Some(ast) = &ast_opt {
            // Get diagnostics (already includes unused variable detection).
            // resolver is called with the documents lock *released* — no reentrant deadlock.
            //
            // The resolver is position-aware: it receives the byte offset of each `use`
            // statement so that `no lib` cancellations that precede the statement are
            // respected.  Passing `Some(use_site_offset)` to
            // `resolve_module_to_path_with_doc_at_offset` causes
            // `effective_inc_context_for_doc` to call
            // `resolve_use_lib_paths_from_source_at_offset` instead of the whole-file
            // scan, ensuring `no lib 'lib'` strips the path before `use GoneModule` is
            // checked.
            let provider = DiagnosticsProvider::new(ast, text.clone());
            let resolver = |module: &str, use_site_offset: usize| {
                self.resolve_module_to_path_with_doc_at_offset(
                    module,
                    Some(&text),
                    Some(uri),
                    Some(use_site_offset),
                )
                .is_some()
            };
            let search_context = self
                .effective_inc_context_for_doc(Some(uri), Some(&text), None)
                .map(|context| context.search_display_paths())
                .unwrap_or_default();
            let source_path = source_path_from_uri(uri);

            // Wire semantic queries when workspace data is available for this URI.
            // Falls back to NullSemanticQueries (legacy behavior) when the URI is
            // not yet indexed or the workspace feature is disabled.
            #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
            let mut diagnostics = {
                let semantic_diags = self.workspace_index().and_then(|workspace_index| {
                    workspace_index.with_semantic_queries_for_uri(uri, |file_id, queries| {
                        provider.get_diagnostics_with_search_context_and_semantics(
                            ast,
                            &parse_errors,
                            &text,
                            Some(&resolver),
                            &search_context,
                            source_path.as_deref(),
                            file_id,
                            &queries,
                        )
                    })
                });
                semantic_diags.unwrap_or_else(|| {
                    provider.get_diagnostics_with_search_context(
                        ast,
                        &parse_errors,
                        &text,
                        Some(&resolver),
                        &search_context,
                        source_path.as_deref(),
                    )
                })
            };
            #[cfg(not(all(feature = "workspace", not(target_arch = "wasm32"))))]
            let mut diagnostics = provider.get_diagnostics_with_search_context(
                ast,
                &parse_errors,
                &text,
                Some(&resolver),
                &search_context,
                source_path.as_deref(),
            );

            // Add configured policy critic diagnostics.
            self.collect_policy_critic_diagnostics(ast, &text, &mut diagnostics);

            // Add external perlcritic diagnostics (opt-in)
            self.collect_external_perlcritic_diagnostics(uri, &text, &mut diagnostics);

            // Add dead code diagnostics from workspace-wide symbol analysis
            #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
            {
                if let Some(workspace_index) = self.workspace_index() {
                    let dead_code_diags =
                        perl_lsp_rs_core::providers::diagnostics::detect_dead_code(
                            &workspace_index,
                            uri,
                            &text,
                            &line_starts,
                        );
                    diagnostics.extend(dead_code_diags);
                }
            }

            // Convert to LSP diagnostics
            diagnostics
                .into_iter()
                .map(|d| {
                    let (start_line, start_char) = pos16(d.range.0);
                    let (end_line, end_char) = pos16(d.range.1);

                    let mut diag = json!({
                        "range": {
                            "start": {"line": start_line, "character": start_char},
                            "end": {"line": end_line, "character": end_char},
                        },
                        "severity": match d.severity {
                            InternalDiagnosticSeverity::Error => 1,
                            InternalDiagnosticSeverity::Warning => 2,
                            InternalDiagnosticSeverity::Information => 3,
                            InternalDiagnosticSeverity::Hint => 4,
                        },
                        "code": d.code.clone(),
                        "source": push_diagnostic_source(d.code.as_deref()),
                        "message": d.message,
                    });
                    if !d.tags.is_empty() {
                        diag["tags"] = json!(Self::diagnostic_tags_to_lsp(&d.tags));
                    }
                    diag
                })
                .collect()
        } else {
            // No AST available (parse failed completely), just report parse errors
            parse_errors
                .iter()
                .map(|e| {
                    // Extract location and base message from error enum
                    let (location, base_message) = match e {
                        crate::error::ParseError::UnexpectedToken { location, expected, found } => {
                            (*location, format!("Expected {}, found {}", expected, found))
                        }
                        crate::error::ParseError::SyntaxError { location, message } => {
                            (*location, message.clone())
                        }
                        crate::error::ParseError::UnexpectedEof => {
                            (text.len(), "Unexpected end of input".to_string())
                        }
                        crate::error::ParseError::LexerError { message } => (0, message.clone()),
                        _ => (0, e.to_string()),
                    };

                    // Append hint so users see actionable guidance in push fallback path too
                    let message =
                        match perl_lsp_rs_core::providers::diagnostics::build_parse_error_hint(
                            e,
                            &base_message,
                        ) {
                            Some(hint) => format!("{base_message}\nSuggestion: {hint}"),
                            None => base_message,
                        };

                    // Convert byte offset to line/column
                    let (line, character) = pos16(location);

                    json!({
                        "range": {
                            "start": {"line": line, "character": character},
                            "end": {"line": line, "character": character + 1},
                        },
                        "severity": 1, // Error
                        "code": DiagnosticCode::ParseError.as_str(),
                        "source": "perl-parser",
                        "message": message,
                    })
                })
                .collect()
        };

        // Generation-aware staleness guard: if a newer didChange arrived while
        // diagnostics were being computed, discard this result â€” the debouncer
        // will fire again for the latest version.
        if generation.load(Ordering::SeqCst) != gen_at_snapshot {
            tracing::debug!(
                uri = %normalized_uri,
                gen_at_snapshot,
                current_gen = generation.load(Ordering::SeqCst),
                "Skipping stale diagnostic publish (generation advanced during computation)"
            );
            return;
        }

        tracing::debug!(
            count = lsp_diagnostics.len(),
            uri = %normalized_uri,
            version,
            tier = %degradation_tier,
            "Publishing diagnostics"
        );

        // Send diagnostics notification with version.
        // This ensures diagnostics are cleared when all errors are fixed.
        if let Err(e) = self.notify(
            "textDocument/publishDiagnostics",
            json!({
                "uri": uri,
                "version": version,
                "diagnostics": lsp_diagnostics
            }),
        ) {
            tracing::error!(uri, error = %e, "Failed to publish diagnostics");
        }
    }

    /// Build LSP diagnostics from a document's `parse_errors`, without the
    /// AST-based semantic / critic / dead-code / module-resolution passes.
    ///
    /// Returns an empty `Vec` when the document has no parse errors —
    /// publishing this still produces an empty `publishDiagnostics` payload,
    /// which is how LSP signals "the parse cleared". This is what makes the
    /// `syntax_only_clears_when_parse_errors_clear` acceptance case work.
    fn syntax_only_lsp_diagnostics(
        parse_errors: &[perl_parser::error::ParseError],
        text: &str,
        line_starts: &perl_parser::position::LineStartsCache,
        rope: &ropey::Rope,
        markup_message_support: bool,
    ) -> Vec<Value> {
        let pos16 = |offset: usize| line_starts.offset_to_position_rope(rope, offset);
        parse_errors
            .iter()
            .map(|e| {
                let (location, base_message) = match e {
                    crate::error::ParseError::UnexpectedToken { location, expected, found } => {
                        (*location, format!("Expected {}, found {}", expected, found))
                    }
                    crate::error::ParseError::SyntaxError { location, message } => {
                        (*location, message.clone())
                    }
                    crate::error::ParseError::UnexpectedEof => {
                        (text.len(), "Unexpected end of input".to_string())
                    }
                    crate::error::ParseError::LexerError { message } => (0, message.clone()),
                    crate::error::ParseError::RecursionLimit => (0, e.to_string()),
                    crate::error::ParseError::InvalidNumber { .. } => (0, e.to_string()),
                    crate::error::ParseError::InvalidString => (0, e.to_string()),
                    crate::error::ParseError::UnclosedDelimiter { .. } => (0, e.to_string()),
                    crate::error::ParseError::InvalidRegex { .. } => (0, e.to_string()),
                    crate::error::ParseError::NestingTooDeep { .. } => (0, e.to_string()),
                    crate::error::ParseError::Cancelled => (0, e.to_string()),
                    crate::error::ParseError::Recovered { location, .. } => {
                        (*location, e.to_string())
                    }
                };
                let message =
                    match perl_lsp_rs_core::providers::diagnostics::build_parse_error_hint(
                        e,
                        &base_message,
                    ) {
                        Some(hint) => format!("{base_message}\nSuggestion: {hint}"),
                        None => base_message,
                    };
                let (line, character) = pos16(location);
                json!({
                    "range": {
                        "start": {"line": line, "character": character},
                        "end": {"line": line, "character": character + 1},
                    },
                    "severity": 1, // Error
                    "code": DiagnosticCode::ParseError.as_str(),
                    "source": "perl-parser",
                    "message": Self::diagnostic_message_value(
                        &message,
                        None,
                        markup_message_support,
                    ),
                })
            })
            .collect()
    }

    /// Push-path publication restricted to parse errors. See
    /// [`Self::publish_diagnostics`] for the full pipeline.
    fn publish_syntax_only_diagnostics(&self, uri: &str) {
        let normalized_uri = self.normalize_uri_key(uri);

        let snapshot = {
            let documents = self.documents.lock();
            documents.get(&normalized_uri).or_else(|| documents.get(uri)).map(|doc| {
                (
                    doc.parse_errors.clone(),
                    doc.text.clone(),
                    doc.version,
                    doc.line_starts.clone(),
                    doc.rope.clone(),
                    Arc::clone(&doc.generation),
                    doc.generation.load(Ordering::SeqCst),
                )
            })
        };

        let Some((parse_errors, text, version, line_starts, rope, generation, gen_at_snapshot)) =
            snapshot
        else {
            return;
        };

        let lsp_diagnostics =
            Self::syntax_only_lsp_diagnostics(&parse_errors, &text, &line_starts, &rope, false);

        // Generation-aware staleness guard mirrors the full path.
        if generation.load(Ordering::SeqCst) != gen_at_snapshot {
            tracing::debug!(
                uri = %normalized_uri,
                gen_at_snapshot,
                current_gen = generation.load(Ordering::SeqCst),
                "Skipping stale syntax-only diagnostic publish (generation advanced)"
            );
            return;
        }

        tracing::debug!(
            count = lsp_diagnostics.len(),
            uri = %normalized_uri,
            version,
            "Publishing syntax-only diagnostics"
        );

        if let Err(e) = self.notify(
            "textDocument/publishDiagnostics",
            json!({
                "uri": uri,
                "version": version,
                "diagnostics": lsp_diagnostics
            }),
        ) {
            tracing::error!(uri, error = %e, "Failed to publish syntax-only diagnostics");
        }
    }

    /// Publish parse-error diagnostics immediately (fast path, ~10ms).
    ///
    /// Called on `didChange` before the full debounced diagnostic cycle so that
    /// syntax errors are visible to the user as soon as parsing completes, without
    /// waiting for the 250ms debounce that gates the slower scope-analysis and
    /// perlcritic passes.
    ///
    /// Only emits a notification when:
    /// - The client uses push diagnostics (no pull-diagnostic capability), AND
    /// - The document has at least one parse error to report.
    ///
    /// The slow path (`publish_diagnostics`) will follow and replace this
    /// notification with the full diagnostic set â€” LSP publishDiagnostics is
    /// replace-mode, so the client never sees a partial accumulation.
    pub(crate) fn publish_parse_errors_fast(&self, uri: &str) {
        // Fast path is only meaningful for push-diagnostic clients.
        // Pull-diagnostic clients request diagnostics on-demand.
        if self.client_supports_pull_diags.load(Ordering::Relaxed) {
            return;
        }

        let normalized_uri = self.normalize_uri_key(uri);
        let snapshot = {
            let documents = self.documents.lock();
            documents.get(&normalized_uri).or_else(|| documents.get(uri)).map(|doc| {
                (
                    doc.parse_errors.clone(),
                    doc.version,
                    doc.line_starts.clone(),
                    doc.rope.clone(),
                    doc.text.clone(),
                )
            })
            // lock is released here
        };
        let Some((parse_errors, version, line_starts, rope, text)) = snapshot else { return };

        // Nothing to fast-publish when there are no parse errors.
        if parse_errors.is_empty() {
            return;
        }

        let pos16 = |offset: usize| line_starts.offset_to_position_rope(&rope, offset);

        let lsp_diagnostics: Vec<Value> =
            parse_errors
                .iter()
                .map(|e| {
                    let (location, base_message) = match e {
                        crate::error::ParseError::UnexpectedToken { location, expected, found } => {
                            (*location, format!("Expected {}, found {}", expected, found))
                        }
                        crate::error::ParseError::SyntaxError { location, message } => {
                            (*location, message.clone())
                        }
                        crate::error::ParseError::UnexpectedEof => {
                            (text.len(), "Unexpected end of input".to_string())
                        }
                        crate::error::ParseError::LexerError { message } => (0, message.clone()),
                        _ => (0, e.to_string()),
                    };
                    let message =
                        match perl_lsp_rs_core::providers::diagnostics::build_parse_error_hint(
                            e,
                            &base_message,
                        ) {
                            Some(hint) => format!("{base_message}\nSuggestion: {hint}"),
                            None => base_message,
                        };
                    let (line, character) = pos16(location);
                    json!({
                        "range": {
                            "start": {"line": line, "character": character},
                            "end": {"line": line, "character": character + 1},
                        },
                        "severity": 1,
                        "code": DiagnosticCode::ParseError.as_str(),
                        "source": "perl-parser",
                        "message": message,
                    })
                })
                .collect();

        tracing::debug!(
            count = lsp_diagnostics.len(),
            uri = %normalized_uri,
            version,
            "Publishing fast parse-error diagnostics"
        );

        if let Err(e) = self.notify(
            "textDocument/publishDiagnostics",
            json!({
                "uri": uri,
                "version": version,
                "diagnostics": lsp_diagnostics
            }),
        ) {
            tracing::error!(uri, error = %e, "Failed to publish fast parse-error diagnostics");
        }
    }

    /// Handle textDocument/diagnostic request (pull diagnostics - LSP 3.17)
    ///
    /// Computes diagnostics for a single document using the pull-based model
    /// introduced in LSP 3.17. Uses PullDiagnosticsProvider with context
    /// from the orchestrator for clean separation of concerns.
    ///
    /// # LSP Protocol
    ///
    /// Request: `textDocument/diagnostic`
    /// Response: `DocumentDiagnosticReport`
    /// Capability: `textDocument.diagnostic`
    ///
    /// # Arguments
    ///
    /// * `params` - JSON-RPC parameters containing document URI and optional previousResultId
    ///
    /// # Returns
    ///
    /// DocumentDiagnosticReport with kind "unchanged" or "full" depending on content changes
    ///
    /// # Caching Strategy
    ///
    /// Uses MD5 hash of document content as result ID for efficient change detection.
    /// Returns "unchanged" response when content hash matches previousResultId.
    pub(super) fn handle_document_diagnostic(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        use crate::features::diagnostics::PullDiagnosticsProvider;
        use lsp_types::Uri;

        if let Some(params) = params {
            let uri_str = params["textDocument"]["uri"].as_str().unwrap_or("");
            let previous_result_id = params["previousResultId"].as_str().map(|s| s.to_string());

            // Parse URI
            let uri: Uri = match uri_str.parse() {
                Ok(u) => u,
                Err(_) => {
                    return Ok(Some(json!({
                        "kind": "full",
                        "items": []
                    })));
                }
            };

            // Syntax-only short-circuit for pull diagnostics. Mirrors the
            // push-path gate in `publish_diagnostics`.
            if self.runtime_tuning.diagnostic_mode
                == perl_lsp_rs_core::runtime::tuning::DiagnosticMode::SyntaxOnly
            {
                let doc_snapshot = {
                    let documents = self.documents.lock();
                    self.get_document(&documents, uri_str).cloned()
                };
                if let Some(doc) = doc_snapshot {
                    let markup_message_support =
                        self.client_capabilities.lock().markup_message_support;
                    let items = Self::syntax_only_lsp_diagnostics(
                        &doc.parse_errors,
                        &doc.text,
                        &doc.line_starts,
                        &doc.rope,
                        markup_message_support,
                    );
                    return Ok(Some(json!({
                        "kind": "full",
                        "items": items,
                    })));
                }
                let _ = previous_result_id;
                return Ok(Some(json!({ "kind": "full", "items": [] })));
            }

            // Snapshot the document
            let doc_snapshot = {
                let documents = self.documents.lock();
                self.get_document(&documents, uri_str).cloned()
            };

            if let Some(doc) = doc_snapshot {
                // Build context from server state
                let context = self.pull_diagnostics_orchestrator.build_context(self, uri_str);

                // Use PullDiagnosticsProvider for clean, testable logic
                let provider = PullDiagnosticsProvider::new();
                let report = provider.get_document_diagnostics_with_context(
                    &uri,
                    &doc.text,
                    previous_result_id,
                    &context,
                    Some(&doc),
                );

                // Collect external perlcritic diagnostics via orchestrator
                let mut perlcritic_diags = Vec::new();
                self.pull_diagnostics_orchestrator.collect_perlcritic_diagnostics(
                    self,
                    uri_str,
                    &doc.text,
                    &mut perlcritic_diags,
                );

                // Convert report to JSON
                return Ok(Some(self.document_report_to_json(
                    &report,
                    &doc,
                    uri_str,
                    &perlcritic_diags,
                )));
            }
        }

        // Return empty diagnostics if document not found
        Ok(Some(json!({
            "kind": "full",
            "items": []
        })))
    }

    fn diagnostic_message_value(
        message: &str,
        message_data: Option<&Value>,
        markup_message_support: bool,
    ) -> Value {
        if !markup_message_support {
            return json!(message);
        }

        if let Some(markup) = message_data.and_then(|data| data.get("messageMarkup")) {
            if Self::is_markup_content_value(markup) {
                return markup.clone();
            }
        }

        json!({
            "kind": "markdown",
            "value": message,
        })
    }

    fn is_markup_content_value(value: &Value) -> bool {
        matches!(value.get("kind").and_then(Value::as_str), Some("markdown" | "plaintext"))
            && value.get("value").and_then(Value::as_str).is_some()
    }

    /// Convert DocumentDiagnosticReport to JSON, merging perlcritic diagnostics.
    fn document_report_to_json(
        &self,
        report: &lsp_types::DocumentDiagnosticReport,
        doc: &crate::state::DocumentState,
        uri: &str,
        perlcritic_diags: &[InternalDiagnostic],
    ) -> Value {
        use lsp_types::DocumentDiagnosticReport;

        match report {
            DocumentDiagnosticReport::Full(full) => {
                let markup_message_support = self.client_capabilities.lock().markup_message_support;
                let mut items: Vec<Value> = full
                    .full_document_diagnostic_report
                    .items
                    .iter()
                    .map(|d| self.lsp_diagnostic_to_json(d, doc, uri, markup_message_support))
                    .collect();

                // Add perlcritic diagnostics
                for d in perlcritic_diags {
                    items.push(self.internal_diagnostic_to_json(
                        d,
                        doc,
                        uri,
                        markup_message_support,
                    ));
                }

                json!({
                    "kind": "full",
                    "resultId": full.full_document_diagnostic_report.result_id,
                    "items": items
                })
            }
            DocumentDiagnosticReport::Unchanged(unchanged) => {
                json!({
                    "kind": "unchanged",
                    "resultId": unchanged.unchanged_document_diagnostic_report.result_id
                })
            }
        }
    }

    /// Convert LSP diagnostic to JSON value.
    fn lsp_diagnostic_to_json(
        &self,
        d: &lsp_types::Diagnostic,
        _doc: &crate::state::DocumentState,
        _uri: &str,
        markup_message_support: bool,
    ) -> Value {
        let start_line = d.range.start.line;
        let start_char = d.range.start.character;
        let end_line = d.range.end.line;
        let end_char = d.range.end.character;

        let mut diag = json!({
            "range": {
                "start": { "line": start_line, "character": start_char },
                "end": { "line": end_line, "character": end_char },
            },
            "severity": d.severity.map(|s| match s {
                lsp_types::DiagnosticSeverity::ERROR => 1,
                lsp_types::DiagnosticSeverity::WARNING => 2,
                lsp_types::DiagnosticSeverity::INFORMATION => 3,
                lsp_types::DiagnosticSeverity::HINT => 4,
                _ => 2,
            }),
            "code": d.code.as_ref().map(|c| match c {
                lsp_types::NumberOrString::String(s) => json!(s),
                lsp_types::NumberOrString::Number(n) => json!(n),
            }),
            "source": d.source,
            "message": Self::diagnostic_message_value(
                &d.message,
                d.data.as_ref(),
                markup_message_support,
            ),
        });

        if let Some(ref tags) = d.tags {
            diag["tags"] = json!(
                tags.iter()
                    .map(|t| match *t {
                        lsp_types::DiagnosticTag::UNNECESSARY => 1,
                        lsp_types::DiagnosticTag::DEPRECATED => 2,
                        _ => 0,
                    })
                    .collect::<Vec<_>>()
            );
        }

        if let Some(ref data) = d.data {
            diag["data"] = data.clone();
        }

        if let Some(ref related) = d.related_information {
            diag["relatedInformation"] = json!(related.iter().map(|ri| {
                json!({
                    "location": {
                        "uri": ri.location.uri.to_string(),
                        "range": {
                            "start": { "line": ri.location.range.start.line, "character": ri.location.range.start.character },
                            "end": { "line": ri.location.range.end.line, "character": ri.location.range.end.character },
                        }
                    },
                    "message": ri.message
                })
            }).collect::<Vec<_>>());
        }

        diag
    }

    /// Convert internal diagnostic to JSON value.
    fn internal_diagnostic_to_json(
        &self,
        d: &InternalDiagnostic,
        doc: &crate::state::DocumentState,
        uri: &str,
        markup_message_support: bool,
    ) -> Value {
        let start_pos = doc.line_starts.offset_to_position_rope(&doc.rope, d.range.0);
        let end_pos = doc.line_starts.offset_to_position_rope(&doc.rope, d.range.1);

        let mut diag = json!({
            "range": {
                "start": { "line": start_pos.0, "character": start_pos.1 },
                "end": { "line": end_pos.0, "character": end_pos.1 },
            },
            "severity": match d.severity {
                InternalDiagnosticSeverity::Error => 1,
                InternalDiagnosticSeverity::Warning => 2,
                InternalDiagnosticSeverity::Information => 3,
                InternalDiagnosticSeverity::Hint => 4,
            },
            "code": d.code,
            "source": diagnostic_source(d.code.as_deref()),
            "message": Self::diagnostic_message_value(&d.message, None, markup_message_support),
        });

        if !d.tags.is_empty() {
            diag["tags"] = json!(Self::diagnostic_tags_to_lsp(&d.tags));
        }

        if !d.related_information.is_empty() {
            diag["relatedInformation"] = json!(
                d.related_information
                    .iter()
                    .map(|ri| {
                        let ri_start =
                            doc.line_starts.offset_to_position_rope(&doc.rope, ri.location.0);
                        let ri_end =
                            doc.line_starts.offset_to_position_rope(&doc.rope, ri.location.1);
                        json!({
                            "location": {
                                "uri": uri,
                                "range": {
                                    "start": {"line": ri_start.0, "character": ri_start.1},
                                    "end":   {"line": ri_end.0,   "character": ri_end.1},
                                }
                            },
                            "message": ri.message
                        })
                    })
                    .collect::<Vec<_>>()
            );
        }

        if let Some(ref code_str) = d.code {
            let category = DiagnosticCode::parse_code(code_str)
                .map(|dc| format!("{:?}", dc.category()))
                .unwrap_or_else(|| "Other".to_string());
            let fixable = is_fixable_diagnostic(code_str);
            let tag_strings: Vec<String> = d
                .tags
                .iter()
                .map(|t| match t {
                    InternalDiagnosticTag::Unnecessary => "Unnecessary".to_string(),
                    InternalDiagnosticTag::Deprecated => "Deprecated".to_string(),
                })
                .collect();
            diag["data"] = json!({
                "code": code_str,
                "category": category,
                "fixable": fixable,
                "tags": tag_strings,
            });
        }

        diag
    }

    /// Handle workspace/diagnostic request (LSP 3.17 pull diagnostics)
    ///
    /// Computes diagnostics for all open documents in the workspace using the
    /// pull-based model. Provides efficient batch processing with incremental
    /// updates via content-based result IDs.
    ///
    /// # LSP Protocol
    ///
    /// Request: `workspace/diagnostic`
    /// Response: `WorkspaceDiagnosticReport`
    /// Capability: `workspace.diagnostics`
    ///
    /// # Arguments
    ///
    /// * `params` - JSON-RPC parameters with optional previousResultIds map
    ///
    /// # Returns
    ///
    /// WorkspaceDiagnosticReport containing document diagnostic reports with
    /// "unchanged" or "full" kind per document based on content changes
    ///
    /// # Performance
    ///
    /// - Cooperative yielding every 8 documents for responsiveness
    /// - MD5-based content hashing for efficient change detection
    /// - Lock-free document snapshot to avoid blocking other requests
    pub(super) fn handle_workspace_diagnostic(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let previous_result_ids = if let Some(params) = &params {
            if let Some(ids) = params["previousResultIds"].as_array() {
                ids.iter()
                    .filter_map(|item| {
                        let uri = item["uri"].as_str()?;
                        let id = item["value"].as_str()?;
                        Some((uri.to_string(), id.to_string()))
                    })
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let mut items = Vec::new();
        let markup_message_support = self.client_capabilities.lock().markup_message_support;

        // Collect document snapshots without holding lock
        let docs_snapshot: Vec<(String, DocumentState)> = {
            let documents = self.documents.lock();
            documents.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        for (i, (uri_str, doc)) in docs_snapshot.iter().enumerate() {
            // Cooperative yield every 8 documents
            if i & 0x7 == 0 {
                std::thread::yield_now();
            }

            // Check if we have a previous result ID for this document
            let prev_id =
                previous_result_ids.iter().find(|(u, _)| u == uri_str).map(|(_, id)| id.clone());

            if let Some(ast) = &doc.ast {
                let provider = DiagnosticsProvider::new(ast, doc.text.clone());
                // Position-aware resolver: each `use` statement is checked against only
                // the @INC roots that are lexically active at its offset, so `no lib`
                // cancellations that precede the statement are respected.
                let resolver = |module: &str, use_site_offset: usize| {
                    self.resolve_module_to_path_with_doc_at_offset(
                        module,
                        Some(&doc.text),
                        Some(uri_str),
                        Some(use_site_offset),
                    )
                    .is_some()
                };
                let search_context = self
                    .effective_inc_context_for_doc(Some(uri_str), Some(&doc.text), None)
                    .map(|context| context.search_display_paths())
                    .unwrap_or_default();
                let source_path = source_path_from_uri(uri_str);

                // Wire semantic queries when workspace data is available for this URI.
                #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
                let mut diagnostics = {
                    let semantic_diags = self.workspace_index().and_then(|workspace_index| {
                        workspace_index.with_semantic_queries_for_uri(
                            uri_str,
                            |file_id, queries| {
                                provider.get_diagnostics_with_search_context_and_semantics(
                                    ast,
                                    &doc.parse_errors,
                                    &doc.text,
                                    Some(&resolver),
                                    &search_context,
                                    source_path.as_deref(),
                                    file_id,
                                    &queries,
                                )
                            },
                        )
                    });
                    semantic_diags.unwrap_or_else(|| {
                        provider.get_diagnostics_with_search_context(
                            ast,
                            &doc.parse_errors,
                            &doc.text,
                            Some(&resolver),
                            &search_context,
                            source_path.as_deref(),
                        )
                    })
                };
                #[cfg(not(all(feature = "workspace", not(target_arch = "wasm32"))))]
                let mut diagnostics = provider.get_diagnostics_with_search_context(
                    ast,
                    &doc.parse_errors,
                    &doc.text,
                    Some(&resolver),
                    &search_context,
                    source_path.as_deref(),
                );

                // Add native critic diagnostics when explicitly selected.
                self.collect_native_critic_diagnostics(ast, &doc.text, &mut diagnostics);

                // Add external perlcritic diagnostics (opt-in)
                self.collect_external_perlcritic_diagnostics(uri_str, &doc.text, &mut diagnostics);

                // Add dead code diagnostics from workspace-wide symbol analysis
                #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
                {
                    if let Some(workspace_index) = self.workspace_index() {
                        let dead_code_diags =
                            perl_lsp_rs_core::providers::diagnostics::detect_dead_code(
                                &workspace_index,
                                uri_str,
                                &doc.text,
                                &doc.line_starts,
                            );
                        diagnostics.extend(dead_code_diags);
                    }
                }

                // Generate result ID
                let result_id = format!("{:x}", md5::compute(&doc.text));

                // Check if unchanged
                let report = if let Some(prev) = prev_id {
                    if prev == result_id {
                        json!({
                            "uri": uri_str,
                            "version": doc.version,
                            "kind": "unchanged",
                            "resultId": prev
                        })
                    } else {
                        // Convert diagnostics (prev_id exists but content changed)
                        let lsp_diagnostics: Vec<Value> = diagnostics
                            .into_iter()
                            .enumerate()
                            .map(|(j, d)| {
                                // Cooperative yield every 32 items
                                if j & 0x1f == 0 {
                                    std::thread::yield_now();
                                }
                                let start_pos =
                                    doc.line_starts.offset_to_position_rope(&doc.rope, d.range.0);
                                let end_pos =
                                    doc.line_starts.offset_to_position_rope(&doc.rope, d.range.1);

                                let message = match d.suggestion {
                                    Some(ref s) => format!("{}\nSuggestion: {}", d.message, s),
                                    None => d.message.clone(),
                                };

                                let mut diag = json!({
                                    "range": {
                                        "start": {
                                            "line": start_pos.0,
                                            "character": start_pos.1,
                                        },
                                        "end": {
                                            "line": end_pos.0,
                                            "character": end_pos.1,
                                        },
                                    },
                                    "severity": match d.severity {
                                        InternalDiagnosticSeverity::Error => 1,
                                        InternalDiagnosticSeverity::Warning => 2,
                                        InternalDiagnosticSeverity::Information => 3,
                                        InternalDiagnosticSeverity::Hint => 4,
                                    },
                                    "code": d.code.clone(),
                                    "source": diagnostic_source(d.code.as_deref()),
                                    "message": Self::diagnostic_message_value(
                                        &message,
                                        None,
                                        markup_message_support,
                                    ),
                                });
                                if !d.tags.is_empty() {
                                    diag["tags"] = json!(Self::diagnostic_tags_to_lsp(&d.tags));
                                }
                                if !d.related_information.is_empty() {
                                    diag["relatedInformation"] = json!(
                                        d.related_information.iter().map(|ri| {
                                            let ri_start = doc.line_starts.offset_to_position_rope(&doc.rope, ri.location.0);
                                            let ri_end = doc.line_starts.offset_to_position_rope(&doc.rope, ri.location.1);
                                            json!({
                                                "location": {
                                                    "uri": uri_str,
                                                    "range": {
                                                        "start": {"line": ri_start.0, "character": ri_start.1},
                                                        "end":   {"line": ri_end.0,   "character": ri_end.1},
                                                    }
                                                },
                                                "message": ri.message
                                            })
                                        }).collect::<Vec<_>>()
                                    );
                                }
                                if let Some(ref code_str) = d.code {
                                    let category = DiagnosticCode::parse_code(code_str)
                                        .map(|dc| format!("{:?}", dc.category()))
                                        .unwrap_or_else(|| "Other".to_string());
                                    let fixable = is_fixable_diagnostic(code_str);
                                    let tag_strings: Vec<String> =
                                        d.tags.iter().map(|t| match t {
                                            InternalDiagnosticTag::Unnecessary => "Unnecessary".to_string(),
                                            InternalDiagnosticTag::Deprecated => "Deprecated".to_string(),
                                        }).collect();
                                    diag["data"] = json!({
                                        "code": code_str,
                                        "category": category,
                                        "fixable": fixable,
                                        "tags": tag_strings,
                                    });
                                }
                                diag
                            })
                            .collect();

                        json!({
                            "uri": uri_str,
                            "version": doc.version,
                            "kind": "full",
                            "resultId": result_id,
                            "items": lsp_diagnostics
                        })
                    }
                } else {
                    // No previous result, return full
                    let lsp_diagnostics: Vec<Value> = diagnostics
                        .into_iter()
                        .enumerate()
                        .map(|(j, d)| {
                            // Cooperative yield every 32 items
                            if j & 0x1f == 0 {
                                std::thread::yield_now();
                            }
                            let start_pos =
                                doc.line_starts.offset_to_position_rope(&doc.rope, d.range.0);
                            let end_pos =
                                doc.line_starts.offset_to_position_rope(&doc.rope, d.range.1);

                            let message = match d.suggestion {
                                Some(ref s) => format!("{}\nSuggestion: {}", d.message, s),
                                None => d.message.clone(),
                            };

                            let mut diag = json!({
                                "range": {
                                    "start": {
                                        "line": start_pos.0,
                                        "character": start_pos.1,
                                    },
                                    "end": {
                                        "line": end_pos.0,
                                        "character": end_pos.1,
                                    },
                                },
                                "severity": match d.severity {
                                    InternalDiagnosticSeverity::Error => 1,
                                    InternalDiagnosticSeverity::Warning => 2,
                                    InternalDiagnosticSeverity::Information => 3,
                                    InternalDiagnosticSeverity::Hint => 4,
                                },
                                "code": d.code.clone(),
                                "source": diagnostic_source(d.code.as_deref()),
                                "message": Self::diagnostic_message_value(
                                    &message,
                                    None,
                                    markup_message_support,
                                ),
                            });
                            if !d.tags.is_empty() {
                                diag["tags"] = json!(Self::diagnostic_tags_to_lsp(&d.tags));
                            }
                            if !d.related_information.is_empty() {
                                diag["relatedInformation"] = json!(
                                    d.related_information.iter().map(|ri| {
                                        let ri_start = doc.line_starts.offset_to_position_rope(&doc.rope, ri.location.0);
                                        let ri_end = doc.line_starts.offset_to_position_rope(&doc.rope, ri.location.1);
                                        json!({
                                            "location": {
                                                "uri": uri_str,
                                                "range": {
                                                    "start": {"line": ri_start.0, "character": ri_start.1},
                                                    "end":   {"line": ri_end.0,   "character": ri_end.1},
                                                }
                                            },
                                            "message": ri.message
                                        })
                                    }).collect::<Vec<_>>()
                                );
                            }
                            if let Some(ref code_str) = d.code {
                                let category = DiagnosticCode::parse_code(code_str)
                                    .map(|dc| format!("{:?}", dc.category()))
                                    .unwrap_or_else(|| "Other".to_string());
                                let fixable = is_fixable_diagnostic(code_str);
                                let tag_strings: Vec<String> =
                                    d.tags.iter().map(|t| match t {
                                        InternalDiagnosticTag::Unnecessary => "Unnecessary".to_string(),
                                        InternalDiagnosticTag::Deprecated => "Deprecated".to_string(),
                                    }).collect();
                                diag["data"] = json!({
                                    "code": code_str,
                                    "category": category,
                                    "fixable": fixable,
                                    "tags": tag_strings,
                                });
                            }
                            diag
                        })
                        .collect();

                    json!({
                        "uri": uri_str,
                        "version": doc.version,
                        "kind": "full",
                        "resultId": result_id,
                        "items": lsp_diagnostics
                    })
                };

                items.push(report);
            }
        }

        Ok(Some(json!({ "items": items })))
    }

    fn collect_policy_critic_diagnostics(
        &self,
        ast: &std::sync::Arc<perl_parser::ast::Node>,
        doc_text: &str,
        diagnostics: &mut Vec<InternalDiagnostic>,
    ) {
        let critic_engine = { self.config.lock().critic_engine };
        match critic_engine {
            perl_lsp_rs_core::config::CriticEngine::Legacy => {
                let built_in_analyzer = BuiltInAnalyzer::new();
                let violations = built_in_analyzer.analyze(ast, doc_text);
                diagnostics.extend(violations.iter().map(builtin_violation_to_diagnostic));
            }
            perl_lsp_rs_core::config::CriticEngine::Native => {
                self.collect_native_critic_diagnostics(ast, doc_text, diagnostics);
            }
        }
    }

    fn collect_native_critic_diagnostics(
        &self,
        ast: &std::sync::Arc<perl_parser::ast::Node>,
        doc_text: &str,
        diagnostics: &mut Vec<InternalDiagnostic>,
    ) {
        let (critic_engine, severity, profile, native_profile, native_include, native_exclude) = {
            let cfg = self.config.lock();
            (
                cfg.critic_engine,
                cfg.perlcritic_severity,
                cfg.perlcritic_profile.clone(),
                cfg.native_critic_profile.clone(),
                cfg.native_critic_include.clone(),
                cfg.native_critic_exclude.clone(),
            )
        };
        if critic_engine != perl_lsp_rs_core::config::CriticEngine::Native {
            return;
        }

        let critic_config = crate::perl_critic::CriticConfig {
            severity: severity.clamp(1, 5),
            profile,
            include: native_include,
            exclude: native_exclude,
            ..crate::perl_critic::CriticConfig::default()
        };
        let critic_context =
            crate::perl_critic::CriticContext::new(doc_text, ast.as_ref(), &critic_config);
        let profile = crate::perl_critic::NativeCriticProfile::parse(&native_profile)
            .unwrap_or(crate::perl_critic::NativeCriticProfile::Strict);
        let registry = crate::perl_critic::NativeCriticRegistry::for_profile(profile);

        diagnostics
            .extend(registry.check(&critic_context).into_iter().map(native_finding_to_diagnostic));
    }

    /// Collect external perlcritic diagnostics if the feature is enabled.
    ///
    /// Checks the `perlcritic_enabled` config flag and whether `perlcritic` is
    /// installed on the system. If both conditions are met, runs perlcritic on
    /// the file and appends violations with severity mapped from Perl::Critic's
    /// 1-5 scale to LSP severity levels (5 -> Error, 4/3 -> Warning,
    /// 2 -> Information, 1 -> Hint).
    ///
    /// The `CriticAnalyzer` is reused across calls via `self.critic_analyzer`
    /// so that the per-file violation cache survives between `didChange` events.
    /// `invalidate_cache` is called from the `didChange` handler, and the
    /// analyzer is reset to `None` from `didChangeConfiguration` whenever any
    /// critic-related setting changes.
    ///
    /// Emits a workspace-scoped warning when perlcritic is unavailable,
    /// configured profile is missing, or execution fails.
    /// Skips file-local diagnostics for those tooling-state errors.
    /// The `doc_text` parameter is used to convert perlcritic's line/column
    /// positions into byte offsets for the internal diagnostic range.
    #[cfg(not(target_arch = "wasm32"))]
    fn collect_external_perlcritic_diagnostics(
        &self,
        uri: &str,
        doc_text: &str,
        diagnostics: &mut Vec<InternalDiagnostic>,
    ) {
        // Check config: perlcritic must be explicitly enabled (opt-in)
        let (enabled, severity, profile, theme, critic_engine) = {
            let cfg = self.config.lock();
            (
                cfg.perlcritic_enabled,
                cfg.perlcritic_severity,
                cfg.perlcritic_profile.clone(),
                cfg.perlcritic_theme.clone(),
                cfg.critic_engine,
            )
        };
        if !enabled || critic_engine == perl_lsp_rs_core::config::CriticEngine::Native {
            return;
        }
        let profile = profile.and_then(|profile| (!profile.trim().is_empty()).then_some(profile));

        // Convert URI to file system path; skip non-file URIs
        let file_path = match url::Url::parse(uri) {
            Ok(u) => match u.to_file_path() {
                Ok(p) => p,
                Err(()) => {
                    tracing::warn!(uri, "perlcritic: URI is not a file path");
                    return;
                }
            },
            Err(e) => {
                tracing::warn!(uri, error = %e, "perlcritic: failed to parse URI");
                return;
            }
        };

        // Warn the user once if perlcritic is not installed.
        // The `skip_perlcritic_command_check` flag is always `false` in production
        // and is only set to `true` through the test helper
        // `LspServer::test_bypass_perlcritic_command_check`, enabling mock-runtime
        // injection without a real `perlcritic` binary.
        let skip_check =
            self.skip_perlcritic_command_check.load(std::sync::atomic::Ordering::Relaxed);
        if !skip_check && !crate::execute_command::command_exists("perlcritic") {
            self.emit_perlcritic_workspace_warning(
                "missing-binary".to_string(),
                "Perl::Critic is enabled but `perlcritic` was not found on PATH. Install Perl::Critic (for example: `cpanm Perl::Critic`) or disable perl.perlcritic.enabled.",
            );
            return;
        }

        let workspace_root = workspace_root_for_doc(self, uri);
        let resolved_configured_profile = if let Some(ref configured_profile) = profile {
            let resolved = resolve_configured_profile_path(
                configured_profile,
                workspace_root.as_deref(),
                &file_path,
            );
            if resolved.is_none() {
                self.emit_perlcritic_workspace_warning(
                    format!("missing-profile:{configured_profile}"),
                    &format!(
                        "Perl::Critic profile path does not exist: {configured_profile}. Update perl.perlcritic.profile or create the profile file."
                    ),
                );
                return;
            }
            resolved
        } else {
            None
        };

        // Lazy-init the shared CriticAnalyzer.  If the profile or severity
        // changed, `didChangeConfiguration` has already reset the field to
        // `None`, so we rebuild here with the current config.
        //
        // The `.perlcriticrc` walk-up is intentionally placed inside the
        // `is_none()` branch so that filesystem stat calls are skipped on
        // every subsequent diagnostic cycle once the analyzer is warm.
        {
            let mut guard = self.critic_analyzer.lock();
            if guard.is_none() {
                // Walk up the directory tree from the file's parent to the
                // workspace root looking for `.perlcriticrc`.  Ensures that a
                // repo-root config is found even when the file lives in a
                // sub-directory.  Only runs when the analyzer needs (re-)init.
                let resolved_profile = resolved_configured_profile
                    .as_ref()
                    .and_then(|p| p.to_str().map(|s| s.to_string()))
                    .or_else(|| {
                        find_workspace_perlcritic_profile(workspace_root.as_deref(), &file_path)
                    });
                let critic_config = crate::perl_critic::CriticConfig {
                    severity,
                    profile: resolved_profile,
                    theme: theme.clone(),
                    ..crate::perl_critic::CriticConfig::default()
                };
                // Use the injected test runtime when present; otherwise fall back
                // to the OS subprocess runtime.
                let analyzer = {
                    let rt_guard = self.critic_runtime_override.lock();
                    if let Some(ref rt) = *rt_guard {
                        crate::perl_critic::CriticAnalyzer::new(
                            critic_config,
                            std::sync::Arc::clone(rt),
                        )
                    } else {
                        crate::perl_critic::CriticAnalyzer::with_os_runtime(critic_config)
                    }
                };
                *guard = Some(analyzer);
            }
        }

        // Compute a content hash so the cache can detect stale entries when the
        // file changes without triggering a `didChange` LSP event (e.g. external
        // editor or `git checkout` while the server is running).
        let content_hash = crate::perl_critic::hash_content(doc_text);

        // Borrow the shared analyzer to run the analysis.  The lock is held
        // only for the duration of the `analyze_file_with_hash` call.
        let result = {
            let mut guard = self.critic_analyzer.lock();
            guard.as_mut().map(|a| a.analyze_file_with_hash(&file_path, content_hash))
        };

        match result {
            Some(Ok(violations)) => {
                for v in violations {
                    // Map Perl::Critic severity (1-5) to LSP DiagnosticSeverity:
                    // 5 -> Error, 4/3 -> Warning, 2 -> Information, 1 -> Hint
                    let internal_severity = match v.severity {
                        crate::perl_critic::Severity::Gentle => InternalDiagnosticSeverity::Error,
                        crate::perl_critic::Severity::Stern
                        | crate::perl_critic::Severity::Harsh => {
                            InternalDiagnosticSeverity::Warning
                        }
                        crate::perl_critic::Severity::Cruel => {
                            InternalDiagnosticSeverity::Information
                        }
                        crate::perl_critic::Severity::Brutal => InternalDiagnosticSeverity::Hint,
                    };

                    // Convert 0-indexed line/column from CriticAnalyzer to byte offsets.
                    let line_0 = v.range.start.line;
                    let col_0 = v.range.start.column;
                    let start_byte = position_to_offset(doc_text, line_0, col_0).unwrap_or(0);
                    let end_byte =
                        position_to_offset(doc_text, v.range.end.line, v.range.end.column)
                            .unwrap_or(start_byte.saturating_add(1));

                    diagnostics.push(InternalDiagnostic {
                        range: (start_byte, end_byte),
                        severity: internal_severity,
                        code: Some(v.policy),
                        message: v.description,
                        related_information: Vec::new(),
                        tags: Vec::new(),
                        suggestion: None,
                    });
                }
            }
            Some(Err(e)) => {
                self.emit_perlcritic_workspace_warning(
                    format!("execution-failed:{e}"),
                    &format!("Perl::Critic execution failed: {e}"),
                );
                tracing::warn!(uri, error = %e, "perlcritic failed");
            }
            None => {}
        }
    }

    /// No-op stub for WASM targets where subprocess execution is unavailable.
    #[cfg(target_arch = "wasm32")]
    fn collect_external_perlcritic_diagnostics(
        &self,
        _uri: &str,
        _doc_text: &str,
        _diagnostics: &mut Vec<InternalDiagnostic>,
    ) {
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn emit_perlcritic_workspace_warning(&self, key: String, message: &str) {
        let mut sent = self.critic_workspace_warnings_sent.lock();
        if sent.insert(key) {
            let _ = self.show_message(super::window::MessageType::Warning, message);
        }
    }
}

/// Returns `true` when a quick-fix code action exists for the given diagnostic code.
///
/// Mirrors the list in `crates/perl-lsp-rs/src/features/diagnostics/pull.rs`.
/// The authoritative source is `crates/perl-lsp-code-actions/src/code_actions.rs`.
fn is_fixable_diagnostic(code: &str) -> bool {
    is_fixable_perlcritic_policy(code)
        || matches!(
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

fn is_fixable_perlcritic_policy(code: &str) -> bool {
    matches!(
        code,
        "InputOutput::ProhibitBarewordFileHandles"
            | "InputOutput::RequireBriefOpen"
            | "InputOutput::RequireThreeArgOpen"
            | "TestingAndDebugging::RequireUseStrict"
            | "TestingAndDebugging::RequireUseWarnings"
            | "native.testing.require_use_strict"
            | "native.testing.require_use_warnings"
            | "native.common.undef_comparison"
            | "native.io.bareword_filehandle"
            | "native.io.two_arg_open"
            | "Variables::ProhibitUnusedVariables"
    )
}

fn diagnostic_source(code: Option<&str>) -> &'static str {
    match code {
        Some(code) if code.starts_with("native.") => "perl-lsp-critic",
        Some(code) if code.contains("::") && DiagnosticCode::parse_code(code).is_none() => {
            "perlcritic"
        }
        _ => "perl-lsp",
    }
}

fn push_diagnostic_source(code: Option<&str>) -> &'static str {
    match code {
        Some(code) if code.starts_with("native.") => "perl-lsp-critic",
        _ => "perl-parser",
    }
}

fn native_finding_to_diagnostic(finding: crate::perl_critic::CriticFinding) -> InternalDiagnostic {
    InternalDiagnostic {
        range: (finding.range.start.byte, finding.range.end.byte),
        severity: critic_severity_to_internal(finding.severity),
        code: Some(finding.rule_id),
        message: finding.message,
        related_information: Vec::new(),
        tags: Vec::new(),
        suggestion: None,
    }
}

fn critic_severity_to_internal(
    severity: crate::perl_critic::Severity,
) -> InternalDiagnosticSeverity {
    match severity {
        crate::perl_critic::Severity::Gentle => InternalDiagnosticSeverity::Error,
        crate::perl_critic::Severity::Stern | crate::perl_critic::Severity::Harsh => {
            InternalDiagnosticSeverity::Warning
        }
        crate::perl_critic::Severity::Cruel => InternalDiagnosticSeverity::Information,
        crate::perl_critic::Severity::Brutal => InternalDiagnosticSeverity::Hint,
    }
}

/// Convert a built-in analyzer violation to an internal diagnostic.
fn builtin_violation_to_diagnostic(
    violation: &crate::perl_critic::Violation,
) -> InternalDiagnostic {
    InternalDiagnostic {
        range: (violation.range.start.byte, violation.range.end.byte),
        severity: critic_severity_to_internal(violation.severity),
        code: Some(violation.policy.clone()),
        message: violation.description.clone(),
        related_information: Vec::new(),
        tags: Vec::new(),
        suggestion: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use std::sync::Arc as StdArc;
    use std::time::Duration;

    /// Shared-buffer writer for capturing outbound LSP notifications in tests.
    struct SharedVecWriter {
        inner: StdArc<parking_lot::Mutex<Vec<u8>>>,
    }
    impl Write for SharedVecWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.inner.lock().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    fn make_server_with_capture() -> (LspServer, StdArc<parking_lot::Mutex<Vec<u8>>>) {
        let buf = StdArc::new(parking_lot::Mutex::new(Vec::<u8>::new()));
        let writer = SharedVecWriter { inner: StdArc::clone(&buf) };
        let server =
            LspServer::with_io(Box::new(std::io::Cursor::new(Vec::<u8>::new())), Box::new(writer));
        (server, buf)
    }

    /// Positive case: when no concurrent change arrives during diagnostic computation,
    /// `publish_diagnostics` MUST send a `textDocument/publishDiagnostics` notification.
    #[test]
    fn stable_generation_publishes_diagnostics() {
        let (server, buf) = make_server_with_capture();
        let uri = "file:///stable_gen_test.pl";
        server
            .test_handle_did_open(Some(json!({
                "textDocument": {"uri": uri, "languageId": "perl", "version": 1, "text": "my $x = 1;\n"}
            })))
            .unwrap();

        // No concurrent change: generation is stable throughout, publish must fire.
        server.publish_diagnostics(uri);
        drop(server);
        std::thread::sleep(Duration::from_millis(50)); // flush outbound writer

        let bytes = buf.lock().clone();
        let text = String::from_utf8(bytes).unwrap_or_default();
        assert!(
            text.contains("publishDiagnostics"),
            "stable generation must produce a publishDiagnostics notification; got: {text:?}"
        );
    }

    /// Guard wire test: advancing the generation counter before `publish_diagnostics`
    /// is called must not suppress publication â€” the snapshot captures the CURRENT
    /// generation, so stable-during-computation is still the common case.
    /// This confirms the guard does not false-positive.
    #[test]
    fn pre_advanced_generation_does_not_suppress_publish() {
        let (server, buf) = make_server_with_capture();
        let uri = "file:///pre_advanced_gen_test.pl";
        server
            .test_handle_did_open(Some(json!({
                "textDocument": {"uri": uri, "languageId": "perl", "version": 1, "text": "my $y = 2;\n"}
            })))
            .unwrap();

        // Advance generation BEFORE calling publish_diagnostics (simulates a prior
        // didChange that already completed). The snapshot will read this new value,
        // computation runs, and the guard check sees the same value â†’ publishes.
        {
            let docs = server.documents.lock();
            if let Some(doc) = docs.get(uri) {
                doc.generation.fetch_add(1, Ordering::SeqCst);
            }
        }

        server.publish_diagnostics(uri);
        drop(server);
        std::thread::sleep(Duration::from_millis(50));

        let bytes = buf.lock().clone();
        let text = String::from_utf8(bytes).unwrap_or_default();
        assert!(
            text.contains("publishDiagnostics"),
            "pre-advanced generation must not suppress publish (guard must not false-positive); got: {text:?}"
        );
    }

    #[test]
    fn native_critic_engine_publishes_native_policy_diagnostics() {
        let (server, buf) = make_server_with_capture();
        server.test_configure_critic_engine(perl_lsp_rs_core::config::CriticEngine::Native);
        server.test_configure_native_critic_profile("strict");
        let uri = "file:///native_critic_push_test.pl";
        server
            .test_handle_did_open(Some(json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "my $x = 1;\nmy $x = 2;\nmy $unused = 3;\nmy $shadow = 4;\nmy $outer_param = 0;\nmy $cond = 0;\nmy $path = 'file.txt';\nmy $eval_code = 'print 1';\nmy $cmd_out = `ls`;\nmy $qx_out = qx(date);\nmy $readpipe_out = readpipe($path);\nif ($cond = 1) { print $cond; }\nif ($path == undef) { print $path; }\neval { die $path; };\nif ($@) { warn $@; }\nopen(FH, '<', 'file.txt');\nopen(my $log_fh, $path);\nopen(my $pipe_fh, '-|', 'ls');\neval $eval_code;\nsystem($path);\nexec('ls', '-la');\nprint $log_fh;\nprint $pipe_fh;\n{ my $shadow = 5; print $shadow; }\nsub helper($used_param, $unused_param) { return $used_param; }\nsub duplicate_param($dup_param, $dup_param) { return $dup_param; }\nsub shadow_param($outer_param) { return $outer_param; }\nsub unreachable_helper { return 1; my $dead_after_return = 2; }\nprint $x + $shadow + $outer_param + $cond + $cmd_out + $qx_out + $readpipe_out;\n"
                }
            })))
            .unwrap();

        server.publish_diagnostics(uri);
        drop(server);
        std::thread::sleep(Duration::from_millis(50));

        let bytes = buf.lock().clone();
        let text = String::from_utf8(bytes).unwrap_or_default();
        assert!(
            text.contains("native.testing.require_use_strict"),
            "native critic engine should publish native strict finding; got: {text:?}"
        );
        assert!(
            text.contains("native.testing.require_use_warnings"),
            "native critic engine should publish native warnings finding; got: {text:?}"
        );
        assert!(
            text.contains("native.common.assignment_in_condition"),
            "native critic engine should publish native assignment-in-condition finding; got: {text:?}"
        );
        assert!(
            text.contains("Assignment in condition - did you mean '=='?"),
            "native assignment-in-condition finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.common.undef_comparison"),
            "native critic engine should publish native undef-comparison finding; got: {text:?}"
        );
        assert!(
            text.contains("Using '==' with undef -- use defined() to check first"),
            "native undef-comparison finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.common.stale_dollar_at"),
            "native critic engine should publish native stale-dollar-at finding; got: {text:?}"
        );
        assert!(
            text.contains("Checking $@ after eval can observe a stale error"),
            "native stale-dollar-at finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.common.unreachable_code"),
            "native critic engine should publish native unreachable-code finding; got: {text:?}"
        );
        assert!(
            text.contains("Unreachable code: this statement cannot be executed"),
            "native unreachable-code finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.io.bareword_filehandle"),
            "native critic engine should publish native bareword filehandle finding; got: {text:?}"
        );
        assert!(
            text.contains("Bareword filehandle 'FH' should be lexical"),
            "native bareword filehandle finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.io.two_arg_open"),
            "native critic engine should publish native two-arg open finding; got: {text:?}"
        );
        assert!(
            text.contains("Two-argument open should use an explicit mode"),
            "native two-arg open finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.io.pipe_open"),
            "native critic engine should publish native pipe-open finding; got: {text:?}"
        );
        assert!(
            text.contains("Pipe-open executes a shell command"),
            "native pipe-open finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.io.unchecked_open_close"),
            "native critic engine should publish native unchecked open/close finding; got: {text:?}"
        );
        assert!(
            text.contains("open() return value should be checked"),
            "native unchecked open/close finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.security.backtick_exec"),
            "native critic engine should publish native backtick execution finding; got: {text:?}"
        );
        assert!(
            text.contains("Command execution detected"),
            "native backtick execution finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.security.qx_readpipe"),
            "native critic engine should publish native qx/readpipe finding; got: {text:?}"
        );
        assert!(
            text.contains("qx/readpipe command execution detected"),
            "native qx/readpipe finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.security.string_eval"),
            "native critic engine should publish native string eval finding; got: {text:?}"
        );
        assert!(
            text.contains("String eval is a security risk"),
            "native string eval finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.security.system_exec"),
            "native critic engine should publish native system/exec finding; got: {text:?}"
        );
        assert!(
            text.contains("system() executes a shell command"),
            "native system/exec finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.variables.unused_lexical"),
            "native critic engine should publish native unused lexical finding; got: {text:?}"
        );
        assert!(
            text.contains("Lexical variable '$unused' is declared but never used"),
            "native unused lexical finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.variables.unused_parameter"),
            "native critic engine should publish native unused parameter finding; got: {text:?}"
        );
        assert!(
            text.contains("Parameter '$unused_param' is never used"),
            "native unused parameter finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.variables.duplicate_parameter"),
            "native critic engine should publish native duplicate parameter finding; got: {text:?}"
        );
        assert!(
            text.contains("Parameter '$dup_param' appears more than once in this signature"),
            "native duplicate parameter finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.variables.parameter_shadows_global"),
            "native critic engine should publish native parameter shadowing finding; got: {text:?}"
        );
        assert!(
            text.contains("Parameter '$outer_param' shadows an outer declaration"),
            "native parameter shadowing finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.variables.duplicate_lexical"),
            "native critic engine should publish native duplicate lexical finding; got: {text:?}"
        );
        assert!(
            text.contains("Lexical variable '$x' is declared more than once in the same scope"),
            "native duplicate lexical finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("native.variables.shadowed_lexical"),
            "native critic engine should publish native shadowed lexical finding; got: {text:?}"
        );
        assert!(
            text.contains("Lexical variable '$shadow' shadows an outer declaration"),
            "native shadowed lexical finding should preserve rule message; got: {text:?}"
        );
        assert!(
            text.contains("\"source\":\"perl-lsp-critic\""),
            "native critic diagnostics should use perl-lsp-critic source; got: {text:?}"
        );
        assert!(
            !text.contains("TestingAndDebugging::RequireUseStrict"),
            "native critic engine should not publish legacy built-in critic policy IDs; got: {text:?}"
        );
    }

    #[test]
    fn native_critic_recommended_profile_publishes_lower_noise_policy_diagnostics() {
        let (server, buf) = make_server_with_capture();
        server.test_configure_critic_engine(perl_lsp_rs_core::config::CriticEngine::Native);
        server.test_configure_native_critic_profile("recommended");
        let uri = "file:///native_critic_recommended_push_test.pl";
        server
            .test_handle_did_open(Some(json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "my $unused = 1;\nmy $cond = 0;\nif ($cond = 1) { print $cond; }\n"
                }
            })))
            .unwrap();

        server.publish_diagnostics(uri);
        drop(server);
        std::thread::sleep(Duration::from_millis(50));

        let bytes = buf.lock().clone();
        let text = String::from_utf8(bytes).unwrap_or_default();
        assert!(
            text.contains("native.testing.require_use_strict"),
            "recommended native critic profile should publish strict finding; got: {text:?}"
        );
        assert!(
            text.contains("native.common.assignment_in_condition"),
            "recommended native critic profile should publish common-mistake findings; got: {text:?}"
        );
        assert!(
            !text.contains("native.variables.unused_lexical"),
            "recommended native critic profile should omit broader variable-noise findings; got: {text:?}"
        );
    }

    #[test]
    fn native_critic_push_diagnostics_honor_include_and_exclude_filters() {
        let (server, buf) = make_server_with_capture();
        server.test_configure_critic_engine(perl_lsp_rs_core::config::CriticEngine::Native);
        server.test_configure_native_critic_profile("recommended");
        server.test_configure_native_critic_filters(
            vec!["native.testing.require_use_strict".to_string()],
            vec!["native.common.assignment_in_condition".to_string()],
        );
        let uri = "file:///native_critic_filtered_push_test.pl";
        server
            .test_handle_did_open(Some(json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "my $cond = 0;\nif ($cond = 1) { print $cond; }\n"
                }
            })))
            .unwrap();

        server.publish_diagnostics(uri);
        drop(server);
        std::thread::sleep(Duration::from_millis(50));

        let bytes = buf.lock().clone();
        let text = String::from_utf8(bytes).unwrap_or_default();
        assert!(
            text.contains("native.testing.require_use_strict"),
            "native include should keep selected strict rule; got: {text:?}"
        );
        assert!(
            !text.contains("native.common.assignment_in_condition"),
            "native exclude should suppress assignment rule; got: {text:?}"
        );
        assert!(
            !text.contains("native.testing.require_use_warnings"),
            "native include should suppress non-included warning rule; got: {text:?}"
        );
    }

    #[test]
    fn legacy_critic_engine_keeps_legacy_policy_diagnostics_for_push() {
        let (server, buf) = make_server_with_capture();
        server.test_configure_critic_engine(perl_lsp_rs_core::config::CriticEngine::Legacy);
        let uri = "file:///legacy_critic_push_test.pl";
        server
            .test_handle_did_open(Some(json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "my $x = 1;\n"
                }
            })))
            .unwrap();

        server.publish_diagnostics(uri);
        drop(server);
        std::thread::sleep(Duration::from_millis(50));

        let bytes = buf.lock().clone();
        let text = String::from_utf8(bytes).unwrap_or_default();
        assert!(
            text.contains("TestingAndDebugging::RequireUseStrict"),
            "legacy critic engine should keep built-in strict policy ID; got: {text:?}"
        );
        assert!(
            !text.contains("native.testing.require_use_strict"),
            "explicit legacy critic engine should not publish native policy IDs; got: {text:?}"
        );
    }

    #[test]
    fn push_pl701_uses_effective_inc_context_labels() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(workspace.join("lib"))?;
        let script = workspace.join("script.pl");
        let uri = url::Url::from_file_path(&script)
            .map_err(|()| "failed to build script URI")?
            .to_string();
        let folder_uri = url::Url::from_directory_path(&workspace)
            .map_err(|()| "failed to build workspace URI")?
            .to_string();

        let (server, buf) = make_server_with_capture();
        *server.root_path.lock() = Some(workspace.clone());
        let mut config = perl_lsp_rs_core::config::WorkspaceConfig::default();
        config.include_paths = vec!["lib".to_string()];
        config.use_system_inc = false;
        config.use_perl5lib = false;
        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(folder_uri)
                .with_path(workspace.clone())
                .with_effective_workspace_config(config),
        );

        server.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "use Missing::From::Lib;\n"
            }
        })))?;

        server.publish_diagnostics(&uri);
        drop(server);
        std::thread::sleep(Duration::from_millis(50));

        let bytes = buf.lock().clone();
        let text = String::from_utf8(bytes)?;
        assert!(text.contains("PL701"), "missing module should publish PL701; got: {text:?}");
        assert!(
            text.contains("Searched @INC"),
            "PL701 should include searched @INC context; got: {text:?}"
        );
        assert!(
            text.contains("workspace includePaths"),
            "PL701 should label the include root source; got: {text:?}"
        );
        Ok(())
    }

    #[test]
    fn native_critic_engine_adds_native_workspace_diagnostics()
    -> Result<(), Box<dyn std::error::Error>> {
        let (server, _buf) = make_server_with_capture();
        server.test_configure_critic_engine(perl_lsp_rs_core::config::CriticEngine::Native);
        server.test_configure_native_critic_profile("strict");
        let uri = "file:///native_critic_workspace_test.pl";
        server.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "my $x = 1;\nmy $x = 2;\nmy $unused = 3;\nmy $shadow = 4;\nmy $outer_param = 0;\nmy $cond = 0;\nmy $path = 'file.txt';\nmy $eval_code = 'print 1';\nmy $cmd_out = `ls`;\nmy $qx_out = qx(date);\nmy $readpipe_out = readpipe($path);\nif ($cond = 1) { print $cond; }\nif ($path == undef) { print $path; }\neval { die $path; };\nif ($@) { warn $@; }\nopen(FH, '<', 'file.txt');\nopen(my $log_fh, $path);\nopen(my $pipe_fh, '-|', 'ls');\neval $eval_code;\nsystem($path);\nexec('ls', '-la');\nprint $log_fh;\nprint $pipe_fh;\n{ my $shadow = 5; print $shadow; }\nsub helper($used_param, $unused_param) { return $used_param; }\nsub duplicate_param($dup_param, $dup_param) { return $dup_param; }\nsub shadow_param($outer_param) { return $outer_param; }\nsub unreachable_helper { return 1; my $dead_after_return = 2; }\nprint $x + $shadow + $outer_param + $cond + $cmd_out + $qx_out + $readpipe_out;\n"
            }
        })))?;

        let report = server
            .handle_workspace_diagnostic(Some(json!({
                "identifier": "perl-lsp",
                "previousResultIds": []
            })))?
            .ok_or("workspace diagnostic response missing")?;
        let items = report["items"].as_array().ok_or("workspace diagnostic items missing")?;
        let file_report = items
            .iter()
            .find(|item| item["uri"].as_str() == Some(uri))
            .ok_or("workspace diagnostic report missing opened document")?;
        let diagnostics =
            file_report["items"].as_array().ok_or("workspace diagnostic report missing items")?;

        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.testing.require_use_strict")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
            }),
            "native critic engine should add native strict finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.testing.require_use_warnings")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
            }),
            "native critic engine should add native warnings finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.common.assignment_in_condition")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str()
                        == Some("Assignment in condition - did you mean '=='?")
            }),
            "native critic engine should add native assignment-in-condition finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.common.undef_comparison")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str()
                        == Some("Using '==' with undef -- use defined() to check first")
            }),
            "native critic engine should add native undef-comparison finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.common.stale_dollar_at")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str()
                        == Some("Checking $@ after eval can observe a stale error")
            }),
            "native critic engine should add native stale-dollar-at finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.common.unreachable_code")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str()
                        == Some("Unreachable code: this statement cannot be executed")
            }),
            "native critic engine should add native unreachable-code finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.io.bareword_filehandle")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str()
                        == Some("Bareword filehandle 'FH' should be lexical")
            }),
            "native critic engine should add native bareword filehandle finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.io.two_arg_open")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str()
                        == Some("Two-argument open should use an explicit mode")
            }),
            "native critic engine should add native two-arg open finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.io.pipe_open")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str() == Some("Pipe-open executes a shell command")
            }),
            "native critic engine should add native pipe-open finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.io.unchecked_open_close")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str() == Some("open() return value should be checked")
            }),
            "native critic engine should add native unchecked open/close finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.security.backtick_exec")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str() == Some("Command execution detected")
            }),
            "native critic engine should add native backtick execution finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.security.qx_readpipe")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str() == Some("qx/readpipe command execution detected")
            }),
            "native critic engine should add native qx/readpipe finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.security.string_eval")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str() == Some("String eval is a security risk")
            }),
            "native critic engine should add native string eval finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.security.system_exec")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str() == Some("system() executes a shell command")
            }),
            "native critic engine should add native system/exec finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.variables.unused_lexical")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str()
                        == Some("Lexical variable '$unused' is declared but never used")
            }),
            "native critic engine should add native unused lexical finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.variables.unused_parameter")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str() == Some("Parameter '$unused_param' is never used")
            }),
            "native critic engine should add native unused parameter finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.variables.duplicate_parameter")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str()
                        == Some("Parameter '$dup_param' appears more than once in this signature")
            }),
            "native critic engine should add native duplicate parameter finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.variables.parameter_shadows_global")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str()
                        == Some("Parameter '$outer_param' shadows an outer declaration")
            }),
            "native critic engine should add native parameter shadowing finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.variables.duplicate_lexical")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str()
                        == Some(
                            "Lexical variable '$x' is declared more than once in the same scope",
                        )
            }),
            "native critic engine should add native duplicate lexical finding to workspace diagnostics: {report}"
        );
        assert!(
            diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("native.variables.shadowed_lexical")
                    && diag["source"].as_str() == Some("perl-lsp-critic")
                    && diag["message"].as_str()
                        == Some("Lexical variable '$shadow' shadows an outer declaration")
            }),
            "native critic engine should add native shadowed lexical finding to workspace diagnostics: {report}"
        );
        assert!(
            !diagnostics.iter().any(|diag| {
                diag["code"].as_str() == Some("TestingAndDebugging::RequireUseStrict")
            }),
            "native critic workspace diagnostics should not publish legacy built-in policy IDs: {report}"
        );

        Ok(())
    }

    /// Positive case: `publish_parse_errors_fast` must immediately emit a
    /// `textDocument/publishDiagnostics` notification when the document has
    /// parse errors and the client uses push diagnostics.
    #[test]
    fn fast_path_emits_notification_for_documents_with_parse_errors() {
        let (server, buf) = make_server_with_capture();
        let uri = "file:///fast_path_parse_err_test.pl";
        // Open a document with a deliberate syntax error.
        server
            .test_handle_did_open(Some(json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "sub { SYNTAX ERROR HERE }\n"
                }
            })))
            .unwrap();

        server.publish_parse_errors_fast(uri);
        // Drop server to flush the writer thread, then inspect the buffer.
        drop(server);

        let bytes = buf.lock().clone();
        let text = String::from_utf8(bytes).unwrap_or_default();
        assert!(
            text.contains("publishDiagnostics"),
            "fast path must emit publishDiagnostics when parse errors exist; got: {text:?}"
        );
    }

    /// Guard: `publish_parse_errors_fast` with a pull-diagnostic client must NOT
    /// emit any notification (pull clients handle diagnostics on demand).
    #[test]
    fn fast_path_silent_for_pull_diagnostic_clients() {
        let (server, buf) = make_server_with_capture();
        let uri = "file:///fast_path_pull_diags_test.pl";
        // Simulate a client that supports pull diagnostics by setting the flag.
        server.client_supports_pull_diags.store(true, Ordering::Relaxed);
        server
            .test_handle_did_open(Some(json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    // Intentional syntax error â€” fast path would fire if not guarded.
                    "text": "sub { SYNTAX ERROR }\n"
                }
            })))
            .unwrap();

        // Record buffer length before the fast-path call.
        let len_before = buf.lock().len();
        server.publish_parse_errors_fast(uri);
        drop(server);
        let len_after = buf.lock().len();

        assert_eq!(
            len_before,
            len_after,
            "fast path must not emit any bytes for pull-diagnostic clients; \
             buffer grew by {} bytes",
            len_after.saturating_sub(len_before)
        );
    }

    /// Guard: the full diagnostic path must also stay silent for pull clients.
    /// This prevents didOpen from doing slow push-diagnostic work for clients
    /// that will request diagnostics on demand.
    #[test]
    fn slow_path_silent_for_pull_diagnostic_clients() -> Result<(), Box<dyn std::error::Error>> {
        let (server, buf) = make_server_with_capture();
        let uri = "file:///slow_path_pull_diags_test.pl";
        server.client_supports_pull_diags.store(true, Ordering::Relaxed);
        server.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "sub { SYNTAX ERROR }\n"
            }
        })))?;

        assert_eq!(
            0,
            buf.lock().len(),
            "didOpen must not emit push diagnostics for pull-diagnostic clients"
        );

        server.publish_diagnostics(uri);
        drop(server);
        let bytes = buf.lock().clone();
        let text = String::from_utf8(bytes)?;
        assert!(
            !text.contains("publishDiagnostics"),
            "slow path must not emit push diagnostics for pull-diagnostic clients; got: {text:?}"
        );

        Ok(())
    }

    #[test]
    fn builtin_violation_maps_gentle_to_error() {
        let violation = crate::perl_critic::Violation {
            policy: "GentlePolicy".to_string(),
            description: "gentle".to_string(),
            explanation: String::new(),
            severity: crate::perl_critic::Severity::Gentle,
            range: perl_parser::position::Range {
                start: perl_parser::position::Position { byte: 0, line: 0, column: 0 },
                end: perl_parser::position::Position { byte: 0, line: 0, column: 1 },
            },
            file: "test.pl".to_string(),
        };

        let diagnostic = builtin_violation_to_diagnostic(&violation);
        assert_eq!(diagnostic.severity, InternalDiagnosticSeverity::Error);
        assert_eq!(diagnostic.code.as_deref(), Some("GentlePolicy"));
    }

    // --- build_context workspace root tests ---

    /// build_context must use the workspace root of the folder that owns the document,
    /// not the global root_path (which points to the first workspace folder).
    #[test]
    fn build_context_uses_doc_scoped_workspace_root() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let folder_a = temp.path().join("folder-a");
        let folder_b = temp.path().join("folder-b");
        let script_b = folder_b.join("script.pl");
        std::fs::create_dir_all(&folder_a)?;
        std::fs::create_dir_all(&folder_b)?;
        std::fs::write(&script_b, "use strict;\n")?;

        let doc_uri = url::Url::from_file_path(&script_b).map_err(|_| "bad uri")?.to_string();

        let (server, _buf) = make_server_with_capture();
        // root_path points to folder_a (the "primary" folder)
        *server.root_path.lock() = Some(folder_a.clone());
        {
            let mut folders = server.workspace_folders.lock();
            folders.push(
                crate::runtime::workspace_folder::WorkspaceFolderState::new(
                    url::Url::from_directory_path(&folder_a).map_err(|_| "bad uri_a")?.to_string(),
                )
                .with_path(folder_a.clone()),
            );
            folders.push(
                crate::runtime::workspace_folder::WorkspaceFolderState::new(
                    url::Url::from_directory_path(&folder_b).map_err(|_| "bad uri_b")?.to_string(),
                )
                .with_path(folder_b.clone()),
            );
        }

        let orchestrator = PullDiagnosticsOrchestrator::new();
        let context = orchestrator.build_context(&server, &doc_uri);

        assert_eq!(
            context.workspace_root.as_deref(),
            Some(folder_b.as_path()),
            "workspace_root must be the folder containing the document, not root_path"
        );
        Ok(())
    }

    /// When no workspace folder contains the document, build_context must fall back
    /// to the global root_path.
    #[test]
    fn build_context_falls_back_to_root_path_when_no_folder_matches()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let outside = temp.path().join("outside");
        let script = outside.join("script.pl");
        std::fs::create_dir_all(&workspace)?;
        std::fs::create_dir_all(&outside)?;
        std::fs::write(&script, "use strict;\n")?;

        let doc_uri = url::Url::from_file_path(&script).map_err(|_| "bad uri")?.to_string();

        let (server, _buf) = make_server_with_capture();
        *server.root_path.lock() = Some(workspace.clone());
        {
            let mut folders = server.workspace_folders.lock();
            folders.push(
                crate::runtime::workspace_folder::WorkspaceFolderState::new(
                    url::Url::from_directory_path(&workspace)
                        .map_err(|_| "bad folder uri")?
                        .to_string(),
                )
                .with_path(workspace.clone()),
            );
        }

        let orchestrator = PullDiagnosticsOrchestrator::new();
        let context = orchestrator.build_context(&server, &doc_uri);

        assert_eq!(
            context.workspace_root.as_deref(),
            Some(workspace.as_path()),
            "workspace_root must fall back to root_path when no folder contains the document"
        );
        Ok(())
    }

    /// Regression guard for scenario_14_no_lib_cancellation:
    ///
    /// When `use lib 'lib'` is followed by `no lib 'lib'` and then `use GoneModule`,
    /// push diagnostics must emit PL701 (missing module) for GoneModule, NOT PL700
    /// (unused import). PL700 would mean the resolver found the module, which would
    /// be wrong — the `no lib` should have cancelled the path.
    ///
    /// This test uses the DEFAULT WorkspaceConfig (include_paths = ["lib", ".", ...])
    /// to match the UX harness scenario where no explicit includePaths are configured.
    #[test]
    fn push_pl701_fires_after_no_lib_cancels_default_include_path()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(workspace.join("lib"))?;
        // Create GoneModule.pm in lib/ so that without no-lib it WOULD be found.
        std::fs::write(
            workspace.join("lib").join("GoneModule.pm"),
            "package GoneModule;\nsub gone { 1 }\n1;\n",
        )?;
        let script = workspace.join("fixture.pl");
        let uri = url::Url::from_file_path(&script)
            .map_err(|()| "failed to build script URI")?
            .to_string();
        let folder_uri = url::Url::from_directory_path(&workspace)
            .map_err(|()| "failed to build workspace URI")?
            .to_string();

        let (server, buf) = make_server_with_capture();
        *server.root_path.lock() = Some(workspace.clone());
        // Use default config — include_paths = ["lib", ".", "local/lib/perl5"], use_perl5lib = true.
        // This matches the UX harness startup state (no workspace/didChangeConfiguration sent).
        let config = perl_lsp_rs_core::config::WorkspaceConfig::default();
        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(folder_uri)
                .with_path(workspace.clone())
                .with_effective_workspace_config(config),
        );

        let source = "use strict;\n\
use warnings;\n\
use lib 'lib';\n\
no lib 'lib';\n\
use GoneModule;\n\
\n\
print \"unreachable\\n\";\n";

        server.test_handle_did_open(Some(serde_json::json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": source
            }
        })))?;

        server.publish_diagnostics(&uri);
        drop(server);
        std::thread::sleep(Duration::from_millis(50));

        let bytes = buf.lock().clone();
        let text = String::from_utf8(bytes)?;

        assert!(
            text.contains("PL701"),
            "PL701 MUST fire: 'no lib' cancelled the path, GoneModule must not be found.\n\
             Published: {text:?}"
        );
        assert!(
            !text.contains("PL700"),
            "PL700 must not fire after 'no lib' cancels the path; that would mean \
             the missing module was still treated as resolved.\n\
             Published: {text:?}"
        );
        Ok(())
    }
}
