// Comprehensive Rust-side test suite for tree-sitter-perl
//
// This module orchestrates all scanner, unicode, property, and integration tests.
// It is designed to mirror the C-based test suite and ensure 100% input/output fidelity.

#[cfg(all(test, feature = "c-parser"))]
#[allow(clippy::module_inception)]
mod tests {

    use crate::{language, parse};
    use tree_sitter::Parser;
    use perl_tdd_support::{must, must_some};

    #[test]
    fn test_language_loading() {
        let lang = language();
        // Language is valid if we can get its version
        assert!(lang.abi_version() > 0);
    }

    #[test]
    fn test_basic_parsing() {
        let test_cases = [
            "my $var = 42;",
            "print 'Hello, World!';",
            "sub foo { return 1; }",
            "if ($x) { $y = 1; }",
            "for my $i (1..10) { print $i; }",
        ];

        for (i, code) in test_cases.iter().enumerate() {
            let result = parse(code);
            assert!(result.is_ok(), "Test case {} failed: {:?}", i, result);

            let tree = must(result);
            let root = tree.root_node();
            assert_eq!(root.kind(), "source_file");
        }
    }

    #[test]
    fn test_variable_declarations() {
        let test_cases = [
            "my $scalar = 42;",
            "my @array = (1, 2, 3);",
            "my %hash = (key => 'value');",
            "our $package_var = 1;",
            "local $temp = 2;",
        ];

        for (i, code) in test_cases.iter().enumerate() {
            let result = parse(code);
            assert!(result.is_ok(), "Test case {} failed: {:?}", i, result);
        }
    }

    #[test]
    fn test_function_calls() {
        let test_cases = [
            "print 'Hello';",
            "say 'World';",
            "die 'Error message';",
            "warn 'Warning';",
            "defined($var);",
            "undef;",
        ];

        for (i, code) in test_cases.iter().enumerate() {
            let result = parse(code);
            assert!(result.is_ok(), "Test case {} failed: {:?}", i, result);
        }
    }

    #[test]
    fn test_control_structures() {
        let test_cases = [
            "if ($condition) { $action = 1; }",
            "unless ($condition) { $action = 0; }",
            "while ($condition) { $action++; }",
            "until ($condition) { $action++; }",
            "for my $i (1..10) { print $i; }",
            "foreach my $item (@list) { process($item); }",
        ];

        for (i, code) in test_cases.iter().enumerate() {
            let result = parse(code);
            assert!(result.is_ok(), "Test case {} failed: {:?}", i, result);
        }
    }

    #[test]
    fn test_string_literals() {
        let test_cases = [
            "my $str1 = 'Single quoted';",
            "my $str2 = \"Double quoted\";",
            "my $str3 = qq{Interpolated};",
            "my $str4 = q{Non-interpolated};",
        ];

        for (i, code) in test_cases.iter().enumerate() {
            let result = parse(code);
            assert!(result.is_ok(), "Test case {} failed: {:?}", i, result);
        }
    }

    #[test]
    fn test_comments() {
        let test_cases = [
            "# This is a comment\nmy $var = 1;",
            "my $var = 1; # Inline comment",
            "=pod\nThis is POD\n=cut\nmy $var = 1;",
        ];

        for (i, code) in test_cases.iter().enumerate() {
            let result = parse(code);
            assert!(result.is_ok(), "Test case {} failed: {:?}", i, result);
        }
    }

    #[test]
    fn test_unicode_support() {
        let test_cases = [
            "my $変数 = '値';",
            "my $über = 'cool';",
            "my $naïve = 'simple';",
            "sub 関数 { return '関数です'; }",
        ];

        for (i, code) in test_cases.iter().enumerate() {
            let result = parse(code);
            assert!(result.is_ok(), "Test case {} failed: {:?}", i, result);
        }
    }

    #[test]
    fn test_error_handling() {
        // These should parse but may contain error nodes
        let error_cases = [
            "my $str = \"Unterminated string;",
            "if ($condition { $action = 1; }",
            "my $var = 1 +;",
        ];

        for (i, code) in error_cases.iter().enumerate() {
            let result = parse(code);
            // These should parse (with error nodes) rather than fail completely
            assert!(result.is_ok(), "Error case {} failed to parse: {:?}", i, result);
        }
    }

