use color_eyre::eyre::{Context, Result, bail};
use regex::Regex;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use crate::utils::project_root;

const FROM_RAW_PATTERN: &str = r"\b([A-Za-z_][A-Za-z0-9_:]*::)?ExitStatus::from_raw\(";
const ALLOWED_FROM_RAW_PATTERN: &str = r"::from_raw\(\s*raw[_ ]?exit\s*\(";
const SEARCH_ROOTS: &[&str] = &["crates", "xtask", "examples", "tests"];

struct MemoryLifecycleInputs {
    text_sync: String,
    workspace: String,
    runtime_mod: String,
    streaming_tests: String,
    memory_status: String,
    receipt_registry: String,
    memory_receipt_schema: String,
}

fn source_fragment(line: &str) -> &str {
    line.splitn(3, ':').nth(2).unwrap_or(line)
}

fn is_comment_line(fragment: &str) -> bool {
    let trimmed = fragment.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*')
}

fn match_inside_double_quotes(fragment: &str, match_start: usize) -> bool {
    let mut in_string = false;
    let mut escaped = false;

    for ch in fragment[..match_start].chars() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            _ => {}
        }
    }

    in_string
}

fn is_disallowed_from_raw_line(line: &str, disallow_re: &Regex, allowed_re: &Regex) -> bool {
    let fragment = source_fragment(line);
    if is_comment_line(fragment) || allowed_re.is_match(fragment) {
        return false;
    }

    let Some(mat) = disallow_re.find(fragment) else {
        return false;
    };

    !match_inside_double_quotes(fragment, mat.start())
}

fn should_skip_dir(path: &Path) -> bool {
    path.file_name().is_some_and(|name| matches!(name.to_str(), Some("target" | "generated")))
}

fn collect_candidate_lines(root: &Path, disallow_re: &Regex) -> Result<Vec<String>> {
    let mut candidates = Vec::new();

    for relative_root in SEARCH_ROOTS {
        let search_root = root.join(relative_root);
        if !search_root.exists() {
            continue;
        }

        for entry in WalkDir::new(&search_root)
            .into_iter()
            .filter_entry(|entry| !(entry.file_type().is_dir() && should_skip_dir(entry.path())))
        {
            let entry =
                entry.with_context(|| format!("failed to walk {}", search_root.display()))?;
            if !entry.file_type().is_file()
                || entry.path().extension().is_none_or(|ext| ext != "rs")
            {
                continue;
            }

            let contents = fs::read_to_string(entry.path())
                .with_context(|| format!("failed to read {}", entry.path().display()))?;
            let relative_path = entry.path().strip_prefix(root).unwrap_or(entry.path());

            for (line_number, line) in contents.lines().enumerate() {
                if disallow_re.is_match(line) {
                    candidates.push(format!(
                        "{}:{}:{}",
                        relative_path.display(),
                        line_number + 1,
                        line
                    ));
                }
            }
        }
    }

    Ok(candidates)
}

pub fn check_from_raw() -> Result<()> {
    let root = project_root()?;
    let disallow_re = Regex::new(FROM_RAW_PATTERN)?;
    let allowed_re = Regex::new(ALLOWED_FROM_RAW_PATTERN)?;
    let candidates = collect_candidate_lines(&root, &disallow_re)?;

    let violations: Vec<_> = candidates
        .iter()
        .map(String::as_str)
        .filter(|line| is_disallowed_from_raw_line(line, &disallow_re, &allowed_re))
        .collect();

    if violations.is_empty() {
        println!("ExitStatus policy check passed");
        return Ok(());
    }

    for line in violations {
        eprintln!("::error::Disallowed direct from_raw(): {line}");
    }

    bail!("CI policy check found disallowed ExitStatus::from_raw() usage");
}

fn function_body<'a>(contents: &'a str, fn_name: &str) -> Option<&'a str> {
    let fn_pos = contents.find(&format!("fn {fn_name}"))?;
    let body_start = contents[fn_pos..].find('{')? + fn_pos;
    let mut depth = 0usize;

    for (offset, ch) in contents[body_start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let body_end = body_start + offset + ch.len_utf8();
                    return Some(&contents[body_start..body_end]);
                }
            }
            _ => {}
        }
    }

    None
}

