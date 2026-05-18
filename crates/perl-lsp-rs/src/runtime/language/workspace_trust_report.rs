//! Workspace trust report command.
//!
//! The report is intentionally read-only: it summarizes state the server
//! already has instead of scanning files, probing Perl, or recomputing provider
//! evidence.

use super::super::*;
use perl_lsp_rs_core::config::{Perl5LibPrecedence, WorkspaceConfig};
use std::sync::atomic::Ordering;

const WORKSPACE_TRUST_REPORT_SCHEMA_VERSION: &str = "workspace_trust_report.v1";

fn perl5lib_precedence_label(precedence: &Perl5LibPrecedence) -> &'static str {
    match precedence {
        Perl5LibPrecedence::Prepend => "prepend",
        Perl5LibPrecedence::Append => "append",
    }
}

fn configured_perl_path(config: &WorkspaceConfig) -> Option<&str> {
    match config.perl_path.as_deref().map(str::trim) {
        Some(path) if !path.is_empty() => Some(path),
        _ => None,
    }
}

fn perl5lib_paths(config: &WorkspaceConfig) -> Vec<String> {
    if !config.use_perl5lib {
        return Vec::new();
    }

    std::env::var("PERL5LIB")
        .map(|value| WorkspaceConfig::parse_perl5lib(&value))
        .unwrap_or_default()
}

fn setup_hint(code: &str, severity: &str, message: &str, action: &str) -> Value {
    json!({
        "code": code,
        "severity": severity,
        "message": message,
        "action": action,
    })
}

fn setup_hints_summary(config: &WorkspaceConfig) -> Value {
    let perl5lib_paths = perl5lib_paths(config);
    let mut hints = Vec::new();

    if configured_perl_path(config).is_none() {
        hints.push(setup_hint(
            "perl_path_uses_path",
            "info",
            "No explicit Perl binary is configured; perl-lsp will resolve `perl` from PATH when a subprocess needs it.",
            "Set `perl.workspace.perlPath` when the editor should use a specific Perl.",
        ));
    }

    if config.include_paths.is_empty() {
        hints.push(setup_hint(
            "include_paths_empty",
            "warning",
            "No workspace include paths are configured.",
            "Configure `perl.workspace.includePaths` so module resolution can find project libraries.",
        ));
    } else if config.include_paths.iter().any(|path| path.trim().is_empty()) {
        hints.push(setup_hint(
            "include_path_empty_entry",
            "warning",
            "At least one configured include path is empty.",
            "Remove empty entries from `perl.workspace.includePaths`.",
        ));
    }

    if config.use_perl5lib {
        if perl5lib_paths.is_empty() {
            hints.push(setup_hint(
                "perl5lib_enabled_empty",
                "info",
                "PERL5LIB participation is enabled, but the current environment has no PERL5LIB entries.",
                "Configure `perl.workspace.includePaths` for project-specific libraries instead of relying on ambient shell state.",
            ));
        }
    } else {
        hints.push(setup_hint(
            "perl5lib_disabled",
            "info",
            "PERL5LIB is not inherited by workspace module resolution.",
            "Configure `perl.workspace.includePaths` for paths the editor should search.",
        ));
    }

    if config.use_system_inc {
        hints.push(setup_hint(
            "system_inc_not_probed_by_report",
            "info",
            "Interpreter startup @INC is enabled, but the workspace trust report does not probe Perl.",
            "Use provider receipts or module lookup explanations for request-local @INC evidence.",
        ));
    }

    let hint_count = hints.len();
    json!({
        "status": "advisory",
        "hint_count": hint_count,
        "hints": hints,
        "perl_binary": {
            "configured_path": configured_perl_path(config),
            "resolution_status": if configured_perl_path(config).is_some() {
                "configured_not_probed_by_report"
            } else {
                "uses_path_when_needed_not_probed_by_report"
            },
            "version_status": "not_probed_by_report",
            "args_count": config.perl_args.len(),
        },
        "perldoc": perldoc_runtime_state(config),
        "dap": {
            "status": "not_probed_by_lsp_workspace_report",
            "policy": "DAP Perl path and module paths are configured by debug launch state and are not probed by this read-only LSP trust report.",
        },
        "claim_boundary": "Setup hints are derived from current configuration and environment counts only. They do not resolve Perl, run perldoc, inspect DAP sessions, scan files, or change provider behavior.",
    })
}

