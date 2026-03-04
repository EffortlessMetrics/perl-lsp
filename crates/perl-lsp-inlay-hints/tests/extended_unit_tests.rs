//! Extended unit tests for `perl-lsp-inlay-hints`.
//!
//! Additional comprehensive tests covering edge cases, boundary conditions,
//! complex scenarios, and advanced use cases for inlay hints generation.

use perl_lsp_inlay_hints::{
    InlayHint, InlayHintKind, InlayHintsProvider, parameter_hints, trivial_type_hints,
};
use perl_parser_core::ast::{Node, NodeKind, SourceLocation};
use perl_position_tracking::{WirePosition as Position, WireRange as Range};

// ---------------------------------------------------------------------------
// Test Helpers
// ---------------------------------------------------------------------------

/// Identity position mapper: byte offset → (line=0, character=offset).
fn identity_pos(offset: usize) -> (u32, u32) {
    (0, offset as u32)
}

/// Multi-line position mapper: tracks lines.
fn multiline_pos(offset: usize) -> (u32, u32) {
    let lines = offset / 40;
    let chars = offset % 40;
    (lines as u32, chars as u32)
}

/// Build a minimal AST program wrapping the given statements.
fn program(stmts: Vec<Node>) -> Node {
    Node::new(NodeKind::Program { statements: stmts }, SourceLocation::new(0, 1000))
}

/// Build a function-call node.
fn func_call(name: &str, args: Vec<Node>, loc: SourceLocation) -> Node {
    Node::new(NodeKind::FunctionCall { name: name.to_string(), args }, loc)
}

/// Build an expression-statement wrapping an inner node.
fn expr_stmt(inner: Node) -> Node {
    let loc = inner.location;
    Node::new(NodeKind::ExpressionStatement { expression: Box::new(inner) }, loc)
}

/// Build a number literal node.
fn number(value: &str, start: usize, end: usize) -> Node {
    Node::new(NodeKind::Number { value: value.to_string() }, SourceLocation::new(start, end))
}

/// Build a string literal node.
fn string(value: &str, interpolated: bool, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::String { value: value.to_string(), interpolated },
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
    Node::new(NodeKind::HashLiteral { pairs: vec![] }, SourceLocation::new(start, end))
}

/// Build an array literal node.
fn array_literal(elements: Vec<Node>, start: usize, end: usize) -> Node {
    Node::new(NodeKind::ArrayLiteral { elements }, SourceLocation::new(start, end))
}

/// Build a variable node.
fn variable(sigil: &str, name: &str, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Variable { sigil: sigil.to_string(), name: name.to_string() },
        SourceLocation::new(start, end),
    )
}

// ===========================================================================
// Extended Tests: Parameter Hints Edge Cases
// ===========================================================================

#[test]
fn parameter_hints_split_with_three_args() {
    let args =
        vec![regex_node("\\s+", 10, 15), string("hello world", false, 17, 30), number("2", 32, 33)];
    let call = func_call("split", args, SourceLocation::new(5, 34));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 3);
    assert_eq!(hints[0]["label"].as_str(), Some("pattern:"));
    assert_eq!(hints[1]["label"].as_str(), Some("str:"));
    assert_eq!(hints[2]["label"].as_str(), Some("limit:"));
}

#[test]
fn parameter_hints_splice_all_args() {
    let args = vec![
        variable("@", "arr", 10, 14),
        number("0", 16, 17),
        number("2", 19, 20),
        string("new", false, 22, 27),
    ];
    let call = func_call("splice", args, SourceLocation::new(5, 28));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 4);
    assert_eq!(hints[0]["label"].as_str(), Some("array:"));
    assert_eq!(hints[1]["label"].as_str(), Some("offset:"));
    assert_eq!(hints[2]["label"].as_str(), Some("length:"));
    assert_eq!(hints[3]["label"].as_str(), Some("list:"));
}

#[test]
fn parameter_hints_unpack_two_args() {
    let args = vec![string("A4", false, 10, 14), variable("$", "data", 16, 21)];
    let call = func_call("unpack", args, SourceLocation::new(5, 22));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("template:"));
    assert_eq!(hints[1]["label"].as_str(), Some("expr:"));
}

