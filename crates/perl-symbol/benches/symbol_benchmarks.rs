use criterion::{Criterion, criterion_group, criterion_main};
use perl_ast::{Node, NodeKind, SourceLocation};
use perl_symbol::{
    SymbolIndex, extract_symbol_decls, extract_symbol_from_source, get_symbol_range_at_position,
    token_under_cursor,
};
use std::hint::black_box;

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

fn bench_cursor_extract_ascii(c: &mut Criterion) {
    let source = "my $value_name = 1;\n";
    let position = 4;

    c.bench_function("cursor_extract_ascii", |b| {
        b.iter(|| extract_symbol_from_source(black_box(position), black_box(source)));
    });
}

fn bench_cursor_extract_multibyte(c: &mut Criterion) {
    let source = "my $变量_name = 1;\n";
    let position = 4;

    c.bench_function("cursor_extract_multibyte", |b| {
        b.iter(|| extract_symbol_from_source(black_box(position), black_box(source)));
    });
}

fn bench_cursor_range_lookup(c: &mut Criterion) {
    let source = "my $very_long_symbol_name_for_lookup = 1;\n";
    let position = 7;

    c.bench_function("cursor_range_lookup", |b| {
        b.iter(|| get_symbol_range_at_position(black_box(position), black_box(source)));
    });
}

fn bench_token_under_cursor_utf16(c: &mut Criterion) {
    let source = "use Demo::😀Worker::Runner;\n";
    let line = 0;
    let col_utf16 = 13;

    c.bench_function("token_under_cursor_utf16", |b| {
        b.iter(|| token_under_cursor(black_box(source), black_box(line), black_box(col_utf16)));
    });
}

fn make_symbol_names(count: usize) -> Vec<String> {
    (0..count)
        .map(|i| format!("MyApp::Feature::Module{:04}::calculate_total_for_user_{:04}", i, i))
        .collect()
}

fn bench_index_add_1k(c: &mut Criterion) {
    let symbols = make_symbol_names(1_000);

    c.bench_function("index_add_1k", |b| {
        b.iter(|| {
            let mut index = SymbolIndex::new();
            for symbol in &symbols {
                index.add_symbol(black_box(symbol.clone()));
            }
            index
        });
    });
}

fn bench_index_prefix_query_1k(c: &mut Criterion) {
    let symbols = make_symbol_names(1_000);
    let mut index = SymbolIndex::new();
    for symbol in symbols {
        index.add_symbol(symbol);
    }

    c.bench_function("index_prefix_query_1k", |b| {
        b.iter(|| index.search_prefix(black_box("MyApp::Feature::Module09")));
    });
}

fn bench_index_fuzzy_query_1k(c: &mut Criterion) {
    let symbols = make_symbol_names(1_000);
    let mut index = SymbolIndex::new();
    for symbol in symbols {
        index.add_symbol(symbol);
    }

    c.bench_function("index_fuzzy_query_1k", |b| {
        b.iter(|| index.search_fuzzy(black_box("module 042 calculate user")));
    });
}

fn variable_decl(declarator: &str, sigil: &str, name: &str, start: usize) -> Node {
    let var = Node::new(
        NodeKind::Variable { sigil: sigil.to_string(), name: name.to_string() },
        loc(start + 3, start + 3 + name.len() + 1),
    );

    Node::new(
        NodeKind::VariableDeclaration {
            declarator: declarator.to_string(),
            variable: Box::new(var),
            attributes: vec![],
            initializer: None,
        },
        loc(start, start + 3 + name.len() + 1),
    )
}

fn build_surface_program_small() -> Node {
    let package = Node::new(
        NodeKind::Package { name: "Small::Pkg".to_string(), name_span: loc(8, 17), block: None },
        loc(0, 18),
    );

    let sub_body = Node::new(NodeKind::Block { statements: vec![] }, loc(22, 25));
    let sub = Node::new(
        NodeKind::Subroutine {
            name: Some("handle".to_string()),
            name_span: Some(loc(19, 25)),
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(sub_body),
        },
        loc(19, 28),
    );

    let var = variable_decl("my", "$", "counter", 29);
    let use_constant = Node::new(
        NodeKind::Use {
            module: "constant".to_string(),
            args: vec!["MAX_RETRIES".to_string(), "=>".to_string(), "5".to_string()],
            has_filter_risk: false,
        },
        loc(40, 70),
    );

    Node::new(NodeKind::Program { statements: vec![package, sub, var, use_constant] }, loc(0, 70))
}

fn build_surface_program_large() -> Node {
    let mut statements = Vec::new();

    for i in 0..500usize {
        statements.push(variable_decl("my", "$", &format!("scalar_{i}"), i * 10));
        statements.push(variable_decl("our", "@", &format!("array_{i}"), i * 10 + 100_000));
    }

    Node::new(NodeKind::Program { statements }, loc(0, 200_000))
}

fn make_wrapper_stmt(call_name: &str, declarator: &str, var_name: &str) -> Node {
    let variable = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: var_name.to_string() },
        loc(20, 20 + var_name.len() + 1),
    );
    let declaration = Node::new(
        NodeKind::VariableDeclaration {
            declarator: declarator.to_string(),
            variable: Box::new(variable),
            attributes: vec![],
            initializer: None,
        },
        loc(16, 20 + var_name.len() + 1),
    );

    let expr = Node::new(
        NodeKind::FunctionCall {
            name: call_name.to_string(),
            args: vec![
                declaration,
                Node::new(NodeKind::Number { value: "42".to_string() }, loc(32, 34)),
            ],
        },
        loc(12, 34),
    );

    Node::new(NodeKind::ExpressionStatement { expression: Box::new(expr) }, loc(12, 34))
}

fn build_surface_program_constant_wrappers() -> Node {
    let const_fast_use = Node::new(
        NodeKind::Use { module: "Const::Fast".to_string(), args: vec![], has_filter_risk: false },
        loc(0, 12),
    );
    let readonly_use = Node::new(
        NodeKind::Use { module: "Readonly".to_string(), args: vec![], has_filter_risk: false },
        loc(35, 44),
    );

    let const_stmt = make_wrapper_stmt("const", "my", "PI");
    let readonly_stmt = make_wrapper_stmt("Readonly", "our", "LIMIT");

    Node::new(
        NodeKind::Program {
            statements: vec![const_fast_use, readonly_use, const_stmt, readonly_stmt],
        },
        loc(0, 128),
    )
}

fn bench_surface_extract_small(c: &mut Criterion) {
    let program = build_surface_program_small();

    c.bench_function("surface_extract_small", |b| {
        b.iter(|| extract_symbol_decls(black_box(&program), black_box(None)));
    });
}

fn bench_surface_extract_large(c: &mut Criterion) {
    let program = build_surface_program_large();

    c.bench_function("surface_extract_large", |b| {
        b.iter(|| extract_symbol_decls(black_box(&program), black_box(Some("Big::Pkg"))));
    });
}

fn bench_surface_constant_wrapper_cases(c: &mut Criterion) {
    let program = build_surface_program_constant_wrappers();

    c.bench_function("surface_constant_wrapper_cases", |b| {
        b.iter(|| extract_symbol_decls(black_box(&program), black_box(Some("Cfg::Pkg"))));
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
