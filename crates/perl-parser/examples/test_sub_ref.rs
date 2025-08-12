//! Test subroutine references
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic sub references
        "\\&sub",
        "\\&main::foo",
        "\\&Package::method",
        // In assignments
        "my $ref = \\&mysub",
        "$coderef = \\&handler",
        // With sigils
        "&sub",
        "&main::foo",
        "&{$coderef}",
        // Calling through references
        "&$coderef()",
        "$coderef->()",
        "&{$ref}(@args)",
    ];

    for test in tests {
        print!("Testing: {:30} ", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}
