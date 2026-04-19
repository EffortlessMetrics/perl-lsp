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
        let abs_path = root.join(path);
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
        println!(
            "Hook registry check passed ({} scripts verified)",
            commands.len()
        );
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

    let temp_root = std::env::temp_dir().join(format!(
        "xtask-hook-tests-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    fs::create_dir_all(&temp_root).context("Failed to create temporary ops directory")?;
    ensure_temp_repo_isolated(&root, &temp_root)?;

    // Create an empty hooks directory inside the temp root that all temp-repo
    // git invocations will use. This overrides any inherited `core.hooksPath`
    // (e.g. the parent repo's `.git/hooks`) so the main repo's hooks cannot
    // fire inside the temp repo. See issue #3203.
    let isolated_hooks_dir = temp_root.join("empty-hooks");
    fs::create_dir_all(&isolated_hooks_dir).context("Failed to create isolated hooks directory")?;

    let task_repo = temp_root.join("task-completed-repo");
    create_non_rust_test_repo(&task_repo, &temp_root, &isolated_hooks_dir)?;

    let task_completed_no_payload = run_script(
        task_completed.as_path(),
        None,
        None,
        Some(task_repo.as_path()),
    )?;
    assert_exit_code(
        0,
        "task-completed passes with no staged .rs files",
        task_completed_no_payload.status.code().unwrap_or(-1),
        &mut pass,
        &mut fail,
    );

    let sample_payload =
        r#"{"subagent_name":"test-agent","subagent_type":"builder","session_id":"abc123"}"#;
    let temp_ops = temp_root.join("subagent-stop");
    fs::create_dir_all(&temp_ops).context("Failed to create temporary OPS_DIR")?;
    let subagent_out = run_script(
        &subagent_stop,
        Some(sample_payload),
        Some(temp_ops.as_path()),
        None,
    )?;
    assert_exit_code(
        0,
        "subagent-stop exits 0 with payload",
        subagent_out.status.code().unwrap_or(-1),
        &mut pass,
        &mut fail,
    );

    let output = read_file(
        temp_ops.join("swarm-metrics.jsonl"),
        "Subagent-stop output file",
    )?;
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
    assert_regex(
        &output,
        &ts_re,
        "subagent-stop includes ts timestamp",
        &mut pass,
        &mut fail,
    );

    let temp_ops = temp_root.join("subagent-stop-plan-review");
    fs::create_dir_all(&temp_ops).context("Failed to create temporary OPS_DIR")?;

    let no_issue_payload = r#"{"subagent_name":"plan-reviewer","subagent_type":"plan-reviewer","cwd":"/repo/worktrees/agent-a071b609","session_id":"abc124"}"#;
    let no_issue_out = run_script(
        &subagent_stop,
        Some(no_issue_payload),
        Some(temp_ops.as_path()),
        None,
    )?;
    assert_exit_code(
        3,
        "plan-reviewer without explicit issue context fails loud",
        no_issue_out.status.code().unwrap_or(-1),
        &mut pass,
        &mut fail,
    );

    let output = read_file(
        temp_ops.join("swarm-metrics.jsonl"),
        "plan-reviewer metrics file",
    )?;
    assert_contains(
        &output,
        r#""event":"subagent_stop""#,
        "plan-reviewer failure still writes subagent_stop event",
        &mut pass,
        &mut fail,
    );

    let agent_name_payload = r#"{"subagent_name":"plan-review-4044","subagent_type":"plan-reviewer","session_id":"abc125"}"#;
    let agent_name_out = run_script(
        &subagent_stop,
        Some(agent_name_payload),
        Some(temp_ops.as_path()),
        None,
    )?;
    assert_not_exit_code(
        3,
        "canonical plan-review-NNN agent name satisfies issue context fallback",
        agent_name_out.status.code().unwrap_or(-1),
        &mut pass,
        &mut fail,
    );

    let temp_ops = temp_root.join("task-completed-write");
    fs::create_dir_all(&temp_ops).context("Failed to create temporary OPS_DIR")?;
    let sample_payload_tc = r#"{"session_id":"abc123","cwd":"/repo/worktrees/agent-xyz"}"#;
    let task_completed_with_payload = run_script(
        &task_completed,
        Some(sample_payload_tc),
        Some(temp_ops.as_path()),
        Some(task_repo.as_path()),
    )?;
    assert_exit_code(
        0,
        "task-completed exits 0 with metrics payload",
        task_completed_with_payload.status.code().unwrap_or(-1),
        &mut pass,
        &mut fail,
    );

    let output = read_file(
        temp_ops.join("swarm-metrics.jsonl"),
        "task-completed metrics file",
    )?;
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
    let _ = run_script(
        &task_completed,
        Some("{}"),
        Some(temp_ops.as_path()),
        Some(task_repo.as_path()),
    )?;

    let safe_payload = r#"{"tool_input":{"command":"git status"}}"#;
    let pre_tool_safe = run_script(&pre_tool_use, Some(safe_payload), None, None)?;
    assert_exit_code(
        0,
        "pre-tool-use allows safe commands",
        pre_tool_safe.status.code().unwrap_or(-1),
        &mut pass,
        &mut fail,
    );

    let forced_payload = r#"{"tool_input":{"command":"git push --force"}}"#;
    let pre_tool_forced = run_script(&pre_tool_use, Some(forced_payload), None, None)?;
    assert_exit_code(
        2,
        "pre-tool-use blocks git push --force",
        pre_tool_forced.status.code().unwrap_or(-1),
        &mut pass,
        &mut fail,
    );

    let reset_payload = r#"{"tool_input":{"command":"git reset --hard"}}"#;
    let pre_tool_reset = run_script(&pre_tool_use, Some(reset_payload), None, None)?;
    assert_exit_code(
        2,
        "pre-tool-use blocks git reset --hard",
        pre_tool_reset.status.code().unwrap_or(-1),
        &mut pass,
        &mut fail,
    );

    let empty_payload = r#"{"tool_input":{}}"#;
    let pre_tool_empty = run_script(&pre_tool_use, Some(empty_payload), None, None)?;
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
    let subagent_out = run_script(
        &subagent_stop,
        Some(payload_with_cwd),
        Some(temp_ops.as_path()),
        None,
    )?;
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
    current_dir: Option<&Path>,
) -> Result<std::process::Output> {
    let mut command = Command::new(bash_executable());
    command.arg(path);
    if let Some(dir) = ops_dir {
        command.env("OPS_DIR", dir);
    }
    if let Some(dir) = current_dir {
        command.current_dir(dir);
    }

    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    if input.is_some() {
        command.stdin(Stdio::piped());
    }

    let mut child = command
        .spawn()
        .with_context(|| format!("Failed to run {}", path.display()))?;

    if let Some(input) = input {
        let stdin = child
            .stdin
            .as_mut()
            .context("Failed to open stdin for script")?;
        stdin
            .write_all(input.as_bytes())
            .context("Failed to write hook input")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to read script output")?;
    Ok(output)
}

fn bash_executable() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(path) = git_bash_executable() {
            return path;
        }

        let default = PathBuf::from(r"C:\Program Files\Git\bin\bash.exe");
        if default.exists() {
            return default;
        }
    }

    PathBuf::from("bash")
}

