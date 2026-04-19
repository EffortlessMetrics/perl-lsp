//! Signature refactoring code actions.
//!
//! Provides the "Add parameter to signature" code action for Perl 5.20+
//! subroutine signatures. Finds all in-file call sites and generates
//! a workspace edit that updates both the signature and every call.

use crate::types::{CodeAction, CodeActionEdit, CodeActionKind};
use perl_lsp_rename::TextEdit;
use perl_parser_core::ast::{Node, NodeKind, SourceLocation};

/// Attempt to generate an "Add parameter to signature" code action.
///
/// Returns `None` when the action is not applicable (anonymous sub, no
/// signature, or last parameter is already slurpy).
pub fn add_parameter_action(source: &str, node: &Node, ast: &Node) -> Option<CodeAction> {
    // Only named subroutines with a signature
    let (sub_name, signature) = match &node.kind {
        NodeKind::Subroutine {
            name: Some(n),
            signature: Some(sig),
            ..
        } => (n.clone(), sig),
        _ => return None,
    };

    // Reject if the last parameter is slurpy (adding after @rest / %opts is invalid)
    if let NodeKind::Signature { parameters } = &signature.kind
        && parameters
            .last()
            .is_some_and(|p| matches!(p.kind, NodeKind::SlurpyParameter { .. }))
    {
        return None;
    }

    // Find the byte offset of the closing `)` of the signature.
    // We pass the subroutine node so we can scan the source directly when
    // the parser gives us an imprecise signature span.
    let sig_close_paren = find_signature_close_paren(source, node, signature)?;

    // Determine the new parameter text.  The default values mirror the spec:
    // `$options = {}`.  This is a fixed default for the MVP; a real
    // interactive implementation would prompt the user.
    let new_param_text = ", $options = {}";
    let call_default_text = ", {}";

    // Collect all call sites for this sub within the same file.
    let call_sites = collect_call_sites(ast, &sub_name);

    // Build the edits — signature first, then call sites.
    let mut changes = Vec::with_capacity(1 + call_sites.len());

    // Edit 1: Insert new parameter before the closing `)` of the signature.
    changes.push(TextEdit {
        location: SourceLocation {
            start: sig_close_paren,
            end: sig_close_paren,
        },
        new_text: new_param_text.to_string(),
    });

    // Edits 2..N: Insert default value before the closing `)` of each call.
    for call_close_paren in call_sites {
        changes.push(TextEdit {
            location: SourceLocation {
                start: call_close_paren,
                end: call_close_paren,
            },
            new_text: call_default_text.to_string(),
        });
    }

    Some(CodeAction {
        title: "Add parameter to signature".to_string(),
        kind: CodeActionKind::RefactorRewrite,
        diagnostics: Vec::new(),
        edit: CodeActionEdit { changes },
        is_preferred: false,
    })
}

/// Find the byte offset of the closing `)` that wraps the signature parameters.
///
/// The parser may produce a zero-length or approximate span for the Signature
/// node, so we locate the `)` by scanning the source from the Signature node's
/// recorded start position (which falls somewhere inside the `(…)` pair).
/// We walk backwards from there to find the opening `(`, tracking nesting, then
/// walk forward to the matching `)`.
///
/// Falls back to scanning forward from `sub_start` looking for the first
/// unmatched `)` when the signature start is unreliable.
fn find_signature_close_paren(
    source: &str,
    subroutine_node: &Node,
    signature: &Node,
) -> Option<usize> {
    // First, try to find the `(` that starts the signature by scanning forward
    // from the subroutine node's start position.  The signature `(` must appear
    // before the `{` that opens the body.
    let sub_start = subroutine_node.location.start;
    let sub_end = subroutine_node.location.end.min(source.len());

    // Find the first `{` in the subroutine span (body opener).
    let body_open = source[sub_start..sub_end]
        .find('{')
        .map(|p| sub_start + p)?;

    // Find the first `(` before the body opener — that's the signature opener.
    let sig_open = source[sub_start..body_open]
        .find('(')
        .map(|p| sub_start + p)?;

    // Now find the matching `)` using paren depth counting.
    let bytes = source.as_bytes();
    let mut depth: usize = 0;
    let mut i = sig_open;
    while i < sub_end {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }

    // Fallback: use rfind on the signature node region if it has a non-zero span.
    let sig_end = signature.location.end.min(source.len());
    let search_start = signature.location.start;
    if sig_end > search_start {
        source[search_start..sig_end]
            .rfind(')')
            .map(|rel| search_start + rel)
    } else {
        None
    }
}

