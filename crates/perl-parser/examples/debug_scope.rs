use perl_parser::scope_analyzer::ScopeAnalyzer;
use perl_parser::Parser;

fn main() {
    let codes = vec![
        ("simple", "my $x = 10; print $x;"),
        ("unused", "my $x = 10;"),
        ("undeclared_no_strict", "$x = 10;"),
        ("undeclared_strict", "use strict; $x = 10;"),
        ("nested", r#"
            my $outer = 1;
            {
                my $middle = 2;
                {
                    my $inner = 3;
                    print $outer, $middle, $inner;
                }
                print $middle;
            }
        "#),
        ("package", r#"
            our $pkg_var = 10;
            print $pkg_var;
        "#),
    ];
    
    for (name, code) in codes {
        println!("\n=== {} ===", name);
        println!("Code: {}", code);
        
        // First parse to see the AST
        let mut parser = Parser::new(code);
        let ast = match parser.parse() {
            Ok(ast) => {
                println!("AST: {}", ast.to_sexp());
                ast
            }
            Err(e) => {
                println!("Parse error: {}", e);
                continue;
            }
        };
        
        // Now analyze scoping
        let analyzer = ScopeAnalyzer::new();
        let issues = analyzer.analyze(&ast, code, &[]);
        
        println!("Issues found: {}", issues.len());
        for issue in &issues {
            println!("  {:?}: {} at line {}", issue.kind, issue.variable_name, issue.line);
        }
    }
}