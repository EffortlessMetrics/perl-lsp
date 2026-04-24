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
            args: args.iter().map(|s| s.to_string()).collect(),
            has_filter_risk: false,
        },
        location: loc(start, end),
    }
}

fn no_node(module: &str, args: &[&str], start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::No {
            module: module.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            has_filter_risk: false,
        },
        location: loc(start, end),
    }
}

fn build_large_file_ast(size: usize) -> Node {
    let mut statements = Vec::with_capacity(size * 4);
    let mut offset = 0;

    for idx in 0..size {
        statements.push(use_node("strict", &[], offset, offset + 10));
        offset += 12;
        statements.push(use_node("warnings", &[], offset, offset + 12));
        offset += 14;
        statements.push(use_node("feature", &[":5.40"], offset, offset + 16));
        offset += 18;
        if idx % 3 == 0 {
            statements.push(no_node("warnings", &["deprecated"], offset, offset + 19));
            offset += 21;
        }
    }

    Node { kind: NodeKind::Program { statements }, location: loc(0, offset) }
}

fn nested_scope_ast(depth: usize) -> Node {
    let mut offset = 0;
    let mut current =
        Node { kind: NodeKind::Block { statements: Vec::new() }, location: loc(0, 0) };

    for _ in 0..depth {
        let inner = Node {
            kind: NodeKind::Block {
                statements: vec![
                    use_node("strict", &[], offset, offset + 10),
                    use_node("warnings", &[], offset + 11, offset + 23),
                    use_node("feature", &["signatures"], offset + 24, offset + 44),
                ],
            },
            location: loc(offset, offset + 50),
        };
        offset += 51;

        current = Node {
            kind: NodeKind::Block { statements: vec![inner, current] },
            location: loc(0, offset),
        };
    }

    Node { kind: NodeKind::Program { statements: vec![current] }, location: loc(0, offset) }
}

fn version_compat_ast(size: usize) -> Node {
    let mut statements = Vec::with_capacity(size * 2);
    let mut offset = 0;

    for idx in 0..size {
        let version = if idx % 2 == 0 { "v5.36" } else { "v5.40" };
        statements.push(use_node(version, &[], offset, offset + 8));
        offset += 10;
        statements.push(use_node("feature", &[":5.40"], offset, offset + 16));
        offset += 18;
    }

    Node { kind: NodeKind::Program { statements }, location: loc(0, offset) }
}

fn bench_build_large_file(c: &mut Criterion) {
    let ast = build_large_file_ast(4_000);
    c.bench_function("build_large_file", |b| {
        b.iter(|| black_box(PragmaTracker::build(black_box(&ast))))
    });
}

fn bench_query_monotonic_offsets(c: &mut Criterion) {
    let ast = build_large_file_ast(4_000);
    let map = PragmaTracker::build(&ast);
    let end = ast.location.end;
    let offsets: Vec<usize> = (0..end).step_by(37).collect();

    c.bench_function("query_monotonic_offsets", |b| {
        b.iter(|| {
            for offset in &offsets {
                black_box(PragmaTracker::state_for_offset(black_box(&map), black_box(*offset)));
            }
        })
    });
}

fn bench_scope_analyzer_walk_style(c: &mut Criterion) {
    let ast = nested_scope_ast(2_000);
    c.bench_function("scope_analyzer_walk_style", |b| {
        b.iter(|| black_box(PragmaTracker::build(black_box(&ast))))
    });
}

fn bench_version_compat_walk_style(c: &mut Criterion) {
    let ast = version_compat_ast(6_000);
    c.bench_function("version_compat_walk_style", |b| {
        b.iter(|| black_box(PragmaTracker::build(black_box(&ast))))
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
