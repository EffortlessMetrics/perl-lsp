//! Test diamond operator
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic diamond operator
        "<>",
        "while (<>) { }",
        "my $line = <>",
        "$_ = <>",
        
        // Filehandle readline
        "<STDIN>",
        "<STDOUT>",
        "<STDERR>",
        "<DATA>",
        "my $input = <STDIN>",
        
        // In different contexts
        "print while <>",
        "chomp(my $line = <>)",
        "for (<>) { print }",
        
        // Glob patterns (different from readline)
        "glob('*.txt')",
        "@files = <*.txt>",
    ];

    for test in tests {
        print!("Testing: {:40} ", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}