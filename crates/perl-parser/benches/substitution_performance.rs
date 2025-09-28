//! Performance benchmark for substitution operator parsing in PR #158
//! Tests for performance regression in substitution operator implementation
use criterion::{criterion_group, criterion_main, Criterion};
use perl_parser::Parser;
use std::hint::black_box as bb;

// Test cases for performance analysis
const NON_SUBSTITUTION_CODE: &str = r#"
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
for my $i (1..10) {
    print "Number: $i\n";
    my $temp = $i * 2;
    push @array, $temp;
}
"#;

const BASIC_SUBSTITUTION_CODE: &str = r#"
s/foo/bar/;
s/pattern/replacement/g;
s/old/new/i;
s/test/result/gi;
"#;

const COMPLEX_SUBSTITUTION_CODE: &str = r#"
s/foo/bar/g;
s/pattern/replacement/gix;
s/old/new/msxi;
s/test/result/ee;
s/alpha/beta/r;
s{from}{to}g;
s|old|new|i;
s'pattern'replacement'g;
s<find><replace>gi;
s!search!replace!x;
"#;

const MIXED_CODE_WITH_SUBSTITUTION: &str = r#"
my $text = "Hello World";
$text =~ s/Hello/Hi/g;
my @lines = split /\n/, $input;

foreach my $line (@lines) {
    $line =~ s/^\s+//;  # Remove leading whitespace
    $line =~ s/\s+$//;  # Remove trailing whitespace
    $line =~ s/foo/bar/gi;

    if ($line =~ /pattern/) {
        $line =~ s/pattern/replacement/e;
    }
}

sub process_text {
    my $text = shift;
    $text =~ s/old/new/g;
    $text =~ s{pattern}{replacement}g;
    return $text;
}
"#;

const MIXED_CODE_WITHOUT_SUBSTITUTION: &str = r#"
my $text = "Hello World";
my @lines = split /\n/, $input;

foreach my $line (@lines) {
    chomp $line;
    $line = trim($line);

    if ($line =~ /pattern/) {
        $line = replace_pattern($line);
    }
}

sub process_text {
    my $text = shift;
    $text = transform($text);
    return $text;
}

sub trim {
    my $str = shift;
    $str =~ /^\s*(.*?)\s*$/;
    return $1;
}
"#;

// Benchmark functions
fn benchmark_non_substitution_parsing(c: &mut Criterion) {
    c.bench_function("parse_non_substitution_code", |b| {
        b.iter(|| {
            let mut parser = Parser::new(bb(NON_SUBSTITUTION_CODE));
            let _ = parser.parse();
        });
    });
}

fn benchmark_basic_substitution_parsing(c: &mut Criterion) {
    c.bench_function("parse_basic_substitution", |b| {
        b.iter(|| {
            let mut parser = Parser::new(bb(BASIC_SUBSTITUTION_CODE));
            let _ = parser.parse();
        });
    });
}

fn benchmark_complex_substitution_parsing(c: &mut Criterion) {
    c.bench_function("parse_complex_substitution", |b| {
        b.iter(|| {
            let mut parser = Parser::new(bb(COMPLEX_SUBSTITUTION_CODE));
            let _ = parser.parse();
        });
    });
}

fn benchmark_mixed_with_substitution(c: &mut Criterion) {
    c.bench_function("parse_mixed_with_substitution", |b| {
        b.iter(|| {
            let mut parser = Parser::new(bb(MIXED_CODE_WITH_SUBSTITUTION));
            let _ = parser.parse();
        });
    });
}

fn benchmark_mixed_without_substitution(c: &mut Criterion) {
    c.bench_function("parse_mixed_without_substitution", |b| {
        b.iter(|| {
            let mut parser = Parser::new(bb(MIXED_CODE_WITHOUT_SUBSTITUTION));
            let _ = parser.parse();
        });
    });
}

fn benchmark_ast_creation_substitution(c: &mut Criterion) {
    c.bench_function("ast_creation_with_substitution", |b| {
        let mut parser = Parser::new(COMPLEX_SUBSTITUTION_CODE);
        match parser.parse() {
            Ok(ast) => {
                b.iter(|| {
                    let _ = bb(ast.to_sexp());
                });
            }
            Err(_) => {
                // Fallback benchmark if parsing fails
                b.iter(|| {
                    let mut fallback_parser = Parser::new("s/foo/bar/g;");
                    if let Ok(fallback_ast) = fallback_parser.parse() {
                        let _ = bb(fallback_ast.to_sexp());
                    }
                });
            }
        }
    });
}

fn benchmark_delimiter_variants(c: &mut Criterion) {
    let delimiter_tests = vec![
        ("s/foo/bar/g", "slash_delimiters"),
        ("s{foo}{bar}g", "brace_delimiters"),
        ("s|foo|bar|g", "pipe_delimiters"),
        ("s'foo'bar'g", "quote_delimiters"),
        ("s<foo><bar>g", "angle_delimiters"),
        ("s!foo!bar!g", "exclamation_delimiters"),
    ];

    for (code, name) in delimiter_tests {
        c.bench_function(&format!("parse_{}", name), |b| {
            b.iter(|| {
                let mut parser = Parser::new(bb(code));
                let _ = parser.parse();
            });
        });
    }
}

fn benchmark_modifier_variants(c: &mut Criterion) {
    let modifier_tests = vec![
        ("s/foo/bar/", "no_modifiers"),
        ("s/foo/bar/g", "global_modifier"),
        ("s/foo/bar/gi", "global_case_insensitive"),
        ("s/foo/bar/gix", "extended_modifiers"),
        ("s/foo/bar/msxi", "all_modifiers"),
        ("s/foo/bar/ee", "double_eval"),
        ("s/foo/bar/r", "return_modifier"),
    ];

    for (code, name) in modifier_tests {
        c.bench_function(&format!("parse_{}", name), |b| {
            b.iter(|| {
                let mut parser = Parser::new(bb(code));
                let _ = parser.parse();
            });
        });
    }
}

criterion_group!(
    benches,
    benchmark_non_substitution_parsing,
    benchmark_basic_substitution_parsing,
    benchmark_complex_substitution_parsing,
    benchmark_mixed_with_substitution,
    benchmark_mixed_without_substitution,
    benchmark_ast_creation_substitution,
    benchmark_delimiter_variants,
    benchmark_modifier_variants
);
criterion_main!(benches);
