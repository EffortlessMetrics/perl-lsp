//! Error Recovery Regression Tests — Incomplete/Broken Code
//!
//! These tests verify the most critical UX scenario for an LSP: the user is
//! actively typing and the parser must handle incomplete or broken code
//! gracefully.
//!
//! Invariants tested per scenario:
//! 1. Parser does NOT panic — calling `parser.parse()` must not abort the
//!    process (no `panic!`, no stack overflow, no OOM).
//! 2. Partial AST produced — the returned `NodeKind::Program` must contain
//!    at least one statement (not completely empty).
//! 3. Pre-error symbols visible — for "good code above, broken below", symbols
//!    defined before the error point must still appear in the AST.

use perl_parser_core::{NodeKind, Parser};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Recursively collect every `NodeKind::Subroutine` name found in the tree.
fn collect_sub_names(node: &perl_parser_core::Node) -> Vec<String> {
    let mut names = Vec::new();
    collect_sub_names_inner(node, &mut names);
    names
}

fn collect_sub_names_inner(node: &perl_parser_core::Node, out: &mut Vec<String>) {
    if let NodeKind::Subroutine { name: Some(n), .. } = &node.kind {
        out.push(n.clone());
    }
    for child in node.children() {
        collect_sub_names_inner(child, out);
    }
}

/// Recursively collect every `NodeKind::VariableDeclaration` variable name found in the tree.
fn collect_var_names(node: &perl_parser_core::Node) -> Vec<String> {
    let mut names = Vec::new();
    collect_var_names_inner(node, &mut names);
    names
}

fn collect_var_names_inner(node: &perl_parser_core::Node, out: &mut Vec<String>) {
    match &node.kind {
        NodeKind::VariableDeclaration { variable, .. } => {
            if let NodeKind::Variable { name, .. } = &variable.kind {
                out.push(name.clone());
            }
        }
        NodeKind::VariableListDeclaration { variables, .. } => {
            for var in variables {
                if let NodeKind::Variable { name, .. } = &var.kind {
                    out.push(name.clone());
                }
            }
        }
        _ => {}
    }
    for child in node.children() {
        collect_var_names_inner(child, out);
    }
}

