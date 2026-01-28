use perl_lsp::features::completion::{CompletionItemKind, CompletionProvider};
use perl_parser::Parser;
use serial_test::serial;
use std::fs;
#[cfg(unix)]
use std::path::Path;
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Create a test directory structure for comprehensive testing
fn create_test_directory() -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();

    // Create various file types
    fs::write(base_path.join("test.pl"), "#!/usr/bin/perl\nprint 'hello';\n")?;
    fs::write(base_path.join("module.pm"), "package Module;\n1;\n")?;
    fs::write(base_path.join("script.t"), "use Test::More;\nok(1);\n")?;
    fs::write(base_path.join("config.json"), "{}")?;
    fs::write(base_path.join("README.md"), "# Test")?;
    fs::write(base_path.join("Cargo.toml"), "[package]")?;

    // Create subdirectories
    fs::create_dir(base_path.join("lib"))?;
    fs::create_dir(base_path.join("tests"))?;
    fs::create_dir(base_path.join("docs"))?;

    // Create files in subdirectories
    fs::write(base_path.join("lib/helper.pl"), "sub help { }")?;
    fs::write(base_path.join("tests/unit.t"), "use Test::More;")?;
    fs::write(base_path.join("docs/guide.md"), "# Guide")?;

    // Create hidden files and directories (should be filtered out)
    fs::write(base_path.join(".hidden"), "secret")?;
    fs::create_dir(base_path.join(".git"))?;
    fs::write(base_path.join(".git/config"), "[core]")?;

    Ok(temp_dir)
}

#[test]
#[serial]
fn test_basic_file_completion() -> TestResult {
    let temp_dir = create_test_directory()?;
    let old_cwd = std::env::current_dir()?;
    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)?;

    let code = "\"test\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("test").ok_or("Failed to find 'test' in code")? + "test".len();
    let completions = provider.get_completions_with_path(code, pos, Some("."));

    // Should find test.pl
    assert!(completions.iter().any(|c| c.label == "test.pl" && c.kind == CompletionItemKind::File));
    assert!(completions.iter().any(|c| c.label == "tests/" && c.kind == CompletionItemKind::File));

    // Should not find hidden files
    assert!(!completions.iter().any(|c| c.label.starts_with('.')));

    std::env::set_current_dir(old_cwd).ok();
    Ok(())
}

#[test]
#[serial]
fn test_directory_traversal() -> TestResult {
    let temp_dir = create_test_directory()?;
    let old_cwd = std::env::current_dir()?;
    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)?;

    let code = "\"lib/hel\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("hel").ok_or("Failed to find 'hel' in code")? + "hel".len();
    let completions = provider.get_completions_with_path(code, pos, Some("."));

    // Should find lib/helper.pl
    assert!(completions.iter().any(|c| c.label == "lib/helper.pl"));

    std::env::set_current_dir(old_cwd).ok();
    Ok(())
}

#[test]
#[serial]
fn test_security_path_traversal_blocked() -> TestResult {
    let code = "\"../../../etc/passwd\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("passwd").ok_or("Failed to find 'passwd' in code")? + "passwd".len();
    let completions = provider.get_completions_with_path(code, pos, Some("."));

    // Should not return any completions for path traversal attempts
    assert!(completions.is_empty());
    Ok(())
}

#[test]
#[serial]
fn test_security_absolute_paths_blocked() -> TestResult {
    let code = "\"/etc/passwd\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("passwd").ok_or("Failed to find 'passwd' in code")? + "passwd".len();
    let completions = provider.get_completions_with_path(code, pos, Some("."));

    // Should not return completions for absolute paths (except root)
    assert!(completions.is_empty());
    Ok(())
}

#[test]
#[serial]
fn test_security_null_bytes_blocked() -> TestResult {
    let code = "\"test\0\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("test").ok_or("Failed to find 'test' in code")? + "test".len();
    let completions = provider.get_completions_with_path(code, pos, Some("."));

    // Should not return completions for paths with null bytes
    assert!(completions.is_empty());
    Ok(())
}