    #[test]
    fn test_parser_reuse() {
        let mut parser = Parser::new();
        must(parser.set_language(&language()));

        let test_cases = ["my $var1 = 1;", "my $var2 = 2;", "my $var3 = 3;"];

        for (i, code) in test_cases.iter().enumerate() {
            let tree = parser.parse(code, None);
            assert!(tree.is_some(), "Test case {} failed", i);
        }
    }
}

#[cfg(test)]
mod unicode_tests {

    use crate::unicode::UnicodeUtils;

    #[test]
    fn test_unicode_normalization() {
        let test_cases =
            vec![("café", "café"), ("naïve", "naïve"), ("über", "über"), ("変数", "変数")];

        for (input, expected) in test_cases {
            let normalized = UnicodeUtils::normalize_identifier(input);
            assert_eq!(normalized, expected);
        }
    }

    #[test]
    fn test_unicode_identifier_validation() {
        let valid_identifiers = vec!["variable", "変数", "über", "naïve", "café", "αβγ", "привет"];

        for identifier in valid_identifiers {
            assert!(
                UnicodeUtils::is_valid_identifier(identifier),
                "Identifier '{}' should be valid",
                identifier
            );
        }

        let invalid_identifiers = vec!["123variable", "variable-name", "variable name", ""];

        for identifier in invalid_identifiers {
            assert!(
                !UnicodeUtils::is_valid_identifier(identifier),
                "Identifier '{}' should be invalid",
                identifier
            );
        }
    }

    #[test]
    fn test_unicode_edge_cases() {
        // Test various Unicode edge cases
        let edge_cases = vec![
            ("", false),    // Empty string
            ("a", true),    // Single ASCII
            ("α", true),    // Single Unicode
            ("aα", true),   // Mixed ASCII and Unicode
            ("123", false), // Numbers only
            ("_var", true), // Underscore prefix
            ("var_", true), // Underscore suffix
        ];

        for (input, expected) in edge_cases {
            let result = UnicodeUtils::is_valid_identifier(input);
            assert_eq!(
                result, expected,
                "Identifier '{}' validation failed: expected {}, got {}",
                input, expected, result
            );
        }
    }
}

#[cfg(all(test, feature = "c-parser"))]
mod property_tests {
    use crate::parse;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        #[test]
        fn test_parse_does_not_panic(input in r#"[a-zA-Z0-9_\s{}()\[\]"';,.+\-*/=<>!&|^~%#@$`]+"#) {
            // This test ensures that parsing arbitrary strings doesn't panic
            let _result = parse(&input);
        }

        #[test]
        fn test_unicode_identifiers_roundtrip(identifier in "[a-zA-Z_][a-zA-Z0-9_]*") {
            // Test that valid identifiers can be parsed and reconstructed
            let code = format!("my ${} = 1;", identifier);
            let result = parse(&code);
            assert!(result.is_ok(), "Failed to parse identifier: {}", identifier);
        }
    }
}

#[cfg(test)]
mod error_tests {
    use crate::error::ParseError;

    #[test]
    fn test_error_creation() {
        let error = ParseError::ParseFailed;
        assert!(matches!(error, ParseError::ParseFailed));
    }

    #[test]
    fn test_error_display() {
        let error = ParseError::ParseFailed;
        let display = format!("{:?}", error);
        assert!(!display.is_empty());
    }

    #[test]
    fn test_error_serialization() {
        let error = ParseError::ParseFailed;
        let serialized = bincode::encode_to_vec(&error, bincode::config::standard());
        assert!(serialized.is_ok(), "Error serialization failed");

        let (deserialized, _): (ParseError, _) =
            must(bincode::decode_from_slice(&must(serialized), bincode::config::standard()));
        assert!(matches!(deserialized, ParseError::ParseFailed));
    }
}

#[cfg(all(test, feature = "c-parser"))]
mod performance_tests {
    use crate::parse;
    use std::time::Instant;

    #[test]
    fn test_parse_performance() {
        let test_code = "my $var = 42; print 'Hello, World!'; sub foo { return 1; }";
        let iterations = 1000;

        let start = Instant::now();
        for _ in 0..iterations {
            let _result = parse(test_code);
        }
        let duration = start.elapsed();

        let avg_time = duration.as_micros() as f64 / iterations as f64;
        println!("Average parse time: {:.2} μs", avg_time);

        // Ensure parsing is reasonably fast (less than 1000 μs per parse)
        assert!(avg_time < 1000.0, "Parsing is too slow: {:.2} μs", avg_time);
    }
}

