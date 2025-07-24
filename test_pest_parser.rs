fn main() {
    #[cfg(feature = "pure-rust-standalone")]
    {
        use tree_sitter_perl::PureRustParser;
        
        let code = r#"
my $x = 42;
print "Hello, $x\n";
"#;
        
        println!("Testing Pure Rust Pest parser...");
        let parser = PureRustParser::new();
        match parser.parse(code) {
            Ok(ast) => println!("✅ Parse successful! AST: {:?}", ast),
            Err(e) => println!("❌ Parse failed: {:?}", e),
        }
    }
    
    #[cfg(not(feature = "pure-rust-standalone"))]
    {
        println!("Compile with --features pure-rust-standalone");
    }
}