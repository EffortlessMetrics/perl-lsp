use tree_sitter_perl::full_parser::FullPerlParser;
use tree_sitter_perl::pure_rust_parser::PureRustPerlParser;

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

    // First try with the pure rust parser to see where it fails
    println!("Testing with PureRustPerlParser:");
    let mut pure_parser = PureRustPerlParser::new();
    match pure_parser.parse(input) {
        Ok(ast) => {
            println!("Pure parser succeeded!");
            println!("AST: {:?}", ast);
        }
        Err(e) => {
            println!("Pure parser failed: {:?}", e);
        }
    }
    
    println!("\nTesting with FullPerlParser:");
    let mut parser = FullPerlParser::new();
    match parser.parse(input) {
        Ok(ast) => {
            println!("Full parser succeeded!");
            println!("AST: {:?}", ast);
        }
        Err(e) => {
            println!("Full parser failed: {:?}", e);
        }
    }
}