/// Walk the AST and collect the byte offset of the closing `)` for every
/// direct call to `sub_name` (both bare `foo(...)` and qualified `Pkg::foo(...)`).
fn collect_call_sites(ast: &Node, sub_name: &str) -> Vec<usize> {
    let mut sites = Vec::new();
    collect_calls_recursive(ast, sub_name, &mut sites);
    sites
}

fn collect_calls_recursive(node: &Node, sub_name: &str, out: &mut Vec<usize>) {
    if let NodeKind::FunctionCall { name, args } = &node.kind
        && is_call_to(name, sub_name)
    {
        out.extend(find_call_close_paren(node, args));
    }

    // Recurse into children
    visit_children(node, |child| {
        collect_calls_recursive(child, sub_name, out);
    });
}

/// Returns true if a function call name matches the sub (bare or qualified).
fn is_call_to(call_name: &str, sub_name: &str) -> bool {
    // Exact bare name match
    if call_name == sub_name {
        return true;
    }
    // Qualified name ends with ::sub_name
    call_name.rsplit("::").next() == Some(sub_name)
}

/// Find the byte offset of the closing `)` for a FunctionCall node.
///
/// The FunctionCall node's span ends one character past the `)`, so the
/// closing paren is at `node.location.end - 1`.  We insert text at that
/// position to place the new argument before the `)`.
fn find_call_close_paren(call_node: &Node, _args: &[Node]) -> Option<usize> {
    let node_end = call_node.location.end;
    if node_end > 0 {
        Some(node_end - 1)
    } else {
        None
    }
}

/// Visit immediate children of a node with a callback.
///
/// This mirrors the pattern used in `enhanced/mod.rs` for recursive traversal.
fn visit_children<F>(node: &Node, mut f: F)
where
    F: FnMut(&Node),
{
    match &node.kind {
        NodeKind::Program { statements } => {
            for s in statements {
                f(s);
            }
        }
        NodeKind::Block { statements } => {
            for s in statements {
                f(s);
            }
        }
        NodeKind::ExpressionStatement { expression } => f(expression),
        NodeKind::VariableDeclaration {
            variable,
            initializer,
            ..
        } => {
            f(variable);
            if let Some(init) = initializer {
                f(init);
            }
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            f(lhs);
            f(rhs);
        }
        NodeKind::Binary { left, right, .. } => {
            f(left);
            f(right);
        }
        NodeKind::Unary { operand, .. } => f(operand),
        NodeKind::FunctionCall { args, .. } => {
            for a in args {
                f(a);
            }
        }
        NodeKind::MethodCall { object, args, .. } => {
            f(object);
            for a in args {
                f(a);
            }
        }
        NodeKind::Return { value } => {
            value.as_deref().into_iter().for_each(&mut f);
        }
        NodeKind::If {
            condition,
            then_branch,
            elsif_branches,
            else_branch,
        } => {
            f(condition);
            f(then_branch);
            for (cond, branch) in elsif_branches {
                f(cond);
                f(branch);
            }
            if let Some(b) = else_branch {
                f(b);
            }
        }
        NodeKind::While {
            condition, body, ..
        } => {
            f(condition);
            f(body);
        }
        NodeKind::For {
            init,
            condition,
            update,
            body,
            ..
        } => {
            if let Some(init) = init {
                f(init);
            }
            if let Some(cond) = condition {
                f(cond);
            }
            if let Some(upd) = update {
                f(upd);
            }
            f(body);
        }
        NodeKind::Foreach {
            variable,
            list,
            body,
            continue_block,
        } => {
            f(variable);
            f(list);
            f(body);
            if let Some(cb) = continue_block {
                f(cb);
            }
        }
        NodeKind::Subroutine { body, .. } => {
            f(body);
        }
        NodeKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            f(condition);
            f(then_expr);
            f(else_expr);
        }
        _ => {}
    }
}
