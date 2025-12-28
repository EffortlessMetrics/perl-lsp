//! Completion request handlers
//!
//! Handles textDocument/completion requests with support for:
//! - Variable completion (scalars, arrays, hashes)
//! - Function/subroutine completion
//! - Keyword completion
//! - Workspace-wide symbol completion
//! - Cancellation support

use crate::{
    CompletionItemKind, CompletionProvider,
    cancellation::{GLOBAL_CANCELLATION_REGISTRY, PerlLspCancellationToken},
    lsp::protocol::{JsonRpcError, REQUEST_CANCELLED},
    type_inference::TypeInferenceEngine,
};
use serde_json::{Value, json};
use std::sync::Arc;

use super::super::LspServer;

impl LspServer {
    /// Format type information concisely for completion detail
    pub(crate) fn format_type_for_detail(t: &crate::type_inference::PerlType) -> String {
        use crate::type_inference::PerlType;
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
        use regex::Regex;

        // Remove snippet placeholders: ${1:placeholder} -> placeholder, $1 -> empty
        let placeholder_re = Regex::new(r"\$\{(\d+):([^}]+)\}").unwrap();
        let mut result = placeholder_re.replace_all(snippet, "$2").to_string();

        // Remove simple placeholders: $1, $0, etc.
        let simple_re = Regex::new(r"\$\d+").unwrap();
        result = simple_re.replace_all(&result, "").to_string();

        result
    }

