// Test the pure Pest parser directly
use tree_sitter_perl::pure_rust_parser::{PerlParser, PureRustPerlParser, Rule};
use pest::Parser;

fn main() {
    println!("Testing Pure Rust Pest Parser directly...\n");
    
    let test_cases = vec![
        ("Simple variable", "my $x = 42;"),
        ("Print statement", "print \"Hello, world\\n\";"),
        ("If statement", "if ($x > 10) { print $x; }"),
        ("Division", "$y = $x / 2;"),
        ("Regex match", "if ($text =~ /pattern/) { }"),
        ("Substitution", "$text =~ s/foo/bar/g;"),
    ];
    
    // Test 1: Raw Pest parser
    println!("=== Testing Raw Pest Parser ===");
    for (name, code) in &test_cases {
        print!("{}: ", name);
        match PerlParser::parse(Rule::program, code) {
            Ok(pairs) => {
                println!("✅ Parsed {} pairs", pairs.count());
            }
            Err(e) => {
                println!("❌ Failed: {:?}", e);
            }
        }
    }
    
    // Test 2: PureRustPerlParser wrapper
    println!("\n=== Testing PureRustPerlParser ===");
    for (name, code) in &test_cases {
        print!("{}: ", name);
        let mut parser = PureRustPerlParser::new();
        match parser.parse(code) {
            Ok(ast) => {
                println!("✅ Success");
            }
            Err(e) => {
                println!("❌ Failed: {:?}", e);
            }
        }
    }
}