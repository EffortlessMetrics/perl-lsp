use perl_parser::Parser;

fn main() {
    let test_cases = ["my ($x, $y);",
        "state ($a, $b) = (1, 2);",
        "our ($scalar, @array, %hash);",
        "my ();",
        "local ($x1, $y1, $z1) = get_values();",
        "my ($foo, $bar) = (1, 2);",
        "state (@list1, @list2) = ([], [1, 2, 3]);",
        "my ($single) = 42;"];

    for (i, code) in test_cases.iter().enumerate() {
        println!("\nTest case {}: {}", i + 1, code);
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("✓ Success: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("✗ Error: {:?}", e);
            }
        }
    }
}