    /// Handle completion request
    pub(crate) fn handle_completion(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;

            // Reject stale requests
            let req_version = params["textDocument"]["version"].as_i64().map(|n| n as i32);
            self.ensure_latest(uri, req_version)?;

            // Check index readiness (soft wait, no sleeps) - provide basic completion if not ready
            #[cfg(feature = "workspace")]
            let index_ready = self.workspace_index.as_ref().is_some_and(|idx| idx.has_symbols());

            #[cfg(not(feature = "workspace"))]
            let index_ready = false;

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                let offset = self.pos16_to_offset(doc, line, character);

                // Get completions, with fallback for missing AST
                #[cfg_attr(not(feature = "workspace"), allow(unused_mut))]
                let mut completions = if let Some(ast) = &doc.ast {
                    // Get completions from the local completion provider
                    #[cfg(feature = "workspace")]
                    let provider = CompletionProvider::new_with_index_and_source(
                        ast,
                        &doc.text,
                        self.workspace_index.clone(),
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
                    self.lexical_complete(&doc.text, offset)
                };

                // Add workspace-wide completions (functions and modules from other files)
                // Only if index is ready (soft wait, no sleeps)
                #[cfg(feature = "workspace")]
                if index_ready && let Some(ref workspace_index) = self.workspace_index {
                    // Get the current context to filter relevant completions
                    let text_before = &doc.text[..offset.min(doc.text.len())];
                    let prefix = text_before
                        .chars()
                        .rev()
                        .take_while(|&c| c.is_alphanumeric() || c == '_' || c == ':')
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect::<String>();

                    // Find matching symbols in the workspace
                    let workspace_symbols = workspace_index.find_symbols(&prefix);

                    // Add unique workspace symbols as completions
                    use std::collections::HashSet;
                    let mut seen = HashSet::new();
                    for completion in &completions {
                        seen.insert(completion.label.clone());
                    }

                    for symbol in workspace_symbols {
                        // Skip if already in local completions
                        if seen.contains(&symbol.name) {
                            continue;
                        }

                        // Add workspace symbol as completion
                        let kind = match symbol.kind {
                            crate::workspace_index::SymbolKind::Package => {
                                CompletionItemKind::Module
                            }
                            crate::workspace_index::SymbolKind::Subroutine => {
                                CompletionItemKind::Function
                            }
                            crate::workspace_index::SymbolKind::Variable => {
                                CompletionItemKind::Variable
                            }
                            crate::workspace_index::SymbolKind::Class => CompletionItemKind::Module,
                            crate::workspace_index::SymbolKind::Method => {
                                CompletionItemKind::Function
                            }
                            crate::workspace_index::SymbolKind::Constant => {
                                CompletionItemKind::Constant
                            }
                            crate::workspace_index::SymbolKind::Role => CompletionItemKind::Module,
                            crate::workspace_index::SymbolKind::Import => {
                                CompletionItemKind::Module
                            }
                            crate::workspace_index::SymbolKind::Export => {
                                CompletionItemKind::Function
                            }
                        };

                        completions.push(crate::completion::CompletionItem {
                            label: symbol.name.clone(),
                            kind,
                            detail: symbol.qualified_name,
                            insert_text: Some(symbol.name),
                            sort_text: None,
                            filter_text: None,
                            documentation: None,
                            additional_edits: Vec::new(),
                            text_edit_range: None, // Workspace completions don't need precise text edit
                        });
                    }
                }

                let items: Vec<Value> = completions
                    .into_iter()
                    .map(|c| {
                        // Determine insertTextFormat based on client capability and completion kind
                        let is_snippet = c.kind == CompletionItemKind::Snippet;
                        let insert_text_format =
                            if is_snippet && self.client_capabilities.snippet_support {
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
                            if is_snippet && !self.client_capabilities.snippet_support {
                                // Remove snippet syntax: $1, $0, ${1:placeholder}, etc.
                                insert_text = Self::degrade_snippet_to_plaintext(&insert_text);
                            }
                            item["insertText"] = json!(insert_text);
                        }

                        // Only add commit characters for functions and variables, not keywords
                        let needs_commit_chars = matches!(
                            c.kind,
                            CompletionItemKind::Function
                                | CompletionItemKind::Variable
                                | CompletionItemKind::Module
                                | CompletionItemKind::Constant
                        );
                        if needs_commit_chars {
                            item["commitCharacters"] = json!([";", " ", ")", "]", "}"]);
                        }

                        item
                    })
                    .collect();

                eprintln!("Returning {} completions", items.len());
                return Ok(Some(json!({"isIncomplete": false, "items": items})));
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
                    message: "Request cancelled".to_string(),
                    data: None,
                });
            }

            // Use cancellable provider method instead of delegating
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;

            // Reject stale requests
            let req_version = params["textDocument"]["version"].as_i64().map(|n| n as i32);
            if let Err(e) = self.ensure_latest(uri, req_version) {
                if let Some(req_id) = request_id {
                    GLOBAL_CANCELLATION_REGISTRY.remove_request(req_id);
                }
                return Err(e);
            }

            let documents = self.documents.lock().unwrap();
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
                let completions = if let Some(ast) = &doc.ast {
                    #[cfg(feature = "workspace")]
                    let provider = CompletionProvider::new_with_index_and_source(
                        ast,
                        &doc.text,
                        self.workspace_index.clone(),
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
                    self.lexical_complete(&doc.text, offset)
                };

                // Check for cancellation after provider call using relaxed read
                if token.is_cancelled_relaxed() {
                    if let Some(req_id) = request_id {
                        GLOBAL_CANCELLATION_REGISTRY.remove_request(req_id);
                    }
                    return Err(JsonRpcError {
                        code: REQUEST_CANCELLED,
                        message: "Request cancelled during completion generation".to_string(),
                        data: None,
                    });
                }

                // Convert to JSON format with highly optimized cancellation checks
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

                        if let Some(detail) = c.detail {
                            item["detail"] = json!(detail);
                        }
                        if let Some(insert_text) = c.insert_text {
                            item["insertText"] = json!(insert_text);
                        }

                        Some(item)
                    })
                    .collect();

                let result = Ok(Some(json!({"isIncomplete": false, "items": items})));
                if let Some(req_id) = request_id {
                    GLOBAL_CANCELLATION_REGISTRY.remove_request(req_id);
                }
                return result;
            }

            let result = Ok(Some(json!({"isIncomplete": false, "items": []})));

            // Cleanup token after completion
            if let Some(req_id) = request_id {
                GLOBAL_CANCELLATION_REGISTRY.remove_request(req_id);
            }

            result
        } else {
            self.handle_completion(params)
        }
    }

    /// Lexical completion fallback for when AST is unavailable
    pub(crate) fn lexical_complete(
        &self,
        content: &str,
        offset: usize,
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
                    });
                }
            }
            return completions; // Return early for method completions
        }

        // Basic keywords that match the prefix
        let keywords = [
            "my", "our", "local", "state", "sub", "package", "use", "require", "if", "elsif",
            "else", "unless", "while", "until", "for", "foreach", "given", "when", "default",
            "return", "last", "next", "redo", "goto", "die", "warn", "print", "say", "open",
            "close", "read", "write", "push", "pop", "shift", "unshift", "splice", "grep", "map",
            "sort",
        ];

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
                    });
                }
            }
            _ => {
                // No sigil - suggest keywords
                for kw in &keywords {
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
                        });
                    }
                }
            }
        }

        completions
    }
}
