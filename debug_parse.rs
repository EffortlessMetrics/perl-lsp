use tree_sitter_perl::{parse, language};
use tree_sitter::Parser;

fn main() {
    let source = "# this is a comment\n1 + 1;";
    println!("Parsing source: {:?}", source);
    
    let mut parser = Parser::new();
    parser.set_language(&language()).unwrap();
    
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();
    
    println!("Root node: {}", root.kind());
    println!("Has error: {}", root.has_error());
    println!("Tree S-expression:");
    println!("{}", root.to_sexp());
    
    // Walk the tree and look for error nodes
    let mut cursor = root.walk();
    let mut error_count = 0;
    
    loop {
        let node = cursor.node();
        if node.kind() == "ERROR" {
            error_count += 1;
            println!("ERROR node at {}:{} - {}", 
                node.start_position().row, 
                node.start_position().column,
                node.to_sexp());
        }
        
        if cursor.goto_first_child() {
            continue;
        }
        if cursor.goto_next_sibling() {
            continue;
        }
        
        loop {
            if cursor.goto_parent() {
                if cursor.goto_next_sibling() {
                    break;
                }
            } else {
                break;
            }
        }
        
        if cursor.node().id() == root.id() {
            break;
        }
    }
    
    println!("Total ERROR nodes found: {}", error_count);
} 