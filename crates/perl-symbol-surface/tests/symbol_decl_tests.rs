//! Tests for `perl-symbol-surface` SymbolDecl extraction (MVP).
//!
//! These tests validate that `extract_symbol_decls` correctly walks the AST
//! and produces `SymbolDecl` values for packages, subroutines, variables,
//! constants, and classes — without depending on `perl-parser-core`.

use perl_ast::{Node, NodeKind, SourceLocation};
use perl_symbol_surface::{SymbolDecl, extract_symbol_decls};
use perl_symbol_types::{SymbolKind, VarKind};

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

// ── VariableListDeclaration ───────────────────────────────────────────────────

#[test]
fn test_variable_list_declaration_produces_one_decl_per_variable() {
    // my ($x, $y);
    // Verifies: each variable in the list gets its own SymbolDecl with correct
    // full_span (the list decl span) and per-variable anchor_span.
    let var_x =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() }, loc(4, 6));
    let var_y =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "y".to_string() }, loc(8, 10));
    let list_decl = Node::new(
        NodeKind::VariableListDeclaration {
            declarator: "my".to_string(),
            variables: vec![var_x, var_y],
            attributes: vec![],
            initializer: None,
        },
        loc(0, 12),
    );
    let program = Node::new(NodeKind::Program { statements: vec![list_decl] }, loc(0, 12));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 2, "each variable in the list must produce a SymbolDecl");
    let x_decl = decls.iter().find(|d| d.name == "x").unwrap();
    assert_eq!(x_decl.kind, SymbolKind::Variable(VarKind::Scalar));
    assert_eq!(x_decl.full_span, (0, 12)); // list decl span
    assert_eq!(x_decl.anchor_span, Some((4, 6))); // variable node span
    let y_decl = decls.iter().find(|d| d.name == "y").unwrap();
    assert_eq!(y_decl.kind, SymbolKind::Variable(VarKind::Scalar));
    assert_eq!(y_decl.full_span, (0, 12));
    assert_eq!(y_decl.anchor_span, Some((8, 10)));
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
fn test_use_constant_hash_ref_form_produces_no_decl() {
    // use constant { FOO => 1, BAR => 2 };
    // args[0] is "{" — the guard must skip this form entirely.
    let use_node = Node::new(
        NodeKind::Use {
            module: "constant".to_string(),
            args: vec![
                "{".to_string(),
                "FOO".to_string(),
                "1".to_string(),
                "BAR".to_string(),
                "2".to_string(),
                "}".to_string(),
            ],
            has_filter_risk: false,
        },
        loc(0, 36),
    );
    let program = Node::new(NodeKind::Program { statements: vec![use_node] }, loc(0, 36));

    let decls = extract_symbol_decls(&program, None);
    assert!(decls.is_empty(), "hash-ref constant form must produce no SymbolDecl");
}

#[test]
fn test_use_constant_unary_plus_hash_ref_form_produces_no_decl() {
    // use constant +{ FOO => 1 };
    // args[0] is "+" — the guard must skip this unary-plus hash-ref form.
    let use_node = Node::new(
        NodeKind::Use {
            module: "constant".to_string(),
            args: vec![
                "+".to_string(),
                "{".to_string(),
                "FOO".to_string(),
                "1".to_string(),
                "}".to_string(),
            ],
            has_filter_risk: false,
        },
        loc(0, 26),
    );
    let program = Node::new(NodeKind::Program { statements: vec![use_node] }, loc(0, 26));

    let decls = extract_symbol_decls(&program, None);
    assert!(decls.is_empty(), "unary-plus hash-ref constant form must produce no SymbolDecl");
}

// ── Class (Perl 5.38+) ────────────────────────────────────────────────────────

