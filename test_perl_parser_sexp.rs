use perl_parser::Parser;

fn main() {
    let test_cases = vec![
        "my $x = 42;",
        "print $x;",
        "$hashref->%{'foo', 'bar'}",
        "sub foo { return 42; }",
        "if ($x) { print 'yes'; }",
        "package Foo; 1;",
    ];
    
    for code in test_cases {
        println!("Code: {}", code);
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("S-exp: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
        println!();
    }
}