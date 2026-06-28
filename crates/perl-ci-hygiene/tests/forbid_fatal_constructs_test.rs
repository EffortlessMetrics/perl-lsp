use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn new(label: &str) -> TestResult<Self> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = env::temp_dir()
            .join(format!("perl-ci-hygiene-fatal-{label}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path)?;
        fs::write(path.join("Cargo.toml"), "[workspace]\n")?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write_crate_src(&self, crate_name: &str, file: &str, content: &str) -> TestResult<()> {
        let src = self.path.join("crates").join(crate_name).join("src");
        fs::create_dir_all(&src)?;
        fs::write(src.join(file), content)?;
        Ok(())
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn perl_ci_hygiene_binary() -> TestResult<PathBuf> {
    env::var_os("CARGO_BIN_EXE_perl-ci-hygiene").map(PathBuf::from).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "CARGO_BIN_EXE_perl-ci-hygiene was not set by cargo",
        )
        .into()
    })
}

fn run_forbid_fatal_constructs(repo: &Path) -> TestResult<Output> {
    let bin = perl_ci_hygiene_binary()?;
    Ok(Command::new(bin).arg("forbid-fatal-constructs").current_dir(repo).output()?)
}

fn run_forbid_fatal_constructs_verbose(repo: &Path) -> TestResult<Output> {
    let bin = perl_ci_hygiene_binary()?;
    Ok(Command::new(bin)
        .args(["forbid-fatal-constructs", "--verbose"])
        .current_dir(repo)
        .output()?)
}

/// Verifies that clean production code (no abort/exit) passes the check.
#[test]
fn forbid_fatal_constructs_passes_clean_code() -> TestResult {
    let repo = TempRepo::new("clean")?;
    repo.write_crate_src(
        "my-crate",
        "lib.rs",
        r#"
pub fn safe_operation() -> Result<(), String> {
    Ok(())
}
"#,
    )?;

    let out = run_forbid_fatal_constructs(repo.path())?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "clean code should pass\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    Ok(())
}

/// Verifies that std::process::abort() in production code triggers a failure.
#[test]
fn forbid_fatal_constructs_detects_abort_in_lib() -> TestResult {
    let repo = TempRepo::new("abort-in-lib")?;
    repo.write_crate_src(
        "my-crate",
        "lib.rs",
        r#"
pub fn dangerous() {
    std::process::abort();
}
"#,
    )?;

    let out = run_forbid_fatal_constructs(repo.path())?;
    assert_ne!(
        out.status.code(),
        Some(0),
        "abort() in lib code should fail\nstdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("abort"), "output should mention abort\nstdout: {stdout}");
    Ok(())
}

/// Verifies that std::process::exit() outside allowlisted paths triggers a failure.
#[test]
fn forbid_fatal_constructs_detects_exit_in_lib() -> TestResult {
    let repo = TempRepo::new("exit-in-lib")?;
    repo.write_crate_src(
        "my-crate",
        "lib.rs",
        r#"
pub fn shutdown() {
    std::process::exit(0);
}
"#,
    )?;

    let out = run_forbid_fatal_constructs(repo.path())?;
    assert_ne!(
        out.status.code(),
        Some(0),
        "exit() in non-allowlisted lib code should fail\nstdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("exit"), "output should mention exit\nstdout: {stdout}");
    Ok(())
}

/// Verifies that std::process::exit() inside a bin/ directory is allowed.
#[test]
fn forbid_fatal_constructs_allows_exit_in_bin() -> TestResult {
    let repo = TempRepo::new("exit-in-bin")?;
    // Create a bin/ path that is allowlisted.
    let bin_dir = repo.path().join("crates").join("my-crate").join("src").join("bin");
    fs::create_dir_all(&bin_dir)?;
    fs::write(
        bin_dir.join("my-tool.rs"),
        r#"
fn main() {
    std::process::exit(0);
}
"#,
    )?;

    let out = run_forbid_fatal_constructs(repo.path())?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "exit() in bin/ should be allowed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    Ok(())
}

/// Verifies that std::process::abort() in a tests/ directory is excluded from the check.
#[test]
fn forbid_fatal_constructs_ignores_test_directories() -> TestResult {
    let repo = TempRepo::new("abort-in-tests")?;
    // Place abort() call in a tests/ directory — the scanner must skip this path.
    let test_dir = repo.path().join("crates").join("my-crate").join("tests");
    fs::create_dir_all(&test_dir)?;
    fs::write(test_dir.join("integration_test.rs"), "fn helper() { std::process::abort(); }\n")?;

    let out = run_forbid_fatal_constructs(repo.path())?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "abort() in tests/ should be excluded from the scan\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    Ok(())
}

/// Verifies that --verbose prints the policy summary when the check passes.
/// Covers the report_success() code path (lines 105-115 in fatal_constructs.rs).
#[test]
fn forbid_fatal_constructs_verbose_prints_policy_summary() -> TestResult {
    let repo = TempRepo::new("verbose-clean")?;
    repo.write_crate_src("my-crate", "lib.rs", "pub fn safe() {}\n")?;

    let out = run_forbid_fatal_constructs_verbose(repo.path())?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "verbose clean code should pass\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No forbidden fatal constructs"),
        "verbose output should mention success\nstdout: {stdout}"
    );
    assert!(
        stdout.contains("abort()") && stdout.contains("exit()"),
        "verbose output should print policy summary\nstdout: {stdout}"
    );
    Ok(())
}

/// Verifies that non-Rust files inside the crates directory are skipped.
/// Covers the file-extension filter branch (line 27 in fatal_constructs.rs).
#[test]
fn forbid_fatal_constructs_skips_non_rust_files() -> TestResult {
    let repo = TempRepo::new("non-rs-files")?;
    // A clean .rs file so the check runs at all.
    repo.write_crate_src("my-crate", "lib.rs", "pub fn safe() {}\n")?;
    // A non-.rs file that contains the forbidden pattern — must be ignored.
    let src = repo.path().join("crates").join("my-crate").join("src");
    fs::write(src.join("README.md"), "Do not call std::process::abort() here.\n")?;

    let out = run_forbid_fatal_constructs(repo.path())?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "abort pattern in a .md file should be ignored\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    Ok(())
}
