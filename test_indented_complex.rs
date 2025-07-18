use tree_sitter_perl::heredoc_parser::parse_with_heredocs;
use tree_sitter_perl::full_parser::FullPerlParser;

fn main() {
    let input = r#"if ($condition) {
    my $config = <<~'CONFIG';
        server: localhost
        port: 8080
        debug: true
        CONFIG
    print $config;
}"#;

    println!("Testing complex indented heredoc:");
    println!("Input:\n{}", input);
    
    // First test just heredoc parsing
    let (processed, declarations) = parse_with_heredocs(input);
    
    println!("\nProcessed:\n{}", processed);
    println!("\nDeclarations: {:#?}", declarations);
    
    // Now test full parsing
    println!("\nTesting full parsing...");
    let mut parser = FullPerlParser::new();
    match parser.parse(input) {
        Ok(_) => println!("Success!"),
        Err(e) => println!("Error: {:?}", e),
    }
}