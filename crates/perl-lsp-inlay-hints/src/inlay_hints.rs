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

use perl_lexer::create_builtin_signatures;
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
    /// Optional tooltip (deferred to resolve)
    pub tooltip: Option<String>,
    /// Optional source location for jump-to-definition from hint label
    pub location: Option<HintLocation>,
}

/// Source location attached to a hint for label.location support (LSP 3.17).
#[derive(Debug, Clone)]
pub struct HintLocation {
    /// Document URI
    pub uri: String,
    /// Byte range of the target symbol in the source document
    pub range: (usize, usize),
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
                let tooltip = v
                    .get("tooltip")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string());
                Some(InlayHint {
                    position: Position::new(
                        pos["line"].as_u64()? as u32,
                        pos["character"].as_u64()? as u32,
                    ),
                    label,
                    kind,
                    padding_left: v["paddingLeft"].as_bool().unwrap_or(false),
                    padding_right: v["paddingRight"].as_bool().unwrap_or(false),
                    tooltip,
                    location: None,
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
                let tooltip = v
                    .get("tooltip")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string());
                Some(InlayHint {
                    position: Position::new(
                        pos["line"].as_u64()? as u32,
                        pos["character"].as_u64()? as u32,
                    ),
                    label,
                    kind,
                    padding_left: v["paddingLeft"].as_bool().unwrap_or(false),
                    padding_right: v["paddingRight"].as_bool().unwrap_or(false),
                    tooltip,
                    location: None,
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

                    // Phase 1: embed function name and param index in data for
                    // later label.location resolution via inlayHint/resolve.
                    let mut hint = json!({
                        "position": { "line": l, "character": c },
                        "label": format!("{}:", param_names[i]),
                        "kind": 2, // parameter
                        "paddingLeft": false,
                        "paddingRight": true,
                        "data": {
                            "functionName": name.as_str(),
                            "paramIndex": i,
                        }
                    });

                    // Phase 3: embed perldoc summary for tooltip resolution.
                    // The resolver will pick this up when the client requests it.
                    if let Some(doc) = builtin_doc_summary(name.as_str(), &param_names[i], i) {
                        hint["data"]["docSummary"] = json!(doc);
                    }

                    out.push(hint);
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
            NodeKind::Number { .. } => Some(("Num".to_string(), Some("Numeric literal"))),
            NodeKind::String { .. } => Some(("Str".to_string(), Some("String literal"))),
            NodeKind::HashLiteral { .. } => Some(("Hash".to_string(), Some("Hash reference"))),
            NodeKind::ArrayLiteral { .. } => Some(("Array".to_string(), Some("Array reference"))),
            NodeKind::Regex { .. } => Some(("Regex".to_string(), Some("Regular expression"))),
            NodeKind::Subroutine { name: None, .. } => Some((
                "CodeRef".to_string(),
                Some("Anonymous subroutine (code reference)"),
            )),
            // Fall through to semantic type inference for non-literal nodes
            _ => infer_semantic_type(node).map(|t| (t, None)),
        };

        if let Some((hint, tooltip)) = type_hint {
            let (l, c) = to_pos16(node.location.end);

            // Filter by range if specified
            if let Some(filter_range) = range {
                let hint_pos = Position::new(l, c);
                if !pos_in_range(hint_pos, filter_range) {
                    return true;
                }
            }

            let mut val = json!({
                "position": {"line": l, "character": c},
                "label": format!(": {}", hint),
                "kind": 1, // type
                "paddingLeft": true,
                "paddingRight": false
            });

            // Phase 3: embed tooltip text for deferred resolution
            if let Some(tt) = tooltip {
                val["data"] = json!({ "tooltip": tt });
            }

            out.push(val);
        }
        true
    });
    out
}

// ---------------------------------------------------------------------------
// Phase 2: Semantic type inference
// ---------------------------------------------------------------------------

