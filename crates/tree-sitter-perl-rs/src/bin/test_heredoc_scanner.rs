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
    let (output, declarations, skip_lines) = scanner.scan();
    
    println!("Scanner output:");
    println!("{}", output);
    println!("\nDeclarations: {:?}", declarations);
    println!("\nSkip lines: {:?}", skip_lines);
    
    // Also test the full parse_with_heredocs function
    println!("\n\nTesting parse_with_heredocs:");
    match parse_with_heredocs(input) {
        Ok((processed, map)) => {
            println!("Processed output:\n{}", processed);
            println!("\nPlaceholder map:");
            for (k, v) in map {
                println!("  {} => {}", k, v);
            }
        }
        Err(e) => {
            println!("Parse failed: {}", e);
        }
    }
}