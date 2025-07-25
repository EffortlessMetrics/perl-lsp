use perl_parser::{Parser, TokenStream};

fn main() {
    let cases = vec![
        ("sub test(_) { }", "underscore prototype"),
        ("my $x = $a // $b;", "defined-or in assignment"),
        ("say $x // 'default';", "defined-or in expression"),
        ("print *$ref;", "glob deref after keyword"),
        ("use constant { FOO => 42, BAR => 43 };", "constant pragma hash"),
        ("use constant FOO => 42;", "constant pragma scalar"),
    ];
    
    for (code, desc) in cases {
        println!("\n=== {} ===", desc);
        println!("Code: {}", code);
        
        // Show tokens
        println!("Tokens:");
        let mut stream = TokenStream::new(code);
        let mut count = 0;
        while !stream.is_eof() && count < 20 {
            match stream.peek() {
                Ok(token) => {
                    println!("  {:?} '{}' at {}", token.kind, token.text, token.start);
                    stream.next().unwrap();
                }
                Err(e) => {
                    println!("  Token error: {:?}", e);
                    break;
                }
            }
            count += 1;
        }
        
        // Try parsing
        println!("Parse result:");
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => println!("  ✓ Success: {}", ast.to_sexp()),
            Err(e) => println!("  ✗ Error: {:?}", e),
        }
    }
}