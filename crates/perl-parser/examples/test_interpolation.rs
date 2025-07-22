//! Test string interpolation
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Simple strings (no interpolation)
        r#"'hello'"#,
        r#"'hello world'"#,
        
        // Double-quoted strings (interpolation)
        r#""hello""#,
        r#""hello world""#,
        r#""hello $name""#,
        r#""hello ${name}""#,
        r#""array: @array""#,
        r#""hash: %hash""#,
        
        // Escape sequences
        r#""hello\n""#,
        r#""hello\tworld""#,
        r#""quote: \"""#,
        
        // Complex interpolation
        r#""$hash{key}""#,
        r#""$array[0]""#,
        r#""$obj->method()""#,
        r#""${$ref}""#,
        
        // Mixed
        r#""Hello $name, you have $count items""#,
        r#""Path: $ENV{PATH}""#,
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