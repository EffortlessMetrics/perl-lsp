//! Simple test of the lightweight parser

use tree_sitter_perl::minimal_parser::MinimalParser;

fn main() {
    let source = r#"
my $x = 42;
print "Hello, world!";
    "#;

    let ast = MinimalParser::parse(source);

    println!("Parse succeeded!");
    println!("AST: {:#?}", ast);
    println!("\nS-expression:\n{}", ast.to_sexp());
}
