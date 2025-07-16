//! Parser performance benchmarks
//!
//! This module contains benchmarks to measure the overall parsing
//! performance of the tree-sitter Perl parser.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use tree_sitter::Parser;
use tree_sitter_perl::{language, parse};

fn bench_parser_creation(c: &mut Criterion) {
    c.bench_function("parser_creation", |b| {
        b.iter(|| {
            let mut parser = Parser::new();
            parser.set_language(&language()).unwrap();
            black_box(parser);
        });
    });
}

fn bench_simple_parsing(c: &mut Criterion) {
    let test_cases = vec![
        "my $var = 42;",
        "print 'Hello, World!';",
        "sub foo { return 1; }",
        "if ($x) { $y = 1; }",
        "for my $i (1..10) { print $i; }",
    ];

    c.bench_function("simple_parsing", |b| {
        b.iter(|| {
            for code in &test_cases {
                black_box(parse(code).unwrap());
            }
        });
    });
}

fn bench_complex_parsing(c: &mut Criterion) {
    let complex_cases = vec![
        r#"
package MyClass;
use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    bless \%args, $class;
}

sub method {
    my ($self, @params) = @_;
    return $self->{value} + @params;
}
"#,
        r#"
use strict;
use warnings;
use Exporter 'import';

our @EXPORT_OK = qw(foo bar baz);

sub foo {
    my ($param) = @_;
    return defined($param) ? $param : undef;
}

sub bar {
    my (@params) = @_;
    return map { $_ * 2 } grep { $_ > 0 } @params;
}

sub baz {
    my ($hash_ref) = @_;
    return {
        map { $_ => $hash_ref->{$_} }
        grep { defined $hash_ref->{$_} }
        keys %$hash_ref
    };
}
"#,
    ];

    c.bench_function("complex_parsing", |b| {
        b.iter(|| {
            for code in &complex_cases {
                black_box(parse(code).unwrap());
            }
        });
    });
}

fn bench_incremental_parsing(c: &mut Criterion) {
    let mut parser = Parser::new();
    parser.set_language(&language()).unwrap();

    let initial_code = "my $var = 42;";
    let tree = parser.parse(initial_code, None).unwrap();

    let modified_code = "my $var = 42; print 'Hello';";

    c.bench_function("incremental_parsing", |b| {
        b.iter(|| {
            black_box(parser.parse(modified_code, Some(&tree)).unwrap());
        });
    });
}

fn bench_error_recovery(c: &mut Criterion) {
    let error_cases = vec![
        "my $str = \"Unterminated string;",
        "if ($condition { $action = 1; }",
        "my $var = 1 +;",
        "sub foo { return 1; # Unterminated comment",
    ];

    c.bench_function("error_recovery", |b| {
        b.iter(|| {
            for code in &error_cases {
                black_box(parse(code).unwrap());
            }
        });
    });
}

fn bench_unicode_parsing(c: &mut Criterion) {
    let unicode_cases = vec![
        r#"
my $変数 = "値";
my $über = "cool";
my $naïve = "simple";
sub 関数 { return "関数です"; }
"#,
        r#"
use utf8;
my $message = "Hello 世界! 🌍";
my $emoji = "🚀 rocket";
my $mixed = "ASCII + 日本語 + emoji 🎉";
"#,
    ];

    c.bench_function("unicode_parsing", |b| {
        b.iter(|| {
            for code in &unicode_cases {
                black_box(parse(code).unwrap());
            }
        });
    });
}

fn bench_large_file_parsing(c: &mut Criterion) {
    let large_code = generate_large_perl_file(5000);

    c.bench_function("large_file_parsing", |b| {
        b.iter(|| {
            black_box(parse(&large_code).unwrap());
        });
    });
}

fn bench_memory_usage(c: &mut Criterion) {
    let test_cases = (0..100)
        .map(|i| {
            format!(
                "my $var{} = {}; print \"Variable {} = $var{}\";",
                i, i, i, i
            )
        })
        .collect::<Vec<_>>();

    c.bench_function("memory_usage", |b| {
        b.iter(|| {
            for code in &test_cases {
                black_box(parse(code).unwrap());
            }
        });
    });
}

fn generate_large_perl_file(size: usize) -> String {
    let mut code = String::new();

    // Add package declaration
    code.push_str("package LargeFile;\n");
    code.push_str("use strict;\n");
    code.push_str("use warnings;\n\n");

    // Add variables
    for i in 0..size {
        code.push_str(&format!("my $var{} = {};\n", i, i));
    }

    code.push('\n');

    // Add functions
    for i in 0..(size / 10) {
        code.push_str(&format!("sub func{} {{\n", i));
        code.push_str(
            "    my ($param) = @_;
",
        );
        code.push_str(&format!("    return $param + {};\n", i));
        code.push_str("}\n\n");
    }

    // Add main logic
    code.push_str("sub main {\n");
    for i in 0..(size / 20) {
        code.push_str(&format!("    print \"Processing variable {}\";\n", i));
        code.push_str(&format!("    my $result = func{}($var{});\n", i, i));
        code.push_str("    print \"Result: $result\";\n");
    }
    code.push_str("}\n\n");

    code.push_str("main();\n");

    code
}

criterion_group!(
    benches,
    bench_parser_creation,
    bench_simple_parsing,
    bench_complex_parsing,
    bench_incremental_parsing,
    bench_error_recovery,
    bench_unicode_parsing,
    bench_large_file_parsing,
    bench_memory_usage,
);
criterion_main!(benches);
