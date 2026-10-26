//! Git hook selectors must never redirect disposable discovery fixtures into their caller.

#[path = "common/git_fixture.rs"]
mod git_fixture;

use perl_workspace::discovery::{DiscoveryMethod, discover_perl_files};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const FIXTURE_TEST: &str = "fixture_body_discovers_its_own_repository";
const SETUP_ONLY_CONTROL: &str = "PERL_WORKSPACE_SETUP_ONLY_NEGATIVE_CONTROL";
const DISCOVERY_ASSERTION: &str = "fixture discovery must stay within its own repository";

fn git(root: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let mut command = Command::new("git");
    git_fixture::clear_git_context(&mut command);
    let output = command.args(args).current_dir(root).output()?;
    if !output.status.success() {
        return Err(
            format!("fixture git {args:?}: {}", String::from_utf8_lossy(&output.stderr)).into()
        );
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

#[test]
fn fixture_body_discovers_its_own_repository() -> TestResult {
    let body = || {
        let fixture = tempfile::tempdir()?;
        git(fixture.path(), &["init", "--quiet"])?;
        fs::write(fixture.path().join(".gitignore"), "ignored.pm\n")?;
        fs::write(fixture.path().join("visible.pm"), "# fixture\n")?;
        fs::write(fixture.path().join("ignored.pm"), "# ignored\n")?;
        let result = discover_perl_files(fixture.path());
        assert_eq!(result.method, DiscoveryMethod::Git, "{DISCOVERY_ASSERTION}");
        assert_eq!(result.files, vec![fixture.path().join("visible.pm")], "{DISCOVERY_ASSERTION}");
        Ok(())
    };
    // Only the disposable-caller regression selects this incomplete repair:
    // setup Git is clean, while production discovery still inherits selectors.
    if std::env::var_os(SETUP_ONLY_CONTROL).is_some() {
        body()
    } else {
        git_fixture::run_isolated_test(FIXTURE_TEST, body)
    }
}

fn snapshot_tree(root: &Path, entries: &mut BTreeMap<PathBuf, Vec<u8>>) -> TestResult {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            snapshot_tree(&entry.path(), entries)?;
        } else {
            entries.insert(entry.path(), fs::read(entry.path())?);
        }
    }
    Ok(())
}

fn caller_snapshot(
    caller: &Path,
    git_dir: &Path,
    common: &Path,
    alternate_index: &Path,
) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn std::error::Error>> {
    let mut entries = BTreeMap::new();
    for path in [
        common.join("config"),
        common.join("HEAD"),
        git_dir.join("HEAD"),
        git_dir.join("index"),
        alternate_index.to_path_buf(),
        caller.join("tracked.txt"),
        caller.join("staged.txt"),
        caller.join("untracked.txt"),
    ] {
        entries.insert(path.clone(), fs::read(path)?);
    }
    snapshot_tree(&common.join("refs"), &mut entries)?;
    snapshot_tree(&common.join("objects"), &mut entries)?;
    Ok(entries)
}

