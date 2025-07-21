//! Demo of the modern two-crate architecture
//!
//! This example shows how the perl-parser crate cleanly consumes
//! tokens from perl-lexer to produce an AST.

use perl_parser::Parser;

fn main() {
    println!("=== Modern Two-Crate Architecture Demo ===\n");
    println!("This demonstrates the clean separation between:");
    println!("1. perl-lexer: Converts Perl source → Token stream");
    println!("2. perl-parser: Converts Token stream → AST\n");
    
    let test_cases = vec![
        ("Simple variable", "my $x = 42;"),
        ("String assignment", "my $name = \"Perl\";"),
        ("Array declaration", "my @items = (1, 2, 3);"),
        ("Hash declaration", "my %config = (debug => 1);"),
        ("If statement", "if ($x > 10) { print $x; }"),
        ("While loop", "while ($i < 10) { $i++; }"),
        ("Function definition", "sub greet { print \"Hello\"; }"),
        ("Complex expression", "$result = ($a + $b) * $c;"),
    ];
    
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
    
    // Show a more complex example
    println!("=== Complex Example: Fibonacci ===");
    let fibonacci = r#"
sub fibonacci {
    my $n = shift;
    if ($n <= 1) {
        return $n;
    }
    
    my $prev = 0;
    my $curr = 1;
    
    for (my $i = 2; $i <= $n; $i++) {
        my $next = $prev + $curr;
        $prev = $curr;
        $curr = $next;
    }
    
    return $curr;
}

my $result = fibonacci(10);
print "Fibonacci(10) = $result\n";
"#;
    
    println!("Code:");
    println!("{}", fibonacci);
    
    let mut parser = Parser::new(fibonacci);
    
    match parser.parse() {
        Ok(ast) => {
            println!("\n✅ Successfully parsed!");
            println!("\nS-expression (truncated):");
            let sexp = ast.to_sexp();
            if sexp.len() > 200 {
                println!("{}...", &sexp[..200]);
            } else {
                println!("{}", sexp);
            }
        }
        Err(e) => {
            println!("\n❌ Parse error: {}", e);
        }
    }
    
    println!("\n=== Architecture Benefits ===");
    println!("• Clean separation of concerns");
    println!("• Independent testing of lexer and parser");
    println!("• Reusable components (perl-lexer can be used standalone)");
    println!("• Clear error boundaries");
    println!("• Optimal for benchmarking and optimization");
}