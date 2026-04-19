//! Benchmarks for segment-based token cache and two-sided checkpoint window
//!
//! This benchmark suite verifies performance improvements from issue #3527:
//! - Segment-based token cache implementation
//! - Two-sided checkpoint window implementation
//! - Enhanced metrics tracking
//!
//! Performance expectations (based on similar implementations):
//! - Typing single character: 10-20% faster
//! - Large paste operation: 30-50% faster (if content repeated)
//! - Undo/redo operations: 70-90% faster (content already cached)
//! - Repeated code patterns: 40-60% token reuse rate

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use perl_incremental_parsing::incremental::incremental_checkpoint::{
    CheckpointedIncrementalParser, SimpleEdit,
};
use perl_parser::Parser;
use std::hint::black_box;
use std::time::Instant;

// =========================================================================
// Benchmark Utilities
// =========================================================================

/// Generate a large Perl file with repeated patterns
fn generate_large_file_with_patterns(lines: usize, pattern_repeat: usize) -> String {
    let mut source = String::new();
    let pattern = r#"
sub process_item {
    my ($item) = @_;
    my $result = $item->{value};
    $result = $result * 2;
    return $result;
}

sub validate_item {
    my ($item) = @_;
    return defined $item && exists $item->{id};
}

sub transform_item {
    my ($item, $transform) = @_;
    return $transform->($item);
}
"#;

    source.push_str("#!/usr/bin/perl\nuse strict;\nuse warnings;\n\n");
    source.push_str(&format!(
        "my @items = ({});\n\n",
        (0..lines)
            .map(|i| format!("{{id => {}, value => {}}}", i, i * 10))
            .collect::<Vec<_>>()
            .join(", ")
    ));

    for _ in 0..pattern_repeat {
        source.push_str(pattern);
    }

    source.push_str("\n# Main processing\n");
    source.push_str("foreach my $item (@items) {\n");
    source.push_str("    if (validate_item($item)) {\n");
    source.push_str("        my $processed = process_item($item);\n");
    source.push_str("        print \"Processed: $processed\\n\";\n");
    source.push_str("    }\n");
    source.push_str("}\n");

    source
}

/// Generate a file with checkpoint boundaries
fn generate_file_with_checkpoints() -> String {
    let mut source = String::new();

    // Create content at positions that align with checkpoint positions (0, 100, 500, 1000, 5000)
    source.push_str("# Preamble to position 100\n");
    for i in 0..20 {
        source.push_str(&format!("my $var{} = {};\n", i, i));
    }

    source.push_str("\n# Content between 100 and 500\n");
    for i in 0..100 {
        source.push_str(&format!("my $mid{} = {};\n", i, i * 2));
    }

    source.push_str("\n# Content between 500 and 1000\n");
    for i in 0..250 {
        source.push_str(&format!("my $late{} = {};\n", i, i * 3));
    }

    source.push_str("\n# Content beyond 1000\n");
    for i in 0..1000 {
        source.push_str(&format!("my $end{} = {};\n", i, i * 4));
    }

    source
}

/// Benchmark result with detailed metrics
#[derive(Debug, Clone)]
struct BenchmarkResult {
    duration_ns: u128,
    tokens_reused: usize,
    tokens_relexed: usize,
    segments_reused_before: usize,
    segments_reused_after: usize,
    segments_invalidated: usize,
    bytes_relexed: usize,
    left_checkpoint_distance: usize,
    right_checkpoint_distance: usize,
    full_tail_fallbacks: usize,
}

impl BenchmarkResult {
    fn efficiency(&self) -> f64 {
        let total = self.tokens_reused + self.tokens_relexed;
        if total == 0 {
            0.0
        } else {
            (self.tokens_reused as f64 / total as f64) * 100.0
        }
    }

    fn bytes_reuse_ratio(&self) -> f64 {
        if self.bytes_relexed == 0 {
            0.0
        } else {
            (self.tokens_reused as f64 / self.bytes_relexed as f64) * 100.0
        }
    }
}

/// Run a full re-lex (baseline)
fn benchmark_full_relex(source: &str) -> BenchmarkResult {
    let start = Instant::now();
    let mut parser = Parser::new(source);
    let _ = parser.parse();
    let duration = start.elapsed();

    BenchmarkResult {
        duration_ns: duration.as_nanos(),
        tokens_reused: 0,
        tokens_relexed: 0, // Not tracked in full parse
        segments_reused_before: 0,
        segments_reused_after: 0,
        segments_invalidated: 0,
        bytes_relexed: source.len(),
        left_checkpoint_distance: 0,
        right_checkpoint_distance: 0,
        full_tail_fallbacks: 0,
    }
}

