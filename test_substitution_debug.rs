use perl_parser::{Parser, ast::NodeKind};

fn main() {
    let code = "s/old/new/";
    println!("Testing: {}", code);

    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            println!("Successfully parsed!");
            println!("AST: {:?}", ast);

            if let NodeKind::Program { statements } = &ast.kind {
                for stmt in statements {
                    match &stmt.kind {
                        NodeKind::Substitution { pattern, replacement, modifiers, .. } => {
                            println!("Found substitution:");
                            println!("  Pattern: {}", pattern);
                            println!("  Replacement: {}", replacement);
                            println!("  Modifiers: {}", modifiers);
                        }
                        _ => {
                            println!("Other node: {:?}", stmt.kind);
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}