use perl_parser::{Parser, ast::NodeKind};

fn main() {
    let code = "{}";
    println!("Testing: {}", code);

    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            println!("AST: {:?}", ast);
            println!("S-expr: {}", ast.to_sexp());

            // Check what kind of node it really is
            if let NodeKind::Program { statements } = &ast.kind {
                if let Some(stmt) = statements.first() {
                    match &stmt.kind {
                        NodeKind::HashLiteral { pairs } => {
                            println!("It's a HashLiteral with {} pairs", pairs.len());
                        }
                        NodeKind::Block { statements } => {
                            println!("It's a Block with {} statements", statements.len());
                        }
                        _ => {
                            println!("It's something else: {:?}", stmt.kind);
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
