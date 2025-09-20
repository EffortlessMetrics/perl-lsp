//! Find-all-references functionality for symbol usage analysis in email scripts
//!
//! This module provides comprehensive reference finding capabilities for email script
//! development within the PSTX pipeline. Enables developers to quickly locate all
//! usage sites of variables, functions, and packages across email processing code.
//!
//! # PSTX Pipeline Integration
//!
//! - **Extract**: Identifies symbol definitions during email script parsing
//! - **Normalize**: Supports refactoring and symbol standardization
//! - **Thread**: Analyzes variable flow and dependencies in email processing
//! - **Render**: Enables reference highlighting and navigation in editors
//! - **Index**: Powers workspace-wide symbol usage tracking
//!
//! # Usage Examples
//!
//! ```rust
//! use perl_parser::{Parser, references::find_references_single_file};
//!
//! let script = "my $email_count = 0; $email_count++; print $email_count;";
//! let mut parser = Parser::new(script);
//! let ast = parser.parse().unwrap();
//!
//! // Find all references to $email_count
//! if let Some(refs) = find_references_single_file(&ast, 3) { // Position of first $email_count
//!     println!("Found {} references to $email_count", refs.len());
//!     for (start, end) in refs {
//!         println!("Reference at {}-{}: {}", start, end, &script[start..end]);
//!     }
//! }
//! ```

use crate::ast::{Node, NodeKind};

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

fn find_node_at_offset(node: &Node, offset: usize) -> Option<&Node> {
    if offset < node.location.start || offset > node.location.end {
        return None;
    }

    // Check children first for more specific match
    let children = get_node_children(node);
    for child in children {
        if let Some(found) = find_node_at_offset(child, offset) {
            return Some(found);
        }
    }

    // If no child contains the offset, return this node
    Some(node)
}

fn get_node_children(node: &Node) -> Vec<&Node> {
    match &node.kind {
        NodeKind::Program { statements } => statements.iter().collect(),
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            let mut children = vec![variable.as_ref()];
            if let Some(init) = initializer {
                children.push(init.as_ref());
            }
            children
        }
        NodeKind::Assignment { lhs, rhs, .. } => vec![lhs.as_ref(), rhs.as_ref()],
        NodeKind::Binary { left, right, .. } => vec![left.as_ref(), right.as_ref()],
        NodeKind::FunctionCall { args, .. } => args.iter().collect(),
        NodeKind::Subroutine { body, .. } => {
            vec![body.as_ref()]
        }
        NodeKind::ExpressionStatement { expression } => vec![expression.as_ref()],
        _ => vec![],
    }
}
