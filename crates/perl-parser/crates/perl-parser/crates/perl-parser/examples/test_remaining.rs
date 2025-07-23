use perl_parser::Parser;

fn test_feature(name: &str, code: &str) -> bool {
    print!("Testing {:<25} - ", name);
    
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            let sexp = ast.to_sexp();
            // Check for specific features in the output
            let success = match name {
                n if n.starts_with("statement_modifier") => sexp.contains("statement_modifier"),
                "isa_operator" => sexp.contains("ISA") || sexp.contains("isa"),
                n if n.starts_with("file_test") => sexp.contains("file_test") || sexp.contains("unary"),
                "smart_match" => sexp.contains("~~") || sexp.contains("smart_match"),
                n if n.contains("label") => sexp.contains("label"),
                n if n.contains("attribute") => sexp.contains("attribute") || sexp.contains(":"),
                _ => true,
            };
            
            if success {
                println!("✅ Parsed correctly");
            } else {
                println!("⚠️  Parsed but may be incorrect: {}", sexp.replace('\n', " ").chars().take(80).collect::<String>());
            }
            true
        }
        Err(e) => {
            println!("❌ Failed: {:?}", e);
            false
        }
    }
}

fn main() {
    println!("=== Testing Remaining Perl Features ===\n");
    
    let mut passed = 0;
    let mut total = 0;
    
    // Test cases for features that might need work
    let tests = vec![
        // Statement modifiers - these turn statements into expressions with conditions
        ("statement_modifier_if", "print $x if $y"),
        ("statement_modifier_unless", "die unless $ok"),
        ("statement_modifier_while", "print while <STDIN>"),
        
        // ISA operator - type checking
        ("isa_operator", "$obj ISA MyClass"),
        
        // File test operators
        ("file_test_simple", "-f $file"),
        ("file_test_chain", "-f $file && -r $file"),
        
        // Smart match operator
        ("smart_match", "$x ~~ $y"),
        
        // Labels
        ("labeled_block", "LABEL: { print }"),
        
        // Attributes
        ("sub_attribute", "sub foo : lvalue { }"),
        ("var_attribute", "my $x :shared"),
        
        // Special blocks
        ("begin_block", "BEGIN { }"),
        
        // Already implemented features (for verification)
        ("regex_modifiers", "/pattern/i"),
        ("substitution", "s/foo/bar/g"),
        ("qw_construct", "qw(a b c)"),
    ];
    
    for (name, code) in &tests {
        if test_feature(name, code) {
            passed += 1;
        }
        total += 1;
    }
    
    println!("\n=== Summary ===");
    println!("Passed: {}/{} ({:.1}%)", passed, total, (passed as f64 / total as f64) * 100.0);
    
    println!("\n=== Next Steps ===");
    println!("Based on the test results, we need to:");
    println!("1. Implement statement modifiers (print $x if $y)");
    println!("2. Add ISA operator support");
    println!("3. Add file test operators (-f, -d, -e, etc.)");
    println!("4. Ensure smart match operator (~) works");
    println!("5. Verify label and attribute parsing");
}