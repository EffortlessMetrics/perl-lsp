use color_eyre::eyre::{Context, Result, bail};
use regex::Regex;
use std::process::Command;

use crate::utils::project_root;

const FROM_RAW_PATTERN: &str = r"\b([A-Za-z_][A-Za-z0-9_:]*::)?ExitStatus::from_raw\(";
const ALLOWED_FROM_RAW_PATTERN: &str = r"::from_raw\(\s*raw[_ ]?exit\s*\(";

fn source_fragment(line: &str) -> &str {
    line.splitn(3, ':').nth(2).unwrap_or(line)
}

fn is_comment_line(fragment: &str) -> bool {
    let trimmed = fragment.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*')
}

fn match_inside_double_quotes(fragment: &str, match_start: usize) -> bool {
    let mut in_string = false;
    let mut escaped = false;

    for ch in fragment[..match_start].chars() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            _ => {}
        }
    }

    in_string
}

fn is_disallowed_from_raw_line(line: &str, disallow_re: &Regex, allowed_re: &Regex) -> bool {
    let fragment = source_fragment(line);
    if is_comment_line(fragment) || allowed_re.is_match(fragment) {
        return false;
    }

    let Some(mat) = disallow_re.find(fragment) else {
        return false;
    };

    !match_inside_double_quotes(fragment, mat.start())
}

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
        .filter(|line| is_disallowed_from_raw_line(line, &disallow_re, &allowed_re))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_doc_comment_mentions() {
        let disallow_re = Regex::new(FROM_RAW_PATTERN).expect("valid regex");
        let allowed_re = Regex::new(ALLOWED_FROM_RAW_PATTERN).expect("valid regex");
        let line = "xtask/src/main.rs:371:    /// Check for disallowed direct `ExitStatus::from_raw()` usage.";

        assert!(!is_disallowed_from_raw_line(line, &disallow_re, &allowed_re));
    }

    #[test]
    fn ignores_string_literal_mentions() {
        let disallow_re = Regex::new(FROM_RAW_PATTERN).expect("valid regex");
        let allowed_re = Regex::new(ALLOWED_FROM_RAW_PATTERN).expect("valid regex");
        let line = "xtask/src/tasks/ci_policy.rs:56:    bail!(\"CI policy check found disallowed ExitStatus::from_raw() usage\");";

        assert!(!is_disallowed_from_raw_line(line, &disallow_re, &allowed_re));
    }

    #[test]
    fn flags_real_from_raw_usage() {
        let disallow_re = Regex::new(FROM_RAW_PATTERN).expect("valid regex");
        let allowed_re = Regex::new(ALLOWED_FROM_RAW_PATTERN).expect("valid regex");
        let line = "src/lib.rs:10:    let status = std::process::ExitStatus::from_raw(raw_status);";

        assert!(is_disallowed_from_raw_line(line, &disallow_re, &allowed_re));
    }

    #[test]
    fn allows_raw_exit_adapter_usage() {
        let disallow_re = Regex::new(FROM_RAW_PATTERN).expect("valid regex");
        let allowed_re = Regex::new(ALLOWED_FROM_RAW_PATTERN).expect("valid regex");
        let line =
            "src/lib.rs:10:    let status = std::process::ExitStatus::from_raw(raw_exit(signal));";

        assert!(!is_disallowed_from_raw_line(line, &disallow_re, &allowed_re));
    }
}
