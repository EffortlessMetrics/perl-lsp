//! Test substitution operators (s///, tr///, y///)
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic substitution
        "$str =~ s/old/new/",
        "$str =~ s/old/new/g",
        "$str =~ s/old/new/gi",
        "$str =~ s/old/new/gims",
        // With different delimiters
        "$str =~ s|old|new|",
        "$str =~ s{old}{new}",
        "$str =~ s[old][new]",
        // Transliteration
        "$str =~ tr/a-z/A-Z/",
        "$str =~ tr/abc/xyz/",
        "$str =~ y/a-z/A-Z/",
        "$str =~ y/abc/xyz/",
        // With options
        "$str =~ tr/a-z/A-Z/d",
        "$str =~ tr/a-z/A-Z/s",
        "$str =~ tr/a-z/A-Z/c",
        // In context
        "if ($str =~ s/foo/bar/) { }",
        "my $count = $str =~ s/x/y/g",
        // On $_
        "s/old/new/",
        "tr/a-z/A-Z/",
        // Complex patterns
        "$str =~ s/\\s+/ /g",
        "$str =~ s/^\\s+|\\s+$//g",
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
