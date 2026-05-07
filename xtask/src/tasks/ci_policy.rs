use color_eyre::eyre::{Context, Result, bail};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

use crate::utils::project_root;

const FROM_RAW_PATTERN: &str = r"\b([A-Za-z_][A-Za-z0-9_:]*::)?ExitStatus::from_raw\(";
const ALLOWED_FROM_RAW_PATTERN: &str = r"::from_raw\(\s*raw[_ ]?exit\s*\(";
const SEARCH_ROOTS: &[&str] = &["crates", "xtask", "examples", "tests"];
const RETAINED_STATE_INVENTORY: &str = "docs/large-workspaces/RETAINED_STATE_INVENTORY.md";
const RETAINED_OWNER_PATTERN_LABELS: &[(&str, &str)] = &[
    ("Arc<Mutex<", "shared mutex state"),
    ("Arc<RwLock<", "shared rwlock state"),
    ("moka::Cache", "cache"),
    ("Cache::new", "cache"),
    ("DashMap", "concurrent map"),
    ("HashMap", "map"),
    ("BTreeMap", "map"),
    ("VecDeque", "queue"),
    ("JoinSet", "task set"),
    ("tokio::spawn", "spawned task"),
    ("mpsc::", "channel"),
    ("oneshot::", "channel"),
    ("broadcast::", "channel"),
    ("watch::", "channel"),
    ("Child", "child process handle"),
    ("tokio::process::Child", "child process handle"),
    ("std::process::Child", "child process handle"),
    ("Debouncer", "debouncer"),
    ("SessionManager", "session holder"),
    ("sessions:", "session holder"),
];
const MEMORY_SENSITIVE_RUST_PREFIXES: &[&str] = &[
    "crates/perl-lsp-rs/src/runtime/",
    "crates/perl-lsp-rs/src/runtime/language/",
    "crates/perl-workspace/src/workspace/",
    "crates/perl-lsp-perltidy/src/",
    "crates/perl-lsp-rs-core/src/tooling/",
    "crates/perl-dap/src/",
];

struct MemoryLifecycleInputs {
    text_sync: String,
    workspace: String,
    runtime_mod: String,
    streaming_tests: String,
    memory_status: String,
    retained_state_inventory: String,
    receipt_registry: String,
    memory_receipt_schema: String,
}

pub struct RetainedOwnerDriftConfig {
    pub base: String,
    pub report_only: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct RetainedOwnerFinding {
    path: String,
    line: String,
    pattern: &'static str,
    label: &'static str,
    sensitive_path: bool,
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

fn markdown_section<'a>(contents: &'a str, heading: &str) -> Option<&'a str> {
    let section_start = contents.find(heading)?;
    let after_heading = section_start + heading.len();
    let section_end = contents[after_heading..]
        .find("\n## ")
        .map(|offset| after_heading + offset)
        .unwrap_or(contents.len());
    Some(&contents[after_heading..section_end])
}

fn retained_state_inventory_violations(inventory: &str) -> Vec<String> {
    let mut violations = Vec::new();

    if !inventory.contains("- pressure counter or retained-process signal") {
        violations.push(
            "retained-state inventory must require a pressure counter or retained-process signal"
                .to_string(),
        );
    }
    if !inventory.contains(
        "| Owner | State | Key type | Byte-risk | Bounds and cleanup | Pressure counter or signal | Regression test or receipt |",
    ) {
        violations.push(
            "retained-state inventory table must include a pressure counter or signal column"
                .to_string(),
        );
    }
    if !inventory.contains("Is there a pressure counter, retained-process signal, or receipt?") {
        violations.push(
            "retained-state review checklist must ask for a pressure counter or signal".to_string(),
        );
    }

    let Some(current_inventory) = markdown_section(inventory, "## Current Inventory") else {
        violations
            .push("retained-state inventory must keep a Current Inventory section".to_string());
        return violations;
    };

    for (line_index, line) in current_inventory.lines().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|')
            || trimmed.contains("| Owner |")
            || trimmed.contains("|-------|")
        {
            continue;
        }

        let cells: Vec<_> = trimmed.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() != 7 {
            violations.push(format!(
                "retained-state inventory row {} must have 7 cells including pressure signal",
                line_index + 1
            ));
            continue;
        }

        let pressure_signal = cells[5];
        if pressure_signal.is_empty()
            || matches!(
                pressure_signal.to_ascii_lowercase().as_str(),
                "n/a" | "none" | "todo" | "tbd"
            )
        {
            violations.push(format!(
                "retained-state inventory row for {} must name a concrete pressure signal",
                cells[1]
            ));
        }
    }

    violations
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

    violations.extend(retained_state_inventory_violations(&inputs.retained_state_inventory));

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
        retained_state_inventory: read("docs/large-workspaces/RETAINED_STATE_INVENTORY.md")?,
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

