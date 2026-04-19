//! Comprehensive unit tests for `perl-lsp-inlay-hints`.
//!
//! Covers: `InlayHintsProvider`, `InlayHint`, `InlayHintKind`,
//! `parameter_hints`, `trivial_type_hints`, range filtering, and edge cases.

use perl_lsp_inlay_hints::{
    InlayHint, InlayHintKind, InlayHintsProvider, extract_param_names, parameter_hints,
    trivial_type_hints,
};
use perl_parser_core::ast::{Node, NodeKind, SourceLocation};
use perl_position_tracking::{WirePosition as Position, WireRange as Range};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Identity position mapper: byte offset → (line=0, character=offset).
fn identity_pos(offset: usize) -> (u32, u32) {
    (0, offset as u32)
}

/// Build a minimal AST program wrapping the given statements.
fn program(stmts: Vec<Node>) -> Node {
    Node::new(
        NodeKind::Program { statements: stmts },
        SourceLocation::new(0, 100),
    )
}

/// Build a function-call node.
fn func_call(name: &str, args: Vec<Node>, loc: SourceLocation) -> Node {
    Node::new(
        NodeKind::FunctionCall {
            name: name.to_string(),
            args,
        },
        loc,
    )
}

/// Build an expression-statement wrapping an inner node.
fn expr_stmt(inner: Node) -> Node {
    let loc = inner.location;
    Node::new(
        NodeKind::ExpressionStatement {
            expression: Box::new(inner),
        },
        loc,
    )
}

/// Build a number literal node.
fn number(value: &str, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Number {
            value: value.to_string(),
        },
        SourceLocation::new(start, end),
    )
}

/// Build a string literal node.
fn string(value: &str, interpolated: bool, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::String {
            value: value.to_string(),
            interpolated,
        },
        SourceLocation::new(start, end),
    )
}

/// Build an anonymous subroutine node (no name → CodeRef hint).
fn anon_sub(start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Subroutine {
            name: None,
            name_span: None,
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(Node::new(
                NodeKind::Block { statements: vec![] },
                SourceLocation::new(start, end),
            )),
        },
        SourceLocation::new(start, end),
    )
}

/// Build a named subroutine node (has name → no CodeRef hint).
fn named_sub(name: &str, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Subroutine {
            name: Some(name.to_string()),
            name_span: Some(SourceLocation::new(start, start + name.len())),
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(Node::new(
                NodeKind::Block { statements: vec![] },
                SourceLocation::new(start, end),
            )),
        },
        SourceLocation::new(start, end),
    )
}

/// Build a regex literal node.
fn regex_node(pattern: &str, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Regex {
            pattern: pattern.to_string(),
            replacement: None,
            modifiers: String::new(),
            has_embedded_code: false,
        },
        SourceLocation::new(start, end),
    )
}

/// Build a hash literal node.
fn hash_literal(start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::HashLiteral { pairs: vec![] },
        SourceLocation::new(start, end),
    )
}

/// Build an array literal node.
fn array_literal(elements: Vec<Node>, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::ArrayLiteral { elements },
        SourceLocation::new(start, end),
    )
}

/// Build a variable node.
fn variable(sigil: &str, name: &str, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Variable {
            sigil: sigil.to_string(),
            name: name.to_string(),
        },
        SourceLocation::new(start, end),
    )
}

// ===========================================================================
// InlayHintKind
// ===========================================================================

#[test]
fn hint_kind_type_value() {
    assert_eq!(InlayHintKind::Type as u8, 1);
}

#[test]
fn hint_kind_parameter_value() {
    assert_eq!(InlayHintKind::Parameter as u8, 2);
}

#[test]
fn hint_kind_equality() {
    assert_eq!(InlayHintKind::Type, InlayHintKind::Type);
    assert_eq!(InlayHintKind::Parameter, InlayHintKind::Parameter);
    assert_ne!(InlayHintKind::Type, InlayHintKind::Parameter);
}

#[test]
fn hint_kind_clone_copy() {
    let kind = InlayHintKind::Type;
    let cloned = kind;
    assert_eq!(kind, cloned);
}

#[test]
fn hint_kind_debug() {
    let dbg = format!("{:?}", InlayHintKind::Parameter);
    assert!(dbg.contains("Parameter"));
}

// ===========================================================================
// InlayHint struct
// ===========================================================================

