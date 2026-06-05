//! Missing-module lookup explanation command.
//!
//! This command exposes the existing module-resolution state as a user-facing
//! receipt. It does not perform a new workspace scan or change PL701 behavior.

use super::super::*;
use crate::protocol::invalid_params;
use perl_module::resolution::{
    IncRoot, IncRootKind, ModuleUriResolution, resolve_module_uri_with_effective_inc,
};
use std::path::{Path, PathBuf};
use std::time::Duration;

const MISSING_MODULE_LOOKUP_SCHEMA_VERSION: &str = "missing_module_lookup_explanation.v1";
const EXPLAIN_MISSING_MODULE_LOOKUP_COMMAND: &str = "perl.explainMissingModuleLookup";

impl LspServer {
    pub(crate) fn explain_missing_module_lookup(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let params = params.ok_or_else(|| {
            invalid_missing_module_lookup_params("Missing missing-module lookup argument")
        })?;
        let request = MissingModuleLookupRequest::from_value(&params)?;

        let (doc_text, doc_offset, document_open) =
            self.missing_module_document_context(request.doc_uri.as_deref(), request.position);
        let context = match self.effective_inc_context_for_doc(
            request.doc_uri.as_deref(),
            doc_text.as_deref(),
            doc_offset,
        ) {
            Some(context) => context,
            None => {
                let payload = missing_module_root_missing_payload(&request);
                return Ok(Some(payload));
            }
        };

        let workspace_folders = self.workspace_folders.lock().clone();
        let workspace_folder_uris: Vec<String> =
            workspace_folders.iter().map(|folder| folder.uri.clone()).collect();
        let workspace_folder_paths = workspace_folder_paths(&workspace_folder_uris, &context.root);

        let timeout = Duration::from_millis(context.resolution_timeout_ms);
        let open_document_uris: Vec<String> = {
            let documents = self.documents.lock();
            documents
                .keys()
                .filter(|uri| doc_offset.is_none() || context.symbol_uri_reachable(uri))
                .cloned()
                .collect()
        };

        let resolution = resolve_module_uri_with_effective_inc(
            &request.module,
            &open_document_uris,
            &workspace_folder_uris,
            &context.effective_roots,
            timeout,
        );
        let expected_relative_path = expected_module_relative_path(&request.module);
        let searched_inc_paths = searched_inc_paths(
            &context.effective_roots,
            &workspace_folder_paths,
            &expected_relative_path,
            &context.root,
        );
        let result = module_lookup_result(
            &request.module,
            &resolution,
            context.resolution_timeout_ms,
            &open_document_uris,
        );
        let user_message = missing_module_user_message(
            &request.module,
            &resolution,
            context.resolution_timeout_ms,
        );
        let workspace_roots = workspace_folder_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>();

        Ok(Some(json!({
            "schema_version": MISSING_MODULE_LOOKUP_SCHEMA_VERSION,
            "command": EXPLAIN_MISSING_MODULE_LOOKUP_COMMAND,
            "requested_module": request.module,
            "expected_relative_path": expected_relative_path,
            "text_document_uri": request.doc_uri,
            "request_position": request.position.map(|(line, character)| json!({
                "line": line,
                "character": character,
            })),
            "document_open": document_open,
            "workspace": {
                "root": context.root.display().to_string(),
                "folder_uri": context.folder_uri,
                "workspace_root_count": workspace_folder_paths.len(),
            },
            "module_resolution": {
                "result": result,
                "effective_include_paths": searched_inc_paths,
                "open_document_uri_count": open_document_uris.len(),
                "use_system_inc": context.use_system_inc,
                "use_perl5lib": context.use_perl5lib,
                "perl5lib_policy": perl5lib_policy(context.use_perl5lib),
                "resolution_timeout_ms": context.resolution_timeout_ms,
                "dot_or_workspace_root_caveat": dot_or_workspace_root_caveat(
                    &context.effective_roots,
                    &context.root,
                ),
            },
            "user_message": user_message,
            "claim_boundary": "explains one missing-module lookup using existing runtime @INC state only; no workspace scan, diagnostic suppression change, resolver behavior change, or support-tier promotion",
            "copyable_payload": {
                "schema_version": MISSING_MODULE_LOOKUP_SCHEMA_VERSION,
                "perl_lsp_version": env!("CARGO_PKG_VERSION"),
                "provider": "module_resolution",
                "command": EXPLAIN_MISSING_MODULE_LOOKUP_COMMAND,
                "requested_module": request.module,
                "expected_relative_path": expected_relative_path,
                "result": copyable_resolution_result(&resolution),
                "workspace_root_class": workspace_root_class(&workspace_roots),
                "workspace_root_hash": workspace_root_hash(&workspace_roots),
                "effective_include_path_count": context.effective_roots.len(),
                "use_system_inc": context.use_system_inc,
                "use_perl5lib": context.use_perl5lib,
                "perl5lib_policy": perl5lib_policy(context.use_perl5lib),
                "support_tier_link": "docs/project/status/SUPPORT_TIERS.md#claim-rows",
                "request_position": request.position.map(|(line, character)| json!({
                    "line": line,
                    "character": character,
                })),
            },
        })))
    }