fn resolve_diff_base(root: &Path, requested_base: &str) -> Result<String> {
    for candidate in [requested_base, "origin/master", "origin/main", "master", "main", "HEAD~1"] {
        let output = Command::new("git")
            .current_dir(root)
            .args(["rev-parse", "--verify", candidate])
            .output()
            .with_context(|| format!("failed to resolve git ref {candidate}"))?;
        if output.status.success() {
            return Ok(candidate.to_string());
        }
    }

    bail!("could not resolve a diff base for retained-owner drift check");
}

fn diff_name_only(root: &Path, base: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["diff", "--name-only", &format!("{base}...HEAD")])
        .output()
        .context("failed to run git diff --name-only for retained-owner drift check")?;

    if !output.status.success() {
        bail!("git diff --name-only failed for retained-owner drift check");
    }

    let stdout = String::from_utf8(output.stdout).context("git diff output was not UTF-8")?;
    Ok(stdout.lines().map(str::to_string).filter(|line| !line.is_empty()).collect())
}

fn diff_added_lines(root: &Path, base: &str) -> Result<BTreeMap<String, Vec<String>>> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["diff", "--unified=0", &format!("{base}...HEAD"), "--", "*.rs"])
        .output()
        .context("failed to run git diff for retained-owner drift check")?;

    if !output.status.success() {
        bail!("git diff failed for retained-owner drift check");
    }

    let stdout = String::from_utf8(output.stdout).context("git diff output was not UTF-8")?;
    Ok(parse_added_lines_by_file(&stdout))
}

fn parse_added_lines_by_file(diff: &str) -> BTreeMap<String, Vec<String>> {
    let mut current_file: Option<String> = None;
    let mut added_by_file: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            current_file = Some(path.to_string());
            continue;
        }

        if line.starts_with("+++") || !line.starts_with('+') {
            continue;
        }

        let Some(path) = current_file.as_ref() else {
            continue;
        };
        added_by_file.entry(path.clone()).or_default().push(line[1..].to_string());
    }

    added_by_file
}

fn line_contains_retained_owner_pattern(line: &str) -> Option<(&'static str, &'static str)> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
        return None;
    }

    RETAINED_OWNER_PATTERN_LABELS
        .iter()
        .find_map(|(pattern, label)| line.contains(pattern).then_some((*pattern, *label)))
}

fn is_memory_sensitive_path(path: &str) -> bool {
    MEMORY_SENSITIVE_RUST_PREFIXES.iter().any(|prefix| path.starts_with(prefix))
}

fn retained_owner_findings(
    changed_files: &[String],
    added_lines: &BTreeMap<String, Vec<String>>,
) -> Vec<RetainedOwnerFinding> {
    let changed: BTreeSet<_> = changed_files.iter().collect();
    added_lines
        .iter()
        .filter(|(path, _)| changed.contains(path) && is_memory_sensitive_path(path))
        .flat_map(|(path, lines)| {
            lines.iter().filter_map(move |line| {
                let (pattern, label) = line_contains_retained_owner_pattern(line)?;
                Some(RetainedOwnerFinding {
                    path: path.clone(),
                    line: line.trim().to_string(),
                    pattern,
                    label,
                    sensitive_path: is_memory_sensitive_path(path),
                })
            })
        })
        .collect()
}

fn inventory_was_changed(changed_files: &[String]) -> bool {
    changed_files.iter().any(|path| path == RETAINED_STATE_INVENTORY)
}

