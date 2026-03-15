//! Inlay hints provider for Perl code.
//!
//! Provides inlay hints for function parameters and type annotations to improve
//! code readability without modifying the source.
//!
//! # LSP Context
//!
//! Implements `textDocument/inlayHint` for the Parse → Analyze stages to surface
//! inline annotations during language server rendering.
//!
//! # Client capability requirements
//!
//! Clients must advertise the inlay hint capability (`textDocument/inlayHint`)
//! to receive hint payloads.
//!
//! # Protocol compliance
//!
//! Follows the inlay hint protocol for range-scoped responses and stable hint
//! ordering per the LSP specification.

use perl_builtins::builtin_signatures::create_builtin_signatures;
use perl_parser_core::ast::{Node, NodeKind};
use perl_position_tracking::{WirePosition as Position, WireRange as Range};
use perl_semantic_analyzer::declaration::get_node_children;
use serde_json::Value;
use serde_json::json;

/// Inlay hint kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlayHintKind {
    /// Type hint
    Type = 1,
    /// Parameter hint
    Parameter = 2,
}

/// Inlay hint.
#[derive(Debug, Clone)]
pub struct InlayHint {
    /// Position of the hint
    pub position: Position,
    /// Label text
    pub label: String,
    /// Kind of hint
    pub kind: InlayHintKind,
    /// Padding on the left
    pub padding_left: bool,
    /// Padding on the right
    pub padding_right: bool,
}

/// Inlay hints provider.
pub struct InlayHintsProvider;

impl InlayHintsProvider {
    /// Create a new inlay hints provider.
    pub fn new() -> Self {
        Self
    }

    /// Generate inlay hints for the given AST.
    pub fn generate_hints(
        &self,
        ast: &Node,
        to_pos16: &impl Fn(usize) -> (u32, u32),
        range: Option<Range>,
    ) -> Vec<InlayHint> {
        let mut hints = Vec::new();
        hints.extend(self.parameter_hints(ast, to_pos16, range));
        hints.extend(self.trivial_type_hints(ast, to_pos16, range));
        hints
    }

    /// Generate parameter hints.
    pub fn parameter_hints(
        &self,
        ast: &Node,
        to_pos16: &impl Fn(usize) -> (u32, u32),
        range: Option<Range>,
    ) -> Vec<InlayHint> {
        parameter_hints(ast, to_pos16, range)
            .into_iter()
            .filter_map(|v| {
                let pos = v["position"].clone();
                let label = v["label"].as_str()?.to_string();
                let kind = match v["kind"].as_u64().unwrap_or(1) {
                    2 => InlayHintKind::Parameter,
                    _ => InlayHintKind::Type,
                };
                Some(InlayHint {
                    position: Position::new(
                        pos["line"].as_u64()? as u32,
                        pos["character"].as_u64()? as u32,
                    ),
                    label,
                    kind,
                    padding_left: v["paddingLeft"].as_bool().unwrap_or(false),
                    padding_right: v["paddingRight"].as_bool().unwrap_or(false),
                })
            })
            .collect()
    }

    /// Generate trivial type hints.
    pub fn trivial_type_hints(
        &self,
        ast: &Node,
        to_pos16: &impl Fn(usize) -> (u32, u32),
        range: Option<Range>,
    ) -> Vec<InlayHint> {
        trivial_type_hints(ast, to_pos16, range)
            .into_iter()
            .filter_map(|v| {
                let pos = v["position"].clone();
                let label = v["label"].as_str()?.to_string();
                let kind = match v["kind"].as_u64().unwrap_or(1) {
                    2 => InlayHintKind::Parameter,
                    _ => InlayHintKind::Type,
                };
                Some(InlayHint {
                    position: Position::new(
                        pos["line"].as_u64()? as u32,
                        pos["character"].as_u64()? as u32,
                    ),
                    label,
                    kind,
                    padding_left: v["paddingLeft"].as_bool().unwrap_or(false),
                    padding_right: v["paddingRight"].as_bool().unwrap_or(false),
                })
            })
            .collect()
    }
}

impl Default for InlayHintsProvider {
    fn default() -> Self {
        Self::new()
    }
}

fn pos_in_range(pos: Position, range: Range) -> bool {
    if pos.line < range.start.line || pos.line > range.end.line {
        return false;
    }
    if pos.line == range.start.line && pos.character < range.start.character {
        return false;
    }
    if pos.line == range.end.line && pos.character >= range.end.character {
        return false;
    }
    true
}

