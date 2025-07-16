//! Scanner performance benchmarks
//!
//! This module contains benchmarks to compare the performance of
//! the Rust-native scanner against the C scanner implementation.
//!
//! Usage:
//!   cargo bench --bench scanner_benchmarks --features rust-scanner
//!   cargo bench --bench scanner_benchmarks --features c-scanner

use criterion::{Criterion, black_box, criterion_group, criterion_main};

// Import the appropriate scanner based on feature flags
#[cfg(feature = "rust-scanner")]
use tree_sitter_perl::scanner::{PerlScanner, RustScanner, ScannerConfig};

#[cfg(feature = "c-scanner")]
use tree_sitter_perl::scanner::{PerlScanner, CScanner, ScannerConfig};

// Common test cases for both scanners
const TEST_CASES: &[&str] = &[
    // Basic Perl constructs
    "my $var = 42;",
    "print 'Hello, World!';",
    "sub foo { return 1; }",
    
    // Variables and assignments
    "my ($a, $b, $c) = (1, 2, 3);",
    "$hash{key} = 'value';",
    "@array = qw(one two three);",
    
    // Control structures
    "if ($condition) { do_something(); }",
    "for my $item (@list) { process($item); }",
    "while (<>) { chomp; print; }",
    
    // String operations
    "my $str = \"Hello $name\";",
    "my $regex = qr/\\d+/;",
    "my $heredoc = <<'EOF'; content EOF",
    
    // Complex expressions
    "my $result = $a + $b * $c / $d;",
    "my $bool = defined($var) && length($var) > 0;",
    "my @filtered = grep { $_ > 10 } @numbers;",
];

// Large test case for stress testing
const LARGE_TEST_CASE: &str = r#"
package MyApp::Controller;

use strict;
use warnings;
use Exporter 'import';

our @EXPORT_OK = qw(process_data validate_input);

sub new {
    my ($class, %params) = @_;
    my $self = {
        config => $params{config} || {},
        cache  => {},
        debug  => $params{debug} || 0,
    };
    return bless $self, $class;
}

sub process_data {
    my ($self, $data) = @_;
    
    return unless defined $data && ref($data) eq 'ARRAY';
    
    my @results;
    for my $item (@$data) {
        next unless defined $item;
        
        if (ref($item) eq 'HASH') {
            push @results, $self->_process_hash($item);
        } elsif (ref($item) eq 'ARRAY') {
            push @results, map { $self->_process_scalar($_) } @$item;
        } else {
            push @results, $self->_process_scalar($item);
        }
    }
    
    return \@results;
}

sub _process_hash {
    my ($self, $hash) = @_;
    my %processed;
    
    while (my ($key, $value) = each %$hash) {
        $processed{$key} = $self->_process_scalar($value);
    }
    
    return \%processed;
}

sub _process_scalar {
    my ($self, $value) = @_;
    
    return $value unless defined $value;
    
    if ($value =~ /^\d+$/) {
        return $value * 2;
    } elsif ($value =~ /^[A-Za-z_]\w*$/) {
        return uc($value);
    } else {
        return length($value);
    }
}

sub validate_input {
    my ($self, $input) = @_;
    
    return 0 unless defined $input;
    
    if (ref($input) eq 'HASH') {
        return $self->_validate_hash($input);
    } elsif (ref($input) eq 'ARRAY') {
        return $self->_validate_array($input);
    } else {
        return $self->_validate_scalar($input);
    }
}

sub _validate_hash {
    my ($self, $hash) = @_;
    
    for my $key (keys %$hash) {
        return 0 unless $self->_validate_scalar($key);
        return 0 unless $self->_validate_scalar($hash->{$key});
    }
    
    return 1;
}

sub _validate_array {
    my ($self, $array) = @_;
    
    for my $item (@$array) {
        return 0 unless $self->_validate_scalar($item);
    }
    
    return 1;
}

sub _validate_scalar {
    my ($self, $value) = @_;
    
    return 0 unless defined $value;
    return 0 if ref($value);
    
    return 1;
}

1;
"#;

fn bench_scanner_basic(c: &mut Criterion) {
    let mut group = c.benchmark_group("scanner_basic");
    
    // Test each individual case
    for (i, test_case) in TEST_CASES.iter().enumerate() {
        group.bench_function(&format!("case_{}", i), |b| {
            b.iter(|| {
                let mut scanner = create_scanner();
                let input = black_box(test_case.as_bytes());
                let mut tokens = Vec::new();
                
                scanner.scan(input, &mut tokens).unwrap();
                black_box(tokens)
            });
        });
    }
    
    group.finish();
}

fn bench_scanner_large_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("scanner_large_file");
    
    group.bench_function("large_perl_file", |b| {
        b.iter(|| {
            let mut scanner = create_scanner();
            let input = black_box(LARGE_TEST_CASE.as_bytes());
            let mut tokens = Vec::new();
            
            scanner.scan(input, &mut tokens).unwrap();
            black_box(tokens)
        });
    });
    
    group.finish();
}

fn bench_scanner_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("scanner_throughput");
    
    // Test with different input sizes
    let sizes = [100, 1000, 10000];
    
    for size in sizes {
        let test_input = generate_test_input(size);
        
        group.bench_function(&format!("{}_bytes", size), |b| {
            b.iter(|| {
                let mut scanner = create_scanner();
                let input = black_box(test_input.as_bytes());
                let mut tokens = Vec::new();
                
                scanner.scan(input, &mut tokens).unwrap();
                black_box(tokens)
            });
        });
    }
    
    group.finish();
}

fn bench_scanner_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("scanner_memory");
    
    group.bench_function("token_generation", |b| {
        b.iter(|| {
            let mut scanner = create_scanner();
            let input = black_box(LARGE_TEST_CASE.as_bytes());
            let mut tokens = Vec::with_capacity(1000); // Pre-allocate
            
            scanner.scan(input, &mut tokens).unwrap();
            black_box(tokens.len())
        });
    });
    
    group.finish();
}

// Helper function to create the appropriate scanner
fn create_scanner() -> Box<dyn PerlScanner> {
    let config = ScannerConfig::default();
    
    #[cfg(feature = "rust-scanner")]
    {
        Box::new(RustScanner::new(config))
    }
    
    #[cfg(feature = "c-scanner")]
    {
        Box::new(CScanner::new(config))
    }
    
    #[cfg(not(any(feature = "rust-scanner", feature = "c-scanner")))]
    {
        compile_error!("Must specify either rust-scanner or c-scanner feature");
    }
}

// Helper function to generate test input of specified size
fn generate_test_input(size: usize) -> String {
    let mut input = String::with_capacity(size);
    
    // Generate a mix of Perl constructs
    let constructs = [
        "my $var = 42;",
        "print 'test';",
        "sub func { return 1; }",
        "if ($cond) { do_something(); }",
        "$hash{key} = 'value';",
        "@array = (1, 2, 3);",
    ];
    
    let mut current_size = 0;
    let mut construct_index = 0;
    
    while current_size < size {
        let construct = constructs[construct_index % constructs.len()];
        input.push_str(construct);
        input.push('\n');
        
        current_size += construct.len() + 1;
        construct_index += 1;
    }
    
    input.truncate(size);
    input
}

// Configure criterion groups
criterion_group!(
    benches,
    bench_scanner_basic,
    bench_scanner_large_file,
    bench_scanner_throughput,
    bench_scanner_memory_usage,
);

criterion_main!(benches);
