#[cfg(test)]
mod integration_highlight {
    use std::fs;
    use std::path::Path;
    use crate::test_harness::{parse_perl_code, test_corpus_file_parses};

    #[test]
    fn test_parse_all_highlight_files() {
        let highlight_dir = Path::new("test/highlight");
        let mut test_count = 0;
        let mut failed_files = Vec::new();

        for entry in fs::read_dir(highlight_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "pm") {
                test_count += 1;
                
                // Read the highlight test file
                let content = fs::read_to_string(&path).unwrap();
                
                // Parse each line that contains Perl code
                let lines: Vec<&str> = content.lines().collect();
                let mut line_errors = Vec::new();
                
                for (line_num, line) in lines.iter().enumerate() {
                    if !line.trim().is_empty() && !line.starts_with('#') {
                        match parse_perl_code(line) {
                            Ok(_) => (), // Success
                            Err(e) => line_errors.push((line_num + 1, line.to_string(), e)),
                        }
                    }
                }
                
                if line_errors.is_empty() {
                    println!("✓ {:?}", path.file_name().unwrap());
                } else {
                    println!("✗ {:?}: {} errors", path.file_name().unwrap(), line_errors.len());
                    failed_files.push((path, line_errors));
                }
            }
        }

        println!("\nHighlight test summary:");
        println!("Total files: {}", test_count);
        println!("Passed: {}", test_count - failed_files.len());
        println!("Failed: {}", failed_files.len());

        if !failed_files.is_empty() {
            println!("\nFailed files:");
            for (path, errors) in failed_files {
                println!("  {:?}:", path.file_name().unwrap());
                for (line_num, line, error) in errors {
                    println!("    Line {}: '{}' - {}", line_num, line, error);
                }
            }
            panic!("Some highlight files failed to parse");
        }
    }

    #[test]
    fn test_parse_specific_highlight_files() {
        // Test specific highlight files that are known to be complex
        let test_files = [
            "test/highlight/statements.pm",
            "test/highlight/variables.pm",
            "test/highlight/functions.pm",
            "test/highlight/expressions.pm",
        ];

        for file_path in &test_files {
            let path = Path::new(file_path);
            let content = fs::read_to_string(path).unwrap();
            
            // Parse each non-empty, non-comment line
            for (line_num, line) in content.lines().enumerate() {
                if !line.trim().is_empty() && !line.starts_with('#') {
                    assert!(
                        parse_perl_code(line).is_ok(),
                        "Failed to parse line {} in {:?}: '{}'",
                        line_num + 1,
                        path,
                        line
                    );
                }
            }
        }
    }

    #[test]
    fn test_highlight_file_contents() {
        // Test that highlight files contain valid Perl code
        let highlight_dir = Path::new("test/highlight");
        
        for entry in fs::read_dir(highlight_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "pm") {
                let content = fs::read_to_string(&path).unwrap();
                
                // Skip empty files (they might be placeholders)
                if content.trim().is_empty() {
                    println!("⚠ Skipping empty highlight file: {:?}", path);
                    continue;
                }
                
                // Ensure it contains some Perl code (not just comments)
                let code_lines: Vec<&str> = content
                    .lines()
                    .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
                    .collect();
                
                assert!(
                    !code_lines.is_empty(),
                    "Highlight file {:?} contains no Perl code",
                    path
                );
            }
        }
    }
} 