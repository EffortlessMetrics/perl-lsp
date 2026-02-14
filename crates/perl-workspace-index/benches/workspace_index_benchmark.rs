#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use perl_workspace_index::workspace_index::WorkspaceIndex;
use std::fs;
use std::hint::black_box;
use tempfile::TempDir;
use url::Url;
use perl_tdd_support::must;

/// Sample Perl code representing a typical module
const SAMPLE_MODULE: &str = r#"
package MyModule;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my $class = shift;
    return bless {}, $class;
}

sub process_data {
    my ($self, $data) = @_;

    for my $item (@$data) {
        my $result = $self->transform($item);
        print "Result: $result\n";
    }

    return 1;
}

sub transform {
    my ($self, $value) = @_;
    return $value * 2;
}

sub calculate {
    my ($self, $x, $y) = @_;
    return $x + $y;
}

1;
"#;

/// Sample Perl code with multiple packages
const MULTI_PACKAGE_MODULE: &str = r#"
package First::Module;
our $VERSION = '0.01';

sub first_sub {
    return "first";
}

package Second::Module;
our $VERSION = '0.02';

sub second_sub {
    return "second";
}

package Third::Module;
our $VERSION = '0.03';

sub third_sub {
    return "third";
}

1;
"#;

/// Sample script with various constructs
const SAMPLE_SCRIPT: &str = r#"
#!/usr/bin/env perl
use strict;
use warnings;
use MyModule;

my $obj = MyModule->new();

sub main {
    my @data = (1, 2, 3, 4, 5);
    $obj->process_data(\@data);
}

sub helper {
    my ($x) = @_;
    return $x * 3;
}

main();
"#;

/// Modern Perl features sample
const MODERN_PERL: &str = r#"
use v5.38;
use feature qw(signatures try);

sub add($x, $y) {
    return $x + $y;
}

sub safe_divide($a, $b) {
    try {
        die "Division by zero" if $b == 0;
        return $a / $b;
    }
    catch ($e) {
        warn "Error: $e";
        return undef;
    }
}

package Point {
    use feature 'class';

    field $x :param = 0;
    field $y :param = 0;

    method move($dx, $dy) {
        $x += $dx;
        $y += $dy;
    }
}

1;
"#;

/// Complex module with inheritance
const COMPLEX_MODULE: &str = r#"
package Complex::Module;
use strict;
use warnings;
use parent qw(Base::Class);

our $VERSION = '2.34';

sub new {
    my ($class, %args) = @_;
    my $self = $class->SUPER::new(%args);
    return bless $self, $class;
}

sub method_one {
    my ($self, $arg1, $arg2) = @_;
    return $self->helper($arg1) + $arg2;
}

sub method_two {
    my ($self, @items) = @_;
    my @results;
    for my $item (@items) {
        push @results, $self->process($item);
    }
    return @results;
}

sub helper {
    my ($self, $value) = @_;
    return $value * 2;
}

sub process {
    my ($self, $item) = @_;
    return $item->{data} // 0;
}

1;
"#;

