use tree_sitter_perl::full_parser::FullPerlParser;
use tree_sitter_perl::pure_rust_parser::PureRustPerlParser;

fn main() {
    // Start with the simplest case that fails
    let test_cases = vec![
        ("Simple function", "print(\"a\", \"b\");"),
        ("With placeholders", "print(__HEREDOC_1__, __HEREDOC_2__);"),
        ("Two heredocs", r#"print(<<A, <<B);
First
A
Second
B"#),
        ("Three heredocs", r#"print(<<A, <<B, <<C);
First content
A
Second content
B
Third content
C"#),
    ];

    for (name, input) in test_cases {
        println!("\n=== Testing: {} ===", name);
        println!("Input:\n{}", input);
        
        let mut parser = FullPerlParser::new();
        match parser.parse(input) {
            Ok(ast) => {
                println!("✓ Parse succeeded!");
                // Try to generate S-expression
                let pure_parser = PureRustPerlParser::new();
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    pure_parser.to_sexp(&ast)
                })) {
                    Ok(sexp) => println!("S-expression:\n{}", sexp),
                    Err(_) => println!("✗ S-expression generation panicked!"),
                }
            }
            Err(e) => println!("✗ Parse failed: {:?}", e),
        }
    }
}