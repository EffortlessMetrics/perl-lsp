//! Test2 (and Test::More) subtest discovery.
//!
//! A `subtest NAME => sub { ... };` call names a nested group of assertions.
//! This module walks a parsed AST and reconstructs the *structure* of those
//! groups — a tree of [`DiscoveredSubtest`] — without executing anything. The
//! tree drives document symbols, code lenses, and "run/debug nearest subtest".
//!
//! We only read structure. Dynamic subtest names (a variable or expression
//! rather than a string literal) are reported as [`SubtestName::Dynamic`]
//! rather than guessed — the caller decides how to present an unknown name.

use crate::providers::document_symbols::DocumentSymbol;
use perl_parser_core::ast::{Node, NodeKind};
use perl_position_tracking::WireRange;

/// LSP `SymbolKind` used for subtests in the document outline. `12` is
/// `Function`, which renders with a runnable-looking glyph in common clients.
const SUBTEST_SYMBOL_KIND: u32 = 12;

/// The name of a discovered subtest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubtestName {
    /// A statically known name from a string-literal first argument.
    Named(String),
    /// The name is computed at runtime (variable/expression) and not knowable
    /// statically. Callers should present it as unknown, not guess.
    Dynamic,
}

impl SubtestName {
    /// A display label for outlines/lenses.
    pub fn label(&self) -> String {
        match self {
            SubtestName::Named(name) => name.clone(),
            SubtestName::Dynamic => "subtest (dynamic)".to_string(),
        }
    }

    /// The statically known name, if any.
    pub fn as_static(&self) -> Option<&str> {
        match self {
            SubtestName::Named(name) => Some(name.as_str()),
            SubtestName::Dynamic => None,
        }
    }
}

/// A subtest discovered in the source, plus any subtests nested inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredSubtest {
    /// The subtest's name (static or dynamic).
    pub name: SubtestName,
    /// Full range of the `subtest ... => sub { ... }` call.
    pub range: WireRange,
    /// Range of the name argument (used as the outline selection range).
    pub name_range: WireRange,
    /// Nested subtests declared inside this subtest's block.
    pub children: Vec<DiscoveredSubtest>,
}

/// Discover the subtest tree in a parsed document.
pub fn discover_subtests(ast: &Node, source: &str) -> Vec<DiscoveredSubtest> {
    let mut out = Vec::new();
    walk(ast, source, &mut out);
    out
}

/// Convert a discovered subtest tree into LSP document symbols.
pub fn subtest_document_symbols(subtests: &[DiscoveredSubtest]) -> Vec<DocumentSymbol> {
    subtests.iter().map(to_document_symbol).collect()
}

/// Find the innermost subtest whose range contains the 0-based `line`.
///
/// Used by "run/debug nearest subtest": given the cursor line, resolve which
/// subtest the caller is inside. Returns `None` when the line is not inside any
/// subtest. Descends into children so the *innermost* enclosing subtest wins.
pub fn nearest_subtest_at_line(
    subtests: &[DiscoveredSubtest],
    line: u32,
) -> Option<&DiscoveredSubtest> {
    for candidate in subtests {
        if candidate.range.start.line <= line && line <= candidate.range.end.line {
            return Some(nearest_subtest_at_line(&candidate.children, line).unwrap_or(candidate));
        }
    }
    None
}

/// Discover subtests in `ast` and resolve the innermost one enclosing `line`.
pub fn nearest_subtest_in_source(ast: &Node, source: &str, line: u32) -> Option<DiscoveredSubtest> {
    let subtests = discover_subtests(ast, source);
    nearest_subtest_at_line(&subtests, line).cloned()
}

fn to_document_symbol(subtest: &DiscoveredSubtest) -> DocumentSymbol {
    DocumentSymbol {
        name: subtest.name.label(),
        detail: "subtest".to_string(),
        kind: SUBTEST_SYMBOL_KIND,
        range: subtest.range,
        selection_range: subtest.name_range,
        children: subtest.children.iter().map(to_document_symbol).collect(),
    }
}

