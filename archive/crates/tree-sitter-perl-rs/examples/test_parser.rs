//! Simple test of the lightweight parser

#[cfg(not(feature = "pure-rust-standalone"))]
use tree_sitter_perl::minimal_parser::MinimalParser;

#[cfg(feature = "pure-rust-standalone")]
fn main() {
    eprintln!("'test_parser' example is disabled with the 'pure-rust-standalone' feature.");
}

#[cfg(not(feature = "pure-rust-standalone"))]
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
