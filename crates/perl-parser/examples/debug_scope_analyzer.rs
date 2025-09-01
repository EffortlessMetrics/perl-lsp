use perl_parser::{DiagnosticsProvider, Parser};

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
        (
            "bareword_hash_key",
            r#"use strict;
my %h = ();
my $x = $h{key};
print FOO;"#,
        ),
    ];

    for (name, code) in test_cases {
        println!("\n=== Test: {} ===", name);
        println!("Code:\n{}", code);

        // Parse the code first
        let mut parser = Parser::new(code);
        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(e) => {
                println!("Parse error: {:?}", e);
                continue;
            }
        };

        println!("AST: {}", ast.to_sexp());

        // Use DiagnosticsProvider which handles pragma detection properly
        let diagnostics_provider = DiagnosticsProvider::new(&ast, code.to_string());
        let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], code);

        println!("\nFound {} diagnostics:", diagnostics.len());
        for diagnostic in &diagnostics {
            println!(
                "  - {:?}: {} at range {:?}",
                diagnostic.code, diagnostic.message, diagnostic.range
            );
        }
    }
}
