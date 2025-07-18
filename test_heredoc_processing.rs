use tree_sitter_perl::heredoc_parser::parse_with_heredocs;

fn main() {
    let input = r#"my $x = <<'EOF';
Hello / World
EOF
print $x;"#;

    let (processed, declarations) = parse_with_heredocs(input);
    
    println!("Original input:\n{}", input);
    println!("\nProcessed output:\n{}", processed);
    println!("\nDeclarations: {:#?}", declarations);
}