//! Tests for `perl-symbol-surface` SymbolDecl extraction (MVP).
//!
//! These tests validate that `extract_symbol_decls` correctly walks the AST
//! and produces `SymbolDecl` values for packages, subroutines, variables,
//! constants, and classes — without depending on `perl-parser-core`.

use perl_ast::{Node, NodeKind, SourceLocation};
use perl_symbol::surface::{SymbolDecl, extract_symbol_decls};
use perl_symbol::{SymbolKind, VarKind};

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

// ── Package ──────────────────────────────────────────────────────────────────

#[test]
fn test_package_produces_symbol_decl() {
    // package MyApp;
    let node = Node::new(
        NodeKind::Package { name: "MyApp".to_string(), name_span: loc(8, 13), block: None },
        loc(0, 14),
    );
    let program = Node::new(NodeKind::Program { statements: vec![node] }, loc(0, 14));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 1);
    let d = &decls[0];
    assert_eq!(d.kind, SymbolKind::Package);
    assert_eq!(d.name, "MyApp");
    assert_eq!(d.qualified_name, "MyApp");
    assert_eq!(d.full_span, (0, 14));
    assert_eq!(d.anchor_span, Some((8, 13)));
    assert!(d.container.is_none());
}

// ── Subroutine ───────────────────────────────────────────────────────────────

#[test]
fn test_subroutine_produces_symbol_decl() {
    // sub greet { }
    let body = Node::new(NodeKind::Block { statements: vec![] }, loc(10, 13));
    let sub_node = Node::new(
        NodeKind::Subroutine {
            name: Some("greet".to_string()),
            name_span: Some(loc(4, 9)),
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(body),
        },
        loc(0, 13),
    );
    let program = Node::new(NodeKind::Program { statements: vec![sub_node] }, loc(0, 13));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 1);
    let d = &decls[0];
    assert_eq!(d.kind, SymbolKind::Subroutine);
    assert_eq!(d.name, "greet");
    assert_eq!(d.qualified_name, "greet");
    assert_eq!(d.full_span, (0, 13));
    assert_eq!(d.anchor_span, Some((4, 9)));
    assert!(d.container.is_none());
}

#[test]
fn test_anonymous_subroutine_is_skipped() {
    // my $cb = sub { };
    let body = Node::new(NodeKind::Block { statements: vec![] }, loc(9, 12));
    let anon_sub = Node::new(
        NodeKind::Subroutine {
            name: None,
            name_span: None,
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(body),
        },
        loc(0, 12),
    );
    let program = Node::new(NodeKind::Program { statements: vec![anon_sub] }, loc(0, 12));

    let decls = extract_symbol_decls(&program, None);
    assert!(decls.is_empty(), "anonymous sub should produce no SymbolDecl");
}

// ── Variable declaration ─────────────────────────────────────────────────────

#[test]
fn test_scalar_variable_declaration_produces_symbol_decl() {
    // my $count = 0;
    let var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "count".to_string() },
        loc(3, 9),
    );
    let decl_node = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var),
            attributes: vec![],
            initializer: None,
        },
        loc(0, 9),
    );
    let program = Node::new(NodeKind::Program { statements: vec![decl_node] }, loc(0, 9));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 1);
    let d = &decls[0];
    assert_eq!(d.kind, SymbolKind::Variable(VarKind::Scalar));
    assert_eq!(d.name, "count");
    assert_eq!(d.qualified_name, "count");
    assert_eq!(d.full_span, (0, 9));
    // anchor_span is the variable node span
    assert_eq!(d.anchor_span, Some((3, 9)));
}

#[test]
fn test_array_variable_declaration() {
    // my @items;
    let var = Node::new(
        NodeKind::Variable { sigil: "@".to_string(), name: "items".to_string() },
        loc(3, 9),
    );
    let decl_node = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var),
            attributes: vec![],
            initializer: None,
        },
        loc(0, 9),
    );
    let program = Node::new(NodeKind::Program { statements: vec![decl_node] }, loc(0, 9));

    let decls = extract_symbol_decls(&program, None);
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].kind, SymbolKind::Variable(VarKind::Array));
    assert_eq!(decls[0].name, "items");
}

