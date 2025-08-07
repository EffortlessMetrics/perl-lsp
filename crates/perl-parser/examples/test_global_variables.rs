use perl_parser::{ScopeAnalyzer, IssueKind};

fn main() {
    let analyzer = ScopeAnalyzer::new();
    
    // Test code with built-in globals (simplified for parser)
    let code = r#"
use strict;

# These should NOT trigger undeclared variable warnings
print $_;
print $@;
print @ARGV;

my $result = $1;

# This SHOULD trigger an undeclared variable warning  
print $undefined_var;
"#;

    println!("Testing built-in global variable recognition:\n");
    println!("Code:\n{}\n", code);
    
    // Parse to get AST
    let mut parser = perl_parser::Parser::new(code);
    let ast = parser.parse().expect("Failed to parse");
    println!("AST: {:?}\n", ast);
    
    let issues = analyzer.analyze(code);
    
    if issues.is_empty() {
        println!("✅ No issues found (all built-in globals recognized correctly)");
    } else {
        println!("Issues found:");
        for issue in &issues {
            println!("  - {} at line {}: {}", 
                match issue.kind {
                    IssueKind::UndeclaredVariable => "Undeclared",
                    IssueKind::UnusedVariable => "Unused",
                    IssueKind::VariableShadowing => "Shadowing",
                    IssueKind::VariableRedeclaration => "Redeclaration",
                },
                issue.line,
                issue.description
            );
        }
    }
    
    // Count undeclared variable issues
    let undeclared_count = issues.iter()
        .filter(|i| matches!(i.kind, IssueKind::UndeclaredVariable))
        .count();
    
    println!("\nSummary:");
    println!("  Total issues: {}", issues.len());
    println!("  Undeclared variables: {}", undeclared_count);
    println!("  Expected: 1 undeclared variable ($undefined_var)");
    
    if undeclared_count == 1 {
        println!("\n✅ Test PASSED: Built-in globals are correctly recognized!");
    } else {
        println!("\n❌ Test FAILED: Expected 1 undeclared variable, found {}", undeclared_count);
    }
}