/// Infers a semantic type label for an expression node.
///
/// Goes beyond trivial literal detection by examining context:
/// - Scalar variables assigned from known-return-type functions
/// - Array/hash from builtins like `keys`, `values`, `split`
/// - Blessed references from `new` / `bless` calls
/// - Filehandle operations
///
/// Returns `None` when the type cannot be determined.
pub fn infer_semantic_type(node: &Node) -> Option<String> {
    match &node.kind {
        NodeKind::FunctionCall { name, .. } => function_return_type(name),
        NodeKind::MethodCall { method, .. } => method_return_type(method),
        NodeKind::Variable { name, sigil } => {
            // Infer from common naming conventions
            match (sigil.as_str(), name.as_str()) {
                ("$", _) if name.ends_with("_fh") || name.ends_with("_handle") => {
                    Some("FileHandle".to_string())
                }
                ("$", _) if name.ends_with("_ref") => Some("Ref".to_string()),
                ("@", _) if name.ends_with("_nums") => Some("@Nums".to_string()),
                ("@", _) if name.ends_with("_strs") => Some("@Strs".to_string()),
                ("@", _) if name.ends_with("_lines") => Some("@Lines".to_string()),
                ("%", _) => Some("Hash".to_string()),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Return type for known builtin functions.
fn function_return_type(name: &str) -> Option<String> {
    match name {
        "open" => Some("Bool|FileHandle".to_string()),
        "split" => Some("@Str".to_string()),
        "join" => Some("Str".to_string()),
        "keys" | "values" | "each" => Some("List".to_string()),
        "map" | "grep" => Some("@List".to_string()),
        "sort" => Some("@Sorted".to_string()),
        "reverse" => Some("@List|Str".to_string()),
        "scalar" => Some("Scalar".to_string()),
        "ref" => Some("Str|Undef".to_string()),
        "bless" => Some("Object".to_string()),
        "stat" | "lstat" => Some("@Stat".to_string()),
        "localtime" | "gmtime" => Some("@Time|Str".to_string()),
        "caller" => Some("@Caller|Hash".to_string()),
        "wantarray" => Some("Bool|Undef".to_string()),
        "defined" => Some("Bool".to_string()),
        "length" | "index" | "rindex" | "substr" => Some("Int".to_string()),
        "abs" | "int" | "sqrt" | "exp" | "log" | "cos" | "sin" => Some("Num".to_string()),
        "chr" => Some("Str".to_string()),
        "ord" => Some("Int".to_string()),
        "uc" | "lc" | "ucfirst" | "lcfirst" => Some("Str".to_string()),
        "pack" => Some("Str".to_string()),
        "unpack" => Some("@Mixed".to_string()),
        _ => None,
    }
}

/// Return type for known method calls.
fn method_return_type(method: &str) -> Option<String> {
    match method {
        "new" => Some("Object".to_string()),
        "count" | "size" | "length" => Some("Int".to_string()),
        "push" | "unshift" | "splice" => Some("Int".to_string()),
        "pop" | "shift" => Some("Scalar".to_string()),
        "keys" | "values" => Some("@List".to_string()),
        "exists" | "defined" => Some("Bool".to_string()),
        "delete" => Some("Scalar".to_string()),
        "fetch" | "get" => Some("Scalar".to_string()),
        "put" | "set" | "store" => Some("Undef".to_string()),
        "find" | "search" => Some("@Results|Undef".to_string()),
        "first" | "next" => Some("Scalar|Undef".to_string()),
        "all" => Some("@All".to_string()),
        "each" | "iterator" => Some("Iterator".to_string()),
        "isa" => Some("Bool".to_string()),
        "can" => Some("CodeRef|Undef".to_string()),
        "clone" => Some("Object".to_string()),
        "to_string" | "as_string" | "stringify" => Some("Str".to_string()),
        "to_array" | "as_array" | "elements" => Some("@Array".to_string()),
        "to_hash" | "as_hash" => Some("%Hash".to_string()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Phase 3: Documentation integration
// ---------------------------------------------------------------------------

/// Returns a short perldoc-style summary for a builtin function parameter.
///
/// Looks up the builtin's documentation from `perl_lexer::create_builtin_signatures`
/// rather than maintaining a hardcoded list. Falls back to `None` for unknown
/// builtins or parameters.
fn builtin_doc_summary(function: &str, param: &str, _param_index: usize) -> Option<String> {
    let sigs = create_builtin_signatures();
    let builtin = sigs.get(function)?;
    // Use the first signature variant to extract param names and match
    // against the requested parameter.
    if let Some(first_sig) = builtin.signatures.first() {
        let param_names = extract_param_names(first_sig);
        if param_names.contains(&param.to_string()) {
            // Return the builtin's documentation as the summary.
            // The full doc covers the function; callers can truncate or
            // format it as needed.
            return Some(builtin.documentation.to_string());
        }
    }
    None
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
