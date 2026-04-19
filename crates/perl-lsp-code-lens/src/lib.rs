//! LSP code lens provider for Perl
//!
//! Provides inline actions like "Run Test", "X references" above code elements.
//!
//! ## Features
//!
//! - Run Test lenses for test subroutines
//! - Run All Tests lens for `.t` test files
//! - Run Subtest lenses for `subtest "name" => sub { ... }` blocks
//! - Reference count lenses for subroutines and packages
//! - Run Script lens for files with a Perl shebang line
//!
//! ## Usage
//!
//! ```rust,ignore
//! use perl_lsp_code_lens::{CodeLensProvider, get_shebang_lens, resolve_code_lens};
//!
//! let provider = CodeLensProvider::new(source.to_string())
//!     .with_file_path("t/basic.t".to_string());
//! let lenses = provider.extract(&ast);
//! ```

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_parser_core::ast::{Node, NodeKind};
use perl_position_tracking::{WirePosition, WireRange};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// LSP CodeLens
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeLens {
    /// The range to which this code lens applies
    pub range: WireRange,
    /// The command this code lens represents
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Command>,
    /// Data that will be passed to the CodeLensResolve request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// LSP Command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    /// Title of the command (shown in UI)
    pub title: String,
    /// The identifier of the command to execute
    pub command: String,
    /// Arguments to the command
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<Value>>,
}

/// Check if a file path refers to a Perl test file (`.t` extension)
pub fn is_test_file(path: &str) -> bool {
    path.ends_with(".t")
}

/// Code lens provider
pub struct CodeLensProvider {
    source: String,
    file_path: Option<String>,
}

impl CodeLensProvider {
    /// Create a new code lens provider
    pub fn new(source: String) -> Self {
        Self {
            source,
            file_path: None,
        }
    }

    /// Set the file path for test file detection
    pub fn with_file_path(mut self, path: String) -> Self {
        self.file_path = Some(path);
        self
    }

    /// Extract code lenses from an AST
    pub fn extract(&self, ast: &Node) -> Vec<CodeLens> {
        let mut lenses = Vec::new();

        // Add "Run All Tests" lens at top of .t files
        if self.file_path.as_ref().is_some_and(|p| is_test_file(p)) {
            lenses.push(CodeLens {
                range: WireRange::empty(WirePosition::new(0, 0)),
                command: Some(Command {
                    title: "\u{25b6} Run All Tests".to_string(),
                    command: "perl.runTestFile".to_string(),
                    arguments: self.file_path.as_ref().map(|p| vec![json!(p)]),
                }),
                data: None,
            });
        }

        self.visit_node(ast, &mut lenses);
        lenses
    }

