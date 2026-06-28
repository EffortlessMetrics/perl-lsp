use color_eyre::eyre::Result;
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

use crate::{GREEN, NC, RED, YELLOW, display_path, path_has_component, read_lines, walk_entries};

static ABORT_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"std::process::abort\s*\("));
static EXIT_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"std::process::exit\s*\("));

pub(crate) fn forbid_fatal_constructs(repo_root: &Path, verbose: bool) -> Result<i32> {
    let abort_re = compiled_regex(&ABORT_RE, "std::process::abort")?;
    let exit_re = compiled_regex(&EXIT_RE, "std::process::exit")?;

    let mut aborts = Vec::new();
    let mut exits = Vec::new();

    let crates_root = repo_root.join("crates");
    for entry in walk_entries(&crates_root) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let rel = display_path(repo_root, path);
        if is_fatal_excluded(path, repo_root) {
            continue;
        }
        let lines = read_lines(path)?;
        for (line_no, line) in lines.iter().enumerate() {
            let number = line_no + 1;
            if abort_re.is_match(line) {
                aborts.push(format!("{rel}:{number}:{line}"));
            }
            if exit_re.is_match(line) {
                exits.push(format!("{rel}:{number}:{line}"));
            }
        }
    }

    report_abort_hits(&aborts);
    let exit_violations: Vec<String> =
        exits.into_iter().filter(|hit| !is_allowlisted_exit_hit(hit)).collect();
    report_exit_hits(&exit_violations);

    if (!aborts.is_empty()) || !exit_violations.is_empty() {
        return Ok(1);
    }

    if verbose {
        report_success();
    }
    Ok(0)
}

fn compiled_regex(
    regex: &'static LazyLock<Result<Regex, regex::Error>>,
    label: &str,
) -> Result<&'static Regex> {
    regex.as_ref().map_err(|err| color_eyre::eyre::eyre!("failed to compile {label} regex: {err}"))
}

fn report_abort_hits(aborts: &[String]) {
    if aborts.is_empty() {
        return;
    }

    println!("{RED}ERROR: std::process::abort() found in production code{NC}");
    println!();
    println!("abort() is never allowed - it terminates without unwinding.");
    println!("==================================================");
    for hit in aborts {
        println!("{hit}");
    }
    println!("==================================================");
    println!();
    println!("To fix: return an error and let the caller handle it.");
    println!();
}

fn report_exit_hits(exit_violations: &[String]) {
    if exit_violations.is_empty() {
        return;
    }

    println!("{RED}ERROR: std::process::exit() found outside allowlist{NC}");
    println!();
    println!("exit() is only allowed in:");
    println!("  - bin/ directories (CLI entry points)");
    println!("  - lifecycle.rs (LSP exit handler)");
    println!("==================================================");
    for hit in exit_violations {
        println!("{hit}");
    }
    println!("==================================================");
    println!();
    println!("To fix: return an error, use Result<(), E>, or move to an allowlisted path.");
    println!();
}

fn report_success() {
    println!("{GREEN}OK: No forbidden fatal constructs in production code{NC}");
    println!();
    println!("{YELLOW}Policy summary:{NC}");
    println!("  - abort(): NEVER allowed (banned everywhere)");
    println!("  - exit():  allowed in bin/ and lifecycle.rs only");
    println!();
    println!("{YELLOW}Note: panic!/unwrap!/expect! are enforced by Clippy deny lints:{NC}");
    println!("  - clippy::panic, clippy::unwrap_used, clippy::expect_used");
    println!("  - See [workspace.lints.clippy] in Cargo.toml");
}

fn is_fatal_excluded(path: &Path, repo_root: &Path) -> bool {
    let rel = path.strip_prefix(repo_root).unwrap_or(path).to_path_buf();
    let rel_string = format!("/{}", normalize_path_for_match(&rel.display().to_string()));

    if rel_string.contains("/tests/") {
        return true;
    }
    if rel_string.contains("/benches/") {
        return true;
    }
    if path.file_name().is_some_and(|name| name == "build.rs") {
        return true;
    }
    if path.file_name().is_some_and(|name| {
        name.to_string_lossy().ends_with("_test.rs")
            || name.to_string_lossy().ends_with("_tests.rs")
    }) {
        return true;
    }
    for excluded in ["tree-sitter-perl-c", "perl-tdd-support", "perl-ci-hygiene"] {
        if rel_string.contains(&format!("/{excluded}/")) {
            return true;
        }
    }

    path_has_component(path, "tests")
        || path_has_component(path, "benches")
        || path_has_component(path, "build.rs")
        || path_has_component(path, "examples")
}

fn normalize_path_for_match(value: &str) -> String {
    value.replace('\\', "/")
}

fn is_allowlisted_exit_hit(hit: &str) -> bool {
    let normalized = normalize_path_for_match(hit);
    normalized.contains("/bin/") || normalized.contains("/lifecycle.rs:")
}

#[cfg(test)]
mod tests {
    use super::{is_allowlisted_exit_hit, normalize_path_for_match};

    #[test]
    fn normalize_path_for_match_converts_backslashes() {
        assert_eq!(
            normalize_path_for_match(r"crates\perl-ci-hygiene\src\main.rs"),
            "crates/perl-ci-hygiene/src/main.rs"
        );
    }

    #[test]
    fn allowlisted_exit_hit_matches_windows_and_unix_paths() {
        assert!(is_allowlisted_exit_hit(
            r"crates\perl-parser\src\bin\perl-parse.rs:127:std::process::exit(0);"
        ));
        assert!(is_allowlisted_exit_hit(
            "crates/perl-lsp-rs/src/runtime/dispatch/lifecycle.rs:29:std::process::exit(exit_code);"
        ));
        assert!(!is_allowlisted_exit_hit(
            r#"crates\perl-ci-hygiene\src\main.rs:3196:println!("std::process::exit")"#
        ));
    }
}
