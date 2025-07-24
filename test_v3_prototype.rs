use perl_parser::Parser;

fn main() {
    let tests = vec![
        ("Simple prototype", r#"sub foo($) { }"#),
        ("Complex prototype", r#"sub mygrep(&@) { }"#),
        ("Multiple params", r#"sub bar($$) { }"#),
        ("Optional params", r#"sub baz($;$) { }"#),
        ("Slurpy array", r#"sub slurp(@) { }"#),
        ("Slurpy hash", r#"sub hash(%) { }"#),
        ("Mixed prototype", r#"sub mixed($@) { }"#),
        ("Code ref prototype", r#"sub code(&) { }"#),
        ("Prototype attribute", r#"sub qux :prototype($) ($x) { }"#),
    ];
    
    for (name, code) in tests {
        println!("\n=== {} ===", name);
        println!("Code: {}", code);
        
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ SUCCESS!");
                println!("S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ FAILED: {}", e);
            }
        }
    }
}