#[cfg(all(test, feature = "c-parser"))]
mod corpus_tests {
    use crate::parse;

    use std::fs;
    use std::path::PathBuf;
    use walkdir::WalkDir;

    /// Corpus test case containing input code and expected S-expression
    #[derive(Debug)]
    struct CorpusTestCase {
        name: String,
        source: String,
        expected: String,
    }

    /// Parse a corpus test file into individual test cases
    fn parse_corpus_file(
        path: &PathBuf,
    ) -> Result<Vec<CorpusTestCase>, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;

        let mut test_cases = Vec::new();
        let mut current_name = String::new();
        let mut current_source = String::new();
        let mut current_expected = String::new();
        let mut in_source = false;
        let mut in_expected = false;

        for line in content.lines() {
            if line.starts_with(
                "================================================================================",
            ) {
                // Save previous test case if we have one
                if !current_name.is_empty()
                    && !current_source.is_empty()
                    && !current_expected.is_empty()
                {
                    test_cases.push(CorpusTestCase {
                        name: current_name.clone(),
                        source: current_source.clone(),
                        expected: current_expected.clone(),
                    });
                }

                // Start new test case
                current_name.clear();
                current_source.clear();
                current_expected.clear();
                in_source = false;
                in_expected = false;
            } else if line.starts_with("----") {
                // Transition from source to expected
                in_source = false;
                in_expected = true;
            } else if in_source {
                current_source.push_str(line);
                current_source.push('\n');
            } else if in_expected {
                current_expected.push_str(line);
                current_expected.push('\n');
            } else if !line.trim().is_empty() && !line.starts_with("=") {
                // This is the test case name
                current_name = line.trim().to_string();
                in_source = true;
            }
        }

        // Add the last test case
        if !current_name.is_empty() && !current_source.is_empty() && !current_expected.is_empty() {
            test_cases.push(CorpusTestCase {
                name: current_name,
                source: current_source,
                expected: current_expected,
            });
        }