#[test]
fn parameter_hints_grep_block_and_list() {
    let args = vec![
        anon_sub(10, 25),
        array_literal(vec![number("1", 27, 28), number("2", 30, 31)], 27, 32),
    ];
    let call = func_call("grep", args, SourceLocation::new(5, 33));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("block:"));
    assert_eq!(hints[1]["label"].as_str(), Some("list:"));
}

#[test]
fn parameter_hints_map_block_and_list() {
    let args = vec![
        anon_sub(10, 25),
        array_literal(vec![number("1", 27, 28), number("2", 30, 31)], 27, 32),
    ];
    let call = func_call("map", args, SourceLocation::new(5, 33));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("block:"));
    assert_eq!(hints[1]["label"].as_str(), Some("list:"));
}

#[test]
fn parameter_hints_sort_with_block() {
    let args = vec![
        anon_sub(10, 25),
        array_literal(vec![number("3", 27, 28), number("1", 30, 31)], 27, 32),
    ];
    let call = func_call("sort", args, SourceLocation::new(5, 33));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("block:"));
    assert_eq!(hints[1]["label"].as_str(), Some("list:"));
}

#[test]
fn parameter_hints_join_sep_and_list() {
    let args =
        vec![string(",", false, 10, 13), array_literal(vec![string("a", false, 15, 18)], 15, 19)];
    let call = func_call("join", args, SourceLocation::new(5, 20));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("sep:"));
    assert_eq!(hints[1]["label"].as_str(), Some("list:"));
}

#[test]
fn parameter_hints_open_filehandle_mode_expr() {
    let args = vec![
        variable("*", "FH", 10, 13),
        string(">", false, 15, 18),
        string("file.txt", false, 20, 30),
    ];
    let call = func_call("open", args, SourceLocation::new(5, 31));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 3);
    assert_eq!(hints[0]["label"].as_str(), Some("FILEHANDLE:"));
    assert_eq!(hints[1]["label"].as_str(), Some("MODE:"));
    assert_eq!(hints[2]["label"].as_str(), Some("EXPR:"));
}

#[test]
fn parameter_hints_unknown_function_no_hints() {
    let args = vec![string("x", false, 10, 13)];
    let call = func_call("unknown_func", args, SourceLocation::new(5, 14));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert!(hints.is_empty());
}

#[test]
fn parameter_hints_extra_args_stops_at_signature() {
    let args = vec![
        string("hello", false, 10, 17),
        number("0", 19, 20),
        number("3", 22, 23),
        number("99", 25, 27),
        number("99", 29, 31),
    ];
    let call = func_call("substr", args, SourceLocation::new(5, 32));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    // Only 3 hints for the 3 parameters in substr signature
    assert_eq!(hints.len(), 3);
}

#[test]
fn parameter_hints_fewer_args_than_signature() {
    let args = vec![string("hello", false, 10, 17)];
    let call = func_call("substr", args, SourceLocation::new(5, 18));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    // Only 1 hint for the provided 1 argument
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some("str:"));
}

#[test]
fn parameter_hints_rindex_three_args() {
    let args =
        vec![string("hello", false, 10, 17), string("l", false, 19, 22), number("4", 24, 25)];
    let call = func_call("rindex", args, SourceLocation::new(5, 26));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 3);
    assert_eq!(hints[0]["label"].as_str(), Some("str:"));
    assert_eq!(hints[1]["label"].as_str(), Some("substr:"));
    assert_eq!(hints[2]["label"].as_str(), Some("pos:"));
}

// ===========================================================================
// Extended Tests: Type Hints Edge Cases
// ===========================================================================

