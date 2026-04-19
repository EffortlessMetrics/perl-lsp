//! Navigation handlers for go-to-definition, declaration, and related features
//!
//! Handles textDocument/declaration, textDocument/definition, textDocument/typeDefinition,
//! and textDocument/implementation requests.

use super::super::*;
use crate::cancellation::RequestCleanupGuard;
use crate::protocol::{req_position, req_uri};
use crate::util::token_under_cursor;
use std::collections::{HashMap, HashSet, VecDeque};

#[cfg(feature = "workspace")]
use crate::runtime::routing::{IndexAccessMode, route_index_access};
#[cfg(feature = "workspace")]
use std::sync::OnceLock;

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
static MOJO_STRING_ROUTE_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

#[cfg(feature = "workspace")]
static MOJO_KV_ROUTE_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

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

#[derive(Debug, Clone)]
enum EarlyDefinitionTarget {
    Module(String),
    XsBootstrap(String),
}

fn is_module_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':' | b'\'')
}

fn normalize_bootstrap_module(token: &str, current_package: &str) -> Option<String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed == "__PACKAGE__" {
        return Some(current_package.to_string());
    }

    let normalized = normalize_package_separator(trimmed).into_owned();
    let first = normalized.chars().next()?;
    if normalized.contains("::") || first.is_ascii_uppercase() {
        Some(normalized)
    } else {
        None
    }
}

fn parse_bootstrap_argument(
    text: &str,
    mut start: usize,
    current_package: &str,
) -> Option<(usize, usize, String)> {
    while let Some(byte) = text.as_bytes().get(start) {
        if byte.is_ascii_whitespace() || *byte == b',' {
            start += 1;
        } else {
            break;
        }
    }

    let bytes = text.as_bytes();
    let byte = *bytes.get(start)?;

    if byte == b'\'' || byte == b'"' {
        let quote = byte;
        let token_start = start + 1;
        let mut end = token_start;
        while let Some(next) = bytes.get(end) {
            if *next == quote {
                break;
            }
            end += 1;
        }
        let token = text.get(token_start..end)?;
        let module = normalize_bootstrap_module(token, current_package)?;
        return Some((token_start, end, module));
    }

    let mut end = start;
    while let Some(next) = bytes.get(end) {
        if is_module_token_byte(*next) {
            end += 1;
        } else {
            break;
        }
    }

    if end <= start {
        return None;
    }

    let token = text.get(start..end)?;
    let module = normalize_bootstrap_module(token, current_package)?;
    Some((start, end, module))
}

fn extract_xs_loader_target(
    text: &str,
    cursor: usize,
    current_package: &str,
    marker: &str,
) -> Option<String> {
    let mut search_from = 0;
    while let Some(found) = text.get(search_from..)?.find(marker) {
        let marker_start = search_from + found;
        let marker_end = marker_start + marker.len();
        let mut arg_start = marker_end;

        while let Some(byte) = text.as_bytes().get(arg_start) {
            if byte.is_ascii_whitespace() {
                arg_start += 1;
            } else {
                break;
            }
        }

        if text.as_bytes().get(arg_start) == Some(&b'(') {
            arg_start += 1;
        }

        if let Some((token_start, token_end, module_name)) =
            parse_bootstrap_argument(text, arg_start, current_package)
            && ((cursor >= marker_start && cursor <= marker_end)
                || (cursor >= token_start && cursor <= token_end))
        {
            return Some(module_name);
        }

        search_from = marker_end;
    }

    None
}

fn extract_bare_bootstrap_target(
    text: &str,
    cursor: usize,
    current_package: &str,
) -> Option<String> {
    let mut search_from = 0;
    let needle = "bootstrap";
    while let Some(found) = text.get(search_from..)?.find(needle) {
        let start = search_from + found;
        let end = start + needle.len();

        let left_ok = start == 0 || !is_module_token_byte(text.as_bytes()[start - 1]);
        let right_ok = end == text.len() || !is_module_token_byte(text.as_bytes()[end]);
        let qualified = start >= 2 && &text[start - 2..start] == "::";
        if !left_ok || !right_ok || qualified {
            search_from = end;
            continue;
        }

        if let Some((token_start, token_end, module_name)) =
            parse_bootstrap_argument(text, end, current_package)
            && ((cursor >= start && cursor <= end)
                || (cursor >= token_start && cursor <= token_end))
        {
            return Some(module_name);
        }

        search_from = end;
    }

    None
}

fn extract_qualified_bootstrap_target(text: &str, cursor: usize) -> Option<String> {
    let mut search_from = 0;
    let needle = "::bootstrap";
    while let Some(found) = text.get(search_from..)?.find(needle) {
        let suffix_start = search_from + found;
        let mut module_start = suffix_start;
        while module_start > 0 && is_module_token_byte(text.as_bytes()[module_start - 1]) {
            module_start -= 1;
        }

        if module_start == suffix_start {
            search_from = suffix_start + needle.len();
            continue;
        }

        let module = text.get(module_start..suffix_start)?;
        let module_name = normalize_bootstrap_module(module, "main")?;
        let full_end = suffix_start + needle.len();
        if cursor >= module_start && cursor <= full_end {
            return Some(module_name);
        }

        search_from = full_end;
    }

    None
}

