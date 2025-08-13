use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::time::Duration;

// Complex real-world Perl patterns
const PATTERNS: &[(&str, &str)] = &[
    (
        "heredoc_simple",
        r#"
my $text = <<'EOF';
This is a heredoc
with multiple lines
EOF
"#,
    ),
    (
        "heredoc_interpolated",
        r#"
my $name = "World";
my $greeting = <<"END";
Hello, $name!
Today is ${day}
END
"#,
    ),
    (
        "regex_substitution",
        r#"
$text =~ s/foo/bar/g;
$text =~ s{old}{new}gi;
$text =~ s|pattern|replacement|x;
"#,
    ),
    (
        "complex_dereference",
        r#"
my $value = $hash->{key}->[0]->{nested};
my @items = $arrayref->@*;
my %data = $hashref->%{qw(foo bar)};
"#,
    ),
    (
        "method_chains",
        r#"
$object->method1()
       ->method2($arg)
       ->method3({ key => 'value' })
       ->result();
"#,
    ),
    (
        "try_catch",
        r#"
try {
    dangerous_operation();
} catch ($e) {
    warn "Error: $e";
} finally {
    cleanup();
}
"#,
    ),
    (
        "package_class",
        r#"
package MyApp::Controller {
    use Moo;
    has 'config' => (is => 'ro');
    
    sub process {
        my ($self, $request) = @_;
        return $self->config->{handler}->($request);
    }
}
"#,
    ),
    (
        "unicode_identifiers",
        r#"
my $café = "coffee";
my $π = 3.14159;
sub Σ { sum(@_) }
my $日本語 = "Japanese";
"#,
    ),
    (
        "format_declaration",
        r#"
format STDOUT =
@<<<<<<<   @|||||||   @>>>>>>>
$name,     $score,    $grade
.
"#,
    ),
    (
        "mixed_quotes",
        r#"
my $a = 'single';
my $b = "double with $interpolation";
my $c = q{single with braces};
my $d = qq{double with $var and {nested}};
my $e = qw(word list items);
"#,
    ),
    (
        "control_structures",
        r#"
for my $item (@list) {
    next if $item =~ /skip/;
    last if $item eq 'stop';
    redo if rand() < 0.1;
}

given ($value) {
    when (/^\d+$/) { say "number" }
    when ('foo')   { say "foo" }
    default        { say "other" }
}
"#,
    ),
    (
        "subroutine_signatures",
        r#"
sub add ($x, $y) { $x + $y }
sub optional ($x, $y = 10) { $x + $y }
sub slurpy ($first, @rest) { $first, @rest }
sub typed (Int $x, Str $y) { "$y: $x" }
"#,
    ),
];

fn bench_parsers(c: &mut Criterion) {
    let mut group = c.benchmark_group("comprehensive-patterns");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    for (name, code) in PATTERNS {
        // Benchmark perl-parser
        group.bench_with_input(BenchmarkId::new("perl-parser", name), code, |b, code| {
            b.iter(|| {
                use perl_parser::Parser;
                let mut parser = Parser::new(black_box(code));
                let _ = parser.parse();
            });
        });

        // Benchmark tree-sitter-perl-c
        group.bench_with_input(BenchmarkId::new("tree-sitter-c", name), code, |b, code| {
            b.iter(|| {
                use tree_sitter_perl_c::create_parser;
                let mut parser = create_parser();
                let _ = parser.parse(black_box(code), None);
            });
        });
    }

    group.finish();
}

// Additional benchmark for file size scaling
fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("size-scaling");
    group.measurement_time(Duration::from_secs(10));

    // Generate code of different sizes
    let sizes = vec![10, 50, 100, 500, 1000];

    for size in sizes {
        let code = generate_code(size);
        let code_size = code.len();

        group.bench_with_input(
            BenchmarkId::new("perl-parser", format!("{}-lines", size)),
            &code,
            |b, code| {
                b.iter(|| {
                    use perl_parser::Parser;
                    let mut parser = Parser::new(black_box(code));
                    let _ = parser.parse();
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("tree-sitter-c", format!("{}-lines", size)),
            &code,
            |b, code| {
                b.iter(|| {
                    use tree_sitter_perl_c::create_parser;
                    let mut parser = create_parser();
                    let _ = parser.parse(black_box(code), None);
                });
            },
        );

        println!("Code size for {}-lines: {} bytes", size, code_size);
    }

    group.finish();
}

fn generate_code(lines: usize) -> String {
    let mut code = String::new();

    for i in 0..lines {
        match i % 10 {
            0 => code.push_str(&format!("my $var{} = {};\n", i, i)),
            1 => code.push_str(&format!("push @array{}, $var{};\n", i / 10, i)),
            2 => code.push_str(&format!("$hash{{{}}}->{{'key'}} = $var{};\n", i / 10, i)),
            3 => code.push_str(&format!(
                "if ($var{} > {}) {{ $result += $var{}; }}\n",
                i,
                i / 2,
                i
            )),
            4 => code.push_str(&format!("$string{} = \"Value is $var{}\";\n", i, i)),
            5 => code.push_str(&format!("$object{}->method{}($var{});\n", i / 10, i % 5, i)),
            6 => code.push_str(&format!(
                "for my $x (1..{}) {{ $sum{} += $x; }}\n",
                i % 20,
                i
            )),
            7 => code.push_str(&format!(
                "sub func{} {{ my ($x) = @_; return $x * {}; }}\n",
                i, i
            )),
            8 => code.push_str(&format!(
                "$result{} = $var{} =~ /pattern{}/ ? 1 : 0;\n",
                i,
                i,
                i % 10
            )),
            9 => code.push_str(&format!("# Comment line {} with some explanation\n", i)),
            _ => unreachable!(),
        }
    }

    code
}

criterion_group!(benches, bench_parsers, bench_scaling);
criterion_main!(benches);