#[test]
fn trivial_type_hints_number_integer() {
    let node = number("42", 10, 12);
    let call = expr_stmt(node);
    let ast = program(vec![call]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Num"));
    assert_eq!(hints[0]["kind"].as_u64(), Some(1));
}

#[test]
fn trivial_type_hints_number_float() {
    let node = number("3.14", 10, 14);
    let call = expr_stmt(node);
    let ast = program(vec![call]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Num"));
}

#[test]
fn trivial_type_hints_number_negative() {
    let node = number("-42", 10, 13);
    let call = expr_stmt(node);
    let ast = program(vec![call]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Num"));
}

#[test]
fn trivial_type_hints_string_double_quoted() {
    let node = string("hello world", false, 10, 23);
    let call = expr_stmt(node);
    let ast = program(vec![call]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Str"));
}

#[test]
fn trivial_type_hints_string_interpolated() {
    let node = string("hello $name", true, 10, 23);
    let call = expr_stmt(node);
    let ast = program(vec![call]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Str"));
}

#[test]
fn trivial_type_hints_hash_literal() {
    let node = hash_literal(10, 15);
    let call = expr_stmt(node);
    let ast = program(vec![call]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Hash"));
}

#[test]
fn trivial_type_hints_regex() {
    let node = regex_node("\\d+", 10, 15);
    let call = expr_stmt(node);
    let ast = program(vec![call]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Regex"));
}

#[test]
fn trivial_type_hints_anonymous_subroutine() {
    let node = anon_sub(10, 30);
    let call = expr_stmt(node);
    let ast = program(vec![call]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": CodeRef"));
}

#[test]
fn trivial_type_hints_named_subroutine_no_hint() {
    let node = named_sub("my_func", 10, 30);
    let call = expr_stmt(node);
    let ast = program(vec![call]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert!(hints.is_empty());
}

// ===========================================================================
// Extended Tests: Range Filtering
// ===========================================================================

#[test]
fn range_filtering_parameter_hints_in_range() {
    let args = vec![string("hello", false, 10, 17), number("0", 19, 20)];
    let call = func_call("substr", args, SourceLocation::new(5, 21));
    let ast = program(vec![expr_stmt(call)]);

    let range = Range::new(Position::new(0, 10), Position::new(0, 20));
    let hints = parameter_hints(&ast, &identity_pos, Some(range));
    assert_eq!(hints.len(), 2);
}

#[test]
fn range_filtering_parameter_hints_out_of_range_early() {
    let args = vec![string("hello", false, 50, 57), number("0", 59, 60)];
    let call = func_call("substr", args, SourceLocation::new(5, 61));
    let ast = program(vec![expr_stmt(call)]);

    let range = Range::new(Position::new(0, 0), Position::new(0, 30));
    let hints = parameter_hints(&ast, &identity_pos, Some(range));
    assert!(hints.is_empty());
}

#[test]
fn range_filtering_parameter_hints_partial_overlap() {
    let args = vec![string("hello", false, 10, 17), number("0", 19, 20), number("3", 22, 23)];
    let call = func_call("substr", args, SourceLocation::new(5, 24));
    let ast = program(vec![expr_stmt(call)]);

    let range = Range::new(Position::new(0, 15), Position::new(0, 25));
    let hints = parameter_hints(&ast, &identity_pos, Some(range));
    // Should get hints within the range
    assert!(hints.len() > 0);
}

#[test]
fn range_filtering_type_hints_in_range() {
    let node = number("42", 10, 12);
    let call = expr_stmt(node);
    let ast = program(vec![call]);

    let range = Range::new(Position::new(0, 5), Position::new(0, 20));
    let hints = trivial_type_hints(&ast, &identity_pos, Some(range));
    assert_eq!(hints.len(), 1);
}

#[test]
fn range_filtering_type_hints_out_of_range() {
    let node = number("42", 30, 32);
    let call = expr_stmt(node);
    let ast = program(vec![call]);

    let range = Range::new(Position::new(0, 0), Position::new(0, 25));
    let hints = trivial_type_hints(&ast, &identity_pos, Some(range));
    assert!(hints.is_empty());
}

#[test]
fn range_filtering_multiline_in_range() {
    let args = vec![string("hello", false, 10, 17), number("0", 50, 51)];
    let call = func_call("substr", args, SourceLocation::new(5, 52));
    let ast = program(vec![expr_stmt(call)]);

    // Line 1, columns 10-30
    let range = Range::new(Position::new(0, 10), Position::new(1, 30));
    let hints = parameter_hints(&ast, &multiline_pos, Some(range));
    // Should include first arg on line 0
    assert!(hints.len() > 0);
}

#[test]
fn range_filtering_multiline_out_of_range() {
    let node = number("42", 100, 102);
    let call = expr_stmt(node);
    let ast = program(vec![call]);

    let range = Range::new(Position::new(0, 0), Position::new(1, 40));
    let hints = trivial_type_hints(&ast, &multiline_pos, Some(range));
    assert!(hints.is_empty());
}

// ===========================================================================
// Extended Tests: Provider Methods
// ===========================================================================

#[test]
fn provider_generate_hints_combines_parameter_and_type() {
    let args = vec![number("10", 10, 12)];
    let call = func_call("substr", args, SourceLocation::new(5, 13));
    let ast = program(vec![expr_stmt(call)]);

    let provider = InlayHintsProvider::new();
    let hints = provider.generate_hints(&ast, &identity_pos, None);
    // Should have both parameter and type hints
    assert!(hints.len() >= 2);
}

#[test]
fn provider_parameter_hints_method() {
    let args = vec![string("hello", false, 10, 17)];
    let call = func_call("substr", args, SourceLocation::new(5, 18));
    let ast = program(vec![expr_stmt(call)]);

    let provider = InlayHintsProvider::new();
    let hints = provider.parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].kind, InlayHintKind::Parameter);
}

#[test]
fn provider_trivial_type_hints_method() {
    let node = number("42", 10, 12);
    let call = expr_stmt(node);
    let ast = program(vec![call]);

    let provider = InlayHintsProvider::new();
    let hints = provider.trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].kind, InlayHintKind::Type);
}

#[test]
fn provider_generate_hints_with_range() {
    let args = vec![string("hello", false, 10, 17)];
    let call = func_call("substr", args, SourceLocation::new(5, 18));
    let ast = program(vec![expr_stmt(call)]);

    let provider = InlayHintsProvider::new();
    let range = Range::new(Position::new(0, 10), Position::new(0, 20));
    let hints = provider.generate_hints(&ast, &identity_pos, Some(range));
    assert!(!hints.is_empty());
}

// ===========================================================================
// Extended Tests: Multiple Hints in Single Program
// ===========================================================================

#[test]
fn multiple_function_calls_multiple_hints() {
    let args1 = vec![string("hello", false, 10, 17), number("0", 19, 20)];
    let call1 = func_call("substr", args1, SourceLocation::new(5, 21));
    let stmt1 = expr_stmt(call1);

    let args2 = vec![string(",", false, 35, 38), array_literal(vec![], 40, 41)];
    let call2 = func_call("join", args2, SourceLocation::new(30, 42));
    let stmt2 = expr_stmt(call2);

    let ast = program(vec![stmt1, stmt2]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    // substr: 2 hints, join: 2 hints
    assert_eq!(hints.len(), 4);
}

#[test]
fn mixed_parameter_and_type_hints() {
    let args = vec![string("hello", false, 10, 17), number("42", 19, 21)];
    let call = func_call("substr", args, SourceLocation::new(5, 22));
    let stmt = expr_stmt(call);
    let ast = program(vec![stmt]);

    let provider = InlayHintsProvider::new();
    let hints = provider.generate_hints(&ast, &identity_pos, None);

    let param_hints = hints.iter().filter(|h| h.kind == InlayHintKind::Parameter).count();
    let type_hints = hints.iter().filter(|h| h.kind == InlayHintKind::Type).count();

    assert!(param_hints > 0);
    assert!(type_hints > 0);
}

// ===========================================================================
// Extended Tests: Hint Structure Validation
// ===========================================================================

#[test]
fn hint_structure_has_position() {
    let args = vec![string("hello", false, 10, 17)];
    let call = func_call("substr", args, SourceLocation::new(5, 18));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert!(!hints.is_empty());
    assert!(hints[0]["position"].is_object());
}

#[test]
fn hint_structure_has_label() {
    let args = vec![string("hello", false, 10, 17)];
    let call = func_call("substr", args, SourceLocation::new(5, 18));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert!(!hints.is_empty());
    assert!(hints[0]["label"].is_string());
}

#[test]
fn hint_structure_has_kind() {
    let args = vec![string("hello", false, 10, 17)];
    let call = func_call("substr", args, SourceLocation::new(5, 18));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert!(!hints.is_empty());
    assert!(hints[0]["kind"].is_number());
}

#[test]
fn hint_structure_has_padding() {
    let args = vec![string("hello", false, 10, 17)];
    let call = func_call("substr", args, SourceLocation::new(5, 18));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert!(!hints.is_empty());
    assert!(hints[0]["paddingLeft"].is_boolean());
    assert!(hints[0]["paddingRight"].is_boolean());
}

// ===========================================================================
// Extended Tests: Padding Behavior
// ===========================================================================

#[test]
fn parameter_hint_padding_left_false() {
    let args = vec![string("hello", false, 10, 17)];
    let call = func_call("substr", args, SourceLocation::new(5, 18));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints[0]["paddingLeft"].as_bool(), Some(false));
}

#[test]
fn parameter_hint_padding_right_true() {
    let args = vec![string("hello", false, 10, 17)];
    let call = func_call("substr", args, SourceLocation::new(5, 18));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints[0]["paddingRight"].as_bool(), Some(true));
}

#[test]
fn type_hint_padding_left_true() {
    let node = number("42", 10, 12);
    let stmt = expr_stmt(node);
    let ast = program(vec![stmt]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints[0]["paddingLeft"].as_bool(), Some(true));
}

#[test]
fn type_hint_padding_right_false() {
    let node = number("42", 10, 12);
    let stmt = expr_stmt(node);
    let ast = program(vec![stmt]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints[0]["paddingRight"].as_bool(), Some(false));
}

// ===========================================================================
// Extended Tests: Provider Consistency
// ===========================================================================

#[test]
fn provider_new_creates_valid_instance() {
    let provider = InlayHintsProvider::new();
    let ast = program(vec![]);
    let hints = provider.generate_hints(&ast, &identity_pos, None);
    assert!(hints.is_empty());
}

#[test]
fn provider_default_behaves_like_new() {
    let provider1 = InlayHintsProvider::new();
    let provider2: InlayHintsProvider = Default::default();

    let args = vec![string("hello", false, 10, 17)];
    let call = func_call("substr", args, SourceLocation::new(5, 18));
    let ast = program(vec![expr_stmt(call)]);

    let hints1 = provider1.generate_hints(&ast, &identity_pos, None);
    let hints2 = provider2.generate_hints(&ast, &identity_pos, None);

    assert_eq!(hints1.len(), hints2.len());
}

// ===========================================================================
// Extended Tests: Position Accuracy
// ===========================================================================

#[test]
fn position_calculation_identity_mapper() {
    let args = vec![string("hello", false, 10, 17)];
    let call = func_call("substr", args, SourceLocation::new(5, 18));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert!(!hints.is_empty());
    let pos = &hints[0]["position"];
    assert_eq!(pos["line"].as_u64(), Some(0));
}

#[test]
fn position_calculation_multiline_mapper() {
    let args = vec![string("hello", false, 10, 17)];
    let call = func_call("substr", args, SourceLocation::new(5, 18));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &multiline_pos, None);
    assert!(!hints.is_empty());
    let pos = &hints[0]["position"];
    assert!(pos["line"].is_number());
}

// ===========================================================================
// Extended Tests: Label Formatting
// ===========================================================================

#[test]
fn parameter_label_ends_with_colon() {
    let args = vec![string("hello", false, 10, 17)];
    let call = func_call("substr", args, SourceLocation::new(5, 18));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert!(hints[0]["label"].as_str().unwrap_or("").ends_with(':'));
}

#[test]
fn type_label_starts_with_colon_space() {
    let node = number("42", 10, 12);
    let stmt = expr_stmt(node);
    let ast = program(vec![stmt]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert!(hints[0]["label"].as_str().unwrap_or("").starts_with(": "));
}

// ===========================================================================
// Extended Tests: Empty/Zero Arguments
// ===========================================================================

#[test]
fn parameter_hints_no_arguments() {
    let call = func_call("substr", vec![], SourceLocation::new(5, 11));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert!(hints.is_empty());
}

#[test]
fn type_hints_empty_array_literal() {
    let node = array_literal(vec![], 10, 12);
    let stmt = expr_stmt(node);
    let ast = program(vec![stmt]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Array"));
}

#[test]
fn type_hints_empty_hash_literal() {
    let node = hash_literal(10, 12);
    let stmt = expr_stmt(node);
    let ast = program(vec![stmt]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Hash"));
}
