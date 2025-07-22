use perl_parser::Parser;

fn main() {
    // Test cases for fat arrow (=>) operator in expression context
    let test_cases = vec![
        ("Assignment to empty hash ref", "$x = {}"),
        ("Assignment to simple hash ref", "$x = { key => 'value' }"),
        ("Assignment to multi-element hash ref", "$x = { a => 1, b => 2 }"),
        ("Hash variable assignment", "my %h = (foo => 'bar')"),
        ("Assignment to mixed hash ref", "$x = { 'one', 1, two => 2 }"),
        ("Assignment to nested hash ref", "$x = { outer => { inner => 'value' } }"),
        ("Return hash ref", "return { status => 'ok', code => 200 }"),
        ("Hash ref in array", "@a = ({ a => 1 }, { b => 2 })"),
    ];
    
    println!("=== Testing Fat Arrow (=>) Operator in Expression Context ===\n");
    
    for (name, code) in test_cases {
        println!("Test: {}", name);
        println!("Code: {}", code);
        
        let mut parser = Parser::new(code);
        
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success!");
                println!("S-expression: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
        
        println!();
    }
}