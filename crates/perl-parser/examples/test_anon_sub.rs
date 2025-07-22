//! Test anonymous subroutines
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic anonymous subs
        "sub { }",
        "sub { return 42 }",
        "sub { my $x = shift; return $x + 1 }",
        
        // Assigned to variables
        "my $f = sub { }",
        "my $add = sub { $_[0] + $_[1] }",
        "our $handler = sub { die 'Not implemented' }",
        
        // As arguments
        "map { $_ * 2 } @list",
        "grep { $_ > 10 } @numbers",
        "sort { $a <=> $b } @values",
        
        // With signatures (modern Perl)
        "sub ($x) { $x * 2 }",
        "sub ($x, $y) { $x + $y }",
        
        // In expressions
        "(sub { 42 })->()",
        "my $result = (sub { $_[0] ** 2 })->(5)",
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