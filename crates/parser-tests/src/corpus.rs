//! Corpus test loader
//! 
//! Loads test cases from the tree-sitter corpus format

use crate::TestCase;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Parse a tree-sitter corpus file into test cases
pub fn parse_corpus_file(content: &str) -> Result<Vec<TestCase>> {
    let mut tests = Vec::new();
    let mut current_test: Option<TestCase> = None;
    let mut in_input = false;
    let mut in_output = false;
    let mut input_lines = Vec::new();
    let mut output_lines = Vec::new();
    
    for line in content.lines() {
        if line.starts_with("===") || line.starts_with("---") {
            // New test separator
            if let Some(mut test) = current_test.take() {
                test.input = input_lines.join("\n");
                test.expected_sexp = if output_lines.is_empty() {
                    None
                } else {
                    Some(output_lines.join("\n"))
                };
                tests.push(test);
            }
            
            // Reset for new test
            input_lines.clear();
            output_lines.clear();
            in_input = false;
            in_output = false;
            
            if line.starts_with("===") {
                let name = line.trim_start_matches('=').trim();
                current_test = Some(TestCase {
                    name: name.to_string(),
                    input: String::new(),
                    description: None,
                    should_parse: true,
                    expected_sexp: None,
                });
            }
        } else if line == "---" {
            in_input = true;
            in_output = false;
        } else if line == "---" && in_input {
            in_input = false;
            in_output = true;
        } else if in_input {
            input_lines.push(line);
        } else if in_output {
            output_lines.push(line);
        }
    }
    
    // Handle last test
    if let Some(mut test) = current_test {
        test.input = input_lines.join("\n");
        test.expected_sexp = if output_lines.is_empty() {
            None
        } else {
            Some(output_lines.join("\n"))
        };
        tests.push(test);
    }
    
    Ok(tests)
}

/// Load all corpus tests from a directory
pub fn load_corpus_directory(dir: &Path) -> Result<Vec<TestCase>> {
    let mut all_tests = Vec::new();
    
    for entry in fs::read_dir(dir).context("Failed to read corpus directory")? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("txt") {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read {:?}", path))?;
            
            let mut tests = parse_corpus_file(&content)
                .with_context(|| format!("Failed to parse {:?}", path))?;
            
            // Add filename to test names for uniqueness
            let filename = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            
            for test in &mut tests {
                test.name = format!("{}::{}", filename, test.name);
            }
            
            all_tests.extend(tests);
        }
    }
    
    Ok(all_tests)
}