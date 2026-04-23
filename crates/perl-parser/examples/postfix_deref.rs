//! Example demonstrating postfix dereference parsing

use perl_parser::Parser;

fn main() {
    let examples = vec![
        // Basic postfix dereferences
        "$arrayref->@*",
        "$hashref->%*",
        "$scalarref->$*",
        "$coderef->&*",
        "$globref->**",
        // Array and hash slices
        "$arrayref->@[0..2]",
        "$hashref->%{'key1', 'key2'}",
        // Chained operations
        "$data->[0]->@*",
        "$obj->method()->%*",
        "$ref->{users}->[0]->{friends}->@*",
        // In expressions
        "my @copy = $ref->@*;",
        "print $ref->@*;",
        "foreach my $item ($ref->@*) { }",
    ];

    for example in examples {
        println!("\nParsing: {}", example);

        let mut parser = Parser::new(example);
        match parser.parse() {
            Ok(ast) => {
                println!("Success! AST:");
                println!("{}", ast.to_sexp());
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
}
