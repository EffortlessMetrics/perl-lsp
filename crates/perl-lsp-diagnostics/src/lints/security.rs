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

use perl_lsp_diagnostic_types::{Diagnostic, DiagnosticSeverity, RelatedInformation};

/// Check for security anti-patterns
///
/// This function walks the AST looking for:
/// - Two-argument `open` calls (should use 3-arg form)
/// - String `eval` (security risk vs. block `eval`)
/// - Backtick/qx command execution (ensure input is sanitized)
/// - Global signal-handler assignment to `$SIG{__DIE__}` / `$SIG{__WARN__}`
pub fn check_security(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    walk_security_node(node, diagnostics, false);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SignalTableAccess {
    Bare,
    MainQualified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SignalHandlerTarget {
    access: SignalTableAccess,
    signal_name: String,
}

fn walk_security_node(
    node: &Node,
    diagnostics: &mut Vec<Diagnostic>,
    signal_shadowed: bool,
) -> bool {
    match &node.kind {
        NodeKind::Program { statements } => {
            let mut current_shadowed = signal_shadowed;
            for stmt in statements {
                current_shadowed = walk_security_node(stmt, diagnostics, current_shadowed);
            }
            current_shadowed
        }
        NodeKind::Block { statements } => {
            let mut block_shadowed = signal_shadowed;
            for stmt in statements {
                block_shadowed = walk_security_node(stmt, diagnostics, block_shadowed);
            }
            signal_shadowed
        }
        NodeKind::ExpressionStatement { expression } => {
            walk_security_node(expression, diagnostics, signal_shadowed);
            signal_shadowed
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            check_global_signal_handler_assignment(lhs, node, diagnostics, signal_shadowed);
            walk_security_node(lhs, diagnostics, signal_shadowed);
            walk_security_node(rhs, diagnostics, signal_shadowed);
            signal_shadowed
        }
        NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
            if let Some(init) = initializer {
                walk_security_node(init, diagnostics, signal_shadowed);
            }

            let mut updated_shadowed = signal_shadowed;
            if matches!(declarator.as_str(), "my" | "state") && shadows_signal_table(variable) {
                updated_shadowed = true;
            }

            if declarator != "local" {
                walk_security_node(variable, diagnostics, signal_shadowed);
            }
            updated_shadowed
        }
        NodeKind::VariableListDeclaration { declarator, variables, initializer, .. } => {
            if let Some(init) = initializer {
                walk_security_node(init, diagnostics, signal_shadowed);
            }

            if declarator != "local" {
                for variable in variables {
                    walk_security_node(variable, diagnostics, signal_shadowed);
                }
            }

            if matches!(declarator.as_str(), "my" | "state")
                && variables.iter().any(shadows_signal_table)
            {
                true
            } else {
                signal_shadowed
            }
        }
        NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
            walk_security_node(condition, diagnostics, signal_shadowed);
            walk_security_node(then_branch, diagnostics, signal_shadowed);
            for (condition, branch) in elsif_branches {
                walk_security_node(condition, diagnostics, signal_shadowed);
                walk_security_node(branch, diagnostics, signal_shadowed);
            }
            if let Some(branch) = else_branch {
                walk_security_node(branch, diagnostics, signal_shadowed);
            }
            signal_shadowed
        }
        NodeKind::While { condition, body, .. } => {
            walk_security_node(condition, diagnostics, signal_shadowed);
            walk_security_node(body, diagnostics, signal_shadowed);
            signal_shadowed
        }
        NodeKind::For { init, condition, update, body, continue_block } => {
            let mut loop_shadowed = signal_shadowed;
            if let Some(init) = init {
                loop_shadowed = walk_security_node(init, diagnostics, loop_shadowed);
            }
            if let Some(condition) = condition {
                walk_security_node(condition, diagnostics, loop_shadowed);
            }
            if let Some(update) = update {
                walk_security_node(update, diagnostics, loop_shadowed);
            }
            walk_security_node(body, diagnostics, loop_shadowed);
            if let Some(continue_block) = continue_block {
                walk_security_node(continue_block, diagnostics, loop_shadowed);
            }
            signal_shadowed
        }
        NodeKind::Foreach { variable, list, body, continue_block } => {
            let mut loop_shadowed = walk_security_node(variable, diagnostics, signal_shadowed);
            if shadows_signal_table(variable) {
                loop_shadowed = true;
            }
            walk_security_node(list, diagnostics, signal_shadowed);
            walk_security_node(body, diagnostics, loop_shadowed);
            if let Some(continue_block) = continue_block {
                walk_security_node(continue_block, diagnostics, loop_shadowed);
            }
            signal_shadowed
        }
        NodeKind::Given { expr, body } => {
            walk_security_node(expr, diagnostics, signal_shadowed);
            walk_security_node(body, diagnostics, signal_shadowed);
            signal_shadowed
        }
        NodeKind::When { condition, body } => {
            walk_security_node(condition, diagnostics, signal_shadowed);
            walk_security_node(body, diagnostics, signal_shadowed);
            signal_shadowed
        }
        NodeKind::Default { body } => {
            walk_security_node(body, diagnostics, signal_shadowed);
            signal_shadowed
        }
        NodeKind::StatementModifier { statement, condition, .. } => {
            walk_security_node(statement, diagnostics, signal_shadowed);
            walk_security_node(condition, diagnostics, signal_shadowed);
            signal_shadowed
        }
        NodeKind::Subroutine { signature, body, .. } => {
            let mut sub_shadowed = signal_shadowed;
            if let Some(signature) = signature {
                sub_shadowed = walk_security_node(signature, diagnostics, sub_shadowed);
            }
            walk_security_node(body, diagnostics, sub_shadowed);
            signal_shadowed
        }
        NodeKind::Method { signature, body, .. } => {
            let mut method_shadowed = signal_shadowed;
            if let Some(signature) = signature {
                method_shadowed = walk_security_node(signature, diagnostics, method_shadowed);
            }
            walk_security_node(body, diagnostics, method_shadowed);
            signal_shadowed
        }
        NodeKind::Signature { parameters } => {
            let mut signature_shadowed = signal_shadowed;
            for parameter in parameters {
                signature_shadowed = walk_security_node(parameter, diagnostics, signature_shadowed);
            }
            signature_shadowed
        }
        NodeKind::MandatoryParameter { variable }
        | NodeKind::SlurpyParameter { variable }
        | NodeKind::NamedParameter { variable } => {
            let updated_shadowed =
                if shadows_signal_table(variable) { true } else { signal_shadowed };
            walk_security_node(variable, diagnostics, signal_shadowed);
            updated_shadowed
        }
        NodeKind::OptionalParameter { variable, default_value } => {
            walk_security_node(default_value, diagnostics, signal_shadowed);
            let updated_shadowed =
                if shadows_signal_table(variable) { true } else { signal_shadowed };
            walk_security_node(variable, diagnostics, signal_shadowed);
            updated_shadowed
        }
        NodeKind::Package { block: Some(block), .. }
        | NodeKind::PhaseBlock { block, .. }
        | NodeKind::Class { body: block, .. } => {
            walk_security_node(block, diagnostics, signal_shadowed);
            signal_shadowed
        }
        NodeKind::Try { body, catch_blocks, finally_block } => {
            walk_security_node(body, diagnostics, signal_shadowed);
            for (_, catch_body) in catch_blocks {
                walk_security_node(catch_body, diagnostics, signal_shadowed);
            }
            if let Some(finally_block) = finally_block {
                walk_security_node(finally_block, diagnostics, signal_shadowed);
            }
            signal_shadowed
        }
        NodeKind::Binary { left, right, .. } => {
            walk_security_node(left, diagnostics, signal_shadowed);
            walk_security_node(right, diagnostics, signal_shadowed);
            signal_shadowed
        }
        NodeKind::Ternary { condition, then_expr, else_expr } => {
            walk_security_node(condition, diagnostics, signal_shadowed);
            walk_security_node(then_expr, diagnostics, signal_shadowed);
            walk_security_node(else_expr, diagnostics, signal_shadowed);
            signal_shadowed
        }
        NodeKind::Unary { operand, .. } => {
            walk_security_node(operand, diagnostics, signal_shadowed);
            signal_shadowed
        }
        NodeKind::VariableWithAttributes { variable, .. } => {
            walk_security_node(variable, diagnostics, signal_shadowed);
            signal_shadowed
        }
        NodeKind::FunctionCall { name, args } => {
            check_two_arg_open(name, args, node, diagnostics);
            check_string_eval(name, args, node, diagnostics);
            for arg in args {
                walk_security_node(arg, diagnostics, signal_shadowed);
            }
            signal_shadowed
        }
        NodeKind::IndirectCall { object, args, .. } | NodeKind::MethodCall { object, args, .. } => {
            walk_security_node(object, diagnostics, signal_shadowed);
            for arg in args {
                walk_security_node(arg, diagnostics, signal_shadowed);
            }
            signal_shadowed
        }
        NodeKind::Eval { block } => {
            check_eval_node(block, node, diagnostics);
            walk_security_node(block, diagnostics, signal_shadowed);
            signal_shadowed
        }
        // Backtick strings: the parser stores `cmd` and qx(cmd) as
        // String { value: "`cmd`", interpolated: true }
        NodeKind::String { value, interpolated: true } if is_backtick_string(value) => {
            diagnostics.push(Diagnostic {
                range: (node.location.start, node.location.end),
                severity: DiagnosticSeverity::Information,
                code: Some(DiagnosticCode::SecurityBacktickExec.as_str().to_string()),
                message: "Command execution detected. Ensure input is sanitized.".to_string(),
                related_information: vec![RelatedInformation {
                    location: (node.location.start, node.location.end),
                    message: "Consider using open() with a pipe, or IPC::Run for safer command execution with proper input validation".to_string(),
                }],
                tags: Vec::new(),
                suggestion: Some(
                    "Use open(my $fh, '-|', @cmd) or IPC::Run for safer command execution"
                        .to_string(),
                ),
            });
            signal_shadowed
        }
        NodeKind::Return { value: Some(value) } => {
            walk_security_node(value, diagnostics, signal_shadowed);
            signal_shadowed
        }
        NodeKind::Return { value: None } => signal_shadowed,
        NodeKind::LabeledStatement { statement, .. } => {
            walk_security_node(statement, diagnostics, signal_shadowed);
            signal_shadowed
        }
        NodeKind::Error { partial: Some(partial), .. } => {
            walk_security_node(partial, diagnostics, signal_shadowed);
            signal_shadowed
        }
        NodeKind::Heredoc { .. }
        | NodeKind::Tie { .. }
        | NodeKind::Untie { .. }
        | NodeKind::Format { .. } => signal_shadowed,
        NodeKind::Package { block: None, .. }
        | NodeKind::Use { .. }
        | NodeKind::No { .. }
        | NodeKind::DataSection { .. }
        | NodeKind::Number { .. }
        | NodeKind::String { .. }
        | NodeKind::Regex { .. }
        | NodeKind::Match { .. }
        | NodeKind::Substitution { .. }
        | NodeKind::Transliteration { .. }
        | NodeKind::Identifier { .. }
        | NodeKind::Variable { .. }
        | NodeKind::Typeglob { .. }
        | NodeKind::Diamond
        | NodeKind::Ellipsis
        | NodeKind::Undef
        | NodeKind::Readline { .. }
        | NodeKind::Glob { .. }
        | NodeKind::ArrayLiteral { .. }
        | NodeKind::HashLiteral { .. }
        | NodeKind::Do { .. }
        | NodeKind::LoopControl { .. }
        | NodeKind::Goto { .. }
        | NodeKind::Prototype { .. }
        | NodeKind::MissingExpression
        | NodeKind::MissingStatement
        | NodeKind::MissingIdentifier
        | NodeKind::MissingBlock
        | NodeKind::Error { .. }
        | NodeKind::UnknownRest => signal_shadowed,
    }
}

