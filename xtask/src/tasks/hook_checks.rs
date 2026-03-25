use color_eyre::eyre::{Context, ContextCompat, Result, bail};
use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::utils::project_root;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn run_hook_check() -> Result<()> {
    let root = project_root()?;
    let hooks_dir = root.join(".claude/hooks");

    if !hooks_dir.exists() {
        println!("Hook executable check passed");
        return Ok(());
    }

    let mut failed = 0u32;

    for entry in fs::read_dir(&hooks_dir).context("Failed to read .claude/hooks")? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("sh") {
            continue;
        }

        if !path.is_file() {
            continue;
        }

        if !is_executable(&path)? {
            println!("::error::Hook not executable: {}", path.display());
            failed += 1;
        }
    }

    if failed == 0 {
        println!("Hook executable check passed");
        Ok(())
    } else {
        bail!("Hook executable check failed for {failed} file(s)");
    }
}

pub fn run_hook_registry_check() -> Result<()> {
    let root = project_root()?;
    let settings_path = root.join(".claude/settings.json");
    let settings = fs::read_to_string(&settings_path)
        .with_context(|| format!("Failed to read {}", settings_path.display()))?;

    let document: Value = serde_json::from_str(&settings)
        .with_context(|| format!("Failed to parse {}", settings_path.display()))?;

    let commands = extract_hook_commands(&document);
    if commands.is_empty() {
        println!(
            "No .sh hook scripts registered in {} -- nothing to check",
            settings_path.display()
        );
        return Ok(());
    }

    let mut failed = 0u32;

    for path in &commands {
        let abs_path = root.join(&path);
        if !abs_path.exists() {
            println!("::error::Registered hook script missing: {}", path);
            failed += 1;
            continue;
        }

        if !is_executable(&abs_path)? {
            println!("::error::Registered hook script not executable: {}", path);
            failed += 1;
            continue;
        }

        println!("  OK: {}", path);
    }

    if failed == 0 {
        println!("Hook registry check passed ({} scripts verified)", commands.len());
        Ok(())
    } else {
        bail!("Hook registry check failed for {failed} script(s)");
    }
}