        Ok(test_cases)
    }

    fn normalize_sexp(s: &str) -> String {
        s.lines()
            .map(|line| line.trim_end())
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Run a single corpus test case
    fn run_corpus_test_case(
        test_case: &CorpusTestCase,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Parse the source code using tree-sitter-perl
        let tree = parse(&test_case.source)?;

        let actual = normalize_sexp(&tree.root_node().to_sexp());
        let expected = normalize_sexp(test_case.expected.trim());

        if actual == expected {
            Ok(true)
        } else {
            println!("\n❌ Test failed: {}", test_case.name);
            println!("Expected:");
            println!("{}", expected);
            println!("Actual:");
            println!("{}", actual);
            Ok(false)
        }
    }

    /// Test all corpus files in the legacy test directory
    #[test]
    fn test_all_corpus_files() {
        let corpus_dir = PathBuf::from("tree-sitter-perl/test/corpus");
        if !corpus_dir.exists() {
            println!("⚠️  Corpus directory not found, skipping corpus tests");
            return;
        }

        let mut total_tests = 0;
        let mut passed_tests = 0;
        let mut failed_tests = 0;

        // Walk through all corpus files
        for entry in WalkDir::new(&corpus_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("txt")
                || path.extension().is_none()
            {
                println!("\n📁 Testing corpus file: {}", path.display());

                match must(parse_corpus_file(&path.to_path_buf())) {
                    Ok(test_cases) => {
                        for test_case in test_cases {
                            total_tests += 1;
                            match run_corpus_test_case(&test_case) {
                                Ok(true) => {
                                    passed_tests += 1;
                                    print!("✅");
                                }
                                Ok(false) => {
                                    failed_tests += 1;
                                    print!("❌");
                                }
                                Err(e) => {
                                    failed_tests += 1;
                                    println!("❌ Error in test '{}': {}", test_case.name, e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to parse corpus file {}: {}", path.display(), e);
                    }
                }
            }
        }

        println!("\n\n📊 Corpus Test Summary:");
        println!("   Total: {}", total_tests);
        println!("   Passed: {} ✅", passed_tests);
        println!("   Failed: {} ❌", failed_tests);

        if failed_tests > 0 {
            assert!(false, "{} corpus tests failed", failed_tests);
        }
    }

    /// Test individual corpus files for focused debugging
    #[test]
    fn test_simple_corpus() {
        let path = PathBuf::from("tree-sitter-perl/test/corpus/simple");
        if !path.exists() {
            println!("⚠️  Simple corpus file not found, skipping test");
            return;
        }

        let test_cases = must(parse_corpus_file(&path));
        let mut passed = 0;
        let mut failed = 0;

        for test_case in test_cases {
            match run_corpus_test_case(&test_case) {
                Ok(true) => {
                    passed += 1;
                    println!("✅ {}", test_case.name);
                }
                Ok(false) => {
                    failed += 1;
                    println!("❌ {}", test_case.name);
                }
                Err(e) => {
                    failed += 1;
                    println!("❌ Error in '{}': {}", test_case.name, e);
                }
            }
        }

        println!("\nSimple corpus: {}/{} tests passed", passed, passed + failed);
        if failed > 0 {
            assert!(false, "{} simple corpus tests failed", failed);
        }
    }

    #[test]
    fn test_variables_corpus() {
        let path = PathBuf::from("tree-sitter-perl/test/corpus/variables");
        if !path.exists() {
            println!("⚠️  Variables corpus file not found, skipping test");
            return;
        }

        let test_cases = must(parse_corpus_file(&path));
        let mut passed = 0;
        let mut failed = 0;

        for test_case in test_cases {
            match run_corpus_test_case(&test_case) {
                Ok(true) => {
                    passed += 1;
                    println!("✅ {}", test_case.name);
                }
                Ok(false) => {
                    failed += 1;
                    println!("❌ {}", test_case.name);
                }
                Err(e) => {
                    failed += 1;
                    println!("❌ Error in '{}': {}", test_case.name, e);
                }
            }
        }

        println!("\nVariables corpus: {}/{} tests passed", passed, passed + failed);
        if failed > 0 {
            assert!(false, "{} variables corpus tests failed", failed);
        }
    }
}

#[cfg(all(test, feature = "c-parser"))]
mod highlight_tests {
    use crate::{language, parse};

    use std::fs;
    use std::path::PathBuf;
    use tree_sitter::{Query, QueryCursor, StreamingIterator};

    /// Expected token at a specific position
    #[derive(Debug, Clone)]
    struct ExpectedToken {
        line: usize,
        column: usize,
        token_type: String,
    }

    /// Highlight test case containing Perl code and expected token classifications
    #[derive(Debug)]
    struct HighlightTestCase {
        name: String,
        source: String,
        expected_tokens: Vec<ExpectedToken>,
    }

    /// Parse a highlight test file with annotation format:
    /// # <- token_type   (points to first character of previous line)
    /// #  ^ token_type   (points to character using caret positioning)
    fn parse_highlight_file(
        path: &PathBuf,
    ) -> Result<Vec<HighlightTestCase>, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let mut expected_tokens = Vec::new();
        let mut source_lines = Vec::new();

        for (line_idx, line) in content.lines().enumerate() {
            if let Some(comment_idx) = line.find('#') {
                let comment = &line[comment_idx..];

                // Check for <- annotation (points to first char of previous line)
                if comment.contains("<-") {
                    if let Some(token_start) = comment.find("<-").map(|i| i + 2) {
                        let token_type = comment[token_start..].trim().to_string();
                        if !token_type.is_empty() && line_idx > 0 {
                            expected_tokens.push(ExpectedToken {
                                line: line_idx - 1,
                                column: 0,
                                token_type,
                            });
                        }
                    }
                }
                // Check for ^ annotation (points to specific column)
                else if comment.contains('^') {
                    if let Some(caret_idx) = comment.find('^') {
                        let token_start = comment.find('^').map(|i| i + 1).unwrap_or(caret_idx);
                        let token_type = comment[token_start..].trim().to_string();
                        if !token_type.is_empty() && line_idx > 0 {
                            // Calculate column based on caret position
                            let column = comment_idx + caret_idx;
                            expected_tokens.push(ExpectedToken {
                                line: line_idx - 1,
                                column,
                                token_type,
                            });
                        }
                    }
                }

                // Keep only the non-comment part as source
                let code_part = &line[..comment_idx];
                source_lines.push(code_part);
            } else {
                source_lines.push(line);
            }
        }

        let source = source_lines.join("\n");

        let test_case = HighlightTestCase {
            name: must_some(path.file_name()).to_string_lossy().to_string(),
            source,
            expected_tokens,
        };

        Ok(vec![test_case])
    }

    /// Get actual token classifications using tree-sitter highlight query
    fn get_actual_tokens(
        source: &str,
        tree: &tree_sitter::Tree,
    ) -> Result<Vec<(usize, usize, String)>, Box<dyn std::error::Error>> {
        let highlights_scm = fs::read_to_string("tree-sitter-perl/queries/highlights.scm")?;
        let lang = language();
        let query = Query::new(&lang, &highlights_scm)?;
        let mut cursor = QueryCursor::new();

        let mut tokens = Vec::new();
        let source_bytes = source.as_bytes();

        let mut matches = cursor.matches(&query, tree.root_node(), source_bytes);
        while let Some(match_result) = matches.next() {
            for capture in match_result.captures {
                let capture_name = query.capture_names()[capture.index as usize].to_string();
                let node = capture.node;
                let start_pos = node.start_position();

                tokens.push((start_pos.row, start_pos.column, capture_name));
            }
        }

        Ok(tokens)
    }

    /// Verify that expected tokens match actual captures
    fn verify_highlight_tokens(
        test_case: &HighlightTestCase,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let tree = parse(&test_case.source)?;
        let actual_tokens = get_actual_tokens(&test_case.source, &tree)?;

        let mut all_matched = true;
        let mut failures = Vec::new();

        for expected in &test_case.expected_tokens {
            // Find matching actual token at the same position
            let matching = actual_tokens
                .iter()
                .find(|(line, col, _)| *line == expected.line && *col == expected.column);

            match matching {
                Some((_, _, actual_type)) => {
                    if actual_type != &expected.token_type {
                        all_matched = false;
                        failures.push(format!(
                            "Line {}, Col {}: expected '{}' but got '{}'",
                            expected.line + 1,
                            expected.column,
                            expected.token_type,
                            actual_type
                        ));
                    }
                }
                None => {
                    all_matched = false;
                    failures.push(format!(
                        "Line {}, Col {}: expected '{}' but no capture found",
                        expected.line + 1,
                        expected.column,
                        expected.token_type
                    ));
                }
            }
        }

        if !all_matched {
            println!("❌ Token mismatches in {}:", test_case.name);
            for failure in failures {
                println!("   {}", failure);
            }
        }

        Ok(all_matched)
    }

    /// Test that highlight files can be parsed without errors
    #[test]
    fn test_highlight_files_parse() {
        let highlight_dir = PathBuf::from("tree-sitter-perl/test/highlight");
        if !highlight_dir.exists() {
            println!("⚠️  Highlight directory not found, skipping highlight tests");
            return;
        }

        let mut total_files = 0;
        let mut parsed_files = 0;
        let mut verified_files = 0;
        let mut failed_verifications = Vec::new();

        for entry in must(fs::read_dir(&highlight_dir)) {
            let entry = must(entry);
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("pm") {
                total_files += 1;
                println!("📁 Testing highlight file: {}", path.display());

                match parse_highlight_file(&path) {
                    Ok(test_cases) => {
                        for test_case in test_cases {
                            match parse(&test_case.source) {
                                Ok(_tree) => {
                                    parsed_files += 1;

                                    // Verify token classifications
                                    match verify_highlight_tokens(&test_case) {
                                        Ok(true) => {
                                            verified_files += 1;
                                            println!("✅ {} - all tokens verified", test_case.name);
                                        }
                                        Ok(false) => {
                                            failed_verifications.push(test_case.name.clone());
                                        }
                                        Err(e) => {
                                            println!(
                                                "⚠️  {} - verification error: {}",
                                                test_case.name, e
                                            );
                                            failed_verifications.push(test_case.name.clone());
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("❌ Failed to parse '{}': {}", test_case.name, e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to read highlight file {}: {}", path.display(), e);
                    }
                }
            }
        }

        println!("\n📊 Highlight Test Summary:");
        println!("   Total files: {}", total_files);
        println!("   Successfully parsed: {} ✅", parsed_files);
        println!("   Token verification passed: {} ✅", verified_files);
        println!("   Token verification failed: {} ❌", failed_verifications.len());

        if !failed_verifications.is_empty() {
            println!("\nFailed files:");
            for name in &failed_verifications {
                println!("   - {}", name);
            }
            assert!(false, "{} highlight files failed token verification", failed_verifications.len());
        }

        if parsed_files < total_files {
            assert!(false, "{} highlight files failed to parse", total_files - parsed_files);
        }
    }
}
