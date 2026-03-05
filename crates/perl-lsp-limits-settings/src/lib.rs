//! JSON overlay helpers for LSP limits settings.
//!
//! This microcrate keeps settings parsing separate from runtime limit policy so
//! `perl-lsp-limits` can stay focused on limit defaults/accessors.

use serde_json::Value;

/// Trait implemented by limit types that can be updated from JSON settings.
pub trait LimitsSettings {
    fn set_workspace_symbol_cap(&mut self, value: usize);
    fn set_references_cap(&mut self, value: usize);
    fn set_completion_cap(&mut self, value: usize);
    fn set_ast_cache_max_entries(&mut self, value: usize);
    fn set_max_indexed_files(&mut self, value: usize);
    fn set_max_total_symbols(&mut self, value: usize);
    fn set_workspace_scan_deadline_ms(&mut self, value: u64);
    fn set_reference_search_deadline_ms(&mut self, value: u64);
}

/// Apply `perl.limits` settings values from an LSP settings payload.
pub fn apply_from_lsp_settings<T: LimitsSettings>(limits: &mut T, settings: &Value) {
    if let Some(limits_value) = settings.get("limits") {
        apply_limits_section(limits, limits_value);
    }
}

fn apply_limits_section<T: LimitsSettings>(limits: &mut T, limits_value: &Value) {
    if let Some(v) = limits_value.get("workspaceSymbolCap").and_then(serde_json::Value::as_u64) {
        limits.set_workspace_symbol_cap(v as usize);
    }

    if let Some(v) = limits_value.get("referencesCap").and_then(serde_json::Value::as_u64) {
        limits.set_references_cap(v as usize);
    }

    if let Some(v) = limits_value.get("completionCap").and_then(serde_json::Value::as_u64) {
        limits.set_completion_cap(v as usize);
    }

    if let Some(v) = limits_value.get("astCacheMaxEntries").and_then(serde_json::Value::as_u64) {
        limits.set_ast_cache_max_entries(v as usize);
    }

    if let Some(v) = limits_value.get("maxIndexedFiles").and_then(serde_json::Value::as_u64) {
        limits.set_max_indexed_files(v as usize);
    }

    if let Some(v) = limits_value.get("maxTotalSymbols").and_then(serde_json::Value::as_u64) {
        limits.set_max_total_symbols(v as usize);
    }

    if let Some(v) = limits_value.get("workspaceScanDeadlineMs").and_then(serde_json::Value::as_u64)
    {
        limits.set_workspace_scan_deadline_ms(v);
    }

    if let Some(v) =
        limits_value.get("referenceSearchDeadlineMs").and_then(serde_json::Value::as_u64)
    {
        limits.set_reference_search_deadline_ms(v);
    }
}

#[cfg(test)]
mod tests {
    use super::{LimitsSettings, apply_from_lsp_settings};

    #[derive(Default)]
    struct TestLimits {
        workspace_symbol_cap: usize,
        references_cap: usize,
        completion_cap: usize,
        ast_cache_max_entries: usize,
        max_indexed_files: usize,
        max_total_symbols: usize,
        workspace_scan_deadline_ms: u64,
        reference_search_deadline_ms: u64,
    }

    impl LimitsSettings for TestLimits {
        fn set_workspace_symbol_cap(&mut self, value: usize) {
            self.workspace_symbol_cap = value;
        }

        fn set_references_cap(&mut self, value: usize) {
            self.references_cap = value;
        }

        fn set_completion_cap(&mut self, value: usize) {
            self.completion_cap = value;
        }

        fn set_ast_cache_max_entries(&mut self, value: usize) {
            self.ast_cache_max_entries = value;
        }

        fn set_max_indexed_files(&mut self, value: usize) {
            self.max_indexed_files = value;
        }

        fn set_max_total_symbols(&mut self, value: usize) {
            self.max_total_symbols = value;
        }

        fn set_workspace_scan_deadline_ms(&mut self, value: u64) {
            self.workspace_scan_deadline_ms = value;
        }

        fn set_reference_search_deadline_ms(&mut self, value: u64) {
            self.reference_search_deadline_ms = value;
        }
    }

    #[test]
    fn applies_known_limit_fields() {
        let mut limits = TestLimits::default();
        let settings = serde_json::json!({
            "limits": {
                "workspaceSymbolCap": 300,
                "referencesCap": 700,
                "completionCap": 150,
                "astCacheMaxEntries": 80,
                "maxIndexedFiles": 20000,
                "maxTotalSymbols": 600000,
                "workspaceScanDeadlineMs": 45000,
                "referenceSearchDeadlineMs": 2500
            }
        });

        apply_from_lsp_settings(&mut limits, &settings);

        assert_eq!(limits.workspace_symbol_cap, 300);
        assert_eq!(limits.references_cap, 700);
        assert_eq!(limits.completion_cap, 150);
        assert_eq!(limits.ast_cache_max_entries, 80);
        assert_eq!(limits.max_indexed_files, 20_000);
        assert_eq!(limits.max_total_symbols, 600_000);
        assert_eq!(limits.workspace_scan_deadline_ms, 45_000);
        assert_eq!(limits.reference_search_deadline_ms, 2_500);
    }

    #[test]
    fn ignores_missing_limits_section() {
        let mut limits = TestLimits::default();
        let settings = serde_json::json!({ "editor": { "tabSize": 4 } });

        apply_from_lsp_settings(&mut limits, &settings);

        assert_eq!(limits.workspace_symbol_cap, 0);
        assert_eq!(limits.reference_search_deadline_ms, 0);
    }
}