fn memory_lifecycle_violations(inputs: &MemoryLifecycleInputs) -> Vec<String> {
    let mut violations = Vec::new();

    match function_body(&inputs.text_sync, "handle_did_close") {
        Some(body) => {
            if !body.contains("evict_open_document_session_state(uri)") {
                violations.push(
                    "textDocument/didClose must call evict_open_document_session_state(uri)"
                        .to_string(),
                );
            }
            if body.contains("evict_deleted_file_state") {
                violations
                    .push("textDocument/didClose must not call deleted-file eviction".to_string());
            }
        }
        None => violations.push("could not find handle_did_close body".to_string()),
    }

    match function_body(&inputs.text_sync, "handle_did_change_with_cancellation") {
        Some(body) => {
            if !body.contains("for key in self.uri_key_variants(uri)") {
                violations.push(
                    "didChange stream-session cancellation must sweep URI variants".to_string(),
                );
            }
            if body.contains("cancel_for_uri_version(uri,") || body.contains("cancel_for_uri(uri)")
            {
                violations.push(
                    "didChange must not cancel stream sessions using only the raw URI".to_string(),
                );
            }
        }
        None => {
            violations.push("could not find handle_did_change_with_cancellation body".to_string())
        }
    }

    let stale_index_guard_count =
        inputs.text_sync.matches("Skipping stale background index task").count();
    if stale_index_guard_count < 2 {
        violations.push(
            "didOpen and didChange background index tasks must keep stale-generation guards"
                .to_string(),
        );
    }
    if !inputs.text_sync.contains("generation.load(Ordering::Acquire) != 0") {
        violations.push(
            "didOpen background index task must validate the document generation before indexing"
                .to_string(),
        );
    }
    if !inputs.text_sync.contains("generation.load(Ordering::Acquire) != expected_generation") {
        violations.push(
            "didChange background index task must validate the expected document generation before indexing"
                .to_string(),
        );
    }
    if !inputs.text_sync.contains("test_did_close_after_change_storm_drains_background_index_tasks")
    {
        violations.push(
            "close-after-change-storm background index regression must stay present".to_string(),
        );
    }

    if !inputs.workspace.contains("FileChangeType::DELETED") {
        violations.push("watched-file delete branch must stay explicit".to_string());
    }
    if !inputs.workspace.contains("self.evict_deleted_file_state(&uri)")
        || !inputs.workspace.contains("self.evict_deleted_file_state(uri)")
    {
        violations.push(
            "watched-file and explicit delete paths must use deleted-file eviction".to_string(),
        );
    }

    for field in ["stream_sessions", "pending_index_tasks", "parse_cancel_flags"] {
        if !inputs.runtime_mod.contains(&format!("pub {field}: usize")) {
            violations.push(format!("MemoryStateSnapshot must retain {field} counter"));
        }
    }
    for field in [
        "file_watcher_pending_uris",
        "diagnostic_debounce_pending_uris",
        "pending_workspace_configuration_requests",
        "refresh_debounce_active",
        "active_stream_sessions",
    ] {
        if !inputs.runtime_mod.contains(&format!("pub {field}: usize")) {
            violations.push(format!("RuntimePressureSnapshot must retain {field} counter"));
        }
    }

    if !inputs.streaming_tests.contains("completion_stream_cancel_storm_keeps_one_live_session") {
        violations.push(
            "streaming completion cancel-storm memory regression must stay present".to_string(),
        );
    }

    for rule in [
        "Close-only churn may retain workspace-index entries",
        "Close+delete churn must remove file-backed workspace-index entries",
        "tail growth and median tail slope",
    ] {
        if !inputs.memory_status.contains(rule) {
            violations.push(format!("memory plateau status must document rule: {rule}"));
        }
    }

    if !inputs.receipt_registry.contains("check = \"memory-plateau\"")
        || !inputs
            .receipt_registry
            .contains("schema = \".ci/receipts/schemas/memory-plateau.schema.json\"")
    {
        violations.push("memory plateau receipt must stay registered".to_string());
    }

    for field in [
        "\"check\"",
        "\"scenario\"",
        "\"files\"",
        "\"changes_per_file\"",
        "\"tail_growth_kb\"",
        "\"median_tail_slope_kb_per_file\"",
        "\"passed\"",
    ] {
        if !inputs.memory_receipt_schema.contains(field) {
            violations.push(format!("memory plateau receipt schema must require {field}"));
        }
    }
    if !inputs.memory_receipt_schema.contains("\"check\": { \"const\": \"memory-plateau\" }") {
        violations.push("memory plateau schema must constrain check to memory-plateau".to_string());
    }

    violations
}

