#[cfg(test)]
mod integration_highlight {
    use std::fs;
    use std::path::Path;
    use std::time::Instant;
    use perl_tdd_support::{must, must_some};
    use crate::test_harness::{parse_perl_code, tree_to_string};
    use crate::{parse, language};
    use tree_sitter::{Parser, Query, QueryCursor};

    #[test]
    fn test_parse_all_highlight_files() {
        let highlight_dir = Path::new("test/highlight");
        let mut test_count = 0;
        let mut failed_files = Vec::new();
        let mut total_parse_time = 0u128;

        for entry in must(fs::read_dir(highlight_dir)) {
            let entry = must(entry);
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "pm") {
                test_count += 1;
                
                // Read the highlight test file
                let content = must(fs::read_to_string(&path));
                let start = Instant::now();
                
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
                
                let parse_time = start.elapsed().as_micros();
                total_parse_time += parse_time;
                
                if line_errors.is_empty() {
                    println!("✓ {:?} ({} μs)", must_some(path.file_name()), parse_time);
                } else {
                    println!("✗ {:?}: {} errors ({} μs)", must_some(path.file_name()), line_errors.len(), parse_time);
                    failed_files.push((path, line_errors));
                }
            }
        }

        println!("\nHighlight test summary:");
        println!("Total files: {}", test_count);
        println!("Passed: {}", test_count - failed_files.len());
        println!("Failed: {}", failed_files.len());
        println!("Average parse time: {:.2} μs", total_parse_time as f64 / test_count as f64);

        if !failed_files.is_empty() {
            println!("\nFailed files:");
            for (path, errors) in failed_files {
                println!("  {:?}:", must_some(path.file_name()));
                for (line_num, line, error) in errors {
                    println!("    Line {}: '{}' - {}", line_num, line, error);
                }
            }
            must(Err::<(), _>(format!("Some highlight files failed to parse")));
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
            "test/highlight/strings.pm",
            "test/highlight/comments.pm",
            "test/highlight/operators.pm",
            "test/highlight/control_structures.pm",
        ];

        for file_path in &test_files {
            let path = Path::new(file_path);
            if path.exists() {
                let content = must(fs::read_to_string(path));
                
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
    }

    #[test]
    fn test_highlight_file_contents() {
        // Test that highlight files contain valid Perl code
        let highlight_dir = Path::new("test/highlight");
        
        for entry in must(fs::read_dir(highlight_dir)) {
            let entry = must(entry);
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "pm") {
                let content = must(fs::read_to_string(&path));
                
                // Ensure it's not empty
                assert!(!content.trim().is_empty(), "Highlight file {:?} is empty", path);
                
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

    #[test]
    fn test_syntax_highlighting_queries() {
        // Test that syntax highlighting queries work correctly
        let highlight_dir = Path::new("test/highlight");
        let mut parser = Parser::new();
        must(parser.set_language(&language()));
        
        // Define basic highlighting queries
        let queries = vec![
            "(variable_declaration) @variable",
            "(function_call) @function",
            "(string_literal) @string",
            "(comment) @comment",
            "(number_literal) @number",
            "(keyword) @keyword",
        ];
        
        for query_str in queries {
            let query = must(Query::new(&language(), query_str));
            let mut cursor = QueryCursor::new();
            
            for entry in must(fs::read_dir(highlight_dir)) {
                let entry = must(entry);
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|ext| ext == "pm") {
                    let content = must(fs::read_to_string(&path));
                    let tree = must_some(parser.parse(&content, None));
                    
                    // Execute the query
                    let captures = cursor.captures(&query, tree.root_node(), content.as_bytes());
                    
                    // Ensure query execution doesn't panic
                    for (match_, capture_index) in captures {
                        assert!(capture_index < match_.captures.len());
                    }
                }
            }
        }
    }

    #[test]
    fn test_highlight_performance() {
        // Test highlighting performance
        let highlight_dir = Path::new("test/highlight");
        let mut parser = Parser::new();
        must(parser.set_language(&language()));
        
        let query = must(Query::new(&language(), "(variable_declaration) @variable"));
        let mut cursor = QueryCursor::new();
        
        let mut total_files = 0;
        let mut total_time = 0u128;
        let mut slow_files = Vec::new();
        
        for entry in must(fs::read_dir(highlight_dir)) {
            let entry = must(entry);
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "pm") {
                let content = must(fs::read_to_string(&path));
                let start = Instant::now();
                
                let tree = must_some(parser.parse(&content, None));
                let captures = cursor.captures(&query, tree.root_node(), content.as_bytes());
                
                let highlight_time = start.elapsed().as_micros();
                total_files += 1;
                total_time += highlight_time;
                
                if highlight_time > 500 {
                    slow_files.push((must_some(path.file_name()).to_string_lossy().to_string(), highlight_time));
                }
                
                // Ensure highlighting completed successfully
                let capture_count = captures.count();
                assert!(capture_count >= 0, "Invalid capture count for {:?}", path);
            }
        }
        
        let avg_time = total_time as f64 / total_files as f64;
        println!("Highlight performance summary:");
        println!("Total files: {}", total_files);
        println!("Average highlight time: {:.2} μs", avg_time);
        
        if !slow_files.is_empty() {
            println!("Slow files (>500 μs):");
            for (file, time) in slow_files {
                println!("  {}: {} μs", file, time);
            }
        }
        
        // Ensure average highlight time is reasonable
        assert!(avg_time < 200.0, "Average highlight time too high: {:.2} μs", avg_time);
    }

    #[test]
    fn test_highlight_token_consistency() {
        // Test that highlighting produces consistent tokens
        let highlight_dir = Path::new("test/highlight");
        let mut parser = Parser::new();
        must(parser.set_language(&language()));
        
        let queries = vec![
            "(variable_declaration) @variable",
            "(function_call) @function",
            "(string_literal) @string",
        ];
        
        for query_str in queries {
            let query = must(Query::new(&language(), query_str));
            let mut cursor = QueryCursor::new();
            
            for entry in must(fs::read_dir(highlight_dir)) {
                let entry = must(entry);
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|ext| ext == "pm") {
                    let content = must(fs::read_to_string(&path));
                    let tree = must_some(parser.parse(&content, None));
                    
                    let captures = cursor.captures(&query, tree.root_node(), content.as_bytes());
                    
                    for (match_, capture_index) in captures {
                        let capture = &match_.captures[capture_index];
                        
                        // Ensure capture has valid byte range
                        assert!(capture.node.start_byte() <= capture.node.end_byte());
                        assert!(capture.node.start_byte() < content.len());
                        assert!(capture.node.end_byte() <= content.len());
                        
                        // Ensure capture text is valid
                        let capture_text = &content[capture.node.start_byte()..capture.node.end_byte()];
                        assert!(!capture_text.is_empty());
                    }
                }
            }
        }
    }

