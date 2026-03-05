//! Parse `perl.limits` settings JSON into typed limit overrides.

use std::time::Duration;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LspLimitOverrides {
    pub workspace_symbol_cap: Option<usize>,
    pub references_cap: Option<usize>,
    pub completion_cap: Option<usize>,
    pub ast_cache_max_entries: Option<usize>,
    pub max_indexed_files: Option<usize>,
    pub max_total_symbols: Option<usize>,
    pub workspace_scan_deadline: Option<Duration>,
    pub reference_search_deadline: Option<Duration>,
}

impl LspLimitOverrides {
    #[must_use]
    pub fn from_settings(settings: &serde_json::Value) -> Self {
        let Some(limits) = settings.get("limits") else {
            return Self::default();
        };

        Self {
            workspace_symbol_cap: limits
                .get("workspaceSymbolCap")
                .and_then(serde_json::Value::as_u64)
                .map(|value| value as usize),
            references_cap: limits
                .get("referencesCap")
                .and_then(serde_json::Value::as_u64)
                .map(|value| value as usize),
            completion_cap: limits
                .get("completionCap")
                .and_then(serde_json::Value::as_u64)
                .map(|value| value as usize),
            ast_cache_max_entries: limits
                .get("astCacheMaxEntries")
                .and_then(serde_json::Value::as_u64)
                .map(|value| value as usize),
            max_indexed_files: limits
                .get("maxIndexedFiles")
                .and_then(serde_json::Value::as_u64)
                .map(|value| value as usize),
            max_total_symbols: limits
                .get("maxTotalSymbols")
                .and_then(serde_json::Value::as_u64)
                .map(|value| value as usize),
            workspace_scan_deadline: limits
                .get("workspaceScanDeadlineMs")
                .and_then(serde_json::Value::as_u64)
                .map(Duration::from_millis),
            reference_search_deadline: limits
                .get("referenceSearchDeadlineMs")
                .and_then(serde_json::Value::as_u64)
                .map(Duration::from_millis),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LspLimitOverrides;
    use std::time::Duration;

    #[test]
    fn parses_known_limit_fields() {
        let settings = serde_json::json!({
            "limits": {
                "workspaceSymbolCap": 275,
                "referencesCap": 800,
                "completionCap": 95,
                "astCacheMaxEntries": 55,
                "maxIndexedFiles": 30000,
                "maxTotalSymbols": 750000,
                "workspaceScanDeadlineMs": 90000,
                "referenceSearchDeadlineMs": 2500
            }
        });

        let overrides = LspLimitOverrides::from_settings(&settings);

        assert_eq!(overrides.workspace_symbol_cap, Some(275));
        assert_eq!(overrides.references_cap, Some(800));
        assert_eq!(overrides.completion_cap, Some(95));
        assert_eq!(overrides.ast_cache_max_entries, Some(55));
        assert_eq!(overrides.max_indexed_files, Some(30_000));
        assert_eq!(overrides.max_total_symbols, Some(750_000));
        assert_eq!(overrides.workspace_scan_deadline, Some(Duration::from_millis(90_000)));
        assert_eq!(overrides.reference_search_deadline, Some(Duration::from_millis(2_500)));
    }

    #[test]
    fn ignores_missing_limits_section() {
        let settings = serde_json::json!({"other": {"enabled": true}});
        let overrides = LspLimitOverrides::from_settings(&settings);
        assert_eq!(overrides, LspLimitOverrides::default());
    }

    #[test]
    fn ignores_invalid_types() {
        let settings = serde_json::json!({
            "limits": {
                "workspaceSymbolCap": "bad",
                "referenceSearchDeadlineMs": false
            }
        });

        let overrides = LspLimitOverrides::from_settings(&settings);
        assert!(overrides.workspace_symbol_cap.is_none());
        assert!(overrides.reference_search_deadline.is_none());
    }
}