    fn missing_module_document_context(
        &self,
        doc_uri: Option<&str>,
        position: Option<(u32, u32)>,
    ) -> (Option<String>, Option<usize>, bool) {
        let Some(uri) = doc_uri else {
            return (None, None, false);
        };
        let documents = self.documents.lock();
        let Some(doc) = self.get_document(&documents, uri) else {
            return (None, None, false);
        };

        let offset = position.map(|(line, character)| self.pos16_to_offset(doc, line, character));
        (Some(doc.text.clone()), offset, true)
    }
}

struct MissingModuleLookupRequest {
    module: String,
    doc_uri: Option<String>,
    position: Option<(u32, u32)>,
}

impl MissingModuleLookupRequest {
    fn from_value(value: &Value) -> Result<Self, JsonRpcError> {
        if let Some(module) = value.as_str() {
            return Ok(Self { module: module.to_string(), doc_uri: None, position: None });
        }

        let module = request_module(value).ok_or_else(|| {
            invalid_missing_module_lookup_params("Missing module for missing-module lookup")
        })?;
        let doc_uri = request_doc_uri(value);
        let position = request_position(value);

        Ok(Self { module, doc_uri, position })
    }
}

fn invalid_missing_module_lookup_params(summary: &str) -> JsonRpcError {
    invalid_params(&format!(
        "{summary}: {EXPLAIN_MISSING_MODULE_LOOKUP_COMMAND} expects either a module string such as \"Missing::Module\" or an object with module, moduleName, or diagnostic.data.module; optionally include textDocument.uri and position or pass a PL701 diagnostic to explain lookup from a document"
    ))
}

fn request_module(value: &Value) -> Option<String> {
    value
        .get("module")
        .and_then(Value::as_str)
        .or_else(|| value.get("moduleName").and_then(Value::as_str))
        .or_else(|| value.pointer("/diagnostic/data/module").and_then(Value::as_str))
        .map(str::to_string)
        .or_else(|| {
            value
                .pointer("/diagnostic/message")
                .and_then(Value::as_str)
                .and_then(extract_module_from_pl701_message)
        })
}

fn request_doc_uri(value: &Value) -> Option<String> {
    value
        .pointer("/textDocument/uri")
        .and_then(Value::as_str)
        .or_else(|| value.get("uri").and_then(Value::as_str))
        .or_else(|| value.pointer("/request_position/uri").and_then(Value::as_str))
        .map(str::to_string)
}

fn request_position(value: &Value) -> Option<(u32, u32)> {
    value.get("position").or_else(|| value.pointer("/diagnostic/range/start")).and_then(
        |position| {
            let line = position.get("line").and_then(Value::as_u64)?;
            let character = position.get("character").and_then(Value::as_u64)?;
            let line = u32::try_from(line).ok()?;
            let character = u32::try_from(character).ok()?;
            Some((line, character))
        },
    )
}