#[test]
fn inlay_hint_clone() {
    let hint = InlayHint {
        position: Position::new(1, 2),
        label: "test:".to_string(),
        kind: InlayHintKind::Parameter,
        padding_left: false,
        padding_right: true,
        tooltip: None,
        location: None,
    };
    let cloned = hint.clone();
    assert_eq!(cloned.label, "test:");
    assert_eq!(cloned.kind, InlayHintKind::Parameter);
    assert!(cloned.padding_right);
    assert!(!cloned.padding_left);
}

#[test]
fn inlay_hint_debug() {
    let hint = InlayHint {
        position: Position::new(0, 0),
        label: ": Num".to_string(),
        kind: InlayHintKind::Type,
        padding_left: true,
        padding_right: false,
        tooltip: None,
        location: None,
    };
    let dbg = format!("{:?}", hint);
    assert!(dbg.contains("Num"));
    assert!(dbg.contains("Type"));
}

// ===========================================================================
// InlayHintsProvider — construction
// ===========================================================================

#[test]
fn provider_new() {
    let _provider = InlayHintsProvider::new();
}

#[test]
fn provider_default() {
    let _provider: InlayHintsProvider = Default::default();
}

// ===========================================================================
// Empty / trivial ASTs
// ===========================================================================

#[test]
fn empty_program_generates_no_hints() {
    let provider = InlayHintsProvider::new();
    let ast = program(vec![]);
    let hints = provider.generate_hints(&ast, &identity_pos, None);
    assert!(hints.is_empty());
}

#[test]
fn parameter_hints_empty_program() {
    let ast = program(vec![]);
    let hints = parameter_hints(&ast, &identity_pos, None);
    assert!(hints.is_empty());
}

#[test]
fn trivial_type_hints_empty_program() {
    let ast = program(vec![]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert!(hints.is_empty());
}

// ===========================================================================
// Parameter hints — supported functions
// ===========================================================================

#[test]
fn parameter_hints_substr() {
    let args = vec![
        string("hello", false, 10, 17),
        number("0", 19, 20),
        number("3", 22, 23),
    ];
    let call = func_call("substr", args, SourceLocation::new(3, 24));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 3);
    assert_eq!(hints[0]["label"].as_str(), Some("expr:"));
    assert_eq!(hints[1]["label"].as_str(), Some("offset:"));
    assert_eq!(hints[2]["label"].as_str(), Some("length:"));
    // All should be kind=2 (parameter)
    for h in &hints {
        assert_eq!(h["kind"].as_u64(), Some(2));
    }
}

#[test]
fn parameter_hints_index_function() {
    let args = vec![string("hello", false, 10, 17), string("ell", false, 19, 24)];
    let call = func_call("index", args, SourceLocation::new(4, 25));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("str:"));
    assert_eq!(hints[1]["label"].as_str(), Some("substr:"));
    // Verify position is from builtin signature
    assert_eq!(hints[0]["kind"].as_u64(), Some(2));
}

#[test]
fn parameter_hints_rindex() {
    let args = vec![
        string("hello", false, 5, 12),
        string("l", false, 14, 17),
        number("3", 19, 20),
    ];
    let call = func_call("rindex", args, SourceLocation::new(0, 21));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 3);
    assert_eq!(hints[0]["label"].as_str(), Some("str:"));
    assert_eq!(hints[1]["label"].as_str(), Some("substr:"));
    assert_eq!(hints[2]["label"].as_str(), Some("position:"));
}

#[test]
fn parameter_hints_sprintf() {
    let args = vec![string("%s=%d", false, 10, 17), string("x", false, 19, 22)];
    let call = func_call("sprintf", args, SourceLocation::new(2, 23));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("format:"));
    assert_eq!(hints[1]["label"].as_str(), Some("list:"));
}

#[test]
fn parameter_hints_printf() {
    let args = vec![string("%s", false, 10, 14), string("x", false, 16, 19)];
    let call = func_call("printf", args, SourceLocation::new(3, 20));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    // printf's first (most complete) signature is: printf FILEHANDLE FORMAT, LIST
    assert_eq!(hints[0]["label"].as_str(), Some("filehandle:"));
    assert_eq!(hints[1]["label"].as_str(), Some("format:"));
}

#[test]
fn parameter_hints_join() {
    let args = vec![string(",", false, 5, 8), variable("@", "arr", 10, 14)];
    let call = func_call("join", args, SourceLocation::new(0, 15));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("expr:"));
    assert_eq!(hints[1]["label"].as_str(), Some("list:"));
}

