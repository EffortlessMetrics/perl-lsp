//! Completion request handlers
//!
//! Handles textDocument/completion requests with support for:
//! - Variable completion (scalars, arrays, hashes)
//! - Function/subroutine completion
//! - Keyword completion
//! - Workspace-wide symbol completion
//! - Cancellation support

use crate::cancellation::{
    GLOBAL_CANCELLATION_REGISTRY, PerlLspCancellationToken, RequestCleanupGuard,
};
use crate::completion::{
    CompletionItemKind, CompletionProvider, add_xs_api_completions_for_prefix,
};
use crate::{
    protocol::{JsonRpcError, REQUEST_CANCELLED, req_position, req_uri},
    runtime::routing::{IndexAccessMode, route_index_access},
    state::{completion_cap, completion_deadline},
};
use perl_lexer::LSP_RUNTIME_COMPLETION_KEYWORDS;
use perl_parser::type_inference::TypeInferenceEngine;
use regex::Regex;
use serde_json::{Value, json};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use super::super::LspServer;

static SNIPPET_PLACEHOLDER_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static SNIPPET_SIMPLE_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();

fn get_snippet_placeholder_regex() -> Option<&'static Regex> {
    SNIPPET_PLACEHOLDER_RE.get_or_init(|| Regex::new(r"\$\{(\d+):([^}]+)\}")).as_ref().ok()
}

fn get_snippet_simple_regex() -> Option<&'static Regex> {
    SNIPPET_SIMPLE_RE.get_or_init(|| Regex::new(r"\$\d+")).as_ref().ok()
}

/// Returns commit characters for a completion item based on its kind.
/// Each string is exactly one character, per the LSP 3.x spec.
///
/// Returns a static slice to avoid per-item heap allocation in the completion
/// serialization hot path (called once per item, up to `completion_cap()` times
/// per request, on every keypress in editors).
fn commit_chars_for_kind(kind: CompletionItemKind) -> Option<&'static [&'static str]> {
    match kind {
        CompletionItemKind::Function => Some(&["(", ",", ";"]),
        CompletionItemKind::Variable => Some(&["[", "{", ".", ";"]),
        CompletionItemKind::Module => Some(&[":", ";"]),
        CompletionItemKind::Constant => Some(&["[", "{", ".", ";"]),
        CompletionItemKind::Property => Some(&[",", "}"]),
        _ => None,
    }
}

impl LspServer {
    fn split_sigil(name: &str) -> (Option<char>, &str) {
        let mut chars = name.chars();
        match chars.next() {
            Some(sigil @ ('$' | '@' | '%')) => (Some(sigil), &name[sigil.len_utf8()..]),
            _ => (None, name),
        }
    }

    fn workspace_symbol_qualified_name(symbol: &crate::workspace_index::WorkspaceSymbol) -> String {
        match symbol.kind {
            crate::workspace_index::SymbolKind::Variable(_) => {
                if let Some(container) = symbol.container_name.as_ref() {
                    let (sigil, bare_name) = Self::split_sigil(&symbol.name);
                    format!("{}{container}::{bare_name}", sigil.unwrap_or('$'))
                } else {
                    symbol.name.clone()
                }
            }
            _ => symbol
                .qualified_name
                .clone()
                .or_else(|| {
                    symbol
                        .container_name
                        .as_ref()
                        .map(|container| format!("{container}::{}", symbol.name))
                })
                .unwrap_or_else(|| symbol.name.clone()),
        }
    }

    fn qualified_variable_workspace_symbols(
        index: &crate::workspace_index::WorkspaceIndex,
        prefix: &str,
    ) -> Option<Vec<crate::workspace_index::WorkspaceSymbol>> {
        let (requested_sigil, prefix_body) = Self::split_sigil(prefix);
        let requested_sigil = requested_sigil?;
        let mut parts: Vec<&str> = prefix_body.split("::").collect();
        if parts.len() < 2 {
            return None;
        }

        let member_prefix = parts.pop().unwrap_or("");
        let package_name = parts.join("::");

        Some(
            index
                .get_package_members(&package_name)
                .into_iter()
                .filter(|symbol| match symbol.kind {
                    crate::workspace_index::SymbolKind::Variable(_) => {
                        let (symbol_sigil, bare_name) = Self::split_sigil(&symbol.name);
                        symbol_sigil == Some(requested_sigil)
                            && bare_name.starts_with(member_prefix)
                    }
                    _ => false,
                })
                .collect(),
        )
    }

