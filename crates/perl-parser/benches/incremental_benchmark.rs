#[cfg(feature = "incremental")]
use criterion::BatchSize;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

#[cfg(feature = "incremental")]
use perl_parser::incremental::{Edit, IncrementalState, apply_edits};
#[cfg(feature = "incremental")]
use perl_parser::incremental_document::IncrementalDocument;
#[cfg(feature = "incremental")]
use perl_parser::incremental_edit::{IncrementalEdit, IncrementalEditSet};
#[cfg(feature = "incremental")]
use perl_tdd_support::{must, must_some};
#[cfg(feature = "incremental")]
use std::sync::OnceLock;

#[cfg(feature = "incremental")]
mod scorecard_artifact;

#[cfg(feature = "incremental")]
fn bench_incremental_small_edit(c: &mut Criterion) {
    emit_incremental_scorecard_metrics();
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

#[cfg(feature = "incremental")]
fn bench_full_reparse(c: &mut Criterion) {
    emit_incremental_scorecard_metrics();
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

#[cfg(feature = "incremental")]
fn bench_warm_reparse(c: &mut Criterion) {
    emit_incremental_scorecard_metrics();
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
                must(apply_edits(&mut state, &[]));
                black_box(&state.ast);
            },
            BatchSize::SmallInput,
        );
    });
}

#[cfg(feature = "incremental")]
fn bench_multiple_edits(c: &mut Criterion) {
    emit_incremental_scorecard_metrics();
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

#[cfg(feature = "incremental")]
fn bench_incremental_document_single_edit(c: &mut Criterion) {
    emit_incremental_scorecard_metrics();
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

#[cfg(feature = "incremental")]
fn bench_incremental_document_multiple_edits(c: &mut Criterion) {
    emit_incremental_scorecard_metrics();
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

#[cfg(feature = "incremental")]
criterion_group!(
    parse_regime,
    bench_full_reparse,
    bench_warm_reparse,
    bench_incremental_small_edit,
);

#[cfg(feature = "incremental")]
criterion_group!(
    benches,
    bench_multiple_edits,
    bench_incremental_document_single_edit,
    bench_incremental_document_multiple_edits,
);

#[cfg(feature = "incremental")]
criterion_main!(parse_regime, benches);

#[cfg(feature = "incremental")]
fn emit_incremental_scorecard_metrics() {
    static EMITTED: OnceLock<()> = OnceLock::new();
    if EMITTED.get().is_some() {
        return;
    }

    let source = r#"
use strict;
use warnings;

sub process_data {
    my ($data) = @_;
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

    scorecard_artifact::upsert_metric(
        "warm_reparse",
        scorecard_artifact::measure(200, || {
            let mut state = IncrementalState::new(source.clone());
            let _ = apply_edits(&mut state, &[]);
            black_box(&state.ast);
        }),
    );

    let incremental_small = scorecard_artifact::measure(200, || {
        let mut state = IncrementalState::new(source.clone());
        let edit = Edit {
            start_byte: start,
            old_end_byte: old_end,
            new_end_byte: start + "process".len(),
            new_text: "process".to_string(),
        };
        let _ = apply_edits(&mut state, &[edit]);
        black_box(&state.ast);
    });
    scorecard_artifact::upsert_metric("incremental_small_edit", incremental_small);

    let multi_source = r#"
my $x = 1;
my $y = 2;
my $z = 3;
print "$x $y $z\n";
"#
    .to_string();
    let pos_1 = must_some(multi_source.find("= 1")) + 2;
    let pos_2 = must_some(multi_source.find("= 2")) + 2;
    let incremental_multi = scorecard_artifact::measure(200, || {
        let mut state = IncrementalState::new(multi_source.clone());
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
        let _ = apply_edits(&mut state, &edits);
        black_box(&state.ast);
    });
    scorecard_artifact::upsert_metric("incremental_multiple_edits", incremental_multi);

    let _ = EMITTED.set(());
}

#[cfg(not(feature = "incremental"))]
fn bench_incremental_feature_placeholder(c: &mut Criterion) {
    c.bench_function("incremental feature disabled", |b| {
        b.iter(|| {
            black_box(0usize);
        })
    });
}

#[cfg(not(feature = "incremental"))]
criterion_group!(benches, bench_incremental_feature_placeholder);

#[cfg(not(feature = "incremental"))]
criterion_main!(benches);
