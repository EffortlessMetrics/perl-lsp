use color_eyre::eyre::{Context, Result, bail};
use regex::Regex;
use std::process::Command;

use crate::utils::project_root;

const FROM_RAW_PATTERN: &str = r"\b([A-Za-z_][A-Za-z0-9_:]*::)?ExitStatus::from_raw\(";
const ALLOWED_FROM_RAW_PATTERN: &str = r"::from_raw\(\s*raw[_ ]?exit\s*\(";

pub fn check_from_raw() -> Result<()> {
    let root = project_root()?;

    let output = Command::new("git")
        .current_dir(&root)
        .args([
            "grep",
            "-nE",
            FROM_RAW_PATTERN,
            "--",
            "crates/**/*.rs",
            "xtask/**/*.rs",
            "examples/**/*.rs",
            "tests/**/*.rs",
            ":!**/target/**",
            ":!**/generated/**",
        ])
        .output()
        .context("Failed to run git grep")?;

    if let Some(code) = output.status.code() {
        if code > 1 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git grep command failed with exit code {}: {}", code, stderr.trim(),);
        }
    }

    let output_text =
        String::from_utf8(output.stdout).context("git grep output was not valid UTF-8")?;

    let disallow_re = Regex::new(FROM_RAW_PATTERN)?;
    let allowed_re = Regex::new(ALLOWED_FROM_RAW_PATTERN)?;
    let violations: Vec<_> = output_text
        .lines()
        .filter(|line| disallow_re.is_match(line) && !allowed_re.is_match(line))
        .collect();

    if violations.is_empty() {
        println!("ExitStatus policy check passed");
        return Ok(());
    }

    for line in violations {
        eprintln!("::error::Disallowed direct from_raw(): {line}");
    }

    bail!("CI policy check found disallowed ExitStatus::from_raw() usage");
}