    fn workspace_symbol_kind(
        symbol: &crate::workspace_index::WorkspaceSymbol,
    ) -> CompletionItemKind {
        match symbol.kind {
            crate::workspace_index::SymbolKind::Package => CompletionItemKind::Module,
            crate::workspace_index::SymbolKind::Subroutine => CompletionItemKind::Function,
            crate::workspace_index::SymbolKind::Variable(_) => CompletionItemKind::Variable,
            crate::workspace_index::SymbolKind::Class => CompletionItemKind::Module,
            crate::workspace_index::SymbolKind::Method => CompletionItemKind::Function,
            crate::workspace_index::SymbolKind::Constant => CompletionItemKind::Constant,
            crate::workspace_index::SymbolKind::Role => CompletionItemKind::Module,
            crate::workspace_index::SymbolKind::Import => CompletionItemKind::Module,
            crate::workspace_index::SymbolKind::Export => CompletionItemKind::Function,
            crate::workspace_index::SymbolKind::Label => CompletionItemKind::Keyword,
            crate::workspace_index::SymbolKind::Format => CompletionItemKind::Function,
        }
    }

    fn add_runtime_workspace_completions(
        &self,
        completions: &mut Vec<crate::completion::CompletionItem>,
        doc_text: &str,
        offset: usize,
        workspace_mode: &IndexAccessMode,
        cap: usize,
    ) {
        match workspace_mode {
            IndexAccessMode::Full(coordinator) => {
                let index = coordinator.index();

                let text_before = &doc_text[..offset.min(doc_text.len())];
                let prefix = text_before
                    .chars()
                    .rev()
                    .take_while(|&c| {
                        c.is_alphanumeric()
                            || c == '_'
                            || c == ':'
                            || c == '$'
                            || c == '@'
                            || c == '%'
                    })
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect::<String>();

                let qualified_variable_symbols =
                    Self::qualified_variable_workspace_symbols(index, &prefix);
                let replace_prefix_range = (offset.saturating_sub(prefix.len()), offset);
                let qualified_variable_context = qualified_variable_symbols.is_some();
                let workspace_symbols =
                    qualified_variable_symbols.unwrap_or_else(|| index.find_symbols(&prefix));
                use std::collections::HashSet;
                let mut seen: HashSet<String> =
                    completions.iter().map(|completion| completion.label.clone()).collect();

                for symbol in workspace_symbols {
                    if completions.len() >= cap {
                        tracing::debug!(cap, "Completion: cap reached, stopping workspace scan");
                        break;
                    }

                    if seen.contains(&symbol.name) {
                        continue;
                    }

                    let label = symbol.name.clone();
                    let qualified_name = Self::workspace_symbol_qualified_name(&symbol);
                    let detail = Some(qualified_name.clone());
                    let (insert_text, text_edit_range) = if qualified_variable_context
                        && matches!(symbol.kind, crate::workspace_index::SymbolKind::Variable(_))
                    {
                        (Some(qualified_name), Some(replace_prefix_range))
                    } else {
                        (Some(label.clone()), None)
                    };
                    seen.insert(label.clone());

                    completions.push(crate::completion::CompletionItem {
                        label,
                        kind: Self::workspace_symbol_kind(&symbol),
                        detail,
                        insert_text,
                        sort_text: None,
                        filter_text: None,
                        documentation: Self::workspace_symbol_documentation(&symbol),
                        additional_edits: Vec::new(),
                        text_edit_range,
                        commit_characters: None,
                    });
                }
            }
            IndexAccessMode::Partial(reason) => {
                tracing::debug!(reason, "Completion: workspace index partial");
            }
            IndexAccessMode::None => {}
        }
    }

    fn workspace_symbol_documentation(
        symbol: &crate::workspace_index::WorkspaceSymbol,
    ) -> Option<String> {
        symbol.documentation.clone().or_else(|| {
            let qualified_name = Self::workspace_symbol_qualified_name(symbol);

            Some(match symbol.kind {
                crate::workspace_index::SymbolKind::Package => {
                    format!("Package `{qualified_name}` available in the workspace.")
                }
                crate::workspace_index::SymbolKind::Subroutine => {
                    format!("Subroutine `{qualified_name}` defined in the workspace.")
                }
                crate::workspace_index::SymbolKind::Variable(_) => {
                    format!("Variable `{qualified_name}` declared in the workspace.")
                }
                crate::workspace_index::SymbolKind::Class => {
                    format!("Class `{qualified_name}` defined in the workspace.")
                }
                crate::workspace_index::SymbolKind::Method => {
                    format!("Method `{qualified_name}` defined in the workspace.")
                }
                crate::workspace_index::SymbolKind::Constant => {
                    format!("Constant `{qualified_name}` declared in the workspace.")
                }
                crate::workspace_index::SymbolKind::Role => {
                    format!("Role `{qualified_name}` defined in the workspace.")
                }
                crate::workspace_index::SymbolKind::Import => {
                    format!("Imported module `{qualified_name}` used in the workspace.")
                }
                crate::workspace_index::SymbolKind::Export => {
                    format!("Exported function `{qualified_name}` available in the workspace.")
                }
                crate::workspace_index::SymbolKind::Label => {
                    format!("Label `{qualified_name}` declared in the workspace.")
                }
                crate::workspace_index::SymbolKind::Format => {
                    format!("Format `{qualified_name}` declared in the workspace.")
                }
            })
        })
    }

