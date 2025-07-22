//! Test postfix dereferencing operators
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Array postfix dereference
        "$ref->@*",
        "$array_ref->@*",
        "$obj->method()->@*",
        
        // Hash postfix dereference
        "$ref->%*",
        "$hash_ref->%*",
        "$obj->get_hash()->%*",
        
        // Scalar postfix dereference
        "$ref->$*",
        "$scalar_ref->$*",
        
        // Code postfix dereference
        "$ref->&*",
        "$code_ref->&*",
        
        // Glob postfix dereference
        "$ref->**",
        "$glob_ref->**",
        
        // Slice operations
        "$array_ref->@[0,1,2]",
        "$hash_ref->@{qw(foo bar)}",
        
        // Chained dereferencing
        "$data->{users}->@*",
        "$config->{servers}->[0]->%*",
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