//! Test nested structures

use perl_parser::Parser;

fn main() {
    // Test simpler versions first
    println!("=== Simple for loop ===");
    let simple_for = "for (my $i = 0; $i < 10; $i++) { print $i; }";
    test_parse(simple_for);

    println!("\n=== For loop in block ===");
    let for_in_block = "{ for (my $i = 0; $i < 10; $i++) { print $i; } }";
    test_parse(for_in_block);

    println!("\n=== Full nested structure ===");
    let code = r#"
if ($x) {
    while ($y) {
        for (my $i = 0; $i < 10; $i++) {
            print $i;
        }
    }
}
"#;

    println!("Code: {}", code);

    test_parse(code);
}

fn test_parse(code: &str) {
    println!("Code: {}", code);
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            println!("✅ Success!");
            println!("S-expr: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }
}
