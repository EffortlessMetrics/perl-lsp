//! Three-way benchmark comparison: Pure Rust vs Legacy C vs Modern Parser
//! 
//! This benchmark compares the performance of:
//! 1. Pure Rust parser (tree-sitter-perl-rs with Pest)
//! 2. Legacy C parser (original tree-sitter-perl)
//! 3. Modern parser (perl-lexer + perl-parser)

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;

// Test cases of varying complexity
const SIMPLE_EXPR: &str = r#"$x = 42;"#;

const MEDIUM_EXPR: &str = r#"
my $result = ($a + $b) * $c;
if ($result > 100) {
    print "Large result: $result\n";
}
"#;

const COMPLEX_EXPR: &str = r#"
package MyModule;
use strict;
use warnings;

sub process_data {
    my ($self, $data) = @_;
    
    my %results;
    for (my $i = 0; $i < @$data; $i++) {
        my $item = $data->[$i];
        if ($item->{type} eq 'number') {
            $results{numbers}++;
        } elsif ($item->{type} eq 'string') {
            $results{strings}++;
        }
    }
    
    return \%results;
}

1;
"#;

const REAL_WORLD: &str = r#"
#!/usr/bin/env perl
use strict;
use warnings;
use feature 'say';

# Real-world style code with various Perl features
my $config = {
    debug => 1,
    verbose => $ENV{VERBOSE} // 0,
    output_dir => './output',
};

sub log_message {
    my ($level, $msg) = @_;
    return unless $config->{debug} || $level eq 'error';
    
    my $timestamp = localtime();
    say "[$timestamp] $level: $msg";
}

sub process_file {
    my ($filename) = @_;
    
    open my $fh, '<', $filename or die "Can't open $filename: $!";
    
    my @lines;
    while (my $line = <$fh>) {
        chomp $line;
        next if $line =~ /^\s*#/;  # Skip comments
        next unless $line =~ /\S/; # Skip empty lines
        
        push @lines, $line;
    }
    
    close $fh;
    
    log_message('info', "Processed " . scalar(@lines) . " lines from $filename");
    
    return \@lines;
}

# Main processing
my @files = glob("*.txt");
for my $file (@files) {
    eval {
        my $lines = process_file($file);
        
        # Process each line
        for my $line (@$lines) {
            if ($line =~ /ERROR:\s*(.+)/) {
                log_message('error', "Found error: $1");
            }
        }
    };
    
    if ($@) {
        log_message('error', "Failed to process $file: $@");
    }
}

say "Processing complete.";
"#;

// Benchmark the Pure Rust parser
fn bench_pure_rust(c: &mut Criterion) {
    use tree_sitter_perl_rs::{parse_rust, ParserConfig};
    
    let mut group = c.benchmark_group("pure_rust_parser");
    group.measurement_time(Duration::from_secs(10));
    
    for (name, input) in &[
        ("simple", SIMPLE_EXPR),
        ("medium", MEDIUM_EXPR),
        ("complex", COMPLEX_EXPR),
        ("real_world", REAL_WORLD),
    ] {
        group.bench_with_input(BenchmarkId::from_parameter(name), input, |b, i| {
            b.iter(|| {
                let config = ParserConfig::default();
                let result = parse_rust(black_box(i), &config);
                assert!(result.is_ok());
            });
        });
    }
    
    group.finish();
}

// Benchmark the Legacy C parser
fn bench_legacy_c(c: &mut Criterion) {
    use tree_sitter::Parser;
    
    let mut group = c.benchmark_group("legacy_c_parser");
    group.measurement_time(Duration::from_secs(10));
    
    for (name, input) in &[
        ("simple", SIMPLE_EXPR),
        ("medium", MEDIUM_EXPR),
        ("complex", COMPLEX_EXPR),
        ("real_world", REAL_WORLD),
    ] {
        group.bench_with_input(BenchmarkId::from_parameter(name), input, |b, i| {
            b.iter(|| {
                let mut parser = Parser::new();
                parser.set_language(tree_sitter_perl::language()).unwrap();
                let tree = parser.parse(black_box(i), None);
                assert!(tree.is_some());
            });
        });
    }
    
    group.finish();
}

// Benchmark the Modern parser (perl-lexer + perl-parser)
fn bench_modern_parser(c: &mut Criterion) {
    use perl_parser::Parser;
    
    let mut group = c.benchmark_group("modern_parser");
    group.measurement_time(Duration::from_secs(10));
    
    for (name, input) in &[
        ("simple", SIMPLE_EXPR),
        ("medium", MEDIUM_EXPR),
        ("complex", COMPLEX_EXPR),
        ("real_world", REAL_WORLD),
    ] {
        group.bench_with_input(BenchmarkId::from_parameter(name), input, |b, i| {
            b.iter(|| {
                let mut parser = Parser::new(black_box(i));
                let result = parser.parse();
                assert!(result.is_ok());
            });
        });
    }
    
    group.finish();
}

// Combined comparison benchmark
fn bench_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_comparison");
    group.measurement_time(Duration::from_secs(10));
    
    // Test with the complex expression
    let input = COMPLEX_EXPR;
    
    group.bench_function("pure_rust", |b| {
        use tree_sitter_perl_rs::{parse_rust, ParserConfig};
        b.iter(|| {
            let config = ParserConfig::default();
            let result = parse_rust(black_box(input), &config);
            assert!(result.is_ok());
        });
    });
    
    group.bench_function("legacy_c", |b| {
        use tree_sitter::Parser;
        b.iter(|| {
            let mut parser = Parser::new();
            parser.set_language(tree_sitter_perl::language()).unwrap();
            let tree = parser.parse(black_box(input), None);
            assert!(tree.is_some());
        });
    });
    
    group.bench_function("modern", |b| {
        use perl_parser::Parser;
        b.iter(|| {
            let mut parser = Parser::new(black_box(input));
            let result = parser.parse();
            assert!(result.is_ok());
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_pure_rust,
    bench_legacy_c,
    bench_modern_parser,
    bench_comparison
);
criterion_main!(benches);