use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use perl_parser::incremental::{Edit, IncrementalState, apply_edits};
use perl_parser::incremental_document::IncrementalDocument;
use perl_parser::incremental_edit::{IncrementalEdit, IncrementalEditSet};
use perl_tdd_support::{must, must_some};
use std::hint::black_box;
use std::sync::Once;

#[path = "support/scorecard.rs"]
mod scorecard;

static SCORECARD_ONCE: Once = Once::new();

fn emit_incremental_scorecard() {
    SCORECARD_ONCE.call_once(|| {
        let source = r#"
use strict;
use warnings;

sub process_data {
    my ($data) = @_;

    # Process each item
    for my $item (@$data) {
        my $result = transform($item);
        print "Result: $result\n";
    }

    return 1;
}

sub transform {
    my ($value) = @_;
    return $value * 2;
}

my $items = [1, 2, 3, 4, 5];
process_data($items);
"#
        .to_string();

        let transform_start = must_some(source.find("transform"));
        let transform_end = transform_start + "transform".len();
        let path = scorecard::scorecard_path();

        let cold_parse = scorecard::measure_samples_us(|| {
            let state = IncrementalState::new(source.clone());
            black_box(&state.ast);
        });
        let _ = scorecard::upsert_metric(&path, "cold_parse", "incremental_benchmark", cold_parse);

        let warm_reparse = scorecard::measure_samples_us(|| {
            let mut state = IncrementalState::new(source.clone());
            must(apply_edits(&mut state, &[]));
            black_box(&state.ast);
        });
        let _ =
            scorecard::upsert_metric(&path, "warm_reparse", "incremental_benchmark", warm_reparse);

        let small_edit = scorecard::measure_samples_us(|| {
            let mut state = IncrementalState::new(source.clone());
            let edit = Edit {
                start_byte: transform_start,
                old_end_byte: transform_end,
                new_end_byte: transform_start + "process".len(),
                new_text: "process".to_string(),
            };
            must(apply_edits(&mut state, &[edit]));
            black_box(&state.ast);
        });
        let _ = scorecard::upsert_metric(
            &path,
            "incremental_small_edit",
            "incremental_benchmark",
            small_edit,
        );

        let tiny = "my $x = 1;\nmy $y = 2;\nmy $z = 3;\nprint \"$x $y $z\\n\";\n".to_string();
        let pos_1 = must_some(tiny.find("= 1")) + 2;
        let pos_2 = must_some(tiny.find("= 2")) + 2;
        let multiple_edits = scorecard::measure_samples_us(|| {
            let mut state = IncrementalState::new(tiny.clone());
            let edits = vec![
                Edit {
                    start_byte: pos_1,
                    old_end_byte: pos_1 + 1,
                    new_end_byte: pos_1 + 2,
                    new_text: "10".to_string(),
                },
                Edit {
                    start_byte: pos_2,
                    old_end_byte: pos_2 + 1,
                    new_end_byte: pos_2 + 2,
                    new_text: "20".to_string(),
                },
            ];
            must(apply_edits(&mut state, &edits));
            black_box(&state.ast);
        });
        let _ = scorecard::upsert_metric(
            &path,
            "incremental_multiple_edits",
            "incremental_benchmark",
            multiple_edits,
        );
    });
}