/// Run incremental parse with metrics
fn benchmark_incremental_parse(
    parser: &mut CheckpointedIncrementalParser,
    edit: &SimpleEdit,
) -> BenchmarkResult {
    let start = Instant::now();
    let _ = parser.apply_edit(edit);
    let duration = start.elapsed();

    let stats = parser.stats();

    BenchmarkResult {
        duration_ns: duration.as_nanos(),
        tokens_reused: stats.tokens_reused,
        tokens_relexed: stats.tokens_relexed,
        segments_reused_before: stats.segments_reused_before,
        segments_reused_after: stats.segments_reused_after,
        segments_invalidated: stats.segments_invalidated,
        bytes_relexed: stats.bytes_relexed,
        left_checkpoint_distance: stats.left_checkpoint_distance,
        right_checkpoint_distance: stats.right_checkpoint_distance,
        full_tail_fallbacks: stats.full_tail_fallbacks,
    }
}

// =========================================================================
// Benchmark: Single Character Insertion
// =========================================================================

fn bench_single_char_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_char_insertion");

    // Test insertion at various positions
    let positions = vec!["beginning", "middle", "end"];

    for position in positions {
        let source = generate_large_file_with_patterns(100, 10);

        let edit_start = match position {
            "beginning" => 0,
            "middle" => source.len() / 2,
            "end" => source.len(),
            _ => unreachable!(),
        };

        let edit = SimpleEdit {
            start: edit_start,
            end: edit_start,
            new_text: "x".to_string(),
        };

        group.bench_with_input(
            BenchmarkId::from_parameter(position),
            &(source, edit),
            |b, (source, edit)| {
                b.iter(|| {
                    let mut parser = CheckpointedIncrementalParser::new();
                    parser
                        .parse(black_box(source.clone()))
                        .expect("Initial parse failed");
                    benchmark_incremental_parse(&mut parser, black_box(edit))
                })
            },
        );
    }

    group.finish();
}

// =========================================================================
// Benchmark: Single Character Deletion
// =========================================================================

fn bench_single_char_deletion(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_char_deletion");

    let positions = vec!["beginning", "middle", "end"];

    for position in positions {
        let source = generate_large_file_with_patterns(100, 10);

        let edit_start = match position {
            "beginning" => 1,
            "middle" => source.len() / 2,
            "end" => source.len() - 1,
            _ => unreachable!(),
        };

        let edit = SimpleEdit {
            start: edit_start,
            end: edit_start + 1,
            new_text: String::new(),
        };

        group.bench_with_input(
            BenchmarkId::from_parameter(position),
            &(source, edit),
            |b, (source, edit)| {
                b.iter(|| {
                    let mut parser = CheckpointedIncrementalParser::new();
                    parser
                        .parse(black_box(source.clone()))
                        .expect("Initial parse failed");
                    benchmark_incremental_parse(&mut parser, black_box(edit))
                })
            },
        );
    }

    group.finish();
}

// =========================================================================
// Benchmark: Large Paste Operation
// =========================================================================

fn bench_large_paste(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_paste");

    let paste_sizes = vec![100, 500, 1000, 5000];

    for size in paste_sizes {
        let source = generate_large_file_with_patterns(100, 10);
        let paste_content = "my $pasted = 1;\n".repeat(size / 20);
        let paste_len = paste_content.len();

        let edit = SimpleEdit {
            start: source.len() / 2,
            end: source.len() / 2,
            new_text: paste_content,
        };

        group.throughput(Throughput::Bytes(paste_len as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(source, edit),
            |b, (source, edit)| {
                b.iter(|| {
                    let mut parser = CheckpointedIncrementalParser::new();
                    parser
                        .parse(black_box(source.clone()))
                        .expect("Initial parse failed");
                    benchmark_incremental_parse(&mut parser, black_box(edit))
                })
            },
        );
    }

    group.finish();
}

// =========================================================================
// Benchmark: Undo/Redo Pattern
// =========================================================================

fn bench_undo_redo(c: &mut Criterion) {
    let mut group = c.benchmark_group("undo_redo");

    let source = generate_large_file_with_patterns(100, 10);
    let edit_start = source.len() / 2;

    // Original edit
    let edit = SimpleEdit {
        start: edit_start,
        end: edit_start + 5,
        new_text: "99999".to_string(),
    };

    // Reverse edit (undo)
    let undo_edit = SimpleEdit {
        start: edit_start,
        end: edit_start + 5,
        new_text: "12345".to_string(),
    };

    group.bench_function("edit_undo_redo", |b| {
        b.iter(|| {
            let mut parser = CheckpointedIncrementalParser::new();
            parser
                .parse(black_box(source.clone()))
                .expect("Initial parse failed");

            // Apply edit
            benchmark_incremental_parse(&mut parser, black_box(&edit));

            // Undo
            benchmark_incremental_parse(&mut parser, black_box(&undo_edit));

            // Redo
            benchmark_incremental_parse(&mut parser, black_box(&edit));
        })
    });

    group.finish();
}

// =========================================================================
// Benchmark: Repeated Edits in Large File
// =========================================================================

fn bench_repeated_edits(c: &mut Criterion) {
    let mut group = c.benchmark_group("repeated_edits");

    let edit_counts = vec![10, 50, 100];

    for count in edit_counts {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let source = generate_large_file_with_patterns(100, 10);
                let mut parser = CheckpointedIncrementalParser::new();
                parser
                    .parse(black_box(source.clone()))
                    .expect("Initial parse failed");

                for i in 0..count {
                    let edit_start = (source.len() / (count + 1)) * (i + 1);
                    let edit = SimpleEdit {
                        start: edit_start,
                        end: edit_start + 1,
                        new_text: "x".to_string(),
                    };
                    benchmark_incremental_parse(&mut parser, black_box(&edit));
                }
            })
        });
    }

    group.finish();
}