#[test]
fn test_hash_variable_declaration() {
    // my %opts;
    let var = Node::new(
        NodeKind::Variable { sigil: "%".to_string(), name: "opts".to_string() },
        loc(3, 8),
    );
    let decl_node = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var),
            attributes: vec![],
            initializer: None,
        },
        loc(0, 8),
    );
    let program = Node::new(NodeKind::Program { statements: vec![decl_node] }, loc(0, 8));

    let decls = extract_symbol_decls(&program, None);
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].kind, SymbolKind::Variable(VarKind::Hash));
    assert_eq!(decls[0].name, "opts");
}

// ── Constant (use constant) ───────────────────────────────────────────────────

#[test]
fn test_use_constant_produces_symbol_decl() {
    // use constant MAX => 100;
    let use_node = Node::new(
        NodeKind::Use {
            module: "constant".to_string(),
            args: vec!["MAX".to_string(), "100".to_string()],
            has_filter_risk: false,
        },
        loc(0, 23),
    );
    let program = Node::new(NodeKind::Program { statements: vec![use_node] }, loc(0, 23));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 1);
    let d = &decls[0];
    assert_eq!(d.kind, SymbolKind::Constant);
    assert_eq!(d.name, "MAX");
    assert_eq!(d.qualified_name, "MAX");
    // anchor_span is None for use constant (no precise name span available)
    assert!(d.anchor_span.is_none());
}

#[test]
fn test_use_constant_hash_ref_style_produces_all_symbol_decls() {
    // use constant { FOO => 1, BAR => 2, BAZ => 3 };
    let use_node = Node::new(
        NodeKind::Use {
            module: "constant".to_string(),
            args: vec![
                "{".to_string(),
                "FOO".to_string(),
                "=>".to_string(),
                "1".to_string(),
                "BAR".to_string(),
                "=>".to_string(),
                "2".to_string(),
                "BAZ".to_string(),
                "=>".to_string(),
                "3".to_string(),
                "}".to_string(),
            ],
            has_filter_risk: false,
        },
        loc(0, 39),
    );
    let program = Node::new(NodeKind::Program { statements: vec![use_node] }, loc(0, 39));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].kind, SymbolKind::Constant);
    assert_eq!(decls[0].name, "FOO");
    assert_eq!(decls[1].name, "BAR");
    assert_eq!(decls[2].name, "BAZ");
}

#[test]
fn test_use_constant_qw_style_deduplicates_names() {
    // use constant qw(ONE TWO ONE);
    let use_node = Node::new(
        NodeKind::Use {
            module: "constant".to_string(),
            args: vec!["qw(ONE TWO ONE)".to_string()],
            has_filter_risk: false,
        },
        loc(0, 28),
    );
    let program = Node::new(NodeKind::Program { statements: vec![use_node] }, loc(0, 28));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].kind, SymbolKind::Constant);
    assert_eq!(decls[0].name, "ONE");
    assert_eq!(decls[1].kind, SymbolKind::Constant);
    assert_eq!(decls[1].name, "TWO");
}

#[test]
fn test_const_fast_my_scalar_produces_constant_decl() {
    let use_node = Node::new(
        NodeKind::Use { module: "Const::Fast".to_string(), args: vec![], has_filter_risk: false },
        loc(0, 16),
    );
    let variable = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "PI".to_string() },
        loc(26, 29),
    );
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(variable),
            attributes: vec![],
            initializer: None,
        },
        loc(20, 29),
    );
    let expr = Node::new(
        NodeKind::FunctionCall {
            name: "const".to_string(),
            args: vec![
                decl,
                Node::new(NodeKind::Number { value: "3.14159".to_string() }, loc(33, 40)),
            ],
        },
        loc(14, 40),
    );
    let stmt = Node::new(NodeKind::ExpressionStatement { expression: Box::new(expr) }, loc(14, 40));
    let program = Node::new(NodeKind::Program { statements: vec![use_node, stmt] }, loc(0, 40));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 1);
    let decl = &decls[0];
    assert_eq!(decl.kind, SymbolKind::Constant);
    assert_eq!(decl.name, "PI");
    assert_eq!(decl.qualified_name, "PI");
    assert_eq!(decl.anchor_span, Some((26, 29)));
    assert_eq!(decl.declarator.as_deref(), Some("const"));
}

