//! Benchmarks for edge case detection and handling performance

use criterion::{criterion_group, criterion_main, Criterion};

#[cfg(feature = "pure-rust")]
use criterion::BenchmarkId;

#[cfg(feature = "pure-rust")]
use std::hint::black_box;

#[cfg(feature = "pure-rust")]
use tree_sitter_perl::{
    dynamic_delimiter_recovery::RecoveryMode,
    edge_case_handler::{EdgeCaseConfig, EdgeCaseHandler},
    tree_sitter_adapter::TreeSitterAdapter,
    understanding_parser::UnderstandingParser,
};

/// Benchmark different types of Perl code
#[cfg(feature = "pure-rust")]
fn bench_edge_case_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("edge_case_detection");

    // Test cases with increasing complexity
    let test_cases = vec![
        (
            "clean",
            r#"
my $var = 42;
print "Hello, world!\n";
my $text = <<'EOF';
Standard heredoc
with multiple lines
EOF
"#,
        ),
        (
            "dynamic_delimiter",
            r#"
my $delimiter = "EOF";
my $text = <<$delimiter;
Dynamic delimiter content
EOF

my $complex = get_delimiter();
my $data = <<$complex;
Unknown delimiter
UNKNOWN
"#,
        ),
        (
            "phase_dependent",
            r#"
BEGIN {
    our $CONFIG = <<'END';
    compile-time config
END
    $ENV{CONFIG} = $CONFIG;
}

CHECK {
    my $validation = <<'VAL';
    validation rules
VAL
}

END {
    print <<'CLEANUP';
    cleanup code
CLEANUP
}
"#,
        ),
        (
            "multiple_edge_cases",
            r#"
use encoding 'latin1';

BEGIN {
    my $d = shift || "EOF";
    $::config = <<$d;
    Complex case
EOF
}

format REPORT =
<<'HDR'
Header
HDR
.

tie *FH, 'Custom';
print FH <<'END';
Tied output
END

use Filter::Simple;
"#,
        ),
    ];

    // Benchmark each test case
    for (name, code) in &test_cases {
        group.bench_with_input(BenchmarkId::new("analyze", name), code, |b, code| {
            b.iter(|| {
                let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
                black_box(handler.analyze(code))
            });
        });
    }

    group.finish();
}

/// Benchmark different recovery modes
#[cfg(feature = "pure-rust")]
fn bench_recovery_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("recovery_modes");

    let code = r#"
my $delimiter = "EOF";
my $text = <<$delimiter;
Test content for benchmarking
with multiple lines
EOF

my $another = $ENV{DELIM};
my $content = <<$another;
Another dynamic delimiter
UNKNOWN
"#;

    let modes = vec![
        ("conservative", RecoveryMode::Conservative),
        ("best_guess", RecoveryMode::BestGuess),
        ("interactive", RecoveryMode::Interactive),
        ("sandbox", RecoveryMode::Sandbox),
    ];

    for (name, mode) in modes {
        group.bench_with_input(BenchmarkId::new("mode", name), &mode, |b, mode: &RecoveryMode| {
            b.iter(|| {
                let config = EdgeCaseConfig { recovery_mode: mode.clone(), ..Default::default() };
                let mut handler = EdgeCaseHandler::new(config);
                black_box(handler.analyze(code))
            });
        });
    }

    group.finish();
}

/// Benchmark tree-sitter conversion overhead
#[cfg(feature = "pure-rust")]
fn bench_tree_sitter_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("tree_sitter_conversion");

    let test_cases = vec![
        ("small", generate_perl_code(10)),
        ("medium", generate_perl_code(100)),
        ("large", generate_perl_code(1000)),
    ];

    for (size, code) in &test_cases {
        group.bench_with_input(BenchmarkId::new("convert", size), code, |b, code| {
            // Pre-analyze to separate analysis from conversion time
            let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
            let analysis = handler.analyze(code);

            b.iter(|| {
                black_box(TreeSitterAdapter::convert_to_tree_sitter(
                    analysis.ast.clone(),
                    analysis.diagnostics.clone(),
                    code,
                ))
            });
        });
    }

    group.finish();
}