    /// Format type information concisely for completion detail
    pub(crate) fn format_type_for_detail(t: &crate::type_inference::PerlType) -> String {
        use perl_parser::type_inference::PerlType;
        match t {
            PerlType::Scalar(_) => "scalar".to_string(),
            PerlType::Array(_) => "array".to_string(),
            PerlType::Hash { .. } => "hash".to_string(),
            PerlType::Subroutine { .. } => "code".to_string(),
            PerlType::Reference(inner) => format!("ref {}", Self::format_type_for_detail(inner)),
            PerlType::Object(name) => format!("object {}", name),
            PerlType::Glob => "glob".to_string(),
            PerlType::Union(_) => "mixed".to_string(),
            PerlType::Any => "any".to_string(),
            PerlType::Void => "void".to_string(),
        }
    }

    /// Degrade snippet syntax to plaintext for clients that don't support snippets
    pub(crate) fn degrade_snippet_to_plaintext(snippet: &str) -> String {
        // Remove snippet placeholders: ${1:placeholder} -> placeholder
        let result = if let Some(placeholder_re) = get_snippet_placeholder_regex() {
            placeholder_re.replace_all(snippet, "$2")
        } else {
            std::borrow::Cow::Borrowed(snippet)
        };

        // Remove simple placeholders: $1, $0, etc.
        if let Some(simple_re) = get_snippet_simple_regex() {
            simple_re.replace_all(&result, "").to_string()
        } else {
            result.to_string()
        }
    }

