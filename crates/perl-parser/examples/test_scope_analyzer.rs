use perl_parser::{Parser, scope_analyzer::ScopeAnalyzer};

fn main() {
    let analyzer = ScopeAnalyzer::new();
    
    // Test 1: Undeclared variable
    println!("Test 1: Undeclared variable");
    let code = r#"
        use strict;
        my $declared = 10;
        print $undeclared;  # This is not declared
    "#;
    
    // Parse the code first
    let mut parser = Parser::new(code);
    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(e) => {
            println!("Parse error: {}", e);
            return;
        }
    };
    
    // Analyze with AST and empty pragma map
    let issues = analyzer.analyze(&ast, code, &[]);
    println!("Found {} issues:", issues.len());
    for issue in &issues {
        println!("  - {:?} '{}' at line {}: {}", 
            issue.kind, issue.variable_name, issue.line, issue.description);
    }
    println!("Expected: 1 issue of type UndeclaredVariable");
    
    // Test 2: Multiple scope levels
    println!("\nTest 2: Multiple scope levels");
    let code = r#"
        my $x = 10;
        {
            my $y = 20;
            {
                my $z = $x + $y;  # Both should be accessible
                print $z;
            }
            print $y;  # $y accessible here
        }
        print $x;  # Only $x accessible here
    "#;
    
    let mut parser = Parser::new(code);
    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(e) => {
            println!("Parse error: {}", e);
            return;
        }
    };
    let issues = analyzer.analyze(&ast, code, &[]);
    println!("Found {} issues:", issues.len());
    for issue in &issues {
        println!("  - {:?} '{}' at line {}: {}", 
            issue.kind, issue.variable_name, issue.line, issue.description);
    }
    println!("Expected: 0 issues (all variables are used)");
    
    // Test 3: Package variables
    println!("\nTest 3: Package variables");
    let code = r#"
        package MyPackage;
        our $package_var = 10;
        my $lexical_var = 20;
        
        sub get_package { return $package_var; }
        sub get_lexical { return $lexical_var; }
    "#;
    
    let mut parser = Parser::new(code);
    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(e) => {
            println!("Parse error: {}", e);
            return;
        }
    };
    let issues = analyzer.analyze(&ast, code, &[]);
    println!("Found {} issues:", issues.len());
    for issue in &issues {
        println!("  - {:?} '{}' at line {}: {}", 
            issue.kind, issue.variable_name, issue.line, issue.description);
    }
    println!("Expected: 0 issues (both variables are used)");
}