/// Benchmark understanding parser with edge cases
#[cfg(feature = "pure-rust")]
fn bench_understanding_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("understanding_parser");

    let test_cases = vec![
        (
            "clean_code",
            r#"
my $x = 42;
sub foo { return $x * 2; }
print foo();
"#,
        ),
        (
            "with_heredocs",
            r#"
my $config = <<'EOF';
[database]
host = localhost
port = 5432
EOF
"#,
        ),
        (
            "with_edge_cases",
            r#"
BEGIN { $x = <<'E'; data E }
format F = <<'E' text E .
my $d = shift; my $t = <<$d; content EOF
"#,
        ),
    ];

    for (name, code) in &test_cases {
        group.bench_with_input(BenchmarkId::new("parse", name), code, |b, code| {
            b.iter(|| {
                let mut parser = UnderstandingParser::new();
                black_box(parser.parse_with_understanding(code))
            });
        });
    }

    group.finish();
}

/// Generate test Perl code with heredocs
#[cfg(feature = "pure-rust")]
fn generate_perl_code(statements: usize) -> String {
    let mut code = String::new();

    for i in 0..statements {
        match i % 5 {
            0 => code.push_str(&format!("my $var{} = {};\n", i, i * 42)),
            1 => code.push_str(&format!("print \"Line {}\\n\";\n", i)),
            2 => code.push_str(&format!(
                "my $text{} = <<'EOF';\nHeredoc content {}\nwith multiple lines\nEOF\n",
                i, i
            )),
            3 => code.push_str(&format!("sub func{} {{ return {} * 2; }}\n", i, i)),
            4 => {
                // Add some edge cases
                if i % 20 == 4 {
                    code.push_str(&format!(
                        "my $d{} = \"END\";\nmy $t{} = <<$d{};\nDynamic {}\nEND\n",
                        i, i, i, i
                    ));
                } else {
                    code.push_str(&format!("# Comment {}\n", i));
                }
            }
            _ => unreachable!(),
        }
    }

    code
}

/// Benchmark memory usage patterns
#[cfg(feature = "pure-rust")]
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    // Test with deeply nested structures
    let nested_code = generate_nested_code(10);
    let deep_nested = generate_nested_code(100);

    group.bench_function("shallow_nesting", |b| {
        b.iter(|| {
            let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
            let analysis = handler.analyze(&nested_code);
            black_box(analysis);
        });
    });

    group.bench_function("deep_nesting", |b| {
        b.iter(|| {
            let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
            let analysis = handler.analyze(&deep_nested);
            black_box(analysis);
        });
    });

    group.finish();
}

#[cfg(feature = "pure-rust")]
fn generate_nested_code(depth: usize) -> String {
    let mut code = String::new();

    // Generate nested blocks with heredocs
    for i in 0..depth {
        code.push_str(&"  ".repeat(i));
        code.push_str("if ($condition) {\n");

        if i % 3 == 0 {
            code.push_str(&"  ".repeat(i + 1));
            code.push_str(&format!("my $h{} = <<'END';\nNested heredoc at level {}\nEND\n", i, i));
        }
    }

    // Close all blocks
    for i in (0..depth).rev() {
        code.push_str(&"  ".repeat(i));
        code.push_str("}\n");
    }

    code
}

#[cfg(feature = "pure-rust")]
criterion_group!(
    benches,
    bench_edge_case_detection,
    bench_recovery_modes,
    bench_tree_sitter_conversion,
    bench_understanding_parser,
    bench_memory_usage
);

#[cfg(not(feature = "pure-rust"))]
fn dummy_benchmark(_c: &mut Criterion) {
    // No benchmarks when pure-rust feature is disabled
}

#[cfg(not(feature = "pure-rust"))]
criterion_group!(benches, dummy_benchmark);

criterion_main!(benches);
