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

fn package_block(name: &str, body: Node, start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::Package {
            name: name.to_string(),
            name_span: loc(start + 8, start + 8 + name.len()),
            block: Some(Box::new(body)),
        },
        location: loc(start, end),
    }
}

fn phase_block(phase: &str, body: Node, start: usize, end: usize) -> Node {
    Node {
        kind: NodeKind::PhaseBlock {
            phase: phase.to_string(),
            phase_span: Some(loc(start, start + phase.len())),
            block: Box::new(body),
        },
        location: loc(start, end),
    }
}

fn program(statements: Vec<Node>) -> Node {
    let end = statements.last().map_or(0, |node| node.location.end);
    Node { kind: NodeKind::Program { statements }, location: loc(0, end) }
}

fn synthetic_small_file_ast() -> Node {
    program(vec![
        use_node("strict", &[], 0, 12),
        use_node("warnings", &[], 13, 28),
        block(
            vec![no_node("warnings", &["deprecated"], 35, 60), use_node("utf8", &[], 61, 70)],
            30,
            72,
        ),
        use_node("feature", &["'signatures unicode_strings'"], 73, 110),
    ])
}

fn synthetic_large_file_ast() -> Node {
    let mut statements = Vec::new();
    let mut cursor = 0usize;

    for idx in 0..700usize {
        statements.push(use_node("strict", &[], cursor, cursor + 12));
        cursor += 13;

        let version = if idx % 4 == 0 { "v5.36" } else { "v5.40" };
        statements.push(use_node(version, &[], cursor, cursor + 11));
        cursor += 12;

        let package_name = format!("Bench::Pkg{idx}");
        let inner = block(
            vec![
                no_node("strict", &["refs"], cursor + 5, cursor + 25),
                use_node("warnings", &[], cursor + 26, cursor + 42),
                no_node("warnings", &["uninitialized"], cursor + 43, cursor + 72),
                use_node("builtin", &["qw(true false ceil floor)"], cursor + 73, cursor + 112),
            ],
            cursor,
            cursor + 116,
        );
        statements.push(package_block(&package_name, inner, cursor, cursor + 118));
        cursor += 119;

        let phase_inner = block(
            vec![use_node("feature", &["':5.36'"], cursor + 6, cursor + 25)],
            cursor + 5,
            cursor + 30,
        );
        statements.push(phase_block("BEGIN", phase_inner, cursor, cursor + 31));
        cursor += 32;
    }

    program(statements)
}

const REALISTIC_PARSED_FIXTURE: &str = r#"
use v5.40;
use strict;
use warnings;
use feature qw(signatures class);

package Demo::Parser {
    use feature 'try';

    class Worker {
        field $id :param;

        method run($input) {
            try {
                no warnings 'uninitialized';
                return $input // 'default';
            }
            catch ($e) {
                use warnings;
                return "failed: $e";
            }
        }
    }
}

package Legacy::Section;
{
    no strict 'refs';
    use builtin qw(true false);
}
"#;

fn parsed_realistic_fixture_ast() -> Option<Node> {
    let mut parser = perl_parser::Parser::new(REALISTIC_PARSED_FIXTURE);
    parser.parse().ok()
}

fn deterministic_offsets(limit: usize, count: usize) -> Vec<usize> {
    if limit == 0 || count == 0 {
        return Vec::new();
    }

    let mut offsets = Vec::with_capacity(count);
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;

    for _ in 0..count {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        offsets.push((seed as usize) % limit);
    }

    offsets
}

fn build_small_file(c: &mut Criterion) {
    let ast = synthetic_small_file_ast();

    c.bench_function("build_small_file", |b| {
        b.iter(|| {
            let map = PragmaTracker::build(black_box(&ast));
            black_box(map)
        })
    });
}

fn build_large_file(c: &mut Criterion) {
    let parsed_fixture = parsed_realistic_fixture_ast();
    let ast = parsed_fixture.unwrap_or_else(synthetic_large_file_ast);

    c.bench_function("build_large_file", |b| {
        b.iter(|| {
            let map = PragmaTracker::build(black_box(&ast));
            black_box(map)
        })
    });
}

