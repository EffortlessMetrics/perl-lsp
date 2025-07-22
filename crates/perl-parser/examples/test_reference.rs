use perl_parser::Parser;

fn main() {
    let test_cases = vec![
        // Basic reference operations
        r"\$scalar",
        r"\@array",
        r"\%hash",
        r"\&mysub",
        
        // Reference in assignment
        r"my $ref = \$scalar",
        r"$ref = \@array",
        r"$ref = \%hash",
        r"$ref = \&mysub",
        
        // Reference to expressions
        r"\($x + $y)",
        r"\$_",
        r"\$array[0]",
        r"\$hash{key}",
        
        // Nested references
        r"\\$scalar",
        r"\\\$scalar",
    ];
    
    for code in test_cases {
        println!("\nTesting: {}", code);
        
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}