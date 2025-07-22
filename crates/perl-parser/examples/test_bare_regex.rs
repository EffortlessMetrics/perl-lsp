//! Test bare regex in conditionals
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Bare regex matching against $_
        "if (/pattern/) { }",
        "while (/\\w+/) { }",
        "unless (/error/) { }",
        
        // With modifiers
        "if (/pattern/i) { }",
        "if (/pattern/gi) { }",
        
        // Statement modifiers
        "print if /pattern/",
        "next if /skip/",
        "last unless /continue/",
        
        // In expressions
        "/pattern/ && print",
        "/pattern/ || die",
        
        // Negated
        "if (!/pattern/) { }",
        "print unless !/found/",
    ];
    
    for test in tests {
        println!("\nTesting: {}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success!");
                println!("   S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}