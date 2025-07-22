use perl_parser::Parser;

fn main() {
    println!("Perl Parser - List-Style Variable Declarations Demo");
    println!("==================================================\n");

    let examples = vec![
        ("Single variable declaration", "my $x = 42;"),
        ("List declaration without init", "my ($x, $y, $z);"),
        ("List declaration with init", "state ($a, $b) = (1, 2);"),
        ("Mixed sigils in list", "our ($scalar, @array, %hash);"),
        ("Empty list declaration", "my ();"),
        ("Complex initializer", "local ($x, $y) = map { $_ * 2 } (5, 10);"),
        ("Function parameter unpacking", "my ($self, $name, $value) = @_;"),
        ("List in for loop", "for (my ($i, $j) = (0, 10); $i < 10; $i++) { }"),
    ];

    for (description, code) in examples {
        println!("{}", description);
        println!("Code: {}", code);
        
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("AST:  {}", ast.to_sexp());
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
        println!();
    }

    // Demonstrate error handling
    println!("Error handling examples:");
    println!("========================\n");

    let error_cases = vec![
        ("Missing closing paren", "my ($x, $y"),
        ("Invalid separator", "my ($x; $y);"),
        ("Missing variable sigil", "my (x, $y);"),
    ];

    for (description, code) in error_cases {
        println!("{}", description);
        println!("Code: {}", code);
        
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(_) => {
                println!("Unexpectedly succeeded!");
            }
            Err(e) => {
                println!("Expected error: {:?}", e);
            }
        }
        println!();
    }
}