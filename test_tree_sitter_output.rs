use perl_parser::Parser;

fn main() {
    let test_cases = vec![
        // Basic variable and print
        r#"my $x = 42; print $x;"#,
        
        // Function call with arguments
        r#"substr($str, 0, 5)"#,
        
        // If statement
        r#"if ($x > 0) { print "positive"; }"#,
        
        // Subroutine declaration
        r#"sub add { $_[0] + $_[1] }"#,
        
        // Array and hash
        r#"my @arr = (1, 2, 3); my %hash = (a => 1, b => 2);"#,
        
        // Method call
        r#"$obj->method($arg)"#,
        
        // Package and use
        r#"package Foo; use strict;"#,
        
        // Complex expression
        r#"$hash->{key}->[0] = $x * 2 + $y"#,
    ];
    
    for (i, code) in test_cases.iter().enumerate() {
        println!("\n--- Test Case {} ---", i + 1);
        println!("Code: {}", code);
        
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("S-expression: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
}