fn perldoc_runtime_state(config: &WorkspaceConfig) -> Value {
    json!({
        "status": "oracle_contract_reported_not_run",
        "run_status": "not_run_by_report",
        "binary_source": if configured_perl_path(config).is_some() {
            "configured_perl_toolchain_or_perldoc_on_path_when_opened"
        } else {
            "perldoc_on_path_when_opened"
        },
        "timeout_ms": 10_000,
        "allow_perl5lib": config.use_perl5lib,
        "allow_perl5opt": false,
        "allow_local_lib": false,
        "lc_all": "C",
        "argv_policy": "perldoc -T -- <module>",
        "policy": "perldoc:// requests use the Perl oracle environment when opened; this report exposes the contract from configuration but does not resolve or run perldoc.",
    })
}

fn string_field(value: Option<&Value>, field: &str) -> Option<String> {
    value.and_then(|object| object.get(field)).and_then(Value::as_str).map(str::to_string)
}

fn bool_field(value: Option<&Value>, field: &str) -> Option<bool> {
    value.and_then(|object| object.get(field)).and_then(Value::as_bool)
}

fn number_field(value: Option<&Value>, field: &str) -> Option<u64> {
    value.and_then(|object| object.get(field)).and_then(Value::as_u64)
}

fn allowed_launch_path_class(key: &str) -> bool {
    matches!(
        key,
        "absolute"
            | "relative"
            | "workspace_variable"
            | "file_variable"
            | "other_variable"
            | "home_relative"
            | "command"
            | "empty"
            | "unknown"
    )
}

fn path_class_counts_field(value: Option<&Value>, field: &str) -> Value {
    let mut counts = serde_json::Map::new();
    if let Some(object) = value.and_then(|parent| parent.get(field)).and_then(Value::as_object) {
        for (key, count) in object {
            if allowed_launch_path_class(key) {
                if let Some(count) = count.as_u64() {
                    counts.insert(key.clone(), json!(count));
                }
            }
        }
    }
    Value::Object(counts)
}

fn launch_configuration_summary(dap: Option<&Value>) -> Value {
    let launch_config = dap.and_then(|value| value.get("launch_configuration"));
    json!({
        "status": string_field(launch_config, "status").unwrap_or_else(|| "not_supplied".to_string()),
        "configuration_count": number_field(launch_config, "configuration_count"),
        "perl_configuration_count": number_field(launch_config, "perl_configuration_count"),
        "launch_request_count": number_field(launch_config, "launch_request_count"),
        "attach_request_count": number_field(launch_config, "attach_request_count"),
        "perl_path_configured_count": number_field(launch_config, "perl_path_configured_count"),
        "include_paths_configured_count": number_field(launch_config, "include_paths_configured_count"),
        "include_path_entry_count": number_field(launch_config, "include_path_entry_count"),
        "non_string_include_path_count": number_field(launch_config, "non_string_include_path_count"),
        "program_configured_count": number_field(launch_config, "program_configured_count"),
        "cwd_configured_count": number_field(launch_config, "cwd_configured_count"),
        "include_path_kind_counts": path_class_counts_field(launch_config, "include_path_kind_counts"),
        "perl_path_kind_counts": path_class_counts_field(launch_config, "perl_path_kind_counts"),
        "program_path_kind_counts": path_class_counts_field(launch_config, "program_path_kind_counts"),
        "cwd_path_kind_counts": path_class_counts_field(launch_config, "cwd_path_kind_counts"),
        "claim_boundary": string_field(launch_config, "claim_boundary").unwrap_or_else(|| "Launch configuration state was not supplied by the client.".to_string()),
    })
}

