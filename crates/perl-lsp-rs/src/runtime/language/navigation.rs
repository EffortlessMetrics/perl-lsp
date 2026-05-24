//! Navigation handlers for go-to-definition, declaration, and related features
//!
//! Handles textDocument/declaration, textDocument/definition, textDocument/typeDefinition,
//! and textDocument/implementation requests.

use super::super::*;
use crate::cancellation::RequestCleanupGuard;
use crate::protocol::{req_position, req_uri};
use crate::util::{read_text_file_with_encoding, token_under_cursor};
use std::collections::{HashMap, HashSet, VecDeque};

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
use perl_lsp_rs_core::providers::navigation::definition_shadow::{
    DefinitionCutoverResult, goto_definition_live_exact_or_imported,
};
#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
use perl_workspace::semantic::queries::QueryContext;

#[cfg(feature = "workspace")]
use crate::runtime::routing::{IndexAccessMode, route_index_access};
#[cfg(feature = "workspace")]
use std::sync::OnceLock;

mod core_modules;
#[cfg(feature = "workspace")]
mod mojolicious_routes;
mod xs_bootstrap;

use self::core_modules::is_core_perl_module;
#[cfg(feature = "workspace")]
use self::mojolicious_routes::resolve_mojolicious_route_definition;
use self::xs_bootstrap::{extract_xs_bootstrap_target, xs_bootstrap_location};

#[cfg(feature = "workspace")]
static FQN_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

#[cfg(feature = "workspace")]
static ARROW_METHOD_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

#[cfg(feature = "workspace")]
static PACKAGE_ARROW_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

#[cfg(feature = "workspace")]
static VAR_METHOD_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

#[cfg(feature = "workspace")]
static SUPER_METHOD_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

#[cfg(feature = "workspace")]
static GOTO_LABEL_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

#[cfg(feature = "workspace")]
static LABEL_DECLARATION_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

fn lsp_location_count(value: Option<&Value>) -> usize {
    match value {
        Some(Value::Array(items)) => items.len(),
        Some(Value::Object(obj)) if obj.contains_key("uri") || obj.contains_key("targetUri") => 1,
        _ => 0,
    }
}

#[derive(Debug)]
struct NavigationDecisionTraceContext {
    provider: &'static str,
    provider_action: &'static str,
    uri: String,
    line: u32,
    character: u32,
    include_declaration: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
struct TypeDefinitionFallbackTrace {
    reason: &'static str,
    blocker: &'static str,
    source_backed_state: &'static str,
    fact_source: &'static str,
    freshness: &'static str,
    dynamic_boundary: bool,
}

impl Default for TypeDefinitionFallbackTrace {
    fn default() -> Self {
        Self {
            reason: "missing_fact",
            blocker: "missing_fact",
            source_backed_state: "type_definition_not_proven",
            fact_source: "fallback",
            freshness: "fresh",
            dynamic_boundary: false,
        }
    }
}

fn stale_type_definition_fallback_trace() -> TypeDefinitionFallbackTrace {
    TypeDefinitionFallbackTrace {
        reason: "stale_fact",
        blocker: "stale_fact",
        source_backed_state: "stale_type_definition_request",
        fact_source: "request_version",
        freshness: "stale",
        dynamic_boundary: false,
    }
}

fn classify_type_definition_fallback_trace(
    source_text: &str,
    line: u32,
    character: u32,
) -> TypeDefinitionFallbackTrace {
    let Some(line_text) = usize::try_from(line).ok().and_then(|line| source_text.lines().nth(line))
    else {
        return TypeDefinitionFallbackTrace::default();
    };

    let character = usize::try_from(character).unwrap_or_default();
    let compact_before_cursor =
        line_text.chars().take(character).filter(|ch| !ch.is_whitespace()).collect::<String>();
    let compact_from_cursor = line_text
        .chars()
        .skip(character)
        .take(64)
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();

    if compact_from_cursor.starts_with("->$")
        || (compact_before_cursor.ends_with("->") && compact_from_cursor.starts_with('$'))
        || (compact_before_cursor.ends_with("isa=>") && compact_from_cursor.starts_with('$'))
    {
        return TypeDefinitionFallbackTrace {
            reason: "dynamic_boundary",
            blocker: "dynamic_boundary",
            source_backed_state: "dynamic_type_definition_boundary",
            fact_source: "dynamic_boundary",
            freshness: "fresh",
            dynamic_boundary: true,
        };
    }

    TypeDefinitionFallbackTrace::default()
}

#[cfg(feature = "workspace")]
fn get_fqn_regex() -> Result<&'static regex::Regex, JsonRpcError> {
    FQN_RE
        .get_or_init(|| regex::Regex::new(r"([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)"))
        .as_ref()
        .map_err(|err| {
            crate::protocol::internal_error(&format!(
                "Failed to initialize fully-qualified symbol regex: {err}"
            ))
        })
}

#[cfg(feature = "workspace")]
fn get_arrow_method_regex() -> Result<&'static regex::Regex, JsonRpcError> {
    ARROW_METHOD_RE
        .get_or_init(|| {
            regex::Regex::new(
                r"([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)\s*->\s*([A-Za-z_][A-Za-z0-9_]*)",
            )
        })
        .as_ref()
        .map_err(|err| {
            crate::protocol::internal_error(&format!(
                "Failed to initialize method-call regex: {err}"
            ))
        })
}

#[cfg(feature = "workspace")]
fn get_package_arrow_regex() -> Result<&'static regex::Regex, JsonRpcError> {
    PACKAGE_ARROW_RE
        .get_or_init(|| {
            regex::Regex::new(r"([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)\s*->")
        })
        .as_ref()
        .map_err(|err| {
            crate::protocol::internal_error(&format!(
                "Failed to initialize package navigation regex: {err}"
            ))
        })
}

/// Get regex for matching `$var->method` patterns (variable-based method calls).
///
/// Captures: group 1 = variable name (without sigil), group 2 = method name.
#[cfg(feature = "workspace")]
fn get_var_method_regex() -> Result<&'static regex::Regex, JsonRpcError> {
    VAR_METHOD_RE
        .get_or_init(|| {
            regex::Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)\s*->\s*([A-Za-z_][A-Za-z0-9_]*)")
        })
        .as_ref()
        .map_err(|err| {
            crate::protocol::internal_error(&format!(
                "Failed to initialize variable method-call regex: {err}"
            ))
        })
}

#[cfg(feature = "workspace")]
fn get_super_method_regex() -> Result<&'static regex::Regex, JsonRpcError> {
    SUPER_METHOD_RE
        .get_or_init(|| regex::Regex::new(r"\bSUPER::([A-Za-z_][A-Za-z0-9_]*)"))
        .as_ref()
        .map_err(|err| {
            crate::protocol::internal_error(&format!(
                "Failed to initialize SUPER method-call regex: {err}"
            ))
        })
}

