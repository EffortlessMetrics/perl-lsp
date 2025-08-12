//! Test eval blocks and expressions
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic eval block
        "eval { print 'hello' }",
        // Eval with error handling
        "eval { die 'error' }; if ($@) { print 'caught' }",
        // Eval string
        "eval \"print 'dynamic code'\"",
        // Eval in assignment
        "my $result = eval { 1 / $x }",
        // Nested eval
        "eval { eval { risky_operation() } }",
        // Eval with return
        "my $value = eval { return 42 }",
        // Eval in condition
        "if (eval { $x > 0 }) { print 'positive' }",
        // Multiple statements in eval
        "eval { 
            my $temp = compute();
            process($temp);
            cleanup();
        }",
    ];

    for test in tests {
        println!("\nTesting: {}", test.lines().next().unwrap_or(""));
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
