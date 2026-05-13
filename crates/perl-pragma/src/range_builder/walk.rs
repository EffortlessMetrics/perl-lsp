use super::directives::{apply_no_directive, apply_use_directive};
use crate::PragmaState;
use perl_ast::ast::{Node, NodeKind};
use std::ops::Range;

pub(crate) fn build_ranges(
    node: &Node,
    current_state: &mut PragmaState,
    ranges: &mut Vec<(Range<usize>, PragmaState)>,
) {
    match &node.kind {
        NodeKind::Use { module, args, .. } => {
            apply_use_directive(
                node.location.start..node.location.end,
                module,
                args,
                current_state,
                ranges,
            );
        }
        NodeKind::No { module, args, .. } => {
            apply_no_directive(
                node.location.start..node.location.end,
                module,
                args,
                current_state,
                ranges,
            );
        }
        NodeKind::Block { statements } => {
            build_statement_block(statements, node.location.end, current_state, ranges);
        }
        NodeKind::Program { statements } => {
            for stmt in statements {
                build_ranges(stmt, current_state, ranges);
            }
        }
        NodeKind::Subroutine { body, .. }
        | NodeKind::Method { body, .. }
        | NodeKind::Class { body, .. } => {
            build_scoped_body(body, current_state, ranges);
        }
        NodeKind::If { then_branch, elsif_branches, else_branch, .. } => {
            build_scoped_body(then_branch, current_state, ranges);
            for (_, elsif_body) in elsif_branches {
                build_scoped_body(elsif_body, current_state, ranges);
            }
            if let Some(else_b) = else_branch {
                build_scoped_body(else_b, current_state, ranges);
            }
        }
        NodeKind::While { body, continue_block, .. }
        | NodeKind::For { body, continue_block, .. }
        | NodeKind::Foreach { body, continue_block, .. } => {
            build_scoped_body(body, current_state, ranges);
            if let Some(continue_block) = continue_block {
                build_scoped_body(continue_block, current_state, ranges);
            }
        }
        NodeKind::Eval { block } => {
            if matches!(block.kind, NodeKind::Block { .. }) {
                build_scoped_body(block, current_state, ranges);
            }
        }
        NodeKind::Do { block } | NodeKind::Defer { block } | NodeKind::PhaseBlock { block, .. } => {
            build_scoped_body(block, current_state, ranges);
        }
        NodeKind::Given { body, .. } | NodeKind::When { body, .. } | NodeKind::Default { body } => {
            build_scoped_body(body, current_state, ranges);
        }
        NodeKind::Try { body, catch_blocks, finally_block } => {
            build_scoped_body(body, current_state, ranges);
            for (_, catch_body) in catch_blocks {
                build_scoped_body(catch_body, current_state, ranges);
            }
            if let Some(finally_block) = finally_block {
                build_scoped_body(finally_block, current_state, ranges);
            }
        }
        NodeKind::LabeledStatement { statement, .. } => {
            build_ranges(statement, current_state, ranges);
        }
        NodeKind::StatementModifier { statement, condition, .. } => {
            build_ranges(statement, current_state, ranges);
            build_ranges(condition, current_state, ranges);
        }
        NodeKind::Package { block: Some(pkg_block), .. } => {
            build_scoped_body(pkg_block, current_state, ranges);
        }
        _ => {}
    }
}

fn build_scoped_body(
    body: &Node,
    current_state: &mut PragmaState,
    ranges: &mut Vec<(Range<usize>, PragmaState)>,
) {
    let saved_state = current_state.clone();
    build_ranges(body, current_state, ranges);
    *current_state = saved_state;
    ranges.push((body.location.end..body.location.end, current_state.clone()));
}

fn build_statement_block(
    statements: &[Node],
    end: usize,
    current_state: &mut PragmaState,
    ranges: &mut Vec<(Range<usize>, PragmaState)>,
) {
    let saved_state = current_state.clone();
    for stmt in statements {
        build_ranges(stmt, current_state, ranges);
    }
    *current_state = saved_state;
    ranges.push((end..end, current_state.clone()));
}