#[cfg(feature = "workspace")]
fn get_goto_label_regex() -> Result<&'static regex::Regex, JsonRpcError> {
    GOTO_LABEL_RE
        .get_or_init(|| regex::Regex::new(r"\bgoto\s+([A-Za-z_][A-Za-z0-9_]*)"))
        .as_ref()
        .map_err(|err| {
            crate::protocol::internal_error(&format!(
                "Failed to initialize goto label regex: {err}"
            ))
        })
}

#[cfg(feature = "workspace")]
fn get_label_declaration_regex() -> Result<&'static regex::Regex, JsonRpcError> {
    LABEL_DECLARATION_RE
        .get_or_init(|| regex::Regex::new(r"(?m)^\s*([A-Za-z_][A-Za-z0-9_]*)\s*:"))
        .as_ref()
        .map_err(|err| {
            crate::protocol::internal_error(&format!(
                "Failed to initialize label declaration regex: {err}"
            ))
        })
}

#[cfg(feature = "workspace")]
fn find_label_declaration_span(
    text: &str,
    label: &str,
) -> Result<Option<(usize, usize)>, JsonRpcError> {
    let label_re = get_label_declaration_regex()?;
    Ok(label_re.captures_iter(text).find_map(|cap| {
        let declared_label = cap.get(1)?;
        (declared_label.as_str() == label).then_some((declared_label.start(), declared_label.end()))
    }))
}

#[derive(Debug, Clone)]
enum EarlyDefinitionTarget {
    /// Cursor is on a `use Module` / `require Module` statement.
    /// @INC filtering applies: if file-system resolution fails, the workspace
    /// index must also be filtered through `EffectiveIncContext`.
    UseModule(String),
    /// Cursor is on a bare `Package->method` reference.
    /// @INC filtering does not apply — workspace-index method lookup is correct.
    Module(String),
    XsBootstrap(String),
}

/// Look up a symbol definition in the workspace index.
///
/// Tries two lookup strategies:
/// 1. `find_def()` with a structured `SymbolKey`
/// 2. `find_definition()` with a formatted `Package::name` string
///
/// Returns the LSP location if found, or `None` to fall through to same-file resolution.
#[cfg(feature = "workspace")]
fn find_workspace_definition_location(
    workspace_index: &crate::workspace_index::WorkspaceIndex,
    pkg: &str,
    name: &str,
) -> Option<crate::workspace_index::Location> {
    let key = crate::workspace_index::SymbolKey {
        pkg: pkg.to_string().into(),
        name: name.to_string().into(),
        sigil: None,
        kind: crate::workspace_index::SymKind::Sub,
    };

    workspace_index
        .find_def(&key)
        .or_else(|| workspace_index.find_definition(&format!("{pkg}::{name}")))
}

#[cfg(feature = "workspace")]
fn autoload_definition_location(
    workspace_index: &crate::workspace_index::WorkspaceIndex,
    receiver_pkg: &str,
    include_receiver: bool,
) -> Option<crate::workspace_index::Location> {
    include_receiver
        .then(|| find_workspace_definition_location(workspace_index, receiver_pkg, "AUTOLOAD"))
        .flatten()
        .or_else(|| inherited_method_definition_location(workspace_index, receiver_pkg, "AUTOLOAD"))
}

#[cfg(feature = "workspace")]
fn find_plack_middleware_definition_location(
    workspace_index: &crate::workspace_index::WorkspaceIndex,
    module_name: &str,
) -> Option<crate::workspace_index::Location> {
    let expected_suffix =
        std::path::PathBuf::from(format!("{}.pm", module_name.replace("::", "/")));

    for symbol in workspace_index.all_symbols() {
        if symbol.kind != crate::workspace_index::SymbolKind::Package {
            continue;
        }

        let matches_name =
            symbol.name == module_name || symbol.qualified_name.as_deref() == Some(module_name);
        if !matches_name {
            continue;
        }

        if let Some(fs_path) = crate::workspace_index::uri_to_fs_path(&symbol.uri) {
            if fs_path.ends_with(&expected_suffix) {
                return Some(crate::workspace_index::Location {
                    uri: symbol.uri,
                    range: symbol.range,
                });
            }
        }
    }

    None
}

#[cfg(feature = "workspace")]
pub(super) fn workspace_document_text(
    workspace_index: &crate::workspace_index::WorkspaceIndex,
    uri: &str,
) -> Option<String> {
    workspace_index.document_store().get_text(uri).or_else(|| {
        crate::workspace_index::uri_to_fs_path(uri)
            .and_then(|path| read_text_file_with_encoding(&path).ok())
    })
}