/// Detect a global assignment to `$SIG{__DIE__}` or `$SIG{__WARN__}`.
fn check_global_signal_handler_assignment(
    lhs: &Node,
    node: &Node,
    diagnostics: &mut Vec<Diagnostic>,
    signal_shadowed: bool,
) {
    let Some(signal_handler) = signal_handler_name(lhs) else {
        return;
    };

    if signal_handler.access == SignalTableAccess::Bare && signal_shadowed {
        return;
    }

    diagnostics.push(Diagnostic {
        range: (node.location.start, node.location.end),
        severity: DiagnosticSeverity::Warning,
        code: Some(DiagnosticCode::SecuritySignalHandler.as_str().to_string()),
        message: format!(
            "Global assignment to {}{{{}}} changes process-wide behavior. Use local $SIG{{...}} to scope the handler.",
            signal_table_display(&signal_handler.access),
            signal_handler.signal_name
        ),
        related_information: vec![RelatedInformation {
            location: (node.location.start, node.location.end),
            message: "Localized signal handlers avoid leaking exception or warning hooks across unrelated code.".to_string(),
        }],
        tags: Vec::new(),
        suggestion: Some(format!(
            "Use `local $SIG{{{}}} = ...` if the handler should be scoped",
            signal_handler.signal_name
        )),
    });
}