/// Benchmark initial workspace indexing with a small workspace (5-10 files)
///
/// This verifies the <100ms initial indexing performance requirement from the roadmap.
fn bench_initial_index_small_workspace(c: &mut Criterion) {
    c.bench_function("initial index small workspace (5 files)", |b| {
        b.iter_batched(
            || {
                // Setup: create a temporary workspace with 5 files
                let temp_dir = must(TempDir::new());
                let base_path = temp_dir.path();

                // Create 5 different Perl files
                must(fs::write(base_path.join("module1.pm"), SAMPLE_MODULE));
                must(fs::write(base_path.join("module2.pm"), MULTI_PACKAGE_MODULE));
                must(fs::write(base_path.join("script.pl"), SAMPLE_SCRIPT));
                must(fs::write(base_path.join("modern.pm"), MODERN_PERL));
                must(fs::write(base_path.join("complex.pm"), COMPLEX_MODULE));

                (temp_dir, WorkspaceIndex::new())
            },
            |(temp_dir, index)| {
                // Benchmark: index all files
                let base_path = temp_dir.path();

                let uri1 = must(Url::from_file_path(base_path.join("module1.pm")));
                let uri2 = must(Url::from_file_path(base_path.join("module2.pm")));
                let uri3 = must(Url::from_file_path(base_path.join("script.pl")));
                let uri4 = must(Url::from_file_path(base_path.join("modern.pm")));
                let uri5 = must(Url::from_file_path(base_path.join("complex.pm")));

                index.index_file(uri1, SAMPLE_MODULE.to_string()).ok();
                index.index_file(uri2, MULTI_PACKAGE_MODULE.to_string()).ok();
                index.index_file(uri3, SAMPLE_SCRIPT.to_string()).ok();
                index.index_file(uri4, MODERN_PERL.to_string()).ok();
                index.index_file(uri5, COMPLEX_MODULE.to_string()).ok();

                black_box(&index);
            },
            BatchSize::SmallInput,
        );
    });
}

/// Benchmark initial workspace indexing with a medium workspace (10 files)
fn bench_initial_index_medium_workspace(c: &mut Criterion) {
    c.bench_function("initial index medium workspace (10 files)", |b| {
        b.iter_batched(
            || {
                let temp_dir = must(TempDir::new());
                let base_path = temp_dir.path();

                // Create 10 files by duplicating samples with variations
                for i in 0..10 {
                    let filename = format!("module{}.pm", i);
                    let content = match i % 5 {
                        0 => SAMPLE_MODULE,
                        1 => MULTI_PACKAGE_MODULE,
                        2 => SAMPLE_SCRIPT,
                        3 => MODERN_PERL,
                        _ => COMPLEX_MODULE,
                    };
                    must(fs::write(base_path.join(&filename), content));
                }

                (temp_dir, WorkspaceIndex::new())
            },
            |(temp_dir, index)| {
                let base_path = temp_dir.path();

                for i in 0..10 {
                    let filename = format!("module{}.pm", i);
                    let uri = must(Url::from_file_path(base_path.join(&filename)));
                    let content = match i % 5 {
                        0 => SAMPLE_MODULE,
                        1 => MULTI_PACKAGE_MODULE,
                        2 => SAMPLE_SCRIPT,
                        3 => MODERN_PERL,
                        _ => COMPLEX_MODULE,
                    };
                    index.index_file(uri, content.to_string()).ok();
                }

                black_box(&index);
            },
            BatchSize::SmallInput,
        );
    });
}

