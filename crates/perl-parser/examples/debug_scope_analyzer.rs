use perl_parser::{Parser, scope_analyzer::ScopeAnalyzer};

fn main() {
    let test_cases = vec![
        (
            "shadowing",
            r#"
my $x = 10;
{
    my $x = 20;  # This shadows the outer $x
    print $x;
}
"#,
        ),
        (
            "undeclared",
            r#"
use strict;
my $declared = 10;
print $undeclared;  # This is not declared
"#,
        ),
        (
            "package",
            r#"
package MyPackage;
our $package_var = 10;
my $lexical_var = 20;

sub get_package { return $package_var; }
sub get_lexical { return $lexical_var; }
"#,
        ),
    ];

    for (name, code) in test_cases {
        println!("\n=== Test: {} ===", name);
        println!("Code:\n{}", code);

        let analyzer = ScopeAnalyzer::new();

        // Parse the code first
        let mut parser = Parser::new(code);
        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(e) => {
                println!("Parse error: {:?}", e);
                continue;
            }
        };

        let issues = analyzer.analyze(&ast, code, &[]);

        println!("\nFound {} issues:", issues.len());
        for issue in &issues {
            println!(
                "  - {:?}: {} (line {})",
                issue.kind, issue.description, issue.line
            );
        }
    }
}