pub fn run_hook_tests() -> Result<()> {
    let root = project_root()?;
    let hooks_dir = root.join(".claude/hooks");

    let task_completed = hooks_dir.join("task-completed.sh");
    let subagent_stop = hooks_dir.join("subagent-stop.sh");
    let pre_tool_use = hooks_dir.join("pre-tool-use.sh");

    for path in [&task_completed, &subagent_stop, &pre_tool_use] {
        if !path.exists() {
            bail!("Required hook script missing: {}", path.display());
        }

        if !is_executable(path)? {
            bail!("Hook script not executable: {}", path.display());
        }
    }

    let ts_re = Regex::new(r#""ts":"[0-9]{4}-"#)?;

    let mut pass = 0u32;
    let mut fail = 0u32;

    let task_completed_no_payload = run_script(task_completed.as_path(), None, None)?;
    assert_exit_code(
        0,
        "task-completed passes with no staged .rs files",
        task_completed_no_payload.status.code().unwrap_or(-1),
        &mut pass,
        &mut fail,
    );

    let temp_root = PathBuf::from(std::env::temp_dir()).join(format!(
        "xtask-hook-tests-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    fs::create_dir_all(&temp_root).context("Failed to create temporary ops directory")?;

    let sample_payload =
        r#"{"subagent_name":"test-agent","subagent_type":"builder","session_id":"abc123"}"#;
    let temp_ops = temp_root.join("subagent-stop");
    fs::create_dir_all(&temp_ops).context("Failed to create temporary OPS_DIR")?;
    let subagent_out = run_script(&subagent_stop, Some(sample_payload), Some(temp_ops.as_path()))?;
    assert_exit_code(
        0,
        "subagent-stop exits 0 with payload",
        subagent_out.status.code().unwrap_or(-1),
        &mut pass,
        &mut fail,
    );

    let output = read_file(temp_ops.join("swarm-metrics.jsonl"), "Subagent-stop output file")?;
    assert_contains(
        &output,
        r#""event":"subagent_stop""#,
        "subagent-stop writes subagent_stop event",
        &mut pass,
        &mut fail,
    );
    assert_contains(
        &output,
        r#""agent_name":"test-agent""#,
        "subagent-stop writes agent_name",
        &mut pass,
        &mut fail,
    );
    assert_regex(&output, &ts_re, "subagent-stop includes ts timestamp", &mut pass, &mut fail);

    let temp_ops = temp_root.join("task-completed-write");
    fs::create_dir_all(&temp_ops).context("Failed to create temporary OPS_DIR")?;
    let sample_payload_tc = r#"{"session_id":"abc123","cwd":"/repo/worktrees/agent-xyz"}"#;
    let task_completed_with_payload =
        run_script(&task_completed, Some(sample_payload_tc), Some(temp_ops.as_path()))?;
    assert_exit_code(
        0,
        "task-completed exits 0 with metrics payload",
        task_completed_with_payload.status.code().unwrap_or(-1),
        &mut pass,
        &mut fail,
    );

    let output = read_file(temp_ops.join("swarm-metrics.jsonl"), "task-completed metrics file")?;
    assert_contains(
        &output,
        r#""event":"task_completed""#,
        "task-completed writes task_completed event",
        &mut pass,
        &mut fail,
    );
    assert_contains(
        &output,
        r#""session_id":"abc123""#,
        "task-completed captures session_id",
        &mut pass,
        &mut fail,
    );

    let temp_ops = temp_root.join("task-completed-empty");
    fs::create_dir_all(&temp_ops).context("Failed to create temporary OPS_DIR")?;
    let _ = run_script(&task_completed, Some("{}"), Some(temp_ops.as_path()))?;

    let safe_payload = r#"{"tool_input":{"command":"git status"}}"#;
    let pre_tool_safe = run_script(&pre_tool_use, Some(safe_payload), None)?;
    assert_exit_code(
        0,
        "pre-tool-use allows safe commands",
        pre_tool_safe.status.code().unwrap_or(-1),
        &mut pass,
        &mut fail,
    );

    let forced_payload = r#"{"tool_input":{"command":"git push --force"}}"#;
    let pre_tool_forced = run_script(&pre_tool_use, Some(forced_payload), None)?;
    assert_exit_code(
        2,
        "pre-tool-use blocks git push --force",
        pre_tool_forced.status.code().unwrap_or(-1),
        &mut pass,
        &mut fail,
    );

    let reset_payload = r#"{"tool_input":{"command":"git reset --hard"}}"#;
    let pre_tool_reset = run_script(&pre_tool_use, Some(reset_payload), None)?;
    assert_exit_code(
        2,
        "pre-tool-use blocks git reset --hard",
        pre_tool_reset.status.code().unwrap_or(-1),
        &mut pass,
        &mut fail,
    );

    let empty_payload = r#"{"tool_input":{}}"#;
    let pre_tool_empty = run_script(&pre_tool_use, Some(empty_payload), None)?;
    assert_exit_code(
        0,
        "pre-tool-use allows empty command",
        pre_tool_empty.status.code().unwrap_or(-1),
        &mut pass,
        &mut fail,
    );

    let temp_ops = temp_root.join("subagent-stop-cwd");
    fs::create_dir_all(&temp_ops).context("Failed to create temporary OPS_DIR")?;
    let payload_with_cwd =
        r#"{"subagent_type":"builder","cwd":"/repo/worktrees/agent-abc","session_id":"sess1"}"#;
    let subagent_out =
        run_script(&subagent_stop, Some(payload_with_cwd), Some(temp_ops.as_path()))?;
    assert_exit_code(
        0,
        "subagent-stop exits 0 with cwd payload",
        subagent_out.status.code().unwrap_or(-1),
        &mut pass,
        &mut fail,
    );

    if pass > 0 || fail > 0 {
        println!("\n=== Results: {} passed, {} failed ===", pass, fail);
    }

    if fail > 0 {
        bail!("hook tests failed");
    }

    // best effort cleanup
    let _ = fs::remove_dir_all(&temp_root);

    Ok(())
}

fn run_script(
    path: &Path,
    input: Option<&str>,
    ops_dir: Option<&Path>,
) -> Result<std::process::Output> {
    let mut command = Command::new("bash");
    command.arg(path);
    if let Some(dir) = ops_dir {
        command.env("OPS_DIR", dir);
    }

    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    if input.is_some() {
        command.stdin(Stdio::piped());
    }

    let mut child = command.spawn().with_context(|| format!("Failed to run {}", path.display()))?;

    if let Some(input) = input {
        let stdin = child.stdin.as_mut().context("Failed to open stdin for script")?;
        stdin.write_all(input.as_bytes()).context("Failed to write hook input")?;
    }

    let output = child.wait_with_output().context("Failed to read script output")?;
    Ok(output)
}

fn is_executable(path: &Path) -> Result<bool> {
    let metadata = path.metadata().context("Failed to read script metadata")?;
    if metadata.is_dir() {
        return Ok(false);
    }

    #[cfg(unix)]
    {
        Ok(metadata.permissions().mode() & 0o111 != 0)
    }

    #[cfg(not(unix))]
    {
        Ok(true)
    }
}

fn extract_hook_commands(document: &Value) -> Vec<String> {
    let mut commands = HashSet::new();
    if let Some(root_hooks) = document.get("hooks").and_then(Value::as_object) {
        for value in root_hooks.values() {
            if let Some(entries) = value.as_array() {
                for entry in entries {
                    collect_commands(entry, &mut commands);
                    if let Some(hooks) = entry.get("hooks").and_then(Value::as_array) {
                        for hook in hooks {
                            collect_commands(hook, &mut commands);
                        }
                    }
                }
            }
        }
    }

    let mut out: Vec<String> =
        commands.into_iter().filter(|command| command.ends_with(".sh")).collect();
    out.sort_unstable();
    out
}

fn collect_commands(document: &Value, out: &mut HashSet<String>) {
    if let Some(command) = document.get("command").and_then(Value::as_str) {
        if command.ends_with(".sh") {
            out.insert(normalize_hook_path(command));
        }
    }

    if let Some(map) = document.get("hooks").and_then(Value::as_object) {
        for value in map.values() {
            collect_commands(value, out);
        }
    }

    if let Some(array) = document.get("hooks").and_then(Value::as_array) {
        for value in array {
            collect_commands(value, out);
        }
    }
}

fn normalize_hook_path(value: &str) -> String {
    let mut normalized = value.replace("\"$CLAUDE_PROJECT_DIR\"/", "");
    normalized = normalized.replace("$CLAUDE_PROJECT_DIR/", "");
    normalized.trim_matches('"').trim_matches('\\').trim().to_string()
}

fn read_file(path: PathBuf, desc: &str) -> Result<String> {
    if !path.exists() {
        bail!("{desc} not found: {}", path.display());
    }

    fs::read_to_string(&path).with_context(|| format!("Failed to read {desc}: {}", path.display()))
}

fn assert_exit_code(expected: i32, desc: &str, actual: i32, pass: &mut u32, fail: &mut u32) {
    if actual == expected {
        println!("  PASS: {desc} (exit {actual})");
        *pass += 1;
    } else {
        eprintln!("  FAIL: {desc} - expected exit {expected}, got {actual}");
        *fail += 1;
    }
}

fn assert_contains(content: &str, pattern: &str, desc: &str, pass: &mut u32, fail: &mut u32) {
    if content.contains(pattern) {
        println!("  PASS: {desc}");
        *pass += 1;
    } else {
        eprintln!("  FAIL: {desc} - pattern '{pattern}' not found");
        *fail += 1;
    }
}

fn assert_regex(content: &str, pattern: &Regex, desc: &str, pass: &mut u32, fail: &mut u32) {
    if pattern.is_match(content) {
        println!("  PASS: {desc}");
        *pass += 1;
    } else {
        eprintln!("  FAIL: {desc} - pattern '{pattern}' not found");
        *fail += 1;
    }
}