#[test]
fn parameter_hints_split() {
    let args = vec![
        regex_node("/,/", 6, 9),
        string("a,b,c", false, 11, 18),
        number("3", 20, 21),
    ];
    let call = func_call("split", args, SourceLocation::new(0, 22));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 3);
    assert_eq!(hints[0]["label"].as_str(), Some("pattern:"));
    assert_eq!(hints[1]["label"].as_str(), Some("expr:"));
    assert_eq!(hints[2]["label"].as_str(), Some("limit:"));
}

#[test]
fn parameter_hints_splice() {
    let args = vec![
        variable("@", "arr", 7, 11),
        number("0", 13, 14),
        number("2", 16, 17),
    ];
    let call = func_call("splice", args, SourceLocation::new(0, 18));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 3);
    assert_eq!(hints[0]["label"].as_str(), Some("array:"));
    assert_eq!(hints[1]["label"].as_str(), Some("offset:"));
    assert_eq!(hints[2]["label"].as_str(), Some("length:"));
}

#[test]
fn parameter_hints_unpack() {
    let args = vec![string("A4", false, 7, 11), variable("$", "data", 13, 18)];
    let call = func_call("unpack", args, SourceLocation::new(0, 19));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("template:"));
    assert_eq!(hints[1]["label"].as_str(), Some("expr:"));
}

#[test]
fn parameter_hints_pack() {
    let args = vec![string("A4", false, 5, 9), variable("@", "data", 11, 16)];
    let call = func_call("pack", args, SourceLocation::new(0, 17));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("template:"));
    assert_eq!(hints[1]["label"].as_str(), Some("list:"));
}

#[test]
fn parameter_hints_grep() {
    let args = vec![anon_sub(5, 15), variable("@", "items", 17, 23)];
    let call = func_call("grep", args, SourceLocation::new(0, 24));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("block:"));
    assert_eq!(hints[1]["label"].as_str(), Some("list:"));
}

#[test]
fn parameter_hints_map() {
    let args = vec![anon_sub(4, 14), variable("@", "list", 16, 21)];
    let call = func_call("map", args, SourceLocation::new(0, 22));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("block:"));
    assert_eq!(hints[1]["label"].as_str(), Some("list:"));
}

#[test]
fn parameter_hints_sort() {
    let args = vec![anon_sub(5, 15), variable("@", "data", 17, 22)];
    let call = func_call("sort", args, SourceLocation::new(0, 23));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("block:"));
    assert_eq!(hints[1]["label"].as_str(), Some("list:"));
}

#[test]
fn parameter_hints_push() {
    let args = vec![variable("@", "arr", 5, 9), string("x", false, 11, 14)];
    let call = func_call("push", args, SourceLocation::new(0, 15));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("array:"));
    assert_eq!(hints[1]["label"].as_str(), Some("list:"));
}

#[test]
fn parameter_hints_open() {
    let args = vec![
        variable("$", "fh", 5, 8),
        string("<", false, 10, 13),
        string("file.txt", false, 15, 25),
    ];
    let call = func_call("open", args, SourceLocation::new(0, 26));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 3);
    assert_eq!(hints[0]["label"].as_str(), Some("filehandle:"));
    assert_eq!(hints[1]["label"].as_str(), Some("mode:"));
    assert_eq!(hints[2]["label"].as_str(), Some("filename:"));
}

// ===========================================================================
// Parameter hints — unknown functions produce no hints
// ===========================================================================

#[test]
fn parameter_hints_unknown_function() {
    let args = vec![number("1", 8, 9), number("2", 11, 12)];
    let call = func_call("my_func", args, SourceLocation::new(0, 13));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert!(hints.is_empty());
}

// ===========================================================================
// Parameter hints — more args than signature labels
// ===========================================================================

#[test]
fn parameter_hints_extra_args_ignored() {
    // join has 2 labels: ["expr", "list"] — 3rd arg should be skipped
    let args = vec![
        string(",", false, 5, 8),
        variable("@", "a", 10, 12),
        variable("@", "b", 14, 16),
    ];
    let call = func_call("join", args, SourceLocation::new(0, 17));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
}

// ===========================================================================
// Parameter hints — fewer args than signature labels
// ===========================================================================

#[test]
fn parameter_hints_fewer_args() {
    // split has 3 labels but we pass only 1
    let args = vec![regex_node("/,/", 6, 9)];
    let call = func_call("split", args, SourceLocation::new(0, 10));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some("pattern:"));
}

// ===========================================================================
// Parameter hints — padding
// ===========================================================================

#[test]
fn parameter_hints_padding() {
    let args = vec![string(",", false, 5, 8), variable("@", "arr", 10, 14)];
    let call = func_call("join", args, SourceLocation::new(0, 15));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["paddingLeft"].as_bool(), Some(false));
    assert_eq!(hints[0]["paddingRight"].as_bool(), Some(true));
}

