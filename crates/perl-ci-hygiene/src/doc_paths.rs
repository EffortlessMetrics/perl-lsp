use color_eyre::eyre::Result;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const RED: &str = "\x1b[0;31m";
const NC: &str = "\x1b[0m";

pub(crate) fn cmd_check_doc_paths(repo_root: &Path, docs_dir: Option<&str>) -> Result<i32> {
    let docs_dir = docs_dir.unwrap_or("docs");
    let docs_path = if Path::new(docs_dir).is_absolute() {
        PathBuf::from(docs_dir)
    } else {
        repo_root.join(docs_dir)
    };
    let home_user_path = Regex::new(r"/home/([A-Za-z0-9._-]+)")?;
    let users_name_path = Regex::new(r"/Users/([A-Za-z0-9._-]+)")?;

    let mut hard_failures = Vec::new();
    let mut warnings = Vec::new();

    if !docs_path.is_dir() {
        return Err(color_eyre::eyre::eyre!("Docs directory not found: {}", docs_path.display()));
    }

    for entry in WalkDir::new(&docs_path).into_iter().flatten() {
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
            if has_machine_specific_home_path(line, &home_user_path) {
                hard_failures.push(format!("{rel}:{number}:{line}"));
            }
            if has_machine_specific_users_path(line, &users_name_path) {
                warnings.push(format!("{rel}:{number}:{line}"));
            }
        }
    }

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

pub(crate) fn has_machine_specific_home_path(line: &str, home_user_path: &Regex) -> bool {
    home_user_path.captures_iter(line).any(|captures| {
        captures.get(1).is_some_and(|name| !name.as_str().eq_ignore_ascii_case("user"))
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

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).display().to_string()
}

fn is_text_file(path: &Path) -> bool {
    let text_exts = [
        "md", "txt", "rst", "adoc", "json", "toml", "yml", "yaml", "sh", "bash", "zsh", "fish",
        "ps1", "cmd", "bat", "rs", "js", "ts", "py", "pl", "pm", "t",
    ];

    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| text_exts.contains(&ext))
}