/// Benchmark incremental update (single file change in already-indexed workspace)
///
/// This verifies the <10ms incremental update performance requirement from the roadmap.
fn bench_incremental_update(c: &mut Criterion) {
    c.bench_function("incremental update single file", |b| {
        b.iter_batched(
            || {
                // Setup: create and index a workspace
                let temp_dir = must(TempDir::new());
                let base_path = temp_dir.path();

                must(fs::write(base_path.join("module1.pm"), SAMPLE_MODULE));
                must(fs::write(base_path.join("module2.pm"), MULTI_PACKAGE_MODULE));
                must(fs::write(base_path.join("script.pl"), SAMPLE_SCRIPT));
                must(fs::write(base_path.join("modern.pm"), MODERN_PERL));
                must(fs::write(base_path.join("complex.pm"), COMPLEX_MODULE));

                let index = WorkspaceIndex::new();

                // Initial indexing
                index
                    .index_file(
                        must(Url::from_file_path(base_path.join("module1.pm"))),
                        SAMPLE_MODULE.to_string(),
                    )
                    .ok();
                index
                    .index_file(
                        must(Url::from_file_path(base_path.join("module2.pm"))),
                        MULTI_PACKAGE_MODULE.to_string(),
                    )
                    .ok();
                index
                    .index_file(
                        must(Url::from_file_path(base_path.join("script.pl"))),
                        SAMPLE_SCRIPT.to_string(),
                    )
                    .ok();
                index
                    .index_file(
                        must(Url::from_file_path(base_path.join("modern.pm"))),
                        MODERN_PERL.to_string(),
                    )
                    .ok();
                index
                    .index_file(
                        must(Url::from_file_path(base_path.join("complex.pm"))),
                        COMPLEX_MODULE.to_string(),
                    )
                    .ok();

                let update_uri = must(Url::from_file_path(base_path.join("module1.pm")));

                (temp_dir, index, update_uri)
            },
            |(temp_dir, index, update_uri)| {
                // Benchmark: update a single file (simulating a user edit)
                let updated_content = r#"
package MyModule;
use strict;
use warnings;

our $VERSION = '2.00';  # Version changed

sub new {
    my $class = shift;
    return bless {}, $class;
}

sub process_data {
    my ($self, $data) = @_;

    for my $item (@$data) {
        my $result = $self->transform($item);
        print "Updated result: $result\n";  # Changed
    }

    return 1;
}

sub transform {
    my ($self, $value) = @_;
    return $value * 3;  # Changed multiplier
}

sub calculate {
    my ($self, $x, $y) = @_;
    return $x + $y;
}

sub new_method {  # New method added
    my ($self) = @_;
    return "new";
}

1;
"#;

                index.index_file(update_uri, updated_content.to_string()).ok();
                black_box(&index);
                black_box(temp_dir);
            },
            BatchSize::SmallInput,
        );
    });
}

/// Benchmark symbol lookup (O(1) hash table lookup)
///
/// Tests the performance of find_definition, which should be very fast
/// due to hash table indexing.
fn bench_symbol_lookup(c: &mut Criterion) {
    // Setup: create an indexed workspace once for all iterations
    let temp_dir = must(TempDir::new());
    let base_path = temp_dir.path();

    must(fs::write(base_path.join("module1.pm"), SAMPLE_MODULE));
    must(fs::write(base_path.join("module2.pm"), MULTI_PACKAGE_MODULE));
    must(fs::write(base_path.join("complex.pm"), COMPLEX_MODULE));

    let index = WorkspaceIndex::new();

    index
        .index_file(
            must(Url::from_file_path(base_path.join("module1.pm"))),
            SAMPLE_MODULE.to_string(),
        )
        .ok();
    index
        .index_file(
            must(Url::from_file_path(base_path.join("module2.pm"))),
            MULTI_PACKAGE_MODULE.to_string(),
        )
        .ok();
    index
        .index_file(
            must(Url::from_file_path(base_path.join("complex.pm"))),
            COMPLEX_MODULE.to_string(),
        )
        .ok();

    c.bench_function("symbol lookup by name", |b| {
        b.iter(|| {
            // Benchmark: lookup various symbols
            let def1 = index.find_definition("MyModule::process_data");
            let def2 = index.find_definition("First::Module::first_sub");
            let def3 = index.find_definition("Complex::Module::method_one");

            black_box(def1);
            black_box(def2);
            black_box(def3);
        });
    });
}

/// Benchmark find_references (cross-file reference lookup)
fn bench_find_references(c: &mut Criterion) {
    // Setup: create an indexed workspace
    let temp_dir = must(TempDir::new());
    let base_path = temp_dir.path();

    must(fs::write(base_path.join("module1.pm"), SAMPLE_MODULE));
    must(fs::write(base_path.join("module2.pm"), MULTI_PACKAGE_MODULE));
    must(fs::write(base_path.join("complex.pm"), COMPLEX_MODULE));

    let index = WorkspaceIndex::new();

    index
        .index_file(
            must(Url::from_file_path(base_path.join("module1.pm"))),
            SAMPLE_MODULE.to_string(),
        )
        .ok();
    index
        .index_file(
            must(Url::from_file_path(base_path.join("module2.pm"))),
            MULTI_PACKAGE_MODULE.to_string(),
        )
        .ok();
    index
        .index_file(
            must(Url::from_file_path(base_path.join("complex.pm"))),
            COMPLEX_MODULE.to_string(),
        )
        .ok();

    c.bench_function("find references cross-file", |b| {
        b.iter(|| {
            // Benchmark: find all references to a symbol
            let refs = index.find_references("process_data");
            black_box(refs);
        });
    });
}