    /// Handle completion request
    pub(crate) fn handle_completion(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let start = Instant::now();
        let deadline = completion_deadline();
        let cap = completion_cap();

        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            // Reject stale requests
            let req_version =
                params["textDocument"]["version"].as_i64().and_then(|n| i32::try_from(n).ok());
            self.ensure_latest(uri, req_version)?;

            // Use routing to determine workspace index access mode
            let workspace_mode = route_index_access(self.coordinator());

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                let offset = self.pos16_to_offset(doc, line, character);

                // Get completions, with fallback for missing AST
                #[cfg_attr(not(feature = "workspace"), allow(unused_mut))]
                let mut completions = if let Some(ast) = &doc.ast {
                    // Only provide workspace index when Full access is available
                    // This ensures we don't bypass routing policy
                    #[cfg(feature = "workspace")]
                    let workspace_idx = match &workspace_mode {
                        IndexAccessMode::Full(coordinator) => Some(Arc::clone(coordinator.index())),
                        _ => None,
                    };

                    #[cfg(feature = "workspace")]
                    let provider = CompletionProvider::new_with_index_and_source(
                        ast,
                        &doc.text,
                        workspace_idx,
                    );

                    #[cfg(not(feature = "workspace"))]
                    let provider =
                        CompletionProvider::new_with_index_and_source(ast, &doc.text, None);

                    let mut base_completions =
                        provider.get_completions_with_path(&doc.text, offset, Some(uri));

                    // Enhance completions with type information
                    let mut type_engine = TypeInferenceEngine::new();
                    let _ = type_engine.infer(ast); // Build type environment

                    // Add type information to completion items where possible
                    for completion in &mut base_completions {
                        // Add type detail to variables based on inferred types
                        if completion.kind == CompletionItemKind::Variable {
                            // Try to get the actual inferred type for the variable
                            let var_name =
                                completion.label.trim_start_matches(['$', '@', '%', '&']);
                            if let Some(perl_type) = type_engine.get_type_at(var_name) {
                                completion.detail = Some(Self::format_type_for_detail(&perl_type));
                            } else {
                                // Fallback to sigil-based type hint
                                let type_hint = if completion.label.starts_with('$') {
                                    "scalar"
                                } else if completion.label.starts_with('@') {
                                    "array"
                                } else if completion.label.starts_with('%') {
                                    "hash"
                                } else if completion.label.starts_with('&') {
                                    "code"
                                } else {
                                    "unknown"
                                };
                                completion.detail = Some(type_hint.to_string());
                            }
                        }
                    }

                    base_completions
                } else {
                    // Fallback: provide basic keyword completions when AST is unavailable
                    self.lexical_complete(&doc.text, offset, Some(uri))
                };

                // Add workspace-wide completions using routing policy
                #[cfg(feature = "workspace")]
                if start.elapsed() < deadline {
                    self.add_runtime_workspace_completions(
                        &mut completions,
                        &doc.text,
                        offset,
                        &workspace_mode,
                        cap,
                    );
                }

                // Apply cap before converting to JSON
                let is_incomplete = completions.len() > cap;
                completions.truncate(cap);

                // Snapshot capability flag once before the loop to avoid
                // acquiring client_capabilities lock per completion item
                let client_caps = self.client_capabilities.lock();
                let snippet_support = client_caps.snippet_support;
                let commit_chars_support = client_caps.completion_commit_characters_support;

                let items: Vec<Value> = completions
                    .into_iter()
                    .map(|c| {
                        // Determine insertTextFormat based on client capability and completion kind
                        let is_snippet = c.kind == CompletionItemKind::Snippet;
                        let insert_text_format = if is_snippet && snippet_support {
                            2 // Snippet format
                        } else {
                            1 // PlainText format
                        };

                        let mut item = json!({
                            "label": c.label,
                            "kind": match c.kind {
                                CompletionItemKind::Variable => 6,
                                CompletionItemKind::Function => 3,
                                CompletionItemKind::Keyword => 14,
                                CompletionItemKind::Module => 9,
                                CompletionItemKind::File => 17,
                                CompletionItemKind::Snippet => 15,
                                CompletionItemKind::Constant => 14,
                                CompletionItemKind::Property => 7,
                            },
                            "insertTextFormat": insert_text_format,
                        });

                        // Only include detail if it has a value
                        if let Some(detail) = c.detail {
                            item["detail"] = json!(detail);
                        }

                        // Only include insertText if it has a value
                        if let Some(mut insert_text) = c.insert_text {
                            // Degrade snippets to plaintext if client doesn't support snippets
                            if is_snippet && !snippet_support {
                                // Remove snippet syntax: $1, $0, ${1:placeholder}, etc.
                                insert_text = Self::degrade_snippet_to_plaintext(&insert_text);
                            }
                            item["insertText"] = json!(insert_text);
                        }

                        if let Some(documentation) = c.documentation {
                            item["documentation"] = json!({
                                "kind": "markdown",
                                "value": documentation
                            });
                        }

                        if commit_chars_support && let Some(chars) = commit_chars_for_kind(c.kind) {
                            item["commitCharacters"] = json!(chars);
                        }

                        // Serialize additionalTextEdits (e.g. auto-import `use Module;`)
                        if !c.additional_edits.is_empty() {
                            let edits: Vec<Value> = c
                                .additional_edits
                                .iter()
                                .map(|(loc, new_text)| {
                                    let (sl, sc) = self.offset_to_pos16(doc, loc.start);
                                    let (el, ec) = self.offset_to_pos16(doc, loc.end);
                                    json!({
                                        "range": {
                                            "start": { "line": sl, "character": sc },
                                            "end": { "line": el, "character": ec }
                                        },
                                        "newText": new_text
                                    })
                                })
                                .collect();
                            item["additionalTextEdits"] = json!(edits);
                        }

                        item
                    })
                    .collect();

                if is_incomplete {
                    tracing::debug!(
                        count = items.len(),
                        cap,
                        elapsed = ?start.elapsed(),
                        "Completion: returning items (capped)"
                    );
                } else {
                    tracing::debug!(count = items.len(), "Returning completions");
                }
                return Ok(Some(json!({"isIncomplete": is_incomplete, "items": items})));
            }
        }

        Ok(Some(json!({"isIncomplete": false, "items": []})))
    }

    /// Handle completion request with cancellation support
    pub(crate) fn handle_completion_cancellable(
        &self,
        params: Option<Value>,
        request_id: Option<&Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        // RAII guard ensures cleanup on all exit paths (early returns, errors, panics)
        let _cleanup_guard = RequestCleanupGuard::from_ref(request_id);

        if let Some(params) = params {
            // Create or get cancellation token for this request
            let token = if let Some(req_id) = request_id {
                GLOBAL_CANCELLATION_REGISTRY.get_token(req_id).unwrap_or_else(|| {
                    let token = PerlLspCancellationToken::new(
                        req_id.clone(),
                        "textDocument/completion".to_string(),
                    );
                    let _ = GLOBAL_CANCELLATION_REGISTRY.register_token(token.clone());
                    token
                })
            } else {
                PerlLspCancellationToken::new(
                    serde_json::Value::Null,
                    "textDocument/completion".to_string(),
                )
            };

            // Early cancellation check with relaxed read
            if token.is_cancelled_relaxed() {
                return Err(JsonRpcError {
                    code: REQUEST_CANCELLED,
                    message: "Request cancelled - completion provider".to_string(),
                    data: None,
                });
            }

            // Use cancellable provider method instead of delegating
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            // Reject stale requests
            let req_version =
                params["textDocument"]["version"].as_i64().and_then(|n| i32::try_from(n).ok());
            self.ensure_latest(uri, req_version)?;

            // Use routing to determine workspace index access mode
            let workspace_mode = route_index_access(self.coordinator());

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                let offset = self.pos16_to_offset(doc, line, character);

                // Create optimized cancellation callback with reduced frequency
                // Performance optimization: reduced overhead from 16.66% to <10%
                let check_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
                let cancel_fn = {
                    let token_clone = token.clone();
                    let counter = check_count.clone();
                    move || {
                        let count = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        // Adaptive checking: less frequent as processing continues
                        let check_interval = if count < 20 { 5 } else { 25 }; // Reduced from default frequency
                        count.is_multiple_of(check_interval) && token_clone.is_cancelled()
                    }
                };

                // Get completions with optimized cancellation support
                let mut completions = if let Some(ast) = &doc.ast {
                    // Only provide workspace index when Full access is available
                    // This ensures we don't bypass routing policy
                    #[cfg(feature = "workspace")]
                    let workspace_idx = match &workspace_mode {
                        IndexAccessMode::Full(coordinator) => Some(Arc::clone(coordinator.index())),
                        _ => None,
                    };

                    #[cfg(feature = "workspace")]
                    let provider = CompletionProvider::new_with_index_and_source(
                        ast,
                        &doc.text,
                        workspace_idx,
                    );
                    #[cfg(not(feature = "workspace"))]
                    let provider =
                        CompletionProvider::new_with_index_and_source(ast, &doc.text, None);

                    // Use cancellable provider method
                    provider.get_completions_with_path_cancellable(
                        &doc.text,
                        offset,
                        Some(uri),
                        &cancel_fn,
                    )
                } else {
                    self.lexical_complete(&doc.text, offset, Some(uri))
                };

                // Check for cancellation after provider call using relaxed read
                if token.is_cancelled_relaxed() {
                    return Err(JsonRpcError {
                        code: REQUEST_CANCELLED,
                        message: "Request cancelled during completion generation".to_string(),
                        data: None,
                    });
                }

                #[cfg(feature = "workspace")]
                self.add_runtime_workspace_completions(
                    &mut completions,
                    &doc.text,
                    offset,
                    &workspace_mode,
                    completion_cap(),
                );

                // Convert to JSON format with highly optimized cancellation checks
                let client_caps = self.client_capabilities.lock();
                let commit_chars_support = client_caps.completion_commit_characters_support;
                let snippet_support = client_caps.snippet_support;

                let items: Vec<Value> = completions
                    .into_iter()
                    .enumerate()
                    .filter_map(|(idx, c)| {
                        // Ultra-optimized cancellation check (every 250 items to reduce overhead to <5%)
                        if idx % 250 == 0 && idx > 0 && token.is_cancelled_relaxed() {
                            return None;
                        }

                        let mut item = json!({
                            "label": c.label,
                            "kind": match c.kind {
                                CompletionItemKind::Variable => 6,
                                CompletionItemKind::Function => 3,
                                CompletionItemKind::Keyword => 14,
                                CompletionItemKind::Module => 9,
                                CompletionItemKind::File => 17,
                                CompletionItemKind::Snippet => 15,
                                CompletionItemKind::Constant => 14,
                                CompletionItemKind::Property => 7,
                            },
                        });
                        let is_snippet = c.kind == CompletionItemKind::Snippet;
                        let insert_text_format = if is_snippet && snippet_support { 2 } else { 1 };
                        item["insertTextFormat"] = json!(insert_text_format);

                        if let Some(detail) = c.detail {
                            item["detail"] = json!(detail);
                        }
                        if let Some(mut insert_text) = c.insert_text {
                            if is_snippet && !snippet_support {
                                insert_text = Self::degrade_snippet_to_plaintext(&insert_text);
                            }
                            item["insertText"] = json!(insert_text);
                        }
                        if let Some(documentation) = c.documentation {
                            item["documentation"] = json!({
                                "kind": "markdown",
                                "value": documentation
                            });
                        }

                        if commit_chars_support && let Some(chars) = commit_chars_for_kind(c.kind) {
                            item["commitCharacters"] = json!(chars);
                        }

                        // Serialize additionalTextEdits (e.g. auto-import `use Module;`)
                        if !c.additional_edits.is_empty() {
                            let edits: Vec<Value> = c
                                .additional_edits
                                .iter()
                                .map(|(loc, new_text)| {
                                    let (sl, sc) = self.offset_to_pos16(doc, loc.start);
                                    let (el, ec) = self.offset_to_pos16(doc, loc.end);
                                    json!({
                                        "range": {
                                            "start": { "line": sl, "character": sc },
                                            "end": { "line": el, "character": ec }
                                        },
                                        "newText": new_text
                                    })
                                })
                                .collect();
                            item["additionalTextEdits"] = json!(edits);
                        }

                        Some(item)
                    })
                    .collect();

                return Ok(Some(json!({"isIncomplete": false, "items": items})));
            }

            Ok(Some(json!({"isIncomplete": false, "items": []})))
        } else {
            self.handle_completion(params)
        }
    }

    /// Lexical completion fallback for when AST is unavailable
    pub(crate) fn lexical_complete(
        &self,
        content: &str,
        offset: usize,
        filepath: Option<&str>,
    ) -> Vec<crate::completion::CompletionItem> {
        let mut completions = Vec::new();

        // Get the prefix we're completing
        let text_before = &content[..offset.min(content.len())];
        let prefix = text_before
            .chars()
            .rev()
            .take_while(|&c| c.is_alphanumeric() || c == '_')
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();
        let prefix_start = offset.saturating_sub(prefix.len());

        // Check if we're in a method call context (after ->)
        let is_method_call = text_before.ends_with("->")
            || text_before
                .chars()
                .rev()
                .skip_while(|c| c.is_alphanumeric() || *c == '_')
                .take(2)
                .collect::<String>()
                == ">-";

        // Check what sigil we're after (if any)
        let sigil = text_before.chars().rev().find(|&c| !(c.is_alphanumeric() || c == '_'));

        // If we're completing after '->', provide common method completions
        if is_method_call {
            let common_methods = [
                ("new", "constructor"),
                ("init", "initializer"),
                ("process", "processor"),
                ("run", "executor"),
                ("execute", "executor"),
                ("handle", "handler"),
                ("get", "getter"),
                ("set", "setter"),
                ("create", "constructor"),
                ("build", "builder"),
                ("parse", "parser"),
                ("format", "formatter"),
                ("validate", "validator"),
                ("transform", "transformer"),
                ("render", "renderer"),
            ];

            for (method, kind) in &common_methods {
                if method.starts_with(&prefix) || prefix.is_empty() {
                    completions.push(crate::completion::CompletionItem {
                        label: method.to_string(),
                        kind: CompletionItemKind::Function,
                        detail: Some(format!("method ({})", kind)),
                        documentation: None,
                        insert_text: Some(method.to_string()),
                        additional_edits: vec![],
                        sort_text: None,
                        filter_text: None,
                        text_edit_range: None,
                        commit_characters: None,
                    });
                }
            }
            return completions; // Return early for method completions
        }

        match sigil {
            Some('$') => {
                // Scalar variables - suggest common ones
                if "_".starts_with(&prefix) || prefix.is_empty() {
                    completions.push(crate::completion::CompletionItem {
                        label: "_".to_string(),
                        kind: CompletionItemKind::Variable,
                        detail: Some("Default variable".to_string()),
                        documentation: None,
                        insert_text: Some("_".to_string()),
                        additional_edits: vec![],
                        sort_text: None,
                        filter_text: None,
                        text_edit_range: None,
                        commit_characters: None,
                    });
                }
            }
            Some('@') => {
                // Array variables - suggest common ones
                if "ARGV".starts_with(&prefix) || prefix.is_empty() {
                    completions.push(crate::completion::CompletionItem {
                        label: "ARGV".to_string(),
                        kind: CompletionItemKind::Variable,
                        detail: Some("Command line arguments".to_string()),
                        documentation: None,
                        insert_text: Some("ARGV".to_string()),
                        additional_edits: vec![],
                        sort_text: None,
                        filter_text: None,
                        text_edit_range: None,
                        commit_characters: None,
                    });
                }
                if "_".starts_with(&prefix) || prefix.is_empty() {
                    completions.push(crate::completion::CompletionItem {
                        label: "_".to_string(),
                        kind: CompletionItemKind::Variable,
                        detail: Some("Function arguments".to_string()),
                        documentation: None,
                        insert_text: Some("_".to_string()),
                        additional_edits: vec![],
                        sort_text: None,
                        filter_text: None,
                        text_edit_range: None,
                        commit_characters: None,
                    });
                }
            }
            Some('%') => {
                // Hash variables - suggest common ones
                if "ENV".starts_with(&prefix) || prefix.is_empty() {
                    completions.push(crate::completion::CompletionItem {
                        label: "ENV".to_string(),
                        kind: CompletionItemKind::Variable,
                        detail: Some("Environment variables".to_string()),
                        documentation: None,
                        insert_text: Some("ENV".to_string()),
                        additional_edits: vec![],
                        sort_text: None,
                        filter_text: None,
                        text_edit_range: None,
                        commit_characters: None,
                    });
                }
            }
            _ => {
                add_xs_api_completions_for_prefix(
                    &mut completions,
                    &prefix,
                    prefix_start,
                    offset,
                    content,
                    filepath,
                );

                // No sigil - suggest keywords
                for kw in LSP_RUNTIME_COMPLETION_KEYWORDS {
                    if kw.starts_with(&prefix) {
                        completions.push(crate::completion::CompletionItem {
                            label: kw.to_string(),
                            kind: CompletionItemKind::Keyword,
                            detail: None,
                            documentation: None,
                            insert_text: Some(kw.to_string()),
                            additional_edits: vec![],
                            sort_text: None,
                            filter_text: None,
                            text_edit_range: None,
                            commit_characters: None,
                        });
                    }
                }
            }
        }

        completions
    }

    /// Handle completionItem/resolve request
    ///
    /// This method enriches a completion item with additional information
    /// such as documentation for built-in functions. This enables lazy loading
    /// of completion details, improving initial completion list performance.
    pub(crate) fn handle_completion_resolve(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let Some(mut item) = params else {
            return Ok(None);
        };

        // Extract the label and kind upfront (clone to avoid borrow issues)
        let label = item.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let kind = item.get("kind").and_then(|v| v.as_u64()).unwrap_or(0);
        let has_doc = item.get("documentation").is_some();

        // Check if this is a built-in function and add documentation
        let builtin_signatures = crate::builtin_signatures::create_builtin_signatures();
        if let Some(sig) = builtin_signatures.get(label.as_str()) {
            // Build markdown documentation
            let mut doc_parts = Vec::new();

            // Add signatures
            if !sig.signatures.is_empty() {
                doc_parts.push("**Signatures:**".to_string());
                for signature in &sig.signatures {
                    doc_parts.push(format!("- `{}`", signature));
                }
                doc_parts.push(String::new()); // blank line
            }

            // Add documentation
            doc_parts.push(sig.documentation.to_string());

            let documentation = doc_parts.join("\n");

            // Add documentation to the completion item
            if let Some(obj) = item.as_object_mut() {
                obj.insert(
                    "documentation".to_string(),
                    json!({
                        "kind": "markdown",
                        "value": documentation
                    }),
                );
            }
            return Ok(Some(item));
        }

        // For variables, add type hint documentation if available
        if kind == 6 && !has_doc {
            // Variable kind
            let type_doc = if label.starts_with('$') {
                Some("Scalar variable - holds a single value (string, number, or reference)")
            } else if label.starts_with('@') {
                Some("Array variable - holds an ordered list of scalars")
            } else if label.starts_with('%') {
                Some("Hash variable - holds a set of key-value pairs")
            } else {
                None
            };

            if let Some(doc) = type_doc {
                if let Some(obj) = item.as_object_mut() {
                    obj.insert(
                        "documentation".to_string(),
                        json!({
                            "kind": "markdown",
                            "value": doc
                        }),
                    );
                }
            }
            return Ok(Some(item));
        }

        // For keywords, add brief documentation
        if kind == 14 && !has_doc {
            // Keyword kind
            let keyword_doc = match label.as_str() {
                "my" => Some("Declares a lexically scoped variable"),
                "our" => Some("Declares a package variable visible to all code in its package"),
                "local" => Some("Temporarily saves and restores a variable's value"),
                "state" => Some("Declares a persistent lexical variable (Perl 5.10+)"),
                "sub" => Some("Declares a subroutine"),
                "package" => Some("Declares a namespace"),
                "use" => Some("Imports a module at compile time"),
                "require" => Some("Loads a module at runtime"),
                "if" => Some("Conditional execution"),
                "elsif" => Some("Additional conditional branch"),
                "else" => Some("Default conditional branch"),
                "unless" => Some("Negated conditional execution"),
                "while" => Some("Loop while condition is true"),
                "until" => Some("Loop until condition is true"),
                "for" => Some("C-style loop or list iteration"),
                "foreach" => Some("Iterate over a list"),
                "given" => Some("Switch statement (Perl 5.10+)"),
                "when" => Some("Case in a switch statement"),
                "default" => Some("Default case in a switch statement"),
                "return" => Some("Returns from a subroutine"),
                "last" => Some("Exits a loop immediately"),
                "next" => Some("Skips to the next iteration of a loop"),
                "redo" => Some("Restarts the current iteration without re-evaluating condition"),
                "goto" => Some("Transfers control to another location"),
                _ => None,
            };

            if let Some(doc) = keyword_doc {
                if let Some(obj) = item.as_object_mut() {
                    obj.insert(
                        "documentation".to_string(),
                        json!({
                            "kind": "markdown",
                            "value": doc
                        }),
                    );
                }
            }
        }

        Ok(Some(item))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancellable_completion_cross_file_variable() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();

        server.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": "file:///workspace/Config.pm",
                "languageId": "perl",
                "version": 1,
                "text": "package Config;\nour $CONFIG_PATH = '/etc/app.conf';\nour $DEBUG_MODE = 1;\n1;\n"
            }
        })))?;

        server.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": "file:///workspace/app.pl",
                "languageId": "perl",
                "version": 1,
                "text": "use Config;\nprint $Config::CONF\n"
            }
        })))?;

        let response = server
            .handle_completion_cancellable(
                Some(json!({
                    "textDocument": { "uri": "file:///workspace/app.pl" },
                    "position": { "line": 1, "character": 19 }
                })),
                Some(&json!(1)),
            )?
            .ok_or("expected completion response")?;

        let items = response["items"].as_array().ok_or("expected completion items")?;
        let item = items
            .iter()
            .find(|item| item["label"].as_str() == Some("$CONFIG_PATH"))
            .ok_or_else(|| format!("expected $CONFIG_PATH completion, got: {items:?}"))?;
        assert_eq!(
            item["insertText"].as_str(),
            Some("$Config::CONFIG_PATH"),
            "qualified workspace variable completion should preserve package qualifier"
        );

        Ok(())
    }

    #[test]
    fn test_completion_resolve_builtin_function() -> Result<(), Box<dyn std::error::Error>> {
        // Test that built-in function documentation is added
        let item = json!({
            "label": "print",
            "kind": 3  // Function
        });

        let server = LspServer::default();
        let result = server.handle_completion_resolve(Some(item));

        assert!(result.is_ok());
        let resolved =
            result.map_err(|e| e.message.to_string())?.ok_or("expected resolved value")?;

        // Check that documentation was added
        assert!(resolved.get("documentation").is_some());
        let doc = resolved.get("documentation").ok_or("expected documentation")?;
        assert_eq!(doc.get("kind").and_then(|v| v.as_str()), Some("markdown"));

        let value = doc.get("value").and_then(|v| v.as_str()).unwrap_or("");
        assert!(value.contains("Signatures:"));
        assert!(value.contains("print"));
        Ok(())
    }

    #[test]
    fn test_completion_resolve_keyword() -> Result<(), Box<dyn std::error::Error>> {
        // Test that keyword documentation is added
        let item = json!({
            "label": "my",
            "kind": 14  // Keyword
        });

        let server = LspServer::default();
        let result = server.handle_completion_resolve(Some(item));

        assert!(result.is_ok());
        let resolved =
            result.map_err(|e| e.message.to_string())?.ok_or("expected resolved value")?;

        // Check that documentation was added
        assert!(resolved.get("documentation").is_some());
        let doc = resolved.get("documentation").ok_or("expected documentation")?;
        let value = doc.get("value").and_then(|v| v.as_str()).unwrap_or("");
        assert!(value.contains("lexically scoped"));
        Ok(())
    }

    #[test]
    fn test_completion_resolve_variable() -> Result<(), Box<dyn std::error::Error>> {
        // Test that variable documentation is added
        let item = json!({
            "label": "$foo",
            "kind": 6  // Variable
        });

        let server = LspServer::default();
        let result = server.handle_completion_resolve(Some(item));

        assert!(result.is_ok());
        let resolved =
            result.map_err(|e| e.message.to_string())?.ok_or("expected resolved value")?;

        // Check that documentation was added
        assert!(resolved.get("documentation").is_some());
        let doc = resolved.get("documentation").ok_or("expected documentation")?;
        let value = doc.get("value").and_then(|v| v.as_str()).unwrap_or("");
        assert!(value.contains("Scalar variable"));
        Ok(())
    }

    #[test]
    fn test_completion_resolve_array_variable() -> Result<(), Box<dyn std::error::Error>> {
        // Test that array variable documentation is added
        let item = json!({
            "label": "@items",
            "kind": 6  // Variable
        });

        let server = LspServer::default();
        let result = server.handle_completion_resolve(Some(item));

        assert!(result.is_ok());
        let resolved =
            result.map_err(|e| e.message.to_string())?.ok_or("expected resolved value")?;

        // Check that documentation was added
        assert!(resolved.get("documentation").is_some());
        let doc = resolved.get("documentation").ok_or("expected documentation")?;
        let value = doc.get("value").and_then(|v| v.as_str()).unwrap_or("");
        assert!(value.contains("Array variable"));
        Ok(())
    }

    #[test]
    fn test_completion_resolve_passthrough() -> Result<(), Box<dyn std::error::Error>> {
        // Test that unknown items are passed through unchanged (except for no documentation)
        let item = json!({
            "label": "some_custom_function",
            "kind": 3  // Function
        });

        let server = LspServer::default();
        let result = server.handle_completion_resolve(Some(item.clone()));

        assert!(result.is_ok());
        let resolved =
            result.map_err(|e| e.message.to_string())?.ok_or("expected resolved value")?;

        // Label should be preserved
        assert_eq!(resolved.get("label").and_then(|v| v.as_str()), Some("some_custom_function"));
        // Kind should be preserved
        assert_eq!(resolved.get("kind").and_then(|v| v.as_u64()), Some(3));
        Ok(())
    }

    #[test]
    fn test_degrade_snippet_removes_placeholders_with_defaults() {
        // ${1:placeholder} should become "placeholder"
        let result = LspServer::degrade_snippet_to_plaintext("function(${1:arg1}, ${2:arg2})");
        assert_eq!(result, "function(arg1, arg2)");
    }

    #[test]
    fn test_degrade_snippet_removes_simple_placeholders() {
        // $1, $0 should be removed entirely
        let result = LspServer::degrade_snippet_to_plaintext("print $1;$0");
        assert_eq!(result, "print ;");
    }

    #[test]
    fn test_degrade_snippet_mixed_placeholders() {
        // Mix of both types
        let result = LspServer::degrade_snippet_to_plaintext("sub ${1:name} { $0 }");
        assert_eq!(result, "sub name {  }");
    }

    #[test]
    fn test_degrade_snippet_no_placeholders() {
        // Plain text should pass through unchanged
        let result = LspServer::degrade_snippet_to_plaintext("just plain text");
        assert_eq!(result, "just plain text");
    }

    #[test]
    fn test_degrade_snippet_empty_string() {
        let result = LspServer::degrade_snippet_to_plaintext("");
        assert_eq!(result, "");
    }
}