pub fn check_memory_retained_owner_drift(config: RetainedOwnerDriftConfig) -> Result<()> {
    let root = project_root()?;
    let base = resolve_diff_base(&root, &config.base)?;
    let changed_files = diff_name_only(&root, &base)?;
    let added_lines = diff_added_lines(&root, &base)?;
    let findings = retained_owner_findings(&changed_files, &added_lines);

    if findings.is_empty() {
        println!(
            "Memory retained-owner drift check passed: no new retained-owner patterns found in memory-sensitive Rust paths"
        );
        return Ok(());
    }

    if inventory_was_changed(&changed_files) {
        println!(
            "Memory retained-owner drift check passed: retained-owner patterns found and {RETAINED_STATE_INVENTORY} changed"
        );
        for finding in findings {
            println!(
                "::notice file={}::new {} pattern `{}` is covered by retained-state inventory changes",
                finding.path, finding.label, finding.pattern
            );
        }
        return Ok(());
    }

    for finding in &findings {
        let annotation = if finding.sensitive_path { "warning" } else { "notice" };
        println!(
            "::{annotation} file={}::new {} pattern `{}` without {RETAINED_STATE_INVENTORY}: {}",
            finding.path, finding.label, finding.pattern, finding.line
        );
    }

    let sensitive_count = findings.iter().filter(|finding| finding.sensitive_path).count();
    if !config.report_only && sensitive_count > 0 {
        bail!(
            "retained-owner drift check found {sensitive_count} memory-sensitive additions without {RETAINED_STATE_INVENTORY}"
        );
    }

    if config.report_only {
        println!(
            "Memory retained-owner drift check completed in report-only mode; update {RETAINED_STATE_INVENTORY} when the new storage/task owner is long-lived"
        );
    } else {
        println!(
            "Memory retained-owner drift check passed: only non-sensitive retained-owner patterns were added without {RETAINED_STATE_INVENTORY}"
        );
    }
    Ok(())
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
            retained_state_inventory: r#"
                - pressure counter or retained-process signal

                ## Current Inventory

                | Owner | State | Key type | Byte-risk | Bounds and cleanup | Pressure counter or signal | Regression test or receipt |
                |-------|-------|----------|-----------|--------------------|----------------------------|----------------------------|
                | `LspServer` | Open documents | URI | Source text | Close/delete cleanup | `MemoryStateSnapshot.documents` | close/delete test |

                ## Review Checklist

                - Is there a pressure counter, retained-process signal, or receipt?
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
            retained_state_inventory: String::new(),
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
            retained_state_inventory: String::new(),
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

    #[test]
    fn memory_lifecycle_policy_flags_missing_inventory_pressure_signal() {
        let inventory = r#"
            ## Current Inventory

            | Owner | State | Key type | Byte-risk | Bounds and cleanup | Regression test or receipt |
            |-------|-------|----------|-----------|--------------------|----------------------------|
            | `LspServer` | Open documents | URI | Source text | Close/delete cleanup | close/delete test |

            ## Review Checklist

            - Is there a regression test?
        "#;

        let violations = retained_state_inventory_violations(inventory);
        assert!(
            violations.iter().any(|v| v.contains("pressure counter or retained-process signal")),
            "expected missing pressure-signal requirement violation, got {violations:?}"
        );
        assert!(
            violations.iter().any(|v| v.contains("pressure counter or signal column")),
            "expected missing pressure-signal column violation, got {violations:?}"
        );
    }

    #[test]
    fn retained_owner_drift_detects_added_storage_patterns() {
        let diff = r#"
diff --git a/crates/perl-lsp-rs/src/runtime/example.rs b/crates/perl-lsp-rs/src/runtime/example.rs
index 0000000..1111111 100644
--- a/crates/perl-lsp-rs/src/runtime/example.rs
+++ b/crates/perl-lsp-rs/src/runtime/example.rs
@@ -1,0 +1,4 @@
+use std::collections::HashMap;
+let cache = Arc<Mutex<HashMap<String, String>>>;
+// HashMap in comments should not be counted after parsing added lines
+tokio::spawn(async move {});
"#;
        let changed_files = vec!["crates/perl-lsp-rs/src/runtime/example.rs".to_string()];
        let added_lines = parse_added_lines_by_file(diff);
        let findings = retained_owner_findings(&changed_files, &added_lines);

        assert!(
            findings.iter().any(|finding| finding.pattern == "HashMap"),
            "expected HashMap finding, got {findings:?}"
        );
        assert!(
            findings.iter().any(|finding| finding.pattern == "Arc<Mutex<"),
            "expected Arc<Mutex< finding, got {findings:?}"
        );
        assert!(
            findings.iter().any(|finding| finding.pattern == "tokio::spawn"),
            "expected tokio::spawn finding, got {findings:?}"
        );
        assert!(
            findings.iter().all(|finding| finding.sensitive_path),
            "runtime findings should be memory-sensitive: {findings:?}"
        );
        assert_eq!(
            findings.iter().filter(|finding| finding.line.contains("comments")).count(),
            0,
            "comment-only retained-owner mentions should not be findings"
        );
    }

    #[test]
    fn retained_owner_drift_notes_inventory_updates() {
        let changed_files = vec![
            "crates/perl-dap/src/session.rs".to_string(),
            RETAINED_STATE_INVENTORY.to_string(),
        ];

        assert!(inventory_was_changed(&changed_files));
        assert!(is_memory_sensitive_path("crates/perl-dap/src/session.rs"));
        assert!(!is_memory_sensitive_path("docs/large-workspaces/example.md"));
    }
}