fn extract_xs_bootstrap_target(text: &str, cursor: usize, current_package: &str) -> Option<String> {
    extract_xs_loader_target(text, cursor, current_package, "XSLoader::load")
        .or_else(|| {
            extract_xs_loader_target(text, cursor, current_package, "DynaLoader::bootstrap")
        })
        .or_else(|| extract_bare_bootstrap_target(text, cursor, current_package))
        .or_else(|| extract_qualified_bootstrap_target(text, cursor))
}

fn xs_boot_symbol_name(module_name: &str) -> String {
    format!(
        "boot_{}",
        normalize_package_separator(module_name).replace("::", "__")
    )
}

fn xs_bootstrap_location(path: &Path, module_name: &str) -> Value {
    let uri = Url::from_file_path(path)
        .map(|url| url.to_string())
        .unwrap_or_default();
    let boot_symbol = xs_boot_symbol_name(module_name);

    if let Ok(text) = std::fs::read_to_string(path)
        && let Some(offset) = text.find(&boot_symbol)
    {
        let (start_line, start_char) = byte_to_line_col(&text, offset);
        let (end_line, end_char) = byte_to_line_col(&text, offset + boot_symbol.len());
        return json!({
            "uri": uri,
            "range": {
                "start": {"line": start_line, "character": start_char},
                "end": {"line": end_line, "character": end_char},
            },
        });
    }

    location_from_path(path)
}

/// Returns `true` if the module name is a known Perl core pragma or standard module
/// that will never be found on disk in a user's workspace.
///
/// This list covers the pragmas and core modules that every Perl installation ships
/// with and that users most commonly reference with `use` or `require`.  It is
/// intentionally conservative — if a module is not listed here and is not found in
/// the workspace, the definition handler falls through to the normal "not found"
/// path unchanged.
fn is_core_perl_module(name: &str) -> bool {
    matches!(
        name,
        "strict"
            | "warnings"
            | "warnings::register"
            | "utf8"
            | "feature"
            | "constant"
            | "vars"
            | "lib"
            | "parent"
            | "base"
            | "overload"
            | "overloading"
            | "Scalar::Util"
            | "List::Util"
            | "Carp"
            | "Exporter"
            | "POSIX"
            | "Data::Dumper"
            | "File::Basename"
            | "File::Path"
            | "File::Spec"
            | "Storable"
            | "Encode"
            | "MIME::Base64"
            | "Digest::MD5"
            | "Digest::SHA"
            | "IO::File"
            | "IO::Handle"
            | "Fcntl"
            | "Socket"
            | "Time::HiRes"
            | "Time::Local"
            | "Getopt::Long"
            | "Pod::Usage"
    )
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
            .and_then(|path| std::fs::read_to_string(path).ok())
    })
}

#[cfg(feature = "workspace")]
fn get_mojo_string_route_regex() -> Result<&'static regex::Regex, JsonRpcError> {
    MOJO_STRING_ROUTE_RE
        .get_or_init(|| {
            regex::Regex::new(r"->\s*to\s*\(\s*'(?P<controller>[^'#]+)#(?P<action>[^']+)'\s*\)")
        })
        .as_ref()
        .map_err(|err| {
            crate::protocol::internal_error(&format!(
                "Failed to initialize Mojolicious route regex: {err}"
            ))
        })
}

#[cfg(feature = "workspace")]
fn get_mojo_kv_route_regex() -> Result<&'static regex::Regex, JsonRpcError> {
    MOJO_KV_ROUTE_RE
        .get_or_init(|| {
            regex::Regex::new(
                r"->\s*to\s*\(\s*controller\s*=>\s*'(?P<controller>[^']+)'\s*,\s*action\s*=>\s*'(?P<action>[^']+)'\s*\)",
            )
        })
        .as_ref()
        .map_err(|err| {
            crate::protocol::internal_error(&format!(
                "Failed to initialize Mojolicious route regex: {err}"
            ))
        })
}

#[cfg(feature = "workspace")]
fn mojolicious_app_root(current_package: &str) -> Option<String> {
    let package = current_package.trim();
    if package.is_empty() {
        return None;
    }

    Some(package.strip_suffix("::App").unwrap_or(package).to_string())
}

#[cfg(feature = "workspace")]
fn normalize_mojolicious_controller_name(raw: &str) -> Option<String> {
    let normalized = raw.trim().trim_matches('\'').trim_matches('"').trim();
    if normalized.is_empty() {
        return None;
    }

    let mut segments = Vec::new();
    for segment in normalized.split("::").flat_map(|part| part.split('/')) {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        let mut chars = segment.chars();
        let first = chars.next()?;
        let mut normalized_segment = first.to_uppercase().collect::<String>();
        normalized_segment.push_str(chars.as_str());
        segments.push(normalized_segment);
    }

    if segments.is_empty() {
        None
    } else {
        Some(segments.join("::"))
    }
}

