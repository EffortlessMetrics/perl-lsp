use perl_parser::Parser;
use std::time::Instant;

fn main() {
    println!("=== PR #158 Substitution Operator Performance Analysis ===\n");

    // Test cases
    let test_cases = vec![
        (
            "Non-substitution code",
            r#"
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
"#,
        ),
        (
            "Basic substitution",
            r#"
s/foo/bar/;
s/pattern/replacement/g;
s/old/new/i;
s/test/result/gi;
"#,
        ),
        (
            "Complex substitution with delimiters",
            r#"
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
"#,
        ),
        (
            "Mixed code with substitution",
            r#"
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
"#,
        ),
    ];

    let iterations = 1000;

    for (name, code) in test_cases {
        println!("Testing: {}", name);

        let start = Instant::now();
        let mut successful_parses = 0;
        let mut total_nodes = 0;

        for _ in 0..iterations {
            let mut parser = Parser::new(code);
            match parser.parse() {
                Ok(ast) => {
                    successful_parses += 1;
                    // Count AST nodes for complexity measure
                    let sexp = ast.to_sexp();
                    total_nodes += sexp.matches('(').count();
                }
                Err(_) => {
                    // Continue on parse error
                }
            }
        }

        let duration = start.elapsed();
        let avg_time = duration.as_nanos() as f64 / iterations as f64;
        let success_rate = (successful_parses as f64 / iterations as f64) * 100.0;
        let avg_nodes = if successful_parses > 0 { total_nodes / successful_parses } else { 0 };

        println!(
            "  Time: {:.2} µs/parse (total: {:.2} ms)",
            avg_time / 1000.0,
            duration.as_millis()
        );
        println!("  Success rate: {:.1}% ({}/{})", success_rate, successful_parses, iterations);
        println!("  Avg AST nodes: {}", avg_nodes);
        println!("  Performance: {:.0} parses/sec\n", 1_000_000_000.0 / avg_time);
    }

    // Delimiter-specific performance test
    println!("=== Delimiter Performance Analysis ===");
    let delimiter_tests = vec![
        ("s/foo/bar/g", "slash_delimiters"),
        ("s{foo}{bar}g", "brace_delimiters"),
        ("s|foo|bar|g", "pipe_delimiters"),
        ("s'foo'bar'g", "quote_delimiters"),
        ("s<foo><bar>g", "angle_delimiters"),
        ("s!foo!bar!g", "exclamation_delimiters"),
    ];

    for (code, name) in delimiter_tests {
        let start = Instant::now();
        let mut successful = 0;

        for _ in 0..iterations {
            let mut parser = Parser::new(code);
            if parser.parse().is_ok() {
                successful += 1;
            }
        }

        let duration = start.elapsed();
        let avg_time = duration.as_nanos() as f64 / iterations as f64;
        let success_rate = (successful as f64 / iterations as f64) * 100.0;

        println!("{}: {:.2} µs/parse, {:.1}% success", name, avg_time / 1000.0, success_rate);
    }

    // Memory usage estimation (rough)
    println!("\n=== Memory Usage Estimation ===");
    let complex_code = r#"s/foo/bar/g; s{from}{to}g; s|old|new|i; s'pattern'replacement'g;"#;
    let mut parser = Parser::new(complex_code);
    match parser.parse() {
        Ok(ast) => {
            let sexp = ast.to_sexp();
            println!("AST S-expression length: {} characters", sexp.len());
            println!("Estimated memory per substitution: ~{} bytes", sexp.len() / 4);
        }
        Err(e) => {
            println!("Failed to parse complex substitution: {:?}", e);
        }
    }
}
