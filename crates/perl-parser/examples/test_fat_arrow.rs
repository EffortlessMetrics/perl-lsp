use perl_parser::Parser;
use std::fs;

fn main() {
    // Test cases for fat arrow (=>) operator
    let test_cases = vec![
        ("Empty hash ref", "{}"),
        ("Simple hash ref", "{ key => 'value' }"),
        ("Multi-element hash ref", "{ a => 1, b => 2 }"),
        ("Hash assignment", "my %h = (foo => 'bar')"),
        ("Mixed commas and arrows", "{ 'one', 1, two => 2 }"),
        ("Nested hash refs", "{ outer => { inner => 'value' } }"),
        ("Array with fat arrows", "(a => 1, b => 2)"),
        (
            "Function call with arrows",
            "print(foo => 'bar', baz => 'qux')",
        ),
    ];

    println!("=== Testing Fat Arrow (=>) Operator ===\n");

    for (name, code) in test_cases {
        println!("Test: {}", name);
        println!("Code: {}", code);

        let mut parser = Parser::new(code);

        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success!");
                println!("S-expression: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }

        println!();
    }

    // Test with file if provided
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        println!("=== Testing file: {} ===\n", args[1]);

        match fs::read_to_string(&args[1]) {
            Ok(content) => {
                let mut parser = Parser::new(&content);

                match parser.parse() {
                    Ok(ast) => {
                        println!("✅ Successfully parsed file!");
                        println!("\nS-expression:");
                        println!("{}", ast.to_sexp());
                    }
                    Err(e) => {
                        println!("❌ Parse error: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("❌ Error reading file: {}", e);
            }
        }
    }
}
