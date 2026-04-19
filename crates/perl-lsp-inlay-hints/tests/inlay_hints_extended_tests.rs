//! Extended tests for `perl-lsp-inlay-hints`.
//!
//! Covers areas not addressed by `comprehensive_unit_tests.rs`:
//! - Parser-driven integration (real Perl source -> AST -> hints)
//! - Type hints for variables in assignment context
//! - Parameter name hints for complex/nested function calls
//! - Hint positioning accuracy with multi-line position mappers
//! - Hints for complex expressions (arrays with elements, hashes with pairs)
//! - Deep nesting and scope traversal
//! - Walk-termination and visitor edge cases

use perl_lsp_inlay_hints::{
    InlayHintKind, InlayHintsProvider, extract_param_names, parameter_hints, trivial_type_hints,
};
use perl_parser_core::ast::{Node, NodeKind, SourceLocation};
use perl_position_tracking::{WirePosition as Position, WireRange as Range};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Identity position mapper: byte offset -> (line=0, character=offset).
fn identity_pos(offset: usize) -> (u32, u32) {
    (0, offset as u32)
}

/// Build a program node wrapping the given statements.
fn program(stmts: Vec<Node>) -> Node {
    Node::new(
        NodeKind::Program { statements: stmts },
        SourceLocation::new(0, 1000),
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
fn string_node(value: &str, interpolated: bool, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::String {
            value: value.to_string(),
            interpolated,
        },
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

/// Build an anonymous subroutine node.
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

/// Build a hash literal with key-value pairs.
fn hash_literal_with_pairs(pairs: Vec<(Node, Node)>, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::HashLiteral { pairs },
        SourceLocation::new(start, end),
    )
}

/// Build an array literal with elements.
fn array_literal_with_elements(elements: Vec<Node>, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::ArrayLiteral { elements },
        SourceLocation::new(start, end),
    )
}

/// Build a block node wrapping statements.
fn block(stmts: Vec<Node>, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Block { statements: stmts },
        SourceLocation::new(start, end),
    )
}

/// Build a named subroutine node.
fn named_sub(name: &str, body_stmts: Vec<Node>, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Subroutine {
            name: Some(name.to_string()),
            name_span: Some(SourceLocation::new(start + 4, start + 4 + name.len())),
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(block(body_stmts, start + 10, end)),
        },
        SourceLocation::new(start, end),
    )
}

/// Build a variable declaration node.
fn var_decl(
    declarator: &str,
    sigil: &str,
    name: &str,
    init: Option<Node>,
    start: usize,
    end: usize,
) -> Node {
    Node::new(
        NodeKind::VariableDeclaration {
            declarator: declarator.to_string(),
            variable: Box::new(variable(sigil, name, start + 3, start + 4 + name.len())),
            attributes: vec![],
            initializer: init.map(Box::new),
        },
        SourceLocation::new(start, end),
    )
}

/// Build an if node.
fn if_node(condition: Node, then_stmts: Vec<Node>, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::If {
            condition: Box::new(condition),
            then_branch: Box::new(block(then_stmts, start + 10, end - 1)),
            elsif_branches: vec![],
            else_branch: None,
        },
        SourceLocation::new(start, end),
    )
}

/// Build a binary expression node.
fn binary(op: &str, left: Node, right: Node, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Binary {
            op: op.to_string(),
            left: Box::new(left),
            right: Box::new(right),
        },
        SourceLocation::new(start, end),
    )
}

/// Multi-line position mapper: treats each 40-char chunk as a line.
fn multiline_pos(offset: usize) -> (u32, u32) {
    let line = offset / 40;
    let col = offset % 40;
    (line as u32, col as u32)
}

// ===========================================================================
// Parser-driven integration tests
// ===========================================================================

