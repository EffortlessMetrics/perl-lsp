use std::time::Instant;
use tree_sitter_perl::FullPerlParser;

fn main() {
    let test_cases = vec![
        ("simple", r#"print "Hello, World!\n";"#),
        ("variables", r#"my $x = 42; my @arr = (1, 2, 3); my %hash = (a => 1);"#),
        ("slash_disambiguation", r#"
            my $x = 10 / 2;  # Division
            if ($str =~ /pattern/) {  # Regex
                $str =~ s/foo/bar/g;  # Substitution
            }
            print 1/ /abc/;  # Division followed by regex
        "#),
        ("complex", r#"
            package MyModule;
            use strict;
            use warnings;
            
            # Test reference operator
            my $scalar = "test";
            my $ref = \$scalar;
            my $aref = \@array;
            my $href = \%hash;
            
            # Test modern octal
            my $perms = 0o755;
            
            # Test ellipsis
            sub not_implemented { ... }
            
            # Unicode identifiers
            my $π = 3.14159;
            my $café = "coffee";
            sub 日本語 { "works" }
            
            # Complex expressions with slash disambiguation
            sub calculate {
                my ($x, $y) = @_;
                return $x / $y if $y != 0;  # Division
                return 0 if $x =~ /^0+$/;   # Regex
            }
            
            # Heredoc with slashes
            my $config = <<'EOF';
            path: /usr/local/bin
            regex: /\w+/
            division: 10/2
            EOF
            
            1;
        "#),
        ("stress_slash", r#"
            # Stress test for slash disambiguation
            my $a = 1/2/3;  # Nested divisions
            my $b = $x / $y =~ /pattern/;  # Division and regex
            print 1/ /foo/ /bar/;  # Complex case
            s/a/b/g / s/c/d/;  # Substitution followed by division
            my $c = (10/2) =~ s/5/five/r;  # Division result with substitution
        "#),
    ];

    println!("=== Lexer+Pest (Pure Rust) Parser Benchmark ===");
    println!("Testing the multi-phase parser with Rust lexer preprocessing\n");

    for (name, code) in &test_cases {
        println!("Test: {}", name);
        println!("Code size: {} bytes", code.len());
        
        // Warmup
        let mut parser = FullPerlParser::new();
        for _ in 0..10 {
            let _ = parser.parse(code);
        }
        
        // Benchmark
        let iterations = 1000;
        let mut times = Vec::with_capacity(iterations);
        let mut success_count = 0;
        
        let start_total = Instant::now();
        for _ in 0..iterations {
            let mut parser = FullPerlParser::new();
            let start = Instant::now();
            match parser.parse(code) {
                Ok(_) => success_count += 1,
                Err(e) => eprintln!("Parse error: {:?}", e),
            }
            times.push(start.elapsed());
        }
        let total_duration = start_total.elapsed();
        
        // Calculate statistics
        times.sort();
        let min = times[0];
        let max = times[times.len() - 1];
        let median = times[times.len() / 2];
        let avg = total_duration / iterations as u32;
        
        println!("  Success rate: {}/{}", success_count, iterations);
        println!("  Min: {:?}", min);
        println!("  Max: {:?}", max);
        println!("  Median: {:?}", median);
        println!("  Average: {:?}", avg);
        println!("  Throughput: {:.2} MB/s", 
                 (code.len() as f64 * iterations as f64) / total_duration.as_secs_f64() / 1_000_000.0);
        println!();
    }
    
    println!("\n=== Analysis ===");
    println!("This parser uses a multi-phase approach:");
    println!("1. Heredoc processing");
    println!("2. Rust lexer preprocessing for slash disambiguation");
    println!("3. Pest parsing with disambiguated input");
    println!("4. AST building");
    println!("5. Postprocessing to restore original tokens");
    println!("\nThe Rust lexer preprocessing makes slash handling deterministic,");
    println!("avoiding the ambiguity issues that plague traditional parsers.");
}