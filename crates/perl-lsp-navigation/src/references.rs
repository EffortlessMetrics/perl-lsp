//! Find-all-references functionality for symbol usage analysis in Perl scripts
//!
//! This module provides comprehensive reference finding capabilities for Perl script
//! development within the LSP workflow. Enables developers to quickly locate all
//! usage sites of variables, functions, and packages across Perl code.
//!
//! # LSP Workflow Integration
//!
//! - **Parse**: Identifies symbol definitions during Perl script parsing
//! - **Index**: Supports refactoring and symbol standardization
//! - **Navigate**: Analyzes variable flow and dependencies in Perl code
//! - **Complete**: Enables reference highlighting and navigation in editors
//! - **Analyze**: Powers workspace-wide symbol usage tracking
//!
//! # Usage Examples
//!
//! ```rust,ignore
//! use perl_parser_core::{Parser, Node};
//! use perl_lsp_providers::ide::lsp_compat::references::find_references_single_file;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let script = "my $count = 0; $count++; print $count;";
//! let mut parser = Parser::new(script);
//! let ast = parser.parse()?;
//!
//! // Find all references to $count
//! if let Some(refs) = find_references_single_file(&ast, 3) { // Position of first $count
//!     println!("Found {} references to $count", refs.len());
//!     for (start, end) in refs {
//!         println!("Reference at {}-{}: {}", start, end, &script[start..end]);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use perl_ast_navigation::{find_node_at_offset, get_node_children};
use perl_parser_core::ast::{Node, NodeKind};

/// Return (start_offset, end_offset) for same-file references
pub fn find_references_single_file(ast: &Node, offset: usize) -> Option<Vec<(usize, usize)>> {
    let needle = find_node_at_offset(ast, offset)?;

    // Determine target "identity"
    let (want_kind, want_pkg, want_name, want_sigil) = match &needle.kind {
        NodeKind::Variable { sigil, name } => {
            let sigil_char = sigil.chars().next();
            ("var", "main".to_string(), name.clone(), sigil_char)
        }
        NodeKind::FunctionCall { name, .. } => {
            let (pkg, bare) = if let Some(idx) = name.rfind("::") {
                (name[..idx].to_string(), name[idx + 2..].to_string())
            } else {
                ("main".to_string(), name.clone())
            };
            ("sub", pkg, bare, None)
        }
        NodeKind::Subroutine { name: Some(name), .. } => {
            let (pkg, bare) = if let Some(idx) = name.rfind("::") {
                (name[..idx].to_string(), name[idx + 2..].to_string())
            } else {
                ("main".to_string(), name.clone())
            };
            ("sub", pkg, bare, None)
        }
        _ => return None,
    };

    let mut out = Vec::new();

    fn walk(
        node: &Node,
        out: &mut Vec<(usize, usize)>,
        want_kind: &str,
        want_pkg: &str,
        want_name: &str,
        want_sigil: Option<char>,
    ) {
        let location = &node.location;
        match &node.kind {
            NodeKind::Variable { sigil, name } if want_kind == "var" => {
                let sig_char = sigil.chars().next();
                if sig_char == want_sigil && name == want_name {
                    out.push((location.start, location.end));
                }
            }
            NodeKind::FunctionCall { name, .. } if want_kind == "sub" => {
                let (pkg, bare) = if let Some(idx) = name.rfind("::") {
                    (&name[..idx], &name[idx + 2..])
                } else {
                    ("main", name.as_str())
                };
                if bare == want_name && pkg == want_pkg {
                    out.push((location.start, location.end));
                }
            }
            NodeKind::Subroutine { name: Some(name), .. } if want_kind == "sub" => {
                if name == want_name {
                    out.push((location.start, location.end));
                }
            }
            _ => {}
        }

        // Walk children
        for ch in get_node_children(node) {
            walk(ch, out, want_kind, want_pkg, want_name, want_sigil);
        }
    }

    walk(ast, &mut out, want_kind, &want_pkg, &want_name, want_sigil);
    Some(out)
}
