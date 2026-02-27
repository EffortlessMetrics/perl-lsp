//! Code Lens provider for LSP
//!
//! Provides inline actions like "Run Test", "X references" above code elements.

use crate::convert::{WirePosition, WireRange};
use perl_parser::ast::{Node, NodeKind};
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

/// Code lens provider
pub struct CodeLensProvider {
    source: String,
}

impl CodeLensProvider {
    /// Create a new code lens provider
    pub fn new(source: String) -> Self {
        Self { source }
    }

    /// Extract code lenses from an AST
    pub fn extract(&self, ast: &Node) -> Vec<CodeLens> {
        let mut lenses = Vec::new();
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
                    // Add "Run Test" lens for test subroutines
                    if self.is_test_subroutine(name) {
                        self.add_run_test_lens(node, name, lenses);
                    }

                    // Add "X references" lens for all subroutines
                    self.add_references_lens(node, name, lenses);
                }

                // Visit body
                self.visit_node(body, lenses);
            }

            NodeKind::Package { name, block, name_span: _ } => {
                // Add "X references" lens for packages
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

            // Look for shebang line to add "Run Script" lens
            _ => {
                // Visit children for other node types
                self.visit_children(node, lenses);
            }
        }
    }

    /// Check if a subroutine is a test
    fn is_test_subroutine(&self, name: &str) -> bool {
        // Common test naming patterns
        name.starts_with("test_") || 
        name.ends_with("_test") ||
        name.starts_with("t_") ||
        name == "test" ||
        // Test::More style
        name.starts_with("ok_") ||
        name.starts_with("is_") ||
        name.starts_with("like_") ||
        name.starts_with("can_")
    }

    /// Add a "Run Test" code lens
    fn add_run_test_lens(&self, node: &Node, name: &str, lenses: &mut Vec<CodeLens>) {
        let range =
            WireRange::from_byte_offsets(&self.source, node.location.start, node.location.end);

        lenses.push(CodeLens {
            range,
            command: Some(Command {
                title: "▶ Run Test".to_string(),
                command: "perl.runTest".to_string(),
                arguments: Some(vec![json!(name)]),
            }),
            data: None,
        });
    }

    /// Add an "X references" code lens
    fn add_references_lens(&self, node: &Node, name: &str, lenses: &mut Vec<CodeLens>) {
        let start_pos = WirePosition::from_byte_offset(&self.source, node.location.start);

        // Put the lens on the same line as the declaration (zero-width range)
        lenses.push(CodeLens {
            range: WireRange::empty(start_pos),
            command: None, // Will be resolved later
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

    /// Visit all children of a node generically
    #[allow(clippy::ptr_arg)] // might need Vec in future for push operations
    fn visit_children(&self, _node: &Node, _lenses: &mut Vec<CodeLens>) {
        // Most nodes don't have generic children to visit
    }
}

/// Resolve a code lens (add command with reference count)
pub fn resolve_code_lens(lens: CodeLens, reference_count: usize) -> CodeLens {
    if lens.command.is_none() && lens.data.is_some() {
        // This is a references lens that needs resolving
        let _name = lens
            .data
            .as_ref()
            .and_then(|d| d.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");

        let range = lens.range; // Copy the range

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
                title: "▶ Run Script".to_string(),
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
    use crate::Parser;

    #[test]
    fn test_code_lens_extraction() {
        let source = r#"#!/usr/bin/perl

package TestPackage;

sub test_basic {
    ok(1, "basic test");
}

sub helper_function {
    return 42;
}

sub test_another {
    is(helper_function(), 42);
}
"#;

        let mut parser = Parser::new(source);
        let ast = parser.parse().map_err(|e| format!("parse error: {}", e));
        if let Ok(ast) = ast {
            let provider = CodeLensProvider::new(source.to_string());
            let lenses = provider.extract(&ast);

            // Should have lenses for:
            // - Package reference
            // - test_basic (run test + references)
            // - helper_function (references)
            // - test_another (run test + references)
            assert!(lenses.len() >= 5);

            // Check for run test lenses
            let run_test_lenses: Vec<_> = lenses
                .iter()
                .filter(|l| {
                    l.command.as_ref().map(|c| c.command == "perl.runTest").unwrap_or(false)
                })
                .collect();
            assert_eq!(run_test_lenses.len(), 2); // test_basic and test_another
        }
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
                assert_eq!(cmd.title, "▶ Run Script");
            }
        }

        // No shebang
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
}
