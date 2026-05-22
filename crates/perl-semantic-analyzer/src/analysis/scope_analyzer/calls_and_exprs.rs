//! Handlers for call-shaped and structural recursion node kinds in scope analysis.

use super::{
    AnalysisContext, Scope, ScopeAnalyzer, ScopeIssue, builtin_declaration_arg_positions,
    is_topic_defaulting_builtin, is_topic_modifying_builtin,
};
use crate::ast::Node;
use std::rc::Rc;

/// Handle `NodeKind::FunctionCall`.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_function_call<'a>(
    analyzer: &ScopeAnalyzer,
    node: &'a Node,
    name: &str,
    args: &'a [Node],
    scope: &Rc<Scope>,
    ancestors: &mut Vec<&'a Node>,
    issues: &mut Vec<ScopeIssue>,
    context: &AnalysisContext<'a>,
    strict_vars_mode: bool,
) {
    if let Some((sigil, var_name)) = analyzer.extract_name_like_variable(name) {
        analyzer.record_variable_use(
            scope,
            strict_vars_mode,
            context,
            issues,
            node,
            sigil,
            var_name,
        );
    }

    // Builtins that default to $_ when called with zero arguments implicitly
    // read (and in some cases modify) $_. Mark it as used so that any lexically-
    // scoped `my $_` in scope is not reported as unused or uninitialized.
    if args.is_empty() && is_topic_defaulting_builtin(name) {
        if is_topic_modifying_builtin(name) {
            let _ = scope.initialize_and_use_variable_parts("$", "_");
        } else {
            let _ = scope.use_variable_parts("$", "_");
        }
    }
    ancestors.push(node);
    let declaration_arg_positions = builtin_declaration_arg_positions(name);
    for (arg_index, arg) in args.iter().enumerate() {
        analyzer.analyze_node(arg, scope, ancestors, issues, context);
        if declaration_arg_positions.contains(&arg_index) {
            analyzer.mark_builtin_declaration_arg_consumed(arg, scope, context);
        }
    }
    ancestors.pop();
}

/// Handle `NodeKind::MethodCall`.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_method_call<'a>(
    analyzer: &ScopeAnalyzer,
    node: &'a Node,
    object: &'a Node,
    method: &str,
    args: &'a [Node],
    scope: &Rc<Scope>,
    ancestors: &mut Vec<&'a Node>,
    issues: &mut Vec<ScopeIssue>,
    context: &AnalysisContext<'a>,
    strict_vars_mode: bool,
) {
    ancestors.push(node);
    analyzer.analyze_node(object, scope, ancestors, issues, context);
    if let Some((sigil, var_name)) = analyzer.extract_method_name_variable(method) {
        analyzer.record_variable_use(
            scope,
            strict_vars_mode,
            context,
            issues,
            node,
            sigil,
            var_name,
        );
    }
    for arg in args {
        analyzer.analyze_node(arg, scope, ancestors, issues, context);
    }
    ancestors.pop();
}

/// Handle `NodeKind::Unary`.
pub(super) fn handle_unary<'a>(
    analyzer: &ScopeAnalyzer,
    node: &'a Node,
    operand: &'a Node,
    scope: &Rc<Scope>,
    ancestors: &mut Vec<&'a Node>,
    issues: &mut Vec<ScopeIssue>,
    context: &AnalysisContext<'a>,
) {
    ancestors.push(node);
    analyzer.analyze_node(operand, scope, ancestors, issues, context);
    ancestors.pop();
}

/// Handle `NodeKind::Binary`.
pub(super) fn handle_binary<'a>(
    analyzer: &ScopeAnalyzer,
    node: &'a Node,
    left: &'a Node,
    right: &'a Node,
    scope: &Rc<Scope>,
    ancestors: &mut Vec<&'a Node>,
    issues: &mut Vec<ScopeIssue>,
    context: &AnalysisContext<'a>,
) {
    ancestors.push(node);
    analyzer.analyze_node(left, scope, ancestors, issues, context);
    analyzer.analyze_node(right, scope, ancestors, issues, context);
    ancestors.pop();
}

/// Handle `NodeKind::ArrayLiteral`.
pub(super) fn handle_array_literal<'a>(
    analyzer: &ScopeAnalyzer,
    node: &'a Node,
    elements: &'a [Node],
    scope: &Rc<Scope>,
    ancestors: &mut Vec<&'a Node>,
    issues: &mut Vec<ScopeIssue>,
    context: &AnalysisContext<'a>,
) {
    ancestors.push(node);
    for element in elements {
        analyzer.analyze_node(element, scope, ancestors, issues, context);
    }
    ancestors.pop();
}