// ===========================================================================
// Trivial type hints — each literal kind
// ===========================================================================

#[test]
fn type_hint_number() {
    let ast = program(vec![expr_stmt(number("42", 0, 2))]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Num"));
    assert_eq!(hints[0]["kind"].as_u64(), Some(1));
}

#[test]
fn type_hint_string() {
    let ast = program(vec![expr_stmt(string("hello", false, 0, 7))]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Str"));
}

#[test]
fn type_hint_interpolated_string() {
    let ast = program(vec![expr_stmt(string("hello $x", true, 0, 10))]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Str"));
}

#[test]
fn type_hint_hash_literal() {
    let ast = program(vec![expr_stmt(hash_literal(0, 10))]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Hash"));
}

#[test]
fn type_hint_array_literal() {
    let ast = program(vec![expr_stmt(array_literal(vec![], 0, 5))]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Array"));
}

#[test]
fn type_hint_regex() {
    let ast = program(vec![expr_stmt(regex_node("foo", 0, 5))]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Regex"));
}

#[test]
fn type_hint_anonymous_sub_coderef() {
    let ast = program(vec![expr_stmt(anon_sub(0, 15))]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": CodeRef"));
}

#[test]
fn type_hint_named_sub_no_coderef() {
    let ast = program(vec![named_sub("foo", 0, 20)]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert!(hints.is_empty());
}

// ===========================================================================
// Trivial type hints — padding
// ===========================================================================

#[test]
fn type_hints_padding() {
    let ast = program(vec![expr_stmt(number("1", 0, 1))]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["paddingLeft"].as_bool(), Some(true));
    assert_eq!(hints[0]["paddingRight"].as_bool(), Some(false));
}

// ===========================================================================
// Trivial type hints — position mapping
// ===========================================================================

#[test]
fn type_hint_uses_end_position() {
    let ast = program(vec![expr_stmt(number("42", 5, 7))]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    // identity_pos maps end offset 7 → (0, 7)
    assert_eq!(hints[0]["position"]["line"].as_u64(), Some(0));
    assert_eq!(hints[0]["position"]["character"].as_u64(), Some(7));
}

// ===========================================================================
// Variable node — no type hint
// ===========================================================================

#[test]
fn variable_no_type_hint() {
    let ast = program(vec![expr_stmt(variable("$", "x", 0, 2))]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert!(hints.is_empty());
}

// ===========================================================================
// generate_hints — combines parameter + type hints
// ===========================================================================

#[test]
fn generate_hints_combines_both() {
    let provider = InlayHintsProvider::new();
    // substr call with a number literal arg → parameter hint AND type hint
    let args = vec![string("hello", false, 10, 17), number("0", 19, 20)];
    let call = func_call("substr", args, SourceLocation::new(3, 21));
    let ast = program(vec![expr_stmt(call)]);

    let hints = provider.generate_hints(&ast, &identity_pos, None);
    // 2 parameter hints (str, offset) + type hints for string and number literals
    assert!(hints.len() >= 2);
}

// ===========================================================================
// Range filtering — parameter hints
// ===========================================================================

#[test]
fn parameter_hints_with_range_includes_in_range() {
    let args = vec![string(",", false, 10, 13), variable("@", "arr", 15, 19)];
    let call = func_call("join", args, SourceLocation::new(3, 20));
    let ast = program(vec![expr_stmt(call)]);

    // identity_pos maps offset 10 → (0, 10)
    let range = Range::new(Position::new(0, 0), Position::new(0, 20));
    let hints = parameter_hints(&ast, &identity_pos, Some(range));
    assert_eq!(hints.len(), 2);
}

#[test]
fn parameter_hints_with_range_excludes_out_of_range() {
    let args = vec![string(",", false, 10, 13), variable("@", "arr", 15, 19)];
    let call = func_call("join", args, SourceLocation::new(3, 20));
    let ast = program(vec![expr_stmt(call)]);

    // identity_pos maps offset 10 → (0, 10), range ends before that
    let range = Range::new(Position::new(0, 0), Position::new(0, 5));
    let hints = parameter_hints(&ast, &identity_pos, Some(range));
    assert!(hints.is_empty());
}

// ===========================================================================
// Range filtering — type hints
// ===========================================================================

#[test]
fn type_hints_with_range_includes_in_range() {
    let ast = program(vec![expr_stmt(number("42", 5, 7))]);
    // identity_pos maps end=7 → (0, 7)
    let range = Range::new(Position::new(0, 0), Position::new(0, 20));
    let hints = trivial_type_hints(&ast, &identity_pos, Some(range));
    assert_eq!(hints.len(), 1);
}

#[test]
fn type_hints_with_range_excludes_out_of_range() {
    let ast = program(vec![expr_stmt(number("42", 5, 7))]);
    // identity_pos maps end=7 → (0, 7), range ends at 5
    let range = Range::new(Position::new(0, 0), Position::new(0, 5));
    let hints = trivial_type_hints(&ast, &identity_pos, Some(range));
    assert!(hints.is_empty());
}

#[test]
fn type_hints_range_exact_boundary_excluded() {
    // pos_in_range excludes pos.character >= range.end.character on the same line
    let ast = program(vec![expr_stmt(number("42", 5, 7))]);
    // hint at (0,7), range end at (0,7) → excluded
    let range = Range::new(Position::new(0, 0), Position::new(0, 7));
    let hints = trivial_type_hints(&ast, &identity_pos, Some(range));
    assert!(hints.is_empty());
}

#[test]
fn type_hints_range_just_inside() {
    let ast = program(vec![expr_stmt(number("42", 5, 7))]);
    // hint at (0,7), range end at (0,8) → included
    let range = Range::new(Position::new(0, 0), Position::new(0, 8));
    let hints = trivial_type_hints(&ast, &identity_pos, Some(range));
    assert_eq!(hints.len(), 1);
}

// ===========================================================================
// Range filtering — multi-line position mapper
// ===========================================================================

#[test]
fn range_filtering_multiline() {
    // Position mapper that puts offset 10 on line 2
    let multiline_pos = |offset: usize| -> (u32, u32) {
        if offset < 10 {
            (0, offset as u32)
        } else {
            (2, (offset - 10) as u32)
        }
    };

    let ast = program(vec![expr_stmt(number("42", 10, 12))]);
    // Hint at line 2, char 2 (end offset 12 maps to (2, 2))
    // Range covers only line 0
    let range = Range::new(Position::new(0, 0), Position::new(1, 0));
    let hints = trivial_type_hints(&ast, &multiline_pos, Some(range));
    assert!(hints.is_empty());

    // Range covers line 2
    let range2 = Range::new(Position::new(0, 0), Position::new(3, 0));
    let hints2 = trivial_type_hints(&ast, &multiline_pos, Some(range2));
    assert_eq!(hints2.len(), 1);
}

// ===========================================================================
// generate_hints with range
// ===========================================================================

#[test]
fn generate_hints_respects_range() {
    let provider = InlayHintsProvider::new();
    let ast = program(vec![expr_stmt(number("42", 5, 7))]);

    // Out of range → no hints
    let range_out = Range::new(Position::new(0, 0), Position::new(0, 3));
    let hints = provider.generate_hints(&ast, &identity_pos, Some(range_out));
    assert!(hints.is_empty());

    // In range → some hints
    let range_in = Range::new(Position::new(0, 0), Position::new(0, 20));
    let hints2 = provider.generate_hints(&ast, &identity_pos, Some(range_in));
    assert!(!hints2.is_empty());
}

// ===========================================================================
// InlayHintsProvider method consistency
// ===========================================================================

#[test]
fn provider_methods_match_free_functions() {
    let provider = InlayHintsProvider::new();
    let args = vec![string("hello", false, 10, 17), number("0", 19, 20)];
    let call = func_call("substr", args, SourceLocation::new(3, 21));
    let ast = program(vec![expr_stmt(call)]);

    let method_hints = provider.parameter_hints(&ast, &identity_pos, None);
    let method_type_hints = provider.trivial_type_hints(&ast, &identity_pos, None);
    let combined = provider.generate_hints(&ast, &identity_pos, None);

    assert_eq!(combined.len(), method_hints.len() + method_type_hints.len());
}

// ===========================================================================
// Multiple statements
// ===========================================================================

#[test]
fn multiple_statements_all_hinted() {
    let ast = program(vec![
        expr_stmt(number("1", 0, 1)),
        expr_stmt(string("hi", false, 3, 7)),
        expr_stmt(regex_node("x", 9, 12)),
    ]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 3);

    let labels: Vec<_> = hints.iter().filter_map(|h| h["label"].as_str()).collect();
    assert!(labels.contains(&": Num"));
    assert!(labels.contains(&": Str"));
    assert!(labels.contains(&": Regex"));
}

// ===========================================================================
// Multiple function calls
// ===========================================================================

#[test]
fn multiple_function_calls() {
    let call1 = func_call(
        "join",
        vec![string(",", false, 5, 8), variable("@", "a", 10, 12)],
        SourceLocation::new(0, 13),
    );
    let call2 = func_call(
        "split",
        vec![regex_node("/;/", 20, 23), string("data", false, 25, 31)],
        SourceLocation::new(15, 32),
    );
    let ast = program(vec![expr_stmt(call1), expr_stmt(call2)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    // join: 2 hints + split: 2 hints = 4
    assert_eq!(hints.len(), 4);
}

// ===========================================================================
// Nested function calls
// ===========================================================================

#[test]
fn nested_function_calls_no_crash() {
    // substr(join(",", @arr), 0, 5)
    // inner: join(",", @arr)
    let inner = func_call(
        "join",
        vec![string(",", false, 12, 15), variable("@", "arr", 17, 21)],
        SourceLocation::new(7, 22),
    );
    // outer: substr(<inner>, 0, 5)
    let outer = func_call(
        "substr",
        vec![inner, number("0", 24, 25), number("5", 27, 28)],
        SourceLocation::new(0, 29),
    );
    let ast = program(vec![expr_stmt(outer)]);

    // Should not panic and should produce hints for both calls
    let hints = parameter_hints(&ast, &identity_pos, None);
    // substr has 3 args, but first arg is a FunctionCall (join) which itself has 2 args
    // The walker visits the outer call first, then walks into args (which includes join)
    assert!(hints.len() >= 3);
}

// ===========================================================================
// Edge case: zero-length location
// ===========================================================================

#[test]
fn zero_length_location() {
    let ast = program(vec![expr_stmt(number("0", 5, 5))]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    // Should still produce a hint, just at the same position
    assert_eq!(hints.len(), 1);
}

// ===========================================================================
// Edge case: large offsets
// ===========================================================================

#[test]
fn large_offsets() {
    let big_offset = 1_000_000;
    let ast = program(vec![expr_stmt(number("42", big_offset, big_offset + 2))]);

    let large_pos = |offset: usize| -> (u32, u32) { (0, offset as u32) };
    let hints = trivial_type_hints(&ast, &large_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(
        hints[0]["position"]["character"].as_u64(),
        Some((big_offset + 2) as u64)
    );
}

// ===========================================================================
// Edge case: push with column 4 adjustment
// ===========================================================================

#[test]
fn push_parameter_positions() {
    // Verify push parameter hints use the correct position from arg location
    let args = vec![variable("@", "arr", 5, 9), string("val", false, 11, 16)];
    let call = func_call("push", args, SourceLocation::new(0, 17));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["position"]["character"].as_u64(), Some(5));
    assert_eq!(hints[0]["label"].as_str(), Some("array:"));
    assert_eq!(hints[1]["position"]["character"].as_u64(), Some(11));
    assert_eq!(hints[1]["label"].as_str(), Some("list:"));
}

#[test]
fn push_parameter_positions_different_offset() {
    // Verify positions track actual arg locations
    let args = vec![variable("@", "arr", 6, 10), string("val", false, 12, 17)];
    let call = func_call("push", args, SourceLocation::new(0, 18));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["position"]["character"].as_u64(), Some(6));
}

// ===========================================================================
// InlayHintsProvider::generate_hints — InlayHint fields
// ===========================================================================

#[test]
fn generate_hints_returns_correct_inlay_hint_fields() {
    let provider = InlayHintsProvider::new();
    let ast = program(vec![expr_stmt(number("42", 5, 7))]);
    let hints = provider.generate_hints(&ast, &identity_pos, None);

    // Should have at least the type hint for the number
    let type_hints: Vec<_> = hints
        .iter()
        .filter(|h| h.kind == InlayHintKind::Type)
        .collect();
    assert!(!type_hints.is_empty());

    let h = &type_hints[0];
    assert_eq!(h.label, ": Num");
    assert_eq!(h.position.line, 0);
    assert_eq!(h.position.character, 7);
    assert!(h.padding_left);
    assert!(!h.padding_right);
}

#[test]
fn generate_hints_parameter_hint_fields() {
    let provider = InlayHintsProvider::new();
    let args = vec![string(",", false, 5, 8), variable("@", "arr", 10, 14)];
    let call = func_call("join", args, SourceLocation::new(0, 15));
    let ast = program(vec![expr_stmt(call)]);

    let hints = provider.generate_hints(&ast, &identity_pos, None);
    let param_hints: Vec<_> = hints
        .iter()
        .filter(|h| h.kind == InlayHintKind::Parameter)
        .collect();
    assert!(!param_hints.is_empty());

    let h = &param_hints[0];
    assert_eq!(h.label, "expr:");
    assert!(!h.padding_left);
    assert!(h.padding_right);
}

// ===========================================================================
// Range edge cases for pos_in_range (tested indirectly)
// ===========================================================================

#[test]
fn range_start_boundary() {
    // Hint exactly at range start should be included
    let ast = program(vec![expr_stmt(number("1", 5, 7))]);
    // hint at (0,7)
    let range = Range::new(Position::new(0, 7), Position::new(0, 10));
    let hints = trivial_type_hints(&ast, &identity_pos, Some(range));
    assert_eq!(hints.len(), 1);
}

#[test]
fn range_before_start_excluded() {
    let ast = program(vec![expr_stmt(number("1", 5, 7))]);
    // hint at (0,7), range starts at (0,8)
    let range = Range::new(Position::new(0, 8), Position::new(0, 20));
    let hints = trivial_type_hints(&ast, &identity_pos, Some(range));
    assert!(hints.is_empty());
}

#[test]
fn range_different_line_before() {
    // hint on line 0, range starts at line 1
    let ast = program(vec![expr_stmt(number("1", 5, 7))]);
    let range = Range::new(Position::new(1, 0), Position::new(2, 0));
    let hints = trivial_type_hints(&ast, &identity_pos, Some(range));
    assert!(hints.is_empty());
}

#[test]
fn range_different_line_after() {
    // hint on line 0, range ends at line 0 but we need line > end_line
    let pos_line2 = |offset: usize| -> (u32, u32) { (2, offset as u32) };
    let ast = program(vec![expr_stmt(number("1", 5, 7))]);
    // hint on line 2, range ends at line 1
    let range = Range::new(Position::new(0, 0), Position::new(1, 100));
    let hints = trivial_type_hints(&ast, &pos_line2, Some(range));
    assert!(hints.is_empty());
}

// ===========================================================================
// No panics on unusual AST shapes
// ===========================================================================

#[test]
fn undef_node_no_hints() {
    let ast = program(vec![expr_stmt(Node::new(
        NodeKind::Undef,
        SourceLocation::new(0, 5),
    ))]);
    let param_h = parameter_hints(&ast, &identity_pos, None);
    let type_h = trivial_type_hints(&ast, &identity_pos, None);
    assert!(param_h.is_empty());
    assert!(type_h.is_empty());
}

#[test]
fn identifier_node_no_hints() {
    let node = Node::new(
        NodeKind::Identifier {
            name: "foo".to_string(),
        },
        SourceLocation::new(0, 3),
    );
    let ast = program(vec![expr_stmt(node)]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert!(hints.is_empty());
}

// ===========================================================================
// Stress: many statements
// ===========================================================================

#[test]
fn many_statements_performance() {
    let stmts: Vec<Node> = (0..1000)
        .map(|i| {
            let start = i * 10;
            let end = start + 5;
            expr_stmt(number(&i.to_string(), start, end))
        })
        .collect();
    let ast = program(stmts);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1000);
}

// ===========================================================================
// Stress: many function calls
// ===========================================================================

#[test]
fn many_function_calls_performance() {
    let stmts: Vec<Node> = (0..500)
        .map(|i| {
            let start = i * 20;
            let args = vec![
                string(",", false, start + 5, start + 8),
                variable("@", "a", start + 10, start + 12),
            ];
            expr_stmt(func_call(
                "join",
                args,
                SourceLocation::new(start, start + 13),
            ))
        })
        .collect();
    let ast = program(stmts);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1000); // 500 calls × 2 params each
}

// ===========================================================================
// extract_param_names — unit tests for signature parsing
// ===========================================================================

#[test]
fn extract_params_comma_separated() {
    let params = extract_param_names("open FILEHANDLE, MODE, FILENAME");
    assert_eq!(params, vec!["filehandle", "mode", "filename"]);
}

#[test]
fn extract_params_space_separated() {
    let params = extract_param_names("map BLOCK LIST");
    assert_eq!(params, vec!["block", "list"]);
}

#[test]
fn extract_params_mixed_separators() {
    let params = extract_param_names("printf FILEHANDLE FORMAT, LIST");
    assert_eq!(params, vec!["filehandle", "format", "list"]);
}

#[test]
fn extract_params_slash_delimiters_stripped() {
    let params = extract_param_names("split /PATTERN/, EXPR, LIMIT");
    assert_eq!(params, vec!["pattern", "expr", "limit"]);
}

#[test]
fn extract_params_no_params() {
    let params = extract_param_names("fork");
    assert!(params.is_empty());
}

#[test]
fn extract_params_single_param() {
    let params = extract_param_names("chomp VARIABLE");
    assert_eq!(params, vec!["variable"]);
}

#[test]
fn extract_params_substr() {
    let params = extract_param_names("substr EXPR, OFFSET, LENGTH, REPLACEMENT");
    assert_eq!(params, vec!["expr", "offset", "length", "replacement"]);
}

#[test]
fn extract_params_push() {
    let params = extract_param_names("push ARRAY, LIST");
    assert_eq!(params, vec!["array", "list"]);
}

// ===========================================================================
// Builtin-driven parameter hints — push, open, substr
// ===========================================================================

#[test]
fn builtin_push_shows_array_and_list_hints() {
    // push(@array, $value) → array:, list:
    let args = vec![
        variable("@", "array", 5, 11),
        variable("$", "value", 13, 19),
    ];
    let call = func_call("push", args, SourceLocation::new(0, 20));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("array:"));
    assert_eq!(hints[1]["label"].as_str(), Some("list:"));
    assert_eq!(hints[0]["kind"].as_u64(), Some(2));
    assert_eq!(hints[1]["kind"].as_u64(), Some(2));
}

#[test]
fn builtin_open_shows_filehandle_mode_filename_hints() {
    // open(my $fh, '<', $file) → filehandle:, mode:, filename:
    let args = vec![
        variable("$", "fh", 5, 8),
        string("<", false, 10, 13),
        variable("$", "file", 15, 20),
    ];
    let call = func_call("open", args, SourceLocation::new(0, 21));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 3);
    assert_eq!(hints[0]["label"].as_str(), Some("filehandle:"));
    assert_eq!(hints[1]["label"].as_str(), Some("mode:"));
    assert_eq!(hints[2]["label"].as_str(), Some("filename:"));
    assert_eq!(hints[0]["kind"].as_u64(), Some(2));
}

#[test]
fn builtin_substr_shows_parameter_name_hints() {
    // substr($str, $offset, $length) → expr:, offset:, length:
    let args = vec![
        variable("$", "str", 7, 11),
        variable("$", "offset", 13, 20),
        variable("$", "length", 22, 29),
    ];
    let call = func_call("substr", args, SourceLocation::new(0, 30));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 3);
    assert_eq!(hints[0]["label"].as_str(), Some("expr:"));
    assert_eq!(hints[1]["label"].as_str(), Some("offset:"));
    assert_eq!(hints[2]["label"].as_str(), Some("length:"));
}

// ===========================================================================
// Builtin-driven hints — additional coverage for dynamic signature lookup
// ===========================================================================

#[test]
fn builtin_bless_shows_ref_classname_hints() {
    // bless($ref, "MyClass") → ref:, classname:
    let args = vec![
        variable("$", "ref", 6, 10),
        string("MyClass", false, 12, 21),
    ];
    let call = func_call("bless", args, SourceLocation::new(0, 22));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("ref:"));
    assert_eq!(hints[1]["label"].as_str(), Some("classname:"));
}

#[test]
fn builtin_rename_shows_oldname_newname_hints() {
    // rename($old, $new) → oldname:, newname:
    let args = vec![variable("$", "old", 7, 11), variable("$", "new", 13, 17)];
    let call = func_call("rename", args, SourceLocation::new(0, 18));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("oldname:"));
    assert_eq!(hints[1]["label"].as_str(), Some("newname:"));
}

#[test]
fn single_param_builtins_skip_hints() {
    // Functions with only 1 parameter (like chomp, defined, etc.)
    // should not produce hints to reduce noise
    let args = vec![variable("$", "x", 6, 8)];
    let call = func_call("chomp", args, SourceLocation::new(0, 9));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert!(
        hints.is_empty(),
        "Single-param builtins should not produce hints"
    );
}

#[test]
fn no_param_builtins_skip_hints() {
    // Functions with no parameters (like fork, time, etc.)
    // should not produce hints
    let args: Vec<Node> = vec![];
    let call = func_call("fork", args, SourceLocation::new(0, 4));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert!(hints.is_empty());
}

#[test]
fn builtin_atan2_shows_y_x_hints() {
    // atan2($y, $x) → y:, x:
    let args = vec![variable("$", "y", 6, 8), variable("$", "x", 10, 12)];
    let call = func_call("atan2", args, SourceLocation::new(0, 13));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("y:"));
    assert_eq!(hints[1]["label"].as_str(), Some("x:"));
}