fn client_runtime_state_summary(argument: Option<&Value>) -> Value {
    let client_state = argument.and_then(|value| value.get("client_runtime_state"));
    let perldoc = client_state.and_then(|value| value.get("perldoc"));
    let dap = client_state.and_then(|value| value.get("dap"));

    json!({
        "schema_version": "workspace_trust_client_runtime.v1",
        "source": string_field(client_state, "source").unwrap_or_else(|| "not_supplied".to_string()),
        "perldoc": {
            "status": string_field(perldoc, "status").unwrap_or_else(|| "not_supplied".to_string()),
            "uri_scheme": string_field(perldoc, "uri_scheme"),
            "client_surface": string_field(perldoc, "client_surface"),
        },
        "dap": {
            "status": string_field(dap, "status").unwrap_or_else(|| "not_supplied".to_string()),
            "adapter_registered": bool_field(dap, "adapter_registered"),
            "active_perl_debug_session": bool_field(dap, "active_perl_debug_session"),
            "managed_adapter_exists": bool_field(dap, "managed_adapter_exists"),
            "launch_json_workspace_count": number_field(dap, "launch_json_workspace_count"),
            "workspace_folder_count": number_field(dap, "workspace_folder_count"),
            "launch_configuration": launch_configuration_summary(dap),
        },
        "claim_boundary": "Client runtime state is caller-supplied and sanitized to known fields. It does not start DAP, run perldoc, probe Perl, scan workspace files beyond client-reported launch configuration state, or change provider behavior.",
    })
}

fn workspace_config_summary(config: &WorkspaceConfig) -> Value {
    let perl5lib_paths = perl5lib_paths(config);
    json!({
        "include_paths": &config.include_paths,
        "effective_include_paths": config.effective_include_paths(&perl5lib_paths),
        "use_system_inc": config.use_system_inc,
        "system_inc_status": if config.use_system_inc {
            "configured_not_probed_by_report"
        } else {
            "disabled"
        },
        "use_perl5lib": config.use_perl5lib,
        "perl5lib_entry_count": perl5lib_paths.len(),
        "perl5lib_precedence": perl5lib_precedence_label(&config.perl5lib_precedence),
        "perl_path": &config.perl_path,
        "perl_args_count": config.perl_args.len(),
        "resolution_timeout_ms": config.resolution_timeout_ms,
    })
}

fn support_tiers_summary() -> Value {
    json!({
        "parser": "measured-bounded",
        "module_resolution": "measured-bounded",
        "completion": "partial-live-with-fallback",
        "goto_definition": "partial-live-with-fallback",
        "references": "partial-live-with-fallback",
        "hover": "partial-live-with-fallback",
        "diagnostics": "partial-live-with-fallback",
        "rename": "partial-live-with-fallback",
        "safe_delete": "partial-live-with-fallback",
        "workspace_symbols": "partial-live-with-fallback",
        "document_symbols": "partial-live-with-fallback",
        "semantic_tokens": "partial-live-with-fallback",
        "provider_decision_explanations": "partial-live-with-fallback",
        "workspace_trust_report": "partial-live-with-fallback",
        "real_workspace_baseline": "measured-bounded",
    })
}

