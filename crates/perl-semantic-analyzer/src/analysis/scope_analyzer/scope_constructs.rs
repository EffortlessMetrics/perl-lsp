//! Handlers for nodes that open new scopes or have structural recursion semantics.

use super::{
    AnalysisContext, IssueKind, Scope, ScopeAnalyzer, ScopeIssue, sigil_to_index,
    split_variable_name,
};
use crate::ast::{Node, NodeKind};
use std::collections::HashSet;
use std::rc::Rc;

/// Handle `NodeKind::Block`.
pub(super) fn handle_block<'a>(
    analyzer: &ScopeAnalyzer,
    node: &'a Node,
    statements: &'a [Node],
    scope: &Rc<Scope>,
    ancestors: &mut Vec<&'a Node>,
    issues: &mut Vec<ScopeIssue>,
    context: &AnalysisContext<'a>,
) {
    let block_scope = Rc::new(Scope::with_parent(scope.clone()));
    ancestors.push(node);
    for stmt in statements {
        analyzer.analyze_node(stmt, &block_scope, ancestors, issues, context);
    }
    ancestors.pop();
    analyzer.collect_unused_variables(&block_scope, issues, context);
}

/// Handle `NodeKind::PhaseBlock`.
pub(super) fn handle_phase_block<'a>(
    analyzer: &ScopeAnalyzer,
    node: &'a Node,
    block: &'a Node,
    scope: &Rc<Scope>,
    ancestors: &mut Vec<&'a Node>,
    issues: &mut Vec<ScopeIssue>,
    context: &AnalysisContext<'a>,
) {
    let phase_scope = Rc::new(Scope::with_parent(scope.clone()));
    ancestors.push(node);
    analyzer.analyze_node(block, &phase_scope, ancestors, issues, context);
    ancestors.pop();
    analyzer.collect_unused_variables(&phase_scope, issues, context);
}

/// Handle `NodeKind::For`.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_for<'a>(
    analyzer: &ScopeAnalyzer,
    node: &'a Node,
    init: Option<&'a Node>,
    condition: Option<&'a Node>,
    update: Option<&'a Node>,
    body: &'a Node,
    scope: &Rc<Scope>,
    ancestors: &mut Vec<&'a Node>,
    issues: &mut Vec<ScopeIssue>,
    context: &AnalysisContext<'a>,
) {
    let loop_scope = Rc::new(Scope::with_parent(scope.clone()));

    ancestors.push(node);

    if let Some(init_node) = init {
        analyzer.analyze_node(init_node, &loop_scope, ancestors, issues, context);
    }
    if let Some(cond) = condition {
        analyzer.analyze_node(cond, &loop_scope, ancestors, issues, context);
    }
    if let Some(upd) = update {
        analyzer.analyze_node(upd, &loop_scope, ancestors, issues, context);
    }
    analyzer.analyze_node(body, &loop_scope, ancestors, issues, context);

    ancestors.pop();

    analyzer.collect_unused_variables(&loop_scope, issues, context);
}

/// Handle `NodeKind::Foreach`.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_foreach<'a>(
    analyzer: &ScopeAnalyzer,
    node: &'a Node,
    variable: &'a Node,
    list: &'a Node,
    body: &'a Node,
    continue_block: Option<&'a Node>,
    scope: &Rc<Scope>,
    ancestors: &mut Vec<&'a Node>,
    issues: &mut Vec<ScopeIssue>,
    context: &AnalysisContext<'a>,
) {
    let loop_scope = Rc::new(Scope::with_parent(scope.clone()));

    ancestors.push(node);

    // Declare the loop variable and immediately mark it initialized — the list
    // provides its value at runtime so there is no uninitialized window.
    analyzer.analyze_node(variable, &loop_scope, ancestors, issues, context);
    analyzer.mark_initialized(variable, &loop_scope, context);
    analyzer.analyze_node(list, &loop_scope, ancestors, issues, context);
    analyzer.analyze_node(body, &loop_scope, ancestors, issues, context);
    if let Some(cb) = continue_block {
        analyzer.analyze_node(cb, &loop_scope, ancestors, issues, context);
    }

    ancestors.pop();

    analyzer.collect_unused_variables(&loop_scope, issues, context);
}

