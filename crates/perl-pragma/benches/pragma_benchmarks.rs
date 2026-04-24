use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use perl_ast::SourceLocation;
use perl_ast::ast::{Node, NodeKind};
use perl_pragma::PragmaTracker;
use std::hint::black_box;
use std::time::Duration;

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

fn use_node(module: &str, args: &[&str], start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::Use {
            module: module.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            has_filter_risk: false,
        },
        location: loc(start, end),
    }
}

fn no_node(module: &str, args: &[&str], start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::No {
            module: module.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            has_filter_risk: false,
        },
        location: loc(start, end),
    }
}

fn build_large_program(statement_count: usize) -> Node {
    let mut statements = Vec::with_capacity(statement_count);
    let mut offset = 0_usize;

    for idx in 0..statement_count {
        let (node, span) = match idx % 8 {
            0 => (use_node("strict", &[], offset, offset + 11), 11),
            1 => (use_node("warnings", &[], offset, offset + 13), 13),
            2 => (use_node("feature", &["qw(say state signatures)"], offset, offset + 36), 36),
            3 => (use_node("builtin", &["qw(trim true false)"], offset, offset + 30), 30),
            4 => (no_node("warnings", &["deprecated"], offset, offset + 24), 24),
            5 => (use_node("locale", &["':not_characters'"], offset, offset + 32), 32),
            6 => (no_node("feature", &[":all"], offset, offset + 15), 15),
            _ => (use_node("v5.40", &[], offset, offset + 10), 10),
        };
        statements.push(node);
        offset += span + 1;
    }

    Node { kind: NodeKind::Program { statements }, location: loc(0, offset) }
}

fn benchmark_build_large_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_large_file");
    group.measurement_time(Duration::from_secs(4));

    for size in [1_000_usize, 5_000_usize] {
        let ast = build_large_program(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &ast, |b, ast| {
            b.iter(|| {
                black_box(PragmaTracker::build(black_box(ast)));
            });
        });
    }

    group.finish();
}

fn benchmark_query_monotonic_offsets(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_monotonic_offsets");
    group.measurement_time(Duration::from_secs(4));

    for size in [1_000_usize, 5_000_usize] {
        let ast = build_large_program(size);
        let map = PragmaTracker::build(&ast);
        let max = ast.location.end;
        let offsets: Vec<usize> = (0..max).step_by(7).collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), &offsets, |b, offsets| {
            b.iter(|| {
                for offset in offsets {
                    black_box(PragmaTracker::state_for_offset(black_box(&map), black_box(*offset)));
                }
            });
        });
    }

    group.finish();
}

criterion_group!(benches, benchmark_build_large_file, benchmark_query_monotonic_offsets);
criterion_main!(benches);
