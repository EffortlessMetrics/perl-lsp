use perl_parser::{ScopeAnalyzer, IssueKind, Parser};

fn main() {
    let analyzer = ScopeAnalyzer::new();
    
    // Simple test with strict mode
    let code = r#"use strict;
my $x = 1;
print $y;"#;

    println!("=== Testing undefined variable detection ===\n");
    println!("Code:\n{}\n", code);
    
    // First, parse the code to see what AST we get
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            println!("AST: {}\n", ast.to_sexp());
        }
        Err(e) => {
            println!("Parse error: {:?}\n", e);
        }
    }
    
    // Now analyze with scope analyzer
    // Need to parse first to get AST
    let mut parser2 = Parser::new(code);
    let ast = match parser2.parse() {
        Ok(ast) => ast,
        Err(e) => {
            println!("Failed to parse for analysis: {:?}", e);
            return;
        }
    };
    
    // Analyze with empty pragma map (will be inferred from code)
    let issues = analyzer.analyze(&ast, code, &[]);
    
    println!("Detected {} issues:", issues.len());
    for issue in &issues {
        println!("  - {:?} '{}' at line {}: {}", 
            issue.kind,
            issue.variable_name,
            issue.line,
            issue.description
        );
    }
    
    // Check if we found the undefined variable
    let has_undefined = issues.iter()
        .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable) && i.variable_name == "$y");
    
    if has_undefined {
        println!("\n✅ SUCCESS: Undefined variable $y was detected!");
    } else {
        println!("\n❌ FAILURE: Undefined variable $y was NOT detected!");
        println!("\nDebugging info:");
        println!("  - Code contains 'use strict': {}", code.contains("use strict"));
        println!("  - Total issues found: {}", issues.len());
    }
}