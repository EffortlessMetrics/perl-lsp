use tree_sitter_perl::heredoc_parser::parse_with_heredocs;

fn main() {
    let input = r#"my %config = (
    name => "Test",
    description => <<'DESC'
);
This is a long description
that spans multiple lines
DESC
print $config{description};"#;

    println!("Original input:");
    println!("{}", input);
    println!("\n{}", "=".repeat(50));
    
    let (processed, declarations) = parse_with_heredocs(input);
    
    println!("\nProcessed output:");
    println!("{}", processed);
    
    println!("\nDeclarations:");
    for (i, decl) in declarations.iter().enumerate() {
        println!("  [{}] terminator: {}, line: {}, content: {:?}", 
                 i, decl.terminator, decl.declaration_line, decl.content);
    }
}