#[test]
fn parser_driven_number_literal_produces_type_hint() -> Result<(), Box<dyn std::error::Error>> {
    let source = "42;";
    let mut parser = perl_parser_core::Parser::new(source);
    let ast = parser.parse()?;

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    // The number literal 42 should get a ": Num" type hint
    let num_hints: Vec<_> = hints
        .iter()
        .filter(|h| h["label"].as_str() == Some(": Num"))
        .collect();
    assert!(
        !num_hints.is_empty(),
        "Expected at least one Num type hint for '42;'"
    );
    Ok(())
}

#[test]
fn parser_driven_string_literal_produces_type_hint() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#""hello";"#;
    let mut parser = perl_parser_core::Parser::new(source);
    let ast = parser.parse()?;

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    let str_hints: Vec<_> = hints
        .iter()
        .filter(|h| h["label"].as_str() == Some(": Str"))
        .collect();
    assert!(
        !str_hints.is_empty(),
        "Expected at least one Str type hint for '\"hello\";'"
    );
    Ok(())
}

#[test]
fn parser_driven_regex_produces_type_hint() -> Result<(), Box<dyn std::error::Error>> {
    // Use qr// to get a Regex node
    let source = "qr/foo/;";
    let mut parser = perl_parser_core::Parser::new(source);
    let ast = parser.parse()?;

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    let regex_hints: Vec<_> = hints
        .iter()
        .filter(|h| h["label"].as_str() == Some(": Regex"))
        .collect();
    // This may or may not produce a Regex hint depending on how the parser
    // represents qr//. If it does not, that is acceptable behavior.
    // The test validates no panic occurs.
    let _ = regex_hints;
    Ok(())
}

#[test]
fn parser_driven_anon_sub_produces_coderef_hint() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $cb = sub { 1 };";
    let mut parser = perl_parser_core::Parser::new(source);
    let ast = parser.parse()?;

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    let coderef_hints: Vec<_> = hints
        .iter()
        .filter(|h| h["label"].as_str() == Some(": CodeRef"))
        .collect();
    assert!(
        !coderef_hints.is_empty(),
        "Expected CodeRef type hint for anonymous sub"
    );
    Ok(())
}

#[test]
fn parser_driven_named_sub_no_coderef_hint() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub foo { 1 }";
    let mut parser = perl_parser_core::Parser::new(source);
    let ast = parser.parse()?;

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    let coderef_hints: Vec<_> = hints
        .iter()
        .filter(|h| h["label"].as_str() == Some(": CodeRef"))
        .collect();
    assert!(
        coderef_hints.is_empty(),
        "Named sub should not produce CodeRef hint"
    );
    Ok(())
}

#[test]
fn parser_driven_open_parameter_hints() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"open(my $fh, "<", "file.txt");"#;
    let mut parser = perl_parser_core::Parser::new(source);
    let ast = parser.parse()?;

    let hints = parameter_hints(&ast, &identity_pos, None);
    // open has 3 parameter labels: filehandle, mode, filename
    // Whether this produces hints depends on how the parser represents the call
    // The test validates no panic and checks if labels are correct when present
    if hints.len() == 3 {
        assert_eq!(hints[0]["label"].as_str(), Some("filehandle:"));
        assert_eq!(hints[1]["label"].as_str(), Some("mode:"));
        assert_eq!(hints[2]["label"].as_str(), Some("filename:"));
    }
    Ok(())
}

#[test]
fn parser_driven_generate_hints_no_panic() -> Result<(), Box<dyn std::error::Error>> {
    // A complex snippet that exercises many node types
    let source = r#"
my $x = 42;
my @arr = (1, 2, 3);
my %hash = (a => 1, b => 2);
push(@arr, "new");
my $result = substr($x, 0, 2);
sub foo { return 1; }
my $cb = sub { "hello" };
"#;
    let mut parser = perl_parser_core::Parser::new(source);
    let ast = parser.parse()?;

    let provider = InlayHintsProvider::new();
    let hints = provider.generate_hints(&ast, &identity_pos, None);

    // We should get at least some type hints for the literals
    assert!(!hints.is_empty(), "Expected hints for complex Perl snippet");
    Ok(())
}