#[cfg(feature = "workspace")]
fn inherited_method_definition_location(
    workspace_index: &crate::workspace_index::WorkspaceIndex,
    receiver_pkg: &str,
    method_name: &str,
) -> Option<crate::workspace_index::Location> {
    let mut visited = HashSet::from([receiver_pkg.to_string()]);
    let mut queue = VecDeque::new();
    let mut related_package_cache: HashMap<String, Vec<String>> = HashMap::new();

    let mut enqueue_related_packages =
        |package_name: &str, queue: &mut VecDeque<String>, visited: &HashSet<String>| {
            let related_packages = related_package_cache
                .entry(package_name.to_string())
                .or_insert_with(|| {
                    let Some(package_location) = workspace_index.find_definition(package_name)
                    else {
                        return Vec::new();
                    };
                    let Some(text) =
                        workspace_document_text(workspace_index, &package_location.uri)
                    else {
                        return Vec::new();
                    };

                    let mut parser = Parser::new(&text);
                    let Ok(ast) = parser.parse() else {
                        return Vec::new();
                    };

                    crate::semantic::SemanticAnalyzer::analyze_with_source(&ast, &text)
                        .class_models
                        .into_iter()
                        .find(|model| model.name == package_name)
                        .map(|model| {
                            // Include both parent classes and composed roles in the BFS
                            // so that `with 'Role'` methods are resolved alongside
                            // `extends`/`use parent` methods.
                            // NOTE: BFS visited-set (above) handles diamond and circular inheritance.
                            // NOTE: C3 MRO ordering is a pre-existing approximation; BFS does not
                            // honour strict C3 order. Filed as follow-up (see issue #3482).
                            model
                                .parents
                                .iter()
                                .chain(model.roles.iter())
                                .cloned()
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .clone();

            for related_package in related_packages {
                if !visited.contains(&related_package) {
                    queue.push_back(related_package);
                }
            }
        };

    enqueue_related_packages(receiver_pkg, &mut queue, &visited);

    while let Some(package_name) = queue.pop_front() {
        if !visited.insert(package_name.clone()) {
            continue;
        }

        if let Some(location) =
            find_workspace_definition_location(workspace_index, &package_name, method_name)
        {
            tracing::debug!(
                receiver_pkg,
                package_name,
                method_name,
                "resolved inherited/role method definition"
            );
            return Some(location);
        }

        enqueue_related_packages(&package_name, &mut queue, &visited);
    }

    None
}

#[cfg(feature = "workspace")]
fn find_symbol_key_definition_location(
    workspace_index: &crate::workspace_index::WorkspaceIndex,
    symbol_key: &crate::workspace_index::SymbolKey,
) -> Option<crate::workspace_index::Location> {
    if symbol_key.kind == crate::workspace_index::SymKind::Pack
        && symbol_key.pkg.starts_with("Plack::Middleware::")
    {
        if let Some(location) =
            find_plack_middleware_definition_location(workspace_index, symbol_key.pkg.as_ref())
        {
            return Some(location);
        }
    }

    if symbol_key.kind == crate::workspace_index::SymKind::Sub && symbol_key.sigil.is_none() {
        find_workspace_definition_location(workspace_index, &symbol_key.pkg, &symbol_key.name)
            .or_else(|| {
                inherited_method_definition_location(
                    workspace_index,
                    &symbol_key.pkg,
                    &symbol_key.name,
                )
            })
    } else {
        workspace_index.find_def(symbol_key).or_else(|| {
            let symbol_name = if symbol_key.kind == crate::workspace_index::SymKind::Sub {
                format!("{}::{}", symbol_key.pkg, symbol_key.name)
            } else {
                symbol_key.name.to_string()
            };
            workspace_index.find_definition(&symbol_name)
        })
    }
}

#[cfg(feature = "workspace")]
fn lookup_workspace_definition(
    coordinator: Option<&std::sync::Arc<crate::workspace_index::IndexCoordinator>>,
    pkg: &str,
    name: &str,
    doc_uri: Option<&str>,
) -> Option<Value> {
    let coord = coordinator?;

    let workspace_index = coord.index();

    // Search for symbols with folder-aware ranking if we have document context
    let ranked_symbols = if let Some(uri) = doc_uri {
        workspace_index.search_symbols_ranked(name, uri)
    } else {
        workspace_index.search_symbols(name)
    };

    // Find the first matching symbol that matches the package
    for symbol in ranked_symbols {
        // Check if this symbol matches our package
        if symbol.container_name.as_deref() == Some(pkg)
            || symbol.qualified_name.as_ref().map(|q| q.starts_with(pkg)).unwrap_or(false)
        {
            if let Some(lsp_location) = crate::workspace_index::lsp_adapter::to_lsp_location(
                &crate::workspace_index::Location { uri: symbol.uri.clone(), range: symbol.range },
            ) {
                return Some(json!([lsp_location]));
            }
        }
    }

    // Fallback to original lookup methods for backward compatibility
    if let Some(def_location) = find_workspace_definition_location(workspace_index, pkg, name)
        .or_else(|| inherited_method_definition_location(workspace_index, pkg, name))
        .or_else(|| {
            if is_universal_method(name) {
                find_workspace_definition_location(workspace_index, "UNIVERSAL", name)
            } else {
                None
            }
        })
    {
        if let Some(lsp_location) =
            crate::workspace_index::lsp_adapter::to_lsp_location(&def_location)
        {
            return Some(json!([lsp_location]));
        }
    }

    None
}

const UNIVERSAL_METHODS: [&str; 4] = ["can", "isa", "DOES", "VERSION"];

fn is_universal_method(name: &str) -> bool {
    UNIVERSAL_METHODS.contains(&name)
}

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
fn semantic_definition_symbol(key: &crate::workspace_index::SymbolKey) -> String {
    if key.kind == crate::workspace_index::SymKind::Pack
        || key.pkg.is_empty()
        || key.name.contains("::")
    {
        key.name.to_string()
    } else {
        format!("{}::{}", key.pkg, key.name)
    }
}

#[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
fn semantic_definition_query_symbol(
    key: &crate::workspace_index::SymbolKey,
    current_package: &str,
    import_source: Option<&str>,
) -> String {
    let is_import_resolved_bare_sub = key.kind == crate::workspace_index::SymKind::Sub
        && key.sigil.is_none()
        && !key.name.contains("::")
        && key.pkg.as_ref() != current_package
        && import_source.is_some_and(|source| source == key.pkg.as_ref());

    if is_import_resolved_bare_sub { key.name.to_string() } else { semantic_definition_symbol(key) }
}

#[cfg(feature = "workspace")]
fn cursor_in_regex_capture(regex: &regex::Regex, text: &str, cursor: usize, group: usize) -> bool {
    regex
        .captures_iter(text)
        .any(|cap| cap.get(group).is_some_and(|m| cursor >= m.start() && cursor <= m.end()))
}

impl LspServer {
    fn navigation_decision_trace_context(
        params: Option<&Value>,
        provider: &'static str,
        provider_action: &'static str,
        include_declaration: Option<bool>,
    ) -> Result<Option<NavigationDecisionTraceContext>, JsonRpcError> {
        let Some(params) = params else {
            return Ok(None);
        };
        let uri = req_uri(params)?.to_string();
        let (line, character) = req_position(params)?;
        Ok(Some(NavigationDecisionTraceContext {
            provider,
            provider_action,
            uri,
            line,
            character,
            include_declaration,
        }))
    }

    fn record_navigation_provider_decision_trace(
        &self,
        context: Option<&NavigationDecisionTraceContext>,
        result: Option<&Value>,
    ) {
        let Some(context) = context else {
            return;
        };
        let result_count = lsp_location_count(result);
        let (decision, reason, fallback_state) = if result_count == 0 {
            ("fallback", "no_result", "no_result")
        } else {
            ("acted", "live_provider_result", "live_provider")
        };

        self.record_provider_decision_trace(
            context.provider,
            &json!({
                "provider": context.provider,
                "provider_action": context.provider_action,
                "decision": decision,
                "reason": reason,
                "uri": context.uri,
                "line": context.line,
                "character": context.character,
                "include_declaration": context.include_declaration,
                "result_count": result_count,
                "fact_source": "navigation_provider",
                "confidence": "low",
                "freshness": "fresh",
                "source_backed": false,
                "source_backed_state": "not_proven_by_provider_trace",
                "fallback_state": fallback_state,
                "dynamic_boundary": false,
                "trace_only_no_live_behavior_change": true,
                "claim_boundary": "records existing navigation response only; no broader live navigation cutover"
            }),
        );
    }

    /// Handle textDocument/declaration request
    pub(crate) fn handle_declaration(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let t0 = std::time::Instant::now();

        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            // Reject stale requests (parity with hover.rs:51-53 and completion.rs:312)
            let req_version =
                params["textDocument"]["version"].as_i64().and_then(|n| i32::try_from(n).ok());
            self.ensure_latest(uri, req_version)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Use the Declaration provider - ast is already an Arc
                    let provider = crate::declaration::DeclarationProvider::new(
                        Arc::clone(ast),
                        doc.text.clone(),
                        uri.to_string(),
                    )
                    .with_parent_map(&doc.parent_map)
                    .with_doc_version(doc.version);

                    // Find declaration at the position
                    if let Some(location_links) = provider.find_declaration(offset, doc.version) {
                        // Check client capability and return appropriate format
                        if self.client_capabilities.lock().declaration_link_support {
                            // Return LocationLink format
                            let result: Vec<Value> = location_links
                                .iter()
                                .map(|link| {
                                    let (orig_start_line, orig_start_char) =
                                        self.offset_to_pos16(doc, link.origin_selection_range.0);
                                    let (orig_end_line, orig_end_char) =
                                        self.offset_to_pos16(doc, link.origin_selection_range.1);

                                    let (target_start_line, target_start_char) =
                                        self.offset_to_pos16(doc, link.target_range.0);
                                    let (target_end_line, target_end_char) =
                                        self.offset_to_pos16(doc, link.target_range.1);

                                    let (sel_start_line, sel_start_char) =
                                        self.offset_to_pos16(doc, link.target_selection_range.0);
                                    let (sel_end_line, sel_end_char) =
                                        self.offset_to_pos16(doc, link.target_selection_range.1);

                                    json!({
                                            "originSelectionRange": {
                                                "start": {
                                                    "line": orig_start_line,
                                                    "character": orig_start_char,
                                                },
                                                "end": {
                                                    "line": orig_end_line,
                                                    "character": orig_end_char,
                                                },
                                            },
                                            "targetUri": link.target_uri,
                                            "targetRange": {
                                            "start": {
                                                "line": target_start_line,
                                                "character": target_start_char,
                                            },
                                            "end": {
                                                "line": target_end_line,
                                                "character": target_end_char,
                                            },
                                        },
                                        "targetSelectionRange": {
                                            "start": {
                                                "line": sel_start_line,
                                                "character": sel_start_char,
                                            },
                                            "end": {
                                                "line": sel_end_line,
                                                "character": sel_end_char,
                                            },
                                        },
                                    })
                                })
                                .collect();

                            return Ok(Some(json!(result)));
                        } else {
                            // Down-convert to Location format for clients that don't support LocationLink
                            let result: Vec<Value> = location_links
                                .iter()
                                .map(|link| {
                                    let (sel_start_line, sel_start_char) =
                                        self.offset_to_pos16(doc, link.target_selection_range.0);
                                    let (sel_end_line, sel_end_char) =
                                        self.offset_to_pos16(doc, link.target_selection_range.1);

                                    json!({
                                        "uri": link.target_uri,
                                        "range": {
                                            "start": {
                                                "line": sel_start_line,
                                                "character": sel_start_char,
                                            },
                                            "end": {
                                                "line": sel_end_line,
                                                "character": sel_end_char,
                                            },
                                        },
                                    })
                                })
                                .collect();

                            return Ok(Some(json!(result)));
                        }
                    }
                }

                // Performance monitoring
                let dt = t0.elapsed();
                if doc.text.len() < 50_000 && dt > std::time::Duration::from_millis(50) {
                    tracing::warn!(elapsed = ?dt, uri, "slow declaration");
                }
            }
        }
        Ok(Some(json!([])))
    }

    /// Handle textDocument/definition request
    pub(crate) fn handle_definition(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let trace_context = Self::navigation_decision_trace_context(
            params.as_ref(),
            "goto_definition",
            "textDocument/definition",
            None,
        )?;
        let result = self.handle_definition_inner(params)?;
        self.record_navigation_provider_decision_trace(trace_context.as_ref(), result.as_ref());
        Ok(result)
    }

    fn handle_definition_inner(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            // Reject stale requests (parity with hover.rs:51-53 and completion.rs:312)
            let req_version =
                params["textDocument"]["version"].as_i64().and_then(|n| i32::try_from(n).ok());
            self.ensure_latest(uri, req_version)?;

            // First, extract module reference info while holding the document lock briefly
            // We need to release the lock before calling resolve_module_to_path to avoid deadlock
            let module_lookup_info: Option<(EarlyDefinitionTarget, String, usize)> = {
                let documents = self.documents_guard();
                if let Some(doc) = self.get_document(&documents, uri) {
                    let offset = self.pos16_to_offset(doc, line, character);
                    let radius = 50;
                    let text_start = offset.saturating_sub(radius);
                    let text_around = self.get_text_around_offset(&doc.text, offset, radius);
                    let cursor_in_text = offset - text_start;
                    let current_package = doc.ast.as_ref().map_or_else(
                        || "main".to_string(),
                        |ast| crate::declaration::current_package_at(ast, offset).to_string(),
                    );

                    if let Some(module_name) =
                        extract_xs_bootstrap_target(&text_around, cursor_in_text, &current_package)
                    {
                        Some((
                            EarlyDefinitionTarget::XsBootstrap(module_name),
                            doc.text.clone(),
                            offset,
                        ))
                    } else if let Some(module_name) =
                        self.extract_module_reference_extended(&text_around, cursor_in_text)
                    {
                        Some((
                            EarlyDefinitionTarget::UseModule(module_name),
                            doc.text.clone(),
                            offset,
                        ))
                    } else {
                        // Also check if we're on a package name followed by ->
                        let mut package_name_result = None;
                        let package_pattern = get_package_arrow_regex()?;
                        for cap in package_pattern.captures_iter(&text_around) {
                            if let Some(package_match) = cap.get(1) {
                                let match_start = package_match.start();
                                let match_end = package_match.end();
                                if cursor_in_text >= match_start && cursor_in_text <= match_end {
                                    package_name_result = Some((
                                        EarlyDefinitionTarget::Module(
                                            package_match.as_str().to_string(),
                                        ),
                                        doc.text.clone(),
                                        offset,
                                    ));
                                    break;
                                }
                            }
                        }
                        package_name_result
                    }
                } else {
                    None
                }
            };
            // Lock is released here

            // Now resolve module to path WITHOUT holding the document lock
            if let Some((lookup_target, doc_text, doc_offset)) = module_lookup_info {
                match lookup_target {
                    EarlyDefinitionTarget::XsBootstrap(module_name) => {
                        if let Some(xs_path) = self.resolve_xs_bootstrap_path_with_uri(
                            &module_name,
                            Some(&doc_text),
                            Some(uri),
                        ) {
                            return Ok(Some(json!([xs_bootstrap_location(
                                &xs_path,
                                &module_name
                            )])));
                        }
                    }
                    EarlyDefinitionTarget::UseModule(module_name) => {
                        // Cursor is on a `use Module` / `require Module` statement.
                        // Resolution is authoritative: if the file-system resolver (which
                        // honours position-aware @INC including `no lib` cancellations) finds
                        // a path, return it. If not, return empty rather than falling through
                        // to the workspace-index lookup — the index is @INC-unaware and would
                        // surface files that `no lib` has cancelled. Fixes #8537.
                        if let Some(module_path) = self.resolve_module_to_path_with_doc_at_offset(
                            &module_name,
                            Some(&doc_text),
                            Some(uri),
                            Some(doc_offset),
                        ) {
                            return Ok(Some(json!([{
                                "uri": module_path,
                                "range": {
                                    "start": {
                                        "line": 0,
                                        "character": 0,
                                    },
                                    "end": {
                                        "line": 0,
                                        "character": 0,
                                    },
                                },
                            }])));
                        } else if is_core_perl_module(&module_name) {
                            // Core pragma — not on disk in the user's workspace, so no file jump
                            // is possible.  Log an info message to the LSP output channel
                            // (visible in the VSCode Output panel) so users can discover that
                            // hover (K) shows documentation for core modules.
                            let _ = self.log_message(
                                crate::runtime::window::MessageType::Info,
                                &format!(
                                    "'{module_name}' is a Perl core module. \
                                     No source file is available for goto-definition. \
                                     Use hover (K) to view documentation."
                                ),
                            );
                            tracing::debug!(
                                module = %module_name,
                                "core pragma requested via goto-def — no file target"
                            );
                        }
                        // Return early: file-system resolution is authoritative for `use Module`.
                        // Do NOT fall through to workspace-index lookup, which is @INC-unaware.
                        return Ok(Some(json!([])));
                    }
                    EarlyDefinitionTarget::Module(module_name) => {
                        if let Some(module_path) = self.resolve_module_to_path_with_doc_at_offset(
                            &module_name,
                            Some(&doc_text),
                            Some(uri),
                            Some(doc_offset),
                        ) {
                            return Ok(Some(json!([{
                                "uri": module_path,
                                "range": {
                                    "start": {
                                        "line": 0,
                                        "character": 0,
                                    },
                                    "end": {
                                        "line": 0,
                                        "character": 0,
                                    },
                                },
                            }])));
                        }
                    }
                }
            }

            // Continue with remaining definition lookup logic that needs document access
            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                let offset = self.pos16_to_offset(doc, line, character);
                let radius = 50;
                let text_start = offset.saturating_sub(radius);
                let text_around = self.get_text_around_offset(&doc.text, offset, radius);
                let cursor_in_text = offset - text_start;

                let goto_label_re = get_goto_label_regex()?;
                for cap in goto_label_re.captures_iter(&text_around) {
                    if let Some(label_match) = cap.get(1)
                        && cursor_in_text >= label_match.start()
                        && cursor_in_text <= label_match.end()
                        && let Some((target_start, target_end)) =
                            find_label_declaration_span(&doc.text, label_match.as_str())?
                    {
                        let (def_line, def_char) = self.offset_to_pos16(doc, target_start);
                        let (def_end_line, def_end_char) = self.offset_to_pos16(doc, target_end);
                        return Ok(Some(json!([{
                            "uri": uri,
                            "range": {
                                "start": {
                                    "line": def_line,
                                    "character": def_char,
                                },
                                "end": {
                                    "line": def_end_line,
                                    "character": def_end_char,
                                },
                            },
                        }])));
                    }
                }

                if let Some(mason_location) = self.resolve_mason_definition(uri, &doc.text, offset)
                {
                    if let Some(lsp_location) =
                        crate::workspace_index::lsp_adapter::to_lsp_location(&mason_location)
                    {
                        return Ok(Some(json!([lsp_location])));
                    }
                }

                #[cfg(feature = "workspace")]
                {
                    if let Some(ref ast) = doc.ast {
                        if let Some(coordinator) = self.coordinator() {
                            let workspace_index = coordinator.index();
                            let current_package =
                                crate::declaration::current_package_at(ast, offset);
                            if let Some(def_location) = resolve_mojolicious_route_definition(
                                workspace_index,
                                current_package,
                                &text_around,
                                cursor_in_text,
                            ) {
                                if let Some(lsp_location) =
                                    crate::workspace_index::lsp_adapter::to_lsp_location(
                                        &def_location,
                                    )
                                {
                                    return Ok(Some(json!([lsp_location])));
                                }
                            }
                        }
                    }

                    // Attempt to resolve `SUPER::method` calls using the current package's
                    // inheritance chain before falling back to generic fully-qualified lookup.
                    let current_package = doc
                        .ast
                        .as_ref()
                        .map(|ast| {
                            let byte_offset = self.pos16_to_offset(doc, line, character);
                            crate::declaration::current_package_at(ast, byte_offset)
                        })
                        .unwrap_or("main");

                    let super_re = get_super_method_regex()?;
                    for cap in super_re.captures_iter(&text_around) {
                        if let Some(method_match) = cap.get(1)
                            && cursor_in_text >= method_match.start()
                            && cursor_in_text <= method_match.end()
                        {
                            if let Some(ref ast) = doc.ast {
                                let analyzer =
                                    crate::semantic::SemanticAnalyzer::analyze_with_source(
                                        ast, &doc.text,
                                    );
                                if let Some(location) = analyzer.resolve_inherited_method_location(
                                    current_package,
                                    method_match.as_str(),
                                ) {
                                    let lsp_start = self.offset_to_pos16(doc, location.start);
                                    let lsp_end = self.offset_to_pos16(doc, location.end);
                                    return Ok(Some(json!([{
                                        "uri": uri,
                                        "range": {
                                            "start": {"line": lsp_start.0, "character": lsp_start.1},
                                            "end": {"line": lsp_end.0, "character": lsp_end.1},
                                        },
                                    }])));
                                }
                            }

                            #[cfg(feature = "workspace")]
                            {
                                if let Some(coordinator) = self.coordinator()
                                    && let Some(def_location) =
                                        inherited_method_definition_location(
                                            coordinator.index(),
                                            current_package,
                                            method_match.as_str(),
                                        )
                                        .or_else(|| {
                                            autoload_definition_location(
                                                coordinator.index(),
                                                current_package,
                                                false,
                                            )
                                        })
                                    && let Some(lsp_location) =
                                        crate::workspace_index::lsp_adapter::to_lsp_location(
                                            &def_location,
                                        )
                                {
                                    return Ok(Some(json!([lsp_location])));
                                }
                            }
                        }
                    }

                    // Attempt to resolve fully-qualified symbols like Package::sub
                    let fqn_regex = get_fqn_regex()?;
                    for cap in fqn_regex.captures_iter(&text_around) {
                        if let Some(m) = cap.get(1) {
                            if cursor_in_text >= m.start() && cursor_in_text <= m.end() {
                                let parts: Vec<&str> = m.as_str().split("::").collect();
                                if parts.len() >= 2 {
                                    let name = parts.last().copied().unwrap_or("");
                                    let pkg = parts[..parts.len() - 1].join("::");

                                    if let Some(result) = lookup_workspace_definition(
                                        self.coordinator(),
                                        &pkg,
                                        name,
                                        Some(uri),
                                    ) {
                                        return Ok(Some(result));
                                    }
                                    // Partial/None: fall through to same-file resolution
                                }
                                break;
                            }
                        }
                    }

                    // Attempt to resolve Package->method calls
                    let arrow_re = get_arrow_method_regex()?;
                    for cap in arrow_re.captures_iter(&text_around) {
                        if let (Some(package_match), Some(method_match)) = (cap.get(1), cap.get(2))
                        {
                            if cursor_in_text >= method_match.start()
                                && cursor_in_text <= method_match.end()
                            {
                                let package_name = package_match.as_str();
                                let method_name = method_match.as_str();

                                if let Some(result) = lookup_workspace_definition(
                                    self.coordinator(),
                                    package_name,
                                    method_name,
                                    Some(uri),
                                ) {
                                    return Ok(Some(result));
                                }
                                #[cfg(feature = "workspace")]
                                {
                                    if let Some(coordinator) = self.coordinator()
                                        && let Some(def_location) = autoload_definition_location(
                                            coordinator.index(),
                                            package_name,
                                            true,
                                        )
                                        && let Some(lsp_location) =
                                            crate::workspace_index::lsp_adapter::to_lsp_location(
                                                &def_location,
                                            )
                                    {
                                        return Ok(Some(json!([lsp_location])));
                                    }
                                }
                                if is_universal_method(method_name)
                                    && let Some(result) = lookup_workspace_definition(
                                        self.coordinator(),
                                        "UNIVERSAL",
                                        method_name,
                                        Some(uri),
                                    )
                                {
                                    return Ok(Some(result));
                                }
                                // Partial/None: fall through to same-file resolution
                                break;
                            }
                        }
                    }

                    // Attempt to resolve $var->method() calls (e.g., $self->method())
                    // For $self/$this/$class, resolve using the current package context
                    let var_method_re = get_var_method_regex()?;
                    for cap in var_method_re.captures_iter(&text_around) {
                        if let (Some(var_match), Some(method_match)) = (cap.get(1), cap.get(2)) {
                            if cursor_in_text >= method_match.start()
                                && cursor_in_text <= method_match.end()
                            {
                                let var_name = var_match.as_str();
                                let method_name = method_match.as_str();

                                // For $self/$this/$class, resolve using current package
                                if var_name == "self" || var_name == "this" || var_name == "class" {
                                    if let Some(ref ast) = doc.ast {
                                        let byte_offset =
                                            self.pos16_to_offset(doc, line, character);
                                        let current_package =
                                            crate::declaration::current_package_at(
                                                ast,
                                                byte_offset,
                                            );
                                        if let Some(result) = lookup_workspace_definition(
                                            self.coordinator(),
                                            current_package,
                                            method_name,
                                            Some(uri),
                                        ) {
                                            return Ok(Some(result));
                                        }
                                        #[cfg(feature = "workspace")]
                                        {
                                            if let Some(coordinator) = self.coordinator()
                                                && let Some(def_location) =
                                                    autoload_definition_location(
                                                        coordinator.index(),
                                                        current_package,
                                                        true,
                                                    )
                                                && let Some(lsp_location) =
                                                    crate::workspace_index::lsp_adapter::to_lsp_location(
                                                        &def_location,
                                                    )
                                            {
                                                return Ok(Some(json!([lsp_location])));
                                            }
                                        }
                                    }
                                }
                                if is_universal_method(method_name)
                                    && let Some(result) = lookup_workspace_definition(
                                        self.coordinator(),
                                        "UNIVERSAL",
                                        method_name,
                                        Some(uri),
                                    )
                                {
                                    return Ok(Some(result));
                                }
                                // Fall through for non-self variables
                                break;
                            }
                        }
                    }
                }

                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
                    {
                        let cursor_on_arrow_method = cursor_in_regex_capture(
                            get_arrow_method_regex()?,
                            &text_around,
                            cursor_in_text,
                            2,
                        ) || cursor_in_regex_capture(
                            get_var_method_regex()?,
                            &text_around,
                            cursor_in_text,
                            2,
                        );
                        if !cursor_on_arrow_method {
                            let current_package =
                                crate::declaration::current_package_at(ast, offset);
                            if let Some(symbol_key) =
                                crate::declaration::symbol_at_cursor_with_source(
                                    ast,
                                    offset,
                                    current_package,
                                    &doc.text,
                                )
                            {
                                let workspace_symbol_key =
                                    super::to_workspace_symbol_key(&symbol_key);
                                let import_source =
                                    self.find_import_source(ast, &workspace_symbol_key.name);
                                let semantic_symbol = semantic_definition_query_symbol(
                                    &workspace_symbol_key,
                                    current_package,
                                    import_source.as_deref(),
                                );
                                if let Some(lsp_location) = self.live_exact_definition_location(
                                    uri,
                                    &semantic_symbol,
                                    offset,
                                ) {
                                    return Ok(Some(json!([lsp_location])));
                                }
                            }
                        }
                    }

                    // Try DeclarationProvider first (it handles function calls properly)
                    let provider = crate::declaration::DeclarationProvider::new(
                        Arc::clone(ast),
                        doc.text.clone(),
                        uri.to_string(),
                    )
                    .with_parent_map(&doc.parent_map)
                    .with_doc_version(doc.version);

                    if let Some(location_links) = provider.find_declaration(offset, doc.version) {
                        // Convert to Location format for definition
                        let result: Vec<Value> = location_links
                            .iter()
                            .map(|link| {
                                let (sel_start_line, sel_start_char) =
                                    self.offset_to_pos16(doc, link.target_selection_range.0);
                                let (sel_end_line, sel_end_char) =
                                    self.offset_to_pos16(doc, link.target_selection_range.1);

                                json!({
                                    "uri": link.target_uri,
                                    "range": {
                                        "start": {
                                            "line": sel_start_line,
                                            "character": sel_start_char,
                                        },
                                        "end": {
                                            "line": sel_end_line,
                                            "character": sel_end_char,
                                        },
                                    },
                                })
                            })
                            .collect();

                        if !result.is_empty() {
                            return Ok(Some(json!(result)));
                        }
                    }

                    // Try workspace index for cross-file definitions using routing policy
                    #[cfg(feature = "workspace")]
                    {
                        if let Some(coordinator) = self.coordinator() {
                            let workspace_index = coordinator.index();
                            // Use symbol_at_cursor to get the symbol key
                            let current_package =
                                crate::declaration::current_package_at(ast, offset);
                            if let Some(symbol_key) =
                                crate::declaration::symbol_at_cursor_with_source(
                                    ast,
                                    offset,
                                    current_package,
                                    &doc.text,
                                )
                            {
                                tracing::debug!(symbol_key = ?symbol_key, "looking for definition");
                                let workspace_symbol_key =
                                    super::to_workspace_symbol_key(&symbol_key);

                                if let Some(def_location) = find_symbol_key_definition_location(
                                    workspace_index,
                                    &workspace_symbol_key,
                                ) {
                                    tracing::debug!(location = ?def_location, "found definition");
                                    // Convert internal Location to LSP Location
                                    if let Some(lsp_location) =
                                        crate::workspace_index::lsp_adapter::to_lsp_location(
                                            &def_location,
                                        )
                                    {
                                        return Ok(Some(json!([lsp_location])));
                                    }
                                }

                                if workspace_symbol_key.kind == crate::workspace_index::SymKind::Sub
                                    && workspace_symbol_key.sigil.is_none()
                                    && let Some(import_source) =
                                        self.find_import_source(ast, &workspace_symbol_key.name)
                                    && let Some(def_location) = find_workspace_definition_location(
                                        workspace_index,
                                        &import_source,
                                        &workspace_symbol_key.name,
                                    )
                                    && let Some(lsp_location) =
                                        crate::workspace_index::lsp_adapter::to_lsp_location(
                                            &def_location,
                                        )
                                {
                                    tracing::debug!(
                                        symbol = %workspace_symbol_key.name,
                                        source_pkg = %import_source,
                                        "resolved bare imported symbol through require/import source"
                                    );
                                    return Ok(Some(json!([lsp_location])));
                                }
                            }
                        }
                        // No coordinator: fall through to same-file semantic model
                    }

                    // Fall back to same-file definition
                    let model = crate::semantic::SemanticModel::build(ast, &doc.text);

                    // Find definition at the position
                    if let Some(definition) = model.definition_at(offset) {
                        let (def_line, def_char) =
                            self.offset_to_pos16(doc, definition.location.start);
                        let (def_end_line, def_end_char) =
                            self.offset_to_pos16(doc, definition.location.end);

                        return Ok(Some(json!([{
                            "uri": uri,
                            "range": {
                                "start": {
                                    "line": def_line,
                                    "character": def_char,
                                },
                                "end": {
                                    "line": def_end_line,
                                    "character": def_end_char,
                                },
                            },
                        }])));
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle definition request with cancellation support
    pub(crate) fn handle_definition_cancellable(
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
            if let Some(ref tid) = typed_id {
                let token = GLOBAL_CANCELLATION_REGISTRY.get_token(tid).unwrap_or_else(|| {
                    let token = PerlLspCancellationToken::new(
                        tid.clone(),
                        "textDocument/definition".to_string(),
                    );
                    let _ = GLOBAL_CANCELLATION_REGISTRY.register_token(token.clone());
                    token
                });

                // Early cancellation check with relaxed read
                if token.is_cancelled_relaxed() {
                    return Err(JsonRpcError {
                        code: REQUEST_CANCELLED,
                        message: "Request cancelled - definition provider".to_string(),
                        data: None,
                    });
                }
            }

            // Delegate to original handler
            self.handle_definition(Some(params))
        } else {
            self.handle_definition(params)
        }
    }

    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub(crate) fn definition_runtime_quality_receipt(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let live_provider_result = self.handle_definition(params.clone())?;
        let live_provider_count = lsp_location_count(live_provider_result.as_ref());

        #[cfg(not(all(feature = "workspace", not(target_arch = "wasm32"))))]
        {
            Ok(Some(json!({
                "provider": "definition",
                "live_provider_result": live_provider_result,
                "live_provider_count": live_provider_count,
                "compiler_receipt": null,
                "no_live_behavior_change": true,
                "note": "definition runtime proof unavailable without workspace semantic queries"
            })))
        }

        #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
        {
            let Some(params) = params else {
                return Ok(Some(json!({
                    "provider": "definition",
                    "live_provider_result": live_provider_result,
                    "live_provider_count": live_provider_count,
                    "compiler_receipt": null,
                    "no_live_behavior_change": true,
                    "note": "definition runtime proof missing request params"
                })));
            };

            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;
            let Some((symbol, byte_offset)) = self.navigation_runtime_symbol(uri, line, character)
            else {
                return Ok(Some(json!({
                    "provider": "definition",
                    "live_provider_result": live_provider_result,
                    "live_provider_count": live_provider_count,
                    "compiler_receipt": null,
                    "no_live_behavior_change": true,
                    "note": "definition runtime proof found no symbol at request position"
                })));
            };

            let compiler_receipt = match route_index_access(self.coordinator()) {
                IndexAccessMode::Full(coordinator) => {
                    let index = coordinator.index();
                    index.with_semantic_queries_for_uri(uri, |file_id, queries| {
                        let ctx = QueryContext::new(file_id, None, Some(byte_offset));
                        let mut receipt = goto_definition_live_exact_or_imported(
                            index.as_ref(),
                            &queries,
                            &symbol,
                            &ctx,
                        )
                        .receipt;
                        let compiler_result_count = receipt.new_result.match_count;
                        receipt.notes.push(format!(
                            "definition runtime proof: live_provider_results={live_provider_count}; compiler_fact_candidates={}; compiler_result_count={}; partial live exact/imported cutover",
                            compiler_result_count, compiler_result_count
                        ));
                        receipt
                    })
                }
                IndexAccessMode::Partial(_) | IndexAccessMode::None => None,
            };

            Ok(Some(json!({
                "provider": "definition",
                "symbol": symbol,
                "live_provider_result": live_provider_result,
                "live_provider_count": live_provider_count,
                "compiler_receipt": compiler_receipt,
                "no_live_behavior_change": false,
                "live_cutover": "partial_exact_imported"
            })))
        }
    }

    #[cfg(all(feature = "workspace", any(test, feature = "expose_lsp_test_api")))]
    fn navigation_runtime_symbol(
        &self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Option<(String, u32)> {
        let documents = self.documents_guard();
        let doc = self.get_document(&documents, uri)?;
        let offset = self.pos16_to_offset(doc, line, character);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ast) = doc.ast.as_ref() {
            let current_package = crate::declaration::current_package_at(ast, offset);
            if let Some(symbol_key) = crate::declaration::symbol_at_cursor_with_source(
                ast,
                offset,
                current_package,
                &doc.text,
            ) {
                let workspace_symbol_key = super::to_workspace_symbol_key(&symbol_key);
                let import_source = self.find_import_source(ast, &workspace_symbol_key.name);
                let symbol = semantic_definition_query_symbol(
                    &workspace_symbol_key,
                    current_package,
                    import_source.as_deref(),
                );
                let byte_offset = u32::try_from(offset).ok()?;
                return Some((symbol, byte_offset));
            }
        }
        let symbol = token_under_cursor(&doc.text, line as usize, character as usize)?;
        if symbol.is_empty() {
            return None;
        }
        let byte_offset = u32::try_from(offset).ok()?;
        Some((symbol, byte_offset))
    }

    #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
    fn live_exact_definition_location(
        &self,
        uri: &str,
        symbol: &str,
        byte_offset: usize,
    ) -> Option<Value> {
        let byte_offset = u32::try_from(byte_offset).ok()?;
        let workspace_index = self.workspace_index()?;
        let outcome = workspace_index.with_semantic_queries_for_uri(uri, |file_id, queries| {
            let ctx = QueryContext::new(file_id, None, Some(byte_offset));
            goto_definition_live_exact_or_imported(workspace_index.as_ref(), &queries, symbol, &ctx)
        })?;

        let DefinitionCutoverResult::Exact(candidate) = outcome.result else {
            return None;
        };
        let def_location = workspace_index.semantic_anchor_wire_location(candidate.anchor_id)?;
        let location: lsp_types::Location = def_location.into();
        serde_json::to_value(location).ok()
    }

    /// Handle textDocument/typeDefinition request
    pub(crate) fn handle_type_definition(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        use crate::features::type_definition::TypeDefinitionProvider;

        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;
            let trace_context = NavigationDecisionTraceContext {
                provider: "type_definition",
                provider_action: "textDocument/typeDefinition",
                uri: uri.to_string(),
                line,
                character,
                include_declaration: None,
            };
            let req_version =
                params["textDocument"]["version"].as_i64().and_then(|n| i32::try_from(n).ok());
            if let Err(error) = self.ensure_latest(uri, req_version) {
                self.record_type_definition_provider_decision_trace(
                    &trace_context,
                    0,
                    stale_type_definition_fallback_trace(),
                );
                return Err(error);
            }

            // Acquire minimal data under lock, then drop it
            let (ast, doc_text) = {
                let documents = self.documents_guard();
                let Some(doc) = self.get_document(&documents, uri) else {
                    self.record_type_definition_provider_decision_trace(
                        &trace_context,
                        0,
                        TypeDefinitionFallbackTrace::default(),
                    );
                    return Ok(Some(json!([])));
                };
                let Some(ast) = doc.ast.as_ref() else {
                    self.record_type_definition_provider_decision_trace(
                        &trace_context,
                        0,
                        TypeDefinitionFallbackTrace::default(),
                    );
                    return Ok(Some(json!([])));
                };
                (ast.clone(), doc.text.clone())
            };

            // Build doc_map outside the lock using snapshot helper
            let doc_map: HashMap<String, String> =
                self.documents_text_snapshot().into_iter().collect();

            let provider = TypeDefinitionProvider::new();
            if let Some(locations) =
                provider.find_type_definition(ast.as_ref(), line, character, uri, &doc_map)
            {
                if locations.len() == 1 {
                    self.record_type_definition_provider_decision_trace(
                        &trace_context,
                        locations.len(),
                        TypeDefinitionFallbackTrace::default(),
                    );
                    return Ok(Some(json!(locations)));
                }

                self.record_type_definition_ambiguous_identity_trace(
                    &trace_context,
                    locations.len(),
                );
                return Ok(Some(json!([])));
            }
            self.record_type_definition_provider_decision_trace(
                &trace_context,
                0,
                classify_type_definition_fallback_trace(&doc_text, line, character),
            );
        }

        Ok(Some(json!([])))
    }

    fn record_type_definition_provider_decision_trace(
        &self,
        context: &NavigationDecisionTraceContext,
        result_count: usize,
        fallback_trace: TypeDefinitionFallbackTrace,
    ) {
        let acted = result_count > 0;
        let result_count = u64::try_from(result_count).unwrap_or(u64::MAX);
        let mut receipt = json!({
            "provider": context.provider,
            "provider_action": context.provider_action,
            "decision": if acted { "acted" } else { "fallback" },
            "reason": if acted { "source_backed_high_confidence" } else { fallback_trace.reason },
            "uri": context.uri,
            "line": context.line,
            "character": context.character,
            "result_count": result_count,
            "live_provider_result_count": result_count,
            "fact_source": if acted { "parser_syntax" } else { fallback_trace.fact_source },
            "confidence": if acted { "high" } else { "low" },
            "freshness": if acted { "fresh" } else { fallback_trace.freshness },
            "source_backed": acted,
            "source_backed_state": if acted {
                "open_document_type_definition"
            } else {
                fallback_trace.source_backed_state
            },
            "fallback": if acted { "none" } else { "no_result" },
            "fallback_state": if acted { "none" } else { "no_result" },
            "dynamic_boundary": if acted { false } else { fallback_trace.dynamic_boundary },
            "trace_only_no_live_behavior_change": true,
            "claim_boundary": "records existing type-definition safe subset only; direct package/class identifiers and constructor receivers may resolve to open-document package definitions while variable receivers, chained method results, function-call results, missing package definitions, generated/no-source facts, dynamic boundaries, stale facts, low-confidence facts, and ambiguous identities remain fallback or blocked"
        });
        if !acted && let Some(object) = receipt.as_object_mut() {
            object.insert("blocker".to_string(), json!(fallback_trace.blocker));
        }

        self.record_provider_decision_trace(context.provider, &receipt);
    }

    fn record_type_definition_ambiguous_identity_trace(
        &self,
        context: &NavigationDecisionTraceContext,
        candidate_count: usize,
    ) {
        let candidate_count = u64::try_from(candidate_count).unwrap_or(u64::MAX);
        let receipt = json!({
            "provider": context.provider,
            "provider_action": context.provider_action,
            "decision": "fallback",
            "reason": "ambiguous_low_confidence_candidates",
            "blocker": "ambiguous_identity",
            "uri": context.uri,
            "line": context.line,
            "character": context.character,
            "result_count": 0,
            "live_provider_result_count": 0,
            "ambiguous_candidate_count": candidate_count,
            "fact_source": "parser_syntax",
            "confidence": "low",
            "freshness": "fresh",
            "source_backed": false,
            "source_backed_state": "ambiguous_type_definition_identity",
            "fallback": "no_result",
            "fallback_state": "no_result",
            "dynamic_boundary": false,
            "trace_only_no_live_behavior_change": false,
            "claim_boundary": "blocks ambiguous type-definition identities; direct package/class identifiers and constructor receivers may resolve only when they identify one open-document package definition, while duplicate package declarations, variable receivers, chained method results, function-call results, missing package definitions, generated/no-source facts, dynamic boundaries, stale facts, low-confidence facts, and unsupported identities remain fallback or blocked"
        });

        self.record_provider_decision_trace(context.provider, &receipt);
    }

    /// Handle textDocument/implementation request
    pub(crate) fn handle_implementation(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            // Acquire minimal data under lock, then drop it
            let ast = {
                let documents = self.documents_guard();
                let Some(doc) = self.get_document(&documents, uri) else {
                    return Ok(Some(json!([])));
                };
                let Some(ast) = doc.ast.as_ref() else {
                    return Ok(Some(json!([])));
                };
                ast.clone()
            };

            #[cfg(feature = "workspace")]
            {
                // Build doc_map outside the lock using snapshot helper
                let doc_map: HashMap<String, String> =
                    self.documents_text_snapshot().into_iter().collect();

                // Use routing policy - only provide workspace index in Full mode
                let access_mode = route_index_access(self.coordinator());
                let workspace_index = if let IndexAccessMode::Full(coordinator) = access_mode {
                    Some(coordinator.index().clone())
                } else {
                    // Partial/None: same-file analysis only
                    None
                };

                let provider = ImplementationProvider::new(workspace_index);
                let locations =
                    provider.find_implementations(ast.as_ref(), line, character, uri, &doc_map);
                return Ok(Some(json!(locations)));
            }

            #[cfg(not(feature = "workspace"))]
            {
                let _ = (ast, line, character, uri); // Suppress unused warnings
            }
        }

        Ok(Some(json!([])))
    }

    /// Non-blocking definition handler with fallback
    pub(crate) fn on_definition(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let uri = params.pointer("/textDocument/uri").and_then(|v| v.as_str()).unwrap_or("");
        let line = params.pointer("/position/line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let ch =
            params.pointer("/position/character").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let text = self.buffer_text(uri).unwrap_or_default();
        let module = token_under_cursor(&text, line, ch).filter(|s| s.contains("::"));

        if let Some(m) = module {
            if let Some(path) = self.resolve_module_path_with_uri(&m, Some(&text), Some(uri)) {
                let loc = location_from_path(&path);
                return Ok(serde_json::json!([loc]));
            }
        }

        // Fallback: try existing analysis
        // For now, just return empty array
        Ok(serde_json::json!([]))
    }
}
