use perl_parser::Parser;

fn main() {
    println!("=== Testing Missing Features ===\n");

    // Test ISA operator
    println!("1. Testing ISA operator:");
    test_code("$obj ISA 'MyClass'");
    test_code("$x ISA $class");

    // Test file test operators
    println!("\n2. Testing file test operators:");
    test_code("-f $file");
    test_code("-d $dir");
    test_code("-e $path");

    // Test bareword functions
    println!("\n3. Testing bareword function calls:");
    test_code("defined $x");
    test_code("exists $hash{key}");
    test_code("print if defined $var");

    println!("\n=== What we found ===");
    println!("ISA is likely parsed as: identifier ISA identifier");
    println!("File tests are likely parsed as: unary minus followed by identifier");
    println!("Bareword functions need special handling in the parser");
}

fn test_code(code: &str) {
    print!("  {:30} -> ", code);
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            let sexp = ast.to_sexp();
            // Shorten output for readability
            if sexp.len() > 60 {
                println!("{} ...", &sexp[..60]);
            } else {
                println!("{}", sexp);
            }
        }
        Err(e) => println!("Error: {:?}", e),
    }
}
