use color_eyre::eyre::Result;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{NC, RED, display_path, is_text_file, walk_entries};

pub(crate) fn check_doc_paths(repo_root: &Path, docs_dir: Option<&str>) -> Result<i32> {
    let docs_dir = docs_dir.unwrap_or("docs");
    let docs_path = resolve_docs_path(repo_root, docs_dir);
    let home_user_path = Regex::new(r"/home/([A-Za-z0-9._-]+)")?;
    let users_name_path = Regex::new(r"/Users/([A-Za-z0-9._-]+)")?;

    if !docs_path.is_dir() {
        return Err(color_eyre::eyre::eyre!(
            "Docs directory not found: {}",
            docs_path.display()
        ));
    }

    let (hard_failures, warnings) = scan_docs(
        repo_root,
        &docs_path,
        &home_user_path,
        &users_name_path,
    )?;

    if !warnings.is_empty() {
        println!("⚠️  Found macOS user paths that may be machine-specific");
        for hit in warnings {
            println!("{hit}");
        }
        println!();
    }

    if hard_failures.is_empty() {
        println!("✅ No machine-specific paths found in documentation");
        return Ok(0);
    }

    println!("{RED}❌ Found machine-specific /home/ paths (not /home/user examples){NC}");
    for hit in hard_failures {
        println!("{hit}");
    }
    println!();
    println!("Fix: Replace absolute paths with repo-relative paths or generic examples");
    println!("  - Use relative paths: docs/file.md instead of /home/.../docs/file.md");
    println!("  - Use generic examples: /home/user/project for user-facing docs");
    Ok(1)
}

fn resolve_docs_path(repo_root: &Path, docs_dir: &str) -> PathBuf {
    if Path::new(docs_dir).is_absolute() {
        PathBuf::from(docs_dir)
    } else {
        repo_root.join(docs_dir)
    }
}

fn scan_docs(
    repo_root: &Path,
    docs_path: &Path,
    home_user_path: &Regex,
    users_name_path: &Regex,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut hard_failures = Vec::new();
    let mut warnings = Vec::new();

    for entry in walk_entries(docs_path) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !is_text_file(path) {
            continue;
        }
        let rel = display_path(repo_root, path);
        let contents = fs::read_to_string(path)?;
        for (line_no, line) in contents.lines().enumerate() {
            let number = line_no + 1;
            if has_machine_specific_home_path(line, home_user_path) {
                hard_failures.push(format!("{rel}:{number}:{line}"));
            }
            if has_machine_specific_users_path(line, users_name_path) {
                warnings.push(format!("{rel}:{number}:{line}"));
            }
        }
    }

    Ok((hard_failures, warnings))
}

pub(crate) fn has_machine_specific_home_path(line: &str, home_user_path: &Regex) -> bool {
    home_user_path.captures_iter(line).any(|captures| {
        captures
            .get(1)
            .is_some_and(|name| !name.as_str().eq_ignore_ascii_case("user"))
    })
}

pub(crate) fn has_machine_specific_users_path(line: &str, users_name_path: &Regex) -> bool {
    users_name_path.captures_iter(line).any(|captures| {
        captures.get(1).is_some_and(|name| {
            let value = name.as_str();
            !(value.eq_ignore_ascii_case("name") || value.eq_ignore_ascii_case("user"))
        })
    })
}
