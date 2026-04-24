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

fn variable(name: &str, sigil: &str, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Variable { sigil: sigil.to_string(), name: name.to_string() },
        loc(start, end),
    )
}

fn variable_decl(name: &str, sigil: &str, declarator: &str, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::VariableDeclaration {
            declarator: declarator.to_string(),
            variable: Box::new(variable(name, sigil, start, start + 1 + name.len())),
            attributes: Vec::new(),
            initializer: None,
        },
        loc(start, end),
    )
}

fn subroutine(name: &str, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Subroutine {
            name: Some(name.to_string()),
            name_span: Some(loc(start + 4, start + 4 + name.len())),
            prototype: None,
            signature: None,
            attributes: Vec::new(),
            body: Box::new(Node::new(NodeKind::Block { statements: Vec::new() }, loc(start, end))),
        },
        loc(start, end),
    )
}

fn synthetic_surface_ast(symbols: usize) -> Node {
    let mut statements = Vec::with_capacity(symbols + 2);
    statements.push(Node::new(
        NodeKind::Package { name: "Bench::Pkg".to_string(), name_span: loc(0, 10), block: None },
        loc(0, 20),
    ));

    for i in 0..symbols {
        let start = 20 + (i * 8);
        if i % 2 == 0 {
            statements.push(variable_decl(&format!("value_{i}"), "@", "my", start, start + 7));
        } else {
            statements.push(subroutine(&format!("run_{i}"), start, start + 7));
        }
    }

    Node::new(NodeKind::Program { statements }, loc(0, 20 + symbols * 8))
}

fn constant_wrapper_surface_ast() -> Node {
    let statements = vec![
        Node::new(
            NodeKind::Use {
                module: "constant".to_string(),
                args: vec![
                    "{".to_string(),
                    "ONE".to_string(),
                    "=>".to_string(),
                    "1".to_string(),
                    "}".to_string(),
                ],
                has_filter_risk: false,
            },
            loc(0, 10),
        ),
        Node::new(
            NodeKind::Use {
                module: "Const::Fast".to_string(),
                args: Vec::new(),
                has_filter_risk: false,
            },
            loc(10, 20),
        ),
        Node::new(
            NodeKind::Use {
                module: "Readonly".to_string(),
                args: Vec::new(),
                has_filter_risk: false,
            },
            loc(20, 30),
        ),
        Node::new(
            NodeKind::FunctionCall {
                name: "const".to_string(),
                args: vec![variable_decl("FAST_CONST", "$", "my", 30, 40)],
            },
            loc(30, 40),
        ),
        Node::new(
            NodeKind::FunctionCall {
                name: "Readonly".to_string(),
                args: vec![Node::new(
                    NodeKind::VariableListDeclaration {
                        declarator: "my".to_string(),
                        variables: vec![
                            variable("READONLY_ONE", "$", 40, 55),
                            variable("READONLY_TWO", "$", 56, 71),
                        ],
                        attributes: Vec::new(),
                        initializer: None,
                    },
                    loc(40, 72),
                )],
            },
            loc(40, 72),
        ),
    ];

    Node::new(NodeKind::Program { statements }, loc(0, 80))
}

fn benchmark_cursor_extract_ascii(c: &mut Criterion) {
    let source = "my $bench_symbol = compute_total($input);";
    let cursor = 5;
    c.bench_function("cursor_extract_ascii", |b| {
        b.iter(|| {
            let _ = black_box(extract_symbol_from_source(black_box(cursor), black_box(source)));
        });
    });
}

fn benchmark_cursor_extract_multibyte(c: &mut Criterion) {
    let source = "my $値 = process($値);";
    let cursor = 5;
    c.bench_function("cursor_extract_multibyte", |b| {
        b.iter(|| {
            let _ = black_box(extract_symbol_from_source(black_box(cursor), black_box(source)));
        });
    });
}

fn benchmark_cursor_range_lookup(c: &mut Criterion) {
    let source = "my $cursor_target = $other + 1;";
    let cursor = 5;
    c.bench_function("cursor_range_lookup", |b| {
        b.iter(|| {
            let _ = black_box(get_symbol_range_at_position(black_box(cursor), black_box(source)));
        });
    });
}

fn benchmark_token_under_cursor_utf16(c: &mut Criterion) {
    let source = "use Demo::😀Module::Worker;\n";
    let line = 0;
    let col_utf16 = 12;
    c.bench_function("token_under_cursor_utf16", |b| {
        b.iter(|| {
            let _ = black_box(token_under_cursor(
                black_box(source),
                black_box(line),
                black_box(col_utf16),
            ));
        });
    });
}

fn symbol_fixtures_1k() -> Vec<String> {
    (0..1_000).map(|i| format!("Bench::Service::Symbol{i:04}_Handler")).collect()
}

fn make_index(symbols: &[String]) -> SymbolIndex {
    let mut index = SymbolIndex::new();
    for symbol in symbols {
        index.add_symbol(symbol.clone());
    }
    index
}

fn benchmark_index_add_1k(c: &mut Criterion) {
    let symbols = symbol_fixtures_1k();
    c.bench_function("index_add_1k", |b| {
        b.iter(|| {
            let mut index = SymbolIndex::new();
            for symbol in black_box(&symbols) {
                index.add_symbol(symbol.clone());
            }
            black_box(index);
        });
    });
}

fn benchmark_index_prefix_query_1k(c: &mut Criterion) {
    let symbols = symbol_fixtures_1k();
    let index = make_index(&symbols);
    c.bench_function("index_prefix_query_1k", |b| {
        b.iter(|| {
            let _ = black_box(index.search_prefix(black_box("Bench::Service::Symbol09")));
        });
    });
}

fn benchmark_index_fuzzy_query_1k(c: &mut Criterion) {
    let symbols = symbol_fixtures_1k();
    let index = make_index(&symbols);
    c.bench_function("index_fuzzy_query_1k", |b| {
        b.iter(|| {
            let _ = black_box(index.search_fuzzy(black_box("service handler 0099")));
        });
    });
}

fn benchmark_surface_extract_small(c: &mut Criterion) {
    let ast = synthetic_surface_ast(24);
    c.bench_function("surface_extract_small", |b| {
        b.iter(|| {
            let _ = black_box(extract_symbol_decls(black_box(&ast), black_box(Some("main"))));
        });
    });
}

fn benchmark_surface_extract_large(c: &mut Criterion) {
    let ast = synthetic_surface_ast(2_000);
    c.bench_function("surface_extract_large", |b| {
        b.iter(|| {
            let _ = black_box(extract_symbol_decls(black_box(&ast), black_box(Some("main"))));
        });
    });
}

fn benchmark_surface_constant_wrapper_cases(c: &mut Criterion) {
    let ast = constant_wrapper_surface_ast();
    c.bench_function("surface_constant_wrapper_cases", |b| {
        b.iter(|| {
            let _ = black_box(extract_symbol_decls(black_box(&ast), black_box(Some("main"))));
        });
    });
}

criterion_group!(
    symbol_benches,
    benchmark_cursor_extract_ascii,
    benchmark_cursor_extract_multibyte,
    benchmark_cursor_range_lookup,
    benchmark_token_under_cursor_utf16,
    benchmark_index_add_1k,
    benchmark_index_prefix_query_1k,
    benchmark_index_fuzzy_query_1k,
    benchmark_surface_extract_small,
    benchmark_surface_extract_large,
    benchmark_surface_constant_wrapper_cases
);
criterion_main!(symbol_benches);
