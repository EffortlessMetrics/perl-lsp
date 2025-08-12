use perl_lexer::{PerlLexer, Token};
use std::time::Instant;

fn collect_all_tokens(mut lexer: PerlLexer) -> Vec<Token> {
    let mut tokens = Vec::new();
    while let Some(token) = lexer.next_token() {
        tokens.push(token);
    }
    tokens
}

fn main() {
    let test_cases = vec![
        ("simple", "my $x = 42; print $x;"),
        ("slash_division", "10 / 2 + 5 / 3"),
        ("slash_regex", "if (/pattern/) { }"),
        ("operators", "$a += $b -= $c *= $d /= $e"),
        ("whitespace", "    my   $x   =   42  ;   # comment\n"),
        ("numbers", "123 456.789 1_234_567 1.23e45"),
    ];

    println!("Perl Lexer Performance Test");
    println!("===========================");

    // Warmup
    for _ in 0..1000 {
        let lexer = PerlLexer::new("my $x = 42;");
        let _ = collect_all_tokens(lexer);
    }

    for (name, input) in &test_cases {
        let iterations = 10_000;
        let start = Instant::now();

        for _ in 0..iterations {
            let lexer = PerlLexer::new(input);
            let _ = collect_all_tokens(lexer);
        }

        let elapsed = start.elapsed();
        let per_iter = elapsed / iterations;

        println!(
            "{:<15} {:>8.2} µs/iter   (input: {} bytes)",
            name,
            per_iter.as_nanos() as f64 / 1000.0,
            input.len()
        );
    }

    // Test larger file
    println!("\nLarge file test:");
    let mut large_input = String::new();
    for i in 0..100 {
        large_input.push_str(&format!("my $var{} = {}; print $var{};\n", i, i * 2, i));
    }

    let iterations = 100;
    let start = Instant::now();

    for _ in 0..iterations {
        let lexer = PerlLexer::new(&large_input);
        let tokens = collect_all_tokens(lexer);
        assert!(tokens.len() > 0);
    }

    let elapsed = start.elapsed();
    let per_iter = elapsed / iterations;

    println!(
        "Large file      {:>8.2} µs/iter   (input: {} bytes, {} tokens)",
        per_iter.as_nanos() as f64 / 1000.0,
        large_input.len(),
        {
            let lexer = PerlLexer::new(&large_input);
            collect_all_tokens(lexer).len()
        }
    );
}
