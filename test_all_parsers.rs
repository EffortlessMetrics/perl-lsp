use std::time::Instant;

const TEST_CODE: &str = r#"
my $x = 42;
if ($x > 40) {
    print "Hello, world!\n";
}
"#;

fn main() {
    println!("Testing all three Perl parsers...\n");
    
    // Test 1: perl-lexer + perl-parser
    {
        use perl_parser::Parser;
        let start = Instant::now();
        let mut parser = Parser::new(TEST_CODE);
        match parser.parse() {
            Ok(_) => println!("✅ perl-parser: Success ({}µs)", start.elapsed().as_micros()),
            Err(e) => println!("❌ perl-parser: Failed - {}", e),
        }
    }
    
    // Test 2: tree-sitter-perl-c
    {
        use tree_sitter_perl_c::create_parser;
        let start = Instant::now();
        let mut parser = create_parser();
        match parser.parse(TEST_CODE, None) {
            Some(_) => println!("✅ tree-sitter-c: Success ({}µs)", start.elapsed().as_micros()),
            None => println!("❌ tree-sitter-c: Failed"),
        }
    }
    
    // Test 3: tree-sitter-perl-rs (Pure Rust Pest)
    #[cfg(feature = "tree-sitter-perl-rs")]
    {
        use tree_sitter_perl::PureRustParser;
        let start = Instant::now();
        let parser = PureRustParser::new();
        match parser.parse(TEST_CODE) {
            Ok(_) => println!("✅ tree-sitter-perl-rs: Success ({}µs)", start.elapsed().as_micros()),
            Err(e) => println!("❌ tree-sitter-perl-rs: Failed - {:?}", e),
        }
    }
    #[cfg(not(feature = "tree-sitter-perl-rs"))]
    {
        println!("⚠️  tree-sitter-perl-rs: Not compiled (use --features tree-sitter-perl-rs)");
    }
}