/// Count the top-level statements in a `Program` node, or 0 for other nodes.
fn top_level_count(ast: &perl_parser_core::Node) -> usize {
    match &ast.kind {
        NodeKind::Program { statements } => statements.len(),
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Scenario 1: Incomplete sub declaration — missing closing brace
// ---------------------------------------------------------------------------

/// User is still typing the body of `sub foo`. The parser must not crash, must
/// produce a Program node, and must find `foo` in the partial AST.
#[test]
fn incomplete_sub_no_crash() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub foo {\n    my $x = 1;\n"; // missing closing brace

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Invariant 1: no panic (we got here)
    // Invariant 2: Program with at least one statement
    assert!(
        matches!(ast.kind, NodeKind::Program { .. }),
        "Parser must return a Program node for incomplete sub"
    );
    assert!(
        top_level_count(&ast) >= 1,
        "Partial AST must have at least one top-level statement"
    );
    Ok(())
}

#[test]
fn incomplete_sub_contains_sub_name() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub foo {\n    my $x = 1;\n"; // missing closing brace

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Invariant 3: the sub name is visible even without closing brace
    let subs = collect_sub_names(&ast);
    assert!(
        subs.contains(&"foo".to_string()),
        "Sub 'foo' must be findable in the partial AST; found subs: {:?}",
        subs
    );
    Ok(())
}

#[test]
fn incomplete_sub_records_unclosed_error() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub foo {\n    my $x = 1;\n";

    let mut parser = Parser::new(code);
    let _ast = parser.parse()?;

    // Parser should recognise the unclosed block
    let errors = parser.errors();
    assert!(
        !errors.is_empty(),
        "Parser must record at least one error for unclosed sub block"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 2: Incomplete if statement — missing closing brace
// ---------------------------------------------------------------------------

#[test]
fn incomplete_if_no_crash() -> Result<(), Box<dyn std::error::Error>> {
    let code = "if ($x > 0) {\n    print \"yes\";\n"; // missing closing brace

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Invariant 1 & 2
    assert!(
        matches!(ast.kind, NodeKind::Program { .. }),
        "Parser must return a Program node for incomplete if statement"
    );
    assert!(
        top_level_count(&ast) >= 1,
        "Partial AST must have at least one top-level statement for incomplete if"
    );
    Ok(())
}

#[test]
fn incomplete_if_records_errors() -> Result<(), Box<dyn std::error::Error>> {
    let code = "if ($x > 0) {\n    print \"yes\";\n";

    let mut parser = Parser::new(code);
    let _ast = parser.parse()?;

    // Parser should recognise the unclosed block
    let errors = parser.errors();
    assert!(
        !errors.is_empty(),
        "Parser must record at least one error for unclosed if block"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 3: Unterminated string literal
// ---------------------------------------------------------------------------

#[test]
fn unterminated_string_no_crash() -> Result<(), Box<dyn std::error::Error>> {
    // Double-quoted string with no closing quote — common mid-type scenario
    let code = "my $str = \"hello\n";

    let mut parser = Parser::new(code);
    // Parser may return Ok(partial AST) or an Err; either is acceptable as
    // long as it does not panic.
    let result = parser.parse();
    // Invariant 1: we reach this line — no panic, no stack overflow
    match result {
        Ok(ast) => {
            assert!(
                matches!(ast.kind, NodeKind::Program { .. }),
                "Successful parse of unterminated string must yield a Program node"
            );
        }
        Err(_) => {
            // An error return is acceptable; the key invariant is no panic
        }
    }
    Ok(())
}

#[test]
fn unterminated_string_with_good_code_before() -> Result<(), Box<dyn std::error::Error>> {
    // Multiple statements where the second one has an unterminated string
    let code = "my $x = 1;\nmy $str = \"hello\n";

    let mut parser = Parser::new(code);
    let result = parser.parse();
    // Invariant 1: no panic
    match result {
        Ok(ast) => {
            // If the parser succeeds, there should be at least one statement
            let count = top_level_count(&ast);
            assert!(count >= 1, "Parser returned Ok but AST is completely empty");
        }
        Err(_) => {
            // Catastrophic parse error for unterminated string is acceptable
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 4: Incomplete array literal — missing closing paren
// ---------------------------------------------------------------------------

/// User is mid-typing an array assignment and has not yet closed the paren.
/// `my @arr = (1, 2` — no trailing comma, just truncated before `)`.
/// The InsertedCloser recovery should kick in and preserve the declaration.
#[test]
fn incomplete_array_no_crash() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my @arr = (1, 2\n"; // missing closing paren, no trailing comma

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Invariant 1 & 2
    assert!(
        matches!(ast.kind, NodeKind::Program { .. }),
        "Parser must return a Program node for incomplete array literal"
    );
    assert!(
        top_level_count(&ast) >= 1,
        "Partial AST must have at least one statement for incomplete array"
    );
    Ok(())
}

#[test]
fn incomplete_array_variable_declared() -> Result<(), Box<dyn std::error::Error>> {
    // `my @arr = (1, 2` — truncated before `)`.  No trailing comma so the
    // parser can recover via InsertedCloser and emit a VariableDeclaration.
    let code = "my @arr = (1, 2\n";

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Invariant 3: @arr declaration must appear in the AST
    let vars = collect_var_names(&ast);
    assert!(
        vars.contains(&"arr".to_string()),
        "Variable 'arr' must be findable in AST for incomplete array; found vars: {:?}",
        vars
    );
    Ok(())
}

/// When a trailing comma is present and the parser cannot recover the expression,
/// it produces a single ERROR node rather than a partial declaration.
/// The invariant here is no panic — an ERROR node is acceptable.
#[test]
fn incomplete_array_trailing_comma_no_crash() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my @arr = (1, 2,\n"; // trailing comma — harder to recover

    let mut parser = Parser::new(code);
    // May return Ok with error node or Err — either is fine, no panic
    let result = parser.parse();
    match result {
        Ok(ast) => {
            assert!(
                matches!(ast.kind, NodeKind::Program { .. }),
                "Parser must return a Program node even for trailing-comma truncation"
            );
        }
        Err(_) => {
            // Catastrophic error is also acceptable for this pattern
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 5: Partial expression — incomplete binary operation
// ---------------------------------------------------------------------------

#[test]
fn incomplete_expression_no_crash() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $result = $x +\n"; // incomplete expression after operator

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Invariant 1 & 2
    assert!(
        matches!(ast.kind, NodeKind::Program { .. }),
        "Parser must return a Program node for incomplete expression"
    );
    assert!(
        top_level_count(&ast) >= 1,
        "Partial AST must have at least one statement for incomplete expression"
    );
    Ok(())
}

#[test]
fn incomplete_expression_variable_visible() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $result = $x +\n";

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Invariant 3: $result must still be declared in the AST
    let vars = collect_var_names(&ast);
    assert!(
        vars.contains(&"result".to_string()),
        "Variable 'result' must appear in AST for incomplete expression; found vars: {:?}",
        vars
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 6: Complete code above, broken code below — pre-error symbols survive
// ---------------------------------------------------------------------------

/// This is the most critical LSP scenario: the user has existing good code,
/// then types a new incomplete sub at the end. The already-defined `good_sub`
/// must still be findable in the AST — otherwise completions, hover, and
/// go-to-definition break for all code above the cursor.
#[test]
fn pre_error_sub_visible_after_broken_code() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub good_sub { return 1; }\nsub bad_sub {\n"; // bad_sub missing closing brace

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Invariant 1: no panic
    // Invariant 2: at least the good_sub statement
    assert!(
        top_level_count(&ast) >= 1,
        "AST must have at least one top-level statement (good_sub)"
    );

    // Invariant 3: good_sub is visible in the AST
    let subs = collect_sub_names(&ast);
    assert!(
        subs.contains(&"good_sub".to_string()),
        "Pre-error sub 'good_sub' must be visible in partial AST; found subs: {:?}",
        subs
    );
    Ok(())
}

#[test]
fn pre_error_variable_visible_after_broken_code() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $config = 42;\nsub broken {\n"; // broken sub with missing brace

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Invariant 3: $config must still appear in the AST
    let vars = collect_var_names(&ast);
    assert!(
        vars.contains(&"config".to_string()),
        "Pre-error variable '$config' must be visible in partial AST; found vars: {:?}",
        vars
    );
    Ok(())
}

#[test]
fn multiple_good_subs_before_broken_code() -> Result<(), Box<dyn std::error::Error>> {
    // Several complete subs followed by a truncated one — all complete subs must remain
    let code = concat!(
        "sub alpha { return 1; }\n",
        "sub beta  { return 2; }\n",
        "sub gamma {\n", // missing closing brace
    );

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Invariant 2: program is not empty
    assert!(
        top_level_count(&ast) >= 2,
        "AST must contain alpha and beta even though gamma is incomplete"
    );

    // Invariant 3: both complete subs are visible
    let subs = collect_sub_names(&ast);
    assert!(
        subs.contains(&"alpha".to_string()),
        "'alpha' must be visible in partial AST; found subs: {:?}",
        subs
    );
    assert!(
        subs.contains(&"beta".to_string()),
        "'beta' must be visible in partial AST; found subs: {:?}",
        subs
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 7: Completely empty input — edge case
// ---------------------------------------------------------------------------

#[test]
fn empty_input_no_crash() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("");
    let ast = parser.parse()?;

    assert!(
        matches!(ast.kind, NodeKind::Program { .. }),
        "Empty input must still produce a Program node"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 8: Only whitespace / comments
// ---------------------------------------------------------------------------

#[test]
fn whitespace_only_no_crash() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("   \n\n\t  ");
    let ast = parser.parse()?;

    assert!(
        matches!(ast.kind, NodeKind::Program { .. }),
        "Whitespace-only input must produce a Program node"
    );
    Ok(())
}

#[test]
fn comments_only_no_crash() -> Result<(), Box<dyn std::error::Error>> {
    let code = "# This is a comment\n# Another comment\n";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    assert!(
        matches!(ast.kind, NodeKind::Program { .. }),
        "Comment-only input must produce a Program node"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 9: Truncated file — EOF mid-statement (most common LSP scenario)
// ---------------------------------------------------------------------------

/// The buffer was cut off mid-statement with no newline. This simulates a
/// file that was saved in the middle of a keystroke sequence.
#[test]
fn truncated_mid_statement_no_crash() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;\nmy $y ="; // truncated at assignment operator

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    // Invariant 1 & 2
    assert!(
        matches!(ast.kind, NodeKind::Program { .. }),
        "Truncated mid-statement must still produce a Program node"
    );
    // $x was fully declared before the truncation
    let vars = collect_var_names(&ast);
    assert!(
        vars.contains(&"x".to_string()),
        "Variable '$x' declared before truncation must still be visible; found vars: {:?}",
        vars
    );
    Ok(())
}

/// Truncated inside a method chain — common when typing `.method(` with no close
#[test]
fn truncated_method_chain_no_crash() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $obj = Foo->new();\n$obj->bar("; // unclosed method call

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    assert!(
        matches!(ast.kind, NodeKind::Program { .. }),
        "Truncated method chain must produce a Program node"
    );
    Ok(())
}
