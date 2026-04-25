use criterion::{Criterion, Throughput, criterion_group, criterion_main};
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

fn block(statements: Vec<Node>, start: usize, end: usize) -> Node {
    Node { kind: NodeKind::Block { statements }, location: loc(start, end) }
}

fn program(statements: Vec<Node>) -> Node {
    let end = statements.last().map_or(0, |node| node.location.end);
    Node { kind: NodeKind::Program { statements }, location: loc(0, end) }
}

fn synthetic_small_ast() -> Node {
    program(vec![
        use_node("strict", &[], 0, 12),
        use_node("warnings", &[], 13, 28),
        use_node("v5.36", &[], 29, 40),
        block(
            vec![
                no_node("warnings", &["experimental"], 45, 73),
                use_node("feature", &["'signatures'"], 74, 98),
                no_node("strict", &["refs"], 99, 116),
            ],
            42,
            120,
        ),
        use_node("builtin", &["qw(true false ceil)"], 121, 151),
    ])
}

fn synthetic_large_ast(statement_count: usize) -> Node {
    let mut statements = Vec::with_capacity(statement_count * 3);
    let mut cursor = 0usize;

    for idx in 0..statement_count {
        let use_end = cursor + 12;
        statements.push(use_node("strict", &[], cursor, use_end));
        cursor = use_end + 1;

        let warn_end = cursor + 15;
        statements.push(use_node("warnings", &[], cursor, warn_end));
        cursor = warn_end + 1;

        let block_start = cursor;
        let mut block_items = Vec::with_capacity(3);
        let local_no_end = cursor + 18;
        block_items.push(no_node("strict", &["refs"], cursor, local_no_end));
        cursor = local_no_end + 1;

        let feature_end = cursor + 24;
        block_items.push(use_node("feature", &["'signatures'"], cursor, feature_end));
        cursor = feature_end + 1;

        if idx % 5 == 0 {
            let locale_end = cursor + 22;
            block_items.push(use_node("locale", &["':not_characters'"], cursor, locale_end));
            cursor = locale_end + 1;
        }

        let block_end = cursor + 3;
        statements.push(block(block_items, block_start, block_end));
        cursor = block_end + 1;
    }

    program(statements)
}

fn deterministic_offsets(limit: usize, count: usize) -> Vec<usize> {
    let mut seed = 0xA076_1D64_78BD_642Fu64;
    let upper = limit.max(1);
    let mut offsets = Vec::with_capacity(count);

    for _ in 0..count {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        offsets.push((seed as usize) % upper);
    }

    offsets
}

fn bench_build_small_file(c: &mut Criterion) {
    let ast = synthetic_small_ast();
    c.bench_function("build_small_file", |b| {
        b.iter(|| {
            let map = PragmaTracker::build(black_box(&ast));
            black_box(map)
        })
    });
}

fn bench_build_large_file(c: &mut Criterion) {
    let ast = synthetic_large_ast(700);
    c.bench_function("build_large_file", |b| {
        b.iter(|| {
            let map = PragmaTracker::build(black_box(&ast));
            black_box(map)
        })
    });
}

fn bench_query_random_offsets(c: &mut Criterion) {
    let ast = synthetic_large_ast(700);
    let map = PragmaTracker::build(&ast);
    let max_offset = map.last().map_or(10_000, |(range, _)| range.end);
    let offsets = deterministic_offsets(max_offset, 2_048);

    let mut group = c.benchmark_group("query_random_offsets");
    group.throughput(Throughput::Elements(offsets.len() as u64));
    group.bench_function("query_random_offsets", |b| {
        b.iter(|| {
            let mut checksum = 0usize;
            for offset in &offsets {
                let state = PragmaTracker::state_for_offset(black_box(&map), *offset);
                if state.strict_refs {
                    checksum = checksum.saturating_add(1);
                }
                if state.warnings {
                    checksum = checksum.saturating_add(1);
                }
            }
            black_box(checksum)
        })
    });
    group.finish();
}

