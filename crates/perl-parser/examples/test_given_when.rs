//! Test given/when smart match
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic given/when
        "given ($x) {
            when (1) { print 'one' }
            when (2) { print 'two' }
            default { print 'other' }
        }",
        
        // When with multiple values
        "given ($value) {
            when ([1, 2, 3]) { print 'small' }
            when ([10, 20, 30]) { print 'medium' }
            default { print 'other' }
        }",
        
        // When with regex
        "given ($str) {
            when (/^foo/) { print 'starts with foo' }
            when (/bar$/) { print 'ends with bar' }
            default { print 'no match' }
        }",
        
        // Smart match operator
        "$x ~~ @array",
        "$str ~~ /pattern/",
        "$value ~~ [1, 2, 3]",
        
        // Nested given
        "given ($x) {
            when (1) { 
                given ($y) {
                    when ('a') { print '1a' }
                    when ('b') { print '1b' }
                }
            }
        }",
    ];
    
    for test in tests {
        println!("\nTesting: {}", test.lines().next().unwrap_or(""));
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
    }
}