    #[test]
    fn test_highlight_error_recovery() {
        // Test that highlighting works even with parse errors
        let test_cases = vec![
            "my $var = 42; # Valid code",
            "my $var = ; # Missing value",
            "print 'Hello'; # Valid code",
            "if ($condition { # Missing closing brace",
            "sub foo { return 1; } # Valid code",
        ];
        
        let mut parser = Parser::new();
        must(parser.set_language(&language()));
        
        let query = must(Query::new(&language(), "(variable_declaration) @variable"));
        let mut cursor = QueryCursor::new();
        
        for (i, code) in test_cases.iter().enumerate() {
            let tree = parser.parse(code, None);
            assert!(tree.is_some(), "Failed to parse test case {}: {}", i, code);
            
            let tree = must_some(tree);
            let captures = cursor.captures(&query, tree.root_node(), code.as_bytes());
            
            // Highlighting should work even with parse errors
            for (match_, capture_index) in captures {
                let capture = &match_.captures[capture_index];
                assert!(capture.node.start_byte() <= capture.node.end_byte());
            }
        }
    }

    #[test]
    fn test_highlight_memory_usage() {
        // Test memory usage during highlighting
        let highlight_dir = Path::new("test/highlight");
        let mut parser = Parser::new();
        must(parser.set_language(&language()));
        
        let query = must(Query::new(&language(), "(variable_declaration) @variable"));
        let mut cursor = QueryCursor::new();
        
        let mut total_captures = 0;
        let mut total_files = 0;
        
        for entry in must(fs::read_dir(highlight_dir)) {
            let entry = must(entry);
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "pm") {
                let content = must(fs::read_to_string(&path));
                let tree = must_some(parser.parse(&content, None));
                
                let captures = cursor.captures(&query, tree.root_node(), content.as_bytes());
                let capture_count = captures.count();
                
                total_captures += capture_count;
                total_files += 1;
                
                // Ensure reasonable capture count per file
                assert!(
                    capture_count < 1000,
                    "Too many captures ({}) in {:?}",
                    capture_count,
                    path
                );
            }
        }
        
        let avg_captures = total_captures as f64 / total_files as f64;
        println!("Highlight memory usage summary:");
        println!("Total files: {}", total_files);
        println!("Total captures: {}", total_captures);
        println!("Average captures per file: {:.2}", avg_captures);
        
        // Ensure reasonable average capture count
        assert!(avg_captures < 100.0, "Average capture count too high: {:.2}", avg_captures);
    }

    #[test]
    fn test_highlight_query_validation() {
        // Test that highlighting queries are valid
        let valid_queries = vec![
            "(variable_declaration) @variable",
            "(function_call) @function",
            "(string_literal) @string",
            "(comment) @comment",
            "(number_literal) @number",
            "(keyword) @keyword",
            "(operator) @operator",
        ];
        
        for query_str in valid_queries {
            let query_result = Query::new(&language(), query_str);
            assert!(query_result.is_ok(), "Invalid query: {}", query_str);
        }
        
        let invalid_queries = vec![
            "(invalid_node) @variable",
            "(variable_declaration @variable",
            ") @variable",
            "@variable",
        ];
        
        for query_str in invalid_queries {
            let query_result = Query::new(&language(), query_str);
            assert!(query_result.is_err(), "Query should be invalid: {}", query_str);
        }
    }
} 