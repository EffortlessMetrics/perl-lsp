// Final test of all three parsers
use std::time::Instant;

const TEST_CASES: &[(&str, &str)] = &[
    ("Simple", "my $x = 42;"),
    ("Division", "$y = $x / 2;"),
    ("Regex", "if ($text =~ /pattern/) { }"),
    ("Complex", r#"
sub hello {
    my $name = shift;
    print "Hello, $name!\n";
}
hello("World");
"#),
];

fn main() {
    println!("=== Testing All Three Perl Parsers ===\n");
    
    for (name, code) in TEST_CASES {
        println!("Test case: {}", name);
        println!("Code: {}", code.trim());
        
        // Test 1: perl-parser
        {
            use perl_parser::Parser;
            let start = Instant::now();
            let mut parser = Parser::new(code);
            match parser.parse() {
                Ok(_) => println!("  ✅ perl-parser: Success ({}µs)", start.elapsed().as_micros()),
                Err(e) => println!("  ❌ perl-parser: Failed - {}", e),
            }
        }
        
        // Test 2: tree-sitter-c
        {
            use tree_sitter_perl_c::create_parser;
            let start = Instant::now();
            let mut parser = create_parser();
            match parser.parse(code, None) {
                Some(_) => println!("  ✅ tree-sitter-c: Success ({}µs)", start.elapsed().as_micros()),
                None => println!("  ❌ tree-sitter-c: Failed"),
            }
        }
        
        // Test 3: tree-sitter-perl-rs (Pest)
        #[cfg(feature = "pure-rust-standalone")]
        {
            use tree_sitter_perl::PureRustParser;
            let start = Instant::now();
            let parser = PureRustParser::new();
            match parser.parse(code) {
                Ok(_) => println!("  ✅ tree-sitter-perl-rs: Success ({}µs)", start.elapsed().as_micros()),
                Err(e) => println!("  ❌ tree-sitter-perl-rs: Failed - {:?}", e),
            }
        }
        #[cfg(not(feature = "pure-rust-standalone"))]
        {
            println!("  ⚠️  tree-sitter-perl-rs: Not compiled");
        }
        
        println!();
    }
}