#[test]
fn test_const_fast_my_array_produces_constant_decl() {
    let use_node = Node::new(
        NodeKind::Use { module: "Const::Fast".to_string(), args: vec![], has_filter_risk: false },
        loc(0, 16),
    );
    let variable = Node::new(
        NodeKind::Variable { sigil: "@".to_string(), name: "ARRAY".to_string() },
        loc(26, 32),
    );
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(variable),
            attributes: vec![],
            initializer: None,
        },
        loc(20, 32),
    );
    let expr = Node::new(
        NodeKind::FunctionCall {
            name: "const".to_string(),
            args: vec![decl, Node::new(NodeKind::ArrayLiteral { elements: vec![] }, loc(36, 38))],
        },
        loc(14, 38),
    );
    let stmt = Node::new(NodeKind::ExpressionStatement { expression: Box::new(expr) }, loc(14, 38));
    let program = Node::new(NodeKind::Program { statements: vec![use_node, stmt] }, loc(0, 38));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].kind, SymbolKind::Constant);
    assert_eq!(decls[0].name, "ARRAY");
    assert_eq!(decls[0].anchor_span, Some((26, 32)));
}

#[test]
fn test_readonly_my_scalar_produces_constant_decl() {
    let use_node = Node::new(
        NodeKind::Use { module: "Readonly".to_string(), args: vec![], has_filter_risk: false },
        loc(0, 13),
    );
    let variable = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "PI".to_string() },
        loc(23, 26),
    );
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(variable),
            attributes: vec![],
            initializer: None,
        },
        loc(17, 26),
    );
    let expr = Node::new(
        NodeKind::FunctionCall {
            name: "Readonly".to_string(),
            args: vec![
                decl,
                Node::new(NodeKind::Number { value: "3.14159".to_string() }, loc(30, 37)),
            ],
        },
        loc(14, 37),
    );
    let stmt = Node::new(NodeKind::ExpressionStatement { expression: Box::new(expr) }, loc(14, 37));
    let program = Node::new(NodeKind::Program { statements: vec![use_node, stmt] }, loc(0, 37));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 1);
    let decl = &decls[0];
    assert_eq!(decl.kind, SymbolKind::Constant);
    assert_eq!(decl.name, "PI");
    assert_eq!(decl.qualified_name, "PI");
    assert_eq!(decl.anchor_span, Some((23, 26)));
    assert_eq!(decl.declarator.as_deref(), Some("Readonly"));
}

#[test]
fn test_readonly_hash_produces_constant_decl() {
    let use_node = Node::new(
        NodeKind::Use { module: "Readonly".to_string(), args: vec![], has_filter_risk: false },
        loc(0, 13),
    );
    let variable = Node::new(
        NodeKind::Variable { sigil: "%".to_string(), name: "HASH".to_string() },
        loc(23, 28),
    );
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(variable),
            attributes: vec![],
            initializer: None,
        },
        loc(17, 28),
    );
    let expr = Node::new(
        NodeKind::FunctionCall {
            name: "Readonly".to_string(),
            args: vec![decl, Node::new(NodeKind::HashLiteral { pairs: vec![] }, loc(32, 34))],
        },
        loc(14, 34),
    );
    let stmt = Node::new(NodeKind::ExpressionStatement { expression: Box::new(expr) }, loc(14, 34));
    let program = Node::new(NodeKind::Program { statements: vec![use_node, stmt] }, loc(0, 34));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].kind, SymbolKind::Constant);
    assert_eq!(decls[0].name, "HASH");
    assert_eq!(decls[0].anchor_span, Some((23, 28)));
}