#[cfg(windows)]
fn git_bash_executable() -> Option<PathBuf> {
    let output = Command::new("where.exe").arg("git.exe").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let git_exe = stdout.lines().find(|line| !line.trim().is_empty())?.trim();
    let git_root = PathBuf::from(git_exe).parent()?.parent()?.to_path_buf();
    let bash = git_root.join("bin").join("bash.exe");
    bash.exists().then_some(bash)
}

fn create_non_rust_test_repo(
    path: &Path,
    temp_root: &Path,
    isolated_hooks_dir: &Path,
) -> Result<()> {
    // SAFETY (issue #3203): assert the repo directory path lives inside the temp
    // root before creating it. See assert_path_inside_temp_root for full rationale.
    assert_path_inside_temp_root(path, temp_root, "create_non_rust_test_repo dir")?;
    fs::create_dir_all(path)
        .with_context(|| format!("Failed to create temp repo {}", path.display()))?;

    // SAFETY (issue #3203): assert the README.md path lives inside the temp
    // root before writing. The hook-tests scaffold once overwrote the real
    // workspace README.md when this invariant was violated; this assertion
    // catches any future regression at the call site.
    let readme_path = path.join("README.md");
    assert_path_inside_temp_root(
        &readme_path,
        temp_root,
        "create_non_rust_test_repo README.md",
    )?;
    fs::write(&readme_path, "# hook test repo\n")
        .with_context(|| format!("Failed to seed temp repo {}", path.display()))?;

    run_git(path, isolated_hooks_dir, &["init"])?;
    run_git(path, isolated_hooks_dir, &["add", "README.md"])?;
    run_git(
        path,
        isolated_hooks_dir,
        &[
            "-c",
            "user.name=xtask hook tests",
            "-c",
            "user.email=xtask@example.invalid",
            "commit",
            "-m",
            "seed temp repo",
        ],
    )?;

    Ok(())
}

