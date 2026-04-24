use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use perl_ast::ast::{Node, NodeKind, SourceLocation};
use perl_parser_core::Parser;
use perl_pragma::PragmaTracker;
use std::hint::black_box;

const REALISTIC_FIXTURE: &str =
    include_str!("../../perl-parser/tests/fixtures/diagnostics_test.pl");

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

fn use_stmt(offset: usize, module: &str, args: &[&str]) -> Node {
    Node::new(
        NodeKind::Use {
            module: module.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            has_filter_risk: false,
        },
        loc(offset, offset + 1),
    )
}

fn no_stmt(offset: usize, module: &str, args: &[&str]) -> Node {
    Node::new(
        NodeKind::No {
            module: module.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            has_filter_risk: false,
        },
        loc(offset, offset + 1),
    )
}

fn scoped_block(start: usize, end: usize, statements: Vec<Node>) -> Node {
    Node::new(NodeKind::Block { statements }, SourceLocation { start, end })
}

fn small_fixture_ast() -> Node {
    let statements = vec![
        use_stmt(0, "strict", &[]),
        use_stmt(3, "warnings", &[]),
        no_stmt(6, "strict", &["refs"]),
        scoped_block(
            8,
            14,
            vec![
                use_stmt(9, "feature", &["'signatures'"]),
                no_stmt(12, "warnings", &["'uninitialized'"]),
            ],
        ),
    ];

    Node::new(NodeKind::Program { statements }, loc(0, 15))
}

fn large_fixture_ast() -> Node {
    let mut statements = Vec::new();
    let mut cursor = 0;

    for i in 0..2000 {
        match i % 6 {
            0 => statements.push(use_stmt(cursor, "strict", &[])),
            1 => statements.push(use_stmt(cursor, "warnings", &[])),
            2 => statements.push(no_stmt(cursor, "warnings", &["'deprecated'"])),
            3 => statements.push(use_stmt(cursor, "feature", &["'try'", "'signatures'"])),
            4 => statements.push(no_stmt(cursor, "strict", &["'refs'"])),
            _ => {
                statements.push(scoped_block(
                    cursor,
                    cursor + 4,
                    vec![use_stmt(cursor + 1, "utf8", &[]), no_stmt(cursor + 2, "locale", &[])],
                ));
            }
        }
        cursor += 5;
    }

    Node::new(NodeKind::Program { statements }, loc(0, cursor + 1))
}

fn version_compat_fixture_ast() -> Node {
    let mut statements = Vec::new();
    let mut cursor = 0;

    for version in ["v5.10", "v5.12", "v5.16", "v5.20", "v5.34", "v5.36", "v5.40"] {
        statements.push(use_stmt(cursor, version, &[]));
        cursor += 2;
        statements.push(use_stmt(cursor, "feature", &[":all", "'signatures'"]));
        cursor += 2;
        statements.push(no_stmt(cursor, "feature", &["'switch'"]));
        cursor += 2;
    }

    Node::new(NodeKind::Program { statements }, loc(0, cursor + 1))
}

fn parsed_realistic_fixture_ast() -> Node {
    let mut parser = Parser::new(REALISTIC_FIXTURE);
    match parser.parse() {
        Ok(ast) => ast,
        Err(_) => small_fixture_ast(),
    }
}

fn deterministic_offsets(max_offset: usize, count: usize) -> Vec<usize> {
    let mut state = 0xC0FFEE_u64;
    let mut offsets = Vec::with_capacity(count);
    let span = max_offset.max(1) as u64;

    for _ in 0..count {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        offsets.push((state % span) as usize);
    }

    offsets
}

fn pragma_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("perl_pragma");

    let small_ast = small_fixture_ast();
    let large_ast = large_fixture_ast();
    let version_ast = version_compat_fixture_ast();
    let realistic_ast = parsed_realistic_fixture_ast();

    let large_map = PragmaTracker::build(&large_ast);
    let max_offset = large_ast.location.end;
    let random_offsets = deterministic_offsets(max_offset, 2048);
    let mut monotonic_offsets = deterministic_offsets(max_offset, 2048);
    monotonic_offsets.sort_unstable();

    group.bench_function("build_small_file", |b| {
        b.iter(|| {
            black_box(PragmaTracker::build(black_box(&small_ast)));
        });
    });

    group.bench_function("build_large_file", |b| {
        b.iter(|| {
            black_box(PragmaTracker::build(black_box(&large_ast)));
        });
    });

    group.bench_function("query_random_offsets", |b| {
        b.iter(|| {
            let mut strict_hits = 0usize;
            for offset in &random_offsets {
                if PragmaTracker::state_for_offset(black_box(&large_map), black_box(*offset))
                    .strict_vars
                {
                    strict_hits += 1;
                }
            }
            black_box(strict_hits);
        });
    });

    group.bench_function("query_monotonic_offsets", |b| {
        b.iter(|| {
            let mut warnings_hits = 0usize;
            for offset in &monotonic_offsets {
                if PragmaTracker::state_for_offset(black_box(&large_map), black_box(*offset))
                    .warnings
                {
                    warnings_hits += 1;
                }
            }
            black_box(warnings_hits);
        });
    });

    let final_offset = large_ast.location.end;
    group.bench_function("final_state_lookup", |b| {
        b.iter(|| {
            black_box(PragmaTracker::state_for_offset(
                black_box(&large_map),
                black_box(final_offset),
            ));
        });
    });

    group.bench_function("version_compat_walk_style", |b| {
        b.iter_batched(
            || version_ast.clone(),
            |ast| {
                black_box(PragmaTracker::build(black_box(&ast)));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("scope_analyzer_walk_style", |b| {
        b.iter_batched(
            || realistic_ast.clone(),
            |ast| {
                let map = PragmaTracker::build(black_box(&ast));
                let mut aggregate = 0usize;
                for offset in deterministic_offsets(ast.location.end.max(1), 512) {
                    if PragmaTracker::state_for_offset(&map, offset).warnings {
                        aggregate += 1;
                    }
                }
                black_box(aggregate);
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, pragma_benchmarks);
criterion_main!(benches);
