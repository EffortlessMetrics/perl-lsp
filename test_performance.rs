use std::time::Instant;
use tree_sitter_perl::perl_lexer::PerlLexer;

fn main() {
    let test_cases = vec![
        ("simple", r#"print "Hello, World!";"#),
        ("variables", r#"my $x = 42; my @arr = (1, 2, 3); my %hash = (a => 1);"#),
        ("references", r#"my $ref = \$scalar; my $aref = \@array; my $href = \%hash;"#),
        ("operators", r#"$x = 1 + 2 * 3; $y = $x ** 2; $z = $x ... $y;"#),
        ("unicode", r#"my $π = 3.14159; my $café = "coffee"; sub 日本語 { }"#),
        ("octal", r#"my $perms = 0755; my $modern = 0o755; my $hex = 0xFF;"#),
    ];

    println!("Performance Test Results");
    println!("========================\n");

    for (name, code) in test_cases {
        let start = Instant::now();
        let mut lexer = PerlLexer::new(code);
        let mut token_count = 0;
        
        while let Some(_token) = lexer.next_token() {
            token_count += 1;
        }
        
        let duration = start.elapsed();
        let microseconds = duration.as_micros();
        
        println!("{:15} {} tokens in {} µs ({:.2} µs/token)", 
                 format!("{}:", name), 
                 token_count, 
                 microseconds,
                 microseconds as f64 / token_count as f64);
    }
}