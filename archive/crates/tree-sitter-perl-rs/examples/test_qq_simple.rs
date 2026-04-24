//! Test simple qq case
use tree_sitter_perl::EnhancedFullParser;

fn main() {
    let test_cases = [
        ("Simple pipe delimiter", r#"my $str = qq|Hello|;"#),
        ("Pipe with text", r#"my $str = qq|Hello World|;"#),
        ("Pipe with simple var", r#"my $str = qq|Hello $name|;"#),
        ("Pipe with hash element", r#"my $str = qq|Path: $ENV{PATH}|;"#),
    ];

    for (name, code) in test_cases {
        println!("Testing: {}", name);
        println!("Code: {}", code);

        let mut parser = EnhancedFullParser::new();
        match parser.parse(code) {
            Ok(_ast) => {
                println!("✓ Parsed successfully");
            }
            Err(e) => {
                println!("✗ Failed to parse: {}", e);
                println!("Enhanced parser error: {:?}", e);
            }
        }
        println!();
    }
}