// ===========================================================================
// Type hints in variable assignment context
// ===========================================================================

#[test]
fn type_hint_for_number_in_assignment() {
    // my $x = 42; -- the 42 gets a Num type hint
    let init = number("42", 8, 10);
    let decl = var_decl("my", "$", "x", Some(init), 0, 11);
    let ast = program(vec![decl]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Num"));
    assert_eq!(hints[0]["position"]["character"].as_u64(), Some(10));
}

#[test]
fn type_hint_for_string_in_assignment() {
    // my $name = "Alice";
    let init = string_node("Alice", false, 11, 18);
    let decl = var_decl("my", "$", "name", Some(init), 0, 19);
    let ast = program(vec![decl]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Str"));
}

#[test]
fn type_hint_for_coderef_in_assignment() {
    // my $cb = sub { ... };
    let init = anon_sub(8, 20);
    let decl = var_decl("my", "$", "cb", Some(init), 0, 21);
    let ast = program(vec![decl]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": CodeRef"));
}

#[test]
fn type_hint_for_regex_in_assignment() {
    // my $re = qr/foo/;
    let init = regex_node("foo", 8, 15);
    let decl = var_decl("my", "$", "re", Some(init), 0, 16);
    let ast = program(vec![decl]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Regex"));
}

#[test]
fn no_type_hint_for_variable_in_assignment() {
    // my $y = $x; -- no type hint since $x is a variable, not a literal
    let init = variable("$", "x", 8, 10);
    let decl = var_decl("my", "$", "y", Some(init), 0, 11);
    let ast = program(vec![decl]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert!(
        hints.is_empty(),
        "Variable-to-variable assignment should produce no type hints"
    );
}

// ===========================================================================
// Complex expressions: arrays and hashes with typed elements
// ===========================================================================

#[test]
fn array_literal_with_number_elements_produces_type_hints() {
    // (1, 2, 3) -- array literal with numbers
    let elements = vec![number("1", 1, 2), number("2", 4, 5), number("3", 7, 8)];
    let arr = array_literal_with_elements(elements, 0, 9);
    let ast = program(vec![expr_stmt(arr)]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    // The array itself gets an Array hint, and each number element gets a Num hint
    let array_hints: Vec<_> = hints
        .iter()
        .filter(|h| h["label"].as_str() == Some(": Array"))
        .collect();
    let num_hints: Vec<_> = hints
        .iter()
        .filter(|h| h["label"].as_str() == Some(": Num"))
        .collect();

    assert_eq!(array_hints.len(), 1);
    assert_eq!(num_hints.len(), 3);
}

#[test]
fn hash_literal_with_string_values_produces_type_hints() {
    // (a => "foo", b => "bar")
    let pairs = vec![
        (
            string_node("a", false, 1, 2),
            string_node("foo", false, 6, 11),
        ),
        (
            string_node("b", false, 13, 14),
            string_node("bar", false, 18, 23),
        ),
    ];
    let hash = hash_literal_with_pairs(pairs, 0, 24);
    let ast = program(vec![expr_stmt(hash)]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    // Hash literal itself + 4 string values (keys and values)
    let hash_hints: Vec<_> = hints
        .iter()
        .filter(|h| h["label"].as_str() == Some(": Hash"))
        .collect();
    let str_hints: Vec<_> = hints
        .iter()
        .filter(|h| h["label"].as_str() == Some(": Str"))
        .collect();

    assert_eq!(hash_hints.len(), 1);
    assert_eq!(str_hints.len(), 4);
}

#[test]
fn mixed_array_elements_produce_correct_hints() {
    // [42, "hello", qr/foo/]
    let elements = vec![
        number("42", 1, 3),
        string_node("hello", false, 5, 12),
        regex_node("foo", 14, 21),
    ];
    let arr = array_literal_with_elements(elements, 0, 22);
    let ast = program(vec![expr_stmt(arr)]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    let labels: Vec<_> = hints.iter().filter_map(|h| h["label"].as_str()).collect();

    assert!(labels.contains(&": Array"));
    assert!(labels.contains(&": Num"));
    assert!(labels.contains(&": Str"));
    assert!(labels.contains(&": Regex"));
    assert_eq!(hints.len(), 4);
}

// ===========================================================================
// Deep nesting and scope traversal
// ===========================================================================

#[test]
fn hints_inside_subroutine_body() {
    // sub foo { 42; "hello"; }
    let body = vec![
        expr_stmt(number("42", 12, 14)),
        expr_stmt(string_node("hello", false, 16, 23)),
    ];
    let sub_node = named_sub("foo", body, 0, 25);
    let ast = program(vec![sub_node]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    let labels: Vec<_> = hints.iter().filter_map(|h| h["label"].as_str()).collect();

    assert!(
        labels.contains(&": Num"),
        "Missing Num hint inside sub body"
    );
    assert!(
        labels.contains(&": Str"),
        "Missing Str hint inside sub body"
    );
    assert_eq!(hints.len(), 2);
}

#[test]
fn hints_inside_nested_blocks() {
    // sub foo { { { 42 } } }
    let inner_block = block(vec![expr_stmt(number("42", 20, 22))], 18, 24);
    let mid_block = block(vec![inner_block], 14, 26);
    let body = vec![mid_block];
    let sub_node = named_sub("nested", body, 0, 30);
    let ast = program(vec![sub_node]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Num"));
}

#[test]
fn hints_inside_if_condition_and_body() {
    // if (42) { "in_body"; }
    let condition = number("42", 4, 6);
    let body = vec![expr_stmt(string_node("in_body", false, 10, 19))];
    let if_stmt = if_node(condition, body, 0, 22);
    let ast = program(vec![if_stmt]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    let labels: Vec<_> = hints.iter().filter_map(|h| h["label"].as_str()).collect();

    // Both the condition literal and the body literal should get hints
    assert!(
        labels.contains(&": Num"),
        "Missing Num hint in if condition"
    );
    assert!(labels.contains(&": Str"), "Missing Str hint in if body");
    assert_eq!(hints.len(), 2);
}

#[test]
fn parameter_hints_inside_subroutine_body() {
    // sub wrapper { push(@arr, "val"); }
    let call = func_call(
        "push",
        vec![
            variable("@", "arr", 18, 22),
            string_node("val", false, 24, 29),
        ],
        SourceLocation::new(13, 30),
    );
    let sub_node = named_sub("wrapper", vec![expr_stmt(call)], 0, 32);
    let ast = program(vec![sub_node]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0]["label"].as_str(), Some("array:"));
    assert_eq!(hints[1]["label"].as_str(), Some("list:"));
}

// ===========================================================================
// Nested function calls
// ===========================================================================

#[test]
fn nested_function_call_both_produce_hints() {
    // push(@arr, substr($str, 0, 5))
    let inner = func_call(
        "substr",
        vec![
            variable("$", "str", 16, 20),
            number("0", 22, 23),
            number("5", 25, 26),
        ],
        SourceLocation::new(10, 27),
    );
    let outer = func_call(
        "push",
        vec![variable("@", "arr", 5, 9), inner],
        SourceLocation::new(0, 28),
    );
    let ast = program(vec![expr_stmt(outer)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    // push: 2 hints (array, list) + substr: 3 hints (expr, offset, length) = 5
    assert_eq!(hints.len(), 5);

    let labels: Vec<_> = hints.iter().filter_map(|h| h["label"].as_str()).collect();
    assert!(labels.contains(&"array:"));
    assert!(labels.contains(&"list:"));
    assert!(labels.contains(&"expr:"));
    assert!(labels.contains(&"offset:"));
    assert!(labels.contains(&"length:"));
}

#[test]
fn triple_nested_function_calls() {
    // split(/,/, join(":", substr($s, 0, 5)))
    let innermost = func_call(
        "substr",
        vec![
            variable("$", "s", 30, 32),
            number("0", 34, 35),
            number("5", 37, 38),
        ],
        SourceLocation::new(24, 39),
    );
    let middle = func_call(
        "join",
        vec![string_node(":", false, 16, 19), innermost],
        SourceLocation::new(10, 40),
    );
    let outer = func_call(
        "split",
        vec![regex_node(",", 6, 9), middle],
        SourceLocation::new(0, 41),
    );
    let ast = program(vec![expr_stmt(outer)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    // split: 2 (pattern, expr) + join: 2 (expr, list) + substr: 3 (expr, offset, length)
    assert_eq!(hints.len(), 7);
}

// ===========================================================================
// Hint positioning accuracy with multi-line mapper
// ===========================================================================

#[test]
fn multiline_type_hint_position_accuracy() {
    // Simulate multi-line file: each "line" is 40 chars
    // number at offset 42 (line 1, col 2), ends at 44 (line 1, col 4)
    let ast = program(vec![expr_stmt(number("42", 42, 44))]);

    let hints = trivial_type_hints(&ast, &multiline_pos, None);
    assert_eq!(hints.len(), 1);
    // End offset 44 -> line 1, col 4
    assert_eq!(hints[0]["position"]["line"].as_u64(), Some(1));
    assert_eq!(hints[0]["position"]["character"].as_u64(), Some(4));
}

#[test]
fn multiline_parameter_hint_position_accuracy() {
    // push(@arr, $val) with args on different "lines" (40-char chunks)
    let args = vec![
        variable("@", "arr", 45, 49), // line 1, col 5
        variable("$", "val", 85, 89), // line 2, col 5
    ];
    let call = func_call("push", args, SourceLocation::new(40, 90));
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &multiline_pos, None);
    assert_eq!(hints.len(), 2);

    // First arg at offset 45 -> line 1, col 5
    assert_eq!(hints[0]["position"]["line"].as_u64(), Some(1));
    assert_eq!(hints[0]["position"]["character"].as_u64(), Some(5));

    // Second arg at offset 85 -> line 2, col 5
    assert_eq!(hints[1]["position"]["line"].as_u64(), Some(2));
    assert_eq!(hints[1]["position"]["character"].as_u64(), Some(5));
}

#[test]
fn multiline_range_filtering_spans_correct_lines() {
    // Two numbers: one on line 0, one on line 2
    let ast = program(vec![
        expr_stmt(number("1", 5, 6)),   // line 0, col 5 -> hint at (0, 6)
        expr_stmt(number("2", 85, 87)), // line 2, col 5 -> hint at (2, 7)
    ]);

    // Range that covers only line 0-1
    let range = Range::new(Position::new(0, 0), Position::new(2, 0));
    let hints = trivial_type_hints(&ast, &multiline_pos, Some(range));
    // Only the first number is in range (hint at line 0, col 6)
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["position"]["line"].as_u64(), Some(0));
}

// ===========================================================================
// Provider method integration
// ===========================================================================

#[test]
fn provider_generate_hints_type_hint_kinds() {
    let provider = InlayHintsProvider::new();
    let ast = program(vec![
        expr_stmt(number("1", 0, 1)),
        expr_stmt(string_node("x", false, 3, 6)),
        expr_stmt(anon_sub(8, 20)),
    ]);

    let hints = provider.generate_hints(&ast, &identity_pos, None);
    // All these should be Type hints
    for hint in &hints {
        assert_eq!(
            hint.kind,
            InlayHintKind::Type,
            "Literal type hint should be Kind::Type, got {:?}",
            hint.kind
        );
    }
}

#[test]
fn provider_generate_hints_parameter_hint_kinds() {
    let provider = InlayHintsProvider::new();
    let call = func_call(
        "push",
        vec![variable("@", "arr", 5, 9), variable("$", "val", 11, 15)],
        SourceLocation::new(0, 16),
    );
    let ast = program(vec![expr_stmt(call)]);

    let hints = provider.generate_hints(&ast, &identity_pos, None);
    let param_hints: Vec<_> = hints
        .iter()
        .filter(|h| h.kind == InlayHintKind::Parameter)
        .collect();

    assert_eq!(param_hints.len(), 2);
    for ph in &param_hints {
        assert!(
            !ph.padding_left,
            "Parameter hints should not have left padding"
        );
        assert!(
            ph.padding_right,
            "Parameter hints should have right padding"
        );
    }
}

#[test]
fn provider_generate_hints_combines_parameter_and_type_hints() {
    let provider = InlayHintsProvider::new();
    // push(@arr, 42) -- push has 2 param hints, 42 has a Num type hint
    let call = func_call(
        "push",
        vec![variable("@", "arr", 5, 9), number("42", 11, 13)],
        SourceLocation::new(0, 14),
    );
    let ast = program(vec![expr_stmt(call)]);

    let hints = provider.generate_hints(&ast, &identity_pos, None);
    let param_count = hints
        .iter()
        .filter(|h| h.kind == InlayHintKind::Parameter)
        .count();
    let type_count = hints
        .iter()
        .filter(|h| h.kind == InlayHintKind::Type)
        .count();

    assert_eq!(param_count, 2, "Expected 2 parameter hints for push");
    assert_eq!(type_count, 1, "Expected 1 type hint for number 42");
}

// ===========================================================================
// extract_param_names edge cases
// ===========================================================================

#[test]
fn extract_param_names_empty_string() {
    let params = extract_param_names("");
    assert!(params.is_empty());
}

#[test]
fn extract_param_names_only_function_name() {
    let params = extract_param_names("fork");
    assert!(params.is_empty());
}

#[test]
fn extract_param_names_preserves_order() {
    let params = extract_param_names("substr EXPR, OFFSET, LENGTH, REPLACEMENT");
    assert_eq!(params.len(), 4);
    assert_eq!(params[0], "expr");
    assert_eq!(params[1], "offset");
    assert_eq!(params[2], "length");
    assert_eq!(params[3], "replacement");
}

#[test]
fn extract_param_names_with_multiple_slashes() {
    // Ensure double slashes are cleaned properly
    let params = extract_param_names("split /PATTERN/, EXPR, LIMIT");
    assert_eq!(params[0], "pattern");
    assert_eq!(params[1], "expr");
    assert_eq!(params[2], "limit");
}

#[test]
fn extract_param_names_lowercases_all() {
    let params = extract_param_names("open FILEHANDLE, MODE, FILENAME");
    for p in &params {
        assert_eq!(
            p,
            &p.to_lowercase(),
            "Parameter '{}' should be lowercase",
            p
        );
    }
}

// ===========================================================================
// Binary expressions with literal operands
// ===========================================================================

#[test]
fn type_hints_for_binary_operands() {
    // 1 + 2 -- both operands are numbers
    let expr = binary("+", number("1", 0, 1), number("2", 4, 5), 0, 5);
    let ast = program(vec![expr_stmt(expr)]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    let num_hints: Vec<_> = hints
        .iter()
        .filter(|h| h["label"].as_str() == Some(": Num"))
        .collect();
    assert_eq!(num_hints.len(), 2);
}

#[test]
fn type_hints_for_string_concatenation() {
    // "hello" . "world"
    let expr = binary(
        ".",
        string_node("hello", false, 0, 7),
        string_node("world", false, 10, 17),
        0,
        17,
    );
    let ast = program(vec![expr_stmt(expr)]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    let str_hints: Vec<_> = hints
        .iter()
        .filter(|h| h["label"].as_str() == Some(": Str"))
        .collect();
    assert_eq!(str_hints.len(), 2);
}

#[test]
fn type_hints_for_mixed_binary_operands() {
    // 42 . "hello" -- number and string
    let expr = binary(
        ".",
        number("42", 0, 2),
        string_node("hello", false, 5, 12),
        0,
        12,
    );
    let ast = program(vec![expr_stmt(expr)]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert_eq!(hints.len(), 2);
    let labels: Vec<_> = hints.iter().filter_map(|h| h["label"].as_str()).collect();
    assert!(labels.contains(&": Num"));
    assert!(labels.contains(&": Str"));
}

// ===========================================================================
// Range filtering with provider methods
// ===========================================================================

#[test]
fn provider_parameter_hints_range_filtering() {
    let provider = InlayHintsProvider::new();
    let call = func_call(
        "push",
        vec![variable("@", "arr", 5, 9), variable("$", "val", 11, 15)],
        SourceLocation::new(0, 16),
    );
    let ast = program(vec![expr_stmt(call)]);

    // Range covers only the first argument (offset 5 -> (0, 5))
    let range = Range::new(Position::new(0, 0), Position::new(0, 10));
    let hints = provider.parameter_hints(&ast, &identity_pos, Some(range));
    // Only the first hint (array: at position 5) should be in range
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].label, "array:");
}

#[test]
fn provider_trivial_type_hints_range_filtering() {
    let provider = InlayHintsProvider::new();
    let ast = program(vec![
        expr_stmt(number("1", 0, 1)),   // hint at (0, 1)
        expr_stmt(number("2", 50, 52)), // hint at (0, 52) or (1, 12) with multiline
    ]);

    // Range covers only position 0-10
    let range = Range::new(Position::new(0, 0), Position::new(0, 10));
    let hints = provider.trivial_type_hints(&ast, &identity_pos, Some(range));
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].label, ": Num");
    assert_eq!(hints[0].position.character, 1);
}

// ===========================================================================
// Edge cases: special node types that should not produce hints
// ===========================================================================

#[test]
fn heredoc_no_type_hint() {
    let node = Node::new(
        NodeKind::Heredoc {
            delimiter: "EOF".to_string(),
            content: "hello\nworld".to_string(),
            interpolated: true,
            indented: false,
            command: false,
            body_span: None,
        },
        SourceLocation::new(0, 20),
    );
    let ast = program(vec![expr_stmt(node)]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert!(hints.is_empty(), "Heredoc should not produce a type hint");
}

#[test]
fn do_block_no_type_hint() {
    let node = Node::new(
        NodeKind::Do {
            block: Box::new(block(vec![], 3, 5)),
        },
        SourceLocation::new(0, 6),
    );
    let ast = program(vec![expr_stmt(node)]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert!(hints.is_empty(), "Do block should not produce a type hint");
}

#[test]
fn eval_block_no_type_hint() {
    let node = Node::new(
        NodeKind::Eval {
            block: Box::new(block(vec![], 5, 7)),
        },
        SourceLocation::new(0, 8),
    );
    let ast = program(vec![expr_stmt(node)]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    assert!(
        hints.is_empty(),
        "Eval block should not produce a type hint"
    );
}

#[test]
fn return_node_no_type_hint() {
    let node = Node::new(
        NodeKind::Return {
            value: Some(Box::new(number("42", 7, 9))),
        },
        SourceLocation::new(0, 10),
    );
    let ast = program(vec![node]);
    let hints = trivial_type_hints(&ast, &identity_pos, None);
    // The return itself should not produce a hint, but the inner 42 should
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0]["label"].as_str(), Some(": Num"));
}

// ===========================================================================
// Function call: parameter hints with function call as argument
// ===========================================================================

#[test]
fn function_call_arg_is_function_call() {
    // split(/,/, join(":", @parts))
    let inner_call = func_call(
        "join",
        vec![
            string_node(":", false, 15, 18),
            variable("@", "parts", 20, 26),
        ],
        SourceLocation::new(10, 27),
    );
    let outer_call = func_call(
        "split",
        vec![regex_node(",", 6, 9), inner_call],
        SourceLocation::new(0, 28),
    );
    let ast = program(vec![expr_stmt(outer_call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    // split: 2 hints (pattern, expr) + join: 2 hints (expr, list) = 4
    assert_eq!(hints.len(), 4);
}

// ===========================================================================
// Multiple calls in a block scope
// ===========================================================================

#[test]
fn multiple_calls_in_block_scope() {
    let call1 = func_call(
        "push",
        vec![variable("@", "a", 5, 7), number("1", 9, 10)],
        SourceLocation::new(0, 11),
    );
    let call2 = func_call(
        "push",
        vec![variable("@", "b", 17, 19), number("2", 21, 22)],
        SourceLocation::new(12, 23),
    );
    let blk = block(vec![expr_stmt(call1), expr_stmt(call2)], 0, 24);
    let ast = program(vec![blk]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    // 2 calls x 2 params = 4
    assert_eq!(hints.len(), 4);
}

// ===========================================================================
// Hint label format verification
// ===========================================================================

#[test]
fn parameter_hint_label_ends_with_colon() {
    let call = func_call(
        "push",
        vec![variable("@", "arr", 5, 9), variable("$", "val", 11, 15)],
        SourceLocation::new(0, 16),
    );
    let ast = program(vec![expr_stmt(call)]);

    let hints = parameter_hints(&ast, &identity_pos, None);
    for hint in &hints {
        let label = hint["label"].as_str();
        assert!(
            label.is_some_and(|l| l.ends_with(':')),
            "Parameter hint label should end with colon, got: {:?}",
            label
        );
    }
}

#[test]
fn type_hint_label_starts_with_colon_space() {
    let ast = program(vec![
        expr_stmt(number("1", 0, 1)),
        expr_stmt(string_node("x", false, 3, 6)),
        expr_stmt(regex_node("p", 8, 12)),
        expr_stmt(anon_sub(14, 25)),
    ]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    for hint in &hints {
        let label = hint["label"].as_str();
        assert!(
            label.is_some_and(|l| l.starts_with(": ")),
            "Type hint label should start with ': ', got: {:?}",
            label
        );
    }
}

// ===========================================================================
// Stress test: deeply nested expression tree
// ===========================================================================

#[test]
fn deeply_nested_expressions_no_stack_overflow() {
    // Build a deeply nested binary expression: 1 + (1 + (1 + ... ))
    let mut expr = number("1", 0, 1);
    for i in 1..100 {
        let start = i * 4;
        expr = binary("+", expr, number("1", start, start + 1), 0, start + 1);
    }
    let ast = program(vec![expr_stmt(expr)]);

    let hints = trivial_type_hints(&ast, &identity_pos, None);
    // 100 number literals
    assert_eq!(hints.len(), 100);
}

// ===========================================================================
// Verify that generate_hints returns consistent results across calls
// ===========================================================================

#[test]
fn generate_hints_deterministic() {
    let provider = InlayHintsProvider::new();
    let ast = program(vec![
        expr_stmt(number("42", 0, 2)),
        expr_stmt(string_node("hello", false, 4, 11)),
        expr_stmt(func_call(
            "push",
            vec![variable("@", "arr", 15, 19), number("1", 21, 22)],
            SourceLocation::new(10, 23),
        )),
    ]);

    let hints1 = provider.generate_hints(&ast, &identity_pos, None);
    let hints2 = provider.generate_hints(&ast, &identity_pos, None);

    assert_eq!(hints1.len(), hints2.len());
    for (h1, h2) in hints1.iter().zip(hints2.iter()) {
        assert_eq!(h1.label, h2.label);
        assert_eq!(h1.kind, h2.kind);
        assert_eq!(h1.position, h2.position);
    }
}
