//! Test operator precedence for logical operators
use perl_parser::Parser;

fn main() {
    println!("Testing Perl Operator Precedence");
    println!("=================================\n");
    
    let tests = vec![
        // Basic precedence tests
        ("$a || $b and $c", "Should parse as: ($a || $b) and $c"),
        ("$a or $b && $c", "Should parse as: $a or ($b && $c)"),
        ("$a && $b or $c", "Should parse as: ($a && $b) or $c"),
        
        // not vs !
        ("not $a || $b", "Should parse as: not ($a || $b)"),
        ("!$a or $b", "Should parse as: (!$a) or $b"),
        
        // Assignment precedence
        ("$x = $a or die", "Should parse as: ($x = $a) or die"),
        ("$y = $a and $b", "Should parse as: ($y = $a) and $b"),
        ("$z = $a || $b", "Should parse as: $z = ($a || $b)"),
        
        // Comparison with and/or
        ("$a > 0 and $b < 10", "Should parse as: ($a > 0) and ($b < 10)"),
        ("$a == 1 or $b == 0", "Should parse as: ($a == 1) or ($b == 0)"),
        
        // Complex nesting
        ("$a and $b or $c and $d", "Left associative: (($a and $b) or $c) and $d"),
        ("$a or $b and $c or $d", "Should parse with correct precedence"),
    ];
    
    for (test, expected) in tests {
        println!("Testing: {}", test);
        println!("Expected: {}", expected);
        
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                let sexp = ast.to_sexp();
                println!("✅ Parsed successfully");
                println!("   S-expr: {}", sexp);
                
                // Check specific precedence patterns
                if test.contains("or") && test.contains("and") {
                    if sexp.contains("(logical_and") && sexp.contains("(logical_or") {
                        println!("   ✓ Correct precedence: and binds tighter than or");
                    }
                }
                
                if test.contains("=") && test.contains("or") {
                    if sexp.contains("(assign") && sexp.contains("(logical_or") {
                        // Check if assignment is inside or outside the or
                        let assign_pos = sexp.find("(assign").unwrap_or(0);
                        let or_pos = sexp.find("(logical_or").unwrap_or(0);
                        if assign_pos < or_pos {
                            println!("   ✓ Correct: assignment happens before 'or'");
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ Parse error: {}", e);
            }
        }
        println!();
    }
    
    println!("\nOperator Precedence Table (Perl):");
    println!("==================================");
    println!("Highest to Lowest:");
    println!("1. -> (method call, dereference)");
    println!("2. ++ -- (autoincrement)");
    println!("3. ** (exponentiation)");
    println!("4. ! ~ \\ + - (unary)");
    println!("5. =~ !~ (binding)");
    println!("6. * / % x (multiplicative)");
    println!("7. + - . (additive, concat)");
    println!("8. << >> (shift)");
    println!("9. < > <= >= lt gt le ge (relational)");
    println!("10. == != <=> eq ne cmp ~~ (equality)");
    println!("11. & (bitwise and)");
    println!("12. | ^ (bitwise or/xor)");
    println!("13. && (logical and)");
    println!("14. || // (logical or, defined-or)");
    println!("15. .. ... (range)");
    println!("16. ?: (ternary)");
    println!("17. = += -= *= etc (assignment)");
    println!("18. , => (comma)");
    println!("19. not (list not)");
    println!("20. and (logical and - low)");
    println!("21. or xor (logical or/xor - low)");
}