fn extract_module_from_pl701_message(message: &str) -> Option<String> {
    let start = message.find("Module '")? + "Module '".len();
    let rest = &message[start..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

fn expected_module_relative_path(module: &str) -> String {
    format!("{}.pm", module.replace("::", "/"))
}

fn searched_inc_paths(
    roots: &[IncRoot],
    workspace_folder_paths: &[PathBuf],
    relative_path: &str,
    workspace_root: &Path,
) -> Vec<Value> {
    let mut ordered_roots = roots.to_vec();
    ordered_roots.sort_by_key(|root| root.precedence);
    ordered_roots
        .iter()
        .map(|root| {
            let candidates = candidate_paths_for_root(root, workspace_folder_paths, relative_path)
                .into_iter()
                .map(|path| {
                    json!({
                        "path": path.display().to_string(),
                        "exists": path.is_file(),
                        "inside_workspace": path.starts_with(workspace_root),
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "path": root.path.display().to_string(),
                "source": root_source_label(&root.source),
                "kind": inc_root_kind_label(root.kind),
                "precedence": root.precedence,
                "candidate_paths": candidates,
            })
        })
        .collect()
}

fn candidate_paths_for_root(
    root: &IncRoot,
    workspace_folder_paths: &[PathBuf],
    relative_path: &str,
) -> Vec<PathBuf> {
    match root.kind {
        IncRootKind::FileLocalLexical | IncRootKind::WorkspaceRelative => {
            if root.path.is_absolute() {
                vec![root.path.join(relative_path)]
            } else {
                workspace_folder_paths
                    .iter()
                    .map(|workspace| {
                        if root.path == Path::new(".") {
                            workspace.join(relative_path)
                        } else {
                            workspace.join(&root.path).join(relative_path)
                        }
                    })
                    .collect()
            }
        }
        IncRootKind::ExternalAbsolute
        | IncRootKind::Perl5LibEnv
        | IncRootKind::InterpreterStartup
        | IncRootKind::RuntimeDerived => vec![root.path.join(relative_path)],
    }
}

fn module_lookup_result(
    module: &str,
    resolution: &ModuleUriResolution,
    timeout_ms: u64,
    open_document_uris: &[String],
) -> Value {
    match resolution {
        ModuleUriResolution::Resolved(uri) => {
            let source = if open_document_uris.iter().any(|open_uri| open_uri == uri) {
                "open_document"
            } else {
                "effective_inc"
            };
            json!({
                "status": "resolved",
                "resolved": true,
                "resolved_uri": uri,
                "source": source,
                "why": "A source-backed module file matched the requested module name.",
            })
        }
        ModuleUriResolution::TimedOut => json!({
            "status": "timed_out",
            "resolved": false,
            "resolved_uri": null,
            "source": "timeout",
            "why": format!(
                "Lookup for module {module} exceeded the configured {timeout_ms}ms resolution timeout."
            ),
        }),
        ModuleUriResolution::NotFound => json!({
            "status": "not_found",
            "resolved": false,
            "resolved_uri": null,
            "source": "effective_inc",
            "why": "No open document or searched @INC candidate matched the expected relative module path.",
        }),
    }
}

fn copyable_resolution_result(resolution: &ModuleUriResolution) -> &'static str {
    match resolution {
        ModuleUriResolution::Resolved(_) => "resolved",
        ModuleUriResolution::TimedOut => "timed_out",
        ModuleUriResolution::NotFound => "not_found",
    }
}

fn missing_module_user_message(
    module: &str,
    resolution: &ModuleUriResolution,
    timeout_ms: u64,
) -> String {
    match resolution {
        ModuleUriResolution::Resolved(_) => {
            format!(
                "Module {module} resolved through the current effective @INC state. This explanation does not change diagnostics."
            )
        }
        ModuleUriResolution::TimedOut => {
            format!(
                "Module {module} lookup timed out after {timeout_ms}ms. Consider increasing `perl.workspace.resolutionTimeout` for slow filesystems."
            )
        }
        ModuleUriResolution::NotFound => {
            format!(
                "Module {module} was not found in the current effective @INC state. Check `perl.workspace.includePaths`, PERL5LIB policy, or install the module."
            )
        }
    }
}

fn missing_module_root_missing_payload(request: &MissingModuleLookupRequest) -> Value {
    json!({
        "schema_version": MISSING_MODULE_LOOKUP_SCHEMA_VERSION,
        "command": EXPLAIN_MISSING_MODULE_LOOKUP_COMMAND,
        "requested_module": request.module,
        "expected_relative_path": expected_module_relative_path(&request.module),
        "text_document_uri": request.doc_uri,
        "request_position": request.position.map(|(line, character)| json!({
            "line": line,
            "character": character,
        })),
        "module_resolution": {
            "result": {
                "status": "workspace_root_missing",
                "resolved": false,
                "resolved_uri": null,
                "source": "workspace",
                "why": "No workspace root is available, so perl-lsp cannot assemble effective @INC roots.",
            },
            "effective_include_paths": [],
            "open_document_uri_count": 0,
            "use_system_inc": false,
            "use_perl5lib": false,
            "perl5lib_policy": "workspace_root_missing",
            "resolution_timeout_ms": null,
            "dot_or_workspace_root_caveat": false,
        },
        "user_message": format!(
            "Module {} could not be checked because no workspace root is available. Open a project folder before using module lookup explanations.",
            request.module
        ),
        "claim_boundary": "explains one missing-module lookup using existing runtime @INC state only; no workspace scan, diagnostic suppression change, resolver behavior change, or support-tier promotion",
        "copyable_payload": {
            "schema_version": MISSING_MODULE_LOOKUP_SCHEMA_VERSION,
            "perl_lsp_version": env!("CARGO_PKG_VERSION"),
            "provider": "module_resolution",
            "command": EXPLAIN_MISSING_MODULE_LOOKUP_COMMAND,
            "requested_module": request.module,
            "expected_relative_path": expected_module_relative_path(&request.module),
            "result": "workspace_root_missing",
            "workspace_root_class": "none",
            "workspace_root_hash": null,
            "effective_include_path_count": 0,
            "use_system_inc": false,
            "use_perl5lib": false,
            "perl5lib_policy": "workspace_root_missing",
            "support_tier_link": "docs/project/status/SUPPORT_TIERS.md#claim-rows",
        },
    })
}

fn workspace_folder_paths(workspace_folder_uris: &[String], fallback_root: &Path) -> Vec<PathBuf> {
    let mut paths = workspace_folder_uris
        .iter()
        .filter_map(|uri| super::super::source_path_from_uri(uri))
        .collect::<Vec<_>>();
    if paths.is_empty() {
        paths.push(fallback_root.to_path_buf());
    }
    paths
}

fn root_source_label(source: &str) -> &'static str {
    match source {
        "use-lib-lexical" => "use lib",
        "workspace-include-paths" => "workspace includePaths",
        "perl5lib-env" => "PERL5LIB",
        "interpreter-startup-inc" => "interpreter startup @INC",
        _ => "unknown @INC source",
    }
}

fn inc_root_kind_label(kind: IncRootKind) -> &'static str {
    match kind {
        IncRootKind::FileLocalLexical => "file_local_lexical",
        IncRootKind::WorkspaceRelative => "workspace_relative",
        IncRootKind::ExternalAbsolute => "external_absolute",
        IncRootKind::Perl5LibEnv => "perl5lib_env",
        IncRootKind::InterpreterStartup => "interpreter_startup",
        IncRootKind::RuntimeDerived => "runtime_derived",
    }
}

fn perl5lib_policy(use_perl5lib: bool) -> &'static str {
    if !use_perl5lib {
        return "disabled_by_workspace_config";
    }

    if std::env::var_os("PERL5LIB").is_some() {
        "enabled_from_environment"
    } else {
        "enabled_but_environment_empty"
    }
}

