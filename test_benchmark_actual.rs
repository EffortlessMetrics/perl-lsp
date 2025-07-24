use std::time::Instant;

fn main() {
    // Test the actual benchmark setup
    println!("Testing parser benchmark configuration...\n");
    
    // Test 1: perl-parser
    {
        use perl_parser::Parser;
        let code = "my $x = 42;";
        let start = Instant::now();
        for _ in 0..1000 {
            let mut parser = Parser::new(code);
            let _ = parser.parse();
        }
        let elapsed = start.elapsed();
        println!("perl-parser: 1000 iterations in {:?} ({:.2} µs/iter)", 
                 elapsed, elapsed.as_micros() as f64 / 1000.0);
    }
    
    // Test 2: tree-sitter-perl-c
    {
        use tree_sitter_perl_c::create_parser;
        let code = "my $x = 42;";
        let start = Instant::now();
        for _ in 0..1000 {
            let mut parser = create_parser();
            let _ = parser.parse(code, None);
        }
        let elapsed = start.elapsed();
        println!("tree-sitter-c: 1000 iterations in {:?} ({:.2} µs/iter)", 
                 elapsed, elapsed.as_micros() as f64 / 1000.0);
    }
    
    // Test 3: tree-sitter-perl-rs with pure-rust-standalone
    #[cfg(feature = "pure-rust-standalone")]
    {
        use tree_sitter_perl::PureRustParser;
        let code = "my $x = 42;";
        let start = Instant::now();
        let parser = PureRustParser::new();
        
        // Try one parse first to see if it works
        match parser.parse(code) {
            Ok(ast) => {
                println!("\nPure Rust parser works! Testing performance...");
                let start = Instant::now();
                for _ in 0..1000 {
                    let _ = parser.parse(code);
                }
                let elapsed = start.elapsed();
                println!("tree-sitter-perl-rs: 1000 iterations in {:?} ({:.2} µs/iter)", 
                         elapsed, elapsed.as_micros() as f64 / 1000.0);
            }
            Err(e) => {
                println!("\n❌ Pure Rust parser FAILED: {:?}", e);
                println!("This would crash in benchmarks!");
            }
        }
    }
}