#[test]
fn test_readonly_our_hash_produces_constant_decl() {
    let use_node = Node::new(
        NodeKind::Use { module: "Readonly".to_string(), args: vec![], has_filter_risk: false },
        loc(0, 12),
    );
    let variable = Node::new(
        NodeKind::Variable { sigil: "%".to_string(), name: "HASH".to_string() },
        loc(24, 29),
    );
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "our".to_string(),
            variable: Box::new(variable),
            attributes: vec![],
            initializer: None,
        },
        loc(20, 29),
    );
    let expr = Node::new(
        NodeKind::FunctionCall {
            name: "Readonly".to_string(),
            args: vec![decl, Node::new(NodeKind::HashLiteral { pairs: vec![] }, loc(33, 35))],
        },
        loc(12, 35),
    );
    let stmt = Node::new(NodeKind::ExpressionStatement { expression: Box::new(expr) }, loc(12, 35));
    let program = Node::new(NodeKind::Program { statements: vec![use_node, stmt] }, loc(0, 35));

    let decls = extract_symbol_decls(&program, Some("My::Pkg"));

    assert_eq!(decls.len(), 1);
    let decl = &decls[0];
    assert_eq!(decl.kind, SymbolKind::Constant);
    assert_eq!(decl.name, "HASH");
    assert_eq!(decl.qualified_name, "My::Pkg::HASH");
    assert_eq!(decl.declarator.as_deref(), Some("Readonly"));
}

#[test]
fn test_variable_declaration_with_attributes_is_unwrapped() {
    // my $count :shared;
    let inner = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "count".to_string() },
        loc(3, 9),
    );
    let wrapped = Node::new(
        NodeKind::VariableWithAttributes { variable: Box::new(inner), attributes: vec![] },
        loc(3, 9),
    );
    let decl_node = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(wrapped),
            attributes: vec![],
            initializer: None,
        },
        loc(0, 9),
    );
    let program = Node::new(NodeKind::Program { statements: vec![decl_node] }, loc(0, 9));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 1);
    let decl = &decls[0];
    assert_eq!(decl.kind, SymbolKind::Variable(VarKind::Scalar));
    assert_eq!(decl.name, "count");
    assert_eq!(decl.anchor_span, Some((3, 9)));
}

// ── Class (Perl 5.38+) ────────────────────────────────────────────────────────

#[test]
fn test_class_produces_symbol_decl() {
    // class Point { }
    let body = Node::new(NodeKind::Block { statements: vec![] }, loc(12, 15));
    let class_node = Node::new(
        NodeKind::Class { name: "Point".to_string(), parents: vec![], body: Box::new(body) },
        loc(0, 15),
    );
    let program = Node::new(NodeKind::Program { statements: vec![class_node] }, loc(0, 15));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 1);
    let d = &decls[0];
    assert_eq!(d.kind, SymbolKind::Class);
    assert_eq!(d.name, "Point");
    assert_eq!(d.qualified_name, "Point");
    assert_eq!(d.full_span, (0, 15));
    // Class has no name_span field in AST, so anchor_span is None
    assert!(d.anchor_span.is_none());
}

// ── Format / Label ───────────────────────────────────────────────────────────

#[test]
fn test_format_produces_symbol_decl() {
    // format STDOUT =
    let format_node = Node::new(
        NodeKind::Format { name: "STDOUT".to_string(), body: "@<<<".to_string() },
        loc(0, 18),
    );
    let program = Node::new(NodeKind::Program { statements: vec![format_node] }, loc(0, 18));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 1);
    let d = &decls[0];
    assert_eq!(d.kind, SymbolKind::Format);
    assert_eq!(d.name, "STDOUT");
    assert_eq!(d.qualified_name, "STDOUT");
    assert_eq!(d.full_span, (0, 18));
    assert!(d.anchor_span.is_none());
}

#[test]
fn test_labeled_statement_produces_label_decl_and_walks_inner_statement() -> Result<(), String> {
    // OUTER: my $count;
    let var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "count".to_string() },
        loc(10, 16),
    );
    let var_decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var),
            attributes: vec![],
            initializer: None,
        },
        loc(7, 16),
    );
    let labeled = Node::new(
        NodeKind::LabeledStatement { label: "OUTER".to_string(), statement: Box::new(var_decl) },
        loc(0, 16),
    );
    let program = Node::new(NodeKind::Program { statements: vec![labeled] }, loc(0, 16));

    let decls = extract_symbol_decls(&program, None);
    assert_eq!(decls.len(), 2);

    let label = decls.iter().find(|d| d.kind == SymbolKind::Label).ok_or("label decl")?;
    assert_eq!(label.name, "OUTER");
    assert_eq!(label.qualified_name, "OUTER");
    assert_eq!(label.full_span, (0, 16));
    assert!(label.anchor_span.is_none());

    let variable = decls
        .iter()
        .find(|d| d.kind == SymbolKind::Variable(VarKind::Scalar))
        .ok_or("variable decl")?;
    assert_eq!(variable.name, "count");
    Ok(())
}

