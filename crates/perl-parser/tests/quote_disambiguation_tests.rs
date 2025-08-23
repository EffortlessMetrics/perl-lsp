use perl_parser::{Parser, ast::NodeKind};

#[test]
fn test_qwerty_not_quote_operator() {
    let code = r#"
# This comment has qwerty in it
use constant qw(FOO BAR);
my $qwerty = 1;
my $x = FOO;
"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("should parse");

    // Find the use statement and verify it has the constants
    if let NodeKind::Program { statements } = &ast.kind {
        let mut found_use = false;
        let mut found_qwerty_var = false;

        for stmt in statements {
            // Check for use constant qw(FOO BAR)
            if let NodeKind::Use { module, args, .. } = &stmt.kind {
                if module == "constant" {
                    found_use = true;
                    // Should have captured FOO and BAR as arguments
                    assert!(
                        args.contains(&"qw(FOO BAR)".to_string()),
                        "Expected use constant to have qw(FOO BAR) argument, got {:?}",
                        args
                    );
                }
            }

            // Check for my $qwerty = 1
            if let NodeKind::VariableDeclaration { variable, initializer, .. } = &stmt.kind {
                if let NodeKind::Variable { sigil, name } = &variable.kind {
                    if sigil == "$" && name == "qwerty" {
                        found_qwerty_var = true;
                        assert!(initializer.is_some(), "Expected $qwerty to have an initializer");
                    }
                }
            }
        }

        assert!(found_use, "Failed to find use constant statement");
        assert!(found_qwerty_var, "Failed to find $qwerty variable declaration");
    } else {
        panic!("Expected Program node");
    }
}

#[test]
fn test_real_qw_operator() {
    let code = "my @list = qw(foo bar baz);";
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("should parse");

    // Verify that real qw() is parsed correctly
    if let NodeKind::Program { statements } = &ast.kind {
        if let Some(stmt) = statements.first() {
            if let NodeKind::VariableDeclaration { initializer, .. } = &stmt.kind {
                assert!(initializer.is_some(), "Expected initializer for qw() assignment");
                // The qw() should produce some kind of list/array
                return;
            }
        }
    }
    panic!("Failed to find expected structure");
}

#[test]
fn test_identifier_starting_with_q() {
    let code = r#"
my $query = "SELECT * FROM users";
my $quick = 42;
my $question = "What?";
"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("should parse");

    // All identifiers starting with 'q' should parse as regular identifiers
    if let NodeKind::Program { statements } = &ast.kind {
        let mut found_vars = Vec::new();

        for stmt in statements {
            if let NodeKind::VariableDeclaration { variable, .. } = &stmt.kind {
                if let NodeKind::Variable { name, .. } = &variable.kind {
                    found_vars.push(name.clone());
                }
            }
        }

        assert!(found_vars.contains(&"query".to_string()), "Expected to find $query");
        assert!(found_vars.contains(&"quick".to_string()), "Expected to find $quick");
        assert!(found_vars.contains(&"question".to_string()), "Expected to find $question");
    }
}
