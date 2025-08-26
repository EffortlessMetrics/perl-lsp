/// Unit tests for quote operator parsing fixes
/// These tests lock in the behavior for:
/// - qr() with modifiers properly attached
/// - q() and other parenthesis-delimited quotes
/// - Different delimiters producing identical AST shapes
use perl_parser::Parser;

#[test]
fn qr_paren_mods_are_attached() {
    let code = "my $re = qr(t)ia;";
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("parse");
    let s = format!("{:?}", ast);

    // Check that modifiers are attached to the regex node
    assert!(s.contains("Regex"), "Expected Regex node: {}", s);

    // Should NOT have a separate identifier for modifiers
    let count = s.matches("Identifier").count();
    assert!(count <= 1, "Modifiers leaked as identifier (found {} identifiers): {}", count, s);
}

#[test]
fn qr_hash_mods_are_attached() {
    let code = "my $re = qr#t#ia;";
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("parse");
    let s = format!("{:?}", ast);

    // Check that modifiers are attached to the regex node
    assert!(s.contains("Regex"), "Expected Regex node: {}", s);

    // Should NOT have a separate identifier for modifiers
    let count = s.matches("Identifier").count();
    assert!(count <= 1, "Modifiers leaked as identifier (found {} identifiers): {}", count, s);
}

#[test]
fn qr_different_delimiters_same_shape() {
    let codes = vec![
        "qr(pattern)i",
        "qr{pattern}i",
        "qr[pattern]i",
        "qr<pattern>i",
        "qr#pattern#i",
        "qr!pattern!i",
    ];

    let mut shapes = Vec::new();

    for code in &codes {
        let mut parser = Parser::new(code);
        let ast = parser.parse().expect("parse");
        let shape = extract_shape(&ast);
        shapes.push((code, shape));
    }

    // All shapes should be identical
    let first_shape = &shapes[0].1;
    for (code, shape) in &shapes[1..] {
        assert_eq!(shape, first_shape, "Shape mismatch for '{}' vs '{}'", code, shapes[0].0);
    }
}

#[test]
fn m_different_delimiters_same_shape() {
    let codes = vec![
        "m(pattern)gc",
        "m{pattern}gc",
        "m[pattern]gc",
        "m<pattern>gc",
        "m#pattern#gc",
        "m!pattern!gc",
    ];

    let mut shapes = Vec::new();

    for code in &codes {
        let mut parser = Parser::new(code);
        let ast = parser.parse().expect("parse");
        let shape = extract_shape(&ast);
        shapes.push((code, shape));
    }

    // All shapes should be identical
    let first_shape = &shapes[0].1;
    for (code, shape) in &shapes[1..] {
        assert_eq!(shape, first_shape, "Shape mismatch for '{}' vs '{}'", code, shapes[0].0);
    }
}

#[test]
fn s_different_delimiters_same_shape() {
    let codes = vec![
        "s(old)(new)ge",
        "s{old}{new}ge",
        "s[old][new]ge",
        "s<old><new>ge",
        "s#old#new#ge",
        "s!old!new!ge",
    ];

    let mut shapes = Vec::new();

    for code in &codes {
        let mut parser = Parser::new(code);
        let ast = parser.parse().expect("parse");
        let shape = extract_shape(&ast);
        shapes.push((code, shape));
    }

    // All shapes should be identical
    let first_shape = &shapes[0].1;
    for (code, shape) in &shapes[1..] {
        assert_eq!(shape, first_shape, "Shape mismatch for '{}' vs '{}'", code, shapes[0].0);
    }
}

#[test]
fn tr_y_alias_same_shape() {
    let tr_code = "tr(abc)(xyz)d";
    let y_code = "y(abc)(xyz)d";

    let mut tr_parser = Parser::new(tr_code);
    let tr_ast = tr_parser.parse().expect("parse tr");
    let tr_shape = extract_shape(&tr_ast);

    let mut y_parser = Parser::new(y_code);
    let y_ast = y_parser.parse().expect("parse y");
    let y_shape = extract_shape(&y_ast);

    assert_eq!(tr_shape, y_shape, "tr and y should produce identical shapes");
}