/// Walk `node`, appending every subtest found at this level to `out`. When a
/// subtest is found, its own nested subtests are collected into its `children`
/// and the walk does not descend into it again (so nesting stays a tree, not a
/// flattened list).
fn walk(node: &Node, source: &str, out: &mut Vec<DiscoveredSubtest>) {
    if let Some(subtest) = try_as_subtest(node, source) {
        out.push(subtest);
        return;
    }
    for child in structural_children(node) {
        walk(child, source, out);
    }
}

/// Whether `name` is a Test2 subtest-defining call. Shared by the discovery
/// path (this module) and the code-lens path so the two never diverge on which
/// call names are treated as subtests.
pub fn is_subtest_call_name(name: &str) -> bool {
    matches!(name, "subtest" | "subtest_buffered" | "subtest_streamed")
}

/// If `node` is a `subtest NAME => sub { ... }` call, build a
/// [`DiscoveredSubtest`] (with nested children discovered inside its block).
fn try_as_subtest(node: &Node, source: &str) -> Option<DiscoveredSubtest> {
    let NodeKind::FunctionCall { name, args } = &node.kind else {
        return None;
    };
    if !is_subtest_call_name(name) {
        return None;
    }

    let first = args.first()?;
    let name = subtest_name_from_arg(first);
    let name_range = WireRange::from_byte_offsets(source, first.location.start, first.location.end);
    let range = WireRange::from_byte_offsets(source, node.location.start, node.location.end);

    // Collect nested subtests from the block body of the anonymous sub argument.
    let mut children = Vec::new();
    for arg in args.iter().skip(1) {
        if let NodeKind::Subroutine { body, .. } = &arg.kind {
            walk(body, source, &mut children);
        }
    }

    Some(DiscoveredSubtest { name, range, name_range, children })
}

/// Extract a subtest name from its first argument. A string literal yields a
/// static name (quotes stripped); anything else is dynamic.
fn subtest_name_from_arg(arg: &Node) -> SubtestName {
    match &arg.kind {
        NodeKind::String { value, interpolated } => {
            let unquoted = strip_string_quotes(value);
            // An interpolated string with a variable is only partially static;
            // treat a purely literal (non-interpolated) string as the name and
            // anything interpolated as dynamic to avoid presenting a raw
            // `$var`-laden label as if it were the real name.
            if *interpolated && unquoted.contains('$') {
                SubtestName::Dynamic
            } else {
                SubtestName::Named(unquoted.to_string())
            }
        }
        _ => SubtestName::Dynamic,
    }
}

/// Strip a single layer of matching surrounding quotes from a string token.
/// The parser stores string values including their delimiters (e.g. `'outer'`).
fn strip_string_quotes(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'\'' || first == b'"') && first == last {
            return &value[1..value.len() - 1];
        }
    }
    value
}

/// The structural child nodes of `node` that a subtest could be nested within.
/// This intentionally covers the node kinds that appear in test files
/// (statements, blocks, calls, control flow) rather than every AST variant.
fn structural_children(node: &Node) -> Vec<&Node> {
    match &node.kind {
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            statements.iter().collect()
        }
        NodeKind::ExpressionStatement { expression } => vec![expression.as_ref()],
        NodeKind::FunctionCall { args, .. } => args.iter().collect(),
        NodeKind::Subroutine { body, .. } => vec![body.as_ref()],
        NodeKind::If { condition, then_branch, elsif_branches, else_branch, .. } => {
            let mut children: Vec<&Node> = vec![condition.as_ref(), then_branch.as_ref()];
            for (cond, branch) in elsif_branches {
                children.push(cond.as_ref());
                children.push(branch.as_ref());
            }
            if let Some(else_branch) = else_branch {
                children.push(else_branch.as_ref());
            }
            children
        }
        NodeKind::While { condition, body, .. } => vec![condition.as_ref(), body.as_ref()],
        NodeKind::For { condition, body, .. } => {
            let mut children: Vec<&Node> = Vec::new();
            if let Some(condition) = condition {
                children.push(condition.as_ref());
            }
            children.push(body.as_ref());
            children
        }
        NodeKind::Foreach { list, body, .. } => vec![list.as_ref(), body.as_ref()],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests;
