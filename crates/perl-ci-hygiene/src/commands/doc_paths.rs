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
        return Err(color_eyre::eyre::eyre!("Docs directory not found: {}", docs_path.display()));
    }

    let (hard_failures, warnings) =
        scan_docs(repo_root, &docs_path, &home_user_path, &users_name_path)?;

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

#[cfg(test)]
mod tests {
    use super::{
        check_doc_paths, has_machine_specific_home_path, has_machine_specific_users_path,
        resolve_docs_path,
    };
    use regex::Regex;
    use std::error::Error;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    type TestResult<T = ()> = Result<T, Box<dyn Error>>;

    fn unique_temp_dir(label: &str) -> TestResult<PathBuf> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let dir = std::env::temp_dir()
            .join(format!("perl-ci-hygiene-doc-paths-{label}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    // ── resolve_docs_path ──────────────────────────────────────────────────

    #[test]
    fn resolve_docs_path_relative_joins_repo_root() {
        let root = Path::new("/repo/root");
        let result = resolve_docs_path(root, "docs");
        assert_eq!(result, PathBuf::from("/repo/root/docs"));
    }

    #[test]
    fn resolve_docs_path_absolute_ignores_repo_root() {
        let root = Path::new("/repo/root");
        let result = resolve_docs_path(root, "/absolute/docs");
        assert_eq!(result, PathBuf::from("/absolute/docs"));
    }

    #[test]
    fn resolve_docs_path_nested_relative_joins_correctly() {
        let root = Path::new("/workspace");
        let result = resolve_docs_path(root, "docs/reference");
        assert_eq!(result, PathBuf::from("/workspace/docs/reference"));
    }

    // ── has_machine_specific_home_path ─────────────────────────────────────

    #[test]
    fn home_path_generic_user_is_not_machine_specific() -> TestResult {
        let re = Regex::new(r"/home/([A-Za-z0-9._-]+)")?;
        assert!(!has_machine_specific_home_path("See /home/user/project for an example.", &re,));
        Ok(())
    }

    #[test]
    fn home_path_generic_user_case_insensitive() -> TestResult {
        let re = Regex::new(r"/home/([A-Za-z0-9._-]+)")?;
        // "USER" and "User" must also be treated as generic
        assert!(!has_machine_specific_home_path("path: /home/USER/project", &re));
        assert!(!has_machine_specific_home_path("path: /home/User/project", &re));
        Ok(())
    }

    #[test]
    fn home_path_real_username_is_machine_specific() -> TestResult {
        let re = Regex::new(r"/home/([A-Za-z0-9._-]+)")?;
        assert!(has_machine_specific_home_path("/home/alice/dev/perl-lsp", &re));
        assert!(has_machine_specific_home_path("Built at /home/ubuntu/workspace", &re));
        Ok(())
    }

    #[test]
    fn home_path_single_char_username_is_machine_specific() -> TestResult {
        let re = Regex::new(r"/home/([A-Za-z0-9._-]+)")?;
        assert!(has_machine_specific_home_path("/home/u/project", &re));
        Ok(())
    }

    #[test]
    fn home_path_multiple_matches_any_real_triggers_detection() -> TestResult {
        let re = Regex::new(r"/home/([A-Za-z0-9._-]+)")?;
        // Line has both a generic and a machine-specific path — must return true
        let line = "cp /home/user/template /home/alice/dest";
        assert!(has_machine_specific_home_path(line, &re));
        Ok(())
    }

    #[test]
    fn home_path_no_matches_returns_false() -> TestResult {
        let re = Regex::new(r"/home/([A-Za-z0-9._-]+)")?;
        assert!(!has_machine_specific_home_path("No paths here at all.", &re));
        assert!(!has_machine_specific_home_path("", &re));
        Ok(())
    }

    // ── has_machine_specific_users_path ────────────────────────────────────

    #[test]
    fn users_path_generic_name_placeholder_is_not_machine_specific() -> TestResult {
        let re = Regex::new(r"/Users/([A-Za-z0-9._-]+)")?;
        assert!(!has_machine_specific_users_path("path: /Users/Name/project", &re));
        Ok(())
    }

    #[test]
    fn users_path_generic_user_placeholder_is_not_machine_specific() -> TestResult {
        let re = Regex::new(r"/Users/([A-Za-z0-9._-]+)")?;
        assert!(!has_machine_specific_users_path("path: /Users/user/project", &re));
        Ok(())
    }

    #[test]
    fn users_path_generic_placeholders_case_insensitive() -> TestResult {
        let re = Regex::new(r"/Users/([A-Za-z0-9._-]+)")?;
        assert!(!has_machine_specific_users_path("path: /Users/NAME/project", &re));
        assert!(!has_machine_specific_users_path("path: /Users/USER/project", &re));
        Ok(())
    }

    #[test]
    fn users_path_real_username_is_machine_specific() -> TestResult {
        let re = Regex::new(r"/Users/([A-Za-z0-9._-]+)")?;
        assert!(has_machine_specific_users_path("Personal: /Users/alice/dev/perl-lsp", &re,));
        assert!(has_machine_specific_users_path("/Users/bob/workspace", &re));
        Ok(())
    }

    #[test]
    fn users_path_multiple_matches_any_real_triggers_detection() -> TestResult {
        let re = Regex::new(r"/Users/([A-Za-z0-9._-]+)")?;
        let line = "from /Users/Name/src to /Users/alice/dest";
        assert!(has_machine_specific_users_path(line, &re));
        Ok(())
    }

    #[test]
    fn users_path_no_matches_returns_false() -> TestResult {
        let re = Regex::new(r"/Users/([A-Za-z0-9._-]+)")?;
        assert!(!has_machine_specific_users_path("No paths here.", &re));
        assert!(!has_machine_specific_users_path("", &re));
        Ok(())
    }

    // ── check_doc_paths (end-to-end via public API) ────────────────────────

    #[test]
    fn check_doc_paths_errors_when_docs_dir_missing() -> TestResult {
        let root = unique_temp_dir("missing-docs")?;
        // No "docs" directory created — must return Err
        let result = check_doc_paths(&root, None);
        assert!(result.is_err());
        let _ = fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn check_doc_paths_returns_zero_for_clean_docs() -> TestResult {
        let root = unique_temp_dir("clean-docs")?;
        let docs = root.join("docs");
        fs::create_dir_all(&docs)?;
        fs::write(docs.join("guide.md"), "Use /home/user/project as the example.\n")?;
        let exit_code = check_doc_paths(&root, None)?;
        assert_eq!(exit_code, 0);
        let _ = fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn check_doc_paths_returns_one_for_machine_specific_home_path() -> TestResult {
        let root = unique_temp_dir("home-violation")?;
        let docs = root.join("docs");
        fs::create_dir_all(&docs)?;
        fs::write(docs.join("setup.md"), "Run: /home/alice/dev/perl-lsp/target\n")?;
        let exit_code = check_doc_paths(&root, None)?;
        assert_eq!(exit_code, 1);
        let _ = fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn check_doc_paths_warns_but_returns_zero_for_users_path_only() -> TestResult {
        let root = unique_temp_dir("users-warning")?;
        let docs = root.join("docs");
        fs::create_dir_all(&docs)?;
        fs::write(docs.join("notes.md"), "See /Users/alice/project for context.\n")?;
        let exit_code = check_doc_paths(&root, None)?;
        // macOS /Users/ paths are warnings only, not hard failures
        assert_eq!(exit_code, 0);
        let _ = fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn check_doc_paths_custom_docs_dir() -> TestResult {
        let root = unique_temp_dir("custom-docs")?;
        let custom = root.join("documentation");
        fs::create_dir_all(&custom)?;
        fs::write(custom.join("readme.md"), "Generic docs content.\n")?;
        let exit_code = check_doc_paths(&root, Some("documentation"))?;
        assert_eq!(exit_code, 0);
        let _ = fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn check_doc_paths_empty_docs_dir_returns_zero() -> TestResult {
        let root = unique_temp_dir("empty-docs")?;
        let docs = root.join("docs");
        fs::create_dir_all(&docs)?;
        // No files in docs directory
        let exit_code = check_doc_paths(&root, None)?;
        assert_eq!(exit_code, 0);
        let _ = fs::remove_dir_all(&root);
        Ok(())
    }
}