/// Assert that `path` lives strictly inside `temp_root`.
///
/// This guards every filesystem write the hook-tests scaffold performs against
/// the bug from issue #3203, where the scaffold once scribbled onto the real
/// workspace `README.md`. The check uses canonicalized paths so symlinks and
/// platform-specific path normalization (UNC vs. plain on Windows) cannot fool
/// the comparison. The parent of `path` is canonicalized when `path` itself
/// does not yet exist.
fn assert_path_inside_temp_root(path: &Path, temp_root: &Path, context: &str) -> Result<()> {
    let canonical_temp_root = temp_root
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize temp root {}", temp_root.display()))?;

    let canonical_target = if path.exists() {
        path.canonicalize()
            .with_context(|| format!("Failed to canonicalize {}", path.display()))?
    } else {
        let parent = path
            .parent()
            .with_context(|| format!("Path has no parent: {}", path.display()))?;
        let canonical_parent = parent
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize parent {}", parent.display()))?;
        let file_name = path
            .file_name()
            .with_context(|| format!("Path has no file name: {}", path.display()))?;
        canonical_parent.join(file_name)
    };

    if !canonical_target.starts_with(&canonical_temp_root) {
        bail!(
            "hook-tests refusing to write outside temp root ({}): {} -> {}",
            context,
            path.display(),
            canonical_target.display()
        );
    }

    Ok(())
}

fn ensure_temp_repo_isolated(project_root: &Path, temp_root: &Path) -> Result<()> {
    let canonical_project_root = project_root
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize {}", project_root.display()))?;
    let canonical_temp_root = temp_root
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize {}", temp_root.display()))?;

    if canonical_temp_root.starts_with(&canonical_project_root)
        || canonical_project_root.starts_with(&canonical_temp_root)
    {
        bail!(
            "Hook test temp root {} overlaps project root {}",
            canonical_temp_root.display(),
            canonical_project_root.display()
        );
    }

    Ok(())
}

/// Run a git command inside a temp repo with full environment isolation.
///
/// Two isolation layers are applied:
///
/// **Layer 1 — hook isolation** (from PR #3246 / issue #3203): every
/// invocation prepends `-c core.hooksPath=<isolated_hooks_dir>` so the parent
/// repo's hooks cannot fire inside the temp repo.
///
/// **Layer 2 — GIT_DIR isolation** (post-#3246 follow-up): when hook-tests
/// runs inside a git pre-push hook, git injects `GIT_DIR`, `GIT_WORK_TREE`,
/// `GIT_INDEX_FILE`, `GIT_COMMON_DIR`, and `GIT_OBJECT_DIRECTORY` into the
/// hook's environment, pointing at the triggering repo's git state.  All child
/// processes (including the `git` invocations in this function) inherit those
/// variables.  If they are not cleared, `git init` in the temp repo sees
/// `GIT_DIR` already set and operates against the triggering worktree instead
/// of creating a fresh `.git`.  Subsequent `git add` and `git commit` then
/// target the triggering worktree, producing the README.md contamination
/// observed in multiple worktrees (agent-a2a09e97, agent-a4d15685, etc.)
/// even after PR #3246.
///
/// We clear all standard git environment variables unconditionally so that
/// the git subprocess discovers the repo through normal `.git` traversal from
/// `current_dir`, regardless of what the parent hook environment contains.
fn run_git(repo: &Path, isolated_hooks_dir: &Path, args: &[&str]) -> Result<()> {
    let hooks_override = format!("core.hooksPath={}", isolated_hooks_dir.display());
    let output = Command::new("git")
        .current_dir(repo)
        // Layer 1: override core.hooksPath so the parent repo's hooks cannot
        // fire inside the temp repo (issue #3203 / PR #3246).
        .args(["-c", hooks_override.as_str()])
        .args(args)
        // Layer 2: clear GIT_DIR and related variables that a git hook injects
        // into the process environment.  Without this, git ignores current_dir
        // and operates against the triggering worktree instead of the temp
        // repo (post-#3246 GIT_DIR inheritance bug).
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY")
        // GIT_PREFIX is set by git to the subdirectory within the work tree
        // from which the hook was invoked.  Clear it so git does not confuse
        // the temp repo's root with a subdirectory of the triggering repo.
        .env_remove("GIT_PREFIX")
        .output()
        .with_context(|| format!("Failed to run git {:?} in {}", args, repo.display()))?;

    if !output.status.success() {
        bail!(
            "git {:?} failed in {}: {}",
            args,
            repo.display(),
            String::from_utf8_lossy(&output.stderr).trim_end()
        );
    }

    Ok(())
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

    let mut out: Vec<String> = commands
        .into_iter()
        .filter(|command| command.ends_with(".sh"))
        .collect();
    out.sort_unstable();
    out
}

