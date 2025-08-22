use tree_sitter_perl;

fn main() {
    let source = "1 + 1;";
    let expected = "(source_file
  (expression_statement
    (binary_expression
      (number)
      (number))))";
    
    match tree_sitter_perl::parse(source) {
        Ok(tree) => {
            let actual = tree.root_node().to_sexp();
            println!("Expected:");
            println!("{}", expected);
            println!("\nActual:");
            println!("{}", actual);
            println!("\nMatch: {}", actual == expected);
        }
        Err(e) => {
            println!("Parse error: {}", e);
        }
    }
} 