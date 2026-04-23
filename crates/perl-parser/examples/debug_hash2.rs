use perl_parser::{Parser, ast::NodeKind};

fn main() {
    let code = "{ key => 'value' }";
    println!("Testing: {}", code);

    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            println!("Top-level AST: {:?}", ast.kind);

            if let NodeKind::Program { statements } = &ast.kind {
                for (i, stmt) in statements.iter().enumerate() {
                    println!("Statement {}: {:?}", i, stmt.kind);

                    if let NodeKind::Block { statements: block_stmts } = &stmt.kind {
                        for (j, block_stmt) in block_stmts.iter().enumerate() {
                            println!("  Block statement {}: {:?}", j, block_stmt.kind);
                        }
                    }
                }
            }

            println!("S-expr: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
