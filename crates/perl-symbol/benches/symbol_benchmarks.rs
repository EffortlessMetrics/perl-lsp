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

fn small_surface_program() -> Node {
    let package = Node::new(
        NodeKind::Package { name: "Demo".to_string(), name_span: loc(8, 12), block: None },
        loc(0, 13),
    );

    let variable = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "value".to_string() },
        loc(17, 23),
    );
    let variable_decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(variable),
            attributes: vec![],
            initializer: None,
        },
        loc(14, 24),
    );

    let sub_body = Node::new(NodeKind::Block { statements: vec![] }, loc(35, 38));
    let subroutine = Node::new(
        NodeKind::Subroutine {
            name: Some("run".to_string()),
            name_span: Some(loc(29, 32)),
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(sub_body),
        },
        loc(25, 38),
    );

    Node::new(
        NodeKind::Program { statements: vec![package, variable_decl, subroutine] },
        loc(0, 38),
    )
}

fn large_surface_program() -> Node {
    let mut statements = Vec::new();
    let mut offset = 0usize;

    for i in 0..400 {
        let var_name = format!("value_{i}");
        let sub_name = format!("handle_{i}");
        let const_name = format!("CONST_{i}");

        let variable = Node::new(
            NodeKind::Variable { sigil: "$".to_string(), name: var_name },
            loc(offset + 3, offset + 12),
        );

        statements.push(Node::new(
            NodeKind::VariableDeclaration {
                declarator: "my".to_string(),
                variable: Box::new(variable),
                attributes: vec![],
                initializer: None,
            },
            loc(offset, offset + 13),
        ));
        offset += 14;

        let body = Node::new(NodeKind::Block { statements: vec![] }, loc(offset + 10, offset + 13));
        statements.push(Node::new(
            NodeKind::Subroutine {
                name: Some(sub_name),
                name_span: Some(loc(offset + 4, offset + 9)),
                prototype: None,
                signature: None,
                attributes: vec![],
                body: Box::new(body),
            },
            loc(offset, offset + 13),
        ));
        offset += 14;

        statements.push(Node::new(
            NodeKind::Use {
                module: "constant".to_string(),
                args: vec![const_name, "=>".to_string(), "1".to_string()],
                has_filter_risk: false,
            },
            loc(offset, offset + 20),
        ));
        offset += 21;
    }

    Node::new(NodeKind::Program { statements }, loc(0, offset))
}

fn constant_wrapper_program() -> Node {
    let use_const_fast = Node::new(
        NodeKind::Use { module: "Const::Fast".to_string(), args: vec![], has_filter_risk: false },
        loc(0, 16),
    );

    let use_readonly = Node::new(
        NodeKind::Use { module: "Readonly".to_string(), args: vec![], has_filter_risk: false },
        loc(17, 29),
    );

    let const_var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "FAST_CONST".to_string() },
        loc(30, 41),
    );
    let const_decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(const_var),
            attributes: vec![],
            initializer: None,
        },
        loc(30, 45),
    );
    let const_call = Node::new(
        NodeKind::FunctionCall { name: "const".to_string(), args: vec![const_decl] },
        loc(30, 45),
    );

    let readonly_var_a = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "READONLY_A".to_string() },
        loc(46, 57),
    );
    let readonly_var_b = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "READONLY_B".to_string() },
        loc(58, 69),
    );
    let readonly_list = Node::new(
        NodeKind::VariableListDeclaration {
            declarator: "my".to_string(),
            variables: vec![readonly_var_a, readonly_var_b],
            attributes: vec![],
            initializer: None,
        },
        loc(46, 72),
    );
    let readonly_call = Node::new(
        NodeKind::FunctionCall { name: "Readonly".to_string(), args: vec![readonly_list] },
        loc(46, 72),
    );

    Node::new(
        NodeKind::Program {
            statements: vec![use_const_fast, use_readonly, const_call, readonly_call],
        },
        loc(0, 72),
    )
}

fn symbol_fixture_1k() -> Vec<String> {
    (0..1000)
        .map(|i| format!("Project::Module{module}::calculate_result_{i}", module = i % 50))
        .collect()
}

fn bench_cursor_extract_ascii(c: &mut Criterion) {
    let source = "my $customer_total = $order_total + $tax_value;";
    let position = 4;

    c.bench_function("cursor_extract_ascii", |b| {
        b.iter(|| extract_symbol_from_source(black_box(position), black_box(source)))
    });
}

fn bench_cursor_extract_multibyte(c: &mut Criterion) {
    let source = "my $café_value = $naïve_total + $résumé_count;";
    let position = 4;

    c.bench_function("cursor_extract_multibyte", |b| {
        b.iter(|| extract_symbol_from_source(black_box(position), black_box(source)))
    });
}

fn bench_cursor_range_lookup(c: &mut Criterion) {
    let source = "return $customer_profile_score if $customer_profile_score > 0;";
    let position = 8;

    c.bench_function("cursor_range_lookup", |b| {
        b.iter(|| get_symbol_range_at_position(black_box(position), black_box(source)))
    });
}

fn bench_token_under_cursor_utf16(c: &mut Criterion) {
    let text = "use Demo::😀Module::Worker;\n";
    let line = 0;
    let col_utf16 = 12;

    c.bench_function("token_under_cursor_utf16", |b| {
        b.iter(|| token_under_cursor(black_box(text), black_box(line), black_box(col_utf16)))
    });
}

fn bench_index_add_1k(c: &mut Criterion) {
    let symbols = symbol_fixture_1k();

    c.bench_function("index_add_1k", |b| {
        b.iter(|| {
            let mut index = SymbolIndex::new();
            for symbol in black_box(&symbols) {
                index.add_symbol(symbol.clone());
            }
            index
        })
    });
}

fn bench_index_prefix_query_1k(c: &mut Criterion) {
    let symbols = symbol_fixture_1k();
    let mut index = SymbolIndex::new();
    for symbol in &symbols {
        index.add_symbol(symbol.clone());
    }

    c.bench_function("index_prefix_query_1k", |b| {
        b.iter(|| index.search_prefix(black_box("Project::Module2::calculate")))
    });
}

fn bench_index_fuzzy_query_1k(c: &mut Criterion) {
    let symbols = symbol_fixture_1k();
    let mut index = SymbolIndex::new();
    for symbol in &symbols {
        index.add_symbol(symbol.clone());
    }

    c.bench_function("index_fuzzy_query_1k", |b| {
        b.iter(|| index.search_fuzzy(black_box("module2 calculate result")))
    });
}

fn bench_surface_extract_small(c: &mut Criterion) {
    let root = small_surface_program();

    c.bench_function("surface_extract_small", |b| {
        b.iter(|| extract_symbol_decls(black_box(&root), black_box(None)))
    });
}

fn bench_surface_extract_large(c: &mut Criterion) {
    let root = large_surface_program();

    c.bench_function("surface_extract_large", |b| {
        b.iter(|| extract_symbol_decls(black_box(&root), black_box(None)))
    });
}

fn bench_surface_constant_wrapper_cases(c: &mut Criterion) {
    let root = constant_wrapper_program();

    c.bench_function("surface_constant_wrapper_cases", |b| {
        b.iter(|| extract_symbol_decls(black_box(&root), black_box(None)))
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
