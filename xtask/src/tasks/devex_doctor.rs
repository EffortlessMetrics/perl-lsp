//! Developer doctor task.
//! Mirrors `scripts/devex-doctor.sh` checks with native Rust execution.

use color_eyre::eyre::{Context, Result, bail};
use std::{
    env, fs,
    path::Path,
    process::{Command, Stdio},
};

pub fn run() -> Result<()> {
    let root = env::current_dir().context("failed to determine current directory")?;
    let root = root.to_string_lossy();

    let mut missing_required = false;

    println!("Repository: {root}");
    println!();
    println!("== Required ==");

    check_command("cargo", "cargo", &mut missing_required);
    check_command("rustfmt", "rustfmt", &mut missing_required);
    check_command("rustup", "rustup", &mut missing_required);

    show_version("rustc", "rustc", &["--version"]);
    show_version("cargo", "cargo", &["--version"]);

    println!();
    println!("== Recommended ==");
    check_command_optional("just", "just");
    check_command_optional("nix", "nix");
    check_command_optional("git", "git");
    check_command_optional("rg", "rg");
    check_command_optional("cargo-audit", "cargo-audit");

    println!();
    println!("== Rust components ==");
    if has_command("rustup") {
        let installed_components = get_installed_rustup_components();

        if is_component_installed("rustfmt", installed_components.as_deref()) {
            pass("rustup component installed: rustfmt");
        } else {
            warn("rustup component missing: rustfmt (install: rustup component add rustfmt)");
        }

        if is_component_installed("clippy", installed_components.as_deref()) {
            pass("rustup component installed: clippy");
        } else {
            warn("rustup component missing: clippy (install: rustup component add clippy)");
        }
    } else {
        warn("rustup unavailable; cannot verify components");
    }

    println!();
    println!("== Git hooks ==");
    check_pre_push_hook();
    check_pre_commit_hook();

    println!();
    if Path::new("rust-toolchain.toml").exists() {
        let status = Command::new("bash")
            .arg("scripts/check-rust-toolchain.sh")
            .arg("doctor")
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context("failed to run scripts/check-rust-toolchain.sh")?;
        if !status.success() {
            missing_required = true;
        }
    } else {
        warn("rust-toolchain.toml not found");
    }

    println!();
    println!("== Suggested next commands ==");
    println!("  just devex            # quick environment diagnostics");
    println!("  just pr-fast          # fast validation before a full gate");
    println!("  just ci-gate          # repo-native local gate");
    println!("  nix develop -c just ci-gate");

    if missing_required {
        fail("Missing required tools. Install Rust via https://rustup.rs");
        bail!("required checks did not pass");
    }

    println!();
    pass("Doctor completed: required tooling is available");

    Ok(())
}

fn check_command(program: &str, label: &str, missing_required: &mut bool) {
    if has_command(program) {
        pass(&format!("{label}: found ({})", command_path(program).unwrap_or(program.to_string())));
    } else {
        warn(&format!("{label}: not found{}", install_hint(program)));
        *missing_required = true;
    }
}

fn check_command_optional(program: &str, label: &str) {
    if has_command(program) {
        pass(&format!("{label}: found ({})", command_path(program).unwrap_or(program.to_string())));
    } else {
        warn(&format!("{label}: not found{}", install_hint(program)));
    }
}

fn install_hint(program: &str) -> &'static str {
    match program {
        "cargo" | "rustfmt" | "rustup" => " (install via https://rustup.rs)",
        "just" => " (install: cargo install just)",
        "cargo-audit" => " (install: cargo install cargo-audit)",
        "rg" => " (install ripgrep via your package manager)",
        "nix" => " (install from https://nixos.org/download/)",
        _ => "",
    }
}

fn has_command(program: &str) -> bool {
    if Path::new(program).exists() {
        return true;
    }

    env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|path| {
            #[cfg(windows)]
            {
                let mut candidate = path.join(format!("{program}.exe"));
                if candidate.exists() {
                    return true;
                }
                candidate = path.join(format!("{program}.bat"));
                if candidate.exists() {
                    return true;
                }
                path.join(program).exists()
            }

            #[cfg(not(windows))]
            {
                path.join(program).exists()
            }
        })
    })
}

fn command_path(program: &str) -> Option<String> {
    if Path::new(program).is_file() {
        return Some(program.to_string());
    }
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|path| path.join(program))
            .find(|candidate| candidate.is_file())
            .map(|candidate| candidate.to_string_lossy().to_string())
    })
}

