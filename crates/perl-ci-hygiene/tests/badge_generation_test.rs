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
            .join(format!("perl-ci-hygiene-badge-{label}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path)?;
        fs::write(path.join("Cargo.toml"), "[workspace]\n")?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
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

fn write_publication_facts(repo: &Path, installs: u32) -> TestResult<()> {
    let facts_dir = repo.join("docs").join("project");
    fs::create_dir_all(&facts_dir)?;
    fs::write(
        facts_dir.join("publication-facts.toml"),
        format!(
            r#"[external]

[external.vscode_marketplace_installs]
label = "VS Marketplace installs"
value = {installs}
unit = "installs"
tier = "D"
verified_at = "2026-05-06"
source = "https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs"
"#
        ),
    )?;
    Ok(())
}

fn markdown_readme(count: u32) -> String {
    format!(
        r#"<!-- perl-lsp:vs-marketplace-installs-badge:start -->
[![VS Marketplace Installs (manual)](https://img.shields.io/badge/VS%20Marketplace-{count}%20installs-0078D4)](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs)
<!-- perl-lsp:vs-marketplace-installs-badge:end -->
"#
    )
}

fn html_readme(count: u32) -> String {
    format!(
        r#"<p>
  <!-- perl-lsp:vs-marketplace-installs-badge:start -->
  <a href="https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs"><img src="https://img.shields.io/badge/VS%20Marketplace-{count}%20installs-0078D4" alt="VS Marketplace installs" /></a>
  <!-- perl-lsp:vs-marketplace-installs-badge:end -->
</p>
"#
    )
}

fn write_badge_readmes(repo: &Path, root_count: u32, extension_count: u32) -> TestResult<()> {
    fs::write(repo.join("README.md"), markdown_readme(root_count))?;

    let extension_dir = repo.join("vscode-extension");
    fs::create_dir_all(&extension_dir)?;
    fs::write(extension_dir.join("README.md"), html_readme(extension_count))?;
    Ok(())
}

fn run_generate_badges(repo: &Path, args: &[&str]) -> TestResult<Output> {
    let mut command = Command::new(perl_ci_hygiene_binary()?);
    command.current_dir(repo).arg("generate-badges").args(args);
    Ok(command.output()?)
}

#[test]
fn generate_badges_check_reports_stale_and_expected_counts() -> TestResult {
    let repo = TempRepo::new("check-drift")?;
    write_publication_facts(repo.path(), 287)?;
    write_badge_readmes(repo.path(), 277, 287)?;

    let output = run_generate_badges(repo.path(), &["--check"])?;

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("expected installs: 287"));
    assert!(stderr.contains("stale badge found: 277 but expected 287"));
    assert!(stderr.contains("README.md"));
    Ok(())
}

#[test]
fn generate_badges_updates_root_and_extension_readmes() -> TestResult {
    let repo = TempRepo::new("update-two-readmes")?;
    write_publication_facts(repo.path(), 287)?;
    write_badge_readmes(repo.path(), 277, 277)?;

    let output = run_generate_badges(repo.path(), &[])?;

    assert!(output.status.success());
    let root_readme = fs::read_to_string(repo.path().join("README.md"))?;
    let extension_readme =
        fs::read_to_string(repo.path().join("vscode-extension").join("README.md"))?;
    assert!(root_readme.contains("287%20installs"));
    assert!(extension_readme.contains("287%20installs"));
    assert!(!root_readme.contains("277%20installs"));
    assert!(!extension_readme.contains("277%20installs"));
    Ok(())
}

#[test]
fn generate_badges_check_accepts_current_badges() -> TestResult {
    let repo = TempRepo::new("check-current")?;
    write_publication_facts(repo.path(), 287)?;
    write_badge_readmes(repo.path(), 287, 287)?;

    let output = run_generate_badges(repo.path(), &["--check"])?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("VS Marketplace badge check passed"));
    Ok(())
}
