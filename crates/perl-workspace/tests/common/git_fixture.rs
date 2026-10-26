//! Keep disposable Git fixtures independent of a calling hook's repository.

use std::ffi::OsStr;
use std::process::{Command, Output};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Repository-local selectors reported by `git rev-parse --local-env-vars`.
pub(crate) const LOCAL_GIT_ENV: &[&str] = &[
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_OBJECT_DIRECTORY",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_GRAFT_FILE",
    "GIT_INDEX_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_PREFIX",
    "GIT_SHALLOW_FILE",
    "GIT_COMMON_DIR",
];

const CHILD_TEST: &str = "PERL_WORKSPACE_GIT_FIXTURE_CHILD";

/// Clear selectors on a fixture child, without changing the caller's environment.
pub(crate) fn clear_git_context(command: &mut Command) -> &mut Command {
    for variable in LOCAL_GIT_ENV {
        command.env_remove(variable);
    }
    for (key, _) in std::env::vars_os() {
        let name = key.to_string_lossy();
        if name.starts_with("GIT_CONFIG_KEY_") || name.starts_with("GIT_CONFIG_VALUE_") {
            command.env_remove(key);
        }
    }
    command
}

/// Run the complete fixture, including production functions that themselves invoke Git.
pub(crate) fn run_isolated_test(name: &str, body: impl FnOnce() -> TestResult) -> TestResult {
    if LOCAL_GIT_ENV.iter().any(|key| std::env::var_os(key).is_some()) {
        let mut child = exact_test_child(name)?;
        clear_git_context(&mut child);
        validate_child(name, &child.output()?)?;
    } else {
        body()?;
    }

    if std::env::var_os(CHILD_TEST).as_deref() == Some(OsStr::new(name)) {
        emit_completion(name);
    }
    Ok(())
}

/// Select one complete test body, including its completion receipt.
pub(crate) fn exact_test_child(name: &str) -> Result<Command, Box<dyn std::error::Error>> {
    let mut child = Command::new(std::env::current_exe()?);
    child
        .args(["--exact", name, "--nocapture", "--color", "never", "--test-threads=1"])
        .env(CHILD_TEST, name);
    Ok(child)
}

#[allow(clippy::print_stdout, reason = "exact-test child emits proof after all fixture assertions")]
fn emit_completion(name: &str) {
    println!("\nfixture-complete:{name}");
}

pub(crate) fn validate_child(name: &str, output: &Output) -> TestResult {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let token = format!("fixture-complete:{name}");
    if output.status.success()
        // Forked property runners print a nested banner before returning to
        // the one outer test and its after-body completion receipt.
        && stdout.lines().any(|line| line == "running 1 test")
        && stdout
            .lines()
            .filter(|line| line.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;"))
            .count()
            == 1
        && stdout.lines().filter(|line| *line == token).count() == 1
    {
        return Ok(());
    }
    Err(format!(
        "isolated fixture {name} did not complete exactly once ({}):\n{}\n{}",
        output.status,
        stdout,
        String::from_utf8_lossy(&output.stderr)
    )
    .into())
}
