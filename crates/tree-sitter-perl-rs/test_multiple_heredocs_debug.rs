use tree_sitter_perl::heredoc_parser::parse_with_heredocs;
use tree_sitter_perl::full_parser::FullPerlParser;

fn main() {
    let input = r#"print <<A, <<B, <<C;
First content
A
Second content
B
Third content
C"#;

    println!("=== Testing Multiple Heredocs ===");
    println!("Input:\n{}\n", input);
    
    // First test heredoc processing
    println!("=== Heredoc Processing ===");
    let (processed, declarations) = parse_with_heredocs(input);
    println!("Processed:\n{}", processed);
    println!("\nDeclarations found: {}", declarations.len());
    for (i, decl) in declarations.iter().enumerate() {
        println!("  [{}] terminator: '{}', content: {:?}", 
                 i, decl.terminator, decl.content.as_deref());
    }
    
    // Now test full parsing
    println!("\n=== Full Parser ===");
    let mut parser = FullPerlParser::new();
    match parser.parse(input) {
        Ok(ast) => println!("Parse succeeded!"),
        Err(e) => {
            println!("Parse failed: {:?}", e);
            
            // Show what the parser actually tried to parse
            use tree_sitter_perl::lexer_adapter::LexerAdapter;
            let fully_processed = LexerAdapter::preprocess(&processed);
            println!("\nFully processed input:\n{}", fully_processed);
        }
    }
}