#[test]
fn q_qq_different_delimiters_same_shape() {
    let q_codes = vec!["q(hello)", "q{hello}", "q[hello]", "q<hello>", "q#hello#"];

    let qq_codes = vec!["qq(hello)", "qq{hello}", "qq[hello]", "qq<hello>", "qq#hello#"];

    // Check q variants
    let mut q_shapes = Vec::new();
    for code in &q_codes {
        let mut parser = Parser::new(code);
        let ast = parser.parse().expect("parse");
        let shape = extract_shape(&ast);
        q_shapes.push((code, shape));
    }

    let first_q_shape = &q_shapes[0].1;
    for (code, shape) in &q_shapes[1..] {
        assert_eq!(shape, first_q_shape, "q shape mismatch for '{}' vs '{}'", code, q_shapes[0].0);
    }

    // Check qq variants
    let mut qq_shapes = Vec::new();
    for code in &qq_codes {
        let mut parser = Parser::new(code);
        let ast = parser.parse().expect("parse");
        let shape = extract_shape(&ast);
        qq_shapes.push((code, shape));
    }

    let first_qq_shape = &qq_shapes[0].1;
    for (code, shape) in &qq_shapes[1..] {
        assert_eq!(
            shape, first_qq_shape,
            "qq shape mismatch for '{}' vs '{}'",
            code, qq_shapes[0].0
        );
    }
}

#[test]
fn s_modifiers_are_captured() {
    let (_, _, modifiers) = perl_parser::quote_parser::extract_substitution_parts("s/foo/bar/g");
    assert_eq!(modifiers, "g");
}

#[test]
fn q_word_comparison_operators_parse() {
    // Test all word comparison operators with q()
    let operators = vec!["eq", "ne", "lt", "le", "gt", "ge", "cmp"];

    for op in operators {
        let code = format!("if (q($_) {} 'test') {{ }}", op);
        let mut parser = Parser::new(&code);
        let ast = parser.parse();
        assert!(ast.is_ok(), "Failed to parse 'q($_) {} 'test'': {:?}", op, ast);

        let code = format!("(q(x) {} q(y))", op);
        let mut parser = Parser::new(&code);
        let ast = parser.parse();
        assert!(ast.is_ok(), "Failed to parse '(q(x) {} q(y))': {:?}", op, ast);
    }
}

/// Extract the shape of an AST, ignoring data values
fn extract_shape(node: &perl_parser::ast::Node) -> String {
    use perl_parser::ast::NodeKind;

    match &node.kind {
        NodeKind::Program { statements } => {
            let shapes: Vec<String> = statements.iter().map(extract_shape).collect();
            format!("(program {})", shapes.join(" "))
        }
        NodeKind::Number { .. } => "(number)".to_string(),
        NodeKind::String { .. } => "(string)".to_string(),
        NodeKind::Identifier { .. } => "(identifier)".to_string(),
        NodeKind::Regex { .. } => "(regex)".to_string(),
        NodeKind::Substitution { expr, .. } => {
            format!("(substitution {})", extract_shape(expr))
        }
        NodeKind::Transliteration { expr, .. } => {
            format!("(transliteration {})", extract_shape(expr))
        }
        NodeKind::Match { expr, .. } => {
            format!("(match {})", extract_shape(expr))
        }
        NodeKind::Binary { left, right, .. } => {
            format!("(binary {} {})", extract_shape(left), extract_shape(right))
        }
        NodeKind::Unary { operand, .. } => {
            format!("(unary {})", extract_shape(operand))
        }
        NodeKind::VariableDeclaration { variable, .. } => {
            format!("(var_decl {})", extract_shape(variable))
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            format!("(assign {} {})", extract_shape(lhs), extract_shape(rhs))
        }
        NodeKind::FunctionCall { args, .. } => {
            let shapes: Vec<String> = args.iter().map(extract_shape).collect();
            format!("(call {})", shapes.join(" "))
        }
        NodeKind::Block { statements } => {
            let shapes: Vec<String> = statements.iter().map(extract_shape).collect();
            format!("(block {})", shapes.join(" "))
        }
        _ => "(other)".to_string(),
    }
}