fn bench_query_monotonic_offsets(c: &mut Criterion) {
    let ast = synthetic_large_ast(700);
    let map = PragmaTracker::build(&ast);
    let max_offset = map.last().map_or(10_000, |(range, _)| range.end).max(4_096);
    let step = (max_offset / 2_048).max(1);
    let offsets: Vec<usize> = (0..2_048).map(|idx| idx * step).collect();

    let mut group = c.benchmark_group("query_monotonic_offsets");
    group.throughput(Throughput::Elements(offsets.len() as u64));
    group.bench_function("query_monotonic_offsets", |b| {
        b.iter(|| {
            let mut checksum = 0usize;
            for offset in &offsets {
                let state = PragmaTracker::state_for_offset(black_box(&map), *offset);
                if state.strict_subs {
                    checksum = checksum.saturating_add(1);
                }
                if state.unicode_strings {
                    checksum = checksum.saturating_add(1);
                }
            }
            black_box(checksum)
        })
    });
    group.finish();
}

fn bench_final_state_lookup(c: &mut Criterion) {
    let ast = synthetic_large_ast(700);
    let map = PragmaTracker::build(&ast);
    c.bench_function("final_state_lookup", |b| {
        b.iter(|| {
            let state = PragmaTracker::state_for_offset(black_box(&map), usize::MAX / 2);
            black_box(state)
        })
    });
}

/// Build a synthetic AST that exercises version-implied feature bundles.
///
/// `demo_workspace/main.pl` only has `use strict; use warnings` — no version
/// pragma — so `has_feature("say")` / `has_feature("builtin")` would always
/// return false, leaving the feature-lookup branch unmeasured.  This fixture
/// uses an explicit `use v5.36` so the feature set is populated.
fn synthetic_version_ast() -> Node {
    program(vec![
        use_node("strict", &[], 0, 12),
        use_node("warnings", &[], 13, 28),
        // v5.36 implies: say, state, unicode_strings, unicode_eval, evalbytes,
        // current_sub, fc, postfix_deref, try, signatures, defer, isa
        use_node("v5.36", &[], 29, 40),
        block(
            vec![
                // Explicit no-warnings experiment inside a sub-scope
                no_node("warnings", &["experimental"], 45, 73),
                use_node("feature", &["'signatures'"], 74, 98),
            ],
            42,
            102,
        ),
        // builtin is a v5.40+ feature; add it explicitly so has_feature("builtin") fires
        use_node("feature", &["'builtin'"], 103, 124),
    ])
}

fn bench_version_compat_walk_style(c: &mut Criterion) {
    // Use a synthetic AST with a version pragma so feature lookups are
    // non-trivial (has_feature returns true for known features).
    let ast = synthetic_version_ast();
    let map = PragmaTracker::build(&ast);
    let max_offset = map.last().map_or(512, |(range, _)| range.end).max(512);
    let offsets: Vec<usize> = (0..512).map(|idx| idx * max_offset / 512).collect();

    c.bench_function("version_compat_walk_style", |b| {
        b.iter(|| {
            let mut score = 0usize;
            for offset in &offsets {
                let state = PragmaTracker::state_for_offset(black_box(&map), *offset);
                if state.has_feature("say") {
                    score = score.saturating_add(1);
                }
                if state.has_feature("builtin") {
                    score = score.saturating_add(2);
                }
                if state.warnings {
                    score = score.saturating_add(1);
                }
            }
            black_box(score)
        })
    });
}

fn bench_scope_analyzer_walk_style(c: &mut Criterion) {
    let ast = synthetic_large_ast(900);
    let map = PragmaTracker::build(&ast);
    let max_offset = map.last().map_or(2_048, |(range, _)| range.end).max(2_048);
    let offsets = deterministic_offsets(max_offset, 4_096);

    // Use bench_function so Criterion registers the stable ID as
    // "scope_analyzer_walk_style" (not "scope_analyzer_walk_style/dense_offsets"
    // which bench_with_input + BenchmarkId would produce).
    let mut group = c.benchmark_group("scope_analyzer_walk_style");
    group.throughput(Throughput::Elements(offsets.len() as u64));
    group.bench_function("scope_analyzer_walk_style", |b| {
        b.iter(|| {
            let mut scopes_with_strict = 0usize;
            for offset in &offsets {
                let state = PragmaTracker::state_for_offset(black_box(&map), *offset);
                if state.strict_vars && state.strict_subs && state.strict_refs {
                    scopes_with_strict = scopes_with_strict.saturating_add(1);
                }
                if state.locale {
                    scopes_with_strict = scopes_with_strict.saturating_add(1);
                }
            }
            black_box(scopes_with_strict)
        })
    });
    group.finish();
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
