use tree_sitter_perl::full_parser::FullPerlParser;

fn main() {
    // Test cases that should work with our new builtin list operator support
    let test_cases = vec![
        ("Basic print without parens", "print $x;"),
        ("Print with multiple args", "print $a, $b, $c;"),
        ("Say statement", "say \"Hello, world!\";"),
        ("Warn statement", "warn \"Something went wrong\";"),
        ("Die statement", "die \"Fatal error\";"),
        ("Print with heredoc", r#"print <<'EOF';
Hello world
EOF"#),
        ("Mixed heredocs", r#"my $single = <<'SINGLE';
No interpolation here: $var
SINGLE
my $double = <<"DOUBLE";  
Interpolation works: $var
DOUBLE
print $single, $double;"#),
    ];
    
    for (name, input) in test_cases {
        println!("\nTesting: {}", name);
        println!("Input: {}", input);
        
        let mut parser = FullPerlParser::new();
        match parser.parse(input) {
            Ok(_) => println!("✓ Parse succeeded"),
            Err(e) => println!("✗ Parse failed: {:?}", e),
        }
    }
}