//! Real local pre-push callers must survive disposable fixture operations.
#![cfg(unix)]

#[path = "common/git_fixture.rs"]
mod git_fixture;

use anyhow::{Result, bail, ensure};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const TEST: &str = "normal_hooks_preserve_callers_and_execute_fixture_operations";
const ROLE: &str = "XTASK_FIXTURE_PROBE_ROLE";

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()?;
    ensure!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn init_repo(root: &Path) -> Result<()> {
    fs::create_dir_all(root)?;
    git(root, &["init", "-q", "-b", "main"])?;
    git(root, &["config", "user.name", "Fixture identity"])?;
    git(root, &["config", "user.email", "fixture@example.invalid"])?;
    git(root, &["config", "commit.gpgsign", "false"])?;
    fs::write(root.join("tracked.txt"), "committed\n")?;
    git(root, &["add", "tracked.txt"])?;
    git(root, &["commit", "-qm", "initial fixture"])?;
    Ok(())
}

fn snapshot(roots: &[PathBuf]) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    let mut files = BTreeMap::new();
    for root in roots {
        for entry in walkdir::WalkDir::new(root) {
            let entry = entry?;
            if entry.file_type().is_file() {
                files.insert(entry.path().to_owned(), fs::read(entry.path())?);
            }
        }
    }
    Ok(files)
}

/// This body must really initialize, commit, read and update its own fixture.
fn fixture_operations() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let repo = temp.path().join("fixture");
    init_repo(&repo)?;
    ensure!(git(&repo, &["config", "user.email"])? == "fixture@example.invalid");
    let first = git(&repo, &["rev-parse", "HEAD"])?;
    fs::write(repo.join("tracked.txt"), "fixture changed\n")?;
    git(&repo, &["add", "tracked.txt"])?;
    git(&repo, &["commit", "-qm", "fixture second commit"])?;
    ensure!(git(&repo, &["rev-parse", "HEAD"])? != first);
    ensure!(git(&repo, &["show", "HEAD:tracked.txt"])? == "fixture changed");

    // Exercise production Git below a second process boundary too.
    let output = Command::new(assert_cmd::cargo_bin!("xtask"))
        .args(["worktree-cleanup", "--root"])
        .arg(&repo)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()?;
    ensure!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    ensure!(String::from_utf8_lossy(&output.stdout).contains("No stale worktrees to clean up"));
    Ok(())
}

