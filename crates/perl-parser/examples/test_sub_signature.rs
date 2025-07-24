//! Test subroutine signature parsing
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Regular sub
        "sub simple { return 42; }",
        
        // Sub with old-style params
        "sub with_params { my ($x, $y) = @_; return $x + $y; }",
        
        // Sub with signature
        "sub with_signature ($x, $y) { return $x + $y; }",
        
        // Anonymous sub
        "my $anon = sub { return \"anonymous\"; };",
    ];

    for test in tests {
        print!("Testing: {:50} ", test);
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