/// Handle `NodeKind::Subroutine`.
pub(super) fn handle_subroutine<'a>(
    analyzer: &ScopeAnalyzer,
    node: &'a Node,
    signature: Option<&'a Node>,
    body: &'a Node,
    scope: &Rc<Scope>,
    ancestors: &mut Vec<&'a Node>,
    issues: &mut Vec<ScopeIssue>,
    context: &AnalysisContext<'a>,
) {
    let sub_scope = Rc::new(Scope::with_parent(scope.clone()));

    // Check for duplicate parameters and shadowing
    let mut param_names = HashSet::new();

    // Extract parameters from signature if present
    // Optimization: Use slice to avoid cloning the parameters vector (deep copy of AST nodes)
    let params_to_check: &[Node] = if let Some(sig) = signature {
        match &sig.kind {
            NodeKind::Signature { parameters } => parameters.as_slice(),
            _ => &[],
        }
    } else {
        &[]
    };

    for param in params_to_check {
        let extracted = analyzer.extract_variable_name(param);
        if !extracted.is_empty() {
            let full_name = extracted.as_string();
            let (sigil, name) = extracted.parts();

            // Check for duplicate parameters
            if !param_names.insert(full_name.clone()) {
                issues.push(ScopeIssue {
                    kind: IssueKind::DuplicateParameter,
                    variable_name: full_name.clone(),
                    line: context.get_line(param.location.start),
                    range: (param.location.start, param.location.end),
                    description: format!(
                        "Duplicate parameter '{}' in subroutine signature",
                        full_name
                    ),
                });
            }

            // Check if parameter shadows a global or parent scope variable
            if analyzer.has_variable_parts_in_context(scope, sigil, name, context) {
                issues.push(ScopeIssue {
                    kind: IssueKind::ParameterShadowsGlobal,
                    variable_name: full_name.clone(),
                    line: context.get_line(param.location.start),
                    range: (param.location.start, param.location.end),
                    description: format!(
                        "Parameter '{}' shadows a variable from outer scope",
                        full_name
                    ),
                });
            }

            // Declare the parameter in subroutine scope
            analyzer.declare_variable_parts_in_context(
                &sub_scope,
                sigil,
                name,
                param.location.start,
                false,
                true,
                context,
            ); // Parameters are initialized
            // Don't mark parameters as automatically used yet - track their actual usage
        }
    }

    ancestors.push(node);
    analyzer.analyze_node(body, &sub_scope, ancestors, issues, context);
    ancestors.pop();

    // Check for unused parameters
    if let Some(sig) = signature {
        if let NodeKind::Signature { parameters } = &sig.kind {
            for param in parameters {
                let extracted = analyzer.extract_variable_name(param);
                if !extracted.is_empty() {
                    let (sigil, name) = extracted.parts();
                    let full_name = extracted.as_string();

                    // Skip parameters starting with underscore (intentionally unused)
                    if name.starts_with('_') {
                        continue;
                    }

                    // Optimization: Access variable directly from current scope to avoid Rc clone
                    let idx = sigil_to_index(sigil);
                    let vars = sub_scope.variables.borrow();
                    if let Some(map) = vars[idx].as_ref() {
                        if let Some(var) = map.get(name) {
                            if !*var.is_used.borrow() {
                                issues.push(ScopeIssue {
                                    kind: IssueKind::UnusedParameter,
                                    variable_name: full_name.clone(),
                                    line: context.get_line(param.location.start),
                                    range: (param.location.start, param.location.end),
                                    description: format!(
                                        "Parameter '{}' is declared but never used",
                                        full_name
                                    ),
                                });
                                // Mark as used to prevent double reporting
                                *var.is_used.borrow_mut() = true;
                            }
                        }
                    }
                }
            }
        }
    }

    analyzer.collect_unused_variables(&sub_scope, issues, context);
}

/// Handle `NodeKind::Try`.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_try<'a>(
    analyzer: &ScopeAnalyzer,
    node: &'a Node,
    body: &'a Node,
    catch_blocks: &'a [(Option<String>, Box<Node>)],
    finally_block: Option<&'a Node>,
    scope: &Rc<Scope>,
    ancestors: &mut Vec<&'a Node>,
    issues: &mut Vec<ScopeIssue>,
    context: &AnalysisContext<'a>,
) {
    ancestors.push(node);
    analyzer.analyze_node(body, scope, ancestors, issues, context);

    for (catch_var, catch_body) in catch_blocks {
        let catch_scope = Rc::new(Scope::with_parent(scope.clone()));

        if let Some(full_name) = catch_var.as_deref() {
            let catch_var_range = context
                .find_catch_variable_range(catch_body.location.start, full_name)
                .unwrap_or((catch_body.location.start, catch_body.location.start));
            let (sigil, name) = split_variable_name(full_name);
            if !sigil.is_empty() && !name.is_empty() && !name.contains("::") {
                if let Some(issue_kind) =
                    catch_scope.declare_variable_parts(sigil, name, catch_var_range.0, false, true)
                {
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
                        variable_name: full_name.to_string(),
                        line: context.get_line(catch_var_range.0),
                        range: catch_var_range,
                        description,
                    });
                }
            }
        }

        analyzer.analyze_block_with_scope(catch_body, &catch_scope, ancestors, issues, context);
        analyzer.collect_unused_variables(&catch_scope, issues, context);
    }

    if let Some(finally) = finally_block {
        analyzer.analyze_node(finally, scope, ancestors, issues, context);
    }

    ancestors.pop();
}

/// Handle `NodeKind::Package`.
pub(super) fn handle_package<'a>(
    analyzer: &ScopeAnalyzer,
    node: &'a Node,
    name: &str,
    block: Option<&'a Node>,
    scope: &Rc<Scope>,
    ancestors: &mut Vec<&'a Node>,
    issues: &mut Vec<ScopeIssue>,
    context: &AnalysisContext<'a>,
) {
    // Track the active package so that `our` variable declarations can be
    // correctly namespaced.  Two packages that each declare `our $VAR` are
    // declaring *different* package-global variables (`Alpha::VAR` vs
    // `Beta::VAR`) and must not be reported as redeclarations.
    if let Some(block_node) = block {
        // Block form: `package Foo { ... }` — scope is limited to the block.
        // Save the previous package name and restore it after the block.
        let saved_package = context.current_package.borrow().clone();
        *context.current_package.borrow_mut() = name.to_string();

        let pkg_scope = Rc::new(Scope::with_parent(scope.clone()));
        ancestors.push(node);
        analyzer.analyze_node(block_node, &pkg_scope, ancestors, issues, context);
        ancestors.pop();
        analyzer.collect_unused_variables(&pkg_scope, issues, context);

        *context.current_package.borrow_mut() = saved_package;
    } else {
        // Statement form: `package Foo;` — affects the rest of the file.
        // No scope boundary is created; the current scope continues.
        *context.current_package.borrow_mut() = name.to_string();
    }
}
