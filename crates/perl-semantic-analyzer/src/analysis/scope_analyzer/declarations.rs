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
    // `state` variables are implicitly initialized to `undef` on first call (Perl semantics).
    // A bare `state $x;` without an explicit initializer is NOT uninitialized — it is safe
    // to read (yields undef) and must not trigger UninitializedVariable diagnostics.
    let is_initialized = declarator == "state" || initializer.is_some();

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

    // For `our` variables, pre-compute the qualified name (e.g., "Foo::x") so we can
    // perform package-aware redeclaration checking independent of the scope tracker.
    let our_qualified =
        if is_our { analyzer.package_variable_name(var_name_part, context) } else { None };

    let issue_kind_opt = analyzer.declare_variable_parts_in_context(
        scope,
        sigil,
        var_name_part,
        variable.location.start,
        is_our,
        is_initialized,
        context,
    );

    match issue_kind_opt {
        Some(IssueKind::VariableRedeclaration) if is_our => {
            // `our` re-declares a package global. Whether this is an error depends on
            // the package context:
            //   - Same package visit (no `package X` statement in between) → error.
            //   - Different generation (package switched since last declaration) → silent
            //     re-import; update the recorded generation so subsequent re-declarations
            //     within this new visit are still detected.
            let emit_error = if let Some(qname) = &our_qualified {
                let current_gen = context.package_change_generation.get();
                // Extract before the match so the `Ref` borrow guard is dropped before
                // any potential `borrow_mut()` call in the match arms.
                let prev_gen_opt = context.our_decl_generations.borrow().get(qname).copied();
                match prev_gen_opt {
                    Some(prev_gen) if prev_gen == current_gen => {
                        // Same visit: genuine same-package redeclaration.
                        true
                    }
                    _ => {
                        // Different generation or first time in our_decl_generations:
                        // package switched between declarations → re-import, silently accept.
                        // Update the generation so further `our $x` in this visit are caught.
                        context
                            .our_decl_generations
                            .borrow_mut()
                            .insert(qname.clone(), current_gen);
                        false
                    }
                }
            } else {
                // Qualified name unavailable (e.g., name already contains "::").
                // Silently accept for backward compatibility.
                false
            };

            if emit_error {
                let line = context.get_line(variable.location.start);
                let full_name = extracted.as_string();
                let description =
                    format!("Variable '{}' is already declared in this scope", full_name);
                issues.push(ScopeIssue {
                    kind: IssueKind::VariableRedeclaration,
                    variable_name: full_name,
                    line,
                    range: (variable.location.start, variable.location.end),
                    description,
                });
            }
        }
        Some(issue_kind) => {
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
        None => {
            // Successful first declaration. Record the generation for `our` variables so
            // that a later redeclaration within the same package visit can be detected.
            if let Some(qname) = &our_qualified {
                let pkg_gen = context.package_change_generation.get();
                context.our_decl_generations.borrow_mut().entry(qname.clone()).or_insert(pkg_gen);
            }
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
    // `state` variables are implicitly initialized to `undef` on first call (Perl semantics).
    // A bare `state ($x, $y);` list without an explicit initializer is NOT uninitialized.
    let is_initialized = declarator == "state" || initializer.is_some();

    // Analyze initializer first
    if let Some(init) = initializer {
        analyzer.analyze_node(init, scope, ancestors, issues, context);
    }

    for variable in variables {
        let extracted = analyzer.extract_variable_name(variable);
        let (sigil, var_name_part) = extracted.parts();

        // For `our` list elements, compute the qualified name for package-aware checking.
        let our_qualified =
            if is_our { analyzer.package_variable_name(var_name_part, context) } else { None };

        let issue_kind_opt = analyzer.declare_variable_parts_in_context(
            scope,
            sigil,
            var_name_part,
            variable.location.start,
            is_our,
            is_initialized,
            context,
        );

        match issue_kind_opt {
            Some(IssueKind::VariableRedeclaration) if is_our => {
                // Apply the same package-context check as handle_variable_declaration.
                let emit_error = if let Some(qname) = &our_qualified {
                    let current_gen = context.package_change_generation.get();
                    // Extract before the match so the `Ref` borrow guard is dropped before
                    // any potential `borrow_mut()` call in the match arms.
                    let prev_gen_opt = context.our_decl_generations.borrow().get(qname).copied();
                    match prev_gen_opt {
                        Some(prev_gen) if prev_gen == current_gen => true,
                        _ => {
                            context
                                .our_decl_generations
                                .borrow_mut()
                                .insert(qname.clone(), current_gen);
                            false
                        }
                    }
                } else {
                    false
                };

                if emit_error {
                    let line = context.get_line(variable.location.start);
                    let full_name = extracted.as_string();
                    let description =
                        format!("Variable '{}' is already declared in this scope", full_name);
                    issues.push(ScopeIssue {
                        kind: IssueKind::VariableRedeclaration,
                        variable_name: full_name,
                        line,
                        range: (variable.location.start, variable.location.end),
                        description,
                    });
                }
            }
            Some(issue_kind) => {
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
            None => {
                // First declaration: record generation for `our` variables.
                if let Some(qname) = &our_qualified {
                    let pkg_gen = context.package_change_generation.get();
                    context
                        .our_decl_generations
                        .borrow_mut()
                        .entry(qname.clone())
                        .or_insert(pkg_gen);
                }
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
