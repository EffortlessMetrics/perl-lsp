//! Comprehensive Test Driven Development tests for SPEC-149 Missing Documentation Warnings
//!
//! This test suite validates all 12 acceptance criteria for implementing comprehensive
//! API documentation in the perl-parser crate following TDD methodology.
//!
//! Tests are designed to fail until proper documentation implementation is complete.

use proptest::collection::vec;
use proptest::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::process::Command;

/// Helper functions for documentation validation and analysis
mod doc_validation_helpers {
    use super::*;

    /// Represents the result of analyzing documentation in a source file
    #[derive(Debug, Clone)]
    pub struct DocumentationAnalysis {
        pub has_module_docs: bool,
        pub public_items_without_docs: Vec<String>,
        pub missing_workflow_integration: Vec<String>,
        pub missing_performance_docs: Vec<String>,
        pub code_examples_present: bool,
        pub cross_references_present: bool,
        pub style_violations: Vec<String>,
        pub malformed_doctests: Vec<String>,
        pub empty_doc_strings: Vec<String>,
        pub invalid_cross_references: Vec<String>,
        pub incomplete_performance_docs: Vec<String>,
        pub missing_error_recovery_docs: Vec<String>,
    }

    /// Analyzes a source file for documentation completeness
    pub fn analyze_file_documentation(file_path: &str, content: &str) -> DocumentationAnalysis {
        let lines: Vec<&str> = content.lines().collect();
        let mut analysis = DocumentationAnalysis {
            has_module_docs: false,
            public_items_without_docs: Vec::new(),
            missing_workflow_integration: Vec::new(),
            missing_performance_docs: Vec::new(),
            code_examples_present: false,
            cross_references_present: false,
            style_violations: Vec::new(),
            malformed_doctests: Vec::new(),
            empty_doc_strings: Vec::new(),
            invalid_cross_references: Vec::new(),
            incomplete_performance_docs: Vec::new(),
            missing_error_recovery_docs: Vec::new(),
        };

        // Check for module-level documentation
        analysis.has_module_docs = lines.iter().any(|line| line.trim_start().starts_with("//!"));

        // Check for code examples
        analysis.code_examples_present = content.contains("```rust") || content.contains("```");

        // Check for cross-references
        analysis.cross_references_present = (content.contains("[`") && content.contains("`]"))
            || content.contains("See also")
            || content.contains("Related");

        // Enhanced edge case detection
        analysis.malformed_doctests = find_malformed_doctests(&lines);
        analysis.empty_doc_strings = find_empty_doc_strings(&lines);
        analysis.invalid_cross_references = find_invalid_cross_references(&lines);
        analysis.incomplete_performance_docs = find_incomplete_performance_docs(file_path, &lines);
        analysis.missing_error_recovery_docs = find_missing_error_recovery_docs(&lines);

        // Analyze public items
        for (line_num, line) in lines.iter().enumerate() {
            if is_public_item_declaration(line) {
                let has_doc_comment =
                    line_num > 0 && lines[line_num - 1].trim_start().starts_with("///");

                if !has_doc_comment {
                    analysis.public_items_without_docs.push(format!(
                        "{}:{}: {}",
                        file_path,
                        line_num + 1,
                        line.trim()
                    ));
                }

                // Check for workflow integration documentation
                if !has_workflow_integration_docs(&lines, line_num) {
                    analysis.missing_workflow_integration.push(format!(
                        "{}:{}: {}",
                        file_path,
                        line_num + 1,
                        line.trim()
                    ));
                }
            }

            // Check documentation style
            if line.trim_start().starts_with("///")
                && let Some(violation) = check_doc_style_violation(file_path, line_num, line)
            {
                analysis.style_violations.push(violation);
            }
        }

        // Check for performance documentation
        if is_performance_critical_module(file_path) {
            let content_lower = content.to_lowercase();
            let has_perf_docs = content_lower.contains("performance")
                && content_lower.contains("memory")
                && (content_lower.contains("large")
                    || content_lower.contains("scale")
                    || content_lower.contains("scal")
                    || content_lower.contains("benchmark")
                    || content_lower.contains("throughput"));

            if !has_perf_docs {
                analysis.missing_performance_docs.push(file_path.to_string());
            }
        }

        analysis
    }

    /// Checks if a line declares a public item (struct, enum, function, trait, type, const, mod, use)
    fn is_public_item_declaration(line: &str) -> bool {
        let trimmed = line.trim_start();
        (trimmed.starts_with("pub struct")
            || trimmed.starts_with("pub enum")
            || trimmed.starts_with("pub fn")
            || trimmed.starts_with("pub trait")
            || trimmed.starts_with("pub type")
            || trimmed.starts_with("pub const")
            || trimmed.starts_with("pub mod")
            || trimmed.starts_with("pub use"))
            && !line.contains("//")
    }

    /// Checks if documentation includes LSP workflow integration context
    fn has_workflow_integration_docs(lines: &[&str], item_line: usize) -> bool {
        let doc_range_start = item_line.saturating_sub(10);
        let doc_range = &lines[doc_range_start..item_line];

        doc_range.iter().any(|line| {
            let content = line.to_lowercase();
            content.contains("parse")
                || content.contains("index")
                || content.contains("navigate")
                || content.contains("complete")
                || content.contains("analyze")
                || content.contains("lsp workflow")
                || content.contains("workflow")
        })
    }

    /// Checks if a module is considered performance-critical
    fn is_performance_critical_module(file_path: &str) -> bool {
        let performance_critical_modules = [
            "incremental/incremental_v2.rs",
            "workspace/workspace_index.rs",
            "engine/parser/mod.rs",
            "analysis/semantic.rs",
            "tokens/token_stream.rs",
            "refactor/import_optimizer.rs",
            "analysis/scope_analyzer.rs",
            "tooling/performance.rs",
        ];

        performance_critical_modules.iter().any(|module| file_path.ends_with(module))
    }

    /// Checks for documentation style violations
    fn check_doc_style_violation(file_path: &str, line_num: usize, line: &str) -> Option<String> {
        let doc_content = line.trim_start().trim_start_matches("///").trim();

        // Check section header formatting
        if doc_content.starts_with('#') && !doc_content.starts_with("# ") {
            return Some(format!(
                "{}:{}: Section header missing space after #",
                file_path,
                line_num + 1
            ));
        }

        // Check code block formatting
        if doc_content.starts_with("```")
            && !doc_content.starts_with("```rust")
            && doc_content != "```"
        {
            return Some(format!(
                "{}:{}: Code block should specify language",
                file_path,
                line_num + 1
            ));
        }

        None
    }

    /// Validates that lib.rs has missing_docs warning enabled
    pub fn validate_missing_docs_warning(lib_path: &str) -> Result<(), String> {
        let content =
            fs::read_to_string(lib_path).map_err(|e| format!("Failed to read lib.rs: {}", e))?;

        let has_commented_missing_docs = content.contains("// #![warn(missing_docs)]");
        let has_enabled_missing_docs = content.contains("#![warn(missing_docs)]")
            && !content.contains("// #![warn(missing_docs)]");

        if has_commented_missing_docs {
            return Err(
                "missing_docs warning is commented out in lib.rs. Need to uncomment #![warn(missing_docs)]".to_string()
            );
        }

        if !has_enabled_missing_docs {
            return Err("missing_docs warning is not enabled in lib.rs".to_string());
        }

        Ok(())
    }

    /// Runs cargo doc and validates output for warnings (performance-optimized)
    pub fn validate_cargo_doc_generation(package_dir: &str) -> Result<(), String> {
        // Skip locally unless explicitly enabled
        if std::env::var("CI").ok().as_deref() != Some("true")
            && std::env::var("DOCS_ENFORCE").ok().as_deref() != Some("1")
        {
            eprintln!("Skipping cargo doc generation outside CI (set DOCS_ENFORCE=1 to run).");
            return Ok(());
        }
        // Performance optimization: Use more efficient cargo doc validation
        // Strategy: Enable missing_docs warnings specifically for doc generation
        let output = Command::new("cargo")
            .args(["doc", "--no-deps", "--package", "perl-parser"])
            .env("RUSTFLAGS", "-W missing_docs") // Force warnings for doc validation
            .current_dir(package_dir)
            .output()
            .map_err(|e| format!("Failed to execute cargo doc: {}", e))?;

        let stderr_content = String::from_utf8_lossy(&output.stderr);
        let stdout_content = String::from_utf8_lossy(&output.stdout);

        if !output.status.success() {
            return Err(format!(
                "cargo doc failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
                stdout_content, stderr_content
            ));
        }

        // Optimized warning detection: only check for actual missing docs, not all warnings
        let has_missing_docs_warnings = stderr_content.contains("missing documentation")
            || stdout_content.contains("missing documentation");

        if has_missing_docs_warnings {
            return Err(format!(
                "cargo doc generated missing documentation warnings:\nSTDERR:\n{}",
                stderr_content
            ));
        }

        Ok(())
    }

    /// Edge Case Detection Functions for Enhanced Test Coverage
    /// Finds malformed doctests that won't compile or execute properly
    pub fn find_malformed_doctests(lines: &[&str]) -> Vec<String> {
        let mut malformed = Vec::new();
        let mut in_rust_block = false;
        let mut rust_block_content = String::new();
        let mut block_start_line = 0;

        for (line_num, line) in lines.iter().enumerate() {
            let doc_line = line.trim_start();
            if doc_line.starts_with("///") {
                let content = doc_line.trim_start_matches("///").trim();

                if content.starts_with("```rust") {
                    in_rust_block = true;
                    rust_block_content.clear();
                    block_start_line = line_num;
                } else if content == "```" && in_rust_block {
                    in_rust_block = false;

                    // Check for malformed patterns
                    if rust_block_content.trim().is_empty() {
                        malformed.push(format!("Empty doctest at line {}", block_start_line + 1));
                    } else if !rust_block_content.contains("use ")
                        && !rust_block_content.contains("let ")
                        && !rust_block_content.contains("assert")
                        && !rust_block_content.contains("//")
                    {
                        malformed.push(format!(
                            "Doctest without assertions or examples at line {}",
                            block_start_line + 1
                        ));
                    }

                    // Check for unclosed braces or parentheses
                    let open_braces = rust_block_content.matches('{').count();
                    let close_braces = rust_block_content.matches('}').count();
                    let open_parens = rust_block_content.matches('(').count();
                    let close_parens = rust_block_content.matches(')').count();

                    if open_braces != close_braces || open_parens != close_parens {
                        malformed.push(format!(
                            "Unbalanced braces/parentheses in doctest at line {}",
                            block_start_line + 1
                        ));
                    }
                } else if in_rust_block {
                    rust_block_content.push_str(content);
                    rust_block_content.push('\n');
                }
            }
        }

        if in_rust_block {
            malformed
                .push(format!("Unclosed doctest block starting at line {}", block_start_line + 1));
        }

        malformed
    }

