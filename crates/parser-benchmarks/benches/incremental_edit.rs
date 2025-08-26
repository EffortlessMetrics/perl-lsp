use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use perl_parser::incremental::{Edit, IncrementalState, apply_edits};

fn bench_small_edit_large_doc(c: &mut Criterion) {
    let base = include_str!("../../../large_test.pl");
    let large = base.repeat(100);
    let edit = Edit { start_byte: 0, old_end_byte: 0, new_end_byte: 1, new_text: "#".to_string() };

    c.bench_function("incremental_small_edit_large_doc", |b| {
        b.iter_batched(
            || IncrementalState::new(large.clone()),
            |mut state| {
                apply_edits(&mut state, &[edit.clone()]).unwrap();
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench_small_edit_large_doc);
criterion_main!(benches);
