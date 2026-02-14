use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use perl_parser::incremental::{Edit, IncrementalState, apply_edits};
use perl_parser::incremental_document::IncrementalDocument;
use perl_parser::incremental_edit::{IncrementalEdit, IncrementalEditSet};
use std::hint::black_box;
use perl_tdd_support::{must, must_some};

fn bench_incremental_small_edit(c: &mut Criterion) {
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

    let mut state = IncrementalState::new(source);

    c.bench_function("incremental small edit", |b| {
        b.iter(|| {
            // Edit line 9: change "transform" to "process"
            let edit = Edit {
                start_byte: 180, // approximate position
                old_end_byte: 189,
                new_end_byte: 187,
                new_text: "process".to_string(),
            };
            must(apply_edits(&mut state, &[edit]));
            black_box(&state.ast);
        })
    });
}

fn bench_full_reparse(c: &mut Criterion) {
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

fn bench_multiple_edits(c: &mut Criterion) {
    let source = r#"
my $x = 1;
my $y = 2;
my $z = 3;
print "$x $y $z\n";
"#
    .to_string();

    let mut state = IncrementalState::new(source);

    c.bench_function("incremental multiple edits", |b| {
        b.iter(|| {
            let edits = vec![
                Edit {
                    start_byte: 8,
                    old_end_byte: 9,
                    new_end_byte: 10,
                    new_text: "10".to_string(),
                },
                Edit {
                    start_byte: 19,
                    old_end_byte: 20,
                    new_end_byte: 10,
                    new_text: "20".to_string(),
                },
            ];
            must(apply_edits(&mut state, &edits));
            black_box(&state.ast);
        })
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

criterion_group!(
    benches,
    bench_incremental_small_edit,
    bench_full_reparse,
    bench_multiple_edits,
    bench_incremental_document_single_edit,
    bench_incremental_document_multiple_edits,
);
criterion_main!(benches);