    /// Visit a node and extract code lenses
    fn visit_node(&self, node: &Node, lenses: &mut Vec<CodeLens>) {
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.visit_node(stmt, lenses);
                }
            }

            NodeKind::Subroutine {
                name,
                prototype: _,
                signature: _,
                attributes: _,
                body,
                name_span: _,
            } => {
                if let Some(name) = name {
                    if self.is_test_subroutine(name) {
                        self.add_run_test_lens(node, name, lenses);
                    }
                    self.add_references_lens(node, name, lenses);
                }
                self.visit_node(body, lenses);
            }

            NodeKind::Package {
                name,
                block,
                name_span: _,
            } => {
                self.add_references_lens(node, name, lenses);
                if let Some(block) = block {
                    self.visit_node(block, lenses);
                }
            }

            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.visit_node(stmt, lenses);
                }
            }

            NodeKind::ExpressionStatement { expression } => {
                self.visit_node(expression, lenses);
            }

            NodeKind::FunctionCall { name, args } => {
                if name == "subtest" {
                    self.add_subtest_lens(node, args, lenses);
                }
            }

            _ => {
                self.visit_children(node, lenses);
            }
        }
    }

    /// Check if a subroutine is a test
    fn is_test_subroutine(&self, name: &str) -> bool {
        let core = name.starts_with("test_")
            || name.ends_with("_test")
            || name.starts_with("t_")
            || name == "test";
        let test_file_only = self.file_path.as_deref().map(is_test_file).unwrap_or(false)
            && (name.starts_with("ok_")
                || name.starts_with("is_")
                || name.starts_with("like_")
                || name.starts_with("can_"));
        core || test_file_only
    }

    /// Add "Run Test" and "Debug Test" code lenses
    fn add_run_test_lens(&self, node: &Node, name: &str, lenses: &mut Vec<CodeLens>) {
        let range =
            WireRange::from_byte_offsets(&self.source, node.location.start, node.location.end);
        let file_path_str = self.file_path.as_deref().unwrap_or("");
        let test_id = format!("{}::{}", file_path_str, name);
        lenses.push(CodeLens {
            range,
            command: Some(Command {
                title: "\u{25b6} Run Test".to_string(),
                command: "perl.runTest".to_string(),
                arguments: Some(vec![json!(test_id)]),
            }),
            data: None,
        });
        lenses.push(CodeLens {
            range,
            command: Some(Command {
                title: "\u{1f41e} Debug Test".to_string(),
                command: "perl.debugTest".to_string(),
                arguments: Some(vec![json!(test_id)]),
            }),
            data: None,
        });
    }

    /// Add a "Run Subtest" code lens for `subtest "name" => sub { ... }`
    fn add_subtest_lens(&self, node: &Node, args: &[Node], lenses: &mut Vec<CodeLens>) {
        let subtest_name = args.first().and_then(|arg| match &arg.kind {
            NodeKind::String { value, .. } => {
                // Token text includes surrounding quotes (e.g. `"basic math"`);
                // strip them to get the bare label.
                Some(extract_quoted_string(value.as_str()).unwrap_or(value.as_str()))
            }
            _ => None,
        });
        let name = subtest_name.unwrap_or("<anonymous>");
        let range =
            WireRange::from_byte_offsets(&self.source, node.location.start, node.location.end);
        lenses.push(CodeLens {
            range,
            command: Some(Command {
                title: format!("\u{25b6} Run Subtest: {name}"),
                command: "perl.runSubtest".to_string(),
                arguments: Some(vec![json!(name)]),
            }),
            data: None,
        });
    }

    /// Add an "X references" code lens
    fn add_references_lens(&self, node: &Node, name: &str, lenses: &mut Vec<CodeLens>) {
        let start_pos = WirePosition::from_byte_offset(&self.source, node.location.start);
        lenses.push(CodeLens {
            range: WireRange::empty(start_pos),
            command: None,
            data: Some(json!({
                "name": name,
                "kind": match &node.kind {
                    NodeKind::Subroutine { .. } => "subroutine",
                    NodeKind::Package { .. } => "package",
                    _ => "unknown",
                }
            })),
        });
    }

    /// Detect subtest patterns using text-based scanning
    ///
    /// Since the parser may represent `subtest "name" => sub { ... }` as an
    /// identifier + hash rather than a FunctionCall, we use text-based scanning
    /// to reliably detect subtest blocks.
    pub fn extract_subtest_lenses(source: &str) -> Vec<CodeLens> {
        let mut lenses = Vec::new();
        for (line_num, line) in source.lines().enumerate() {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("subtest ") {
                let name = extract_quoted_string(rest);
                let label = name.unwrap_or("<anonymous>");
                let col = line.len() - trimmed.len();
                lenses.push(CodeLens {
                    range: WireRange::new(
                        WirePosition::new(line_num as u32, col as u32),
                        WirePosition::new(line_num as u32, (col + trimmed.len()) as u32),
                    ),
                    command: Some(Command {
                        title: format!("\u{25b6} Run Subtest: {label}"),
                        command: "perl.runSubtest".to_string(),
                        arguments: Some(vec![json!(label)]),
                    }),
                    data: None,
                });
            }
        }
        lenses
    }

    /// Visit all children of a node generically
    #[allow(clippy::ptr_arg)]
    fn visit_children(&self, _node: &Node, _lenses: &mut Vec<CodeLens>) {
        // Most nodes don't have generic children to visit
    }
}

