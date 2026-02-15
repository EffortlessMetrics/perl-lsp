use std::time::Instant;
use tree_sitter::{Parser, Language};

// External C function
unsafe extern "C" {
    fn tree_sitter_perl() -> *const tree_sitter::ffi::TSLanguage;
}

fn language() -> Language {
    unsafe { Language::from_raw(tree_sitter_perl()) }
}

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

    println!("=== C Parser (tree-sitter) Benchmark ===");
    println!("Testing the C implementation with scanner.c\n");

    for (name, code) in &test_cases {
        println!("Test: {}", name);
        println!("Code size: {} bytes", code.len());
        
        // Create parser once for warmup
        let mut parser = Parser::new();
        if parser.set_language(&language()).is_err() {
            eprintln!("Failed to set language");
            continue;
        }
        
        // Warmup
        for _ in 0..10 {
            let _ = parser.parse(code, None);
        }
        
        // Benchmark
        let iterations = 1000;
        let mut times = Vec::with_capacity(iterations);
        let mut success_count = 0;
        
        let start_total = Instant::now();
        for _ in 0..iterations {
            let start = Instant::now();
            match parser.parse(code, None) {
                Some(_) => success_count += 1,
                None => eprintln!("Parse failed"),
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
    println!("This is the C parser using scanner.c for lexing.");
    println!("It handles slash disambiguation through stateful scanning.");
    println!("The C implementation is typically faster but less safe than Rust.");
}
