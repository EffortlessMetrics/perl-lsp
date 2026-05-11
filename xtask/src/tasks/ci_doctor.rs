//! CI doctor: local/CI parity diagnostic.
//!
//! `cargo xtask ci doctor` — fast (<10 s) environment check that shows
//! whether the local setup matches what CI expects.  Exits non-zero only
//! on hard failures; advisory checks emit warnings and keep exit 0.
//!
//! ## Checks (v1)
//! 1. rustc version matches `rust-toolchain.toml` channel
//! 2. `rustfmt` component installed
//! 3. `clippy` component installed
//! 4. working tree clean (warns on untracked-only)
//! 5. fmt drift (`cargo xtask fmt --check`, advisory)
//! 6. Perl interpreter available
//! 7. `perl-lsp` release binary present (advisory)
//! 8. platform notes

use color_eyre::eyre::{Result, bail};
use serde::Deserialize;
use std::{env, fs, path::Path, process::Command};

use crate::utils::project_root;

// ── output helpers ────────────────────────────────────────────────────────────

fn pass(msg: &str) {
    println!("✅ {msg}");
}

fn warn(msg: &str) {
    println!("⚠️  {msg}");
}

fn fail(msg: &str) {
    println!("❌ {msg}");
}

fn section(title: &str) {
    println!("── {title} ──");
}

// ── TOML deserialization for rust-toolchain.toml ──────────────────────────────

#[derive(Deserialize)]
struct RustToolchainFile {
    toolchain: RustToolchain,
}

#[derive(Deserialize)]
struct RustToolchain {
    channel: String,
}

fn read_pinned_channel(root: &Path) -> Option<String> {
    let path = root.join("rust-toolchain.toml");
    let raw = fs::read_to_string(&path).ok()?;
    let file: RustToolchainFile = toml::from_str(&raw).ok()?;
    let channel = file.toolchain.channel.trim().trim_matches('"').trim_matches('\'').to_string();
    if channel.is_empty() { None } else { Some(channel) }
}

fn installed_rustc_version() -> Option<String> {
    let output = Command::new("rustc").arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    // "rustc 1.95.0 (abc123 2026-05-01)" → "1.95.0"
    let version = text.split_whitespace().nth(1)?;
    Some(version.to_string())
}

// ── individual checks ─────────────────────────────────────────────────────────

fn check_toolchain(root: &Path, failures: &mut usize) {
    section("Toolchain");

    match (read_pinned_channel(root), installed_rustc_version()) {
        (None, _) => {
            warn("rust-toolchain.toml not found or unparseable; skipping channel check");
        }
        (_, None) => {
            fail("rustc not found or --version failed");
            *failures += 1;
        }
        (Some(pinned), Some(installed)) => {
            if installed == pinned
                || installed.starts_with(&pinned)
                || pinned.starts_with(&installed)
            {
                pass(&format!("rustc {installed} matches pinned channel {pinned}"));
            } else {
                warn(&format!(
                    "rustc {installed} differs from pinned channel {pinned}; run: rustup override set {pinned}"
                ));
            }
        }
    }
}

fn get_installed_components() -> Option<String> {
    let output = Command::new("rustup").args(["component", "list", "--installed"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

fn is_component_present(component: &str, installed: &str) -> bool {
    installed.lines().any(|line| {
        let name = line.split_whitespace().next().unwrap_or("");
        name == component || name.starts_with(&format!("{component}-"))
    })
}

fn check_rust_components(failures: &mut usize) {
    section("Rust components");

    let installed = get_installed_components();

    for component in &["rustfmt", "clippy"] {
        match &installed {
            None => {
                warn(&format!(
                    "rustup unavailable; cannot verify {component} — install rustup from https://rustup.rs"
                ));
            }
            Some(list) => {
                if is_component_present(component, list) {
                    pass(&format!("{component} component installed"));
                } else {
                    fail(&format!(
                        "{component} component missing — fix: rustup component add {component}"
                    ));
                    *failures += 1;
                }
            }
        }
    }
}

/// Returns: (staged+unstaged count, untracked-only)
fn git_porcelain_status() -> Option<(usize, bool)> {
    let output = Command::new("git").args(["status", "--porcelain"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return Some((0, false));
    }
    let untracked_only = lines.iter().all(|l| l.starts_with("??"));
    Some((lines.len(), untracked_only))
}

fn check_git_state(warnings: &mut usize) {
    section("Working tree");

    match git_porcelain_status() {
        None => warn("git not available or not in a repo; cannot check working tree"),
        Some((0, _)) => pass("working tree clean"),
        Some((count, true)) => {
            warn(&format!("{count} untracked file(s) — staged/unstaged: 0 (usually fine for CI)"));
            *warnings += 1;
        }
        Some((count, false)) => {
            warn(&format!("{count} modified/staged file(s) — commit or stash before pushing"));
            *warnings += 1;
        }
    }
}

fn check_fmt_drift(root: &Path, warnings: &mut usize) {
    section("Format drift (advisory)");

    // Run `cargo xtask fmt --check` from the workspace root.
    // This is advisory: we warn but do not fail ci-doctor.
    let current_exe = env::current_exe().ok();
    let xtask_bin = current_exe.as_deref().unwrap_or(Path::new("cargo-xtask"));

    let status = Command::new(xtask_bin).current_dir(root).args(["fmt", "--check"]).status();

    match status {
        Ok(s) if s.success() => pass("no fmt drift"),
        Ok(_) => {
            warn("fmt drift detected — fix: cargo xtask fmt");
            *warnings += 1;
        }
        Err(_) => {
            // Fall back to plain cargo fmt --check
            let fallback = Command::new("cargo")
                .current_dir(root)
                .args(["fmt", "--all", "--", "--check"])
                .status();
            match fallback {
                Ok(s) if s.success() => pass("no fmt drift"),
                Ok(_) => {
                    warn("fmt drift detected — fix: cargo fmt --all");
                    *warnings += 1;
                }
                Err(_) => warn("cargo fmt unavailable; skipping drift check"),
            }
        }
    }
}

fn check_perl(failures: &mut usize) {
    let output = Command::new("perl").arg("-v").output();
    match output {
        Err(_) => {
            fail("perl not found — install perl (needed for perl-lsp features)");
            *failures += 1;
        }
        Ok(out) if !out.status.success() => {
            fail("perl -v exited non-zero");
            *failures += 1;
        }
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            // Extract "v5.xx.y" from first line
            let version = text
                .lines()
                .flat_map(|l| l.split_whitespace())
                .find(|t| t.starts_with("v5.") || t.starts_with("(v5."))
                .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric() && c != '.'))
                .unwrap_or("unknown");
            pass(&format!("perl available ({version})"));
        }
    }
}

fn check_binary(root: &Path, warnings: &mut usize) {
    let binary = if cfg!(windows) { "perl-lsp.exe" } else { "perl-lsp" };
    let binary_path = root.join("target").join("release").join(binary);

    if binary_path.is_file() {
        pass(&format!("release binary present: {}", binary_path.display()));
    } else {
        warn(&format!(
            "release binary missing: {} — build with: cargo build -p perl-lsp-rs --release",
            binary_path.display()
        ));
        *warnings += 1;
    }
}

fn print_platform_notes(warnings: &mut usize) {
    section("Platform");

    let platform = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };

    println!("   platform: {platform}");

    #[cfg(windows)]
    {
        warn(
            "Windows: CI runs on Linux — path-length limits and CRLF line endings may cause divergence",
        );
        *warnings += 1;
        warn("Windows: ensure git core.autocrlf = false for consistent diffs");
        *warnings += 1;
    }

    #[cfg(not(windows))]
    {
        let _ = warnings; // suppress unused warning
        if cfg!(target_os = "macos") {
            println!(
                "   macOS: CI runs on Linux; filesystem is case-insensitive here vs case-sensitive on CI"
            );
        } else {
            println!("   linux: matches CI platform");
        }
    }
}