// ── Container tracking ────────────────────────────────────────────────────────

#[test]
fn test_subroutine_inside_package_has_container() -> Result<(), String> {
    // package Foo; sub bar { }
    let body = Node::new(NodeKind::Block { statements: vec![] }, loc(18, 21));
    let sub_node = Node::new(
        NodeKind::Subroutine {
            name: Some("bar".to_string()),
            name_span: Some(loc(14, 17)),
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(body),
        },
        loc(13, 21),
    );
    let pkg_node = Node::new(
        NodeKind::Package { name: "Foo".to_string(), name_span: loc(8, 11), block: None },
        loc(0, 12),
    );
    let program = Node::new(NodeKind::Program { statements: vec![pkg_node, sub_node] }, loc(0, 21));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 2);
    // Package decl has no container
    let pkg_decl =
        decls.iter().find(|d| d.kind == SymbolKind::Package).ok_or("expected Package decl")?;
    assert!(pkg_decl.container.is_none());

    // Sub decl uses current package context
    let sub_decl = decls
        .iter()
        .find(|d| d.kind == SymbolKind::Subroutine)
        .ok_or("expected Subroutine decl")?;
    assert_eq!(sub_decl.container.as_deref(), Some("Foo"));
    assert_eq!(sub_decl.qualified_name, "Foo::bar");
    Ok(())
}

// ── Nested block walking ──────────────────────────────────────────────────────

#[test]
fn test_subroutine_inside_package_block() -> Result<(), String> {
    // package Foo { sub baz { } }
    let inner_body = Node::new(NodeKind::Block { statements: vec![] }, loc(20, 23));
    let inner_sub = Node::new(
        NodeKind::Subroutine {
            name: Some("baz".to_string()),
            name_span: Some(loc(16, 19)),
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(inner_body),
        },
        loc(15, 24),
    );
    let pkg_block = Node::new(NodeKind::Block { statements: vec![inner_sub] }, loc(11, 25));
    let pkg_node = Node::new(
        NodeKind::Package {
            name: "Foo".to_string(),
            name_span: loc(8, 11),
            block: Some(Box::new(pkg_block)),
        },
        loc(0, 25),
    );
    let program = Node::new(NodeKind::Program { statements: vec![pkg_node] }, loc(0, 25));

    let decls = extract_symbol_decls(&program, None);

    // Should include both the Package decl and the Subroutine inside
    assert_eq!(decls.len(), 2);
    let sub_decl = decls
        .iter()
        .find(|d| d.kind == SymbolKind::Subroutine)
        .ok_or("expected Subroutine decl")?;
    assert_eq!(sub_decl.name, "baz");
    assert_eq!(sub_decl.container.as_deref(), Some("Foo"));
    assert_eq!(sub_decl.qualified_name, "Foo::baz");
    Ok(())
}

// ── Declarator tracking ───────────────────────────────────────────────────────

#[test]
fn test_our_variable_has_declarator() {
    // our $VERSION = '1.0';
    let var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "VERSION".to_string() },
        loc(4, 12),
    );
    let decl_node = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "our".to_string(),
            variable: Box::new(var),
            attributes: vec![],
            initializer: None,
        },
        loc(0, 12),
    );
    let program = Node::new(NodeKind::Program { statements: vec![decl_node] }, loc(0, 12));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].declarator.as_deref(), Some("our"));
}

#[test]
fn test_my_variable_has_declarator() {
    // my $x = 42;
    let var =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() }, loc(3, 5));
    let decl_node = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var),
            attributes: vec![],
            initializer: None,
        },
        loc(0, 5),
    );
    let program = Node::new(NodeKind::Program { statements: vec![decl_node] }, loc(0, 5));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].declarator.as_deref(), Some("my"));
}

