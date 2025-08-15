use perl_parser::Parser;

fn main() {
    // Test format declarations
    let test_cases = vec![
        // Basic format
        (
            r#"format STDOUT =
@<<<<< @||||| @>>>>>
$name, $age, $score
.
"#,
            "basic format declaration",
        ),
        // AUTOLOAD and DESTROY blocks
        ("AUTOLOAD { print $AUTOLOAD }", "AUTOLOAD block"),
        ("DESTROY { cleanup() }", "DESTROY block"),
        // Subroutine attributes
        ("sub foo : lvalue { }", "sub with lvalue attribute"),
        ("sub bar : method { }", "sub with method attribute"),
        // Variable attributes
        ("my $x :shared;", "variable with attribute"),
        // Labels
        ("LABEL: for (@list) { last LABEL; }", "labeled loop"),
        // Default in given/when
        ("given ($x) { when (1) { } default { } }", "given/when with default"),
        // Class and method (Perl 5.38+)
        ("class Foo { }", "class declaration"),
        ("method bar { }", "method declaration"),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for (code, desc) in test_cases {
        print!("Testing {}: ", desc);
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(_) => {
                println!("✅ PASS");
                passed += 1;
            }
            Err(e) => {
                println!("❌ FAIL - {}", e);
                failed += 1;
            }
        }
    }

    println!("\nSummary: {} passed, {} failed", passed, failed);
}