#[cfg(feature = "workspace")]
fn resolve_mojolicious_route_definition(
    workspace_index: &crate::workspace_index::WorkspaceIndex,
    current_package: &str,
    text_around: &str,
    cursor_in_text: usize,
) -> Option<crate::workspace_index::Location> {
    let app_root = mojolicious_app_root(current_package)?;

    let try_route = |controller: &str, action: &str| {
        let controller_name = normalize_mojolicious_controller_name(controller)?;
        let package_name = format!("{app_root}::Controller::{controller_name}");
        find_workspace_definition_location(workspace_index, &package_name, action)
    };

    let string_re = get_mojo_string_route_regex().ok()?;
    for cap in string_re.captures_iter(text_around) {
        let Some(full_match) = cap.get(0) else {
            continue;
        };
        if cursor_in_text < full_match.start() || cursor_in_text >= full_match.end() {
            continue;
        }

        let Some(controller_match) = cap.name("controller") else {
            continue;
        };
        let Some(action_match) = cap.name("action") else {
            continue;
        };

        if (cursor_in_text >= controller_match.start() && cursor_in_text < controller_match.end())
            || (cursor_in_text >= action_match.start() && cursor_in_text < action_match.end())
        {
            if let Some(location) = try_route(controller_match.as_str(), action_match.as_str()) {
                return Some(location);
            }
        }
    }

    let kv_re = get_mojo_kv_route_regex().ok()?;
    for cap in kv_re.captures_iter(text_around) {
        let Some(full_match) = cap.get(0) else {
            continue;
        };
        if cursor_in_text < full_match.start() || cursor_in_text >= full_match.end() {
            continue;
        }

        let Some(controller_match) = cap.name("controller") else {
            continue;
        };
        let Some(action_match) = cap.name("action") else {
            continue;
        };

        if (cursor_in_text >= controller_match.start() && cursor_in_text < controller_match.end())
            || (cursor_in_text >= action_match.start() && cursor_in_text < action_match.end())
        {
            if let Some(location) = try_route(controller_match.as_str(), action_match.as_str()) {
                return Some(location);
            }
        }
    }

    None
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
            || symbol
                .qualified_name
                .as_ref()
                .map(|q| q.starts_with(pkg))
                .unwrap_or(false)
        {
            if let Some(lsp_location) = crate::workspace_index::lsp_adapter::to_lsp_location(
                &crate::workspace_index::Location {
                    uri: symbol.uri.clone(),
                    range: symbol.range,
                },
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

impl LspServer {
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
            let req_version = params["textDocument"]["version"]
                .as_i64()
                .and_then(|n| i32::try_from(n).ok());
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
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            // Reject stale requests (parity with hover.rs:51-53 and completion.rs:312)
            let req_version = params["textDocument"]["version"]
                .as_i64()
                .and_then(|n| i32::try_from(n).ok());
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
                            EarlyDefinitionTarget::Module(module_name),
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
                                crate::declaration::symbol_at_cursor(ast, offset, current_package)
                            {
                                tracing::debug!(symbol_key = ?symbol_key, "looking for definition");

                                if let Some(def_location) = find_symbol_key_definition_location(
                                    workspace_index,
                                    &symbol_key,
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
        // RAII guard ensures cleanup on all exit paths (early returns, errors, panics)
        let _cleanup_guard = RequestCleanupGuard::from_ref(request_id);

        if let Some(params) = params {
            // Create or get cancellation token for this request
            let token = if let Some(req_id) = request_id {
                GLOBAL_CANCELLATION_REGISTRY
                    .get_token(req_id)
                    .unwrap_or_else(|| {
                        let token = PerlLspCancellationToken::new(
                            req_id.clone(),
                            "textDocument/definition".to_string(),
                        );
                        let _ = GLOBAL_CANCELLATION_REGISTRY.register_token(token.clone());
                        token
                    })
            } else {
                PerlLspCancellationToken::new(
                    serde_json::Value::Null,
                    "textDocument/definition".to_string(),
                )
            };

            // Early cancellation check with relaxed read
            if token.is_cancelled_relaxed() {
                return Err(JsonRpcError {
                    code: REQUEST_CANCELLED,
                    message: "Request cancelled - definition provider".to_string(),
                    data: None,
                });
            }

            // Delegate to original handler
            self.handle_definition(Some(params))
        } else {
            self.handle_definition(params)
        }
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

            // Build doc_map outside the lock using snapshot helper
            let doc_map: HashMap<String, String> =
                self.documents_text_snapshot().into_iter().collect();

            let provider = TypeDefinitionProvider::new();
            if let Some(locations) =
                provider.find_type_definition(ast.as_ref(), line, character, uri, &doc_map)
            {
                return Ok(Some(json!(locations)));
            }
        }

        Ok(Some(json!([])))
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
        let uri = params
            .pointer("/textDocument/uri")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let line = params
            .pointer("/position/line")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let ch = params
            .pointer("/position/character")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

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