fn bench_incremental_small_edit(c: &mut Criterion) {
    emit_incremental_scorecard();

    let source = r#"
use strict;
use warnings;

sub process_data {
    my ($data) = @_;

    # Process each item
    for my $item (@$data) {
        my $result = transform($item);
        print "Result: $result\n";
    }

    return 1;
}

sub transform {
    my ($value) = @_;
    return $value * 2;
}

my $items = [1, 2, 3, 4, 5];
process_data($items);
"#
    .to_string();

    let start = must_some(source.find("transform"));
    let old_end = start + "transform".len();

    c.bench_function("incremental small edit", |b| {
        b.iter_batched(
            || IncrementalState::new(source.clone()),
            |mut state| {
                let edit = Edit {
                    start_byte: start,
                    old_end_byte: old_end,
                    new_end_byte: start + "process".len(),
                    new_text: "process".to_string(),
                };
                must(apply_edits(&mut state, &[edit]));
                black_box(&state.ast);
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_full_reparse(c: &mut Criterion) {
    emit_incremental_scorecard();

    let source = r#"
use strict;
use warnings;

sub process_data {
    my ($data) = @_;

    # Process each item
    for my $item (@$data) {
        my $result = transform($item);
        print "Result: $result\n";
    }

    return 1;
}

sub transform {
    my ($value) = @_;
    return $value * 2;
}

my $items = [1, 2, 3, 4, 5];
process_data($items);
"#
    .to_string();

    c.bench_function("full reparse", |b| {
        b.iter(|| {
            let state = IncrementalState::new(black_box(source.clone()));
            black_box(&state.ast);
        })
    });
}

/// Warm reparse: build an `IncrementalState` once, then measure the cost of
/// reparsing the same content from an already-allocated state via
/// `apply_edits(&mut state, &[])`, which internally takes the `full_reparse`
/// path without the cold-start allocations incurred by `IncrementalState::new`.
///
/// This is the missing third regime of the cold / warm / incremental trifecta
/// called out in the #4063 parser scorecard plan-review (the pyright phase-
/// timing lesson and the rust-analyzer/gopls cold-vs-warm separation).
///
/// - `bench_full_reparse` — cold: fresh allocation of state, rope, line_index,
///   AST, and tokens (everything paid from scratch).
/// - `bench_warm_reparse` — warm: allocator warm, state object reused, content
///   reparsed via `apply_edits(&mut state, &[])`.
/// - `bench_incremental_small_edit` — incremental: allocator warm, state
///   reused, single small edit applied via the checkpoint-driven incremental
///   lexing path.
fn bench_warm_reparse(c: &mut Criterion) {
    emit_incremental_scorecard();

    let source = r#"
use strict;
use warnings;

sub process_data {
    my ($data) = @_;

    # Process each item
    for my $item (@$data) {
        my $result = transform($item);
        print "Result: $result\n";
    }

    return 1;
}

sub transform {
    my ($value) = @_;
    return $value * 2;
}

my $items = [1, 2, 3, 4, 5];
process_data($items);
"#
    .to_string();

    c.bench_function("warm reparse", |b| {
        b.iter_batched(
            || IncrementalState::new(source.clone()),
            |mut state| {
                // Empty edit list triggers the warm full_reparse path
                // without recreating the outer IncrementalState allocation.
                must(apply_edits(&mut state, &[]));
                black_box(&state.ast);
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_multiple_edits(c: &mut Criterion) {
    emit_incremental_scorecard();

    let source = r#"
my $x = 1;
my $y = 2;
my $z = 3;
print "$x $y $z\n";
"#
    .to_string();

    let pos_1 = must_some(source.find("= 1")) + 2;
    let pos_2 = must_some(source.find("= 2")) + 2;

    c.bench_function("incremental multiple edits", |b| {
        b.iter_batched(
            || IncrementalState::new(source.clone()),
            |mut state| {
                let edits = vec![
                    Edit {
                        start_byte: pos_1,
                        old_end_byte: pos_1 + 1,
                        new_end_byte: pos_1 + 2,
                        new_text: "10".to_string(),
                    },
                    Edit {
                        start_byte: pos_2,
                        old_end_byte: pos_2 + 1,
                        new_end_byte: pos_2 + 2,
                        new_text: "20".to_string(),
                    },
                ];
                must(apply_edits(&mut state, &edits));
                black_box(&state.ast);
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_incremental_document_single_edit(c: &mut Criterion) {
    let source = "my $x = 42; my $y = 100; print $x + $y;";
    let start = must_some(source.find("42"));
    let end = start + 2;

    c.bench_function("incremental_document single edit", |b| {
        b.iter_batched(
            || must(IncrementalDocument::new(source.to_string())),
            |mut doc| {
                let edit = IncrementalEdit::new(start, end, "43".to_string());
                must(doc.apply_edit(edit));
                black_box(doc.metrics.nodes_reused);
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_incremental_document_multiple_edits(c: &mut Criterion) {
    let source = "sub calc { my $a = 10; my $b = 20; $a + $b }";
    let pos_a = must_some(source.find("10"));
    let pos_b = must_some(source.find("20"));

    c.bench_function("incremental_document multiple edits", |b| {
        b.iter_batched(
            || must(IncrementalDocument::new(source.to_string())),
            |mut doc| {
                let mut edits = IncrementalEditSet::new();
                edits.add(IncrementalEdit::new(pos_a, pos_a + 2, "15".to_string()));
                edits.add(IncrementalEdit::new(pos_b, pos_b + 2, "25".to_string()));
                must(doc.apply_edits(&edits));
                black_box(doc.metrics.nodes_reused);
            },
            BatchSize::SmallInput,
        );
    });
}

// Cold / warm / incremental regime group — the three parse regimes a
// language server has to care about, instrumented together so their p50/p95
// estimates land in sibling Criterion reports under the same group name.
//
// See `docs/project/metrics/parser.md` ("Cold / warm / incremental regimes")
// and issue #4063 for the rationale.
criterion_group!(
    parse_regime,
    bench_full_reparse,           // cold
    bench_warm_reparse,           // warm
    bench_incremental_small_edit, // incremental
);

criterion_group!(
    benches,
    bench_multiple_edits,
    bench_incremental_document_single_edit,
    bench_incremental_document_multiple_edits,
);
criterion_main!(parse_regime, benches);
