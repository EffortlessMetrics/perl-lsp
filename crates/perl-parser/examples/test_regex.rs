//! Test regex match operators
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic regex match
        r#"$str =~ /pattern/"#,
        r#"$str !~ /pattern/"#,
        // Regex with modifiers
        r#"$str =~ /pattern/i"#,
        r#"$str =~ /pattern/gi"#,
        r#"$str =~ /pattern/msix"#,
        // Complex patterns
        r#"$str =~ /^\d+$/"#,
        r#"$str =~ /foo.*bar/"#,
        r#"$str =~ /a{2,4}/"#,
        // Substitution
        r#"$str =~ s/old/new/"#,
        r#"$str =~ s/old/new/g"#,
        r#"$str =~ s/old/new/gi"#,
        // Transliteration
        r#"$str =~ tr/a-z/A-Z/"#,
        r#"$str =~ y/a-z/A-Z/"#,
        // Match in conditional
        r#"if ($str =~ /pattern/) { }"#,
        r#"print if /pattern/"#,
    ];

    for test in tests {
        println!("\nTesting: {}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success!");
                println!("   S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}
