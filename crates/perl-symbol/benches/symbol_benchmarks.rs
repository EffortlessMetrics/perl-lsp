use criterion::{Criterion, criterion_group, criterion_main};
use perl_ast::{Node, NodeKind, SourceLocation};
use perl_symbol::{
    SymbolIndex, extract_symbol_decls, extract_symbol_from_source, get_symbol_range_at_position,
    token_under_cursor,
};
use std::hint::black_box;

fn loc() -> SourceLocation {
    SourceLocation { start: 0, end: 0 }
}

fn variable_node(sigil: &str, name: &str) -> Node {
    Node::new(NodeKind::Variable { sigil: sigil.to_string(), name: name.to_string() }, loc())
}

fn variable_decl_node(declarator: &str, sigil: &str, name: &str) -> Node {
    Node::new(
        NodeKind::VariableDeclaration {
            declarator: declarator.to_string(),
            variable: Box::new(variable_node(sigil, name)),
            attributes: Vec::new(),
            initializer: None,
        },
        loc(),
    )
}

fn use_node(module: &str, args: Vec<&str>) -> Node {
    Node::new(
        NodeKind::Use {
            module: module.to_string(),
            args: args.into_iter().map(str::to_string).collect(),
            has_filter_risk: false,
        },
        loc(),
    )
}

fn expression_stmt(node: Node) -> Node {
    Node::new(NodeKind::ExpressionStatement { expression: Box::new(node) }, loc())
}

fn build_small_surface_ast() -> Node {
    let sub_body = Node::new(
        NodeKind::Block { statements: vec![variable_decl_node("my", "$", "inside_sub")] },
        loc(),
    );
    let sub = Node::new(
        NodeKind::Subroutine {
            name: Some("helper".to_string()),
            name_span: None,
            prototype: None,
            signature: None,
            attributes: Vec::new(),
            body: Box::new(sub_body),
        },
        loc(),
    );

    Node::new(
        NodeKind::Program {
            statements: vec![
                Node::new(
                    NodeKind::Package {
                        name: "Bench::Small".to_string(),
                        name_span: loc(),
                        block: None,
                    },
                    loc(),
                ),
                variable_decl_node("our", "$", "state"),
                use_node("constant", vec!["FLAG", "=>", "1"]),
                sub,
            ],
        },
        loc(),
    )
}

fn build_large_surface_ast() -> Node {
    let mut statements = Vec::new();
    for i in 0..500 {
        statements.push(variable_decl_node("my", "$", &format!("var_{i}")));
        statements.push(variable_decl_node("our", "@", &format!("arr_{i}")));
        statements.push(use_node("constant", vec!["CONST", "=>", "1"]));
    }

    Node::new(NodeKind::Program { statements }, loc())
}

fn build_constant_wrapper_ast() -> Node {
    let const_call = Node::new(
        NodeKind::FunctionCall {
            name: "const".to_string(),
            args: vec![variable_decl_node("my", "$", "CF_ONE")],
        },
        loc(),
    );

    let readonly_call = Node::new(
        NodeKind::FunctionCall {
            name: "Readonly".to_string(),
            args: vec![Node::new(
                NodeKind::VariableListDeclaration {
                    declarator: "my".to_string(),
                    variables: vec![variable_node("$", "RO_ONE"), variable_node("$", "RO_TWO")],
                    attributes: Vec::new(),
                    initializer: None,
                },
                loc(),
            )],
        },
        loc(),
    );

    Node::new(
        NodeKind::Program {
            statements: vec![
                use_node("Const::Fast", Vec::new()),
                use_node("Readonly", Vec::new()),
                expression_stmt(const_call),
                expression_stmt(readonly_call),
            ],
        },
        loc(),
    )
}

fn build_index_fixture_1k() -> SymbolIndex {
    let mut index = SymbolIndex::new();
    for i in 0..1_000 {
        index.add_symbol(format!("Workspace::Module{idx:04}::computeTotal", idx = i));
    }
    index
}

fn bench_cursor_extract_ascii(c: &mut Criterion) {
    let source = "my $simple_name = $another_name + $third_name;";
    let position = 5;

    c.bench_function("cursor_extract_ascii", |b| {
        b.iter(|| extract_symbol_from_source(black_box(position), black_box(source)))
    });
}

fn bench_cursor_extract_multibyte(c: &mut Criterion) {
    let source = "my $naïve_value = $δοκιμή + $値;";
    let position = 5;

    c.bench_function("cursor_extract_multibyte", |b| {
        b.iter(|| extract_symbol_from_source(black_box(position), black_box(source)))
    });
}

fn bench_cursor_range_lookup(c: &mut Criterion) {
    let source = "my $worker_name = $another_worker;";
    let position = 5;

    c.bench_function("cursor_range_lookup", |b| {
        b.iter(|| get_symbol_range_at_position(black_box(position), black_box(source)))
    });
}

fn bench_token_under_cursor_utf16(c: &mut Criterion) {
    let text = "use Café::Δemo::Worker;\n";
    let line = 0;
    let col_utf16 = 10;

    c.bench_function("token_under_cursor_utf16", |b| {
        b.iter(|| token_under_cursor(black_box(text), black_box(line), black_box(col_utf16)))
    });
}

fn bench_index_add_1k(c: &mut Criterion) {
    let symbols: Vec<String> = (0..1_000)
        .map(|i| format!("Bench::Namespace{idx:04}::calculate_{idx:04}", idx = i))
        .collect();

    c.bench_function("index_add_1k", |b| {
        b.iter(|| {
            let mut index = SymbolIndex::new();
            for symbol in &symbols {
                index.add_symbol(black_box(symbol.clone()));
            }
            index
        })
    });
}

fn bench_index_prefix_query_1k(c: &mut Criterion) {
    let index = build_index_fixture_1k();

    c.bench_function("index_prefix_query_1k", |b| {
        b.iter(|| index.search_prefix(black_box("Workspace::Module00")))
    });
}

fn bench_index_fuzzy_query_1k(c: &mut Criterion) {
    let index = build_index_fixture_1k();

    c.bench_function("index_fuzzy_query_1k", |b| {
        b.iter(|| index.search_fuzzy(black_box("workspace module compute total")))
    });
}

fn bench_surface_extract_small(c: &mut Criterion) {
    let ast = build_small_surface_ast();

    c.bench_function("surface_extract_small", |b| {
        b.iter(|| extract_symbol_decls(black_box(&ast), black_box(Some("main"))))
    });
}

fn bench_surface_extract_large(c: &mut Criterion) {
    let ast = build_large_surface_ast();

    c.bench_function("surface_extract_large", |b| {
        b.iter(|| extract_symbol_decls(black_box(&ast), black_box(Some("main"))))
    });
}

fn bench_surface_constant_wrapper_cases(c: &mut Criterion) {
    let ast = build_constant_wrapper_ast();

    c.bench_function("surface_constant_wrapper_cases", |b| {
        b.iter(|| extract_symbol_decls(black_box(&ast), black_box(Some("main"))))
    });
}

criterion_group!(
    benches,
    bench_cursor_extract_ascii,
    bench_cursor_extract_multibyte,
    bench_cursor_range_lookup,
    bench_token_under_cursor_utf16,
    bench_index_add_1k,
    bench_index_prefix_query_1k,
    bench_index_fuzzy_query_1k,
    bench_surface_extract_small,
    bench_surface_extract_large,
    bench_surface_constant_wrapper_cases
);
criterion_main!(benches);
