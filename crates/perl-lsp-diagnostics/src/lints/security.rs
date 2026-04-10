//! Security-focused lint checks
//!
//! This module provides lint checks that detect common security anti-patterns
//! in Perl code. These are patterns that `perl -c` and PPI cannot catch because
//! they require AST-level analysis.
//!
//! # Diagnostic codes
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `security-two-arg-open` | Warning | `open(FH, ">file")` -- use 3-arg open for safety |
//! | `security-string-eval` | Warning | `eval "$string"` -- string eval is a security risk |
//! | `security-backtick-exec` | Information | Backtick/qx command execution detected |
//! | `security-signal-handler` | Warning | Global `$SIG{__DIE__}` / `$SIG{__WARN__}` assignment |

use perl_diagnostics_codes::DiagnosticCode;
use perl_parser_core::ast::{Node, NodeKind};

use super::super::walker::walk_node;
use perl_lsp_diagnostic_types::{Diagnostic, DiagnosticSeverity, RelatedInformation};

/// Check for security anti-patterns
///
/// This function walks the AST looking for:
/// - Two-argument `open` calls (should use 3-arg form)
/// - String `eval` (security risk vs. block `eval`)
/// - Backtick/qx command execution (ensure input is sanitized)
/// - Global signal-handler assignment to `$SIG{__DIE__}` / `$SIG{__WARN__}`
pub fn check_security(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    walk_node(node, &mut |n| {
        match &n.kind {
            NodeKind::FunctionCall { name, args } => {
                check_two_arg_open(name, args, n, diagnostics);
                check_string_eval(name, args, n, diagnostics);
            }

            NodeKind::Assignment { lhs, .. } => {
                check_global_signal_handler_assignment(lhs, n, diagnostics);
            }

            // The parser produces Eval { block } for both `eval { ... }` and
            // `eval "string"`.  Block evals are safe (exception handling);
            // string/variable evals are a security risk.
            NodeKind::Eval { block } => {
                check_eval_node(block, n, diagnostics);
            }

            // Backtick strings: the parser stores `cmd` and qx(cmd) as
            // String { value: "`cmd`", interpolated: true }
            NodeKind::String { value, interpolated: true } if is_backtick_string(value) => {
                diagnostics.push(Diagnostic {
                    range: (n.location.start, n.location.end),
                    severity: DiagnosticSeverity::Information,
                    code: Some(DiagnosticCode::SecurityBacktickExec.as_str().to_string()),
                    message: "Command execution detected. Ensure input is sanitized.".to_string(),
                    related_information: vec![RelatedInformation {
                        location: (n.location.start, n.location.end),
                        message: "Consider using open() with a pipe, or IPC::Run for safer command execution with proper input validation".to_string(),
                    }],
                    tags: Vec::new(),
                    suggestion: Some(
                        "Use open(my $fh, '-|', @cmd) or IPC::Run for safer command execution"
                            .to_string(),
                    ),
                });
            }

            _ => {}
        }
    });
}

/// Detect a global assignment to `$SIG{__DIE__}` or `$SIG{__WARN__}`.
fn check_global_signal_handler_assignment(
    lhs: &Node,
    node: &Node,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(signal_name) = signal_handler_name(lhs) else {
        return;
    };

    diagnostics.push(Diagnostic {
        range: (node.location.start, node.location.end),
        severity: DiagnosticSeverity::Warning,
        code: Some(DiagnosticCode::SecuritySignalHandler.as_str().to_string()),
        message: format!(
            "Global assignment to $SIG{{{signal_name}}} changes process-wide behavior. Use local $SIG{{...}} to scope the handler."
        ),
        related_information: vec![RelatedInformation {
            location: (node.location.start, node.location.end),
            message: "Localized signal handlers avoid leaking exception or warning hooks across unrelated code.".to_string(),
        }],
        tags: Vec::new(),
        suggestion: Some(format!(
            "Use `local $SIG{{{signal_name}}} = ...` if the handler should be scoped"
        )),
    });
}

/// Extract the signal-handler key if the node targets `$SIG{__DIE__}` or `$SIG{__WARN__}`.
fn signal_handler_name(node: &Node) -> Option<&str> {
    let NodeKind::Binary { op, left, right } = &node.kind else {
        return None;
    };

    if op != "{}" {
        return None;
    }

    let is_sig_hash = matches!(
        &left.kind,
        NodeKind::Variable { sigil, name } if (sigil == "$" || sigil == "%") && name == "SIG"
    );
    if !is_sig_hash {
        return None;
    }

    match &right.kind {
        NodeKind::Identifier { name } if name == "__DIE__" || name == "__WARN__" => {
            Some(name.as_str())
        }
        NodeKind::String { value, .. } => {
            let trimmed = value.trim_matches(['"', '\'']);
            if trimmed == "__DIE__" || trimmed == "__WARN__" { Some(trimmed) } else { None }
        }
        _ => None,
    }
}