#[test]
fn test_class_produces_symbol_decl() {
    // class Point { }
    let body = Node::new(NodeKind::Block { statements: vec![] }, loc(12, 15));
    let class_node =
        Node::new(NodeKind::Class { name: "Point".to_string(), body: Box::new(body) }, loc(0, 15));
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

// ── Method inside class ───────────────────────────────────────────────────────

#[test]
fn test_method_inside_class_has_container_and_qualified_name() {
    // class Animal { method speak { } }
    // Verifies: Method arm uses the class name as package context,
    // container = "Animal", qualified_name = "Animal::speak".
    let method_body = Node::new(NodeKind::Block { statements: vec![] }, loc(24, 27));
    let method_node = Node::new(
        NodeKind::Method {
            name: "speak".to_string(),
            signature: None,
            attributes: vec![],
            body: Box::new(method_body),
        },
        loc(14, 28),
    );
    let class_body = Node::new(NodeKind::Block { statements: vec![method_node] }, loc(13, 29));
    let class_node = Node::new(
        NodeKind::Class { name: "Animal".to_string(), body: Box::new(class_body) },
        loc(0, 29),
    );
    let program = Node::new(NodeKind::Program { statements: vec![class_node] }, loc(0, 29));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 2);
    let method_decl = decls.iter().find(|d| d.kind == SymbolKind::Method).unwrap();
    assert_eq!(method_decl.name, "speak");
    assert_eq!(method_decl.container.as_deref(), Some("Animal"));
    assert_eq!(method_decl.qualified_name, "Animal::speak");
    assert!(method_decl.anchor_span.is_none()); // Method has no name_span in current AST
}

// ── Container tracking ────────────────────────────────────────────────────────

#[test]
fn test_subroutine_inside_package_has_container() {
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
    let pkg_decl = decls.iter().find(|d| d.kind == SymbolKind::Package).unwrap();
    assert!(pkg_decl.container.is_none());

    // Sub decl uses current package context
    let sub_decl = decls.iter().find(|d| d.kind == SymbolKind::Subroutine).unwrap();
    assert_eq!(sub_decl.container.as_deref(), Some("Foo"));
    assert_eq!(sub_decl.qualified_name, "Foo::bar");
}

// ── Nested block walking ──────────────────────────────────────────────────────

#[test]
fn test_subroutine_inside_package_block() {
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
    let sub_decl = decls.iter().find(|d| d.kind == SymbolKind::Subroutine).unwrap();
    assert_eq!(sub_decl.name, "baz");
    assert_eq!(sub_decl.container.as_deref(), Some("Foo"));
    assert_eq!(sub_decl.qualified_name, "Foo::baz");
}

#[test]
fn test_package_block_context_does_not_leak_to_siblings() {
    // package Foo { } sub outside { }
    // The sub after the block must NOT inherit Foo as its package context.
    let inner_body = Node::new(NodeKind::Block { statements: vec![] }, loc(11, 12));
    let pkg_node = Node::new(
        NodeKind::Package {
            name: "Foo".to_string(),
            name_span: loc(8, 11),
            block: Some(Box::new(inner_body)),
        },
        loc(0, 13),
    );
    let sub_body = Node::new(NodeKind::Block { statements: vec![] }, loc(24, 27));
    let sub_node = Node::new(
        NodeKind::Subroutine {
            name: Some("outside".to_string()),
            name_span: Some(loc(18, 25)),
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(sub_body),
        },
        loc(14, 27),
    );
    let program = Node::new(NodeKind::Program { statements: vec![pkg_node, sub_node] }, loc(0, 27));

    let decls = extract_symbol_decls(&program, None);

    assert_eq!(decls.len(), 2);
    let sub_decl = decls.iter().find(|d| d.kind == SymbolKind::Subroutine).unwrap();
    // Must NOT be Foo::outside — the block-scoped package must not leak
    assert!(
        sub_decl.container.is_none(),
        "package block scope must not leak to post-block siblings"
    );
    assert_eq!(sub_decl.qualified_name, "outside");
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
    };
    // Must be Clone, Debug, PartialEq
    let d2 = d.clone();
    assert_eq!(d, d2);
    let _ = format!("{:?}", d);
}