#[test]
#[serial]
fn test_hidden_files_filtered() -> TestResult {
    let temp_dir = create_test_directory()?;
    let old_cwd = std::env::current_dir()?;
    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)?;

    let code = "\".h\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find(".h").ok_or("Failed to find '.h' in code")? + ".h".len();
    let completions = provider.get_completions_with_path(code, pos, Some("."));

    // Should not suggest hidden files
    assert!(!completions.iter().any(|c| c.label.contains(".hidden")));
    assert!(!completions.iter().any(|c| c.label.contains(".git")));

    std::env::set_current_dir(old_cwd).ok();
    Ok(())
}

#[test]
#[serial]
fn test_file_type_detection() -> TestResult {
    let temp_dir = create_test_directory()?;
    let old_cwd = std::env::current_dir()?;
    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)?;

    let code = "\"test.p\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("test.p").ok_or("Failed to find 'test.p' in code")? + "test.p".len();
    let completions = provider.get_completions_with_path(code, pos, Some("."));

    // Should find Perl file with appropriate detail
    let perl_completion = completions.iter().find(|c| c.label == "test.pl");
    assert!(perl_completion.is_some());
    let perl_completion = perl_completion.ok_or("No test.pl completion found")?;
    let detail = perl_completion.detail.as_ref().ok_or("No detail in completion")?;
    assert_eq!(detail, "Perl file");

    std::env::set_current_dir(old_cwd).ok();
    Ok(())
}

#[test]
#[serial]
fn test_directory_completion_with_slash() -> TestResult {
    let temp_dir = create_test_directory()?;
    let old_cwd = std::env::current_dir()?;
    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)?;

    let code = "\"lib\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("lib").ok_or("Failed to find 'lib' in code")? + "lib".len();
    let completions = provider.get_completions_with_path(code, pos, Some("."));

    // Should find lib directory with trailing slash
    assert!(
        completions.iter().any(|c| c.label == "lib/" && c.detail.as_deref() == Some("directory"))
    );

    std::env::set_current_dir(old_cwd).ok();
    Ok(())
}

#[test]
#[serial]
fn test_empty_prefix_completion() -> TestResult {
    let temp_dir = create_test_directory()?;
    let old_cwd = std::env::current_dir()?;
    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)?;

    let code = "\"\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = 1; // Inside the empty string
    let completions = provider.get_completions_with_path(code, pos, Some("."));

    // Should provide completions for current directory
    assert!(completions.iter().any(|c| c.label == "test.pl"));
    assert!(completions.iter().any(|c| c.label == "lib/"));

    std::env::set_current_dir(old_cwd).ok();
    Ok(())
}

#[test]
#[serial]
fn test_performance_limits() -> TestResult {
    let temp_dir = TempDir::new()?;
    let old_cwd = std::env::current_dir()?;
    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)?;

    // Create many files to test performance limits
    for i in 0..100 {
        fs::write(temp_dir.path().join(format!("file_{}.txt", i)), "content")?;
    }

    let code = "\"file_\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("file_").ok_or("Failed to find 'file_' in code")? + "file_".len();
    let completions = provider.get_completions_with_path(code, pos, Some("."));

    // Should respect max_results limit (50 in implementation)
    assert!(completions.len() <= 50);

    std::env::set_current_dir(old_cwd).ok();
    Ok(())
}

#[test]
#[serial]
fn test_cancellation_support() -> TestResult {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let temp_dir = create_test_directory()?;
    let old_cwd = std::env::current_dir()?;
    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)?;

    let code = "\"test\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("test").ok_or("Failed to find 'test' in code")? + "test".len();

    // Test with immediate cancellation
    let cancelled = Arc::new(AtomicBool::new(true));
    let cancelled_clone = cancelled.clone();
    let completions = provider.get_completions_with_path_cancellable(code, pos, None, &|| {
        cancelled_clone.load(Ordering::Relaxed)
    });

    // Should return empty due to cancellation
    assert!(completions.is_empty());

    std::env::set_current_dir(old_cwd).ok();
    Ok(())
}