/// Benchmark workspace symbol search (fuzzy matching)
fn bench_workspace_symbol_search(c: &mut Criterion) {
    // Setup: create an indexed workspace
    let temp_dir = must(TempDir::new());
    let base_path = temp_dir.path();

    // Create multiple files with various symbols
    for i in 0..10 {
        let filename = format!("module{}.pm", i);
        let content = match i % 5 {
            0 => SAMPLE_MODULE,
            1 => MULTI_PACKAGE_MODULE,
            2 => SAMPLE_SCRIPT,
            3 => MODERN_PERL,
            _ => COMPLEX_MODULE,
        };
        must(fs::write(base_path.join(&filename), content));
    }

    let index = WorkspaceIndex::new();

    for i in 0..10 {
        let filename = format!("module{}.pm", i);
        let uri = must(Url::from_file_path(base_path.join(&filename)));
        let content = match i % 5 {
            0 => SAMPLE_MODULE,
            1 => MULTI_PACKAGE_MODULE,
            2 => SAMPLE_SCRIPT,
            3 => MODERN_PERL,
            _ => COMPLEX_MODULE,
        };
        index.index_file(uri, content.to_string()).ok();
    }

    c.bench_function("workspace symbol search", |b| {
        b.iter(|| {
            // Benchmark: search for symbols matching a query
            let results1 = index.search_symbols("process");
            let results2 = index.search_symbols("method");
            let results3 = index.search_symbols("sub");

            black_box(results1);
            black_box(results2);
            black_box(results3);
        });
    });
}

/// Benchmark file removal and re-indexing
fn bench_file_removal_and_reindex(c: &mut Criterion) {
    c.bench_function("file removal and re-index", |b| {
        b.iter_batched(
            || {
                // Setup: create and index a workspace
                let temp_dir = must(TempDir::new());
                let base_path = temp_dir.path();

                must(fs::write(base_path.join("module1.pm"), SAMPLE_MODULE));
                must(fs::write(base_path.join("module2.pm"), MULTI_PACKAGE_MODULE));
                must(fs::write(base_path.join("complex.pm"), COMPLEX_MODULE));

                let index = WorkspaceIndex::new();

                let uri1 = must(Url::from_file_path(base_path.join("module1.pm")));
                let uri2 = must(Url::from_file_path(base_path.join("module2.pm")));
                let uri3 = must(Url::from_file_path(base_path.join("complex.pm")));

                index.index_file(uri1.clone(), SAMPLE_MODULE.to_string()).ok();
                index.index_file(uri2.clone(), MULTI_PACKAGE_MODULE.to_string()).ok();
                index.index_file(uri3.clone(), COMPLEX_MODULE.to_string()).ok();

                (temp_dir, index, uri2)
            },
            |(temp_dir, index, uri_to_remove)| {
                // Benchmark: remove a file and re-index it
                index.remove_file_url(&uri_to_remove);
                index.index_file(uri_to_remove, MULTI_PACKAGE_MODULE.to_string()).ok();

                black_box(&index);
                black_box(temp_dir);
            },
            BatchSize::SmallInput,
        );
    });
}

/// Benchmark state transitions in IndexCoordinator
///
/// Validates that state transitions are fast enough for LSP responsiveness (<1ms overhead)
fn bench_state_transitions(c: &mut Criterion) {
    use perl_workspace_index::workspace_index::IndexCoordinator;

    c.bench_function("state transitions", |b| {
        b.iter_batched(
            IndexCoordinator::new,
            |coordinator| {
                // Building → Ready
                coordinator.transition_to_ready(100, 5000);
                black_box(coordinator.state());

                // Ready → Building
                coordinator.transition_to_building(100);
                black_box(coordinator.state());

                // Building progress update
                coordinator.update_building_progress(50);
                black_box(coordinator.state());

                black_box(coordinator);
            },
            BatchSize::SmallInput,
        );
    });
}