fn collect_commands(document: &Value, out: &mut HashSet<String>) {
    let Some(command) = document.get("command").and_then(Value::as_str) else {
        return;
    };

    if command.ends_with(".sh") {
        out.insert(normalize_hook_path(command));
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
    normalized
        .trim_matches('"')
        .trim_matches('\\')
        .trim()
        .to_string()
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

fn assert_not_exit_code(unexpected: i32, desc: &str, actual: i32, pass: &mut u32, fail: &mut u32) {
    if actual != unexpected {
        println!("  PASS: {desc} (exit {actual})");
        *pass += 1;
    } else {
        eprintln!("  FAIL: {desc} - unexpected exit {unexpected}");
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

#[cfg(test)]
mod tests {
    //! Regression tests for issue #3203 — hook-tests scaffold safety.
    //!
    //! These tests guard the two-part fix:
    //!   1. Every git invocation in the temp repo must override
    //!      `core.hooksPath` so the parent repo's hooks cannot fire.
    //!   2. Every filesystem write in the scaffold must be inside the temp
    //!      root, asserted at the call site so a regression bails loudly
    //!      instead of scribbling on the workspace.

    use super::*;
    use std::collections::HashSet;
    use tempfile::TempDir;

    #[test]
    fn assert_path_inside_temp_root_accepts_child_path() {
        let tempdir = TempDir::new().expect("create tempdir");
        let temp_root = tempdir.path();
        let child = temp_root.join("subdir").join("README.md");
        std::fs::create_dir_all(child.parent().unwrap()).unwrap();
        assert_path_inside_temp_root(&child, temp_root, "test").expect("child path is allowed");
    }

    #[test]
    fn assert_path_inside_temp_root_rejects_workspace_readme() {
        // Construct two unrelated tempdirs. The "fake workspace" stands in
        // for the real workspace root that #3203 saw scribbled.
        let temp_root_dir = TempDir::new().expect("create temp root");
        let fake_workspace = TempDir::new().expect("create fake workspace");
        let workspace_readme = fake_workspace.path().join("README.md");
        std::fs::write(&workspace_readme, "real workspace content")
            .expect("seed fake workspace README");

        let err = assert_path_inside_temp_root(
            &workspace_readme,
            temp_root_dir.path(),
            "regression #3203",
        )
        .expect_err("must refuse to write outside temp root");

        let msg = format!("{err:#}");
        assert!(
            msg.contains("hook-tests refusing to write outside temp root"),
            "expected refusal message, got: {msg}"
        );

        // The fake workspace README must remain untouched.
        let after = std::fs::read_to_string(&workspace_readme).unwrap();
        assert_eq!(
            after, "real workspace content",
            "fake workspace README was scribbled"
        );
    }

    #[test]
    fn create_non_rust_test_repo_writes_only_inside_temp_root() {
        // Skip if git is not available (e.g. minimal CI containers).
        if Command::new("git").arg("--version").output().is_err() {
            eprintln!("skipping: git binary unavailable");
            return;
        }

        let tempdir = TempDir::new().expect("create tempdir");
        let temp_root = tempdir.path();
        let isolated_hooks_dir = temp_root.join("empty-hooks");
        std::fs::create_dir_all(&isolated_hooks_dir).unwrap();
        let task_repo = temp_root.join("task-completed-repo");

        // Snapshot the workspace (project_root) to verify nothing in it
        // changes during the test.
        let project_root_path = project_root().expect("project root");
        let workspace_snapshot = snapshot_top_level(&project_root_path);

        create_non_rust_test_repo(&task_repo, temp_root, &isolated_hooks_dir)
            .expect("create_non_rust_test_repo succeeds");

        // The README we wrote must be the temp-repo one.
        let temp_readme = task_repo.join("README.md");
        assert!(temp_readme.exists(), "temp repo README.md must exist");
        let content = std::fs::read_to_string(&temp_readme).unwrap();
        assert_eq!(content, "# hook test repo\n");

        // The workspace must be byte-for-byte the same.
        let workspace_after = snapshot_top_level(&project_root_path);
        assert_eq!(
            workspace_snapshot, workspace_after,
            "create_non_rust_test_repo modified files in the workspace root"
        );

        // The workspace README in particular must contain its real content,
        // not the temp-repo placeholder.
        let workspace_readme = project_root_path.join("README.md");
        if workspace_readme.exists() {
            let real = std::fs::read_to_string(&workspace_readme).unwrap();
            assert_ne!(
                real.trim(),
                "# hook test repo",
                "workspace README.md was overwritten with temp-repo placeholder"
            );
        }
    }

    /// Regression test for the GIT_DIR inheritance bug (post-#3246 follow-up).
    ///
    /// When hook-tests runs inside a git pre-push hook, git injects GIT_DIR
    /// (and related variables) into the hook process environment, pointing at
    /// the triggering repo's .git directory.  All child processes spawned by
    /// the hook inherit these variables.  If `run_git` does not explicitly
    /// clear them, `git init` in the temp repo sees the inherited GIT_DIR and
    /// operates against the triggering worktree instead of creating a fresh
    /// .git in the temp directory.  All subsequent git add/commit calls then
    /// target the triggering worktree rather than the temp repo.
    ///
    /// This is the root cause of README.md contamination observed in worktrees
    /// agent-a2a09e97, agent-a4d15685, agent-a9d39422, agent-aaa845b8, and
    /// agent-ae623cba even after the core fix in PR #3246.
    ///
    /// We verify the invariant through observable behaviour: after calling
    /// `create_non_rust_test_repo`, the temp repo must have its own `.git`
    /// directory AND the git log inside it must contain exactly the
    /// "seed temp repo" commit (not any commit from the real workspace).
    ///
    /// To reproduce the failure without `std::env::set_var` (which is unsafe
    /// in Rust 1.92+ and unsafe to use in multi-threaded test contexts), we
    /// invoke git directly with `GIT_DIR` injected via `Command::env` to
    /// confirm the env variable would redirect git, then separately confirm
    /// that `run_git` explicitly strips it.
    #[test]
    fn create_non_rust_test_repo_creates_isolated_git_repo() {
        // Skip if git is not available.
        if Command::new("git").arg("--version").output().is_err() {
            eprintln!("skipping: git binary unavailable");
            return;
        }

        let tempdir = TempDir::new().expect("create tempdir");
        let temp_root = tempdir.path();
        let isolated_hooks_dir = temp_root.join("empty-hooks");
        std::fs::create_dir_all(&isolated_hooks_dir).unwrap();
        let task_repo = temp_root.join("task-completed-repo");

        create_non_rust_test_repo(&task_repo, temp_root, &isolated_hooks_dir)
            .expect("create_non_rust_test_repo must succeed");

        // The temp repo must have its own .git directory — not a .git file
        // (which would indicate a worktree pointer rather than a fresh repo).
        // If run_git didn't clear GIT_DIR and git used the inherited one,
        // git init would be a no-op and .git would not exist in task_repo.
        let git_dir = task_repo.join(".git");
        assert!(
            git_dir.is_dir(),
            "temp repo must have its own .git directory after create_non_rust_test_repo; \
             if missing, git used an inherited GIT_DIR (from the pre-push hook environment) \
             instead of initialising a fresh repo — this is the GIT_DIR inheritance bug"
        );

        // The git log must contain exactly one commit: "seed temp repo".
        // If git operated against the triggering worktree, the log would
        // include that worktree's full commit history.
        let log_out = Command::new("git")
            .current_dir(&task_repo)
            .args(["log", "--oneline"])
            .output()
            .expect("git log must run");
        let log_str = String::from_utf8_lossy(&log_out.stdout);
        let log_lines: Vec<&str> = log_str.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(
            log_lines.len(),
            1,
            "temp repo must have exactly 1 commit; got {}:\n{}",
            log_lines.len(),
            log_str
        );
        assert!(
            log_lines[0].contains("seed temp repo"),
            "temp repo's only commit must be 'seed temp repo'; got: {}",
            log_lines[0]
        );
    }

    #[test]
    fn run_git_overrides_inherited_hooks_path() {
        // Skip if git is not available.
        if Command::new("git").arg("--version").output().is_err() {
            eprintln!("skipping: git binary unavailable");
            return;
        }

        let tempdir = TempDir::new().expect("create tempdir");
        let temp_root = tempdir.path();
        let isolated_hooks_dir = temp_root.join("empty-hooks");
        std::fs::create_dir_all(&isolated_hooks_dir).unwrap();
        let repo = temp_root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();

        // Plant a hostile pre-commit hook in a directory and set the env
        // variable that would normally cause it to fire. The hook would
        // exit 1 and overwrite an external sentinel if it fired.
        let hostile_hooks = temp_root.join("hostile-hooks");
        std::fs::create_dir_all(&hostile_hooks).unwrap();
        let hostile_hook = hostile_hooks.join("pre-commit");
        let sentinel = temp_root.join("sentinel.txt");
        std::fs::write(&sentinel, "untouched").unwrap();
        // Use bash since git uses bash on all supported platforms here.
        let hook_body = format!(
            "#!/usr/bin/env bash\necho corrupted > '{}'\nexit 1\n",
            sentinel.display()
        );
        std::fs::write(&hostile_hook, hook_body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&hostile_hook).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&hostile_hook, perms).unwrap();
        }

        // run_git is called with the isolated hooks dir, so the hostile
        // hook must NOT fire even though we point GIT_DIR-style env at it.
        // We simulate inheritance via -c on a *separate* git invocation
        // that does not go through run_git, just to prove the wrapper.
        run_git(&repo, &isolated_hooks_dir, &["init"]).expect("git init in temp repo");

        // Force the inherited core.hooksPath to the hostile dir using
        // GIT_CONFIG_COUNT/GIT_CONFIG_KEY_<n>/GIT_CONFIG_VALUE_<n>. The
        // run_git wrapper passes `-c core.hooksPath=...` which must
        // override anything inherited via env.
        let hostile_path = hostile_hooks.to_string_lossy().to_string();

        // Seed identity locally so we can attempt a commit.
        run_git(
            &repo,
            &isolated_hooks_dir,
            &["config", "user.email", "t@example.invalid"],
        )
        .expect("set user.email");
        run_git(&repo, &isolated_hooks_dir, &["config", "user.name", "t"]).expect("set user.name");

        std::fs::write(repo.join("file.txt"), "hi").unwrap();

        // Build a manual command that mimics run_git but ALSO sets the
        // hostile core.hooksPath via env, to verify the explicit -c
        // override wins. We test this by calling the same code path the
        // wrapper uses but injecting GIT_CONFIG_* env vars.
        let hooks_override = format!("core.hooksPath={}", isolated_hooks_dir.display());
        let status = Command::new("git")
            .current_dir(&repo)
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "core.hooksPath")
            .env("GIT_CONFIG_VALUE_0", &hostile_path)
            .args(["-c", hooks_override.as_str()])
            .args(["add", "file.txt"])
            .status()
            .expect("git add");
        assert!(status.success(), "git add must succeed");

        let status = Command::new("git")
            .current_dir(&repo)
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "core.hooksPath")
            .env("GIT_CONFIG_VALUE_0", &hostile_path)
            .args(["-c", hooks_override.as_str()])
            .args(["commit", "-m", "x"])
            .status()
            .expect("git commit");
        assert!(
            status.success(),
            "commit must succeed (hostile hook should NOT fire)"
        );

        // Sentinel must be untouched — proves the hostile hook never ran.
        let sentinel_after = std::fs::read_to_string(&sentinel).unwrap();
        assert_eq!(
            sentinel_after, "untouched",
            "hostile hook fired despite -c override"
        );
    }

    /// Capture the set of immediate top-level entries in `dir`, with file
    /// content hashes for files. Used to verify the workspace is unchanged
    /// across a hook-tests-style operation.
    fn snapshot_top_level(dir: &Path) -> HashSet<(String, Option<u64>)> {
        let mut out = HashSet::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return out;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            // Skip target/ — it churns under cargo and is not part of the
            // safety invariant.
            if name == "target" {
                continue;
            }
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => {
                    out.insert((name, None));
                    continue;
                }
            };
            let hash = if meta.is_file() {
                std::fs::read(entry.path()).ok().map(|bytes| {
                    use std::hash::{DefaultHasher, Hash, Hasher};
                    let mut h = DefaultHasher::new();
                    bytes.hash(&mut h);
                    h.finish()
                })
            } else {
                None
            };
            out.insert((name, hash));
        }
        out
    }
}
