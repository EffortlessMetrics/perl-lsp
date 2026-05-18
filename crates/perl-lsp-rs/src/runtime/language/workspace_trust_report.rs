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

fn workspace_config_summary(config: &WorkspaceConfig) -> Value {
    let perl5lib_paths = if config.use_perl5lib {
        std::env::var("PERL5LIB")
            .map(|value| WorkspaceConfig::parse_perl5lib(&value))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
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
    pub(crate) fn workspace_trust_report(&self) -> Result<Option<Value>, JsonRpcError> {
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
