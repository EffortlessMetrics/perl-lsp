//! Developer doctor task.
//! Mirrors `scripts/devex-doctor.sh` checks with native Rust execution.

use color_eyre::eyre::{Context, Result, bail};
use std::{
    env,
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
        if is_component_installed("rustfmt") {
            pass("rustup component installed: rustfmt");
        } else {
            warn("rustup component missing: rustfmt (install: rustup component add rustfmt)");
        }

        if is_component_installed("clippy") {
            pass("rustup component installed: clippy");
        } else {
            warn("rustup component missing: clippy (install: rustup component add clippy)");
        }
    } else {
        warn("rustup unavailable; cannot verify components");
    }

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
        pass(&format!(
            "{label}: found ({})",
            command_path(program).unwrap_or(program.to_string())
        ));
    } else {
        warn(&format!("{label}: not found"));
        *missing_required = true;
    }
}

fn check_command_optional(program: &str, label: &str) {
    if has_command(program) {
        pass(&format!(
            "{label}: found ({})",
            command_path(program).unwrap_or(program.to_string())
        ));
    } else {
        warn(&format!("{label}: not found"));
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

fn is_component_installed(component: &str) -> bool {
    let output = match Command::new("rustup")
        .args(["component", "list", "--installed"])
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return false,
    };
    let lines = String::from_utf8_lossy(&output.stdout);
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

fn pass(message: &str) {
    println!("✅ {message}");
}

fn warn(message: &str) {
    println!("⚠️  {message}");
}

fn fail(message: &str) {
    println!("❌ {message}");
}
