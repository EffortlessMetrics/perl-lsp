use perl_parser::Parser;

fn main() {
    println!("=== Testing Missing Features ===\n");
    
    // Test ISA operator
    println!("1. Testing ISA operator:");
    let isa_tests = vec![
        "$obj ISA 'MyClass'",
        "$x ISA $class",
        "ref($x) ISA 'ARRAY'",
    ];
    
    for test in isa_tests {
        println!("  Code: {}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => println!("  ✅ Result: {}", ast.to_sexp()),
            Err(e) => println!("  ❌ Error: {:?}", e),
        }
    }
    
    // Test file test operators
    println!("\n2. Testing file test operators:");
    let file_tests = vec![
        "-f $file",
        "-d $dir",
        "-e $path",
        "-r $file && -w $file",
        "-s $file > 0",
    ];
    
    for test in file_tests {
        println!("  Code: {}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => println!("  ✅ Result: {}", ast.to_sexp()),
            Err(e) => println!("  ❌ Error: {:?}", e),
        }
    }
    
    // Test defined without parens
    println!("\n3. Testing 'defined' without parentheses:");
    let defined_tests = vec![
        "defined $x",
        "print if defined $var",
        "return 42 if defined $value",
    ];
    
    for test in defined_tests {
        println!("  Code: {}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => println!("  ✅ Result: {}", ast.to_sexp()),
            Err(e) => println!("  ❌ Error: {:?}", e),
        }
    }
    
    println!("\n=== Summary ===");
    println!("Features that need implementation:");
    println!("1. ISA operator - currently parsed as two identifiers");
    println!("2. File test operators (-f, -d, etc.) - minus is parsed as subtraction");
    println!("3. Bareword function calls (defined, exists without parens)");
}