/// Detect string `eval` from `NodeKind::Eval` nodes.
///
/// The parser produces `Eval { block }` for both `eval { ... }` and
/// `eval "string"`. Block evals (`eval { ... }`) are safe exception handling;
/// string/variable evals are a security risk.
fn check_eval_node(block: &Node, eval_node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    let is_string_eval = matches!(&block.kind, NodeKind::String { .. } | NodeKind::Variable { .. })
        || matches!(&block.kind, NodeKind::Binary { op, .. } if op == ".");

    if !is_string_eval {
        return;
    }

    diagnostics.push(Diagnostic {
        range: (eval_node.location.start, eval_node.location.end),
        severity: DiagnosticSeverity::Warning,
        code: Some(DiagnosticCode::SecurityStringEval.as_str().to_string()),
        message: "String eval is a security risk. Consider eval { } for exception handling."
            .to_string(),
        related_information: vec![RelatedInformation {
            location: (eval_node.location.start, eval_node.location.end),
            message: "String eval executes arbitrary Perl code at runtime. If the string contains user input, this allows code injection.".to_string(),
        }],
        tags: Vec::new(),
        suggestion: Some(
            "Use eval { } for exception handling, or consider safer alternatives like Try::Tiny"
                .to_string(),
        ),
    });
}

/// Detect two-argument `open` calls.
///
/// `open(FH, ">file")` is unsafe because the mode and filename are combined,
/// allowing shell injection if the filename comes from user input.
fn check_two_arg_open(name: &str, args: &[Node], node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    if name != "open" || args.len() != 2 {
        return;
    }

    diagnostics.push(Diagnostic {
        range: (node.location.start, node.location.end),
        severity: DiagnosticSeverity::Warning,
        code: Some(DiagnosticCode::TwoArgOpen.as_str().to_string()),
        message: "Use 3-argument open for safety: open(my $fh, '>', 'file')".to_string(),
        related_information: vec![RelatedInformation {
            location: (node.location.start, node.location.end),
            message: "Two-argument open combines mode and filename, which can allow shell injection if the filename is derived from user input".to_string(),
        }],
        tags: Vec::new(),
        suggestion: Some("Replace with 3-arg form: open(my $fh, '>', $file)".to_string()),
    });
}

/// Detect string `eval` calls.
///
/// `eval "code"` executes arbitrary Perl code at runtime, which is a security
/// risk when the string contains user input. Block eval (`eval { ... }`) is
/// safe and used for exception handling.
fn check_string_eval(name: &str, args: &[Node], node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    if name != "eval" {
        return;
    }

    // Check that the first argument is a string (not a block/other expression).
    // eval { ... } is parsed as NodeKind::Eval, not FunctionCall, so reaching
    // here already means this is the function-call form. But we still check
    // the arg is a string to avoid false positives on eval($coderef).
    let is_string_arg = args.first().is_some_and(|arg| match &arg.kind {
        NodeKind::String { .. } | NodeKind::Variable { .. } => true,
        NodeKind::Binary { op, .. } if op == "." => true,
        _ => false,
    });

    if !is_string_arg && !args.is_empty() {
        return;
    }

    diagnostics.push(Diagnostic {
        range: (node.location.start, node.location.end),
        severity: DiagnosticSeverity::Warning,
        code: Some(DiagnosticCode::SecurityStringEval.as_str().to_string()),
        message: "String eval is a security risk. Consider eval { } for exception handling."
            .to_string(),
        related_information: vec![RelatedInformation {
            location: (node.location.start, node.location.end),
            message: "String eval executes arbitrary Perl code at runtime. If the string contains user input, this allows code injection.".to_string(),
        }],
        tags: Vec::new(),
        suggestion: Some(
            "Use eval { } for exception handling, or consider safer alternatives like Try::Tiny"
                .to_string(),
        ),
    });
}

/// Check if a string value represents a backtick command execution.
///
/// The parser stores backtick literals (`` `cmd` ``) and qx(cmd) as
/// `String { value: "`cmd`", interpolated: true }`.
fn is_backtick_string(value: &str) -> bool {
    value.starts_with('`') && value.ends_with('`') && value.len() >= 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser::Parser;
    use perl_tdd_support::must;

    fn security_diags(source: &str) -> Vec<Diagnostic> {
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_security(&ast, &mut diags);
        diags
    }

    #[test]
    fn global_sig_warn_handler_is_flagged() {
        let diags = security_diags("$SIG{__WARN__} = sub { };");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL602")),
            "global __WARN__ handler should be flagged: {diags:?}"
        );
    }

    #[test]
    fn global_sig_die_handler_is_flagged() {
        let diags = security_diags("%SIG{__DIE__} = sub { };");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL602")),
            "global __DIE__ handler should be flagged: {diags:?}"
        );
    }

    #[test]
    fn local_sig_handlers_are_not_flagged() {
        let warn_diags = security_diags("local $SIG{__WARN__} = sub { };");
        let die_diags = security_diags("local $SIG{__DIE__} = sub { };");

        assert!(
            warn_diags.iter().all(|d| d.code.as_deref() != Some("PL602")),
            "localized __WARN__ handler should not be flagged: {warn_diags:?}"
        );
        assert!(
            die_diags.iter().all(|d| d.code.as_deref() != Some("PL602")),
            "localized __DIE__ handler should not be flagged: {die_diags:?}"
        );
    }
}
