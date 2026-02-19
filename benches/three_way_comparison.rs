//! Three-way parser comparison benchmark
//!
//! This benchmark compares the performance of three Perl parser implementations:
//! 1. Legacy C parser (tree-sitter-perl C implementation)
//! 2. Pest monolith (pure Rust with PEG grammar)
//! 3. Modern stack (perl-lexer + perl-parser)

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use std::hint::black_box;
use std::time::Duration;

// Test files of varying complexity
const TINY_SCRIPT: &str = "my $x = 42;";

const SMALL_SCRIPT: &str = r#"
my $x = 42;
my @array = (1, 2, 3);
if ($x > 40) {
    print "Hello\n";
}
"#;

const MEDIUM_SCRIPT: &str = r#"
sub fibonacci {
    my $n = shift;
    return $n if $n <= 1;
    
    my ($prev, $curr) = (0, 1);
    for (my $i = 2; $i <= $n; $i++) {
        ($prev, $curr) = ($curr, $prev + $curr);
    }
    return $curr;
}

my @results;
for my $i (1..10) {
    push @results, fibonacci($i);
}
print join(", ", @results), "\n";
"#;

const LARGE_SCRIPT: &str = include_str!("../test_corpus/real_world/medium_module.pl");

struct TestCase {
    name: &'static str,
    code: &'static str,
}

fn get_test_cases() -> Vec<TestCase> {
    vec![
        TestCase { name: "tiny", code: TINY_SCRIPT },
        TestCase { name: "small", code: SMALL_SCRIPT },
        TestCase { name: "medium", code: MEDIUM_SCRIPT },
        // Uncomment when ready for large tests
        // TestCase { name: "large", code: LARGE_SCRIPT },
    ]
}

fn benchmark_legacy_c(c: &mut Criterion) {
    let mut group = c.benchmark_group("legacy_c");
    group.measurement_time(Duration::from_secs(10));
    
    for test in get_test_cases() {
        group.bench_with_input(
            BenchmarkId::from_parameter(test.name),
            &test.code,
            |b, code| {
                b.iter(|| {
                    // Use tree-sitter C parser
                    let mut parser = tree_sitter::Parser::new();
                    parser.set_language(&tree_sitter_perl::language()).unwrap();
                    let _ = parser.parse(black_box(code), None);
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_pest_monolith(c: &mut Criterion) {
    use tree_sitter_perl::PureRustPerlParser;
    
    let mut group = c.benchmark_group("pest_monolith");
    group.measurement_time(Duration::from_secs(10));
    
    for test in get_test_cases() {
        group.bench_with_input(
            BenchmarkId::from_parameter(test.name),
            &test.code,
            |b, code| {
                b.iter(|| {
                    let mut parser = PureRustPerlParser::new();
                    let _ = parser.parse(black_box(code));
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_modern_stack(c: &mut Criterion) {
    use perl_parser::Parser;
    
    let mut group = c.benchmark_group("modern_stack");
    group.measurement_time(Duration::from_secs(10));
    
    for test in get_test_cases() {
        group.bench_with_input(
            BenchmarkId::from_parameter(test.name),
            &test.code,
            |b, code| {
                b.iter(|| {
                    let mut parser = Parser::new(black_box(code));
                    let _ = parser.parse();
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_comparison(c: &mut Criterion) {
    use perl_parser::Parser;
    use tree_sitter_perl::PureRustPerlParser;
    
    let mut group = c.benchmark_group("all_parsers");
    group.measurement_time(Duration::from_secs(5));
    
    let code = MEDIUM_SCRIPT;
    
    group.bench_function("legacy_c", |b| {
        b.iter(|| {
            let mut parser = tree_sitter::Parser::new();
            parser.set_language(&tree_sitter_perl::language()).unwrap();
            let _ = parser.parse(black_box(code), None);
        });
    });
    
    group.bench_function("pest_monolith", |b| {
        b.iter(|| {
            let mut parser = PureRustPerlParser::new();
            let _ = parser.parse(black_box(code));
        });
    });
    
    group.bench_function("modern_stack", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(code));
            let _ = parser.parse();
        });
    });
    
    group.finish();
}

// Isolated component benchmarks
fn benchmark_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("components");
    
    // Benchmark just the lexer
    group.bench_function("perl_lexer_only", |b| {
        use perl_lexer::{PerlLexer, TokenType};
        
        b.iter(|| {
            let mut lexer = PerlLexer::new(black_box(MEDIUM_SCRIPT));
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
    
    // Baseline: cost of iterating a pre-collected token vector (no parsing)
    group.bench_function("token_iteration_baseline", |b| {
        use perl_lexer::{PerlLexer, TokenType};

        // Pre-tokenize once outside the benchmark loop
        let mut lexer = PerlLexer::new(MEDIUM_SCRIPT);
        let mut tokens = Vec::new();
        while let Some(token) = lexer.next_token() {
            if matches!(token.token_type, TokenType::EOF) {
                break;
            }
            tokens.push(token);
        }

        b.iter(|| {
            // Measure only the parse step: iterate pre-collected tokens
            let mut count = 0usize;
            for token in black_box(&tokens) {
                count += token.text.len();
            }
            black_box(count);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_legacy_c,
    benchmark_pest_monolith,
    benchmark_modern_stack,
    benchmark_comparison,
    benchmark_components
);
criterion_main!(benches);