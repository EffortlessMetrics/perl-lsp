use color_eyre::eyre::{Context, Result, bail};
use regex::Regex;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use crate::utils::project_root;

const FROM_RAW_PATTERN: &str = r"\b([A-Za-z_][A-Za-z0-9_:]*::)?ExitStatus::from_raw\(";
const ALLOWED_FROM_RAW_PATTERN: &str = r"::from_raw\(\s*raw[_ ]?exit\s*\(";
const SEARCH_ROOTS: &[&str] = &["crates", "xtask", "examples", "tests"];

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

fn should_skip_dir(path: &Path) -> bool {
    path.file_name().is_some_and(|name| matches!(name.to_str(), Some("target" | "generated")))
}

fn collect_candidate_lines(root: &Path, disallow_re: &Regex) -> Result<Vec<String>> {
    let mut candidates = Vec::new();

    for relative_root in SEARCH_ROOTS {
        let search_root = root.join(relative_root);
        if !search_root.exists() {
            continue;
        }

        for entry in WalkDir::new(&search_root)
            .into_iter()
            .filter_entry(|entry| !(entry.file_type().is_dir() && should_skip_dir(entry.path())))
        {
            let entry =
                entry.with_context(|| format!("failed to walk {}", search_root.display()))?;
            if !entry.file_type().is_file()
                || entry.path().extension().is_none_or(|ext| ext != "rs")
            {
                continue;
            }

            let contents = fs::read_to_string(entry.path())
                .with_context(|| format!("failed to read {}", entry.path().display()))?;
            let relative_path = entry.path().strip_prefix(root).unwrap_or(entry.path());

            for (line_number, line) in contents.lines().enumerate() {
                if disallow_re.is_match(line) {
                    candidates.push(format!(
                        "{}:{}:{}",
                        relative_path.display(),
                        line_number + 1,
                        line
                    ));
                }
            }
        }
    }

    Ok(candidates)
}

pub fn check_from_raw() -> Result<()> {
    let root = project_root()?;
    let disallow_re = Regex::new(FROM_RAW_PATTERN)?;
    let allowed_re = Regex::new(ALLOWED_FROM_RAW_PATTERN)?;
    let candidates = collect_candidate_lines(&root, &disallow_re)?;

    let violations: Vec<_> = candidates
        .iter()
        .map(String::as_str)
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