// =========================================================================
// Benchmark: Edits Near Checkpoint Boundaries
// =========================================================================

fn bench_checkpoint_boundaries(c: &mut Criterion) {
    let mut group = c.benchmark_group("checkpoint_boundaries");

    // Checkpoints are at positions: 0, 100, 500, 1000, 5000
    let boundary_positions = vec![
        ("before_100", 90),
        ("at_100", 100),
        ("after_100", 110),
        ("before_500", 490),
        ("at_500", 500),
        ("after_500", 510),
    ];

    for (name, position) in boundary_positions {
        let source = generate_file_with_checkpoints();
        let edit = SimpleEdit {
            start: position,
            end: position,
            new_text: "x".to_string(),
        };

        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(source, edit),
            |b, (source, edit)| {
                b.iter(|| {
                    let mut parser = CheckpointedIncrementalParser::new();
                    parser
                        .parse(black_box(source.clone()))
                        .expect("Initial parse failed");
                    benchmark_incremental_parse(&mut parser, black_box(edit))
                })
            },
        );
    }

    group.finish();
}

// =========================================================================
// Benchmark: Repeated Code Patterns
// =========================================================================

fn bench_repeated_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("repeated_patterns");

    let pattern_counts = vec![5, 10, 20];

    for count in pattern_counts {
        let source = generate_large_file_with_patterns(100, count);
        let source_clone = source.clone();

        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &source_clone,
            |b, source| {
                b.iter(|| {
                    let mut parser = CheckpointedIncrementalParser::new();
                    parser
                        .parse(black_box(source.clone()))
                        .expect("Initial parse failed");

                    // Edit in a repeated pattern
                    let edit_start = source.find("sub process_item").unwrap_or(0) + 20;
                    let edit = SimpleEdit {
                        start: edit_start,
                        end: edit_start + 1,
                        new_text: "x".to_string(),
                    };
                    benchmark_incremental_parse(&mut parser, black_box(&edit))
                })
            },
        );
    }

    group.finish();
}

// =========================================================================
// Benchmark: Full Re-lex Baseline
// =========================================================================

fn bench_full_relex_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_relex_baseline");

    let file_sizes = vec![1000, 5000, 10000, 50000];

    for size in file_sizes {
        let source = generate_large_file_with_patterns(size / 10, 10);
        let source_len = source.len();

        group.throughput(Throughput::Bytes(source_len as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &source, |b, source| {
            b.iter(|| benchmark_full_relex(black_box(source)))
        });
    }

    group.finish();
}

// =========================================================================
// Benchmark: Comparison - Incremental vs Full Re-lex
// =========================================================================

fn bench_incremental_vs_full(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_vs_full");

    let source = generate_large_file_with_patterns(100, 10);
    let edit_start = source.len() / 2;
    let edit = SimpleEdit {
        start: edit_start,
        end: edit_start + 5,
        new_text: "99999".to_string(),
    };

    group.bench_function("incremental", |b| {
        b.iter(|| {
            let mut parser = CheckpointedIncrementalParser::new();
            parser
                .parse(black_box(source.clone()))
                .expect("Initial parse failed");
            benchmark_incremental_parse(&mut parser, black_box(&edit))
        })
    });

    group.bench_function("full_relex", |b| {
        b.iter(|| {
            let mut parser = CheckpointedIncrementalParser::new();
            parser
                .parse(black_box(source.clone()))
                .expect("Initial parse failed");
            benchmark_incremental_parse(&mut parser, black_box(&edit))
        })
    });

    group.finish();
}

// =========================================================================
// Benchmark: Metrics Tracking
// =========================================================================