fn hook_guard() -> Result<()> {
    let root = PathBuf::from(
        std::env::var_os("XTASK_FIXTURE_CALLER_ROOT")
            .ok_or_else(|| anyhow::anyhow!("missing disposable caller root"))?,
    );
    if let Some(git_dir) = std::env::var_os("GIT_DIR") {
        ensure!(Path::new(&git_dir).canonicalize()?.starts_with(&root));
    } else {
        ensure!(
            std::env::var("XTASK_FIXTURE_EXPECT_LINKED")? == "false",
            "normal linked-worktree hook lacks GIT_DIR"
        );
    }
    ensure!(
        Path::new(&git(&std::env::current_dir()?, &["rev-parse", "--absolute-git-dir"])?)
            .starts_with(&root)
    );
    let roots = [root.join("caller"), root.join("linked")];
    let roots: Vec<_> = roots.into_iter().filter(|path| path.exists()).collect();
    let before = snapshot(&roots)?;
    let environment: BTreeMap<OsString, OsString> = std::env::vars_os().collect();
    let output = Command::new(std::env::current_exe()?)
        .args(["--exact", TEST, "--nocapture", "--color", "never", "--test-threads=1"])
        .env(ROLE, "fixture")
        .output()?;
    let after = snapshot(&roots)?;
    ensure!(before == after, "fixture changed caller config/index/refs/objects/worktree");
    ensure!(environment == std::env::vars_os().collect(), "fixture changed hook environment");
    ensure!(
        output.status.success(),
        "fixture failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    ensure!(
        stdout
            .lines()
            .filter(|line| line.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;"))
            .count()
            == 1
    );
    fs::write(root.join("hook-completed"), "caller unchanged; fixture completed\n")?;
    Ok(())
}

fn replay_hooks() -> Result<()> {
    for linked in [false, true] {
        for alternate_index in [false, true] {
            let temp = tempfile::tempdir()?;
            let root = temp.path().canonicalize()?;
            let repo = root.join("caller");
            init_repo(&repo)?;
            let remote = root.join("origin.git");
            git(
                &root,
                &[
                    "init",
                    "-q",
                    "--bare",
                    remote.to_str().ok_or_else(|| anyhow::anyhow!("non-UTF-8 fixture root"))?,
                ],
            )?;
            git(
                &repo,
                &[
                    "remote",
                    "add",
                    "origin",
                    remote.to_str().ok_or_else(|| anyhow::anyhow!("non-UTF-8 remote"))?,
                ],
            )?;
            let caller = if linked {
                let linked_path = root.join("linked");
                git(
                    &repo,
                    &[
                        "worktree",
                        "add",
                        "-qb",
                        "linked-fixture",
                        linked_path
                            .to_str()
                            .ok_or_else(|| anyhow::anyhow!("non-UTF-8 worktree"))?,
                    ],
                )?;
                linked_path
            } else {
                repo.clone()
            };
            fs::write(caller.join("staged.txt"), "unrelated staged data\n")?;
            git(&caller, &["add", "staged.txt"])?;
            fs::write(caller.join("tracked.txt"), "unrelated unstaged data\n")?;
            fs::write(caller.join("untracked.txt"), "unrelated untracked data\n")?;
            let hook = repo.join(".git/hooks/pre-push");
            fs::write(
                &hook,
                "#!/bin/sh\nexport XTASK_FIXTURE_PROBE_ROLE=guard\nexec \"$XTASK_FIXTURE_TEST_EXE\" --exact normal_hooks_preserve_callers_and_execute_fixture_operations --nocapture --color never --test-threads=1\n",
            )?;
            fs::set_permissions(&hook, fs::Permissions::from_mode(0o755))?;
            let mut push = Command::new("git");
            push.current_dir(&caller)
                .args(["push", "origin", "HEAD:refs/heads/caller-proof"])
                .env("XTASK_FIXTURE_TEST_EXE", std::env::current_exe()?)
                .env("XTASK_FIXTURE_CALLER_ROOT", &root)
                .env("XTASK_FIXTURE_EXPECT_LINKED", linked.to_string())
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1");
            if alternate_index {
                let index = PathBuf::from(git(
                    &caller,
                    &["rev-parse", "--path-format=absolute", "--git-path", "index"],
                )?);
                let alternate = caller.join("alternate.index");
                fs::copy(index, &alternate)?;
                push.env("GIT_INDEX_FILE", alternate).env("GIT_WORK_TREE", &caller);
            }
            let output = push.output()?;
            ensure!(
                output.status.success(),
                "linked={linked} alternate_index={alternate_index}: {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            ensure!(
                fs::read_to_string(root.join("hook-completed"))?
                    == "caller unchanged; fixture completed\n"
            );
        }
    }
    Ok(())
}

#[test]
fn normal_hooks_preserve_callers_and_execute_fixture_operations() -> Result<()> {
    match std::env::var(ROLE).as_deref() {
        Ok("guard") => hook_guard(),
        Ok("fixture") => git_fixture::run_isolated_test(module_path!(), TEST, fixture_operations),
        Err(std::env::VarError::NotPresent) => {
            git_fixture::run_isolated_test(module_path!(), TEST, replay_hooks)
        }
        other => bail!("unexpected fixture probe role: {other:?}"),
    }
}

#[test]
fn child_completion_rejects_missing_failed_and_ambiguous_proof() -> Result<()> {
    let valid = format!(
        "running 1 test\nfixture-complete:{TEST}\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n"
    );
    let success = Command::new("sh").args(["-c", "exit 0"]).status()?;
    let failure = Command::new("sh").args(["-c", "exit 1"]).status()?;
    ensure!(success.success() && !failure.success());
    let output =
        |text: &str, status| Output { status, stdout: text.as_bytes().to_vec(), stderr: vec![] };
    git_fixture::validate_child(TEST, &output(&valid, success))?;
    for invalid in [
        valid.replace("1 passed", "0 passed"),
        valid.replace(&format!("fixture-complete:{TEST}\n"), ""),
        valid.replace(TEST, "wrong-test"),
        format!("{valid}fixture-complete:{TEST}\n"),
        format!("{valid}{valid}"),
    ] {
        ensure!(git_fixture::validate_child(TEST, &output(&invalid, success)).is_err());
    }
    ensure!(git_fixture::validate_child(TEST, &output(&valid, failure)).is_err());
    Ok(())
}

#[test]
fn fixture_selector_list_covers_installed_git_contract() -> Result<()> {
    let output = Command::new("git").args(["rev-parse", "--local-env-vars"]).output()?;
    ensure!(output.status.success());
    for name in String::from_utf8(output.stdout)?.lines() {
        ensure!(git_fixture::LOCAL_GIT_ENV.contains(&name), "unhandled Git selector: {name}");
    }
    Ok(())
}
