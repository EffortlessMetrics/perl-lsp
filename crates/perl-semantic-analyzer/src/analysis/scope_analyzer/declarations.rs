//! Handlers for declaration node kinds in scope analysis.

use super::{
    AnalysisContext, IssueKind, Scope, ScopeAnalyzer, ScopeIssue, is_builtin_global,
    split_variable_name,
};
use crate::ast::{Node, NodeKind};
use std::rc::Rc;

/// Handle `NodeKind::VariableDeclaration`.
///
/// Returns `true` if the caller should return early (builtin-global local skipped).
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_variable_declaration<'a>(
    analyzer: &ScopeAnalyzer,
    _node: &'a Node,
    declarator: &str,
    variable: &'a Node,
    initializer: Option<&'a Node>,
    scope: &Rc<Scope>,
    ancestors: &mut Vec<&'a Node>,
    issues: &mut Vec<ScopeIssue>,
    context: &AnalysisContext<'a>,
) -> bool {
    let extracted = analyzer.extract_variable_name(variable);
    let (sigil, var_name_part) = extracted.parts();

    let is_our = declarator == "our";
    let is_initialized = initializer.is_some();

    // `local` of a builtin special variable (e.g. `local $/`, `local $,`) temporarily
    // modifies the global; it does not create a new lexical binding.  Declaring it in
    // the lexical scope would cause a spurious UnusedVariable diagnostic because all
    // later uses of `$/` etc. are recognised by is_builtin_global and never counted as
    // uses of the scope entry.  Skip the declaration entirely and only analyse any
    // initialiser expression that may be present.
    if declarator == "local" && is_builtin_global(sigil, var_name_part) {
        // For `local $special = expr`, the parser embeds the assignment inside
        // `variable` as an Assignment node rather than in `initializer`.  Walk the
        // variable node's children to pick up any RHS expressions.
        if let Some(init) = initializer {
            analyzer.analyze_node(init, scope, ancestors, issues, context);
        }
        if let NodeKind::Assignment { rhs, .. } = &variable.kind {
            analyzer.analyze_node(rhs, scope, ancestors, issues, context);
        }
        return true;
    }

    // If checking initializer first (e.g. my $x = $x), we need to analyze initializer in
    // current scope BEFORE declaring the variable (standard Perl behavior)
    // Actually Perl evaluates RHS before LHS assignment, so usages in initializer refer to OUTER scope.
    // So we analyze initializer first.
    if let Some(init) = initializer {
        analyzer.analyze_node(init, scope, ancestors, issues, context);
    }

    if let Some(issue_kind) = analyzer.declare_variable_parts_in_context(
        scope,
        sigil,
        var_name_part,
        variable.location.start,
        is_our,
        is_initialized,
        context,
    ) {
        // `our` re-declares a package global — valid Perl idiom when switching
        // packages (`package Foo; our $x; package Bar; our $x;`).  Never report
        // VariableRedeclaration for `our` declarations.
        if is_our && issue_kind == IssueKind::VariableRedeclaration {
            // Silently accept: different-package re-use of the same bare name.
        } else {
            let line = context.get_line(variable.location.start);
            // Optimization: Only allocate full name string when we actually have an issue to report
            let full_name = extracted.as_string();
            // Build description first (borrows full_name), then move full_name into struct
            let description = match issue_kind {
                IssueKind::VariableShadowing => {
                    format!("Variable '{}' shadows a variable in outer scope", full_name)
                }
                IssueKind::VariableRedeclaration => {
                    format!("Variable '{}' is already declared in this scope", full_name)
                }
                _ => String::new(),
            };
            issues.push(ScopeIssue {
                kind: issue_kind,
                variable_name: full_name,
                line,
                range: (variable.location.start, variable.location.end),
                description,
            });
        }
    }
    false
}

/// Handle `NodeKind::VariableListDeclaration`.
pub(super) fn handle_variable_list_declaration<'a>(
    analyzer: &ScopeAnalyzer,
    initializer: Option<&'a Node>,
    declarator: &str,
    variables: &'a [Node],
    scope: &Rc<Scope>,
    ancestors: &mut Vec<&'a Node>,
    issues: &mut Vec<ScopeIssue>,
    context: &AnalysisContext<'a>,
) {
    let is_our = declarator == "our";
    let is_initialized = initializer.is_some();

    // Analyze initializer first
    if let Some(init) = initializer {
        analyzer.analyze_node(init, scope, ancestors, issues, context);
    }

    for variable in variables {
        let extracted = analyzer.extract_variable_name(variable);
        let (sigil, var_name_part) = extracted.parts();

        if let Some(issue_kind) = analyzer.declare_variable_parts_in_context(
            scope,
            sigil,
            var_name_part,
            variable.location.start,
            is_our,
            is_initialized,
            context,
        ) {
            // `our` redeclaration is always valid — see VariableDeclaration handler.
            if is_our && issue_kind == IssueKind::VariableRedeclaration {
                // Silently accept.
            } else {
                let line = context.get_line(variable.location.start);
                // Optimization: Only allocate full name string when we actually have an issue to report
                let full_name = extracted.as_string();
                // Build description first (borrows full_name), then move full_name into struct
                let description = match issue_kind {
                    IssueKind::VariableShadowing => {
                        format!("Variable '{}' shadows a variable in outer scope", full_name)
                    }
                    IssueKind::VariableRedeclaration => {
                        format!("Variable '{}' is already declared in this scope", full_name)
                    }
                    _ => String::new(),
                };
                issues.push(ScopeIssue {
                    kind: issue_kind,
                    variable_name: full_name,
                    line,
                    range: (variable.location.start, variable.location.end),
                    description,
                });
            }
        }
    }
}

/// Handle `NodeKind::Use` — register `use vars` variable declarations.
pub(super) fn handle_use(
    analyzer: &ScopeAnalyzer,
    node: &Node,
    module: &str,
    args: &[String],
    scope: &Rc<Scope>,
    context: &AnalysisContext<'_>,
) {
    // Handle 'use vars' pragma for global variable declarations
    if module == "vars" {
        for arg in args {
            // Parse qw() style arguments to extract individual variable names
            if arg.starts_with("qw(") && arg.ends_with(")") {
                let content = &arg[3..arg.len() - 1]; // Remove qw( and )
                for var_name in content.split_whitespace() {
                    if !var_name.is_empty() {
                        let (sigil, name) = split_variable_name(var_name);
                        if !sigil.is_empty() {
                            // Declare these variables as globals in the current scope
                            analyzer.declare_variable_parts_in_context(
                                scope,
                                sigil,
                                name,
                                node.location.start,
                                true,
                                true,
                                context,
                            ); // true = is_our (global), true = initialized (assumed)
                        }
                    }
                }
            } else {
                // Handle regular variable names (not in qw())
                let var_name = arg.trim();
                if !var_name.is_empty() {
                    let (sigil, name) = split_variable_name(var_name);
                    if !sigil.is_empty() {
                        analyzer.declare_variable_parts_in_context(
                            scope,
                            sigil,
                            name,
                            node.location.start,
                            true,
                            true,
                            context,
                        );
                    }
                }
            }
        }
    }
}
