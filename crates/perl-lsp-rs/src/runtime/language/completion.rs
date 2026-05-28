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
    protocol::{JsonRpcError, JsonRpcId, REQUEST_CANCELLED, req_position, req_uri},
    runtime::routing::{IndexAccessMode, route_index_access},
    state::{completion_cap, completion_deadline},
};
use perl_lexer::LSP_RUNTIME_COMPLETION_KEYWORDS;
use perl_module::resolution::{IncRoot, IncRootKind};
use perl_parser::type_inference::TypeInferenceEngine;
use regex::Regex;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use super::super::LspServer;

/// Sentinel request ID used for the notification-path token in
/// [`Self::handle_completion_cancellable`]. Real client request IDs are
/// always positive integers (per LSP convention) or strings; this negative
/// integer cannot collide with any client- or server-generated ID. The
/// token created with this ID is intentionally **not** registered in the
/// global cancellation registry — it exists only as a local handle that the
/// provider's cancel-check closure can read.
const UNCANCELLABLE_LOCAL_TOKEN_ID: JsonRpcId = JsonRpcId::Integer(-1);

static SNIPPET_PLACEHOLDER_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static SNIPPET_SIMPLE_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();

#[derive(Debug)]
struct CompletionDecisionSummary {
    compiler_fact_items: usize,
    generated_items: usize,
    dynamic_boundary_items: usize,
    fallback_items: usize,
    sample_labels: Vec<String>,
}

