use criterion::{Criterion, criterion_group, criterion_main};
use perl_ast::SourceLocation;
use perl_ast::ast::{Node, NodeKind};
use perl_pragma::PragmaTracker;
use std::hint::black_box;

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

fn use_node(module: &str, args: &[&str], start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::Use {
            module: module.to_string(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
            has_filter_risk: false,
        },
        location: loc(start, end),
    }
}

fn no_node(module: &str, args: &[&str], start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::No {
            module: module.to_string(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
            has_filter_risk: false,
        },
        location: loc(start, end),
    }
}

fn block(statements: Vec<Node>, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Block { statements }, location: loc(start, end) }
}

fn program(statements: Vec<Node>) -> Node {
    let end = statements.last().map_or(0, |n| n.location.end);
    Node { kind: NodeKind::Program { statements }, location: loc(0, end) }
}

fn build_large_program() -> Node {
    let mut statements = Vec::with_capacity(2_200);
    let mut cursor = 0;

    for i in 0..1_000 {
        statements.push(use_node("warnings", &[], cursor, cursor + 8));
        cursor += 9;

        let inner = vec![
            use_node("feature", &["'signatures'"], cursor + 1, cursor + 22),
            no_node("warnings", &["'uninitialized'"], cursor + 23, cursor + 50),
            use_node("builtin", &["qw(true false ceil floor)"], cursor + 51, cursor + 90),
        ];
        statements.push(block(inner, cursor, cursor + 92));
        cursor += 93;

        if i % 5 == 0 {
            statements.push(use_node("v5.40", &[], cursor, cursor + 8));
            cursor += 9;
        }
    }

    program(statements)
}

fn bench_build_large_file(c: &mut Criterion) {
    let ast = build_large_program();

    c.bench_function("build_large_file", |b| {
        b.iter(|| PragmaTracker::build(black_box(&ast)));
    });
}

fn bench_query_monotonic_offsets(c: &mut Criterion) {
    let ast = build_large_program();
    let map = PragmaTracker::build(&ast);
    let max = map.last().map_or(0, |(range, _)| range.end);
    let offsets: Vec<usize> = (0..max).step_by(16).collect();

    c.bench_function("query_monotonic_offsets", |b| {
        b.iter(|| {
            for offset in &offsets {
                let _ = PragmaTracker::state_for_offset(black_box(&map), *offset);
            }
        });
    });
}

fn bench_scope_analyzer_walk_style(c: &mut Criterion) {
    let mut statements = Vec::with_capacity(800);
    let mut cursor = 0;
    for _ in 0..200 {
        let nested = block(
            vec![
                use_node("strict", &[], cursor + 1, cursor + 10),
                block(
                    vec![
                        use_node("feature", &["':5.36'"], cursor + 11, cursor + 30),
                        no_node("strict", &["'refs'"], cursor + 31, cursor + 48),
                    ],
                    cursor + 10,
                    cursor + 50,
                ),
            ],
            cursor,
            cursor + 52,
        );
        statements.push(nested);
        cursor += 53;
    }

    let ast = program(statements);

    c.bench_function("scope_analyzer_walk_style", |b| {
        b.iter(|| PragmaTracker::build(black_box(&ast)));
    });
}

fn bench_version_compat_walk_style(c: &mut Criterion) {
    let mut statements = Vec::with_capacity(1200);
    let mut cursor = 0;

    for i in 0..600 {
        statements.push(use_node("v5.10", &[], cursor, cursor + 8));
        cursor += 9;
        if i % 2 == 0 {
            statements.push(use_node("feature", &["':5.40'"], cursor, cursor + 16));
        } else {
            statements.push(no_node("feature", &["':all'"], cursor, cursor + 14));
        }
        cursor += 17;
    }

    let ast = program(statements);

    c.bench_function("version_compat_walk_style", |b| {
        b.iter(|| PragmaTracker::build(black_box(&ast)));
    });
}

criterion_group!(
    benches,
    bench_build_large_file,
    bench_query_monotonic_offsets,
    bench_scope_analyzer_walk_style,
    bench_version_compat_walk_style
);
criterion_main!(benches);
