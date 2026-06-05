use perl_parser_core::ast::{Node, NodeKind};
use std::collections::HashSet;

/// Collect the set of module names that the buffer's AST `use`s,
/// regardless of whether each `use` has an explicit symbol list.
///
/// This does NOT track which symbols were imported; it only records *which
/// packages* are referenced by `use` statements. Used by the bounded
/// Unknown-receiver method-completion fallback (#7929) to know which workspace
/// packages are visible from the current file.
///
/// `use Foo;`, `use Foo ();`, `use Foo qw(bar);`, and `use Foo BAR;` all add
/// `Foo` to the returned set. Pragma-style lowercase modules (e.g. `use
/// strict;`) are excluded.
pub(in crate::providers::completion::completion) fn collect_used_module_names(
    ast: &Node,
) -> HashSet<String> {
    let mut modules: HashSet<String> = HashSet::new();
    walk(ast, &mut modules);
    modules
}

fn walk(node: &Node, modules: &mut HashSet<String>) {
    match &node.kind {
        NodeKind::Use { module, .. } => {
            if is_importable_module(module) {
                modules.insert(module.clone());
            }
        }
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            for stmt in statements {
                walk(stmt, modules);
            }
        }
        NodeKind::Package { block: Some(block), .. } => {
            walk(block, modules);
        }
        _ => {}
    }
}

pub(super) fn is_importable_module(module: &str) -> bool {
    module.chars().next().is_some_and(|c: char| c.is_ascii_uppercase())
}