fn dot_or_workspace_root_caveat(roots: &[IncRoot], workspace_root: &Path) -> bool {
    roots.iter().any(|root| {
        if root.path == Path::new(".") {
            return true;
        }
        let absolute = if root.path.is_absolute() {
            root.path.clone()
        } else {
            workspace_root.join(&root.path)
        };
        absolute == workspace_root
    })
}

fn workspace_root_class(workspace_roots: &[String]) -> &'static str {
    match workspace_roots.len() {
        0 => "none",
        1 => "single_root",
        _ => "multi_root",
    }
}

fn workspace_root_hash(workspace_roots: &[String]) -> Option<String> {
    if workspace_roots.is_empty() {
        return None;
    }

    let mut roots = workspace_roots.iter().map(|root| root.replace('\\', "/")).collect::<Vec<_>>();
    roots.sort();
    Some(format!("{:x}", md5::compute(roots.join("\n"))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must_err;
    use serde_json::json;

    #[test]
    fn missing_module_lookup_argument_errors_include_guidance()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();

        let missing_arg = must_err(server.explain_missing_module_lookup(None));
        assert_lookup_shape_guidance(&missing_arg, "Missing missing-module lookup argument");

        let missing_module = match MissingModuleLookupRequest::from_value(&json!({})) {
            Ok(_) => return Err("expected missing module request to fail".into()),
            Err(error) => error,
        };
        assert_lookup_shape_guidance(&missing_module, "Missing module for missing-module lookup");

        Ok(())
    }

    fn assert_lookup_shape_guidance(error: &JsonRpcError, summary: &str) {
        assert_eq!(error.code, crate::protocol::INVALID_PARAMS);
        assert!(error.data.is_none());
        assert!(
            error.message.contains(summary),
            "error message must preserve summary; got {}",
            error.message
        );
        assert!(
            error.message.contains(EXPLAIN_MISSING_MODULE_LOOKUP_COMMAND),
            "error message must name command; got {}",
            error.message
        );
        for expected in [
            "Missing::Module",
            "module",
            "moduleName",
            "diagnostic.data.module",
            "textDocument.uri",
            "position",
            "PL701",
        ] {
            assert!(
                error.message.contains(expected),
                "error message must mention {expected}; got {}",
                error.message
            );
        }
    }
}