pub fn check_memory_lifecycle() -> Result<()> {
    let root = project_root()?;
    let read = |relative: &str| -> Result<String> {
        fs::read_to_string(root.join(relative))
            .with_context(|| format!("failed to read {relative}"))
    };

    let inputs = MemoryLifecycleInputs {
        text_sync: read("crates/perl-lsp-rs/src/runtime/text_sync.rs")?,
        workspace: read("crates/perl-lsp-rs/src/runtime/workspace.rs")?,
        runtime_mod: read("crates/perl-lsp-rs/src/runtime/mod.rs")?,
        streaming_tests: read("crates/perl-lsp-rs/tests/lsp_streaming_completion_tests.rs")?,
        memory_status: read("docs/project/status/memory_plateau.md")?,
        receipt_registry: read(".ci/receipts/registry.toml")?,
        memory_receipt_schema: read(".ci/receipts/schemas/memory-plateau.schema.json")?,
    };

    let violations = memory_lifecycle_violations(&inputs);
    if violations.is_empty() {
        println!("Memory lifecycle policy check passed");
        return Ok(());
    }

    for violation in violations {
        eprintln!("::error::{violation}");
    }

    bail!("memory lifecycle policy check failed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_doc_comment_mentions() {
        let disallow_re = Regex::new(FROM_RAW_PATTERN).expect("valid regex");
        let allowed_re = Regex::new(ALLOWED_FROM_RAW_PATTERN).expect("valid regex");
        let line = "xtask/src/main.rs:371:    /// Check for disallowed direct `ExitStatus::from_raw()` usage.";

        assert!(!is_disallowed_from_raw_line(line, &disallow_re, &allowed_re));
    }

    #[test]
    fn ignores_string_literal_mentions() {
        let disallow_re = Regex::new(FROM_RAW_PATTERN).expect("valid regex");
        let allowed_re = Regex::new(ALLOWED_FROM_RAW_PATTERN).expect("valid regex");
        let line = "xtask/src/tasks/ci_policy.rs:56:    bail!(\"CI policy check found disallowed ExitStatus::from_raw() usage\");";

        assert!(!is_disallowed_from_raw_line(line, &disallow_re, &allowed_re));
    }

    #[test]
    fn flags_real_from_raw_usage() {
        let disallow_re = Regex::new(FROM_RAW_PATTERN).expect("valid regex");
        let allowed_re = Regex::new(ALLOWED_FROM_RAW_PATTERN).expect("valid regex");
        let line = "src/lib.rs:10:    let status = std::process::ExitStatus::from_raw(raw_status);";

        assert!(is_disallowed_from_raw_line(line, &disallow_re, &allowed_re));
    }

    #[test]
    fn allows_raw_exit_adapter_usage() {
        let disallow_re = Regex::new(FROM_RAW_PATTERN).expect("valid regex");
        let allowed_re = Regex::new(ALLOWED_FROM_RAW_PATTERN).expect("valid regex");
        let line =
            "src/lib.rs:10:    let status = std::process::ExitStatus::from_raw(raw_exit(signal));";

        assert!(!is_disallowed_from_raw_line(line, &disallow_re, &allowed_re));
    }

    #[test]
    fn memory_lifecycle_policy_accepts_current_shape() {
        let inputs = MemoryLifecycleInputs {
            text_sync: r#"
                fn handle_did_change_with_cancellation(&self) {
                    for key in self.uri_key_variants(uri) {
                        self.stream_sessions().cancel_for_uri_version(&key, version);
                    }
                }
                fn handle_did_close(&self) {
                    self.evict_open_document_session_state(uri);
                }
                fn background_index_open() {
                    if generation.load(Ordering::Acquire) != 0 {
                        tracing::debug!("Skipping stale background index task");
                    }
                }
                fn background_index_change() {
                    if generation.load(Ordering::Acquire) != expected_generation {
                        tracing::debug!("Skipping stale background index task");
                    }
                }
                fn test_did_close_after_change_storm_drains_background_index_tasks() {}
            "#
            .to_string(),
            workspace: r#"
                match change_type {
                    FileChangeType::DELETED => self.evict_deleted_file_state(&uri),
                    _ => {}
                }
                self.evict_deleted_file_state(uri);
            "#
            .to_string(),
            runtime_mod: r#"
                pub struct MemoryStateSnapshot {
                    pub stream_sessions: usize,
                    pub pending_index_tasks: usize,
                    pub parse_cancel_flags: usize,
                }
                pub struct RuntimePressureSnapshot {
                    pub file_watcher_pending_uris: usize,
                    pub diagnostic_debounce_pending_uris: usize,
                    pub pending_workspace_configuration_requests: usize,
                    pub refresh_debounce_active: usize,
                    pub active_stream_sessions: usize,
                }
            "#
            .to_string(),
            streaming_tests: "fn completion_stream_cancel_storm_keeps_one_live_session() {}"
                .to_string(),
            memory_status: r#"
                Close-only churn may retain workspace-index entries.
                Close+delete churn must remove file-backed workspace-index entries.
                The plateau gate tracks tail growth and median tail slope.
            "#
            .to_string(),
            receipt_registry: r#"
                check = "memory-plateau"
                schema = ".ci/receipts/schemas/memory-plateau.schema.json"
            "#
            .to_string(),
            memory_receipt_schema: r#"
                {
                  "required": [
                    "check",
                    "scenario",
                    "files",
                    "changes_per_file",
                    "tail_growth_kb",
                    "median_tail_slope_kb_per_file",
                    "passed"
                  ],
                  "properties": {
                    "check": { "const": "memory-plateau" }
                  }
                }
            "#
            .to_string(),
        };

        assert!(memory_lifecycle_violations(&inputs).is_empty());
    }

    #[test]
    fn memory_lifecycle_policy_flags_close_delete_conflation() {
        let inputs = MemoryLifecycleInputs {
            text_sync: r#"
                fn handle_did_change_with_cancellation(&self) {
                    self.stream_sessions().cancel_for_uri(uri);
                }
                fn handle_did_close(&self) {
                    self.evict_deleted_file_state(uri);
                }
            "#
            .to_string(),
            workspace: String::new(),
            runtime_mod: String::new(),
            streaming_tests: String::new(),
            memory_status: String::new(),
            receipt_registry: String::new(),
            memory_receipt_schema: String::new(),
        };

        let violations = memory_lifecycle_violations(&inputs);
        assert!(violations.iter().any(|v| v.contains("didClose must not call")));
        assert!(violations.iter().any(|v| v.contains("raw URI")));
    }

    #[test]
    fn memory_lifecycle_policy_flags_missing_background_index_generation_guard() {
        let inputs = MemoryLifecycleInputs {
            text_sync: r#"
                fn handle_did_change_with_cancellation(&self) {
                    for key in self.uri_key_variants(uri) {
                        self.stream_sessions().cancel_for_uri_version(&key, version);
                    }
                }
                fn handle_did_close(&self) {
                    self.evict_open_document_session_state(uri);
                }
            "#
            .to_string(),
            workspace: String::new(),
            runtime_mod: String::new(),
            streaming_tests: String::new(),
            memory_status: String::new(),
            receipt_registry: String::new(),
            memory_receipt_schema: String::new(),
        };

        let violations = memory_lifecycle_violations(&inputs);
        assert!(
            violations.iter().any(|v| v.contains("stale-generation guards")),
            "expected stale-generation guard violation, got {violations:?}"
        );
        assert!(
            violations.iter().any(|v| v.contains("change-storm background index regression")),
            "expected regression-test presence violation, got {violations:?}"
        );
    }
}
