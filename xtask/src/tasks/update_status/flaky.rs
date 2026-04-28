//! Flaky-test registry reader and formatter.
//!
//! Reads `.ci/flaky-tests.json` and surfaces active/resolved counts for quality.md.

use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Active/resolved counts from `.ci/flaky-tests.json`.
#[derive(Debug, Default)]
pub(super) struct FlakyTestSummary {
    pub active: usize,
    pub resolved: usize,
}

// ---------------------------------------------------------------------------
// Collectors
// ---------------------------------------------------------------------------

/// Read `.ci/flaky-tests.json` and return active/resolved counts.
/// Returns a zero-filled summary if the file is absent or unparseable.
pub(super) fn collect_flaky_test_summary(root: &Path) -> FlakyTestSummary {
    let path = root.join(".ci/flaky-tests.json");
    let Ok(raw) = fs::read_to_string(&path) else {
        return FlakyTestSummary::default();
    };
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return FlakyTestSummary::default();
    };
    let active = doc.pointer("/summary/active").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let resolved = doc.pointer("/summary/resolved").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    FlakyTestSummary { active, resolved }
}

// ---------------------------------------------------------------------------
// Formatters
// ---------------------------------------------------------------------------

/// Format the flaky-tests summary as a markdown table with a sourcing note.
pub(super) fn format_flaky_tests_section(summary: &FlakyTestSummary) -> String {
    format!(
        "| State | Count |\n\
         |-------|-------|\n\
         | Active | {} |\n\
         | Resolved | {} |\n\
         \n\
         _Sourced from `.ci/flaky-tests.json`. Run `just status-update --only quality` to refresh._",
        summary.active, summary.resolved
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::{Context, Result};

    #[test]
    fn test_collect_flaky_test_summary_reads_file() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let ci_dir = dir.path().join(".ci");
        fs::create_dir_all(&ci_dir)?;
        let json = r#"{
            "schema_version": 1,
            "entries": [],
            "summary": { "total": 3, "active": 1, "resolved": 2, "by_subsystem": {} }
        }"#;
        fs::write(ci_dir.join("flaky-tests.json"), json)?;
        let summary = collect_flaky_test_summary(dir.path());
        assert_eq!(summary.active, 1);
        assert_eq!(summary.resolved, 2);
        Ok(())
    }

    #[test]
    fn test_collect_flaky_test_summary_missing_file_returns_zeros() {
        let dir = tempfile::tempdir().expect("tempdir");
        let summary = collect_flaky_test_summary(dir.path());
        assert_eq!(summary.active, 0);
        assert_eq!(summary.resolved, 0);
    }

    #[test]
    fn test_format_flaky_tests_section_contains_counts() {
        let summary = FlakyTestSummary { active: 2, resolved: 5 };
        let section = format_flaky_tests_section(&summary);
        assert!(section.contains("| Active | 2 |"), "missing active count row");
        assert!(section.contains("| Resolved | 5 |"), "missing resolved count row");
        assert!(section.contains("flaky-tests.json"), "missing source attribution");
    }

    #[test]
    fn test_flaky_tests_json_exists_and_is_valid() -> Result<()> {
        let root = crate::utils::project_root()?;
        let path = root.join(".ci/flaky-tests.json");
        assert!(path.exists(), ".ci/flaky-tests.json must exist");
        let raw = fs::read_to_string(&path).context("reading .ci/flaky-tests.json")?;
        let doc: serde_json::Value =
            serde_json::from_str(&raw).context("parsing .ci/flaky-tests.json")?;
        assert_eq!(doc["schema_version"], 1, "schema_version must be 1");
        assert!(doc["entries"].is_array(), "entries must be an array");
        assert!(doc["summary"].is_object(), "summary must be an object");
        Ok(())
    }
}
