//! Benchmarks for the perl-parser crate
//!
//! This benchmark suite measures the performance of the modern two-crate
//! architecture and enables comparison with other implementations.

use criterion::{Criterion, criterion_group, criterion_main};
use perl_parser::Parser;
use std::hint::black_box;

const SIMPLE_SCRIPT: &str = r#"
my $x = 42;
my $y = "Hello, World!";
my @array = (1, 2, 3, 4, 5);
my %hash = (key => "value", foo => "bar");

if ($x > 40) {
    print "$y\n";
}

sub calculate {
    my ($a, $b) = @_;
    return $a + $b;
}

my $result = calculate(10, 20);
"#;

const COMPLEX_SCRIPT: &str = r#"
package MyModule;
use strict;
use warnings;

sub new {
    my $class = shift;
    my $self = {
        name => shift,
        value => shift || 0,
    };
    bless $self, $class;
    return $self;
}

sub process {
    my $self = shift;
    my @data = @_;
    
    my @results;
    foreach my $item (@data) {
        if ($item =~ /^(\d+)$/) {
            push @results, $1 * $self->{value};
        } elsif ($item =~ /^(\w+)=(\d+)$/) {
            push @results, { $1 => $2 * $self->{value} };
        }
    }
    
    return \@results;
}

sub fibonacci {
    my $n = shift;
    return $n if $n <= 1;
    
    my ($prev, $curr) = (0, 1);
    for (my $i = 2; $i <= $n; $i++) {
        ($prev, $curr) = ($curr, $prev + $curr);
    }
    return $curr;
}

1;
"#;

fn benchmark_simple_parsing(c: &mut Criterion) {
    c.bench_function("parse_simple_script", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(SIMPLE_SCRIPT));
            let _ = parser.parse();
        });
    });
}

fn benchmark_complex_parsing(c: &mut Criterion) {
    c.bench_function("parse_complex_script", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(COMPLEX_SCRIPT));
            let _ = parser.parse();
        });
    });
}

fn benchmark_ast_generation(c: &mut Criterion) {
    c.bench_function("ast_to_sexp", |b| {
        let mut parser = Parser::new(COMPLEX_SCRIPT);
        match parser.parse() {
            Ok(ast) => {
                b.iter(|| {
                    let _ = black_box(ast.to_sexp());
                });
            }
            Err(_) => {
                // Must call iter even when skipping to avoid panic
                eprintln!("Warning: Skipping ast_to_sexp benchmark due to parse error");
                b.iter(|| {
                    // Dummy operation to satisfy criterion
                    black_box(());
                });
            }
        }
    });
}

fn benchmark_isolated_components(c: &mut Criterion) {
    // Benchmark just the lexer phase
    c.bench_function("lexer_only", |b| {
        use perl_lexer::{PerlLexer, TokenType};

        b.iter(|| {
            let mut lexer = PerlLexer::new(black_box(COMPLEX_SCRIPT));
            let mut count = 0;

            while let Some(token) = lexer.next_token() {
                if matches!(token.token_type, TokenType::EOF) {
                    break;
                }
                count += 1;
            }

            black_box(count);
        });
    });

    // Benchmark parser with pre-tokenized input (simulated)
    // This would require exposing more internals, so we skip for now
}

criterion_group!(
    benches,
    benchmark_simple_parsing,
    benchmark_complex_parsing,
    benchmark_ast_generation,
    benchmark_isolated_components
);
criterion_main!(benches);