/// Benchmark state query operations (hot path)
///
/// Validates that state() calls are fast enough for every LSP request (<100ns)
fn bench_state_query(c: &mut Criterion) {
    use perl_workspace_index::workspace_index::IndexCoordinator;

    let coordinator = IndexCoordinator::new();
    coordinator.transition_to_ready(100, 5000);

    c.bench_function("state query (hot path)", |b| {
        b.iter(|| {
            let state = coordinator.state();
            black_box(state);
        });
    });
}

/// Benchmark parse storm detection and recovery
///
/// Validates that parse storm detection overhead is minimal (<10μs per notify)
fn bench_parse_storm_detection(c: &mut Criterion) {
    use perl_workspace_index::workspace_index::IndexCoordinator;

    c.bench_function("parse storm detection and recovery", |b| {
        b.iter_batched(
            || {
                let coordinator = IndexCoordinator::new();
                coordinator.transition_to_ready(100, 5000);
                coordinator
            },
            |coordinator| {
                // Trigger parse storm (15 changes, threshold is 10)
                for i in 0..15 {
                    coordinator.notify_change(&format!("file{}.pm", i));
                }

                // Check degradation
                black_box(coordinator.state());

                // Recover
                for i in 0..15 {
                    coordinator.notify_parse_complete(&format!("file{}.pm", i));
                }

                // Check recovery
                black_box(coordinator.state());
                black_box(coordinator);
            },
            BatchSize::SmallInput,
        );
    });
}

/// Benchmark early-exit optimization (content hash check)
///
/// Validates that early-exit check is fast enough to be worth it (<1μs)
fn bench_early_exit_optimization(c: &mut Criterion) {
    c.bench_function("early exit content hash check", |b| {
        b.iter_batched(
            || {
                let temp_dir = must(TempDir::new());
                let base_path = temp_dir.path();
                must(fs::write(base_path.join("module1.pm"), SAMPLE_MODULE));

                let index = WorkspaceIndex::new();
                let uri = must(Url::from_file_path(base_path.join("module1.pm")));

                // Initial index
                index.index_file(uri.clone(), SAMPLE_MODULE.to_string()).ok();

                (temp_dir, index, uri)
            },
            |(temp_dir, index, uri)| {
                // Benchmark: re-index with same content (should early-exit)
                index.index_file(uri, SAMPLE_MODULE.to_string()).ok();

                black_box(&index);
                black_box(temp_dir);
            },
            BatchSize::SmallInput,
        );
    });
}

/// Benchmark coordinator with resource limits enforcement
///
/// Validates that limit checking overhead is acceptable (<10μs per check)
fn bench_resource_limit_enforcement(c: &mut Criterion) {
    use perl_workspace_index::workspace_index::{IndexCoordinator, IndexResourceLimits};

    c.bench_function("resource limit enforcement", |b| {
        b.iter_batched(
            || {
                let limits = IndexResourceLimits {
                    max_files: 1000,
                    max_total_symbols: 50000,
                    ..Default::default()
                };
                let coordinator = IndexCoordinator::with_limits(limits);
                coordinator.transition_to_ready(500, 25000);
                coordinator
            },
            |coordinator| {
                // Check limits (this happens after every index operation)
                coordinator.enforce_limits();
                black_box(coordinator.state());
                black_box(coordinator);
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_initial_index_small_workspace,
    bench_initial_index_medium_workspace,
    bench_incremental_update,
    bench_symbol_lookup,
    bench_find_references,
    bench_workspace_symbol_search,
    bench_file_removal_and_reindex,
    bench_state_transitions,
    bench_state_query,
    bench_parse_storm_detection,
    bench_early_exit_optimization,
    bench_resource_limit_enforcement,
);
criterion_main!(benches);