fn get_installed_rustup_components() -> Option<String> {
    let output = Command::new("rustup").args(["component", "list", "--installed"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

fn is_component_installed(component: &str, installed_components: Option<&str>) -> bool {
    let Some(lines) = installed_components else {
        return false;
    };
    lines.lines().any(|line| {
        let value = line.split_whitespace().next().unwrap_or("");
        value == component || value.starts_with(&(format!("{component}-")))
    })
}

fn show_version(program: &str, command: &str, args: &[&str]) {
    match Command::new(command).args(args).output() {
        Ok(output) if output.status.success() => {
            let output = String::from_utf8_lossy(&output.stdout);
            let first_line = output.lines().next().unwrap_or("");
            pass(&format!("{program} version: {first_line}"));
        }
        _ => warn(&format!("{program} version check failed")),
    }
}

fn check_pre_push_hook() {
    if !has_command("git") {
        warn("git unavailable; cannot verify pre-push hook");
        return;
    }

    let repo_root = git_output(&["rev-parse", "--show-toplevel"]);
    let git_common_dir = git_output(&["rev-parse", "--git-common-dir"]);
    let (repo_root, git_common_dir) = match (repo_root, git_common_dir) {
        (Some(root), Some(common)) => (root, common),
        _ => {
            warn("not in a git repository; cannot verify pre-push hook");
            return;
        }
    };

    let hook_path = Path::new(&git_common_dir).join("hooks").join("pre-push");
    let expected_hook = Path::new(&repo_root).join("hooks").join("pre-push");

    if !hook_path.is_file() {
        warn("pre-push hook not installed (fix: cargo xtask ci-hygiene install-githooks)");
        return;
    }

    if !is_executable(&hook_path) {
        warn(&format!("pre-push hook present but not executable: {}", hook_path.display()));
        return;
    }

    if expected_hook.is_file() {
        let installed = fs::read_to_string(&hook_path);
        let expected = fs::read_to_string(&expected_hook);
        match (installed, expected) {
            (Ok(installed), Ok(expected)) => {
                if normalize_hook_text(&installed) == normalize_hook_text(&expected) {
                    pass("pre-push hook installed and current");
                } else {
                    warn(&format!(
                        "pre-push hook installed but stale: {} (fix: cargo xtask ci-hygiene install-githooks)",
                        hook_path.display()
                    ));
                }
            }
            _ => {
                warn("unable to read pre-push hook content for staleness check");
            }
        }
        return;
    }

    pass("pre-push hook installed");
}

fn check_pre_commit_hook() {
    if !has_command("git") {
        return;
    }

    let git_common_dir = match git_output(&["rev-parse", "--git-common-dir"]) {
        Some(dir) => dir,
        None => {
            warn("not in a git repository; cannot verify pre-commit hook");
            return;
        }
    };

    let hook_path = Path::new(&git_common_dir).join("hooks").join("pre-commit");

    if !hook_path.is_file() {
        warn("pre-commit hook missing or not executable (run: cargo xtask ci-hygiene install-githooks)");
        return;
    }

    if !is_executable(&hook_path) {
        warn(&format!(
            "pre-commit hook present but not executable: {} (run: cargo xtask ci-hygiene install-githooks)",
            hook_path.display()
        ));
        return;
    }

    pass(&format!("git hook installed: {}", hook_path.display()));
}

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn normalize_hook_text(content: &str) -> String {
    let mut lines: Vec<String> =
        content.lines().map(|line| line.trim_end_matches('\r').to_string()).collect();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn pass(message: &str) {
    println!("✅ {message}");
}

fn warn(message: &str) {
    println!("⚠️  {message}");
}

fn fail(message: &str) {
    println!("❌ {message}");
}

#[cfg(test)]
mod tests {
    use super::normalize_hook_text;

    #[test]
    fn normalize_hook_text_removes_crlf_and_trailing_blank_lines() {
        let input = "#!/usr/bin/env bash\r\nset -eu\r\n\r\n\r\n";
        let normalized = normalize_hook_text(input);
        assert_eq!(normalized, "#!/usr/bin/env bash\nset -eu");
    }

    #[test]
    fn normalize_hook_text_preserves_internal_blank_lines() {
        let input = "line1\n\nline3\n";
        let normalized = normalize_hook_text(input);
        assert_eq!(normalized, "line1\n\nline3");
    }
}
