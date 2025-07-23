use perl_lexer::Lexer;
use perl_parser::Parser;

fn main() {
    let code = r#"
my $x = 42;
if ($x > 40) {
    print "Hello, world!\n";
}
"#;

    println!("Testing perl-lexer + perl-parser integration...\n");
    
    // Test lexer
    println!("=== Lexer Output ===");
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize();
    for token in &tokens[..10] {  // Show first 10 tokens
        println!("{:?}", token);
    }
    println!("... {} total tokens", tokens.len());
    
    // Test parser
    println!("\n=== Parser Output ===");
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            println!("Parse successful!");
            println!("AST: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("Parse error: {}", e);
        }
    }
}