#[test]
fn test_state_variable_has_declarator() {
    // state $count = 0;
    let var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "count".to_string() },
        loc(6, 12),
    );
    let decl_node = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "state".to_string(),
            variable: Box::new(var),
            attributes: vec![],
            initializer: None,
        },
        loc(0, 12),
    );
    let program = Node::new(NodeKind::Program { statements: vec![decl_node] }, loc(0, 12));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].declarator.as_deref(), Some("state"));
}

#[test]
fn test_local_variable_has_declarator() {
    // local $x = 1;
    let var =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() }, loc(6, 8));
    let decl_node = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "local".to_string(),
            variable: Box::new(var),
            attributes: vec![],
            initializer: None,
        },
        loc(0, 8),
    );
    let program = Node::new(NodeKind::Program { statements: vec![decl_node] }, loc(0, 8));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].declarator.as_deref(), Some("local"));
}

#[test]
fn test_our_vs_my_declarations_are_distinguished() -> Result<(), String> {
    // our $GLOBAL = 1; my $local = 2;
    let var_our = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "GLOBAL".to_string() },
        loc(4, 11),
    );
    let decl_our = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "our".to_string(),
            variable: Box::new(var_our),
            attributes: vec![],
            initializer: None,
        },
        loc(0, 11),
    );

    let var_my = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "local".to_string() },
        loc(16, 22),
    );
    let decl_my = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var_my),
            attributes: vec![],
            initializer: None,
        },
        loc(13, 22),
    );

    let program = Node::new(NodeKind::Program { statements: vec![decl_our, decl_my] }, loc(0, 22));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 2);

    let our_decl = decls.iter().find(|d| d.name == "GLOBAL").ok_or("expected GLOBAL decl")?;
    assert_eq!(our_decl.declarator.as_deref(), Some("our"), "GLOBAL should have 'our' declarator");

    let my_decl = decls.iter().find(|d| d.name == "local").ok_or("expected local decl")?;
    assert_eq!(my_decl.declarator.as_deref(), Some("my"), "local should have 'my' declarator");

    Ok(())
}

#[test]
fn test_our_variables_in_list_declaration_have_declarator() -> Result<(), String> {
    // our ($FOO, $BAR);
    let var_foo = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "FOO".to_string() },
        loc(5, 9),
    );
    let var_bar = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "BAR".to_string() },
        loc(11, 15),
    );
    let decl_node = Node::new(
        NodeKind::VariableListDeclaration {
            declarator: "our".to_string(),
            variables: vec![var_foo, var_bar],
            attributes: vec![],
            initializer: None,
        },
        loc(0, 16),
    );
    let program = Node::new(NodeKind::Program { statements: vec![decl_node] }, loc(0, 16));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 2);
    for d in &decls {
        assert_eq!(
            d.declarator.as_deref(),
            Some("our"),
            "expected 'our' declarator for {} but got {:?}",
            d.name,
            d.declarator
        );
    }
    Ok(())
}

#[test]
fn test_non_variable_decls_have_no_declarator() {
    // sub greet { }
    let body = Node::new(NodeKind::Block { statements: vec![] }, loc(10, 13));
    let sub_node = Node::new(
        NodeKind::Subroutine {
            name: Some("greet".to_string()),
            name_span: Some(loc(4, 9)),
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(body),
        },
        loc(0, 13),
    );
    let program = Node::new(NodeKind::Program { statements: vec![sub_node] }, loc(0, 13));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 1);
    assert!(
        decls[0].declarator.is_none(),
        "subroutines should have no declarator, got {:?}",
        decls[0].declarator
    );
}

// ── SymbolDecl structural properties ─────────────────────────────────────────

#[test]
fn test_symbol_decl_derives() {
    let d = SymbolDecl {
        kind: SymbolKind::Subroutine,
        name: "foo".to_string(),
        qualified_name: "Foo::foo".to_string(),
        full_span: (0, 10),
        anchor_span: Some((4, 7)),
        container: Some("Foo".to_string()),
        declarator: None,
    };
    // Must be Clone, Debug, PartialEq
    let d2 = d.clone();
    assert_eq!(d, d2);
    let _ = format!("{:?}", d);
}