fn query_random_offsets(c: &mut Criterion) {
    let ast = synthetic_large_file_ast();
    let map = PragmaTracker::build(&ast);
    let offsets = deterministic_offsets(ast.location.end.max(1), 512);

    c.bench_function("query_random_offsets", |b| {
        b.iter(|| {
            for offset in &offsets {
                black_box(PragmaTracker::state_for_offset(black_box(&map), *offset));
            }
        })
    });
}

fn query_monotonic_offsets(c: &mut Criterion) {
    let ast = synthetic_large_file_ast();
    let map = PragmaTracker::build(&ast);
    let upper = ast.location.end.max(1);
    let step = (upper / 512).max(1);
    let offsets: Vec<usize> = (0..upper).step_by(step).take(512).collect();

    c.bench_function("query_monotonic_offsets", |b| {
        b.iter(|| {
            for offset in &offsets {
                black_box(PragmaTracker::state_for_offset(black_box(&map), *offset));
            }
        })
    });
}

fn final_state_lookup(c: &mut Criterion) {
    let ast = synthetic_large_file_ast();
    let map = PragmaTracker::build(&ast);
    let final_offset = ast.location.end.saturating_sub(1);

    c.bench_function("final_state_lookup", |b| {
        b.iter(|| black_box(PragmaTracker::state_for_offset(black_box(&map), final_offset)))
    });
}

fn version_compat_walk_style(c: &mut Criterion) {
    let mut statements = Vec::new();
    let mut cursor = 0usize;

    for version in ["v5.10", "v5.12", "v5.16", "v5.20", "v5.34", "v5.36", "v5.38", "v5.40"] {
        statements.push(use_node(version, &[], cursor, cursor + 10));
        cursor += 11;
        statements.push(use_node("feature", &["':5.36'"], cursor, cursor + 20));
        cursor += 21;
        statements.push(no_node("feature", &["'switch'"], cursor, cursor + 40));
        cursor += 41;
        statements.push(use_node("warnings", &[], cursor, cursor + 55));
        cursor += 56;
    }

    let ast = program(statements);

    c.bench_function("version_compat_walk_style", |b| {
        b.iter(|| {
            let map = PragmaTracker::build(black_box(&ast));
            black_box(map)
        })
    });
}

fn scope_analyzer_walk_style(c: &mut Criterion) {
    let ast = program(vec![
        use_node("strict", &[], 0, 12),
        package_block(
            "Analyzer::One",
            block(
                vec![
                    use_node("warnings", &[], 20, 35),
                    block(
                        vec![
                            no_node("strict", &["subs"], 40, 58),
                            phase_block(
                                "BEGIN",
                                block(
                                    vec![
                                        use_node("feature", &["'signatures'"], 65, 90),
                                        no_node("warnings", &["deprecated"], 91, 117),
                                    ],
                                    62,
                                    120,
                                ),
                                60,
                                121,
                            ),
                        ],
                        38,
                        125,
                    ),
                ],
                18,
                130,
            ),
            13,
            132,
        ),
        package_block(
            "Analyzer::Two",
            block(
                vec![
                    use_node("v5.40", &[], 138, 149),
                    no_node("feature", &["'class'"], 150, 170),
                    use_node("builtin", &["qw(true false)"], 171, 198),
                ],
                136,
                201,
            ),
            133,
            203,
        ),
    ]);

    c.bench_function("scope_analyzer_walk_style", |b| {
        b.iter(|| {
            let map = PragmaTracker::build(black_box(&ast));
            black_box(map)
        })
    });
}

criterion_group!(
    benches,
    build_small_file,
    build_large_file,
    query_random_offsets,
    query_monotonic_offsets,
    final_state_lookup,
    version_compat_walk_style,
    scope_analyzer_walk_style
);
criterion_main!(benches);