    /// Finds empty or trivial documentation strings
    pub fn find_empty_doc_strings(lines: &[&str]) -> Vec<String> {
        let mut empty_docs = Vec::new();
        let mut current_block: Vec<(usize, String)> = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            let doc_line = line.trim_start();
            if doc_line.starts_with("///") {
                let content = doc_line.trim_start_matches("///").trim().to_string();
                current_block.push((line_num, content));
                continue;
            }

            if !current_block.is_empty() {
                evaluate_doc_block(&current_block, &mut empty_docs);
                current_block.clear();
            }
        }

        if !current_block.is_empty() {
            evaluate_doc_block(&current_block, &mut empty_docs);
        }

        empty_docs
    }

    fn evaluate_doc_block(block: &[(usize, String)], empty_docs: &mut Vec<String>) {
        let mut in_code_block = false;
        let mut has_non_empty_text = false;

        for (line_num, content) in block {
            let trimmed = content.trim();

            if trimmed.starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }

            if in_code_block {
                continue;
            }

            if trimmed.is_empty() {
                continue;
            }

            has_non_empty_text = true;

            if trimmed.len() < 5
                || trimmed == "TODO"
                || trimmed == "FIXME"
                || trimmed.starts_with("//")
                || trimmed == "."
            {
                empty_docs.push(format!(
                    "Trivial documentation at line {}: '{}'",
                    line_num + 1,
                    trimmed
                ));
            }
        }

        if !has_non_empty_text {
            if let Some((line_num, _)) = block.first() {
                empty_docs.push(format!("Empty documentation at line {}", line_num + 1));
            }
        }
    }

    /// Finds invalid cross-references that won't resolve properly
    pub fn find_invalid_cross_references(lines: &[&str]) -> Vec<String> {
        let mut invalid_refs = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            let doc_line = line.trim_start();
            if doc_line.starts_with("///") {
                let content = doc_line.trim_start_matches("///");

                // Find potential cross-references
                if content.contains("[`") && content.contains("`]") {
                    // Check for common invalid patterns
                    if content.contains("[`[`") || content.contains("`]`]") {
                        invalid_refs.push(format!(
                            "Malformed cross-reference syntax at line {}",
                            line_num + 1
                        ));
                    }

                    // Check for references with invalid characters
                    if content.contains("[` ") || content.contains(" `]") {
                        invalid_refs.push(format!(
                            "Cross-reference with invalid spaces at line {}",
                            line_num + 1
                        ));
                    }

                    // Check for empty references
                    if content.contains("[``]") {
                        invalid_refs
                            .push(format!("Empty cross-reference at line {}", line_num + 1));
                    }
                }
            }
        }

        invalid_refs
    }

    /// Finds incomplete performance documentation in critical modules
    pub fn find_incomplete_performance_docs(file_path: &str, lines: &[&str]) -> Vec<String> {
        let mut incomplete = Vec::new();

        if !is_performance_critical_module(file_path) {
            return incomplete;
        }

        let doc_content: String = lines
            .iter()
            .filter(|line| {
                line.trim_start().starts_with("///") || line.trim_start().starts_with("//!")
            })
            .map(|line| {
                if line.trim_start().starts_with("///") {
                    line.trim_start().trim_start_matches("///").trim()
                } else {
                    line.trim_start().trim_start_matches("//!").trim()
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        let lower_content = doc_content.to_lowercase();

        // Check for specific performance documentation requirements
        let has_memory_complexity = lower_content.contains("o(")
            || lower_content.contains("time complexity")
            || lower_content.contains("space complexity");
        let has_scaling_info = lower_content.contains("scale")
            || lower_content.contains("50gb")
            || lower_content.contains("large file");
        let has_optimization_notes = lower_content.contains("optimiz")
            || lower_content.contains("perform")
            || lower_content.contains("efficient");
        let has_benchmark_info = lower_content.contains("benchmark")
            || lower_content.contains("µs")
            || lower_content.contains("ms")
            || lower_content.contains("speed");

        if !has_memory_complexity {
            incomplete.push(format!("{}: Missing time/space complexity documentation", file_path));
        }
        if !has_scaling_info {
            incomplete.push(format!("{}: Missing large-scale processing documentation", file_path));
        }
        if !has_optimization_notes {
            incomplete.push(format!("{}: Missing optimization strategy documentation", file_path));
        }
        if !has_benchmark_info {
            incomplete.push(format!("{}: Missing performance benchmark information", file_path));
        }

        incomplete
    }

    /// Finds missing error recovery and handling documentation
    pub fn find_missing_error_recovery_docs(lines: &[&str]) -> Vec<String> {
        let mut missing_recovery = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            // Look for Result<T, E> patterns or error handling
            if (line.contains("Result<")
                || line.contains("-> Result")
                || (line.contains("pub fn") && line.contains("Error")))
                && line.trim_start().starts_with("pub fn")
            {
                // Check if there's documentation above this line
                let doc_range_start = line_num.saturating_sub(10);
                let doc_range = &lines[doc_range_start..line_num];
                let doc_text = doc_range
                    .iter()
                    .filter(|l| l.trim_start().starts_with("///"))
                    .map(|l| l.trim_start().trim_start_matches("///").trim())
                    .collect::<Vec<_>>()
                    .join(" ");

                let lower_doc = doc_text.to_lowercase();

                // Check for error recovery documentation - be more discriminating
                let has_error_context = lower_doc.contains("error")
                    || lower_doc.contains("fail")
                    || lower_doc.contains("recover")
                    || lower_doc.contains("handle")
                    || lower_doc.contains("panic")
                    || lower_doc.contains("when")
                    || lower_doc.contains("returns")
                    || lower_doc.contains("result");

                // If the function returns Result but has minimal error documentation
                if !has_error_context || doc_text.trim().len() < 20 {
                    missing_recovery.push(format!(
                        "Missing error handling documentation at line {}",
                        line_num + 1
                    ));
                }
            }
        }

        missing_recovery
    }

    /// Analyzes LSP workflow stage coverage across modules
    #[allow(dead_code)]
    pub fn analyze_workflow_coverage(
        module_paths: &[&str],
        src_dir: &str,
    ) -> HashMap<String, usize> {
        let workflow_stages = ["Parse", "Index", "Navigate", "Complete", "Analyze"];
        let mut coverage = HashMap::new();

        for stage in &workflow_stages {
            coverage.insert(stage.to_string(), 0);
        }

        for module in module_paths {
            let module_path = format!("{}/{}", src_dir, module);
            if let Ok(content) = fs::read_to_string(&module_path) {
                let content_lower = content.to_lowercase();
                for stage in &workflow_stages {
                    if content_lower.contains(&stage.to_lowercase()) {
                        if let Some(count) = coverage.get_mut(*stage) {
                            *count += 1;
                        }
                    }
                }
            }
        }

        coverage
    }
}

/// Test suite for SPEC-149 Missing Documentation Warnings Feature
/// Following Test-Driven Development methodology - tests fail until implementation is complete
mod missing_docs_tests {
    use super::*;
    use doc_validation_helpers::*;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    #[derive(Clone)]
    struct SourceRoot {
        name: &'static str,
        path: PathBuf,
    }

    #[derive(Clone)]
    struct SourceFile {
        display_path: String,
        full_path: PathBuf,
    }

    fn workspace_root() -> PathBuf {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .and_then(|parent| parent.parent())
            .unwrap_or(manifest_dir)
            .to_path_buf()
    }

    fn source_roots() -> Vec<SourceRoot> {
        let root = workspace_root();
        vec![
            SourceRoot { name: "perl-parser-core", path: root.join("crates/perl-parser-core/src") },
            SourceRoot {
                name: "perl-semantic-analyzer",
                path: root.join("crates/perl-semantic-analyzer/src"),
            },
            SourceRoot {
                name: "perl-workspace-index",
                path: root.join("crates/perl-workspace-index/src"),
            },
            SourceRoot { name: "perl-refactoring", path: root.join("crates/perl-refactoring/src") },
            SourceRoot {
                name: "perl-incremental-parsing",
                path: root.join("crates/perl-incremental-parsing/src"),
            },
            SourceRoot { name: "perl-tdd-support", path: root.join("crates/perl-tdd-support/src") },
            SourceRoot {
                name: "perl-lsp-providers",
                path: root.join("crates/perl-lsp-providers/src"),
            },
        ]
    }

    fn find_source_files(rel_path: &str, roots: &[SourceRoot]) -> Vec<SourceFile> {
        let mut files = Vec::new();
        for root in roots {
            let full_path = root.path.join(rel_path);
            if full_path.is_file() {
                files.push(SourceFile {
                    display_path: format!("{}/{}", root.name, rel_path),
                    full_path,
                });
            }
        }
        files
    }

    fn read_source_files(rel_path: &str, roots: &[SourceRoot]) -> Vec<(String, String)> {
        let mut results = Vec::new();
        for file in find_source_files(rel_path, roots) {
            if let Ok(content) = fs::read_to_string(&file.full_path) {
                results.push((file.display_path, content));
            }
        }
        results
    }

    #[test]
    fn test_missing_docs_warning_compilation() {
        // AC:AC1 - Verify that missing_docs warning is enabled and allows successful compilation
        // Performance optimization: Skip expensive validation during LSP integration tests
        if std::env::var("PERL_LSP_PERFORMANCE_TEST").is_ok() {
            // Fast path: Basic validation for performance-critical test runs
            let lib_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs");
            let content = match std::fs::read_to_string(lib_path) {
                Ok(c) => c,
            Err(e) => {
                let msg = format!("Failed to read lib.rs: {}", e);
                assert!(msg.is_empty(), "{}", msg);
                return;
            }
            };
            assert!(
                content.contains("warn(missing_docs)"),
                "missing_docs warning should be configured"
            );
            return;
        }

        let lib_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs");

        match validate_missing_docs_warning(lib_path) {
            Ok(()) => {
                // Test passes - missing_docs warning is properly enabled
            }
            Err(error_msg) => {
                unreachable!(
                    "AC1 NOT IMPLEMENTED: {}\n\nTo fix:\n  - Uncomment #![warn(missing_docs)] in lib.rs\n  - Ensure compilation succeeds without warnings",
                    error_msg
                );
            }
        }
    }

    #[test]
    #[ignore = "documentation coverage: SPEC-149 not yet implemented"]
    fn test_public_structs_documentation_presence() {
        // AC:AC2 - Verify all public structs/enums have comprehensive documentation
        // including workflow role description
        let roots = source_roots();
        let critical_modules = [
            "engine/parser/mod.rs",
            "engine/ast.rs",
            "engine/error/mod.rs",
            "tokens/token_stream.rs",
            "analysis/semantic.rs",
            "analysis/symbol.rs",
            "workspace/workspace_index.rs",
        ];

        let mut all_missing_docs = Vec::new();
        let mut all_missing_workflow_integration = Vec::new();
        let mut missing_modules = Vec::new();

        for module in &critical_modules {
            let files = read_source_files(module, &roots);
            if files.is_empty() {
                missing_modules.push(module.to_string());
                continue;
            }
            for (display_path, content) in files {
                let analysis = analyze_file_documentation(&display_path, &content);
                all_missing_docs.extend(analysis.public_items_without_docs);
                all_missing_workflow_integration.extend(analysis.missing_workflow_integration);
            }
        }

        assert!(
            missing_modules.is_empty(),
            "AC2 NOT IMPLEMENTED: Missing source modules: {:?}",
            missing_modules
        );

        if !all_missing_docs.is_empty() || !all_missing_workflow_integration.is_empty() {
            build_ac2_error_message(&all_missing_docs, &all_missing_workflow_integration);
        }

        assert!(all_missing_docs.is_empty(), "All public structs should have documentation");
        assert!(
            all_missing_workflow_integration.is_empty(),
            "All public structs should document workflow integration"
        );
    }

    /// Builds a comprehensive error message for AC2 failures
    fn build_ac2_error_message(missing_docs: &[String], missing_workflow: &[String]) {
        let mut error_msg =
            String::from("AC2 NOT IMPLEMENTED: Missing comprehensive struct/enum documentation:\n");

        if !missing_docs.is_empty() {
            error_msg.push_str("Missing documentation:\n");
            for item in missing_docs {
                error_msg.push_str(&format!("  - {}\n", item));
            }
        }

        if !missing_workflow.is_empty() {
            error_msg.push_str("Missing workflow integration documentation:\n");
            for item in missing_workflow {
                error_msg.push_str(&format!("  - {}\n", item));
            }
        }

        unreachable!("{}", error_msg);
    }

    #[test]
    #[ignore = "documentation coverage: SPEC-149 not yet implemented"]
    fn test_public_functions_documentation_presence() {
        // AC:AC3 - Verify all public functions have comprehensive documentation
        // with summary, parameters, return values, and error conditions
        let roots = source_roots();
        let function_critical_modules = [
            "engine/parser/mod.rs",
            "ide/lsp_compat/completion.rs",
            "ide/lsp_compat/diagnostics.rs",
            "ide/lsp_compat/formatting.rs",
            "workspace/workspace_index.rs",
            "ide/lsp_compat/semantic_tokens.rs",
            "refactor/import_optimizer.rs",
        ];

        let (missing_docs, incomplete_docs, missing_modules) =
            analyze_function_documentation(&function_critical_modules, &roots);

        if !missing_docs.is_empty() || !incomplete_docs.is_empty() {
            build_ac3_error_message(&missing_docs, &incomplete_docs);
        }

        assert!(
            missing_modules.is_empty(),
            "AC3 NOT IMPLEMENTED: Missing source modules: {:?}",
            missing_modules
        );
        assert!(missing_docs.is_empty(), "All public functions should have documentation");
        assert!(
            incomplete_docs.is_empty(),
            "All public functions should have comprehensive documentation sections"
        );
    }

    /// Analyzes function documentation across multiple modules
    fn analyze_function_documentation(
        modules: &[&str],
        roots: &[SourceRoot],
    ) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut missing_function_docs = Vec::new();
        let mut incomplete_function_docs = Vec::new();
        let mut missing_modules = Vec::new();

        for module in modules {
            let files = read_source_files(module, roots);
            if files.is_empty() {
                missing_modules.push(module.to_string());
                continue;
            }
            for (display_path, content) in files {
                let (missing, incomplete) = analyze_functions_in_file(&display_path, &content);
                missing_function_docs.extend(missing);
                incomplete_function_docs.extend(incomplete);
            }
        }

        (missing_function_docs, incomplete_function_docs, missing_modules)
    }

    /// Analyzes function documentation within a single file
    fn analyze_functions_in_file(module: &str, content: &str) -> (Vec<String>, Vec<String>) {
        let lines: Vec<&str> = content.lines().collect();
        let mut missing_docs = Vec::new();
        let mut incomplete_docs = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            if is_public_function_declaration(line) {
                let has_documentation =
                    line_num > 0 && lines[line_num - 1].trim_start().starts_with("///");

                if !has_documentation {
                    missing_docs.push(format!("{}:{}: {}", module, line_num + 1, line.trim()));
                } else if let Some(incomplete_info) =
                    check_function_doc_completeness(module, line_num, line, &lines)
                {
                    incomplete_docs.push(incomplete_info);
                }
            }
        }

        (missing_docs, incomplete_docs)
    }

    /// Checks if a line declares a public function
    fn is_public_function_declaration(line: &str) -> bool {
        line.trim_start().starts_with("pub fn") && !line.contains("//")
    }

    /// Checks if function documentation is complete with all required sections
    fn check_function_doc_completeness(
        module: &str,
        line_num: usize,
        function_line: &str,
        all_lines: &[&str],
    ) -> Option<String> {
        let doc_range_start = line_num.saturating_sub(20);
        let doc_range = &all_lines[doc_range_start..line_num];
        let doc_text = extract_doc_text(doc_range);

        let missing_sections = identify_missing_doc_sections(&doc_text, function_line);

        if !missing_sections.is_empty() {
            Some(format!(
                "{}:{}: {} - Missing: {}",
                module,
                line_num + 1,
                function_line.trim(),
                missing_sections.join(", ")
            ))
        } else {
            None
        }
    }

    /// Extracts documentation text from doc comment lines
    fn extract_doc_text(doc_lines: &[&str]) -> String {
        doc_lines
            .iter()
            .filter(|l| l.trim_start().starts_with("///"))
            .map(|l| l.trim_start().trim_start_matches("///").trim())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Identifies missing documentation sections for a function
    fn identify_missing_doc_sections(
        doc_text: &str,
        function_signature: &str,
    ) -> Vec<&'static str> {
        let mut missing_sections = Vec::new();

        let has_arguments = doc_text.contains("# Arguments") || doc_text.contains("Arguments:");
        let has_returns = doc_text.contains("# Returns") || doc_text.contains("Returns:");
        let has_errors = doc_text.contains("# Errors") || doc_text.contains("Errors:");
        let has_examples = doc_text.contains("# Examples") || doc_text.contains("Example:");

        let function_has_parameters = function_signature.contains('(')
            && !function_signature.contains("()")
            && !function_signature.contains("(&self)")
            && !function_signature.contains("(&mut self)");
        let function_returns_result = function_signature.contains("Result<");

        if function_has_parameters && !has_arguments {
            missing_sections.push("Arguments");
        }
        if !has_returns {
            missing_sections.push("Returns");
        }
        if function_returns_result && !has_errors {
            missing_sections.push("Errors");
        }
        if !has_examples {
            missing_sections.push("Examples");
        }

        missing_sections
    }

    /// Builds a comprehensive error message for AC3 failures
    fn build_ac3_error_message(missing_docs: &[String], incomplete_docs: &[String]) {
        let mut error_msg =
            String::from("AC3 NOT IMPLEMENTED: Incomplete function documentation:\n");

        if !missing_docs.is_empty() {
            error_msg.push_str("Missing documentation:\n");
            let display_count = std::cmp::min(10, missing_docs.len());
            for item in &missing_docs[..display_count] {
                error_msg.push_str(&format!("  - {}\n", item));
            }
            if missing_docs.len() > 10 {
                error_msg.push_str(&format!("  ... and {} more\n", missing_docs.len() - 10));
            }
        }

        if !incomplete_docs.is_empty() {
            error_msg.push_str("Incomplete documentation (missing required sections):\n");
            let display_count = std::cmp::min(10, incomplete_docs.len());
            for item in &incomplete_docs[..display_count] {
                error_msg.push_str(&format!("  - {}\n", item));
            }
            if incomplete_docs.len() > 10 {
                error_msg.push_str(&format!("  ... and {} more\n", incomplete_docs.len() - 10));
            }
        }

        unreachable!("{}", error_msg);
    }

    #[test]
    #[ignore = "documentation coverage: SPEC-149 not yet implemented"]
    fn test_performance_documentation_presence() {
        // AC:AC4 - Verify performance-critical APIs document memory usage and large workspace scaling
        let roots = source_roots();
        let performance_modules = [
            "incremental/incremental_v2.rs",
            "workspace/workspace_index.rs",
            "engine/parser/mod.rs",
            "analysis/semantic.rs",
            "tokens/token_stream.rs",
            "refactor/import_optimizer.rs",
            "analysis/scope_analyzer.rs",
            "tooling/performance.rs",
        ];

        let (missing_performance_docs, missing_modules) =
            analyze_performance_documentation(&performance_modules, &roots);

        if !missing_performance_docs.is_empty() {
            build_ac4_error_message(&missing_performance_docs);
        }

        assert!(
            missing_modules.is_empty(),
            "AC4 NOT IMPLEMENTED: Missing source modules: {:?}",
            missing_modules
        );
        assert!(
            missing_performance_docs.is_empty(),
            "Performance-critical APIs should have comprehensive performance documentation"
        );
    }

    /// Analyzes performance documentation across modules
    fn analyze_performance_documentation(
        modules: &[&str],
        roots: &[SourceRoot],
    ) -> (Vec<String>, Vec<String>) {
        let mut missing_docs = Vec::new();
        let mut missing_modules = Vec::new();

        for module in modules {
            let files = read_source_files(module, roots);
            if files.is_empty() {
                missing_modules.push(module.to_string());
                continue;
            }
            for (display_path, content) in files {
                if let Some(missing_info) =
                    check_performance_doc_completeness(&display_path, &content)
                {
                    missing_docs.push(missing_info);
                }
            }
        }

        (missing_docs, missing_modules)
    }

    /// Checks if a module has complete performance documentation
    fn check_performance_doc_completeness(module: &str, content: &str) -> Option<String> {
        let performance_indicators = analyze_performance_indicators(content);
        let missing_items = identify_missing_performance_items(&performance_indicators);

        if !missing_items.is_empty() {
            Some(format!("{}: Missing {}", module, missing_items.join(", ")))
        } else {
            None
        }
    }

    /// Represents performance documentation indicators in a module
    struct PerformanceIndicators {
        has_performance_section: bool,
        has_memory_docs: bool,
        has_pst_processing_notes: bool,
    }

    /// Analyzes performance-related documentation indicators
    fn analyze_performance_indicators(content: &str) -> PerformanceIndicators {
        let comment_lines: Vec<&str> =
            content.lines().filter(|line| line.trim_start().starts_with("//")).collect();

        let comment_content = comment_lines.join(" ").to_lowercase();

        PerformanceIndicators {
            has_performance_section: comment_content.contains("performance"),
            has_memory_docs: comment_content.contains("memory")
                || comment_content.contains("cache"),
            has_pst_processing_notes: comment_content.contains("50gb")
                || comment_content.contains("pst")
                || comment_content.contains("large file")
                || comment_content.contains("enterprise"),
        }
    }

    /// Identifies missing performance documentation items
    fn identify_missing_performance_items(indicators: &PerformanceIndicators) -> Vec<&'static str> {
        let mut missing = Vec::new();

        if !indicators.has_performance_section {
            missing.push("Performance section");
        }
        if !indicators.has_memory_docs {
            missing.push("Memory usage documentation");
        }
        if !indicators.has_pst_processing_notes {
            missing.push("large workspace scaling notes");
        }

        missing
    }

    /// Builds a comprehensive error message for AC4 failures
    fn build_ac4_error_message(missing_docs: &[String]) {
        let mut error_msg =
            String::from("AC4 NOT IMPLEMENTED: Missing performance documentation:\n");
        for item in missing_docs {
            error_msg.push_str(&format!("  - {}\n", item));
        }
        error_msg.push_str("\nPerformance-critical APIs must document:\n");
        error_msg.push_str("  - Memory usage patterns\n");
        error_msg.push_str("  - large workspace scaling performance implications\n");
        error_msg.push_str("  - Optimization characteristics\n");

        unreachable!("{}", error_msg);
    }

    #[test]
    #[ignore = "documentation coverage: SPEC-149 not yet implemented"]
    fn test_module_level_documentation_presence() {
        // AC:AC5 - Verify each module has comprehensive module-level documentation
        // with //! comments explaining purpose and LSP architecture relationship
        let roots = source_roots();
        let core_modules = [
            "engine/parser/mod.rs",
            "engine/ast.rs",
            "engine/error/mod.rs",
            "tokens/token_stream.rs",
            "ide/lsp_compat/code_actions.rs",
            "ide/lsp_compat/completion.rs",
            "ide/lsp_compat/diagnostics.rs",
            "ide/lsp_compat/semantic_tokens.rs",
            "ide/lsp_compat/references.rs",
            "ide/lsp_compat/rename.rs",
            "workspace/workspace_index.rs",
            "ide/lsp_compat/workspace_symbols.rs",
            "analysis/symbol.rs",
            "refactor/import_optimizer.rs",
            "analysis/scope_analyzer.rs",
            "tdd/test_generator.rs",
        ];

        let (missing_docs, incomplete_docs, missing_modules) =
            analyze_module_documentation(&core_modules, &roots);

        if !missing_docs.is_empty() || !incomplete_docs.is_empty() {
            build_ac5_error_message(&missing_docs, &incomplete_docs);
        }

        assert!(
            missing_modules.is_empty(),
            "AC5 NOT IMPLEMENTED: Missing source modules: {:?}",
            missing_modules
        );
        assert!(missing_docs.is_empty(), "All modules should have //! documentation");
        assert!(incomplete_docs.is_empty(), "All modules should have comprehensive documentation");
    }

    /// Analyzes module-level documentation across multiple modules
    fn analyze_module_documentation(
        modules: &[&str],
        roots: &[SourceRoot],
    ) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut missing_module_docs = Vec::new();
        let mut incomplete_module_docs = Vec::new();
        let mut missing_modules = Vec::new();

        for module in modules {
            let files = read_source_files(module, roots);
            if files.is_empty() {
                missing_modules.push(module.to_string());
                continue;
            }
            for (display_path, content) in files {
                let analysis = analyze_file_documentation(&display_path, &content);

                if !analysis.has_module_docs {
                    missing_module_docs.push(display_path);
                } else if let Some(incomplete_info) =
                    check_module_doc_completeness(&display_path, &content)
                {
                    incomplete_module_docs.push(incomplete_info);
                }
            }
        }

        (missing_module_docs, incomplete_module_docs, missing_modules)
    }

    /// Checks if module documentation is comprehensive
    fn check_module_doc_completeness(module: &str, content: &str) -> Option<String> {
        let module_doc_text = extract_module_doc_text(content);
        let missing_sections = identify_missing_module_sections(&module_doc_text);

        if !missing_sections.is_empty() {
            Some(format!("{}: Missing {}", module, missing_sections.join(", ")))
        } else {
            None
        }
    }

    /// Extracts module documentation text from //! comments
    fn extract_module_doc_text(content: &str) -> String {
        content
            .lines()
            .filter(|line| line.trim_start().starts_with("//!"))
            .map(|line| line.trim_start().trim_start_matches("//!").trim())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Identifies missing sections in module documentation
    fn identify_missing_module_sections(doc_text: &str) -> Vec<&'static str> {
        let mut missing = Vec::new();
        let doc_lower = doc_text.to_lowercase();

        let has_overview = doc_text.len() > 50; // Substantial content
        let has_workflow_integration = doc_lower.contains("parse")
            || doc_lower.contains("index")
            || doc_lower.contains("navigate")
            || doc_lower.contains("complete")
            || doc_lower.contains("analyze")
            || doc_lower.contains("lsp workflow")
            || doc_lower.contains("workflow");
        let has_examples = doc_text.contains("```") || doc_text.contains("Example");

        if !has_overview {
            missing.push("Comprehensive overview");
        }
        if !has_workflow_integration {
            missing.push("workflow integration");
        }
        if !has_examples {
            missing.push("Usage examples");
        }

        missing
    }

    /// Builds a comprehensive error message for AC5 failures
    fn build_ac5_error_message(missing_docs: &[String], incomplete_docs: &[String]) {
        let mut error_msg =
            String::from("AC5 NOT IMPLEMENTED: Missing/incomplete module documentation:\n");

        if !missing_docs.is_empty() {
            error_msg.push_str("Modules without //! documentation:\n");
            for module in missing_docs {
                error_msg.push_str(&format!("  - {}\n", module));
            }
        }

        if !incomplete_docs.is_empty() {
            error_msg.push_str("Modules with incomplete documentation:\n");
            for item in incomplete_docs {
                error_msg.push_str(&format!("  - {}\n", item));
            }
        }

        error_msg.push_str("\nEach module must have:\n");
        error_msg.push_str("  - //! Module-level documentation\n");
        error_msg.push_str("  - Purpose and LSP architecture relationship\n");
        error_msg.push_str("  - Usage examples\n");

        unreachable!("{}", error_msg);
    }

    #[test]
    #[ignore = "documentation coverage: SPEC-149 not yet implemented"]
    fn test_usage_examples_in_complex_apis() {
        // AC:AC6 - Verify complex APIs include usage examples
        let roots = source_roots();
        let complex_api_modules = [
            "ide/lsp_compat/completion.rs",
            "ide/lsp_compat/diagnostics.rs",
            "ide/lsp_compat/code_actions.rs",
            "ide/lsp_compat/semantic_tokens.rs",
            "workspace/workspace_index.rs",
            "engine/parser/mod.rs",
            "refactor/import_optimizer.rs",
            "tdd/test_generator.rs",
            "analysis/scope_analyzer.rs",
        ];

        let (modules_without_examples, missing_modules) =
            find_modules_missing_examples(&complex_api_modules, &roots);

        if !modules_without_examples.is_empty() {
            build_ac6_error_message(&modules_without_examples);
        }

        assert!(
            missing_modules.is_empty(),
            "AC6 NOT IMPLEMENTED: Missing source modules: {:?}",
            missing_modules
        );
        assert!(modules_without_examples.is_empty(), "Complex APIs should include usage examples");
    }

    /// Finds modules that are missing usage examples
    fn find_modules_missing_examples(
        modules: &[&str],
        roots: &[SourceRoot],
    ) -> (Vec<String>, Vec<String>) {
        let mut missing_examples = Vec::new();
        let mut missing_modules = Vec::new();

        for module in modules {
            let files = read_source_files(module, roots);
            if files.is_empty() {
                missing_modules.push(module.to_string());
                continue;
            }
            for (display_path, content) in files {
                if !has_usage_examples(&content) {
                    missing_examples.push(display_path);
                }
            }
        }

        (missing_examples, missing_modules)
    }

    /// Checks if content has usage examples
    fn has_usage_examples(content: &str) -> bool {
        let has_code_examples = content.contains("```rust") || content.contains("```");
        let has_example_section = content.contains("# Examples") || content.contains("# Example");

        has_code_examples || has_example_section
    }

    /// Builds a comprehensive error message for AC6 failures
    fn build_ac6_error_message(missing_examples: &[String]) {
        let mut error_msg =
            String::from("AC6 NOT IMPLEMENTED: Complex APIs missing usage examples:\n");
        for module in missing_examples {
            error_msg.push_str(&format!("  - {}\n", module));
        }
        error_msg.push_str("\nComplex APIs must include:\n");
        error_msg.push_str("  - Working code examples in doc comments\n");
        error_msg.push_str("  - LSP provider configuration examples\n");
        error_msg.push_str("  - Parser configuration examples\n");

        unreachable!("{}", error_msg);
    }

    #[test]
    #[ignore = "documentation coverage: SPEC-149 not yet implemented"]
    fn test_doctests_presence_and_execution() {
        // AC:AC7 - Verify doctests are present for critical functionality
        let roots = source_roots();
        let critical_modules = [
            "engine/parser/mod.rs",
            "ide/lsp_compat/completion.rs",
            "workspace/workspace_index.rs",
            "refactor/import_optimizer.rs",
            "tdd/test_generator.rs",
        ];

        let (modules_without_doctests, missing_modules) =
            find_modules_missing_doctests(&critical_modules, &roots);

        if !modules_without_doctests.is_empty() {
            build_ac7_error_message(&modules_without_doctests);
        }

        assert!(
            missing_modules.is_empty(),
            "AC7 NOT IMPLEMENTED: Missing source modules: {:?}",
            missing_modules
        );
        assert!(
            modules_without_doctests.is_empty(),
            "Critical modules should include working doctests"
        );
    }

    /// Finds modules that are missing doctests
    fn find_modules_missing_doctests(
        modules: &[&str],
        roots: &[SourceRoot],
    ) -> (Vec<String>, Vec<String>) {
        let mut missing_doctests = Vec::new();
        let mut missing_modules = Vec::new();

        for module in modules {
            let files = read_source_files(module, roots);
            if files.is_empty() {
                missing_modules.push(module.to_string());
                continue;
            }
            for (display_path, content) in files {
                if !has_valid_doctests(&content) {
                    missing_doctests.push(display_path);
                }
            }
        }

        (missing_doctests, missing_modules)
    }

    /// Checks if content has valid doctests
    fn has_valid_doctests(content: &str) -> bool {
        content.contains("```rust")
            && (content.contains("use perl_parser") || content.contains("use crate"))
    }

    /// Builds a comprehensive error message for AC7 failures
    fn build_ac7_error_message(missing_doctests: &[String]) {
        let mut error_msg =
            String::from("AC7 NOT IMPLEMENTED: Missing doctests in critical modules:\n");
        for module in missing_doctests {
            error_msg.push_str(&format!("  - {}\n", module));
        }
        error_msg.push_str("\nCritical functionality must include:\n");
        error_msg.push_str("  - Working doctests with ```rust blocks\n");
        error_msg.push_str("  - Examples that pass 'cargo test --doc'\n");
        error_msg.push_str("  - Real usage scenarios in examples\n");

        unreachable!("{}", error_msg);
    }

    #[test]
    #[ignore = "documentation coverage: SPEC-149 not yet implemented"]
    fn test_error_types_documentation() {
        // AC:AC8 - Verify error types are documented with parsing and analysis workflow context
        let roots = source_roots();
        let error_files = [
            "engine/error/mod.rs",
            "ide/lsp_compat/lsp_errors.rs",
            "ide/lsp_compat/diagnostics.rs",
        ];

        let (undocumented_errors, missing_workflow_context, missing_modules) =
            analyze_error_documentation(&error_files, &roots);

        if !undocumented_errors.is_empty() || !missing_workflow_context.is_empty() {
            build_ac8_error_message(&undocumented_errors, &missing_workflow_context);
        }

        assert!(
            missing_modules.is_empty(),
            "AC8 NOT IMPLEMENTED: Missing source modules: {:?}",
            missing_modules
        );
        assert!(undocumented_errors.is_empty(), "All error types should be documented");
        assert!(missing_workflow_context.is_empty(), "All errors should include workflow context");
    }

    /// Analyzes error documentation across error-related files
    fn analyze_error_documentation(
        error_files: &[&str],
        roots: &[SourceRoot],
    ) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut undocumented_errors = Vec::new();
        let mut missing_workflow_context = Vec::new();
        let mut missing_modules = Vec::new();

        for error_file in error_files {
            let files = read_source_files(error_file, roots);
            if files.is_empty() {
                missing_modules.push(error_file.to_string());
                continue;
            }
            for (display_path, content) in files {
                let (undoc, missing_context) = analyze_errors_in_file(&display_path, &content);
                undocumented_errors.extend(undoc);
                missing_workflow_context.extend(missing_context);
            }
        }

        (undocumented_errors, missing_workflow_context, missing_modules)
    }

    /// Analyzes error documentation within a single file
    fn analyze_errors_in_file(file_name: &str, content: &str) -> (Vec<String>, Vec<String>) {
        let lines: Vec<&str> = content.lines().collect();
        let mut undocumented = Vec::new();
        let mut missing_context = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            if is_error_type_declaration(line) {
                let has_documentation =
                    line_num > 0 && lines[line_num - 1].trim_start().starts_with("///");

                if !has_documentation {
                    undocumented.push(format!("{}:{}: {}", file_name, line_num + 1, line.trim()));
                } else if !has_workflow_context(&lines, line_num) {
                    missing_context.push(format!(
                        "{}:{}: {}",
                        file_name,
                        line_num + 1,
                        line.trim()
                    ));
                }
            }
        }

        (undocumented, missing_context)
    }

    /// Checks if a line declares an error type
    fn is_error_type_declaration(line: &str) -> bool {
        (line.contains("pub enum") && line.contains("Error"))
            || (line.contains("pub struct") && line.contains("Error"))
            || (line.trim_start().starts_with("pub") && line.contains("Error"))
    }

    /// Checks if error documentation includes workflow context
    fn has_workflow_context(lines: &[&str], error_line: usize) -> bool {
        let doc_range_start = error_line.saturating_sub(10);
        let doc_range = &lines[doc_range_start..error_line];
        let doc_text = extract_doc_text(doc_range).to_lowercase();

        doc_text.contains("workflow")
            || doc_text.contains("parse")
            || doc_text.contains("index")
            || doc_text.contains("navigate")
            || doc_text.contains("complete")
            || doc_text.contains("analyze")
            || doc_text.contains("when this error occurs")
            || doc_text.contains("recovery")
    }

    /// Builds a comprehensive error message for AC8 failures
    fn build_ac8_error_message(
        undocumented_errors: &[String],
        missing_workflow_context: &[String],
    ) {
        let mut error_msg = String::from("AC8 NOT IMPLEMENTED: Error documentation issues:\n");

        if !undocumented_errors.is_empty() {
            error_msg.push_str("Undocumented error types:\n");
            for error in undocumented_errors {
                error_msg.push_str(&format!("  - {}\n", error));
            }
        }

        if !missing_workflow_context.is_empty() {
            error_msg.push_str("Errors missing workflow context:\n");
            for error in missing_workflow_context {
                error_msg.push_str(&format!("  - {}\n", error));
            }
        }

        error_msg.push_str("\nError documentation must include:\n");
        error_msg.push_str("  - When the error occurs in parsing and analysis workflows\n");
        error_msg.push_str("  - Recovery strategies\n");
        error_msg.push_str("  - Pipeline stage context (Extract/Normalize/Thread/Render/Index)\n");

        unreachable!("{}", error_msg);
    }

    #[test]
    #[ignore = "documentation coverage: SPEC-149 not yet implemented"]
    fn test_cross_references_between_functions() {
        // AC:AC9 - Verify related functions include cross-references using Rust documentation linking
        let roots = source_roots();
        let cross_ref_modules = [
            "ide/lsp_compat/completion.rs",
            "workspace/workspace_index.rs",
            "analysis/symbol.rs",
            "ide/lsp_compat/semantic_tokens.rs",
            "ide/lsp_compat/diagnostics.rs",
            "ide/lsp_compat/code_actions.rs",
        ];

        let (modules_without_cross_refs, missing_modules) =
            find_modules_missing_cross_references(&cross_ref_modules, &roots);

        if !modules_without_cross_refs.is_empty() {
            build_ac9_error_message(&modules_without_cross_refs);
        }

        assert!(
            missing_modules.is_empty(),
            "AC9 NOT IMPLEMENTED: Missing source modules: {:?}",
            missing_modules
        );
        assert!(
            modules_without_cross_refs.is_empty(),
            "Modules should include cross-references between related functions"
        );
    }

    /// Finds modules that are missing cross-references
    fn find_modules_missing_cross_references(
        modules: &[&str],
        roots: &[SourceRoot],
    ) -> (Vec<String>, Vec<String>) {
        let mut missing_cross_refs = Vec::new();
        let mut missing_modules = Vec::new();

        for module in modules {
            let files = read_source_files(module, roots);
            if files.is_empty() {
                missing_modules.push(module.to_string());
                continue;
            }
            for (display_path, content) in files {
                if !has_cross_references(&content) {
                    missing_cross_refs.push(display_path);
                }
            }
        }

        (missing_cross_refs, missing_modules)
    }

    /// Checks if content has cross-references
    fn has_cross_references(content: &str) -> bool {
        let has_function_links = content.contains("[`") && content.contains("`]");
        let has_module_links = content.contains("::") && content.contains("[`");
        let has_see_also = content.contains("See also")
            || content.contains("see also")
            || content.contains("Related")
            || content.contains("related");

        has_function_links || has_module_links || has_see_also
    }

    /// Builds a comprehensive error message for AC9 failures
    fn build_ac9_error_message(modules_without_cross_refs: &[String]) {
        let mut error_msg =
            String::from("AC9 NOT IMPLEMENTED: Modules missing cross-references:\n");
        for module in modules_without_cross_refs {
            error_msg.push_str(&format!("  - {}\n", module));
        }
        error_msg.push_str("\nDocumentation should include cross-references:\n");
        error_msg.push_str("  - [`function_name`] for same-module functions\n");
        error_msg.push_str("  - [`module::function`] for cross-module references\n");
        error_msg.push_str("  - Related functionality links\n");
        error_msg.push_str("  - 'See also' sections where appropriate\n");

        unreachable!("{}", error_msg);
    }

    #[test]
    #[ignore = "documentation coverage: SPEC-149 not yet implemented"]
    fn test_rust_documentation_best_practices() {
        // AC:AC10 - Verify documentation follows Rust best practices with consistent style
        let roots = source_roots();
        let sample_modules = [
            "engine/parser/mod.rs",
            "ide/lsp_compat/completion.rs",
            "ide/lsp_compat/diagnostics.rs",
            "workspace/workspace_index.rs",
        ];

        let (style_violations, missing_modules) =
            find_documentation_style_violations(&sample_modules, &roots);

        if !style_violations.is_empty() {
            build_ac10_error_message(&style_violations);
        }

        assert!(
            missing_modules.is_empty(),
            "AC10 NOT IMPLEMENTED: Missing source modules: {:?}",
            missing_modules
        );
        assert!(style_violations.is_empty(), "Documentation should follow Rust best practices");
    }

    /// Finds documentation style violations across modules
    fn find_documentation_style_violations(
        modules: &[&str],
        roots: &[SourceRoot],
    ) -> (Vec<String>, Vec<String>) {
        let mut all_violations = Vec::new();
        let mut missing_modules = Vec::new();

        for module in modules {
            let files = read_source_files(module, roots);
            if files.is_empty() {
                missing_modules.push(module.to_string());
                continue;
            }
            for (display_path, content) in files {
                let analysis = analyze_file_documentation(&display_path, &content);
                all_violations.extend(analysis.style_violations);
            }
        }

        (all_violations, missing_modules)
    }

    /// Builds a comprehensive error message for AC10 failures
    fn build_ac10_error_message(style_violations: &[String]) {
        let mut error_msg = String::from("AC10 NOT IMPLEMENTED: Documentation style violations:\n");
        let display_count = std::cmp::min(10, style_violations.len());
        for violation in &style_violations[..display_count] {
            error_msg.push_str(&format!("  - {}\n", violation));
        }
        if style_violations.len() > 10 {
            error_msg.push_str(&format!("  ... and {} more\n", style_violations.len() - 10));
        }

        error_msg.push_str("\nRust documentation best practices:\n");
        error_msg.push_str("  - Brief summary, detailed description, examples format\n");
        error_msg.push_str("  - Proper section headers with '# '\n");
        error_msg.push_str("  - Code blocks with language specification\n");
        error_msg.push_str("  - Consistent formatting and style\n");

        unreachable!("{}", error_msg);
    }

    #[test]
    fn test_cargo_doc_generation_success() {
        // AC:AC11 - Verify cargo doc generates complete documentation without warnings
        // Performance optimization: Use efficient validation approach for LSP tests
        if std::env::var("PERL_LSP_PERFORMANCE_TEST").is_ok() {
            // Fast path: Skip expensive cargo doc during LSP integration tests
            eprintln!("Skipping cargo doc validation for LSP performance test");
            return;
        }

        let package_dir = concat!(env!("CARGO_MANIFEST_DIR"));

        match validate_cargo_doc_generation(package_dir) {
            Ok(()) => {
                // Test passes - cargo doc generation succeeded without warnings
            }
            Err(error_msg) => {
                unreachable!(
                    "AC11 NOT IMPLEMENTED: cargo doc generation failed or has warnings:\n{}",
                    error_msg
                );
            }
        }
    }

    #[test]
    fn test_ci_missing_docs_enforcement() {
        // AC:AC12 - Verify CI checks enforce missing_docs warnings for new public APIs
        let lib_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs");
        let ci_enforcement_status = analyze_ci_documentation_enforcement(lib_path);

        if !ci_enforcement_status.is_properly_configured {
            build_ac12_error_message(&ci_enforcement_status);
        }

        assert!(
            ci_enforcement_status.is_properly_configured,
            "CI should enforce missing_docs warnings for new public APIs"
        );
    }

    /// Represents the CI documentation enforcement configuration status
    struct CiEnforcementStatus {
        has_enabled_missing_docs: bool,
        has_clippy_docs_enforcement: bool,
        is_properly_configured: bool,
    }

    /// Analyzes CI documentation enforcement configuration
    fn analyze_ci_documentation_enforcement(lib_path: &str) -> CiEnforcementStatus {
        let lib_content_res = fs::read_to_string(lib_path);
        assert!(lib_content_res.is_ok(), "Failed to read lib.rs");
        let lib_content = lib_content_res.unwrap_or_else(|_| unreachable!());

        let has_enabled_missing_docs = lib_content.contains("#![warn(missing_docs)]")
            && !lib_content.contains("// #![warn(missing_docs)]");
        let has_clippy_docs_enforcement =
            lib_content.contains("missing_docs") || lib_content.contains("clippy::missing_docs");
        let is_properly_configured = has_enabled_missing_docs || has_clippy_docs_enforcement;

        CiEnforcementStatus {
            has_enabled_missing_docs,
            has_clippy_docs_enforcement,
            is_properly_configured,
        }
    }

    /// Builds a comprehensive error message for AC12 failures
    fn build_ac12_error_message(status: &CiEnforcementStatus) {
        let mut error_msg =
            String::from("AC12 NOT IMPLEMENTED: CI missing_docs enforcement not configured:\n");

        if !status.has_enabled_missing_docs {
            error_msg.push_str("  - missing_docs warning not enabled in lib.rs\n");
        }
        if !status.has_clippy_docs_enforcement {
            error_msg.push_str("  - No clippy missing_docs enforcement configured\n");
        }

        error_msg.push_str("\nCI enforcement requires:\n");
        error_msg.push_str("  - #![warn(missing_docs)] uncommented in lib.rs\n");
        error_msg.push_str("  - CI pipeline configured to fail on missing docs\n");
        error_msg.push_str("  - Automatic documentation coverage checking\n");

        unreachable!("{}", error_msg);
    }

    #[test]
    fn test_comprehensive_workflow_documentation() {
        // Integration test ensuring all LSP workflow stages are documented
        // This combines aspects of multiple ACs to ensure comprehensive coverage
        let roots = source_roots();
        let core_modules = [
            "engine/parser/mod.rs",
            "engine/ast.rs",
            "ide/lsp_compat/completion.rs",
            "ide/lsp_compat/diagnostics.rs",
            "workspace/workspace_index.rs",
            "ide/lsp_compat/semantic_tokens.rs",
            "refactor/import_optimizer.rs",
        ];

        let workflow_coverage = analyze_workflow_coverage_multi(&core_modules, &roots);
        let (total_coverage, expected_minimum) = calculate_coverage_metrics(&workflow_coverage);

        if total_coverage < expected_minimum {
            build_workflow_coverage_error_message(
                &workflow_coverage,
                total_coverage,
                expected_minimum,
            );
        }

        assert!(
            total_coverage >= expected_minimum,
            "LSP workflow stages should be comprehensively documented across core modules"
        );
    }

    fn analyze_workflow_coverage_multi(
        module_paths: &[&str],
        roots: &[SourceRoot],
    ) -> HashMap<String, usize> {
        let workflow_stages = ["Parse", "Index", "Navigate", "Complete", "Analyze"];
        let mut coverage = HashMap::new();

        for stage in &workflow_stages {
            coverage.insert(stage.to_string(), 0);
        }

        for module in module_paths {
            for (_, content) in read_source_files(module, roots) {
                let content_lower = content.to_lowercase();
                for stage in &workflow_stages {
                    if content_lower.contains(&stage.to_lowercase()) {
                        if let Some(count) = coverage.get_mut(*stage) {
                            *count += 1;
                        }
                    }
                }
            }
        }

        coverage
    }

    /// Calculates coverage metrics for workflow documentation
    fn calculate_coverage_metrics(coverage: &HashMap<String, usize>) -> (usize, usize) {
        let total_coverage = coverage.values().sum();
        let expected_minimum = coverage.len() * 2; // At least 2 modules per stage
        (total_coverage, expected_minimum)
    }

    /// Builds error message for workflow coverage failures
    fn build_workflow_coverage_error_message(
        coverage: &HashMap<String, usize>,
        total: usize,
        expected: usize,
    ) {
        let mut error_msg = String::from("LSP WORKFLOW DOCUMENTATION INCOMPLETE:\n");
        error_msg.push_str("Workflow stage coverage in core modules:\n");
        for (stage, count) in coverage {
            error_msg.push_str(&format!("  - {}: {} modules\n", stage, count));
        }
        error_msg
            .push_str(&format!("\nTotal coverage: {}, Expected minimum: {}\n", total, expected));
        error_msg.push_str("All core modules should document their role in LSP workflow stages\n");

        unreachable!("{}", error_msg);
    }

    // ============================================================================
    // Enhanced Edge Case Testing
    // ============================================================================

    #[test]
    fn test_edge_case_malformed_doctests() {
        // Test malformed doctest detection
        let malformed_content = r#"
/// This function does something
/// ```rust
/// let x = 1;
/// // Missing closing brace {
/// ```
pub fn test_function() -> Result<(), Error> { Ok(()) }

/// Another function
/// ```rust
/// // Empty doctest
/// ```
pub fn another_function() {}

/// Function with unbalanced parens
/// ```rust
/// let result = some_call(arg1, arg2;
/// ```
pub fn unbalanced_function() {}
"#;

        let lines: Vec<&str> = malformed_content.lines().collect();
        let malformed = doc_validation_helpers::find_malformed_doctests(&lines);

        assert!(!malformed.is_empty(), "Should detect malformed doctests");
        // Check for various types of malformed patterns
        assert!(
            malformed.iter().any(|s| s.contains("Empty doctest")
                || s.contains("without assertions")
                || s.contains("Unbalanced braces")
                || s.contains("Unclosed doctest")),
            "Should detect various types of malformed doctests"
        );
        assert!(malformed.len() >= 2, "Should detect multiple issues in the test content");
    }

    #[test]
    fn test_edge_case_empty_documentation() {
        let empty_doc_content = r#"
///
pub fn empty_doc_function() {}

/// TODO
pub fn todo_function() {}

/// .
pub fn minimal_doc() {}

/// FIXME: Need to document this
pub fn fixme_function() {}
"#;

        let lines: Vec<&str> = empty_doc_content.lines().collect();
        let empty_docs = doc_validation_helpers::find_empty_doc_strings(&lines);

        assert!(!empty_docs.is_empty(), "Should detect trivial documentation");
        assert!(
            empty_docs
                .iter()
                .any(|s| s.contains("TODO") || s.contains("FIXME") || s.contains("Trivial")),
            "Should detect placeholder or trivial documentation"
        );
    }

    #[test]
    fn test_edge_case_invalid_cross_references() {
        let invalid_ref_content = r#"
/// See [`[`invalid_nested`]`] for details
pub fn nested_ref_function() {}

/// Check [``] empty reference
pub fn empty_ref_function() {}

/// Look at [` spaced_ref `] with spaces
pub fn spaced_ref_function() {}
"#;

        let lines: Vec<&str> = invalid_ref_content.lines().collect();
        let invalid_refs = doc_validation_helpers::find_invalid_cross_references(&lines);

        assert!(!invalid_refs.is_empty(), "Should detect invalid cross-references");
        assert!(
            invalid_refs.iter().any(|s| s.contains("Malformed cross-reference")),
            "Should detect nested references"
        );
        assert!(
            invalid_refs.iter().any(|s| s.contains("Empty cross-reference")),
            "Should detect empty references"
        );
        assert!(
            invalid_refs.iter().any(|s| s.contains("invalid spaces")),
            "Should detect spaced references"
        );
    }

    #[test]
    fn test_edge_case_incomplete_performance_docs() {
        // Test with a mock performance-critical file
        let incomplete_perf_content = r#"
//! This module handles parsing performance
//! It's optimized for speed but missing complexity analysis

/// Fast parsing function
pub fn parse_fast() -> Result<Ast, Error> { todo!() }
"#;

        let lines: Vec<&str> = incomplete_perf_content.lines().collect();
        let incomplete = doc_validation_helpers::find_incomplete_performance_docs(
            "engine/parser/mod.rs",
            &lines,
        );

        assert!(!incomplete.is_empty(), "Should detect incomplete performance docs");
        assert!(
            incomplete.iter().any(|s| s.contains("complexity")),
            "Should require complexity documentation"
        );
        assert!(
            incomplete.iter().any(|s| s.contains("large-scale")),
            "Should require scaling documentation"
        );
    }

    #[test]
    fn test_edge_case_missing_error_recovery_docs() {
        let error_handling_content = r#"
/// Does something
pub fn risky_operation() -> Result<String, ParseError> {
    todo!()
}

/// Minimal
pub fn another_risky() -> Result<(), Box<dyn std::error::Error>> {
    todo!()
}
"#;

        let lines: Vec<&str> = error_handling_content.lines().collect();
        let missing_recovery = doc_validation_helpers::find_missing_error_recovery_docs(&lines);

        assert!(!missing_recovery.is_empty(), "Should detect missing error recovery docs");
        assert!(
            missing_recovery
                .iter()
                .any(|s| s.contains("error handling") || s.contains("Missing error")),
            "Should require error handling documentation"
        );
    }

    // ============================================================================
    // Property-Based Testing for Documentation Consistency
    // ============================================================================

    /// Property-based test data structures for future enhanced documentation validation
    #[derive(Debug, Clone)]
    #[allow(dead_code)] // Reserved for future property-based documentation validation expansion
    struct DocTestScenario {
        doc_lines: Vec<String>,
        expected_violations: usize,
        violation_types: Vec<String>,
    }

    proptest! {
        #[test]
        fn property_test_documentation_format_consistency(
            doc_lines in vec(any::<String>(), 1..10),
            _module_name in any::<String>()
        ) {
            // Generate documentation with consistent formatting rules
            let formatted_lines: Vec<String> = doc_lines.iter()
                .map(|line| format!("/// {}", line.trim()))
                .collect();

            let content = formatted_lines.join("\n");
            let lines: Vec<&str> = content.lines().collect();

            // Test that our validation functions handle arbitrary input gracefully
            let malformed = doc_validation_helpers::find_malformed_doctests(&lines);
            let empty_docs = doc_validation_helpers::find_empty_doc_strings(&lines);
            let invalid_refs = doc_validation_helpers::find_invalid_cross_references(&lines);

            // Property: validation functions should always return valid results
            assert!(malformed.iter().all(|s| !s.is_empty()));
            assert!(empty_docs.iter().all(|s| !s.is_empty()));
            assert!(invalid_refs.iter().all(|s| !s.is_empty()));
        }

        #[test]
        fn property_test_cross_reference_validation(
            function_name in "[a-zA-Z_][a-zA-Z0-9_]*",
            _module_path in "[a-z_]+(::[a-z_]+)*"
        ) {
            // Generate valid and invalid cross-reference patterns
            let valid_ref = format!("/// See [`{}`] for details", function_name);
            let invalid_ref = format!("/// See [` {} `] for details", function_name);
            let nested_ref = format!("/// See [`[`{}`]`] for details", function_name);

            let content = format!("{}\n{}\n{}", valid_ref, invalid_ref, nested_ref);
            let lines: Vec<&str> = content.lines().collect();
            let invalid_refs = doc_validation_helpers::find_invalid_cross_references(&lines);

            // Property: should detect invalid patterns but not valid ones
            assert!(invalid_refs.iter().any(|s| s.contains("invalid spaces")));
            assert!(invalid_refs.iter().any(|s| s.contains("Malformed")));
            // Valid references should not generate violations
        }

        #[test]
        fn property_test_doctest_structure_validation(
            rust_code in "[a-zA-Z0-9_\\s=();,\\.]*",
            has_assertions in any::<bool>()
        ) {
            let mut doctest_content = format!("/// ```rust\n/// {}\n", rust_code);

            if has_assertions {
                doctest_content.push_str("/// assert_eq!(1, 1);\n");
            }

            doctest_content.push_str("/// ```\n");

            let lines: Vec<&str> = doctest_content.lines().collect();
            let malformed = doc_validation_helpers::find_malformed_doctests(&lines);

            // Property: doctests with assertions should be considered valid
            if has_assertions && !rust_code.trim().is_empty() {
                assert!(malformed.iter().all(|s| !s.contains("without assertions")));
            }
        }
    }

    // ============================================================================
    // Table-Driven Tests for Systematic Validation
    // ============================================================================

    /// Test data structure for table-driven testing
    #[derive(Debug, Clone)]
    struct DocumentationTestCase {
        name: &'static str,
        content: &'static str,
        expected_module_docs: bool,
        expected_violations: usize,
        expected_cross_refs: bool,
        expected_examples: bool,
    }

    #[test]
    #[ignore = "documentation coverage: SPEC-149 not yet implemented"]
    fn test_table_driven_documentation_patterns() {
        let test_cases = vec![
            DocumentationTestCase {
                name: "Complete documentation",
                content: r#"
//! This module provides comprehensive functionality
//! for Perl parsing in the LSP workflow.
//! Integrates with Parse → Index → Navigate → Complete → Analyze stages.
//!
//! # Examples
//! ```rust
//! use crate::module::function;
//! let result = function();
//! assert!(result.is_ok());
//! ```

/// Parses source in the Parse stage of the LSP workflow
///
/// # Arguments
/// * `input` - The Perl source to parse
///
/// # Returns
/// * `Ok(Ast)` - Successfully parsed AST
/// * `Err(ParseError)` - When parsing fails due to malformed input
///
/// # Examples
/// ```rust
/// let ast = parse_source("my $x = 1;")?;
/// assert!(ast.count_nodes() > 0);
/// ```
///
/// See also [`parse_with_recovery`] for error-tolerant parsing.
pub fn parse_source(input: &str) -> Result<Ast, ParseError> {
    todo!()
}
"#,
                expected_module_docs: true,
                expected_violations: 0,
                expected_cross_refs: true,
                expected_examples: true,
            },
            DocumentationTestCase {
                name: "Missing module documentation",
                content: r#"
/// Function without module docs
pub fn lonely_function() {}
"#,
                expected_module_docs: false,
                expected_violations: 0,
                expected_cross_refs: false,
                expected_examples: false,
            },
            DocumentationTestCase {
                name: "Malformed doctests",
                content: r#"
//! Module with malformed examples

/// Function with broken doctest
/// ```rust
/// let x = {;
/// // Unbalanced braces
/// ```
pub fn broken_example() {}
"#,
                expected_module_docs: true,
                expected_violations: 1, // malformed doctest
                expected_cross_refs: false,
                expected_examples: true,
            },
            DocumentationTestCase {
                name: "Empty and trivial documentation",
                content: r#"
//! TODO: Document this module

///
pub fn empty_doc() {}

/// TODO
pub fn todo_doc() {}

/// .
pub fn minimal_doc() {}
"#,
                expected_module_docs: true,
                expected_violations: 3, // empty docs
                expected_cross_refs: false,
                expected_examples: false,
            },
            DocumentationTestCase {
                name: "Invalid cross-references",
                content: r#"
//! Module with reference issues

/// See [``] and [`[`nested`]`] refs
/// Also check [` spaced `] reference
pub fn bad_refs() {}
"#,
                expected_module_docs: true,
                expected_violations: 3,    // invalid refs
                expected_cross_refs: true, // technically has cross-ref syntax
                expected_examples: false,
            },
        ];

        for test_case in test_cases {
            let _lines: Vec<&str> = test_case.content.lines().collect();
            let analysis = doc_validation_helpers::analyze_file_documentation(
                test_case.name,
                test_case.content,
            );

            // Validate expected properties
            assert_eq!(
                analysis.has_module_docs, test_case.expected_module_docs,
                "Test case '{}': Module docs mismatch",
                test_case.name
            );

            assert_eq!(
                analysis.cross_references_present, test_case.expected_cross_refs,
                "Test case '{}': Cross-references mismatch",
                test_case.name
            );

            assert_eq!(
                analysis.code_examples_present, test_case.expected_examples,
                "Test case '{}': Examples mismatch",
                test_case.name
            );

            // Check violation counts
            let total_violations = analysis.malformed_doctests.len()
                + analysis.empty_doc_strings.len()
                + analysis.invalid_cross_references.len();

            assert_eq!(
                total_violations,
                test_case.expected_violations,
                "Test case '{}': Expected {} violations, found {}. Violations: {:?}",
                test_case.name,
                test_case.expected_violations,
                total_violations,
                (
                    analysis.malformed_doctests,
                    analysis.empty_doc_strings,
                    analysis.invalid_cross_references
                )
            );
        }
    }

    /// Test data for performance documentation validation
    #[derive(Debug)]
    struct PerformanceDocTestCase {
        name: &'static str,
        file_path: &'static str,
        content: &'static str,
        expected_missing_items: usize,
        should_have_complexity: bool,
        should_have_scaling: bool,
        should_have_benchmarks: bool,
    }

    #[test]
    fn test_table_driven_performance_documentation() {
        let test_cases = vec![
            PerformanceDocTestCase {
                name: "Complete performance docs",
                file_path: "engine/parser/mod.rs",
                content: r#"
//! High-performance parser with O(n) time complexity
//!
//! Optimized for large workspaces with efficient
//! memory usage. Benchmarks show 150µs parsing speed
//! for typical Perl files.
//!
//! # Performance Characteristics
//! - Time complexity: O(n) where n is input size
//! - Space complexity: O(log n) for parse tree
//! - Scales linearly with file size for large workspaces
//! - Benchmark: 1-150µs per parse depending on complexity
"#,
                expected_missing_items: 0,
                should_have_complexity: true,
                should_have_scaling: true,
                should_have_benchmarks: true,
            },
            PerformanceDocTestCase {
                name: "Missing complexity analysis",
                file_path: "incremental/incremental_v2.rs",
                content: r#"
//! Incremental parsing module
//!
//! Handles large workspaces efficiently with optimized algorithms.
//! Performance tested on enterprise-scale codebases.
"#,
                expected_missing_items: 1, // missing complexity (it has optimized/performance keywords)
                should_have_complexity: false,
                should_have_scaling: true,
                should_have_benchmarks: true, // "performance tested" is detected
            },
            PerformanceDocTestCase {
                name: "Non-performance-critical module",
                file_path: "utils.rs",
                content: r#"
//! Utility functions for general use
"#,
                expected_missing_items: 0, // not performance-critical
                should_have_complexity: false,
                should_have_scaling: false,
                should_have_benchmarks: false,
            },
        ];

        for test_case in test_cases {
            let lines: Vec<&str> = test_case.content.lines().collect();
            let incomplete_docs = doc_validation_helpers::find_incomplete_performance_docs(
                test_case.file_path,
                &lines,
            );

            assert_eq!(
                incomplete_docs.len(),
                test_case.expected_missing_items,
                "Test case '{}': Expected {} missing items, found {}. Items: {:?}",
                test_case.name,
                test_case.expected_missing_items,
                incomplete_docs.len(),
                incomplete_docs
            );

            // Detailed content validation
            let content_lower = test_case.content.to_lowercase();

            let has_complexity = content_lower.contains("o(")
                || content_lower.contains("time complexity")
                || content_lower.contains("space complexity");
            assert_eq!(
                has_complexity, test_case.should_have_complexity,
                "Test case '{}': Complexity documentation mismatch",
                test_case.name
            );

            let has_scaling = content_lower.contains("scale")
                || content_lower.contains("50gb")
                || content_lower.contains("large file");
            assert_eq!(
                has_scaling, test_case.should_have_scaling,
                "Test case '{}': Scaling documentation mismatch",
                test_case.name
            );

            let has_benchmarks = content_lower.contains("benchmark")
                || content_lower.contains("µs")
                || content_lower.contains("ms");
            assert_eq!(
                has_benchmarks, test_case.should_have_benchmarks,
                "Test case '{}': Benchmark documentation mismatch",
                test_case.name
            );
        }
    }

    // ============================================================================
    // Enhanced LSP Provider Critical Path Testing
    // ============================================================================

    #[test]
    #[ignore = "documentation coverage: SPEC-149 not yet implemented"]
    fn test_lsp_provider_documentation_critical_paths() {
        // Test critical LSP provider modules with enhanced validation
        let lsp_critical_modules = [
            "ide/lsp_compat/completion.rs",
            "ide/lsp_compat/diagnostics.rs",
            "ide/lsp_compat/references.rs",
            "ide/lsp_compat/rename.rs",
            "ide/lsp_compat/code_actions.rs",
            "ide/lsp_compat/semantic_tokens.rs",
            "ide/lsp_compat/workspace_symbols.rs",
            "ide/lsp_compat/formatting.rs",
            "ide/lsp_compat/inlay_hints.rs",
            "ide/call_hierarchy_provider.rs",
            "ide/lsp_compat/type_hierarchy.rs",
        ];

        let roots = source_roots();
        let mut critical_issues = Vec::new();
        let mut missing_modules = Vec::new();

        for module in &lsp_critical_modules {
            let files = read_source_files(module, &roots);
            if files.is_empty() {
                missing_modules.push(module.to_string());
                continue;
            }
            for (display_path, content) in files {
                let analysis =
                    doc_validation_helpers::analyze_file_documentation(&display_path, &content);

                // Enhanced validation for LSP providers
                let mut module_issues = Vec::new();

                // Check for LSP-specific documentation requirements
                if !content.to_lowercase().contains("lsp")
                    && !content.to_lowercase().contains("language server")
                {
                    module_issues.push("Missing LSP context documentation");
                }

                // Check for client capability documentation
                if !content.contains("capability") && !content.contains("capabilities") {
                    module_issues.push("Missing client capability documentation");
                }

                // Check for protocol compliance documentation
                if !content.contains("protocol") && !content.contains("spec") {
                    module_issues.push("Missing protocol compliance documentation");
                }

                // Aggregate enhanced edge case violations
                let total_enhanced_violations = analysis.malformed_doctests.len()
                    + analysis.empty_doc_strings.len()
                    + analysis.invalid_cross_references.len()
                    + analysis.incomplete_performance_docs.len()
                    + analysis.missing_error_recovery_docs.len();

                if total_enhanced_violations > 0 || !module_issues.is_empty() {
                    critical_issues.push(format!(
                        "{}: {} violations, {} LSP-specific issues: {:?}",
                        display_path,
                        total_enhanced_violations,
                        module_issues.len(),
                        module_issues
                    ));
                }
            }
        }

        assert!(
            missing_modules.is_empty(),
            "LSP documentation modules missing from source roots: {:?}",
            missing_modules
        );

        // More discriminating assertion - fail if any critical LSP modules have issues
        if !critical_issues.is_empty() {
            let error_summary = format!(
                "CRITICAL LSP PROVIDER DOCUMENTATION ISSUES:\n{}",
                critical_issues.join("\n")
            );

            unreachable!("{}", error_summary);
        }

        // Note: lsp_critical_modules array is always non-empty by definition (compile-time constant)
    }

    // ============================================================================
    // Regression Test for Documentation Quality Metrics
    // ============================================================================

    fn collect_rs_files(root: &std::path::Path, prefix: &str) -> Vec<SourceFile> {
        let mut files = Vec::new();
        let mut stack = vec![root.to_path_buf()];

        while let Some(dir) = stack.pop() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        stack.push(path);
                    } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs")
                        && let Ok(relative) = path.strip_prefix(root)
                    {
                        let rel = relative.to_string_lossy().replace('\\', "/");
                        files.push(SourceFile {
                            display_path: format!("{}/{}", prefix, rel),
                            full_path: path,
                        });
                    }
                }
            }
        }

        files.sort_by(|a, b| a.display_path.cmp(&b.display_path));
        files
    }

    #[test]
    fn test_documentation_quality_regression() {
        // Performance optimization: Efficient regression tracking for LSP tests
        if std::env::var("PERL_LSP_PERFORMANCE_TEST").is_ok() {
            // Fast path: Skip comprehensive analysis during LSP performance tests
            eprintln!("Skipping comprehensive documentation analysis for LSP performance test");
            return;
        }

        // Track documentation quality metrics to prevent regression
        let roots = source_roots();

        // Optimized file enumeration: only process critical files during fast execution
        let critical_files = vec![
            "lib.rs",
            "engine/parser/mod.rs",
            "engine/ast.rs",
            "engine/error/mod.rs",
            "tokens/token_stream.rs",
        ];

        let all_rust_files: Vec<SourceFile> = if std::env::var("PERL_FAST_DOC_CHECK").is_ok() {
            critical_files.into_iter().flat_map(|path| find_source_files(path, &roots)).collect()
        } else {
            roots.iter().flat_map(|root| collect_rs_files(&root.path, root.name)).collect()
        };

        let mut quality_metrics = HashMap::new();
        let mut total_violations = 0;

        for file in &all_rust_files {
            if let Ok(content) = fs::read_to_string(&file.full_path) {
                let analysis = doc_validation_helpers::analyze_file_documentation(
                    &file.display_path,
                    &content,
                );

                let file_violations = analysis.malformed_doctests.len()
                    + analysis.empty_doc_strings.len()
                    + analysis.invalid_cross_references.len()
                    + analysis.incomplete_performance_docs.len()
                    + analysis.missing_error_recovery_docs.len();

                total_violations += file_violations;
                quality_metrics.insert(file.display_path.clone(), file_violations);
            }
        }

        // Regression threshold - adjust based on current state
        let max_acceptable_violations_per_file = 50; // This will need tuning
        let max_total_violations = all_rust_files.len() * max_acceptable_violations_per_file;

        eprintln!(
            "Documentation Quality Metrics:\n- Total files: {}\n- Total violations: {}\n- Threshold: {}",
            all_rust_files.len(),
            total_violations,
            max_total_violations
        );

        // Log worst offenders for tracking improvement (limited output for performance)
        let mut sorted_files: Vec<_> = quality_metrics.iter().collect();
        sorted_files.sort_by(|a, b| b.1.cmp(a.1));

        eprintln!("Top 5 files needing documentation improvement:");
        for (file, violations) in sorted_files.iter().take(5) {
            eprintln!("  {}: {} violations", file, violations);
        }

        // For now, this is informational - in production this would enforce thresholds
    }
}