fn summarize(failures: usize, warnings: usize) -> Result<()> {
    println!();
    if failures > 0 {
        fail(&format!(
            "ci doctor: {failures} hard failure(s), {warnings} warning(s) — local env diverges from CI"
        ));
        bail!("{failures} required check(s) failed");
    } else if warnings > 0 {
        warn(&format!(
            "ci doctor: 0 failures, {warnings} warning(s) — minor divergence, CI likely passes"
        ));
    } else {
        pass("ci doctor: all checks passed — local env matches CI");
    }
    Ok(())
}

// ── public entry point ────────────────────────────────────────────────────────

pub fn run() -> Result<()> {
    let root = project_root()?;
    let mut failures = 0usize;
    let mut warnings = 0usize;

    println!("=== cargo xtask ci doctor ===");
    println!("Workspace: {}", root.display());

    println!();
    check_toolchain(&root, &mut failures);

    println!();
    check_rust_components(&mut failures);

    println!();
    check_git_state(&mut warnings);

    println!();
    check_fmt_drift(&root, &mut warnings);

    println!();
    check_perl(&mut failures);
    check_binary(&root, &mut warnings);

    println!();
    print_platform_notes(&mut warnings);

    summarize(failures, warnings)
}

// ── unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn ci_doctor_read_pinned_channel_parses_valid_toml() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("rust-toolchain.toml"), "[toolchain]\nchannel = \"1.95.0\"\n")
            .expect("write");
        let channel = read_pinned_channel(dir.path());
        assert_eq!(channel.as_deref(), Some("1.95.0"));
    }

    #[test]
    fn ci_doctor_read_pinned_channel_returns_none_for_missing_file() {
        let dir = tempdir().expect("tempdir");
        assert!(read_pinned_channel(dir.path()).is_none());
    }

    #[test]
    fn ci_doctor_read_pinned_channel_returns_none_for_invalid_toml() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("rust-toolchain.toml"), "not valid toml {{{{").expect("write");
        assert!(read_pinned_channel(dir.path()).is_none());
    }

    #[test]
    fn ci_doctor_is_component_present_finds_exact_match() {
        let installed = "rustfmt-x86_64-unknown-linux-gnu\ncargo\nclipy\n";
        assert!(is_component_present("rustfmt", installed));
        assert!(!is_component_present("clippy", installed));
    }

    #[test]
    fn ci_doctor_is_component_present_finds_bare_name() {
        let installed = "rustfmt\nclipy\n";
        assert!(is_component_present("rustfmt", installed));
    }

    #[test]
    fn ci_doctor_git_porcelain_untracked_only_detection() {
        // When all lines start with '??' it's untracked-only
        let lines = vec!["?? foo.rs", "?? bar.rs"];
        let all_untracked = lines.iter().all(|l| l.starts_with("??"));
        assert!(all_untracked);

        let mixed = vec!["M  foo.rs", "?? bar.rs"];
        let mixed_untracked = mixed.iter().all(|l| l.starts_with("??"));
        assert!(!mixed_untracked);
    }

    #[test]
    fn ci_doctor_summarize_returns_ok_when_no_failures() {
        assert!(summarize(0, 0).is_ok());
        assert!(summarize(0, 3).is_ok());
    }

    #[test]
    fn ci_doctor_summarize_returns_err_when_failures() {
        assert!(summarize(1, 0).is_err());
        assert!(summarize(2, 1).is_err());
    }
}
