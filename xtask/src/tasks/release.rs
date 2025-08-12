//! Release task implementation

use color_eyre::eyre::{Result, bail};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run(version: String, yes: bool) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );

    // Check prerequisites
    spinner.set_message("Checking prerequisites...");
    check_prerequisites()?;

    // Check for uncommitted changes
    if !check_git_status()? {
        bail!("You have uncommitted changes. Please commit or stash them before releasing.");
    }

    // Confirm release
    if !yes {
        println!("🚀 Preparing release v{}", version);
        println!("This will:");
        println!("  - Build release binaries");
        println!("  - Package VSCode extension");
        println!("  - Run tests");
        println!("  - Create release artifacts");
        println!();
        print!("Continue? [y/N] ");
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Release cancelled.");
            return Ok(());
        }
    }

    // Create release directory
    let release_dir = Path::new("release");
    if release_dir.exists() {
        fs::remove_dir_all(release_dir)?;
    }
    fs::create_dir_all(release_dir.join("binaries"))?;

    // Build release binaries
    spinner.set_message("Building release binaries...");
    build_binaries(&release_dir)?;
    spinner.finish_with_message("✅ Binaries built");

    // Package VSCode extension
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );
    spinner.set_message("Packaging VSCode extension...");
    package_vscode_extension(&release_dir)?;
    spinner.finish_with_message("✅ VSCode extension packaged");

    // Run tests
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );
    spinner.set_message("Running tests...");
    run_tests()?;
    spinner.finish_with_message("✅ Tests passed");

    // Create checksums
    create_checksums(&release_dir)?;

    // Create release archive
    create_release_archive(&release_dir, &version)?;

    println!();
    println!("✅ Release preparation complete!");
    println!("================================================");
    println!("Release artifacts in: release/");
    println!();

    // List artifacts
    let output = Command::new("ls").arg("-lh").arg("release/").output()?;
    println!("{}", String::from_utf8_lossy(&output.stdout));

    println!("📋 Next steps:");
    println!("1. Review the release artifacts");
    println!(
        "2. Create git tag: git tag -a v{} -m 'Release v{}'",
        version, version
    );
    println!("3. Push tag: git push origin v{}", version);
    println!("4. Create GitHub release and upload artifacts");
    println!("5. Run: cargo xtask publish-crates");
    println!("6. Run: cargo xtask publish-vscode");

    Ok(())
}

fn check_prerequisites() -> Result<()> {
    // Check for required tools
    let tools = ["cargo", "strip", "tar", "sha256sum", "npx"];
    for tool in &tools {
        if !command_exists(tool) {
            bail!("Required tool '{}' not found", tool);
        }
    }
    Ok(())
}

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn check_git_status() -> Result<bool> {
    let output = Command::new("git")
        .args(&["diff-index", "--quiet", "HEAD", "--"])
        .output()?;
    Ok(output.status.success())
}

fn build_binaries(release_dir: &Path) -> Result<()> {
    // Build perl-lsp
    let output = Command::new("cargo")
        .args(&[
            "build",
            "--release",
            "-p",
            "perl-parser",
            "--bin",
            "perl-lsp",
        ])
        .output()?;
    if !output.status.success() {
        bail!(
            "Failed to build perl-lsp: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Build perl-dap
    let output = Command::new("cargo")
        .args(&[
            "build",
            "--release",
            "-p",
            "perl-parser",
            "--bin",
            "perl-dap",
        ])
        .output()?;
    if !output.status.success() {
        bail!(
            "Failed to build perl-dap: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Copy binaries
    fs::copy(
        "target/release/perl-lsp",
        release_dir.join("binaries/perl-lsp"),
    )?;
    fs::copy(
        "target/release/perl-dap",
        release_dir.join("binaries/perl-dap"),
    )?;

    // Strip binaries
    Command::new("strip")
        .arg(release_dir.join("binaries/perl-lsp"))
        .output()?;
    Command::new("strip")
        .arg(release_dir.join("binaries/perl-dap"))
        .output()?;

    Ok(())
}

fn package_vscode_extension(release_dir: &Path) -> Result<()> {
    // Change to vscode-extension directory
    let output = Command::new("npm")
        .current_dir("vscode-extension")
        .arg("install")
        .output()?;
    if !output.status.success() {
        bail!(
            "Failed to install dependencies: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = Command::new("npm")
        .current_dir("vscode-extension")
        .args(&["run", "compile"])
        .output()?;
    if !output.status.success() {
        bail!(
            "Failed to compile extension: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = Command::new("npx")
        .current_dir("vscode-extension")
        .args(&["vsce", "package"])
        .output()?;
    if !output.status.success() {
        bail!(
            "Failed to package extension: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Move VSIX to release directory
    for entry in fs::read_dir("vscode-extension")? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("vsix") {
            fs::rename(&path, release_dir.join(path.file_name().unwrap()))?;
        }
    }

    Ok(())
}

fn run_tests() -> Result<()> {
    // Run test_lsp_features.sh
    if Path::new("test_lsp_features.sh").exists() {
        let output = Command::new("./test_lsp_features.sh").output()?;
        if !output.status.success() {
            bail!("LSP tests failed");
        }
    }
    Ok(())
}

fn create_checksums(release_dir: &Path) -> Result<()> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cd {} && sha256sum binaries/* *.vsix > checksums.txt",
            release_dir.display()
        ))
        .output()?;
    if !output.status.success() {
        bail!("Failed to create checksums");
    }
    Ok(())
}

fn create_release_archive(release_dir: &Path, version: &str) -> Result<()> {
    let output = Command::new("tar")
        .current_dir(release_dir.join("binaries"))
        .args(&[
            "-czf",
            &format!("../perl-lsp-{}-linux-x64.tar.gz", version),
            "perl-lsp",
            "perl-dap",
        ])
        .output()?;
    if !output.status.success() {
        bail!("Failed to create release archive");
    }
    Ok(())
}
