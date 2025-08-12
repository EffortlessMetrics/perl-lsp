//! Test comment handling
use perl_parser::Parser;

fn main() {
    let tests = vec![
        "# This is a comment",
        "my $x = 1; # inline comment",
        r#"# First comment
my $x = 1;
# Second comment
my $y = 2;"#,
    ];

    for test in tests {
        println!("\nTesting: {:?}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success!");
                println!("S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}
