//! Test anonymous subroutine simple case
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Empty anonymous sub
        ("my $f = sub { }", "empty anonymous sub"),
        // With semicolon in statement
        ("my $f = sub { return 42; }", "anonymous sub with semicolon"),
        // The failing case
        (
            r#"my $anon = sub { return "anonymous"; };"#,
            "anonymous sub with string and semicolon",
        ),
        // Without the trailing semicolon
        (
            r#"my $anon = sub { return "anonymous"; }"#,
            "anonymous sub without trailing semicolon",
        ),
    ];

    for (test, desc) in tests {
        println!("\n=== Testing: {} ===", desc);
        println!("Input: {}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success! S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}
