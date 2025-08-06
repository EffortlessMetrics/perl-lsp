use perl_parser::completion::{CompletionProvider};
use perl_parser::parser::Parser;

fn main() {
    // Test simple completion
    let code = "pri";
    let mut parser = Parser::new("");
    let ast = parser.parse().unwrap();
    
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, 3);
    
    println!("Completions for 'pri':");
    for completion in &completions {
        println!("  - {}: {:?}", completion.label, completion.detail);
    }
    
    // Test with some actual code
    let code2 = r#"
my $count = 42;
my $counter = 0;

$cou"#;
    
    let mut parser2 = Parser::new(code2);
    let ast2 = parser2.parse().unwrap();
    
    let provider2 = CompletionProvider::new(&ast2);
    let completions2 = provider2.get_completions(code2, code2.len() - 1);
    
    println!("\nCompletions for '$cou':");
    for completion in &completions2 {
        println!("  - {}: {:?}", completion.label, completion.detail);
    }
}