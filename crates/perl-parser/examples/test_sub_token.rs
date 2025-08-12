//! Test __SUB__ token
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic __SUB__ usage
        "__SUB__",
        "my $self = __SUB__",
        // __SUB__ in recursive calls
        "sub factorial { my $n = shift; return $n <= 1 ? 1 : $n * __SUB__->($n - 1); }",
        // __SUB__ in anonymous subs
        "my $fib = sub { my $n = shift; return $n < 2 ? $n : __SUB__->($n-1) + __SUB__->($n-2); }",
        // __SUB__ with method calls
        "__SUB__->()",
        "__SUB__->(@args)",
        // __SUB__ in expressions
        "defined __SUB__",
        "ref __SUB__ eq 'CODE'",
    ];

    for test in tests {
        print!("Testing: {:40} ", test);
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
        println!();
    }
}
