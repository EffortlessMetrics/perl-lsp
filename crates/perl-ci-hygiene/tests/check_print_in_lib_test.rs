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
            .join(format!("perl-ci-hygiene-print-{label}-{}-{nanos}", std::process::id()));
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

    fn write_baseline(&self, count: usize) -> TestResult<()> {
        let ci = self.path.join("ci");
        fs::create_dir_all(&ci)?;
        fs::write(ci.join("print_in_lib_baseline.txt"), format!("{count}\n"))?;
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

fn run_check_print_in_lib(repo: &Path) -> TestResult<Output> {
    let bin = perl_ci_hygiene_binary()?;
    Ok(Command::new(bin).arg("check-print-in-lib").current_dir(repo).output()?)
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Verifies that a println! in actual library code triggers a failure.
#[test]
fn check_print_in_lib_detects_println_in_lib_code() -> TestResult {
    let repo = TempRepo::new("detects-println")?;
    repo.write_baseline(0)?;
    repo.write_crate_src(
        "my-crate",
        "lib.rs",
        r#"
pub fn greet(name: &str) {
    println!("Hello, {name}");
}
"#,
    )?;

    let out = run_check_print_in_lib(repo.path())?;
    assert_ne!(
        out.status.code(),
        Some(0),
        "should fail: println! in library code\nstdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("FAIL"), "should report FAIL in output\nstdout: {stdout}");
    Ok(())
}

/// Verifies that a file with NO print macros passes cleanly.
#[test]
fn check_print_in_lib_passes_for_clean_file() -> TestResult {
    let repo = TempRepo::new("clean-file")?;
    repo.write_baseline(0)?;
    repo.write_crate_src(
        "my-crate",
        "lib.rs",
        r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    )?;

    let out = run_check_print_in_lib(repo.path())?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "should pass for clean library code\nstdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    Ok(())
}

/// Verifies that eprintln! inside a #[cfg(test)] block is excluded.
#[test]
fn check_print_in_lib_allows_print_in_cfg_test_block() -> TestResult {
    let repo = TempRepo::new("cfg-test-block")?;
    repo.write_baseline(0)?;
    repo.write_crate_src(
        "my-crate",
        "lib.rs",
        r#"
pub fn compute(x: i32) -> i32 {
    x * 2
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_compute() {
        let result = compute(3);
        eprintln!("Debug: result = {result}");
        assert_eq!(result, 6);
    }
}
"#,
    )?;

    let out = run_check_print_in_lib(repo.path())?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "eprintln! inside #[cfg(test)] should be allowed\nstdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    Ok(())
}

/// Verifies that a file with a file-level `#![allow(clippy::print_stderr)]`
/// is excluded wholesale (e.g. a CLI module with user-facing output).
#[test]
fn check_print_in_lib_skips_file_with_file_level_allow() -> TestResult {
    let repo = TempRepo::new("file-level-allow")?;
    repo.write_baseline(0)?;
    repo.write_crate_src(
        "my-crate",
        "cli.rs",
        r#"// cli.rs — user-facing CLI output is intentional here.
#![allow(clippy::print_stderr, clippy::print_stdout)]

pub fn run() {
    println!("Starting...");
    eprintln!("Error occurred");
}
"#,
    )?;

    let out = run_check_print_in_lib(repo.path())?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "file with file-level #![allow(clippy::print_*)] should be skipped\nstdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    Ok(())
}

/// Verifies that build.rs files are excluded (they use println! for cargo directives).
#[test]
fn check_print_in_lib_skips_build_rs() -> TestResult {
    let repo = TempRepo::new("build-rs")?;
    repo.write_baseline(0)?;
    // build.rs is at crate root, not in src/
    let crate_dir = repo.path().join("crates").join("my-crate");
    fs::create_dir_all(&crate_dir)?;
    fs::write(
        crate_dir.join("build.rs"),
        r#"fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}
"#,
    )?;
    // Also write a clean lib.rs so the crate is scanned at all
    let src = crate_dir.join("src");
    fs::create_dir_all(&src)?;
    fs::write(src.join("lib.rs"), "pub fn ok() {}\n")?;

    let out = run_check_print_in_lib(repo.path())?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "build.rs files should be excluded from the print check\nstdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    Ok(())
}

/// Verifies that a `test_<name>.rs` file adjacent to src/ is excluded
/// (these are test-driver files, not library code).
#[test]
fn check_print_in_lib_skips_test_prefix_files() -> TestResult {
    let repo = TempRepo::new("test-prefix")?;
    repo.write_baseline(0)?;
    let crate_dir = repo.path().join("crates").join("my-crate");
    let src = crate_dir.join("src");
    fs::create_dir_all(&src)?;
    fs::write(src.join("lib.rs"), "pub fn ok() {}\n")?;
    // test_helper.rs alongside lib.rs — excluded by prefix rule
    fs::write(
        src.join("test_helper.rs"),
        r#"fn helper() {
    println!("debug from test helper");
}
"#,
    )?;

    let out = run_check_print_in_lib(repo.path())?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "test_*.rs files should be excluded from the print check\nstdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    Ok(())
}

/// Verifies that eprintln! inside a `#[cfg(debug_assertions)]` block is allowed
/// (debug-only guardrails are legitimate).
#[test]
fn check_print_in_lib_allows_print_in_debug_assertions_block() -> TestResult {
    let repo = TempRepo::new("debug-assertions")?;
    repo.write_baseline(0)?;
    repo.write_crate_src(
        "my-crate",
        "lib.rs",
        r#"
pub fn process(x: i32) -> i32 {
    let result = x + 1;

    #[cfg(debug_assertions)]
    if result > 100 {
        eprintln!("warn: result {result} exceeds expected range");
    }

    result
}
"#,
    )?;

    let out = run_check_print_in_lib(repo.path())?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "eprintln! inside #[cfg(debug_assertions)] should be allowed\nstdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    Ok(())
}

/// Verifies that a nearby debug-only block does not hide a later production print.
#[test]
fn check_print_in_lib_rejects_print_after_debug_assertions_block() -> TestResult {
    let repo = TempRepo::new("debug-assertions-prod")?;
    repo.write_baseline(0)?;
    repo.write_crate_src(
        "my-crate",
        "lib.rs",
        r#"
pub fn process(x: i32) -> i32 {
    let result = x + 1;

    #[cfg(debug_assertions)]
    if result > 100 {
        eprintln!("warn: result {result} exceeds expected range");
    }

    println!("production output");
    result
}
"#,
    )?;

    let out = run_check_print_in_lib(repo.path())?;
    assert_ne!(
        out.status.code(),
        Some(0),
        "println! outside #[cfg(debug_assertions)] should fail\nstdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("production output"),
        "should report the production print offender\nstdout: {stdout}"
    );
    Ok(())
}

/// Verifies that a combined function-level allow attribute permits intentional output.
#[test]
fn check_print_in_lib_allows_combined_function_print_allow() -> TestResult {
    let repo = TempRepo::new("combined-function-allow")?;
    repo.write_baseline(0)?;
    repo.write_crate_src(
        "my-crate",
        "lib.rs",
        r#"
#[allow(clippy::print_stderr, clippy::print_stdout)]
pub fn startup_banner() {
    println!("starting");
    eprintln!("ready");
}
"#,
    )?;

    let out = run_check_print_in_lib(repo.path())?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "combined function-level print allow should pass\nstdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    Ok(())
}

/// Verifies that documented print examples are not treated as production output.
#[test]
fn check_print_in_lib_ignores_doc_comment_examples() -> TestResult {
    let repo = TempRepo::new("doc-comment-example")?;
    repo.write_baseline(0)?;
    repo.write_crate_src(
        "my-crate",
        "lib.rs",
        r#"
//! ```rust
//! println!("example output");
//! ```

/// Example:
/// println!("example output");
pub fn documented() {}
"#,
    )?;

    let out = run_check_print_in_lib(repo.path())?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "doc comment print examples should pass\nstdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    Ok(())
}

/// Verifies that the baseline ratchet works: count <= baseline passes, count > baseline fails.
#[test]
fn check_print_in_lib_baseline_ratchet_allows_below_baseline() -> TestResult {
    let repo = TempRepo::new("below-baseline")?;
    // Set baseline higher than actual violations (0) — should pass and suggest ratcheting.
    repo.write_baseline(5)?;
    repo.write_crate_src("my-crate", "lib.rs", "pub fn ok() {}\n")?;

    let out = run_check_print_in_lib(repo.path())?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "should pass when count is below baseline\nstdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("NOTE") || stdout.contains("below baseline"),
        "should note that count is below baseline\nstdout: {stdout}"
    );
    Ok(())
}