/// Extract a quoted string value from text like `"name"` or `'name'`
fn extract_quoted_string(s: &str) -> Option<&str> {
    let s = s.trim_start();
    let quote = s.as_bytes().first()?;
    if *quote != b'"' && *quote != b'\'' {
        return None;
    }
    let end = s[1..].find(*quote as char)?;
    Some(&s[1..1 + end])
}

/// Resolve a code lens (add command with reference count)
pub fn resolve_code_lens(lens: CodeLens, reference_count: usize) -> CodeLens {
    if lens.command.is_none() && lens.data.is_some() {
        let _name = lens
            .data
            .as_ref()
            .and_then(|d| d.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");
        let range = lens.range;
        CodeLens {
            range,
            command: Some(Command {
                title: format!(
                    "{} reference{}",
                    reference_count,
                    if reference_count == 1 { "" } else { "s" }
                ),
                command: "editor.action.findReferences".to_string(),
                arguments: Some(vec![json!(range.start.line), json!(range.start.character)]),
            }),
            data: lens.data,
        }
    } else {
        lens
    }
}

/// Check if the file has a shebang line and return a "Run Script" lens
pub fn get_shebang_lens(source: &str) -> Option<CodeLens> {
    if source.starts_with("#!") && source.contains("perl") {
        Some(CodeLens {
            range: WireRange::empty(WirePosition::new(0, 0)),
            command: Some(Command {
                title: "\u{25b6} Run Script".to_string(),
                command: "perl.runScript".to_string(),
                arguments: None,
            }),
            data: None,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must_some;

    fn extract_lenses(source: &str) -> Result<Vec<CodeLens>, String> {
        let mut parser = perl_parser::Parser::new(source);
        let ast = parser.parse().map_err(|e| format!("parse error: {}", e))?;
        let provider = CodeLensProvider::new(source.to_string());
        Ok(provider.extract(&ast))
    }

    fn extract_lenses_with_path(source: &str, path: &str) -> Result<Vec<CodeLens>, String> {
        let mut parser = perl_parser::Parser::new(source);
        let ast = parser.parse().map_err(|e| format!("parse error: {}", e))?;
        let provider = CodeLensProvider::new(source.to_string()).with_file_path(path.to_string());
        Ok(provider.extract(&ast))
    }

    #[test]
    fn test_code_lens_extraction() -> Result<(), String> {
        let source = "#!/usr/bin/perl\n\npackage TestPackage;\n\nsub test_basic {\n    ok(1, \"basic test\");\n}\n\nsub helper_function {\n    return 42;\n}\n\nsub test_another {\n    is(helper_function(), 42);\n}\n";
        let lenses = extract_lenses(source)?;
        assert!(lenses.len() >= 5);
        let run_test_lenses: Vec<_> = lenses
            .iter()
            .filter(|l| {
                l.command
                    .as_ref()
                    .map(|c| c.command == "perl.runTest")
                    .unwrap_or(false)
            })
            .collect();
        assert_eq!(run_test_lenses.len(), 2);
        Ok(())
    }

    #[test]
    fn test_shebang_lens() {
        let source = "#!/usr/bin/perl\nprint 'hello';\n";
        let lens = get_shebang_lens(source);
        assert!(lens.is_some());
        if let Some(lens) = lens {
            let cmd_opt = &lens.command;
            assert!(cmd_opt.is_some(), "expected command in shebang lens");
            if let Some(cmd) = cmd_opt {
                assert_eq!(cmd.title, "\u{25b6} Run Script");
            }
        }
        let source = "use strict;\nprint 'hello';\n";
        let lens = get_shebang_lens(source);
        assert!(lens.is_none());
    }

    #[test]
    fn test_resolve_code_lens() {
        let unresolved = CodeLens {
            range: WireRange::empty(WirePosition::new(5, 0)),
            command: None,
            data: Some(json!({ "name": "foo", "kind": "subroutine" })),
        };
        let resolved = resolve_code_lens(unresolved, 3);
        let cmd_opt = &resolved.command;
        assert!(cmd_opt.is_some(), "expected command in resolved lens");
        if let Some(cmd) = cmd_opt {
            assert_eq!(cmd.title, "3 references");
        }
    }

    #[test]
    fn test_reference_lens_position_accuracy() -> Result<(), String> {
        let source = "package TestPackage;\n\nsub first_function {\n    return 1;\n}\n";
        let lenses = extract_lenses(source)?;
        let reference_lens = lenses
            .iter()
            .find(|lens| {
                lens.data
                    .as_ref()
                    .and_then(|data| data.get("name"))
                    .and_then(|name| name.as_str())
                    == Some("first_function")
            })
            .ok_or("missing reference lens for first_function")?;
        assert_eq!(
            reference_lens.range,
            WireRange::empty(WirePosition::new(2, 0))
        );
        assert_eq!(
            reference_lens.range.start.to_byte_offset(source),
            source.find("sub first_function").unwrap_or(usize::MAX)
        );
        Ok(())
    }

    #[test]
    fn test_run_test_lens_range() -> Result<(), String> {
        let source = "sub test_basic {\n    ok(1, \"basic test\");\n}\n\nsub helper_function {\n    return 42;\n}\n";
        let lenses = extract_lenses(source)?;
        let run_test_lens = lenses
            .iter()
            .find(|lens| {
                lens.command.as_ref().is_some_and(|command| {
                    command.command == "perl.runTest"
                        && command.arguments.as_ref().is_some_and(|arguments| {
                            arguments
                                .first()
                                .and_then(|v| v.as_str())
                                .is_some_and(|s| s.ends_with("::test_basic"))
                        })
                })
            })
            .ok_or("missing run test lens for test_basic")?;
        let start = run_test_lens.range.start.to_byte_offset(source);
        let end = run_test_lens.range.end.to_byte_offset(source);
        assert_eq!(start, source.find("sub test_basic").unwrap_or(usize::MAX));
        assert!(end > start, "run test lens should cover a non-empty range");
        Ok(())
    }

    #[test]
    fn test_is_test_file() {
        assert!(is_test_file("t/basic.t"));
        assert!(is_test_file("/home/user/project/t/01-parse.t"));
        assert!(is_test_file("test.t"));
        assert!(!is_test_file("lib/Foo.pm"));
        assert!(!is_test_file("script.pl"));
        assert!(!is_test_file("t/lib/TestHelper.pm"));
    }

    #[test]
    fn test_run_all_tests_lens_for_t_file() -> Result<(), String> {
        let source = "use Test::More;\nok(1, \"basic\");\ndone_testing();\n";
        let lenses = extract_lenses_with_path(source, "t/basic.t")?;
        let run_all = lenses
            .iter()
            .find(|l| {
                l.command
                    .as_ref()
                    .is_some_and(|c| c.command == "perl.runTestFile")
            })
            .ok_or("missing Run All Tests lens for .t file")?;
        let cmd = run_all.command.as_ref().ok_or("missing command")?;
        assert_eq!(cmd.title, "\u{25b6} Run All Tests");
        assert_eq!(run_all.range, WireRange::empty(WirePosition::new(0, 0)));
        let args = cmd.arguments.as_ref().ok_or("missing arguments")?;
        assert_eq!(args, &[json!("t/basic.t")]);
        Ok(())
    }

    #[test]
    fn test_no_run_all_tests_for_pm_file() -> Result<(), String> {
        let source = "package Foo;\nsub bar { 1 }\n1;\n";
        let lenses = extract_lenses_with_path(source, "lib/Foo.pm")?;
        let run_all = lenses.iter().any(|l| {
            l.command
                .as_ref()
                .is_some_and(|c| c.command == "perl.runTestFile")
        });
        assert!(!run_all, "should not have Run All Tests for .pm file");
        Ok(())
    }

    #[test]
    fn test_no_run_all_tests_without_file_path() -> Result<(), String> {
        let source = "use Test::More;\nok(1);\n";
        let lenses = extract_lenses(source)?;
        let run_all = lenses.iter().any(|l| {
            l.command
                .as_ref()
                .is_some_and(|c| c.command == "perl.runTestFile")
        });
        assert!(!run_all, "should not have Run All Tests without file path");
        Ok(())
    }

    #[test]
    fn test_subtest_lens_detection() -> Result<(), String> {
        let source = "use Test::More;\n\nsubtest \"basic math\" => sub {\n    is(1 + 1, 2, \"addition works\");\n};\n\nsubtest \"string ops\" => sub {\n    is(\"hello\" . \" world\", \"hello world\", \"concatenation\");\n};\n\ndone_testing();\n";
        let lenses = extract_lenses_with_path(source, "t/math.t")?;
        let subtest_lenses: Vec<_> = lenses
            .iter()
            .filter(|l| {
                l.command
                    .as_ref()
                    .is_some_and(|c| c.command == "perl.runSubtest")
            })
            .collect();
        assert_eq!(
            subtest_lenses.len(),
            2,
            "expected 2 subtest lenses, got {}: {:?}",
            subtest_lenses.len(),
            subtest_lenses
                .iter()
                .map(|l| l.command.as_ref().map(|c| &c.title))
                .collect::<Vec<_>>()
        );
        let titles: Vec<_> = subtest_lenses
            .iter()
            .filter_map(|l| l.command.as_ref().map(|c| c.title.as_str()))
            .collect();
        assert!(
            titles.contains(&"\u{25b6} Run Subtest: basic math"),
            "missing 'basic math' subtest lens"
        );
        assert!(
            titles.contains(&"\u{25b6} Run Subtest: string ops"),
            "missing 'string ops' subtest lens"
        );
        Ok(())
    }

    #[test]
    fn test_subtest_lens_without_t_extension() -> Result<(), String> {
        let source = "use Test::More;\nsubtest \"my test\" => sub { ok(1) };\ndone_testing();\n";
        let lenses = extract_lenses(source)?;
        let subtest_lenses: Vec<_> = lenses
            .iter()
            .filter(|l| {
                l.command
                    .as_ref()
                    .is_some_and(|c| c.command == "perl.runSubtest")
            })
            .collect();
        assert_eq!(
            subtest_lenses.len(),
            1,
            "subtest detection should work without .t path"
        );
        Ok(())
    }

    #[test]
    fn test_subtest_single_quotes() {
        let lenses =
            CodeLensProvider::extract_subtest_lenses("subtest 'quoted name' => sub { };\n");
        assert_eq!(lenses.len(), 1);
        let cmd = must_some(lenses[0].command.as_ref());
        assert_eq!(cmd.title, "\u{25b6} Run Subtest: quoted name");
    }

    #[test]
    fn test_extract_quoted_string() {
        assert_eq!(extract_quoted_string("\"hello\" => sub"), Some("hello"));
        assert_eq!(extract_quoted_string("'world' => sub"), Some("world"));
        assert_eq!(extract_quoted_string("no_quotes => sub"), None);
    }

    #[test]
    fn test_no_duplicate_subtest_lenses_comma_style() -> Result<(), String> {
        let source = "use Test::More;\nsubtest \"my test\", sub { ok(1) };\ndone_testing();\n";
        let lenses = extract_lenses(source)?;
        let subtest_lenses: Vec<_> = lenses
            .iter()
            .filter(|l| {
                l.command
                    .as_ref()
                    .is_some_and(|c| c.command == "perl.runSubtest")
            })
            .collect();
        assert_eq!(
            subtest_lenses.len(),
            1,
            "comma-style subtest should produce exactly 1 lens, got {}",
            subtest_lenses.len()
        );
        Ok(())
    }
}
