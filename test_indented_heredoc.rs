use tree_sitter_perl::heredoc_parser::parse_with_heredocs;

fn main() {
    let input = r#"my $x = <<~'EOF';
        Hello
        EOF"#;

    println!("Testing indented heredoc:");
    println!("Input:\n{}", input);
    
    let (processed, declarations) = parse_with_heredocs(input);
    
    println!("\nProcessed:\n{}", processed);
    println!("\nDeclarations: {:#?}", declarations);
}