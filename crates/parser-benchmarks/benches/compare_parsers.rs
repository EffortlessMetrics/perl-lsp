use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;

// Test inputs of various sizes
const SIMPLE_CODE: &str = "my $x = 42; print $x;";

const MEDIUM_CODE: &str = r#"
use strict;
use warnings;

sub fibonacci {
    my ($n) = @_;
    return $n if $n <= 1;
    return fibonacci($n - 1) + fibonacci($n - 2);
}

my @results;
for my $i (0..10) {
    push @results, fibonacci($i);
}

print join(", ", @results), "\n";
"#;

const COMPLEX_CODE: &str = r#"
package MyClass;
use strict;
use warnings;
use feature 'say';

sub new {
    my ($class, %args) = @_;
    my $self = {
        name => $args{name} // 'unnamed',
        data => $args{data} // [],
    };
    return bless $self, $class;
}

sub process_data {
    my ($self) = @_;
    my @processed;
    
    for my $item (@{$self->{data}}) {
        if ($item =~ /^(\d+)\.(\d+)$/) {
            push @processed, $1 + $2;
        } elsif ($item =~ /^"([^"]+)"$/) {
            push @processed, length($1);
        } else {
            push @processed, $item;
        }
    }
    
    return \@processed;
}

# Test postfix dereference
sub get_values {
    my ($self) = @_;
    return $self->{data}->@*;
}

sub get_hash_slice {
    my ($self, @keys) = @_;
    my $hash = { a => 1, b => 2, c => 3 };
    return $hash->%{@keys};
}

1;
"#;

// Benchmark perl-lexer + perl-parser
fn bench_perl_parser(c: &mut Criterion) {
    use perl_parser::Parser;
    
    let mut group = c.benchmark_group("perl-parser");
    group.measurement_time(Duration::from_secs(10));
    
    group.bench_function("simple", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(SIMPLE_CODE));
            let _ = parser.parse();
        });
    });
    
    group.bench_function("medium", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(MEDIUM_CODE));
            let _ = parser.parse();
        });
    });
    
    group.bench_function("complex", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(COMPLEX_CODE));
            let _ = parser.parse();
        });
    });
    
    group.finish();
}

// Benchmark tree-sitter-perl-rs with pure-rust feature
#[cfg(feature = "tree-sitter-perl-rs")]
fn bench_tree_sitter_perl_rs(c: &mut Criterion) {
    use tree_sitter_perl::PureRustParser;
    
    let mut group = c.benchmark_group("tree-sitter-perl-rs");
    group.measurement_time(Duration::from_secs(10));
    
    group.bench_function("simple", |b| {
        b.iter(|| {
            let parser = PureRustParser::new();
            let _ = parser.parse(black_box(SIMPLE_CODE));
        });
    });
    
    group.bench_function("medium", |b| {
        b.iter(|| {
            let parser = PureRustParser::new();
            let _ = parser.parse(black_box(MEDIUM_CODE));
        });
    });
    
    group.bench_function("complex", |b| {
        b.iter(|| {
            let parser = PureRustParser::new();
            let _ = parser.parse(black_box(COMPLEX_CODE));
        });
    });
    
    group.finish();
}

// Benchmark tree-sitter-perl C implementation
#[cfg(feature = "tree-sitter-perl-c")]
fn bench_tree_sitter_perl_c(c: &mut Criterion) {
    use tree_sitter_perl_c::{language, create_parser};
    
    let mut group = c.benchmark_group("tree-sitter-perl-c");
    group.measurement_time(Duration::from_secs(10));
    
    group.bench_function("simple", |b| {
        b.iter(|| {
            let mut parser = create_parser();
            let _ = parser.parse(black_box(SIMPLE_CODE), None);
        });
    });
    
    group.bench_function("medium", |b| {
        b.iter(|| {
            let mut parser = create_parser();
            let _ = parser.parse(black_box(MEDIUM_CODE), None);
        });
    });
    
    group.bench_function("complex", |b| {
        b.iter(|| {
            let mut parser = create_parser();
            let _ = parser.parse(black_box(COMPLEX_CODE), None);
        });
    });
    
    group.finish();
}

// Comparative benchmark showing all three side by side
fn bench_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser-comparison");
    group.measurement_time(Duration::from_secs(10));
    
    for (name, code) in &[
        ("simple", SIMPLE_CODE),
        ("medium", MEDIUM_CODE),
        ("complex", COMPLEX_CODE),
    ] {
        // perl-parser
        group.bench_with_input(
            BenchmarkId::new("perl-parser", name),
            code,
            |b, code| {
                b.iter(|| {
                    use perl_parser::Parser;
                    let mut parser = Parser::new(black_box(code));
                    let _ = parser.parse();
                });
            }
        );
        
        // tree-sitter-perl-rs
        #[cfg(feature = "tree-sitter-perl-rs")]
        group.bench_with_input(
            BenchmarkId::new("tree-sitter-perl-rs", name),
            code,
            |b, code| {
                b.iter(|| {
                    use tree_sitter_perl::PureRustParser;
                    let parser = PureRustParser::new();
                    let _ = parser.parse(black_box(code));
                });
            }
        );
        
        // tree-sitter-perl C
        #[cfg(feature = "tree-sitter-perl-c")]
        group.bench_with_input(
            BenchmarkId::new("tree-sitter-perl-c", name),
            code,
            |b, code| {
                b.iter(|| {
                    use tree_sitter_perl_c::create_parser;
                    let mut parser = create_parser();
                    let _ = parser.parse(black_box(code), None);
                });
            }
        );
    }
    
    group.finish();
}

// Create different groups based on features
#[cfg(all(feature = "tree-sitter-perl-rs", feature = "tree-sitter-perl-c"))]
criterion_group!(
    benches,
    bench_perl_parser,
    bench_tree_sitter_perl_rs,
    bench_tree_sitter_perl_c,
    bench_comparison
);

#[cfg(all(feature = "tree-sitter-perl-rs", not(feature = "tree-sitter-perl-c")))]
criterion_group!(
    benches,
    bench_perl_parser,
    bench_tree_sitter_perl_rs,
    bench_comparison
);

#[cfg(all(not(feature = "tree-sitter-perl-rs"), feature = "tree-sitter-perl-c"))]
criterion_group!(
    benches,
    bench_perl_parser,
    bench_tree_sitter_perl_c,
    bench_comparison
);

#[cfg(all(not(feature = "tree-sitter-perl-rs"), not(feature = "tree-sitter-perl-c")))]
criterion_group!(
    benches,
    bench_perl_parser,
    bench_comparison
);

criterion_main!(benches);