/// Extracts parameter names from a builtin signature string.
///
/// Signature strings follow the Perl perldoc convention, e.g.:
/// - `"open FILEHANDLE, MODE, FILENAME"` → `["filehandle", "mode", "filename"]`
/// - `"push ARRAY, LIST"` → `["array", "list"]`
/// - `"split /PATTERN/, EXPR, LIMIT"` → `["pattern", "expr", "limit"]`
/// - `"map BLOCK LIST"` → `["block", "list"]`
///
/// The function name prefix is stripped, comma-separated groups are split,
/// and within each group space-separated tokens are treated as individual
/// parameters. Slash delimiters (e.g. `/PATTERN/`) are removed and all names
/// are lowercased.
pub fn extract_param_names(signature: &str) -> Vec<String> {
    // Strip function name prefix (first word)
    let rest = match signature.find(' ') {
        Some(idx) => &signature[idx + 1..],
        None => return Vec::new(),
    };

    let mut params = Vec::new();
    // Split on ", " to get comma-separated groups
    for group in rest.split(", ") {
        // Within each group, split on space for space-separated params
        for token in group.split(' ') {
            if token.is_empty() {
                continue;
            }
            // Strip slash delimiters from patterns like /PATTERN/
            let cleaned = token.trim_matches('/');
            params.push(cleaned.to_lowercase());
        }
    }
    params
}

/// Generates inlay hints for function and method parameters.
///
/// This function traverses the AST and identifies function calls, adding inlay
/// hints for parameter names based on the builtin signatures database from the
/// `perl-builtins` crate. Any builtin with a known signature will produce
/// parameter name hints for its arguments.
///
/// # Arguments
///
/// * `ast` - The root node of the AST to traverse.
/// * `to_pos16` - A function that converts a byte offset to a (line, character) tuple.
/// * `range` - An optional range to filter the inlay hints.
///
/// # Returns
///
/// A vector of `serde_json::Value` objects, each representing an inlay hint.
pub fn parameter_hints(
    ast: &Node,
    to_pos16: &impl Fn(usize) -> (u32, u32),
    range: Option<Range>,
) -> Vec<Value> {
    let sigs = create_builtin_signatures();
    let mut out = Vec::new();
    walk_ast(ast, &mut |node| {
        if let NodeKind::FunctionCall { name, args } = &node.kind
            && let Some(builtin) = sigs.get(name.as_str())
        {
            // Use the first (most complete) signature variant to extract
            // parameter names, since it lists all possible parameters.
            if let Some(first_sig) = builtin.signatures.first() {
                let param_names = extract_param_names(first_sig);

                // Skip functions with only a single parameter -- hints
                // for e.g. `chomp($x)` showing `variable:` add noise
                // rather than clarity.
                if param_names.len() <= 1 {
                    return true;
                }

                for (i, arg) in args.iter().enumerate() {
                    if i >= param_names.len() {
                        break;
                    }
                    let (l, c) = to_pos16(arg.location.start);

                    // Filter by range if specified
                    if let Some(filter_range) = range {
                        let hint_pos = Position::new(l, c);
                        if !pos_in_range(hint_pos, filter_range) {
                            continue;
                        }
                    }

                    out.push(json!({
                        "position": { "line": l, "character": c },
                        "label": format!("{}:", param_names[i]),
                        "kind": 2, // parameter
                        "paddingLeft": false,
                        "paddingRight": true
                    }));
                }
            }
        }
        true
    });
    out
}

/// Generates inlay hints for trivial types.
///
/// This function traverses AST and adds inlay hints for literals such as
/// numbers, strings, and code references.
///
/// # Arguments
///
/// * `ast` - The root node of the AST to traverse.
/// * `to_pos16` - A function that converts a byte offset to a (line, character) tuple.
/// * `range` - An optional range to filter the inlay hints.
///
/// # Returns
///
/// A vector of `serde_json::Value` objects, each representing an inlay hint.
pub fn trivial_type_hints(
    ast: &Node,
    to_pos16: &impl Fn(usize) -> (u32, u32),
    range: Option<Range>,
) -> Vec<Value> {
    let mut out = Vec::new();
    walk_ast(ast, &mut |node| {
        let type_hint = match &node.kind {
            NodeKind::Number { .. } => Some("Num"),
            NodeKind::String { .. } => Some("Str"),
            NodeKind::HashLiteral { .. } => Some("Hash"),
            NodeKind::ArrayLiteral { .. } => Some("Array"),
            NodeKind::Regex { .. } => Some("Regex"),
            NodeKind::Subroutine { name: None, .. } => Some("CodeRef"),
            _ => None,
        };

        if let Some(hint) = type_hint {
            let (l, c) = to_pos16(node.location.end);

            // Filter by range if specified
            if let Some(filter_range) = range {
                let hint_pos = Position::new(l, c);
                if !pos_in_range(hint_pos, filter_range) {
                    return true;
                }
            }

            out.push(json!({
                "position": {"line": l, "character": c},
                "label": format!(": {}", hint),
                "kind": 1, // type
                "paddingLeft": true,
                "paddingRight": false
            }));
        }
        true
    });
    out
}

fn walk_ast<F>(node: &Node, visitor: &mut F) -> bool
where
    F: FnMut(&Node) -> bool,
{
    if !visitor(node) {
        return false;
    }

    for child in get_node_children(node) {
        if !walk_ast(child, visitor) {
            return false;
        }
    }

    true
}
