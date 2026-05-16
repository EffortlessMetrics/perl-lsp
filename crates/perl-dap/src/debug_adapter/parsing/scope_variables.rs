//! SRP helpers for `parse_scope_variables_from_lines`.
//!
//! Each function owns exactly one responsibility:
//! - [`parse_assignments`] — iterate, normalize, filter, dedupe, cap
//! - [`sort_and_paginate`] — reverse chronological order, sort, slice
//! - [`compute_child_reference`] — stable child-ref arithmetic
//! - [`render_paged_variable`] — render one variable and its optional children

use super::super::*;
use crate::value::PerlValue;

/// Iterate `lines` in reverse, parse variable assignments, apply scope filter,
/// deduplicate by name, and cap at 256 entries.
///
/// Returns a vec built in reverse-iteration order (most-recent-first).
/// [`sort_and_paginate`] reverses it before sorting to restore chronological order.
pub(super) fn parse_assignments(lines: &[String], scope_type: i32) -> Vec<(String, PerlValue)> {
    let parser = VariableParser::new();
    let mut seen = HashSet::new();
    let mut parsed = Vec::new();

    for line in lines.iter().rev() {
        let normalized = DebugAdapter::normalize_debugger_output_line(line);
        let text = normalized.trim();
        if text.is_empty() {
            continue;
        }
        if let Ok((name, value)) = parser.parse_assignment(text) {
            if !DebugAdapter::scope_allows_variable_name(scope_type, &name) {
                continue;
            }
            if seen.insert(name.clone()) {
                parsed.push((name, value));
            }
            if parsed.len() >= 256 {
                break;
            }
        }
    }

    parsed
}

/// Reverse the vec (restoring chronological order), sort by name, then
/// apply `skip(start).take(count)` pagination.
pub(super) fn sort_and_paginate(
    mut parsed: Vec<(String, PerlValue)>,
    start: usize,
    count: usize,
) -> Vec<(String, PerlValue)> {
    parsed.reverse();
    parsed.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
    parsed.into_iter().skip(start).take(count).collect()
}

/// Compute a stable child reference integer for a paged variable entry.
///
/// `variables_ref` is the scope reference; `start` + `idx` give the absolute
/// position (1-based).  Arithmetic uses saturating operations throughout.
pub(super) fn compute_child_reference(variables_ref: i32, start: usize, idx: usize) -> i32 {
    let absolute_index = start.saturating_add(idx).saturating_add(1);
    variables_ref.saturating_mul(1000).saturating_add(DebugAdapter::i64_to_i32_saturating(
        i64::try_from(absolute_index).unwrap_or(i64::from(i32::MAX)),
    ))
}

/// Render a single variable and, if expandable, its children.
///
/// Returns `(top_level_variable, Some((child_ref, children)))` when the value
/// is expandable and has at least one child; otherwise `None` for the second
/// tuple element.
pub(super) fn render_paged_variable(
    name: String,
    value: PerlValue,
    child_ref: i32,
) -> (Variable, Option<(i32, Vec<Variable>)>) {
    let renderer = PerlVariableRenderer::new();

    let rendered = if value.is_expandable() {
        renderer.render_with_reference(&name, &value, i64::from(child_ref))
    } else {
        renderer.render(&name, &value)
    };
    let top = DebugAdapter::rendered_to_variable(rendered);

    let cache_entry = if value.is_expandable() {
        let children = renderer
            .render_children(&value, 0, 256)
            .into_iter()
            .map(DebugAdapter::rendered_to_variable)
            .collect::<Vec<_>>();
        if children.is_empty() { None } else { Some((child_ref, children)) }
    } else {
        None
    };

    (top, cache_entry)
}
