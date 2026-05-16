//! Handlers for string/regex interpolation node kinds in scope analysis.

use super::{AnalysisContext, Scope, ScopeAnalyzer, ScopeIssue};
use crate::ast::Node;
use std::rc::Rc;

/// Handle `NodeKind::String` — mark interpolated variables as used.
pub(super) fn handle_string(
    analyzer: &ScopeAnalyzer,
    value: &str,
    interpolated: bool,
    scope: &Rc<Scope>,
    context: &AnalysisContext<'_>,
) {
    if interpolated
        || value.starts_with('"')
        || value.starts_with('`')
        || value.starts_with("qq")
        || value.starts_with("qx")
    {
        analyzer.mark_interpolated_variables_used(value, scope, context);
    }
}

/// Handle `NodeKind::Heredoc` — mark interpolated variables as used.
pub(super) fn handle_heredoc(
    analyzer: &ScopeAnalyzer,
    content: &str,
    interpolated: bool,
    scope: &Rc<Scope>,
    context: &AnalysisContext<'_>,
) {
    if interpolated {
        analyzer.mark_interpolated_variables_used(content, scope, context);
    }
}

/// Handle `NodeKind::Match` — flag scope as having seen a regex match, recurse into expr.
pub(super) fn handle_match<'a>(
    analyzer: &ScopeAnalyzer,
    node: &'a Node,
    expr: &'a Node,
    scope: &Rc<Scope>,
    ancestors: &mut Vec<&'a Node>,
    issues: &mut Vec<ScopeIssue>,
    context: &AnalysisContext<'a>,
) {
    scope.has_regex_match.set(true);
    ancestors.push(node);
    analyzer.analyze_node(expr, scope, ancestors, issues, context);
    ancestors.pop();
}

/// Handle `NodeKind::Substitution` — flag scope as having seen a regex match, recurse into expr.
pub(super) fn handle_substitution<'a>(
    analyzer: &ScopeAnalyzer,
    node: &'a Node,
    expr: &'a Node,
    scope: &Rc<Scope>,
    ancestors: &mut Vec<&'a Node>,
    issues: &mut Vec<ScopeIssue>,
    context: &AnalysisContext<'a>,
) {
    scope.has_regex_match.set(true);
    ancestors.push(node);
    analyzer.analyze_node(expr, scope, ancestors, issues, context);
    ancestors.pop();
}

/// Handle `NodeKind::Regex` — flag scope as having seen a standalone regex match.
pub(super) fn handle_regex(scope: &Rc<Scope>) {
    scope.has_regex_match.set(true);
}
