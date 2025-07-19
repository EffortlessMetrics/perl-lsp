use tree_sitter_perl::heredoc_parser::{HeredocScanner, parse_with_heredocs};

fn main() {
    let input = r#"my $single = <<'SINGLE';
No interpolation here: $var
SINGLE
my $double = <<"DOUBLE";
Interpolation works: $var
DOUBLE
my $backtick = <<`BACKTICK`;
echo "Command execution"
BACKTICK
print($single, $double, $backtick);"#;

    let mut scanner = HeredocScanner::new(input);
    let (output, declarations) = scanner.scan();
    
    println!("Scanner output:");
    println!("{}", output);
    println!("\nDeclarations: {:?}", declarations);
    
    // Also test the full parse_with_heredocs function
    println!("\n\nTesting parse_with_heredocs:");
    let (processed, decls) = parse_with_heredocs(input);
    println!("Processed output:\n{}", processed);
    println!("\nDeclarations from parse_with_heredocs: {:?}", decls);
}