#[derive(Debug)]
struct CompletionDecisionContext<'a> {
    uri: &'a str,
    line: u32,
    character: u32,
    ast_available: bool,
    workspace_index_state: &'static str,
    workspace_index_reason: Option<&'static str>,
    is_incomplete: bool,
}

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
    fn record_completion_provider_decision_trace(
        &self,
        context: &CompletionDecisionContext<'_>,
        completions: &[crate::completion::CompletionItem],
    ) {
        let summary = Self::completion_decision_summary(completions);
        let item_count = completions.len();
        let fact_source = if summary.compiler_fact_items > 0 {
            "compiler_fact"
        } else if context.ast_available {
            "parser_syntax"
        } else {
            "fallback"
        };
        let fallback_state = if item_count == 0 {
            "no_result"
        } else if context.ast_available {
            "none"
        } else {
            "legacy_provider"
        };
        let reason = if item_count == 0 {
            "missing_fact"
        } else if summary.dynamic_boundary_items > 0 {
            "dynamic_boundary"
        } else if summary.generated_items > 0 {
            "generated_no_source"
        } else if summary.compiler_fact_items > 0 {
            "source_backed_high_confidence"
        } else {
            "fallback_policy"
        };

        self.record_provider_decision_trace(
            "completion",
            &json!({
                "provider": "completion",
                "provider_action": "textDocument/completion",
                "decision": if item_count > 0 { "acted" } else { "fallback" },
                "reason": reason,
                "uri": context.uri,
                "line": context.line,
                "character": context.character,
                "item_count": item_count,
                "is_incomplete": context.is_incomplete,
                "ast_available": context.ast_available,
                "fact_source": fact_source,
                "confidence": if item_count > 0 { "high" } else { "low" },
                "freshness": "fresh",
                "fallback_state": fallback_state,
                "workspace_index_state": context.workspace_index_state,
                "workspace_index_reason": context.workspace_index_reason,
                "compiler_fact_item_count": summary.compiler_fact_items,
                "generated_item_count": summary.generated_items,
                "dynamic_boundary_item_count": summary.dynamic_boundary_items,
                "fallback_candidate_count": summary.fallback_items,
                "sample_labels": summary.sample_labels,
                "claim_boundary": "records existing completion response only; no new completion candidates or ranking changes"
            }),
        );
    }

    #[cfg(feature = "workspace")]
    fn completion_workspace_index_state(
        workspace_mode: &IndexAccessMode<'_>,
    ) -> (&'static str, Option<&'static str>) {
        match workspace_mode {
            IndexAccessMode::Full(_) => ("full", None),
            IndexAccessMode::Partial(reason) => ("partial", Some(*reason)),
            IndexAccessMode::None => ("none", None),
        }
    }

    #[cfg(not(feature = "workspace"))]
    fn completion_workspace_index_state(
        workspace_mode: &IndexAccessMode,
    ) -> (&'static str, Option<&'static str>) {
        match workspace_mode {
            IndexAccessMode::Partial(reason) => ("partial", Some(*reason)),
            IndexAccessMode::None => ("none", None),
        }
    }

    fn completion_decision_summary(
        completions: &[crate::completion::CompletionItem],
    ) -> CompletionDecisionSummary {
        let sample_labels =
            completions.iter().take(5).map(|completion| completion.label.clone()).collect();
        let mut compiler_fact_items = 0;
        let mut generated_items = 0;
        let mut dynamic_boundary_items = 0;

        for completion in completions {
            let detail = completion.detail.as_deref().unwrap_or("");
            let documentation = completion.documentation.as_deref().unwrap_or("");
            let mut is_compiler_fact = detail.contains("compiler fact")
                || documentation.contains("Compiler visible-symbol");
            let is_generated = detail.contains("generated")
                || documentation.contains("generated")
                || detail.contains("framework")
                || documentation.contains("Framework");
            let is_dynamic = detail.contains("dynamic")
                || documentation.contains("dynamic")
                || detail.contains("Dynamic")
                || documentation.contains("Dynamic");

            if matches!(
                completion.kind,
                CompletionItemKind::Variable
                    | CompletionItemKind::Function
                    | CompletionItemKind::Module
                    | CompletionItemKind::Constant
                    | CompletionItemKind::Property
            ) && detail.contains("high confidence")
            {
                is_compiler_fact = true;
            }

            if is_compiler_fact {
                compiler_fact_items += 1;
            }
            if is_generated {
                generated_items += 1;
            }
            if is_dynamic {
                dynamic_boundary_items += 1;
            }
        }

        CompletionDecisionSummary {
            compiler_fact_items,
            generated_items,
            dynamic_boundary_items,
            fallback_items: completions.len().saturating_sub(compiler_fact_items),
            sample_labels,
        }
    }

    fn module_completion_roots_for_doc(
        &self,
        uri: &str,
        doc_text: &str,
        cursor_offset: usize,
    ) -> (Vec<PathBuf>, Vec<PathBuf>, bool) {
        let mut include_paths: Vec<PathBuf> = Vec::new();
        let mut seen_include: HashSet<PathBuf> = HashSet::new();
        let mut system_inc_paths: Vec<PathBuf> = Vec::new();
        let mut seen_system: HashSet<PathBuf> = HashSet::new();
        let Some(context) =
            self.effective_inc_context_for_doc(Some(uri), Some(doc_text), Some(cursor_offset))
        else {
            return (include_paths, system_inc_paths, false);
        };

        let include_workspace_roots = context.folder_uri.is_some();
        for root in &context.effective_roots {
            match root.kind {
                IncRootKind::InterpreterStartup => {
                    let resolved = root.path.clone();
                    if seen_system.insert(resolved.clone()) {
                        system_inc_paths.push(resolved);
                    }
                }
                IncRootKind::FileLocalLexical => {
                    let resolved = Self::completion_path_for_inc_root(root, &context.root);
                    if seen_include.insert(resolved.clone()) {
                        include_paths.push(resolved);
                    }
                }
                _ if include_workspace_roots => {
                    let resolved = Self::completion_path_for_inc_root(root, &context.root);
                    if seen_include.insert(resolved.clone()) {
                        include_paths.push(resolved);
                    }
                }
                _ => {}
            }
        }

        let mut include_system_inc = context.use_system_inc;

        // Preserve the historical non-workspace completion fallback: when the
        // document is outside every workspace folder, opted-in folder startup
        // @INC roots can still be used for module completion. Workspace-configured
        // roots stay excluded, while explicit file-local `use lib` roots above
        // remain eligible.
        if !include_system_inc && context.folder_uri.is_none() {
            let mut folders = self.workspace_folders.lock();
            for folder in folders.iter_mut() {
                if !folder.effective_workspace_config.use_system_inc {
                    continue;
                }
                include_system_inc = true;
                for path in folder.effective_workspace_config.get_system_inc() {
                    if seen_system.insert(path.clone()) {
                        system_inc_paths.push(path.clone());
                    }
                }
            }
        }

        (include_paths, system_inc_paths, include_system_inc)
    }

    fn completion_path_for_inc_root(root: &IncRoot, context_root: &Path) -> PathBuf {
        match root.kind {
            IncRootKind::FileLocalLexical | IncRootKind::WorkspaceRelative
                if !root.path.is_absolute() =>
            {
                context_root.join(&root.path)
            }
            _ => root.path.clone(),
        }
    }

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

    fn is_module_import_completion_context(doc_text: &str, offset: usize) -> bool {
        if !doc_text.is_char_boundary(offset) {
            return false;
        }
        let before = &doc_text[..offset];
        let line_start = before.rfind('\n').map(|position| position + 1).unwrap_or(0);
        let line = before[line_start..].trim_start();

        if let Some(rest) = line.strip_prefix("use") {
            if rest.chars().next().is_some_and(|c| !c.is_whitespace()) {
                return false;
            }
            let rest = rest.trim_start();
            if rest.contains(';') || rest.contains('(') || rest.contains("qw") {
                return false;
            }
            let first_char = rest.chars().next();
            return first_char.is_none() || first_char.is_some_and(|c| c.is_ascii_uppercase());
        }

        if let Some(rest) = line.strip_prefix("require") {
            if rest.chars().next().is_some_and(|c| !c.is_whitespace()) {
                return false;
            }
            let rest = rest.trim_start();
            if rest.contains(';') {
                return false;
            }
            let Some(first_char) = rest.chars().next() else {
                return true;
            };
            return !matches!(first_char, '0'..='9' | '\'' | '"' | '`' | '.' | '/' | '\\');
        }

        false
    }

    fn add_runtime_workspace_completions(
        &self,
        completions: &mut Vec<crate::completion::CompletionItem>,
        doc_text: &str,
        doc_uri: &str,
        offset: usize,
        workspace_mode: &IndexAccessMode,
        cap: usize,
    ) {
        if Self::is_module_import_completion_context(doc_text, offset) {
            return;
        }

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

                // Detect `use Module` / `require Module` context so we can gate
                // Package-kind completions through @INC reachability (fixes #8537).
                let is_use_module_context = {
                    let before_prefix =
                        &text_before[..text_before.len().saturating_sub(prefix.len())];
                    let trimmed = before_prefix.trim_end();
                    trimmed.ends_with("use") || trimmed.ends_with("require")
                };

                // Build @INC context once (only when needed for filtering).
                let inc_ctx = if is_use_module_context {
                    self.effective_inc_context_for_doc(Some(doc_uri), Some(doc_text), Some(offset))
                } else {
                    None
                };

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

                    // For module-kind symbols in a `use Module` / `require Module`
                    // context, filter by position-aware @INC reachability so that
                    // `no lib` cancellations are honoured (fixes #8537).
                    let is_module_kind = matches!(
                        symbol.kind,
                        crate::workspace_index::SymbolKind::Package
                            | crate::workspace_index::SymbolKind::Class
                            | crate::workspace_index::SymbolKind::Role
                    );
                    if is_use_module_context && is_module_kind {
                        if let Some(ref ctx) = inc_ctx {
                            if !ctx.symbol_uri_reachable(&symbol.uri) {
                                tracing::trace!(
                                    symbol = %symbol.name,
                                    uri = %symbol.uri,
                                    "completion: skipping workspace symbol not reachable via @INC"
                                );
                                continue;
                            }
                        }
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
                        label_details: None,
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

    fn completion_list_default_data() -> Value {
        json!({
            "provider": "perl-lsp",
            "kind": "completion-list",
            "schemaVersion": 1
        })
    }

    fn completion_list_response(
        is_incomplete: bool,
        items: Vec<Value>,
        item_defaults_data_support: bool,
    ) -> Value {
        let has_items = !items.is_empty();
        let mut response = json!({
            "isIncomplete": is_incomplete,
            "items": items
        });

        if item_defaults_data_support && has_items {
            response["itemDefaults"] = json!({
                "data": Self::completion_list_default_data()
            });
        }

        response
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
                let ast_available = doc.ast.is_some();

                // Get completions, with fallback for missing AST
                #[cfg_attr(not(feature = "workspace"), allow(unused_mut))]
                let mut completions = if let Some(ast) = &doc.ast {
                    let (include_paths, system_inc_paths, include_system_inc) =
                        self.module_completion_roots_for_doc(uri, &doc.text, offset);
                    // Only provide workspace index when Full access is available
                    // This ensures we don't bypass routing policy
                    #[cfg(feature = "workspace")]
                    let workspace_idx = match &workspace_mode {
                        IndexAccessMode::Full(coordinator) => Some(Arc::clone(coordinator.index())),
                        _ => None,
                    };

                    #[cfg(feature = "workspace")]
                    let provider = CompletionProvider::new_with_index_and_source_and_paths(
                        ast,
                        &doc.text,
                        workspace_idx,
                        include_paths,
                        system_inc_paths,
                        include_system_inc,
                    )
                    .with_scan_cache(Arc::clone(&self.module_scan_cache));

                    #[cfg(not(feature = "workspace"))]
                    let provider = CompletionProvider::new_with_index_and_source_and_paths(
                        ast,
                        &doc.text,
                        None,
                        include_paths,
                        system_inc_paths,
                        include_system_inc,
                    )
                    .with_scan_cache(Arc::clone(&self.module_scan_cache));

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
                        uri,
                        offset,
                        &workspace_mode,
                        cap,
                    );
                }

                // Apply cap before converting to JSON
                let is_incomplete = completions.len() > cap;
                completions.truncate(cap);
                let (workspace_index_state, workspace_index_reason) =
                    Self::completion_workspace_index_state(&workspace_mode);
                let completion_decision_context = CompletionDecisionContext {
                    uri,
                    line,
                    character,
                    ast_available,
                    workspace_index_state,
                    workspace_index_reason,
                    is_incomplete,
                };
                self.record_completion_provider_decision_trace(
                    &completion_decision_context,
                    &completions,
                );

                // Snapshot capability flags once before the loop to avoid
                // acquiring client_capabilities lock per completion item
                let client_caps = self.client_capabilities.lock();
                let snippet_support = client_caps.snippet_support;
                let commit_chars_support = client_caps.completion_commit_characters_support;
                let label_details_support = client_caps.label_details_support;
                let item_defaults_data_support =
                    client_caps.completion_list_item_defaults_data_support;

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

                        if let Some(sort_text) = c.sort_text {
                            item["sortText"] = json!(sort_text);
                        }

                        if label_details_support {
                            if let Some(ld) = c.label_details {
                                let mut obj = serde_json::Map::new();
                                if let Some(d) = ld.detail {
                                    obj.insert("detail".to_string(), json!(d));
                                }
                                if let Some(desc) = ld.description {
                                    obj.insert("description".to_string(), json!(desc));
                                }
                                if !obj.is_empty() {
                                    item["labelDetails"] = Value::Object(obj);
                                }
                            }
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
                return Ok(Some(Self::completion_list_response(
                    is_incomplete,
                    items,
                    item_defaults_data_support,
                )));
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
        // Convert raw Value ID to typed ID at the boundary.
        let typed_id = request_id.and_then(JsonRpcId::try_from_value);
        // RAII guard ensures cleanup on all exit paths (early returns, errors, panics)
        let _cleanup_guard = RequestCleanupGuard::from_ref(typed_id.as_ref());

        if let Some(params) = params {
            // Create or get cancellation token for this request
            let token = if let Some(ref tid) = typed_id {
                GLOBAL_CANCELLATION_REGISTRY.get_token(tid).unwrap_or_else(|| {
                    let token = PerlLspCancellationToken::new(
                        tid.clone(),
                        "textDocument/completion".to_string(),
                    );
                    let _ = GLOBAL_CANCELLATION_REGISTRY.register_token(token.clone());
                    token
                })
            } else {
                // Notification path: no client-visible id to cancel against. Use a
                // synthetic sentinel id that won't collide with any real client or
                // server ID (which are always positive integers). The token is
                // created but never registered in the global registry, so external
                // cancellation cannot reach it; it exists only for the
                // cancel-check closure that the provider calls during its work.
                PerlLspCancellationToken::new(
                    UNCANCELLABLE_LOCAL_TOKEN_ID,
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
                let ast_available = doc.ast.is_some();

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
                    let (include_paths, system_inc_paths, include_system_inc) =
                        self.module_completion_roots_for_doc(uri, &doc.text, offset);
                    // Only provide workspace index when Full access is available
                    // This ensures we don't bypass routing policy
                    #[cfg(feature = "workspace")]
                    let workspace_idx = match &workspace_mode {
                        IndexAccessMode::Full(coordinator) => Some(Arc::clone(coordinator.index())),
                        _ => None,
                    };

                    #[cfg(feature = "workspace")]
                    let provider = CompletionProvider::new_with_index_and_source_and_paths(
                        ast,
                        &doc.text,
                        workspace_idx,
                        include_paths,
                        system_inc_paths,
                        include_system_inc,
                    )
                    .with_scan_cache(Arc::clone(&self.module_scan_cache));
                    #[cfg(not(feature = "workspace"))]
                    let provider = CompletionProvider::new_with_index_and_source_and_paths(
                        ast,
                        &doc.text,
                        None,
                        include_paths,
                        system_inc_paths,
                        include_system_inc,
                    )
                    .with_scan_cache(Arc::clone(&self.module_scan_cache));

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
                    uri,
                    offset,
                    &workspace_mode,
                    completion_cap(),
                );

                let (workspace_index_state, workspace_index_reason) =
                    Self::completion_workspace_index_state(&workspace_mode);
                let completion_decision_context = CompletionDecisionContext {
                    uri,
                    line,
                    character,
                    ast_available,
                    workspace_index_state,
                    workspace_index_reason,
                    is_incomplete: false,
                };
                self.record_completion_provider_decision_trace(
                    &completion_decision_context,
                    &completions,
                );

                // Convert to JSON format with highly optimized cancellation checks
                let client_caps = self.client_capabilities.lock();
                let commit_chars_support = client_caps.completion_commit_characters_support;
                let snippet_support = client_caps.snippet_support;
                let label_details_support = client_caps.label_details_support;
                let item_defaults_data_support =
                    client_caps.completion_list_item_defaults_data_support;

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

                        if let Some(sort_text) = c.sort_text {
                            item["sortText"] = json!(sort_text);
                        }

                        if label_details_support {
                            if let Some(ld) = c.label_details {
                                let mut obj = serde_json::Map::new();
                                if let Some(d) = ld.detail {
                                    obj.insert("detail".to_string(), json!(d));
                                }
                                if let Some(desc) = ld.description {
                                    obj.insert("description".to_string(), json!(desc));
                                }
                                if !obj.is_empty() {
                                    item["labelDetails"] = Value::Object(obj);
                                }
                            }
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

                return Ok(Some(Self::completion_list_response(
                    false,
                    items,
                    item_defaults_data_support,
                )));
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
                        label_details: None,
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
                        label_details: None,
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
                        label_details: None,
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
                        label_details: None,
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
                        label_details: None,
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
                            label_details: None,
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
        let label_details_support = self.client_capabilities.lock().label_details_support;

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

            if let Some(obj) = item.as_object_mut() {
                obj.insert(
                    "documentation".to_string(),
                    json!({
                        "kind": "markdown",
                        "value": documentation
                    }),
                );
                // Populate labelDetails for clients that declared labelDetailsSupport.
                // detail = primary signature (inline after label), description = source tag.
                if label_details_support && obj.get("labelDetails").is_none() {
                    let sig_detail = sig.signatures.first().copied().unwrap_or("").to_string();
                    if !sig_detail.is_empty() {
                        obj.insert(
                            "labelDetails".to_string(),
                            json!({
                                "detail": sig_detail,
                                "description": "builtin"
                            }),
                        );
                    }
                }
            }
            return Ok(Some(item));
        }

        // For variables, add type hint documentation if available
        if kind == 6 && !has_doc {
            // Variable kind
            let (type_doc, label_detail) = if label.starts_with('$') {
                (
                    Some("Scalar variable - holds a single value (string, number, or reference)"),
                    Some("scalar"),
                )
            } else if label.starts_with('@') {
                (Some("Array variable - holds an ordered list of scalars"), Some("array"))
            } else if label.starts_with('%') {
                (Some("Hash variable - holds a set of key-value pairs"), Some("hash"))
            } else {
                (None, None)
            };

            if let Some(obj) = item.as_object_mut() {
                if let Some(doc) = type_doc {
                    obj.insert(
                        "documentation".to_string(),
                        json!({
                            "kind": "markdown",
                            "value": doc
                        }),
                    );
                }
                if label_details_support {
                    if let Some(detail) = label_detail {
                        if obj.get("labelDetails").is_none() {
                            obj.insert("labelDetails".to_string(), json!({ "detail": detail }));
                        }
                    }
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

    fn explain_provider_decision(
        server: &LspServer,
        provider: &str,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let response = server
            .handle_execute_command(Some(json!({
                "command": "perl.explainProviderDecision",
                "arguments": [{
                    "provider": provider
                }]
            })))?
            .ok_or("missing explain-provider-decision response")?;
        Ok(response)
    }

    #[test]
    fn completion_provider_decision_replays_live_completion_trace()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        let uri = "file:///workspace/completion_trace.pl";
        let text = "my $count = 0;\nmy $counter = $co\n";

        server.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })))?;

        let response = server
            .handle_completion_cancellable(
                Some(json!({
                    "textDocument": { "uri": uri, "version": 1 },
                    "position": { "line": 1, "character": 17 }
                })),
                Some(&json!("completion-provider-decision")),
            )?
            .ok_or("expected completion response")?;
        let items =
            response.get("items").and_then(Value::as_array).ok_or("expected completion items")?;
        assert!(
            items.iter().any(|item| item.get("label").and_then(Value::as_str) == Some("$count")),
            "expected lexical completion to include $count, got: {items:?}"
        );

        let explanation = explain_provider_decision(&server, "completion")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(Value::as_object)
            .ok_or("missing persisted completion request receipt")?;
        let sample_labels = receipt
            .get("sample_labels")
            .and_then(Value::as_array)
            .ok_or("missing completion sample labels")?;

        assert_eq!(
            receipt.get("schema_version").and_then(Value::as_str),
            Some("provider_decision.v1")
        );
        assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("completion"));
        assert_eq!(
            receipt.get("provider_action").and_then(Value::as_str),
            Some("textDocument/completion")
        );
        assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("acted"));
        assert_eq!(receipt.get("uri").and_then(Value::as_str), Some(uri));
        assert_eq!(receipt.get("line").and_then(Value::as_u64), Some(1));
        assert_eq!(receipt.get("character").and_then(Value::as_u64), Some(17));
        assert_eq!(receipt.get("freshness").and_then(Value::as_str), Some("fresh"));
        assert_eq!(receipt.get("fallback").and_then(Value::as_str), Some("none"));
        assert_eq!(
            receipt.get("claim_boundary").and_then(Value::as_str),
            Some(
                "records existing completion response only; no new completion candidates or ranking changes"
            )
        );
        assert!(
            receipt.get("item_count").and_then(Value::as_u64).is_some_and(|count| count > 0),
            "completion receipt should record item count: {receipt:?}"
        );
        assert!(
            sample_labels.iter().filter_map(Value::as_str).any(|label| label == "$count"),
            "completion receipt should include sample labels from the response: {sample_labels:?}"
        );
        Ok(())
    }

    #[test]
    fn completion_provider_decision_records_regular_completion_trace()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        let uri = "file:///workspace/completion_regular_trace.pl";
        let text = "my $ready = 1;\n$re\n";

        server.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })))?;

        let response = server
            .handle_completion(Some(json!({
                "textDocument": { "uri": uri, "version": 1 },
                "position": { "line": 1, "character": 3 }
            })))?
            .ok_or("expected completion response")?;
        let items =
            response.get("items").and_then(Value::as_array).ok_or("expected completion items")?;
        assert!(
            items.iter().any(|item| item.get("label").and_then(Value::as_str) == Some("$ready")),
            "expected regular completion to include $ready, got: {items:?}"
        );

        let explanation = explain_provider_decision(&server, "completion")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(Value::as_object)
            .ok_or("missing persisted completion request receipt")?;

        assert_eq!(
            receipt.get("schema_version").and_then(Value::as_str),
            Some("provider_decision.v1")
        );
        assert_eq!(receipt.get("provider").and_then(Value::as_str), Some("completion"));
        assert_eq!(receipt.get("decision").and_then(Value::as_str), Some("acted"));
        assert_eq!(
            receipt.get("provider_action").and_then(Value::as_str),
            Some("textDocument/completion")
        );
        assert_eq!(receipt.get("is_incomplete").and_then(Value::as_bool), Some(false));
        Ok(())
    }

    #[test]
    fn test_module_completion_roots_includes_use_lib_paths()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        use tempfile::TempDir;
        use url::Url;

        let temp = TempDir::new()?;
        let lib_dir = temp.path().join("mylibs");
        let module_file = lib_dir.join("MyCustom").join("Widget.pm");
        fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        fs::write(&module_file, "package MyCustom::Widget;\n1;\n")?;

        let workspace_uri =
            Url::from_file_path(temp.path()).map_err(|_| "bad workspace path")?.to_string();
        let doc_uri =
            Url::from_file_path(temp.path().join("app.pl")).map_err(|_| "bad doc uri")?.to_string();

        let lib_dir_str = lib_dir.to_string_lossy();
        let doc_text = format!("use lib '{lib_dir_str}';\nuse MyCustom::Widget;\n");

        let server = LspServer::default();

        // Register workspace folder so folder_for_doc_uri / config_for_doc work.
        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(workspace_uri.clone()),
        );

        let (include_paths, _sys, _use_sys) =
            server.module_completion_roots_for_doc(&doc_uri, &doc_text, doc_text.len());

        assert!(
            include_paths.contains(&lib_dir),
            "use lib path should be in include_paths; got: {include_paths:?}",
        );
        Ok(())
    }

    #[test]
    fn test_module_completion_roots_system_inc_fallback_for_non_workspace_uri()
    -> Result<(), Box<dyn std::error::Error>> {
        use perl_lsp_rs_core::config::WorkspaceConfig;

        let server = LspServer::default();
        let mut cfg = WorkspaceConfig::default();
        cfg.use_system_inc = true;

        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(
                "file:///workspace/project".to_string(),
            )
            .with_effective_workspace_config(cfg),
        );

        let (include_paths, system_inc_paths, include_system_inc) = server
            .module_completion_roots_for_doc("file:///tmp/outside_workspace.pl", "use strict;", 0);

        assert!(include_system_inc, "use_system_inc should be propagated");
        assert!(include_paths.is_empty(), "no configured include paths expected");
        assert!(
            !system_inc_paths.is_empty(),
            "fallback system @INC roots should be available for non-workspace URIs"
        );

        Ok(())
    }

    #[test]
    fn test_module_completion_roots_keep_file_local_use_lib_for_non_workspace_uri()
    -> Result<(), Box<dyn std::error::Error>> {
        use tempfile::TempDir;
        use url::Url;

        let temp = TempDir::new()?;
        let workspace = temp.path().join("workspace");
        let lexical_lib = workspace.join("standalone_lib");
        let outside_doc = temp.path().join("outside.pl");
        std::fs::create_dir_all(&lexical_lib)?;
        std::fs::write(&outside_doc, "use strict;\n")?;

        let workspace_uri =
            Url::from_file_path(&workspace).map_err(|_| "bad workspace uri")?.to_string();
        let doc_uri = Url::from_file_path(&outside_doc).map_err(|_| "bad doc uri")?.to_string();
        let mut config = perl_lsp_rs_core::config::WorkspaceConfig::default();
        config.include_paths = vec!["configured_lib".to_string()];
        config.use_system_inc = false;

        let server = LspServer::default();
        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(workspace_uri)
                .with_path(workspace.clone())
                .with_effective_workspace_config(config),
        );

        let doc_text = "use lib 'standalone_lib';\nuse Outside::Widget;\n";
        let (include_paths, system_inc_paths, include_system_inc) =
            server.module_completion_roots_for_doc(&doc_uri, doc_text, doc_text.len());

        assert!(
            include_paths.contains(&lexical_lib),
            "file-local use lib root should survive outside-workspace filtering; got {include_paths:?}"
        );
        assert!(
            !include_paths.iter().any(|path| path.ends_with("configured_lib")),
            "workspace-configured roots should stay excluded for non-workspace docs; got {include_paths:?}"
        );
        assert!(system_inc_paths.is_empty());
        assert!(!include_system_inc);
        Ok(())
    }

    #[test]
    fn test_module_completion_roots_match_effective_inc_context_for_workspace_doc()
    -> Result<(), Box<dyn std::error::Error>> {
        use perl_module::resolution::IncRootKind;
        use tempfile::TempDir;
        use url::Url;

        let temp = TempDir::new()?;
        let workspace = temp.path().join("workspace");
        let doc_path = workspace.join("bin").join("app.pl");
        std::fs::create_dir_all(doc_path.parent().ok_or("missing doc parent")?)?;

        let workspace_uri =
            Url::from_file_path(&workspace).map_err(|_| "bad workspace uri")?.to_string();
        let doc_uri = Url::from_file_path(&doc_path).map_err(|_| "bad doc uri")?.to_string();
        let mut config = perl_lsp_rs_core::config::WorkspaceConfig::default();
        config.include_paths = vec!["lib".to_string()];
        config.use_system_inc = false;

        let server = LspServer::default();
        server.workspace_folders.lock().push(
            crate::runtime::workspace_folder::WorkspaceFolderState::new(workspace_uri)
                .with_path(workspace.clone())
                .with_effective_workspace_config(config),
        );

        let doc_text = "use lib 't/lib';\nuse Demo::Worker;\n";
        let context = server
            .effective_inc_context_for_doc(Some(&doc_uri), Some(doc_text), Some(doc_text.len()))
            .ok_or("expected effective @INC context")?;
        let expected_include_paths: Vec<PathBuf> = context
            .effective_roots
            .iter()
            .filter(|root| root.kind != IncRootKind::InterpreterStartup)
            .map(|root| LspServer::completion_path_for_inc_root(root, &context.root))
            .collect();

        let (include_paths, system_inc_paths, include_system_inc) =
            server.module_completion_roots_for_doc(&doc_uri, doc_text, doc_text.len());

        assert_eq!(include_paths, expected_include_paths);
        assert!(system_inc_paths.is_empty());
        assert!(!include_system_inc);
        Ok(())
    }

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

    #[test]
    fn test_completion_capability_advertises_label_details_support() {
        use perl_lsp_rs_core::features::flags::BuildFlags;
        use perl_lsp_rs_core::protocol::capabilities::capabilities_for;
        use serde_json::to_value;

        let caps = capabilities_for(BuildFlags::production());
        let caps_json = to_value(&caps).expect("serialize ServerCapabilities");

        let completion_item_opt = caps_json
            .pointer("/completionProvider/completionItem")
            .expect("completionProvider.completionItem must be present");
        assert_eq!(
            completion_item_opt.get("labelDetailsSupport").and_then(|v| v.as_bool()),
            Some(true),
            "server must advertise completionItem.labelDetailsSupport: true in capabilities"
        );
    }

    #[test]
    fn test_completion_builtin_function_has_label_details_on_resolve()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        // Simulate a client that declared labelDetailsSupport.
        server.client_capabilities.lock().label_details_support = true;

        let item = json!({
            "label": "print",
            "kind": 3  // Function
        });
        let resolved = server
            .handle_completion_resolve(Some(item))
            .map_err(|e| e.message.to_string())?
            .ok_or("expected resolved value")?;

        let ld =
            resolved.get("labelDetails").ok_or("labelDetails must be present after resolve")?;
        let detail = ld.get("detail").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            detail.contains("print"),
            "labelDetails.detail should contain the primary signature; got: {detail:?}"
        );
        assert_eq!(
            ld.get("description").and_then(|v| v.as_str()),
            Some("builtin"),
            "labelDetails.description should be 'builtin'"
        );
        Ok(())
    }

    #[test]
    fn test_completion_builtin_no_label_details_without_client_support()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        // Default server has label_details_support = false.

        let item = json!({ "label": "print", "kind": 3 });
        let resolved = server
            .handle_completion_resolve(Some(item))
            .map_err(|e| e.message.to_string())?
            .ok_or("expected resolved value")?;

        assert!(
            resolved.get("labelDetails").is_none(),
            "labelDetails must NOT be populated when client did not declare labelDetailsSupport"
        );
        Ok(())
    }

    #[test]
    fn test_completion_variable_has_label_details_on_resolve()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        server.client_capabilities.lock().label_details_support = true;

        let item = json!({ "label": "$count", "kind": 6 });
        let resolved = server
            .handle_completion_resolve(Some(item))
            .map_err(|e| e.message.to_string())?
            .ok_or("expected resolved value")?;

        let ld = resolved.get("labelDetails").ok_or("labelDetails must be present for variable")?;
        assert_eq!(
            ld.get("detail").and_then(|v| v.as_str()),
            Some("scalar"),
            "scalar variable should have labelDetails.detail = 'scalar'"
        );
        Ok(())
    }

    /// PERL5LIB inclusion in the shared completion context is gated on
    /// `use_perl5lib`, NOT `use_system_inc`. `use_system_inc` controls
    /// interpreter startup `@INC` only. This test walks the four-cell matrix to
    /// ensure the two flags remain independent for the effective include-path
    /// source that completion now consumes through `EffectiveIncContext`.
    #[test]
    fn perl5lib_completion_gate_is_use_perl5lib_independent_of_use_system_inc()
    -> Result<(), Box<dyn std::error::Error>> {
        use perl_lsp_rs_core::config::WorkspaceConfig;

        // (use_perl5lib, use_system_inc, expected_perl5lib_present)
        let cells: &[(bool, bool, bool)] =
            &[(true, false, true), (true, true, true), (false, true, false), (false, false, false)];

        for &(use_perl5lib, use_system_inc, expected) in cells {
            let mut config = WorkspaceConfig::default();
            config.use_perl5lib = use_perl5lib;
            config.use_system_inc = use_system_inc;

            let paths = config.effective_include_paths(&["perl5lib".to_string()]);
            let has = paths.iter().any(|path| path == "perl5lib");
            assert_eq!(
                has, expected,
                "cell (use_perl5lib={use_perl5lib}, use_system_inc={use_system_inc}): \
                 expected PERL5LIB present={expected}, got {has}; paths={paths:?}"
            );
        }

        Ok(())
    }

    #[test]
    fn module_import_completion_context_accepts_perl_whitespace_only_after_keyword() {
        assert!(LspServer::is_module_import_completion_context("use\tFoo", "use\tFoo".len()));
        assert!(LspServer::is_module_import_completion_context(
            "require\tFoo",
            "require\tFoo".len()
        ));
        assert!(!LspServer::is_module_import_completion_context("useful Foo", "useful Foo".len()));
        assert!(!LspServer::is_module_import_completion_context(
            "required Foo",
            "required Foo".len()
        ));
    }

    // =========================================================================
    // Module scan cache integration tests (issue #8514)
    // =========================================================================

    /// First call for a given root+prefix triggers a scan (cache miss).
    /// Second call within TTL returns the cached result (no new scan).
    #[test]
    fn test_scan_cache_second_call_within_ttl_does_not_rescan()
    -> Result<(), Box<dyn std::error::Error>> {
        use perl_lsp_rs_core::providers::completion::module_scan_cache::{
            ModuleCompletionScanCache, ScanCacheKey,
        };
        use std::path::PathBuf;

        let temp = tempfile::TempDir::new()?;
        let lib = temp.path().join("lib");
        let mojo_dir = lib.join("Mojo");
        std::fs::create_dir_all(&mojo_dir)?;
        std::fs::write(mojo_dir.join("Controller.pm"), "package Mojo::Controller;\n1;\n")?;

        let cache = ModuleCompletionScanCache::new();
        let canonical = std::fs::canonicalize(&lib).unwrap_or_else(|_| lib.clone());
        let key = ScanCacheKey {
            canonical_root: canonical.clone(),
            prefix_dir: PathBuf::from("Mojo"),
            module_prefix: "Mojo::".to_string(),
        };

        // First: miss — nothing in cache yet
        assert!(cache.get(&key).is_none(), "cache must be cold initially");

        // Manually populate as the scan would
        cache.insert(key.clone(), vec!["Mojo::Controller".to_string()]);

        // Second within TTL: hit
        let hit = cache.get(&key).ok_or("expected cache hit on second call")?;
        assert!(hit.contains(&"Mojo::Controller".to_string()), "cached result must match scan");

        Ok(())
    }

    /// Different prefix dir produces a cache miss.
    #[test]
    fn test_scan_cache_different_prefix_dir_misses() -> Result<(), Box<dyn std::error::Error>> {
        use perl_lsp_rs_core::providers::completion::module_scan_cache::{
            ModuleCompletionScanCache, ScanCacheKey,
        };
        use std::path::PathBuf;

        let cache = ModuleCompletionScanCache::new();
        cache.insert(
            ScanCacheKey {
                canonical_root: PathBuf::from("/lib"),
                prefix_dir: PathBuf::from("Mojo"),
                module_prefix: "Mojo::C".to_string(),
            },
            vec!["Mojo::Controller".to_string()],
        );

        // Different prefix_dir — must miss
        let miss = cache.get(&ScanCacheKey {
            canonical_root: PathBuf::from("/lib"),
            prefix_dir: PathBuf::from("Catalyst"),
            module_prefix: "Catalyst::C".to_string(),
        });
        assert!(miss.is_none(), "different prefix_dir must be a cache miss");

        Ok(())
    }

    /// After TTL expires the cache returns None.
    #[test]
    fn test_scan_cache_after_ttl_misses() -> Result<(), Box<dyn std::error::Error>> {
        use perl_lsp_rs_core::providers::completion::module_scan_cache::{
            ModuleCompletionScanCache, ScanCacheKey,
        };
        use std::path::PathBuf;
        use std::time::Duration;

        let cache = ModuleCompletionScanCache::with_ttl_ms(10);
        let key = ScanCacheKey {
            canonical_root: PathBuf::from("/lib"),
            prefix_dir: PathBuf::from("Mojo"),
            module_prefix: "Mojo::C".to_string(),
        };
        cache.insert(key.clone(), vec!["Mojo::Controller".to_string()]);

        std::thread::sleep(Duration::from_millis(50));
        assert!(cache.get(&key).is_none(), "expired entry must be a cache miss");

        Ok(())
    }

    /// Cancellation token is checked before returning a cached hit.
    #[test]
    fn test_scan_cache_cancellation_checked_before_returning_hit()
    -> Result<(), Box<dyn std::error::Error>> {
        use perl_lsp_rs_core::providers::completion::CompletionProvider;
        use perl_lsp_rs_core::providers::completion::module_scan_cache::{
            ModuleCompletionScanCache, ScanCacheKey,
        };
        use perl_parser::Parser;
        use std::path::PathBuf;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        let temp = tempfile::TempDir::new()?;
        let lib = temp.path().join("lib");
        let mojo_dir = lib.join("Mojo");
        std::fs::create_dir_all(&mojo_dir)?;
        std::fs::write(mojo_dir.join("Controller.pm"), "package Mojo::Controller;\n1;\n")?;

        // Pre-populate the cache as if a prior call had scanned.
        let cache = Arc::new(ModuleCompletionScanCache::new());
        let canonical = std::fs::canonicalize(&lib).unwrap_or_else(|_| lib.clone());
        let key = ScanCacheKey {
            canonical_root: canonical.clone(),
            prefix_dir: PathBuf::from("Mojo"),
            module_prefix: "Mojo::".to_string(),
        };
        cache.insert(key, vec!["Mojo::Controller".to_string()]);

        // Construct a provider with the pre-populated cache and a cancellation flag already set.
        let source = "use Mojo::";
        let mut parser = Parser::new(source);
        let ast = parser.parse().map_err(|e| format!("parse error: {e:?}"))?;

        let cancelled = Arc::new(AtomicBool::new(true));
        let cancel_fn = {
            let c = Arc::clone(&cancelled);
            move || c.load(Ordering::Relaxed)
        };

        let provider = CompletionProvider::new_with_index_and_source_and_paths(
            &ast,
            source,
            None,
            vec![lib.clone()],
            vec![],
            false,
        )
        .with_scan_cache(Arc::clone(&cache));

        // With cancellation flag set, completions must be empty even though cache has a hit.
        let completions =
            provider.get_completions_with_path_cancellable(source, source.len(), None, &cancel_fn);

        // Either the request was cancelled entirely (empty) or the cancellation check
        // at the cache-hit path caused an early return.  Either way, the contract is
        // that a cancelled request does not return results.
        assert!(
            completions.is_empty(),
            "cancelled request must not return completions; got {completions:?}"
        );

        Ok(())
    }

    /// Prefix-filtered cache entries must not satisfy a different leaf prefix
    /// under the same namespace directory.
    #[test]
    fn test_scan_cache_full_prefix_prevents_cross_prefix_hits()
    -> Result<(), Box<dyn std::error::Error>> {
        use perl_lsp_rs_core::providers::completion::CompletionProvider;
        use perl_lsp_rs_core::providers::completion::module_scan_cache::{
            ModuleCompletionScanCache, ScanCacheKey,
        };
        use perl_parser::Parser;
        use std::path::PathBuf;
        use std::sync::Arc;

        let temp = tempfile::TempDir::new()?;
        let lib = temp.path().join("lib");
        let mojo_dir = lib.join("Mojo");
        std::fs::create_dir_all(&mojo_dir)?;
        std::fs::write(mojo_dir.join("Controller.pm"), "package Mojo::Controller;\n1;\n")?;
        std::fs::write(mojo_dir.join("Lite.pm"), "package Mojo::Lite;\n1;\n")?;

        // Simulate an earlier `Mojo::C` request. A later `Mojo::L` request
        // scans the same prefix_dir (`Mojo`) but must not reuse this filtered hit.
        let cache = Arc::new(ModuleCompletionScanCache::new());
        let canonical = std::fs::canonicalize(&lib).unwrap_or_else(|_| lib.clone());
        cache.insert(
            ScanCacheKey {
                canonical_root: canonical,
                prefix_dir: PathBuf::from("Mojo"),
                module_prefix: "Mojo::C".to_string(),
            },
            vec!["Mojo::Controller".to_string()],
        );

        let source = "use Mojo::L";
        let mut parser = Parser::new(source);
        let ast = parser.parse().map_err(|e| format!("parse error: {e:?}"))?;
        let provider = CompletionProvider::new_with_index_and_source_and_paths(
            &ast,
            source,
            None,
            vec![lib.clone()],
            vec![],
            false,
        )
        .with_scan_cache(Arc::clone(&cache));

        let labels: Vec<String> = provider
            .get_completions_with_path(source, source.len(), None)
            .into_iter()
            .map(|item| item.label)
            .collect();

        assert!(
            labels.contains(&"Mojo::Lite".to_string()),
            "different leaf prefix should miss the stale filtered hit and scan Lite; labels={labels:?}"
        );
        assert!(
            !labels.contains(&"Mojo::Controller".to_string()),
            "cached Mojo::C result must not leak into Mojo::L completions; labels={labels:?}"
        );

        Ok(())
    }

    /// Cached and uncached completions produce the same labels.
    #[test]
    fn test_scan_cache_cached_and_uncached_labels_match() -> Result<(), Box<dyn std::error::Error>>
    {
        use perl_lsp_rs_core::providers::completion::CompletionProvider;
        use perl_lsp_rs_core::providers::completion::module_scan_cache::ModuleCompletionScanCache;
        use perl_parser::Parser;
        use std::sync::Arc;

        let temp = tempfile::TempDir::new()?;
        let lib = temp.path().join("lib");
        let mojo_dir = lib.join("Mojo");
        std::fs::create_dir_all(&mojo_dir)?;
        std::fs::write(mojo_dir.join("Controller.pm"), "package Mojo::Controller;\n1;\n")?;
        std::fs::write(mojo_dir.join("Lite.pm"), "package Mojo::Lite;\n1;\n")?;

        let source = "use Mojo::";
        let mut parser = Parser::new(source);
        let ast = parser.parse().map_err(|e| format!("parse error: {e:?}"))?;

        // Uncached call.
        let uncached_provider = CompletionProvider::new_with_index_and_source_and_paths(
            &ast,
            source,
            None,
            vec![lib.clone()],
            vec![],
            false,
        );
        let uncached = uncached_provider.get_completions_with_path(source, source.len(), None);
        let mut uncached_labels: Vec<String> = uncached.iter().map(|c| c.label.clone()).collect();
        uncached_labels.sort();

        // Cached call — first invocation populates the cache.
        let cache = Arc::new(ModuleCompletionScanCache::new());
        let cached_provider = CompletionProvider::new_with_index_and_source_and_paths(
            &ast,
            source,
            None,
            vec![lib.clone()],
            vec![],
            false,
        )
        .with_scan_cache(Arc::clone(&cache));
        let cached_first = cached_provider.get_completions_with_path(source, source.len(), None);
        let mut cached_first_labels: Vec<String> =
            cached_first.iter().map(|c| c.label.clone()).collect();
        cached_first_labels.sort();

        // Second invocation should hit the cache.
        let cached_provider2 = CompletionProvider::new_with_index_and_source_and_paths(
            &ast,
            source,
            None,
            vec![lib.clone()],
            vec![],
            false,
        )
        .with_scan_cache(Arc::clone(&cache));
        let cached_second = cached_provider2.get_completions_with_path(source, source.len(), None);
        let mut cached_second_labels: Vec<String> =
            cached_second.iter().map(|c| c.label.clone()).collect();
        cached_second_labels.sort();

        assert_eq!(
            uncached_labels, cached_first_labels,
            "first cached call must produce same labels as uncached"
        );
        assert_eq!(
            cached_first_labels, cached_second_labels,
            "second cached call (cache hit) must produce same labels as first"
        );

        Ok(())
    }
}