#[test]
#[serial]
fn test_cross_platform_path_handling() -> TestResult {
    let temp_dir = create_test_directory()?;
    let old_cwd = std::env::current_dir()?;
    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)?;

    // Test with Windows-style backslashes (should be normalized)
    let code = "\"lib\\hel\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("hel").ok_or("Failed to find 'hel' in code")? + "hel".len();
    let completions = provider.get_completions_with_path(code, pos, Some("."));

    // Should normalize backslashes and find the file
    assert!(completions.iter().any(|c| c.label == "lib/helper.pl"));

    std::env::set_current_dir(old_cwd).ok();
    Ok(())
}

#[test]
#[serial]
fn test_max_path_length_protection() -> TestResult {
    let very_long_path = "a/".repeat(500) + "test";
    let code = format!("\"{}\"", very_long_path);
    let mut parser = Parser::new(&code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.len() - 1;
    let completions = provider.get_completions(&code, pos);

    // Should reject overly long paths
    assert!(completions.is_empty());
    Ok(())
}

#[test]
#[serial]
fn test_windows_reserved_names_blocked() -> TestResult {
    let temp_dir = TempDir::new()?;
    let old_cwd = std::env::current_dir()?;
    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)?;

    // Create file with Windows reserved name
    if fs::write(temp_dir.path().join("CON.txt"), "content").is_ok() {
        let code = "\"CON\"";
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let provider = CompletionProvider::new_with_index(&ast, None);
        let pos = code.find("CON").ok_or("Failed to find 'CON' in code")? + "CON".len();
        let completions = provider.get_completions_with_path(code, pos, Some("."));

        // Should not suggest Windows reserved names
        assert!(!completions.iter().any(|c| c.label.to_uppercase().contains("CON")));
    }

    std::env::set_current_dir(old_cwd).ok();
    Ok(())
}

#[test]
#[serial]
fn test_completion_text_edit_range() -> TestResult {
    let temp_dir = create_test_directory()?;
    let old_cwd = std::env::current_dir()?;
    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)?;

    let code = "\"test.p\"";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let provider = CompletionProvider::new_with_index(&ast, None);
    let pos = code.find("test.p").ok_or("Failed to find 'test.p' in code")? + "test.p".len();
    let completions = provider.get_completions_with_path(code, pos, Some("."));

    // Check that text_edit_range is correctly set
    if let Some(completion) = completions.iter().find(|c| c.label == "test.pl") {
        assert!(completion.text_edit_range.is_some());
        let (start, end) = completion.text_edit_range.ok_or("No text_edit_range in completion")?;
        assert_eq!(start, 1); // Start of path in string
        assert_eq!(end, pos); // Current position
    }

    std::env::set_current_dir(old_cwd).ok();
    Ok(())
}

#[test]
#[serial]
#[cfg(unix)] // symlink API is Unix-only
fn test_no_symlink_following() -> TestResult {
    use std::os::unix::fs::symlink;

    let temp_dir = TempDir::new()?;
    let old_cwd = std::env::current_dir()?;
    let temp_path = temp_dir.path().to_path_buf();
    std::env::set_current_dir(&temp_path)?;

    // Create a symlink to a file outside the directory
    let target_file = "/etc/hosts";
    if Path::new(target_file).exists()
        && symlink(target_file, temp_dir.path().join("dangerous_link")).is_ok()
    {
        let code = "\"dangerous\"";
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let provider = CompletionProvider::new_with_index(&ast, None);
        let pos =
            code.find("dangerous").ok_or("Failed to find 'dangerous' in code")? + "dangerous".len();
        let completions = provider.get_completions_with_path(code, pos, Some("."));

        // Should not follow symlinks (walkdir configured with follow_links(false))
        // The symlink itself might appear but shouldn't be traversed
        let has_dangerous = completions.iter().any(|c| c.label.contains("dangerous"));
        if has_dangerous {
            // If the symlink appears, it should be treated as a regular file, not followed
            assert!(completions.iter().all(|c| !c.label.contains("hosts")));
        }
    }

    std::env::set_current_dir(old_cwd).ok();
    Ok(())
}
