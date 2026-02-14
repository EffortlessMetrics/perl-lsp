use perl_parser::{Parser, ast::NodeKind};

#[test]
fn debug_substitution_parsing() {
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

                            assert_eq!(pattern, "old");
                            assert_eq!(replacement, "new");
                            assert_eq!(modifiers, "");
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
            use perl_tdd_support::must;
            must(Err::<(), _>(e));
        }
    }
}

#[test]
fn debug_substitution_with_flags() {
    let code = "s/old/new/gi";
    println!("Testing: {}", code);

    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            println!("Successfully parsed!");

            if let NodeKind::Program { statements } = &ast.kind {
                for stmt in statements {
                    match &stmt.kind {
                        NodeKind::Substitution { pattern, replacement, modifiers, .. } => {
                            println!("Found substitution:");
                            println!("  Pattern: {}", pattern);
                            println!("  Replacement: {}", replacement);
                            println!("  Modifiers: {}", modifiers);

                            assert_eq!(pattern, "old");
                            assert_eq!(replacement, "new");
                            assert_eq!(modifiers, "gi");
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
            use perl_tdd_support::must;
            must(Err::<(), _>(e));
        }
    }
}
