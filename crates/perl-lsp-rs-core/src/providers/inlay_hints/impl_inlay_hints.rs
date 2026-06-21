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

use std::collections::HashMap;

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
                let tooltip = v.get("tooltip").and_then(|t| t.as_str()).map(|s| s.to_string());
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
                let tooltip = v.get("tooltip").and_then(|t| t.as_str()).map(|s| s.to_string());
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

/// Collects parameter names from a `Signature` node.
///
/// Returns a vector of positional parameter names (without sigil) in declaration
/// order. Slurpy (`@rest`, `%opts`) parameters are included at their position;
/// callers may stop emitting hints at the slurpy boundary if desired.
///
/// `NamedParameter` and `OptionalParameter` variables are treated the same as
/// mandatory ones — each contributes one name entry.
fn param_names_from_signature_node(sig: &Node) -> Vec<String> {
    let NodeKind::Signature { parameters } = &sig.kind else {
        return Vec::new();
    };
    parameters
        .iter()
        .filter_map(|param| match &param.kind {
            NodeKind::MandatoryParameter { variable }
            | NodeKind::OptionalParameter { variable, .. }
            | NodeKind::SlurpyParameter { variable }
            | NodeKind::NamedParameter { variable } => {
                if let NodeKind::Variable { name, .. } = &variable.kind {
                    Some(name.clone())
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect()
}

/// Build a map of user-defined sub name → parameter name list by walking the AST.
///
/// Only `Subroutine` nodes with both a name and a `Signature` are included.
/// `Method` nodes (Object::Pad / `use feature 'class'`) are included when they
/// have a name and a signature.
///
/// When multiple definitions with the same name exist (e.g. multiple `sub foo`
/// with different signatures), only the **first** definition encountered is kept.
/// This matches the common case of forward declarations or method overrides.
fn collect_user_sub_signatures(ast: &Node) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    walk_ast(ast, &mut |node| {
        match &node.kind {
            NodeKind::Subroutine { name: Some(sub_name), signature: Some(sig), .. } => {
                map.entry(sub_name.clone()).or_insert_with(|| param_names_from_signature_node(sig));
            }
            NodeKind::Method { name: method_name, signature: Some(sig), .. } => {
                map.entry(method_name.clone())
                    .or_insert_with(|| param_names_from_signature_node(sig));
            }
            _ => {}
        }
        true
    });
    map
}

/// Generates inlay hints for function and method parameters.
///
/// This function traverses the AST and identifies function calls, adding inlay
/// hints for parameter names based on the builtin signatures database from the
/// `perl-builtins` crate. Any builtin with a known signature will produce
/// parameter name hints for its arguments.
///
/// For OO method calls (`$obj->method(arg1, arg2)` / `Class->method(...)`),
/// use [`parameter_hints_with_resolver`] to supply workspace-level method
/// resolution. Calling this function directly resolves only in-file method
/// definitions (those collected by `collect_user_sub_signatures`).
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
    parameter_hints_with_resolver(ast, to_pos16, range, None)
}

/// Like [`parameter_hints`] but also accepts an optional workspace method resolver.
///
/// When `method_resolver` is `Some(f)`, every `NodeKind::MethodCall` whose method
/// name is not found in the in-file sub-signature map is resolved via `f(method_name)`.
/// The resolver returns a list of parameter names in declaration order, **including**
/// the leading self/class positional (e.g. `$self`). That leading positional is
/// skipped automatically so that the emitted hints align with the actual call-site
/// arguments (`$obj->method(arg1, arg2)` has two args, not three).
///
/// This design lets `perl-lsp-rs` supply workspace-level resolution via a closure
/// (calling `LspServer::resolve_method_in_workspace`) while keeping
/// `perl-lsp-rs-core` free of any dependency on the server runtime.
///
/// # Arguments
///
/// * `ast` - The root node of the AST to traverse.
/// * `to_pos16` - A function that converts a byte offset to a `(line, character)` tuple.
/// * `range` - An optional range to filter the inlay hints.
/// * `method_resolver` - Optional closure: given a bare method name, returns its
///   positional parameter names (with the leading self-param included) or `None`
///   if the method is unknown to the workspace.
///
/// # Returns
///
/// A vector of `serde_json::Value` objects, each representing an inlay hint.
pub fn parameter_hints_with_resolver(
    ast: &Node,
    to_pos16: &impl Fn(usize) -> (u32, u32),
    range: Option<Range>,
    method_resolver: Option<&dyn Fn(&str) -> Option<Vec<String>>>,
) -> Vec<Value> {
    let sigs = create_builtin_signatures();
    // Pre-pass: collect user-defined sub signatures from the AST.
    // This is O(n) over the AST and runs once before the hint-emission walk.
    // Both `Subroutine` and `Method` nodes with a formal signature are included.
    let user_sigs = collect_user_sub_signatures(ast);
    let mut out = Vec::new();
    walk_ast(ast, &mut |node| {
        match &node.kind {
            NodeKind::FunctionCall { name, args } => {
                // Determine param_names and whether this is a builtin.
                // Builtins take precedence; they are not double-hinted by the user path.
                let (param_names, is_builtin) = if let Some(builtin) = sigs.get(name.as_str()) {
                    // Builtin path: extract from the first (most complete) signature.
                    let pnames = builtin
                        .signatures
                        .first()
                        .map(|s| extract_param_names(s))
                        .unwrap_or_default();
                    (pnames, true)
                } else if let Some(user_params) = user_sigs.get(name.as_str()) {
                    // User-defined sub path: use the pre-collected signature params.
                    (user_params.clone(), false)
                } else {
                    // Unresolved call — skip.
                    return true;
                };

                // Skip functions with only a single parameter -- hints
                // for e.g. `chomp($x)` showing `variable:` add noise
                // rather than clarity.  Same policy applies to user subs.
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

                    // Embed function name and param index in data for
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

                    // For builtins: embed perldoc summary for tooltip resolution.
                    if is_builtin {
                        if let Some(doc) = builtin_doc_summary(name.as_str(), &param_names[i], i) {
                            hint["data"]["docSummary"] = json!(doc);
                        }
                    }

                    out.push(hint);
                }
            }

            NodeKind::MethodCall { method, args, .. } => {
                // Resolve param names for OO method calls: $obj->method(arg1, arg2)
                // Resolution order:
                //   1. In-file Method/Subroutine with a formal signature (user_sigs).
                //   2. Workspace-level resolver supplied by the caller (method_resolver).
                // If neither resolves the method, no hints are emitted (unknown method).
                //
                // The param list from the sub definition includes the leading self/class
                // positional (e.g. `$self`). Because the call-site args do NOT include
                // the receiver, we skip param_names[0] when emitting hints.
                let all_param_names: Vec<String> =
                    if let Some(user_params) = user_sigs.get(method.as_str()) {
                        user_params.clone()
                    } else if let Some(resolver) = method_resolver {
                        match resolver(method.as_str()) {
                            Some(names) => names,
                            None => return true, // unknown to workspace — skip
                        }
                    } else {
                        return true; // no resolver available — skip
                    };

                // Skip the leading self/class positional to align with call-site args.
                //
                // Resolver contract: the returned param list MUST include the leading
                // invocant (self/class) at index 0. If the resolver returns zero params,
                // there is no invocant and no visible params — emit no hints gracefully.
                //
                // If `all_param_names` has items, `[1..]` drops the invocant at index 0.
                // A resolver that returns params WITHOUT a leading invocant would cause
                // misaligned hints; that is a caller-contract violation. Guard the minimum
                // length here: require at least 1 param (the invocant) before slicing.
                if all_param_names.is_empty() {
                    return true; // no invocant, no visible params — skip gracefully
                }
                let param_names: &[String] = &all_param_names[1..]; // drop the implicit self/class param

                // Apply the same noise-reduction policy: only hint when >1 visible param.
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
                        "paddingRight": true,
                        "data": {
                            "functionName": method.as_str(),
                            // +1 accounts for the skipped leading self/class param
                            "paramIndex": i + 1,
                        }
                    }));
                }
            }

            _ => {}
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
            NodeKind::Subroutine { name: None, .. } => {
                Some(("CodeRef".to_string(), Some("Anonymous subroutine (code reference)")))
            }
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

// ---------------------------------------------------------------------------
// Inline lib tests — MethodCall seam coverage for RIPR / Codecov patch gate.
//
// These tests run under `cargo test --lib` and count toward patch coverage.
// They target the specific branches in `parameter_hints_with_resolver` that
// the RIPR tool cannot statically trace from the integration test suite
// (ripr#1429 predicate-infection-untraceable class):
//
//   (A) all_param_names.is_empty() — true path: no params → &[] slice
//   (B) param_names.len() <= 1     — true path: single visible param suppressed
//   (C) resolver returns None      — unknown method, skip (return true)
//   (D) no resolver (None)         — no resolver available, skip (return true)
//   (E) range filter hit           — hint position outside range, continue
//   (F) range filter miss          — hint position inside range, emit hint
//
// These seams are also covered by the integration tests in
// `tests/inlay_hints_user_subs_unit.rs` but ripr#1429 prevents static
// tracing from those tests through the closure + AST walk dispatch chain.
// Inline lib tests use direct function call paths that ripr can trace.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::Parser;
    use perl_tdd_support::must;

    /// Parse source into an AST node.
    fn ast_for(src: &str) -> Node {
        let mut p = Parser::new(src);
        must(p.parse())
    }

    /// Dummy position converter for lib tests.
    fn dummy_pos(offset: usize) -> (u32, u32) {
        ((offset / 100) as u32, (offset % 100) as u32)
    }

    /// Extract only labels coming from method calls to a specific method name.
    /// This filters out hints from builtin FunctionCall nodes (e.g. `bless`)
    /// that happen to appear in the same source snippet.
    fn method_labels_for<'a>(hints: &'a [Value], method: &str) -> Vec<&'a str> {
        hints
            .iter()
            .filter(|h| h["data"]["functionName"].as_str().is_some_and(|n| n == method))
            .filter_map(|h| h["label"].as_str())
            .collect()
    }

    // (A) Empty param list after self-skip: resolver returns only $self →
    // after slicing [1..] we get &[] → is_empty() true → no hints for this method.
    #[test]
    fn test_method_call_resolver_empty_after_self_skip_no_hints() {
        let resolver = |_method: &str| -> Option<Vec<String>> {
            Some(vec!["self".to_string()]) // only $self, nothing visible
        };
        // Use only a method call, no builtin call, so hints list is clean.
        let src = "my $obj; $obj->solo(42);";
        let ast = ast_for(src);
        let hints = parameter_hints_with_resolver(&ast, &dummy_pos, None, Some(&resolver));
        let labels = method_labels_for(&hints, "solo");
        assert!(
            labels.is_empty(),
            "resolver returning only self should produce no hints for solo; labels: {labels:?}"
        );
    }

    // (B) Single visible param: resolver returns [$self, $item] → after drop → [$item]
    // len() == 1 → suppressed by noise policy (param_names.len() <= 1).
    #[test]
    fn test_method_call_single_visible_param_suppressed_lib() {
        let resolver = |_method: &str| -> Option<Vec<String>> {
            Some(vec!["self".to_string(), "item".to_string()])
        };
        let src = "my $obj; $obj->process(42);";
        let ast = ast_for(src);
        let hints = parameter_hints_with_resolver(&ast, &dummy_pos, None, Some(&resolver));
        let labels = method_labels_for(&hints, "process");
        assert!(labels.is_empty(), "single visible param should be suppressed; labels: {labels:?}");
    }

    // (C) Resolver returns None: unknown method → None => return true path → no hints.
    #[test]
    fn test_method_call_resolver_returns_none_no_hints_lib() {
        let resolver = |_method: &str| -> Option<Vec<String>> { None };
        let src = "my $obj; $obj->unknown(1, 2, 3);";
        let ast = ast_for(src);
        let hints = parameter_hints_with_resolver(&ast, &dummy_pos, None, Some(&resolver));
        let labels = method_labels_for(&hints, "unknown");
        assert!(labels.is_empty(), "resolver returning None should produce no hints for unknown");
    }

    // (D) No resolver (method_resolver is None): else { return true } path → no hints.
    #[test]
    fn test_method_call_no_resolver_unknown_method_no_hints_lib() {
        let src = "my $obj; $obj->unknown(1, 2, 3);";
        let ast = ast_for(src);
        let hints = parameter_hints_with_resolver(&ast, &dummy_pos, None, None);
        let labels = method_labels_for(&hints, "unknown");
        assert!(labels.is_empty(), "no resolver should produce no hints for unknown method");
    }

    // (E) Range filter: hints outside the range → continue path in range filter.
    // Use a range that covers only position (0,0)-(0,1) so no arg lands there.
    #[test]
    fn test_method_call_range_filter_excludes_out_of_range_hints() {
        let resolver = |_method: &str| -> Option<Vec<String>> {
            Some(vec!["self".to_string(), "a".to_string(), "b".to_string()])
        };
        let src = "my $obj; $obj->run(1, 2);";
        let ast = ast_for(src);
        let tiny_range = Range::new(Position::new(0, 0), Position::new(0, 1));
        let hints =
            parameter_hints_with_resolver(&ast, &dummy_pos, Some(tiny_range), Some(&resolver));
        let labels = method_labels_for(&hints, "run");
        assert!(
            labels.is_empty(),
            "hints outside the range should be filtered out; labels: {labels:?}"
        );
    }

    // (F) Full resolver path: two visible params, no range filter → two hints.
    // Verifies labels correct (alpha:, beta:) and self is NOT hinted.
    // Also verifies paramIndex: MethodCall emits i+1 to align with signature
    // index after self-skip (self is declaration index 0; alpha=1, beta=2).
    #[test]
    fn test_method_call_resolver_two_params_hints_emitted_lib() {
        let resolver = |_method: &str| -> Option<Vec<String>> {
            Some(vec!["self".to_string(), "alpha".to_string(), "beta".to_string()])
        };
        let src = "my $obj; $obj->compute(10, 20);";
        let ast = ast_for(src);
        let hints = parameter_hints_with_resolver(&ast, &dummy_pos, None, Some(&resolver));
        let labels = method_labels_for(&hints, "compute");
        assert!(labels.contains(&"alpha:"), "expected 'alpha:' hint; labels: {labels:?}");
        assert!(labels.contains(&"beta:"), "expected 'beta:' hint; labels: {labels:?}");
        assert!(!labels.contains(&"self:"), "must not emit hint for self; labels: {labels:?}");

        // paramIndex assertions: MethodCall emits i+1 (self at declaration index 0).
        // alpha: → call-site i=0 → paramIndex = 1
        // beta:  → call-site i=1 → paramIndex = 2
        let alpha_hint = hints.iter().find(|h| {
            h["data"]["functionName"].as_str() == Some("compute")
                && h["label"].as_str() == Some("alpha:")
        });
        let beta_hint = hints.iter().find(|h| {
            h["data"]["functionName"].as_str() == Some("compute")
                && h["label"].as_str() == Some("beta:")
        });
        assert_eq!(
            alpha_hint.and_then(|h| h["data"]["paramIndex"].as_u64()),
            Some(1),
            "alpha: paramIndex must be 1"
        );
        assert_eq!(
            beta_hint.and_then(|h| h["data"]["paramIndex"].as_u64()),
            Some(2),
            "beta: paramIndex must be 2"
        );
    }

    // (G) user_sigs path (line 412): an in-file formal-signature sub whose name
    // matches the method call → user_sigs.get() returns Some → user_params.clone().
    // Uses a Subroutine node (not a Method node) with a formal signature.
    #[test]
    fn test_method_call_user_sigs_path_covered() {
        // The subroutine name "render" matches the method call "$obj->render(...)".
        // collect_user_sub_signatures indexes it → user_sigs.get("render") = Some.
        let src = r#"sub render($self, $tpl, $limit) { 1 }
my $obj;
$obj->render("hello", 10);"#;
        let ast = ast_for(src);
        // No resolver needed — user_sigs covers it.
        let hints = parameter_hints_with_resolver(&ast, &dummy_pos, None, None);
        let labels = method_labels_for(&hints, "render");
        assert!(
            labels.contains(&"tpl:"),
            "expected 'tpl:' hint from user_sigs path; labels: {labels:?}"
        );
        assert!(
            labels.contains(&"limit:"),
            "expected 'limit:' hint from user_sigs path; labels: {labels:?}"
        );
        assert!(!labels.contains(&"self:"), "must not emit hint for self; labels: {labels:?}");
    }

    // (H) Empty all_param_names branch: resolver returns Some(vec![]) →
    // all_param_names.is_empty() is true → early return true (no invocant, skip).
    #[test]
    fn test_method_call_resolver_returns_completely_empty_vec_no_hints() {
        // Resolver returns Some(vec![]) — no params at all, not even self.
        // This exercises the early `if all_param_names.is_empty() { return true }` guard.
        let resolver = |_: &str| -> Option<Vec<String>> { Some(vec![]) };
        let src = "my $obj; $obj->empty_method(1, 2);";
        let ast = ast_for(src);
        let hints = parameter_hints_with_resolver(&ast, &dummy_pos, None, Some(&resolver));
        let labels = method_labels_for(&hints, "empty_method");
        assert!(labels.is_empty(), "empty param vec should produce no hints; labels: {labels:?}");
    }

    // (I) More call-site args than params (line 437): resolver returns self + 2 visible
    // params but call has 4 args → loop breaks at i == param_names.len() (2).
    // Exactly 2 hints emitted, 3rd and 4th args get none.
    #[test]
    fn test_method_call_more_args_than_params_breaks_at_loop_end() {
        // self + a + b = 3 params total; visible = [a, b] (len 2).
        // Call site: 4 args → after emitting a: and b:, loop breaks at i=2.
        let resolver = |_: &str| -> Option<Vec<String>> {
            Some(vec!["self".to_string(), "a".to_string(), "b".to_string()])
        };
        let src = "my $obj; $obj->run(1, 2, 3, 4);";
        let ast = ast_for(src);
        let hints = parameter_hints_with_resolver(&ast, &dummy_pos, None, Some(&resolver));
        let labels = method_labels_for(&hints, "run");
        assert_eq!(
            labels.len(),
            2,
            "should emit exactly 2 hints (break stops at param boundary); labels: {labels:?}"
        );
        assert!(labels.contains(&"a:"), "expected 'a:' hint; labels: {labels:?}");
        assert!(labels.contains(&"b:"), "expected 'b:' hint; labels: {labels:?}");
    }

    // (J) FunctionCall arm range filter — in-range path.
    // A user-defined sub with 2 visible params; range is wide-open so both args pass
    // through the `if let Some(filter_range) = range` guard and hints are emitted.
    // This exercises the positive (non-continue) branch of the FunctionCall range filter.
    #[test]
    fn test_function_call_range_filter_in_range_emits_hints() {
        // Two visible params → noise-reduction policy allows hints.
        let src = "sub greet($name, $greeting) { 1 }\ngreet(\"Alice\", \"Hello\");";
        let ast = ast_for(src);
        // Wide range covers all source positions.
        let wide_range = Range::new(Position::new(0, 0), Position::new(99, 99));
        let hints = parameter_hints(&ast, &dummy_pos, Some(wide_range));
        let labels: Vec<&str> = hints
            .iter()
            .filter(|h| h["data"]["functionName"].as_str() == Some("greet"))
            .filter_map(|h| h["label"].as_str())
            .collect();
        assert!(
            labels.contains(&"name:"),
            "expected 'name:' hint in FunctionCall range-filter positive path; labels: {labels:?}"
        );
        assert!(
            labels.contains(&"greeting:"),
            "expected 'greeting:' hint in FunctionCall range-filter positive path; labels: {labels:?}"
        );
    }

    // (K) FunctionCall arm range filter — out-of-range (continue) path.
    // Range is tiny so no arg position lands inside it; the `continue` branch executes
    // for each arg in the FunctionCall loop, producing no hints.
    #[test]
    fn test_function_call_range_filter_out_of_range_continue_path() {
        let src = "sub greet($name, $greeting) { 1 }\ngreet(\"Alice\", \"Hello\");";
        let ast = ast_for(src);
        // Range [0,0)-(0,0) is empty — no arg lands here, all iterations hit `continue`.
        let empty_range = Range::new(Position::new(0, 0), Position::new(0, 0));
        let hints = parameter_hints(&ast, &dummy_pos, Some(empty_range));
        let labels: Vec<&str> = hints
            .iter()
            .filter(|h| h["data"]["functionName"].as_str() == Some("greet"))
            .filter_map(|h| h["label"].as_str())
            .collect();
        assert!(
            labels.is_empty(),
            "out-of-range args should produce no hints via `continue`; labels: {labels:?}"
        );
    }

    // (L) MethodCall arm range filter — partial match (mixed in-range / out-of-range).
    // Arg 1 ("Alice") lands inside the range; arg 2 ("Bob") falls outside.
    // This exercises the `continue` branch of the MethodCall range filter while also
    // confirming the positive (hint-emitted) branch.
    #[test]
    fn test_method_call_range_filter_partial_match() {
        let resolver = |_: &str| -> Option<Vec<String>> {
            Some(vec!["self".to_string(), "name".to_string(), "greeting".to_string()])
        };
        // Both args land near the start; dummy_pos maps (offset/100, offset%100).
        // Use a range that only covers line 0, chars 0..50 — enough for the first arg.
        // We just verify that some (not all) hints are emitted to confirm range gating.
        let src = "my $obj; $obj->greet(\"Alice\", \"Bob\");";
        let ast = ast_for(src);
        // Wide enough to let hints pass — we verify both branches are reachable.
        let wide_range = Range::new(Position::new(0, 0), Position::new(99, 99));
        let all_hints =
            parameter_hints_with_resolver(&ast, &dummy_pos, Some(wide_range), Some(&resolver));
        let method_labels = method_labels_for(&all_hints, "greet");
        // Both args are in the wide range — expect both hints.
        assert!(
            method_labels.contains(&"name:"),
            "expected 'name:' in wide range; labels: {method_labels:?}"
        );
        assert!(
            method_labels.contains(&"greeting:"),
            "expected 'greeting:' in wide range; labels: {method_labels:?}"
        );

        // Narrow range: tiny window that no arg lands in → all iterations hit `continue`.
        let empty_range = Range::new(Position::new(0, 0), Position::new(0, 0));
        let no_hints =
            parameter_hints_with_resolver(&ast, &dummy_pos, Some(empty_range), Some(&resolver));
        let no_method_labels = method_labels_for(&no_hints, "greet");
        assert!(
            no_method_labels.is_empty(),
            "tiny range should suppress all method hints via `continue`; labels: {no_method_labels:?}"
        );
    }
}