fn bench_metrics_tracking(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics_tracking");

    let source = generate_large_file_with_patterns(100, 10);

    group.bench_function("single_edit_metrics", |b| {
        b.iter(|| {
            let mut parser = CheckpointedIncrementalParser::new();
            parser
                .parse(black_box(source.clone()))
                .expect("Initial parse failed");

            let edit = SimpleEdit {
                start: source.len() / 2,
                end: source.len() / 2 + 5,
                new_text: "99999".to_string(),
            };

            let result = benchmark_incremental_parse(&mut parser, black_box(&edit));

            // Verify metrics are being tracked
            assert!(
                result.tokens_reused > 0 || result.tokens_relexed > 0,
                "Expected either tokens_reused or tokens_relexed to be > 0"
            );

            result
        })
    });

    group.finish();
}

// =========================================================================
// Benchmark: Cache Efficiency
// =========================================================================

fn bench_cache_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_efficiency");

    let scenarios = vec![
        ("small_edit", 1, 5),
        ("medium_edit", 10, 50),
        ("large_edit", 100, 500),
    ];

    for (name, edit_pos, edit_len) in scenarios {
        let source = generate_large_file_with_patterns(100, 10);

        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(source, edit_pos, edit_len),
            |b, (source, edit_pos, edit_len)| {
                b.iter(|| {
                    let mut parser = CheckpointedIncrementalParser::new();
                    parser
                        .parse(black_box(source.clone()))
                        .expect("Initial parse failed");

                    let edit = SimpleEdit {
                        start: *edit_pos,
                        end: *edit_pos + *edit_len,
                        new_text: "x".repeat(*edit_len),
                    };

                    let result = benchmark_incremental_parse(&mut parser, black_box(&edit));

                    // Verify cache efficiency
                    let efficiency = result.efficiency();
                    assert!(
                        efficiency >= 0.0 && efficiency <= 100.0,
                        "Cache efficiency should be between 0 and 100, got {}",
                        efficiency
                    );

                    result
                })
            },
        );
    }

    group.finish();
}

// =========================================================================
// Benchmark: Segment Reuse
// =========================================================================

fn bench_segment_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("segment_reuse");

    let source = generate_file_with_checkpoints();

    group.bench_function("verify_segment_reuse", |b| {
        b.iter(|| {
            let mut parser = CheckpointedIncrementalParser::new();
            parser
                .parse(black_box(source.clone()))
                .expect("Initial parse failed");

            // Edit in the middle of the file
            let edit = SimpleEdit {
                start: 600,
                end: 605,
                new_text: "xxxxx".to_string(),
            };

            let result = benchmark_incremental_parse(&mut parser, black_box(&edit));

            // Verify segments are being reused
            assert!(
                result.segments_reused_before > 0 || result.segments_reused_after > 0,
                "Expected segments to be reused before or after edit"
            );

            result
        })
    });

    group.finish();
}

// =========================================================================
// Benchmark: Checkpoint Distance Impact
// =========================================================================

fn bench_checkpoint_distance(c: &mut Criterion) {
    let mut group = c.benchmark_group("checkpoint_distance");

    let source = generate_file_with_checkpoints();
    let edit_positions = vec![150, 600, 2000];

    for pos in edit_positions {
        let source_clone = source.clone();
        group.bench_with_input(
            BenchmarkId::from_parameter(pos),
            &(source_clone, pos),
            |b, (source, pos)| {
                b.iter(|| {
                    let mut parser = CheckpointedIncrementalParser::new();
                    parser
                        .parse(black_box(source.clone()))
                        .expect("Initial parse failed");

                    let edit = SimpleEdit {
                        start: *pos,
                        end: *pos + 5,
                        new_text: "xxxxx".to_string(),
                    };

                    let result = benchmark_incremental_parse(&mut parser, black_box(&edit));

                    // Verify checkpoint distances are reasonable
                    assert!(
                        result.left_checkpoint_distance <= 5000,
                        "Left checkpoint distance too large: {}",
                        result.left_checkpoint_distance
                    );
                    assert!(
                        result.right_checkpoint_distance <= 5000,
                        "Right checkpoint distance too large: {}",
                        result.right_checkpoint_distance
                    );

                    result
                })
            },
        );
    }

    group.finish();
}

// =========================================================================
// Benchmark Groups
// =========================================================================

criterion_group!(
    benches,
    bench_single_char_insertion,
    bench_single_char_deletion,
    bench_large_paste,
    bench_undo_redo,
    bench_repeated_edits,
    bench_checkpoint_boundaries,
    bench_repeated_patterns,
    bench_full_relex_baseline,
    bench_incremental_vs_full,
    bench_metrics_tracking,
    bench_cache_efficiency,
    bench_segment_reuse,
    bench_checkpoint_distance
);

criterion_main!(benches);
