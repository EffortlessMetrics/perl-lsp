use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use perl_ast::SourceLocation;
use perl_ast::ast::{Node, NodeKind};
use perl_pragma::{PragmaState, PragmaTracker};
use std::hint::black_box;

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

fn dummy_node(start: usize, end: usize) -> Node {
    Node { kind: NodeKind::MissingExpression, location: loc(start, end) }
}

fn block(stmts: Vec<Node>, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Block { statements: stmts }, location: loc(start, end) }
}

fn program(stmts: Vec<Node>) -> Node {
    let end = stmts.last().map_or(0, |node| node.location.end);
    Node { kind: NodeKind::Program { statements: stmts }, location: loc(0, end) }
}

fn small_fixture_ast() -> Node {
    let mut offset = 0;
    let mut next_span = |len: usize| {
        let start = offset;
        offset += len;
        (start, offset)
    };

    let (s0, e0) = next_span(12);
    let (s1, e1) = next_span(14);
    let (s2, e2) = next_span(10);
    let (s3, e3) = next_span(18);
    let (s4, e4) = next_span(22);
    let (s5, e5) = next_span(16);

    let nested = block(
        vec![
            use_node("feature", &["'signatures'"], s3, e3),
            no_node("warnings", &["'experimental'"], s4, e4),
            dummy_node(s5, e5),
        ],
        s3,
        e5,
    );

    program(vec![
        use_node("strict", &[], s0, e0),
        use_node("warnings", &[], s1, e1),
        dummy_node(s2, e2),
        nested,
    ])
}

fn large_fixture_ast() -> Node {
    let mut statements = Vec::new();
    let mut offset = 0usize;

    for i in 0..700 {
        let start = offset;
        let end = start + 8 + (i % 11);
        offset = end + 1;

        let node = match i % 10 {
            0 => use_node("strict", &[], start, end),
            1 => no_node("strict", &["'refs'"], start, end),
            2 => use_node("warnings", &[], start, end),
            3 => no_node("warnings", &["'uninitialized'"], start, end),
            4 => use_node("feature", &["'say'", "'unicode_strings'"], start, end),
            5 => no_node("feature", &["'say'"], start, end),
            6 => use_node("v5.38", &[], start, end),
            7 => use_node("utf8", &[], start, end),
            8 => no_node("utf8", &[], start, end),
            _ => dummy_node(start, end),
        };

        statements.push(node);

        if i % 50 == 0 {
            let block_start = offset;
            let inner_one_end = block_start + 15;
            let inner_two_end = inner_one_end + 17;
            let block_end = inner_two_end + 1;
            offset = block_end + 2;

            let scoped = block(
                vec![
                    use_node("strict", &["'vars'"], block_start, inner_one_end),
                    no_node("warnings", &[], inner_one_end + 1, inner_two_end),
                ],
                block_start,
                block_end,
            );
            statements.push(scoped);
        }
    }

    program(statements)
}

fn random_offsets(max: usize, count: usize) -> Vec<usize> {
    let mut seed = 0xC0FFEEu64;
    let mut offsets = Vec::with_capacity(count);
    for _ in 0..count {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        offsets.push((seed as usize) % max.max(1));
    }
    offsets
}

fn monotonic_offsets(max: usize, step: usize) -> Vec<usize> {
    (0..max.max(1)).step_by(step.max(1)).collect()
}

fn bench_build_small_file(c: &mut Criterion) {
    let ast = small_fixture_ast();
    c.bench_function("build_small_file", |b| {
        b.iter(|| PragmaTracker::build(black_box(&ast)));
    });
}

fn bench_build_large_file(c: &mut Criterion) {
    let ast = large_fixture_ast();
    c.bench_function("build_large_file", |b| {
        b.iter(|| PragmaTracker::build(black_box(&ast)));
    });
}

fn bench_query_random_offsets(c: &mut Criterion) {
    let ast = large_fixture_ast();
    let pragma_map = PragmaTracker::build(&ast);
    let max_offset = ast.location.end;
    let offsets = random_offsets(max_offset, 1024);

    c.bench_function("query_random_offsets", |b| {
        b.iter(|| {
            let mut last = PragmaState::default();
            for offset in &offsets {
                last = PragmaTracker::state_for_offset(black_box(&pragma_map), black_box(*offset));
            }
            black_box(last)
        });
    });
}

fn bench_query_monotonic_offsets(c: &mut Criterion) {
    let ast = large_fixture_ast();
    let pragma_map = PragmaTracker::build(&ast);
    let offsets = monotonic_offsets(ast.location.end, 7);

    c.bench_function("query_monotonic_offsets", |b| {
        b.iter(|| {
            let mut last = PragmaState::default();
            for offset in &offsets {
                last = PragmaTracker::state_for_offset(black_box(&pragma_map), black_box(*offset));
            }
            black_box(last)
        });
    });
}

fn bench_final_state_lookup(c: &mut Criterion) {
    let ast = large_fixture_ast();
    let pragma_map = PragmaTracker::build(&ast);
    let final_offset = ast.location.end.saturating_sub(1);

    c.bench_function("final_state_lookup", |b| {
        b.iter(|| PragmaTracker::state_for_offset(black_box(&pragma_map), black_box(final_offset)));
    });
}

fn bench_version_compat_walk_style(c: &mut Criterion) {
    let ast = large_fixture_ast();
    c.bench_function("version_compat_walk_style", |b| {
        b.iter_batched(
            || PragmaTracker::build(black_box(&ast)),
            |pragma_map| {
                let checkpoints = [0usize, 80, 220, 450, 900, ast.location.end.saturating_sub(1)];
                let mut score = 0usize;
                for offset in checkpoints {
                    let state =
                        PragmaTracker::state_for_offset(&pragma_map, offset.min(ast.location.end));
                    if state.strict_vars {
                        score += 1;
                    }
                    if state.warnings {
                        score += 1;
                    }
                    if state.unicode_strings {
                        score += 1;
                    }
                }
                black_box(score)
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_scope_analyzer_walk_style(c: &mut Criterion) {
    let ast = large_fixture_ast();
    let pragma_map = PragmaTracker::build(&ast);
    let forward = monotonic_offsets(ast.location.end, 11);

    c.bench_function("scope_analyzer_walk_style", |b| {
        b.iter(|| {
            let mut hash = 0usize;
            for offset in &forward {
                let state = PragmaTracker::state_for_offset(black_box(&pragma_map), *offset);
                hash ^= (state.strict_vars as usize)
                    | ((state.strict_subs as usize) << 1)
                    | ((state.strict_refs as usize) << 2)
                    | ((state.warnings as usize) << 3)
                    | ((state.utf8 as usize) << 4);
            }
            for offset in forward.iter().rev().step_by(5) {
                let state = PragmaTracker::state_for_offset(black_box(&pragma_map), *offset);
                hash ^= state.disabled_warning_categories.len();
            }
            black_box(hash)
        });
    });
}

criterion_group!(
    pragma_benches,
    bench_build_small_file,
    bench_build_large_file,
    bench_query_random_offsets,
    bench_query_monotonic_offsets,
    bench_final_state_lookup,
    bench_version_compat_walk_style,
    bench_scope_analyzer_walk_style
);
criterion_main!(pragma_benches);
