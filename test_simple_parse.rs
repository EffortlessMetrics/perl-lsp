use perl_parser::Parser;

fn main() {
    let cases = vec![
        // Test named captures
        r#"/(?<name>\w+)/"#,
        r#"/(?'name'\w+)/"#,
        r#"/(?P<name>\w+)/"#,
        
        // Test quote operators
        "q{hello}",
        "qq{hello}",
        "qr{hello}",
        "qw{hello world}",
        
        // Test format
        r#"format STDOUT =
@<<<<<
$x
.
"#,
        
        // Test sub with attributes
        "sub foo :lvalue :method { }",
        
        // Test prototype with signature
        "sub foo($x, $y) { }",
    ];
    
    for (i, code) in cases.iter().enumerate() {
        println!("\nTest {}: {}", i + 1, code);
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => println!("✓ Parsed successfully"),
            Err(e) => println!("✗ Failed: {:?}", e),
        }
    }
}