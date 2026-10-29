//! Isolate complete disposable Git fixtures from a calling hook's repository.

use std::ffi::OsStr;
use std::io;
use std::process::{Command, Output};

/// Repository-local variables reported by `git rev-parse --local-env-vars`.
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

const CHILD_TEST: &str = "XTASK_GIT_FIXTURE_CHILD";

fn is_local_variable(name: &OsStr) -> bool {
    let name = name.to_string_lossy();
    let name = if cfg!(windows) { name.to_ascii_uppercase() } else { name.into_owned() };
    LOCAL_GIT_ENV.contains(&name.as_str())
        || name.starts_with("GIT_CONFIG_KEY_")
        || name.starts_with("GIT_CONFIG_VALUE_")
}

/// Change only the disposable child, leaving the hook's environment intact.
pub(crate) fn clear_git_context(command: &mut Command) -> &mut Command {
    for (key, _) in std::env::vars_os() {
        if is_local_variable(&key) {
            command.env_remove(key);
        }
    }
    for key in LOCAL_GIT_ENV {
        command.env_remove(key);
    }
    command
}

/// Include setup, assertions and nested production Git calls in one child.
pub(crate) fn run_isolated_test<E: From<io::Error>>(
    module: &str,
    function: &str,
    body: impl FnOnce() -> Result<(), E>,
) -> Result<(), E> {
    // libtest names omit the binary/crate prefix from module_path!().
    let module = module.split_once("::").map_or("", |(_, rest)| rest);
    let name =
        if module.is_empty() { function.to_owned() } else { format!("{module}::{function}") };
    if std::env::vars_os().any(|(key, _)| is_local_variable(&key)) {
        let mut child = Command::new(std::env::current_exe()?);
        child
            .args(["--exact", &name, "--nocapture", "--color", "never", "--test-threads=1"])
            .env(CHILD_TEST, &name);
        clear_git_context(&mut child);
        validate_child(&name, &child.output()?)?;
    } else {
        body()?;
    }
    if std::env::var_os(CHILD_TEST).as_deref() == Some(OsStr::new(&name)) {
        emit_completion(&name);
    }
    Ok(())
}

#[allow(clippy::print_stdout, reason = "after-body token proves the exact fixture completed")]
fn emit_completion(name: &str) {
    println!("\nfixture-complete:{name}");
}

pub(crate) fn validate_child(name: &str, output: &Output) -> io::Result<()> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let token = format!("fixture-complete:{name}");
    if output.status.success()
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
    Err(io::Error::other(format!(
        "isolated fixture {name} did not complete exactly once ({}):\n{}\n{}",
        output.status,
        stdout,
        String::from_utf8_lossy(&output.stderr)
    )))
}