fn signal_table_display(access: &SignalTableAccess) -> &'static str {
    match access {
        SignalTableAccess::Bare => "$SIG",
        SignalTableAccess::MainQualified => "$main::SIG",
    }
}

/// Extract the signal-handler key if the node targets `$SIG{__DIE__}` or `$SIG{__WARN__}`.
fn signal_handler_name(node: &Node) -> Option<SignalHandlerTarget> {
    let NodeKind::Binary { op, left, right } = &node.kind else {
        return None;
    };

    if op != "{}" {
        return None;
    }

    let access = match &left.kind {
        NodeKind::Variable { sigil, name } if (sigil == "$" || sigil == "%") && name == "SIG" => {
            SignalTableAccess::Bare
        }
        NodeKind::Variable { sigil, name }
            if (sigil == "$" || sigil == "%") && (name == "main::SIG" || name == "::SIG") =>
        {
            SignalTableAccess::MainQualified
        }
        _ => return None,
    };

    match &right.kind {
        NodeKind::Identifier { name } if name == "__DIE__" || name == "__WARN__" => {
            Some(SignalHandlerTarget { access, signal_name: name.to_string() })
        }
        NodeKind::String { value, .. } => {
            let trimmed = value.trim_matches(['"', '\'']);
            if trimmed == "__DIE__" || trimmed == "__WARN__" {
                Some(SignalHandlerTarget { access, signal_name: trimmed.to_string() })
            } else {
                None
            }
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

fn shadows_signal_table(node: &Node) -> bool {
    match &node.kind {
        NodeKind::Variable { sigil, name } => sigil == "%" && name == "SIG",
        NodeKind::VariableWithAttributes { variable, .. } => shadows_signal_table(variable),
        NodeKind::VariableDeclaration { declarator, variable, .. } => {
            matches!(declarator.as_str(), "my" | "state") && shadows_signal_table(variable)
        }
        NodeKind::VariableListDeclaration { declarator, variables, .. } => {
            matches!(declarator.as_str(), "my" | "state")
                && variables.iter().any(shadows_signal_table)
        }
        NodeKind::MandatoryParameter { variable }
        | NodeKind::SlurpyParameter { variable }
        | NodeKind::NamedParameter { variable } => shadows_signal_table(variable),
        NodeKind::OptionalParameter { variable, .. } => shadows_signal_table(variable),
        _ => false,
    }
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
    fn quoted_global_sig_warn_handler_is_flagged() {
        let diags = security_diags("$SIG{'__WARN__'} = sub { };");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL602")),
            "quoted __WARN__ handler should be flagged: {diags:?}"
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
    fn main_qualified_sig_warn_handler_is_flagged() {
        let diags = security_diags("$main::SIG{__WARN__} = sub { };");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL602")),
            "main-qualified __WARN__ handler should be flagged: {diags:?}"
        );
    }

    #[test]
    fn lexical_sig_shadow_is_not_flagged() {
        let diags = security_diags("my %SIG; $SIG{__WARN__} = sub { };");
        assert!(
            diags.iter().all(|d| d.code.as_deref() != Some("PL602")),
            "lexical %SIG shadow should not be flagged: {diags:?}"
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
