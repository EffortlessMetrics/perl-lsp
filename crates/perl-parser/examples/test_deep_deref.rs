use perl_parser::Parser;

fn main() {
    let tests = vec!["$hash->{key}", "$hash->{key}->[0]", "$hash->{key}->[0]->{sub}"];

    for test in tests {
        println!("\nTesting: {}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(_) => println!("  ✅ Success"),
            Err(e) => println!("  ❌ Error: {}", e),
        }
    }
}
