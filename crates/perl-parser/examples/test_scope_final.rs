use perl_parser::scope_analyzer::{ScopeAnalyzer, IssueKind};

fn main() {
    let tests = vec![
        ("shadowing", r#"
            my $x = 10;
            {
                my $x = 20;  # This shadows the outer $x
                print $x;
            }
        "#, 1, IssueKind::VariableShadowing),
        
        ("unused", r#"
            my $unused = 42;
            my $used = 10;
            print $used;
        "#, 1, IssueKind::UnusedVariable),
        
        ("undeclared", r#"
            use strict;
            my $declared = 10;
            print $undeclared;  # This is not declared
        "#, 1, IssueKind::UndeclaredVariable),
        
        ("multiple_scopes", r#"
            my $outer = 1;
            {
                my $middle = 2;
                {
                    my $inner = 3;
                    print $outer, $middle, $inner;
                }
                print $middle;  # $inner not accessible here
            }
        "#, 0, IssueKind::UnusedVariable), // Should be 0 issues
        
        ("for_loop", r#"
            for my $i (1..10) {
                print $i;
            }
            # $i should not be accessible here
        "#, 0, IssueKind::UnusedVariable),
        
        ("subroutine", r#"
            sub process {
                my ($a, $b) = @_;
                return $a + $b;
            }
        "#, 0, IssueKind::UnusedVariable), // Parameters are used
        
        ("package_vars", r#"
            package MyPackage;
            our $package_var = 10;
            my $lexical_var = 20;
            
            sub get_package { return $package_var; }
            sub get_lexical { return $lexical_var; }
        "#, 0, IssueKind::UnusedVariable), // Both are used
        
        ("redeclaration", r#"
            my $x = 10;
            $x = 20;  # Reassignment
            my $x = 30;  # Redeclaration in same scope - issue!
            print $x;
        "#, 1, IssueKind::VariableRedeclaration),
        
        ("closure", r#"
            my $captured = 10;
            my $sub = sub {
                return $captured * 2;  # Captures outer variable
            };
        "#, 0, IssueKind::UnusedVariable), // $captured is used in closure
    ];
    
    let mut passed = 0;
    let mut failed = 0;
    
    for (name, code, expected_count, expected_kind) in tests {
        print!("Testing {}: ", name);
        
        let analyzer = ScopeAnalyzer::new();
        let issues = analyzer.analyze(code);
        
        let relevant_issues: Vec<_> = issues.iter()
            .filter(|i| i.kind == expected_kind)
            .collect();
        
        if relevant_issues.len() == expected_count {
            println!("✅ PASS");
            passed += 1;
        } else {
            println!("❌ FAIL - Expected {} {:?} issues, got {}", 
                     expected_count, expected_kind, relevant_issues.len());
            for issue in &issues {
                println!("    {:?}: {} at line {}", 
                         issue.kind, issue.variable_name, issue.line);
            }
            failed += 1;
        }
    }
    
    println!("\n=== Test Summary ===");
    println!("Passed: {}/{}", passed, passed + failed);
    println!("Failed: {}/{}", failed, passed + failed);
    
    if failed > 0 {
        std::process::exit(1);
    }
}