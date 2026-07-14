//! Regression tests for #2074 — extensionless files must not slip into
//! Rust-source walks.
//!
//! Each test exercises a different `walk_rs_files`-based code path in the binary
//! so that the --tests coverage profile covers the changed main.rs call sites.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

// ── shared helpers ────────────────────────────────────────────────────────────

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn new(label: &str) -> TestResult<Self> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = env::temp_dir()
            .join(format!("perl-ci-hygiene-walk-{label}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path)?;
        fs::write(path.join("Cargo.toml"), "[workspace]\n")?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    /// Place extensionless files at the repo root and inside crates/.
    /// These must NOT be processed as Rust sources (#2074 regression guard).
    fn seed_extensionless_files(&self) -> TestResult<()> {
        fs::write(self.path.join("README"), "readme")?;
        fs::write(self.path.join("Makefile"), "all:\n\t@echo ok")?;
        fs::write(self.path.join("LICENSE"), "")?;
        let crates = self.path.join("crates");
        fs::create_dir_all(&crates)?;
        fs::write(crates.join("README"), "readme")?;
        fs::write(crates.join("Makefile"), "all:")?;
        Ok(())
    }

    /// Create a minimal .rs source file under crates/<name>/src/<file>.
    fn write_rs_src(&self, crate_name: &str, file: &str, content: &str) -> TestResult<()> {
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

fn binary() -> TestResult<PathBuf> {
    env::var_os("CARGO_BIN_EXE_perl-ci-hygiene").map(PathBuf::from).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "CARGO_BIN_EXE_perl-ci-hygiene was not set by cargo",
        )
        .into()
    })
}

fn run(repo: &Path, args: &[&str]) -> TestResult<Output> {
    Ok(Command::new(binary()?).args(args).current_dir(repo).output()?)
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// `check-print-in-lib` exercises `walk_rust_source_files_for_ci_checks`.
///
/// Puts extensionless files (README, Makefile) in crates/ alongside a clean
/// .rs file.  The command must exit 0 — extensionless files must not be treated
/// as Rust sources and must not cause any parse/detection failure.
#[test]
fn check_print_in_lib_ignores_extensionless_files() -> TestResult {
    let repo = TempRepo::new("print-in-lib-ext")?;
    repo.seed_extensionless_files()?;
    repo.write_rs_src("my-crate", "lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }\n")?;

    // baseline must exist (0 print offenders)
    let ci = repo.path().join("ci");
    fs::create_dir_all(&ci)?;
    fs::write(ci.join("print_in_lib_baseline.txt"), "0\n")?;

    let out = run(repo.path(), &["check-print-in-lib"])?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "check-print-in-lib must pass with extensionless files present\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    Ok(())
}

/// `check-unwraps-modules` exercises `run_module_ratchet` → `walk_rs_files`.
///
/// Creates the `crates/perl-parser/src/lsp/server_impl/` directory (which the
/// ratchet targets) with a clean .rs file and extensionless neighbours.  The
/// ratchet must walk only the .rs file, find zero unwrap offenders, and exit 0.
#[test]
fn check_unwraps_modules_ignores_extensionless_files() -> TestResult {
    let repo = TempRepo::new("unwraps-modules-ext")?;
    repo.seed_extensionless_files()?;

    // Seed the directory that cmd_check_unwraps_modules targets.
    let server_impl =
        repo.path().join("crates").join("perl-parser").join("src").join("lsp").join("server_impl");
    fs::create_dir_all(&server_impl)?;
    fs::write(
        server_impl.join("handler.rs"),
        "pub fn handle() -> Result<(), String> { Ok(()) }\n",
    )?;
    // Extensionless file inside the targeted directory (must be ignored).
    fs::write(server_impl.join("README"), "internal readme")?;

    // ci/ baselines are auto-created when absent; just ensure the dir exists.
    fs::create_dir_all(repo.path().join("ci"))?;

    let out = run(repo.path(), &["check-unwraps-modules"])?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "check-unwraps-modules must pass with extensionless files present\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    Ok(())
}

/// `ignored-test-count` exercises `collect_ignored_matches` → `walk_rs_files`.
///
/// Creates a crate with a .rs file that has no `#[ignore]` attributes alongside
/// extensionless files.  The command must complete without error (exit 0).
#[test]
fn ignored_test_count_ignores_extensionless_files() -> TestResult {
    let repo = TempRepo::new("ignored-count-ext")?;
    repo.seed_extensionless_files()?;
    repo.write_rs_src("my-crate", "lib.rs", "#[test]\nfn passes() { assert_eq!(1, 1); }\n")?;

    // Run without --update or --check so no baseline file is needed.
    let out = run(repo.path(), &["ignored-test-count"])?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "ignored-test-count must complete with extensionless files present\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    Ok(())
}

/// `check-p0-locks` exercises `walk_rust_sources` → `walk_rs_files`.
///
/// Creates the `crates/perl-parser/src/lsp/server_impl/` directory that the
/// command specifically targets, putting both a clean .rs file and extensionless
/// files inside.  The walk must process only the .rs file and exit 0 (no unsafe
/// lock patterns found).
#[test]
fn check_p0_locks_ignores_extensionless_files() -> TestResult {
    let repo = TempRepo::new("p0-locks-ext")?;
    repo.seed_extensionless_files()?;

    let server_impl =
        repo.path().join("crates").join("perl-parser").join("src").join("lsp").join("server_impl");
    fs::create_dir_all(&server_impl)?;
    fs::write(
        server_impl.join("handler.rs"),
        "pub fn safe_handle(mu: &std::sync::Mutex<i32>) -> i32 {\n\
         let guard = mu.lock().unwrap_or_else(|e| e.into_inner());\n\
         *guard\n}\n",
    )?;
    // Extensionless file inside the targeted directory (must be ignored).
    fs::write(server_impl.join("README"), "server internals")?;

    let out = run(repo.path(), &["check-p0-locks"])?;
    assert_eq!(
        out.status.code(),
        Some(0),
        "check-p0-locks must pass with clean code and extensionless files\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    Ok(())
}
