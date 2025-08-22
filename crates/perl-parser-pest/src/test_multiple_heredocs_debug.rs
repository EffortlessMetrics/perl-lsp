use tree_sitter_perl::full_parser::FullPerlParser;

fn main() {
    // Minimal test case for multiple heredocs
    let input = r#"print(<<A, <<B);
First
A
Second
B
"#;

    eprintln!("Input:\n{}", input);
    eprintln!("======================");
    
    let mut parser = FullPerlParser::new();
    match parser.parse(input) {
        Ok(result) => {
            eprintln!("Parse succeeded!");
            eprintln!("S-expression:\n{}", result);
        }
        Err(e) => {
            eprintln!("Parse failed: {:?}", e);
        }
    }
}