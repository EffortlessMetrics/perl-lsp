//! Test array/hash dereferencing
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Array access
        "$array[0]",
        "$array[1]",
        "$array[$i]",
        "$array[$i + 1]",
        // Hash access
        "$hash{key}",
        "$hash{$key}",
        "$hash{'key'}",
        // Method calls with dereferencing
        "$obj->method()",
        "$obj->method($arg)",
        "$obj->method($arg1, $arg2)",
        // Arrow dereferencing
        "$ref->[0]",
        "$ref->{key}",
        "$ref->{'key'}",
        // Nested dereferencing
        "$data->{users}[$i]{name}",
        "$hash{key1}{key2}",
        "$array[0][1]",
    ];

    for code in tests {
        println!("\nTesting: {}", code);
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success!");
                println!("   S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}
