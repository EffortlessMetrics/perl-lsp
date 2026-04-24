use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use perl_parser::incremental::{Edit, IncrementalState, apply_edits};
use perl_parser::incremental_document::IncrementalDocument;
use perl_parser::incremental_edit::{IncrementalEdit, IncrementalEditSet};
use perl_tdd_support::{must, must_some};
use serde::{Deserialize, Serialize};
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const SCORECARD_FILENAME: &str = "parser_performance_scorecard.json";

#[derive(Debug, Clone)]
struct ScorecardSample {
    scenario: &'static str,
    unit: &'static str,
    iterations: usize,
    mean_ns: u64,
    median_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ParserPerformanceScorecard {
    schema_version: u32,
    measured_at_unix_s: u64,
    results: Vec<ScorecardEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScorecardEntry {
    scenario: String,
    unit: String,
    iterations: usize,
    mean_ns: u64,
    median_ns: u64,
}

fn run_scorecard_sample(iterations: usize, mut op: impl FnMut()) -> (u64, u64) {
    let mut samples: Vec<u64> = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        op();
        let elapsed = started.elapsed().as_nanos();
        let sample = u64::try_from(elapsed).unwrap_or(u64::MAX);
        samples.push(sample);
    }
    samples.sort_unstable();
    let total: u128 = samples.iter().map(|&v| u128::from(v)).sum();
    let mean_ns = u64::try_from(total / iterations as u128).unwrap_or(u64::MAX);
    let median_ns = samples[iterations / 2];
    (mean_ns, median_ns)
}

fn emit_scorecard(samples: &[ScorecardSample]) {
    let path = scorecard_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut scorecard = fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<ParserPerformanceScorecard>(&raw).ok())
        .unwrap_or_default();
    scorecard.schema_version = 1;
    scorecard.measured_at_unix_s =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_secs());

    for sample in samples {
        let entry = ScorecardEntry {
            scenario: sample.scenario.to_string(),
            unit: sample.unit.to_string(),
            iterations: sample.iterations,
            mean_ns: sample.mean_ns,
            median_ns: sample.median_ns,
        };
        if let Some(existing) =
            scorecard.results.iter_mut().find(|row| row.scenario == entry.scenario)
        {
            *existing = entry;
        } else {
            scorecard.results.push(entry);
        }
    }
    scorecard.results.sort_by(|a, b| a.scenario.cmp(&b.scenario));

    if let Ok(raw) = serde_json::to_string_pretty(&scorecard) {
        let _ = fs::write(path, raw);
    }
}

fn scorecard_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("metrics")
        .join(SCORECARD_FILENAME)
}

fn sample_full_reparse() -> ScorecardSample {
    let source = r#"
use strict;
use warnings;
sub process_data { my ($data) = @_; for my $item (@$data) { my $result = transform($item); print "Result: $result\n"; } return 1; }
sub transform { my ($value) = @_; return $value * 2; }
my $items = [1, 2, 3, 4, 5];
process_data($items);
"#
    .to_string();
    let iterations = 25;
    let (mean_ns, median_ns) = run_scorecard_sample(iterations, || {
        let state = IncrementalState::new(source.clone());
        black_box(&state.ast);
    });
    ScorecardSample { scenario: "cold_parse", unit: "ns", iterations, mean_ns, median_ns }
}

fn sample_warm_reparse() -> ScorecardSample {
    let source = r#"
use strict;
use warnings;
sub process_data { my ($data) = @_; for my $item (@$data) { my $result = transform($item); print "Result: $result\n"; } return 1; }
sub transform { my ($value) = @_; return $value * 2; }
my $items = [1, 2, 3, 4, 5];
process_data($items);
"#
    .to_string();
    let iterations = 25;
    let (mean_ns, median_ns) = run_scorecard_sample(iterations, || {
        let mut state = IncrementalState::new(source.clone());
        must(apply_edits(&mut state, &[]));
        black_box(&state.ast);
    });
    ScorecardSample { scenario: "warm_reparse", unit: "ns", iterations, mean_ns, median_ns }
}

fn sample_incremental_small_edit() -> ScorecardSample {
    let source = r#"
use strict;
use warnings;
sub process_data { my ($data) = @_; for my $item (@$data) { my $result = transform($item); print "Result: $result\n"; } return 1; }
sub transform { my ($value) = @_; return $value * 2; }
my $items = [1, 2, 3, 4, 5];
process_data($items);
"#
    .to_string();
    let start = must_some(source.find("transform"));
    let old_end = start + "transform".len();
    let iterations = 25;
    let (mean_ns, median_ns) = run_scorecard_sample(iterations, || {
        let mut state = IncrementalState::new(source.clone());
        let edit = Edit {
            start_byte: start,
            old_end_byte: old_end,
            new_end_byte: start + "process".len(),
            new_text: "process".to_string(),
        };
        must(apply_edits(&mut state, &[edit]));
        black_box(&state.ast);
    });
    ScorecardSample {
        scenario: "incremental_small_edit",
        unit: "ns",
        iterations,
        mean_ns,
        median_ns,
    }
}

fn sample_incremental_multiple_edits() -> ScorecardSample {
    let source = r#"
my $x = 1;
my $y = 2;
my $z = 3;
print "$x $y $z\n";
"#
    .to_string();
    let pos_1 = must_some(source.find("= 1")) + 2;
    let pos_2 = must_some(source.find("= 2")) + 2;
    let iterations = 25;
    let (mean_ns, median_ns) = run_scorecard_sample(iterations, || {
        let mut state = IncrementalState::new(source.clone());
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
    ScorecardSample {
        scenario: "incremental_multiple_edits",
        unit: "ns",
        iterations,
        mean_ns,
        median_ns,
    }
}

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

fn bench_emit_scorecard(_c: &mut Criterion) {
    static SCORECARD_EMITTED: OnceLock<()> = OnceLock::new();
    _c.bench_function("emit_parser_performance_scorecard_incremental", |b| {
        b.iter(|| {
            SCORECARD_EMITTED.get_or_init(|| {
                let samples = vec![
                    sample_full_reparse(),
                    sample_warm_reparse(),
                    sample_incremental_small_edit(),
                    sample_incremental_multiple_edits(),
                ];
                emit_scorecard(&samples);
            });
        });
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
    bench_emit_scorecard,
);
criterion_main!(parse_regime, benches);
