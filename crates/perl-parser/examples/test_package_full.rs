//! Test full package example
use perl_parser::Parser;

fn main() {
    let input = r#"package Bar {
    use parent 'Foo';
    
    sub method {
        my $self = shift;
        $self->SUPER::method();
    }
}"#;

    println!("=== Input length: {} ===", input.len());
    println!("Character at position 112: {:?}", input.chars().nth(112));
    
    // Show context
    let start = 100;
    let end = 125.min(input.len());
    println!("\nContext around position 112:");
    println!("{}", &input[start..end]);
    println!("{}^", " ".repeat(112 - start));
    
    // Find the exact statement
    let lines: Vec<&str> = input.lines().collect();
    let mut pos = 0;
    for (i, line) in lines.iter().enumerate() {
        let line_end = pos + line.len() + 1; // +1 for newline
        if pos <= 112 && 112 < line_end {
            println!("\nError on line {}: {}", i + 1, line);
            println!("Position {} in line: column {}", 112, 112 - pos);
        }
        pos = line_end;
    }
    
    // Parse
    println!("\nParser output:");
    let mut parser = Parser::new(input);
    match parser.parse() {
        Ok(ast) => {
            println!("✅ Success! S-expr: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }
}