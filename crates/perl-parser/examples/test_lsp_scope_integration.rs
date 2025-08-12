use perl_parser::Parser;
use perl_parser::diagnostics::DiagnosticsProvider;

fn main() {
    let test_cases = vec![
        (
            "shadowing",
            r#"
use strict;
my $x = 10;
{
    my $x = 20;  # Should warn about shadowing
    print $x;
}"#,
        ),
        (
            "unused",
            r#"
use strict;
my $unused = 42;  # Should warn about being unused
my $used = 10;
print $used;"#,
        ),
        (
            "undeclared",
            r#"
use strict;
my $declared = 10;
print $undeclared;  # Should error about being undeclared"#,
        ),
        (
            "redeclaration",
            r#"
use strict;
my $x = 10;
$x = 20;
my $x = 30;  # Should error about redeclaration
print $x;"#,
        ),
    ];

    for (name, code) in test_cases {
        println!("\n=== Test: {} ===", name);
        println!("Code:{}", code);

        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                let provider = DiagnosticsProvider::new(&ast, code.to_string());
                let diagnostics = provider.get_diagnostics(&ast, &[], code);

                println!("\nDiagnostics found: {}", diagnostics.len());
                for diag in &diagnostics {
                    println!(
                        "  [{:?}] {} - {}",
                        diag.severity,
                        diag.code.as_ref().unwrap_or(&String::new()),
                        diag.message
                    );
                }
            }
            Err(e) => {
                println!("Parse error: {}", e);
            }
        }
    }

    println!("\n✅ LSP Scope Integration Test Complete!");
}