#[test]
fn fixture_children_preserve_disposable_callers() -> TestResult {
    git_fixture::run_isolated_test("fixture_children_preserve_disposable_callers", || {
        let parent_environment: BTreeMap<_, _> = std::env::vars_os().collect();
        for linked in [false, true] {
            let tmp = tempfile::tempdir()?;
            let repo = tmp.path().join("caller");
            fs::create_dir(&repo)?;
            git(&repo, &["init", "--quiet"])?;
            git(&repo, &["config", "user.name", "Disposable Caller"])?;
            git(&repo, &["config", "user.email", "caller@example.invalid"])?;
            git(&repo, &["config", "commit.gpgsign", "false"])?;
            fs::write(repo.join("tracked.txt"), "initial\n")?;
            git(&repo, &["add", "tracked.txt"])?;
            git(&repo, &["commit", "--quiet", "-m", "initial"])?;
            let caller = if linked {
                let path = tmp.path().join("linked");
                let path_text = path.to_str().ok_or("non-UTF8 disposable worktree path")?;
                git(&repo, &["worktree", "add", "--quiet", "-b", "fixture-linked", path_text])?;
                path
            } else {
                repo
            };
            fs::write(caller.join("staged.txt"), "unrelated staged content\n")?;
            git(&caller, &["add", "staged.txt"])?;
            fs::write(caller.join("tracked.txt"), "unrelated modified content\n")?;
            fs::write(caller.join("untracked.txt"), "unrelated untracked content\n")?;
            let git_dir = PathBuf::from(git(&caller, &["rev-parse", "--absolute-git-dir"])?);
            let common = PathBuf::from(git(
                &caller,
                &["rev-parse", "--path-format=absolute", "--git-common-dir"],
            )?);
            let alternate = tmp.path().join("alternate.index");
            fs::copy(git_dir.join("index"), &alternate)?;
            let fixture_tmp = tmp.path().join("fixtures");
            fs::create_dir(&fixture_tmp)?;
            for scenario in ["clean", "dir", "worktree", "index", "implicit", "combined", "config"]
            {
                let before = caller_snapshot(&caller, &git_dir, &common, &alternate)?;
                let mut child = git_fixture::exact_test_child(FIXTURE_TEST)?;
                git_fixture::clear_git_context(&mut child);
                child.current_dir(&caller).env("TMPDIR", &fixture_tmp).env("TMP", &fixture_tmp);
                match scenario {
                    "dir" => {
                        child.env("GIT_DIR", &git_dir);
                    }
                    "worktree" => {
                        child.env("GIT_WORK_TREE", &caller);
                    }
                    "index" => {
                        child.env("GIT_INDEX_FILE", &alternate);
                    }
                    "implicit" => {
                        child.env("GIT_DIR", &git_dir).env("GIT_IMPLICIT_WORK_TREE", "0");
                    }
                    "combined" => {
                        child
                            .env("GIT_DIR", &git_dir)
                            .env("GIT_COMMON_DIR", &common)
                            .env("GIT_WORK_TREE", &caller)
                            .env("GIT_INDEX_FILE", &alternate)
                            .env("GIT_IMPLICIT_WORK_TREE", "0");
                    }
                    "config" => {
                        child
                            .env("GIT_CONFIG_COUNT", "1")
                            .env("GIT_CONFIG_KEY_0", "core.bare")
                            .env("GIT_CONFIG_VALUE_0", "true");
                    }
                    _ => {}
                }
                let output = child.output()?;
                let after = caller_snapshot(&caller, &git_dir, &common, &alternate)?;
                assert_eq!(before, after, "caller changed: linked={linked}, scenario={scenario}");
                git_fixture::validate_child(FIXTURE_TEST, &output)?;
                assert_eq!(git(&caller, &["config", "user.name"])?, "Disposable Caller");
                assert_eq!(git(&caller, &["config", "user.email"])?, "caller@example.invalid");
            }

            let before = caller_snapshot(&caller, &git_dir, &common, &alternate)?;
            let mut partial = git_fixture::exact_test_child(FIXTURE_TEST)?;
            git_fixture::clear_git_context(&mut partial);
            let output = partial
                .current_dir(&caller)
                .env("GIT_WORK_TREE", &caller)
                .env(SETUP_ONLY_CONTROL, "1")
                .env("TMPDIR", &fixture_tmp)
                .env("TMP", &fixture_tmp)
                .output()?;
            assert!(
                !output.status.success(),
                "setup-only isolation must fail discovery assertions"
            );
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(DISCOVERY_ASSERTION),
                "negative control must reach the intended discovery assertion"
            );
            assert_eq!(
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter(|line| line
                        .starts_with("test result: FAILED. 0 passed; 1 failed; 0 ignored;"))
                    .count(),
                1,
                "negative control must execute and fail exactly one test"
            );
            assert!(git_fixture::validate_child(FIXTURE_TEST, &output).is_err());
            assert_eq!(before, caller_snapshot(&caller, &git_dir, &common, &alternate)?);
        }
        assert_eq!(parent_environment, std::env::vars_os().collect());
        Ok(())
    })
}

#[test]
fn installed_git_selectors_are_removed_only_from_fixture_commands() -> TestResult {
    let installed = git(Path::new("."), &["rev-parse", "--local-env-vars"])?;
    let actual: std::collections::BTreeSet<_> = installed.lines().collect();
    let expected: std::collections::BTreeSet<_> =
        git_fixture::LOCAL_GIT_ENV.iter().copied().collect();
    assert_eq!(actual, expected, "review newly introduced Git repository selectors");
    let before: BTreeMap<_, _> = std::env::vars_os().collect();
    let mut child = Command::new("git");
    git_fixture::clear_git_context(&mut child);
    let removed: BTreeMap<_, _> = child.get_envs().collect();
    for key in git_fixture::LOCAL_GIT_ENV {
        assert_eq!(removed.get(std::ffi::OsStr::new(key)), Some(&None));
    }
    for identity in
        ["GIT_AUTHOR_NAME", "GIT_AUTHOR_EMAIL", "GIT_COMMITTER_NAME", "GIT_COMMITTER_EMAIL"]
    {
        assert!(!removed.contains_key(std::ffi::OsStr::new(identity)));
    }
    assert_eq!(before, std::env::vars_os().collect());
    Ok(())
}

#[test]
fn child_completion_refuses_zero_failed_missing_and_wrong_receipts() -> TestResult {
    let zero = git_fixture::exact_test_child("this_test_does_not_exist")?.output()?;
    assert!(zero.status.success(), "the zero-test negative control must itself exit successfully");
    assert!(git_fixture::validate_child("proof", &zero).is_err());
    let failed = Command::new("git").arg("--not-a-git-option").output()?.status;
    assert!(!failed.success());
    let summary = "running 1 test\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n";
    let valid = format!("{summary}fixture-complete:proof\n");
    let output =
        |status, stdout: String| Output { status, stdout: stdout.into_bytes(), stderr: vec![] };
    git_fixture::validate_child("proof", &output(zero.status, valid.clone()))?;
    git_fixture::validate_child("proof", &output(zero.status, format!("running 1 test\n{valid}")))?;
    for text in [
        summary.to_string(),
        format!("{summary}fixture-complete:wrong\n"),
        format!("{valid}fixture-complete:proof\n"),
        format!("{valid}{summary}"),
    ] {
        assert!(git_fixture::validate_child("proof", &output(zero.status, text)).is_err());
    }
    assert!(git_fixture::validate_child("proof", &output(failed, valid)).is_err());
    Ok(())
}