#[cfg(feature = "workspace")]
fn index_report(server: &LspServer) -> Value {
    let Some(coordinator) = server.coordinator() else {
        return json!({
            "availability": "none",
            "state": "unavailable",
            "reason": "workspace feature unavailable",
            "file_count": 0,
            "symbol_count": 0,
            "indexed_file_count": 0,
            "indexed_symbol_count": 0,
            "pending_index_tasks": server.pending_index_task_count.load(Ordering::Relaxed),
            "indexing_in_progress": false,
        });
    };

    let state = coordinator.state();
    let (availability, state_label, reason, state_file_count, state_symbol_count) = match state {
        crate::workspace_index::IndexState::Ready { file_count, symbol_count, .. } => {
            ("full", "ready", "ready", file_count, symbol_count)
        }
        crate::workspace_index::IndexState::Building {
            phase, indexed_count, total_count, ..
        } => {
            let reason = match phase {
                crate::workspace_index::IndexPhase::Idle => "index building (idle)",
                crate::workspace_index::IndexPhase::Scanning => {
                    "index building (scanning workspace)"
                }
                crate::workspace_index::IndexPhase::Indexing => {
                    if total_count == 0 || indexed_count < total_count {
                        "index building (indexing files)"
                    } else {
                        "index building"
                    }
                }
            };
            ("partial", "building", reason, indexed_count, coordinator.index().symbol_count())
        }
        crate::workspace_index::IndexState::Degraded { available_symbols, .. } => (
            "partial",
            "degraded",
            "index degraded",
            coordinator.index().file_count(),
            available_symbols,
        ),
    };

    json!({
        "availability": availability,
        "state": state_label,
        "reason": reason,
        "file_count": state_file_count,
        "symbol_count": state_symbol_count,
        "indexed_file_count": coordinator.index().file_count(),
        "indexed_symbol_count": coordinator.index().symbol_count(),
        "pending_index_tasks": server.pending_index_task_count.load(Ordering::Relaxed),
        "indexing_in_progress": server.indexing_in_progress.load(Ordering::Relaxed),
    })
}

#[cfg(not(feature = "workspace"))]
fn index_report(server: &LspServer) -> Value {
    json!({
        "availability": "none",
        "state": "unavailable",
        "reason": "workspace feature unavailable",
        "file_count": 0,
        "symbol_count": 0,
        "indexed_file_count": 0,
        "indexed_symbol_count": 0,
        "pending_index_tasks": server.pending_index_task_count.load(Ordering::Relaxed),
        "indexing_in_progress": false,
    })
}

impl LspServer {
    pub(crate) fn workspace_trust_report(
        &self,
        argument: Option<&Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let root_path = self.root_path.lock().clone();
        let folders = self.workspace_folders.lock().clone();
        let global_config = self.workspace_config.lock().clone();
        let open_document_count = self.documents.lock().len();
        let provider_trace_keys = {
            let mut keys: Vec<String> =
                self.provider_decision_traces.lock().keys().cloned().collect();
            keys.sort();
            keys
        };

        let folder_reports: Vec<Value> = folders
            .iter()
            .map(|folder| {
                json!({
                    "uri": &folder.uri,
                    "name": &folder.name,
                    "path": folder.path.as_ref().map(|path| path.display().to_string()),
                    "project_config_present": folder.project_config.is_some(),
                    "effective_workspace_config": workspace_config_summary(&folder.effective_workspace_config),
                })
            })
            .collect();

        Ok(Some(json!({
            "schema_version": WORKSPACE_TRUST_REPORT_SCHEMA_VERSION,
            "command": "perl.workspaceTrustReport",
            "user_message": "Perl LSP workspace trust report generated from current server state.",
            "claim_boundary": "This report aggregates existing runtime state only. It does not scan files, probe Perl, refresh parser corpus receipts, or promote provider support tiers.",
            "workspace": {
                "root_path": root_path.as_ref().map(|path| path.display().to_string()),
                "workspace_folder_count": folders.len(),
                "open_document_count": open_document_count,
                "folders": folder_reports,
            },
            "module_resolution": {
                "global_workspace_config": workspace_config_summary(&global_config),
                "policy": "Configured include paths and optional PERL5LIB participation are reported without probing interpreter startup @INC.",
            },
            "setup_hints": setup_hints_summary(&global_config),
            "client_runtime_state": client_runtime_state_summary(argument),
            "index": index_report(self),
            "providers": {
                "support_tiers": support_tiers_summary(),
                "decision_trace_count": provider_trace_keys.len(),
                "decision_trace_keys": provider_trace_keys,
            },
            "dynamic_boundaries": {
                "policy": "Generated, dynamic, stale, low-confidence, ambiguous, and fallback facts remain labeled, gated, or blocked according to each provider support tier."
            },
        })))
    }
}
