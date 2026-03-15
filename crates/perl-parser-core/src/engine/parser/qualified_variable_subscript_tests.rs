#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use crate::parser::Parser;
    use perl_ast::ast::{Node, NodeKind};

    /// Helper: parse code and return the full AST.
    fn parse_program(code: &str) -> Node {
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => ast,
            Err(e) => panic!("Parse failed for `{code}`: {e:?}"),
        }
    }

    /// Helper: check that the AST sexp contains no ERROR nodes.
    fn assert_no_errors(code: &str) {
        let ast = parse_program(code);
        let sexp = ast.to_sexp();
        assert!(!sexp.contains("ERROR"), "Parse of `{}` produced ERROR nodes: {}", code, sexp,);
    }

    /// Helper: parse code and return the first statement's expression node.
    fn first_expr(code: &str) -> Node {
        let ast = parse_program(code);
        match ast.kind {
            NodeKind::Program { mut statements } if !statements.is_empty() => {
                let stmt = statements.swap_remove(0);
                match stmt.kind {
                    NodeKind::ExpressionStatement { expression } => *expression,
                    other => panic!("Expected ExpressionStatement, got: {}", other.kind_name()),
                }
            }
            _ => panic!("Expected Program with statements, got: {}", ast.to_sexp()),
        }
    }

    // ---------------------------------------------------------------
    // Array subscript on package-qualified scalar variable
    // Perl: $Pkg::Var[0]
    // ---------------------------------------------------------------
    #[test]
    fn qualified_scalar_array_subscript() {
        let code = "$Pkg::Var[0];";
        assert_no_errors(code);

        let expr = first_expr(code);
        // Should be Binary { op: "[]", left: Variable($, "Pkg::Var"), right: Number(0) }
        match &expr.kind {
            NodeKind::Binary { op, left, .. } => {
                assert_eq!(op, "[]", "Expected [] subscript operator, got: {op}");
                match &left.kind {
                    NodeKind::Variable { sigil, name } => {
                        assert_eq!(sigil, "$", "Expected $ sigil");
                        assert_eq!(
                            name, "Pkg::Var",
                            "Expected qualified name Pkg::Var, got: {name}"
                        );
                    }
                    _ => panic!(
                        "Expected Variable node as subscript target, got: {}",
                        left.kind.kind_name()
                    ),
                }
            }
            _ => panic!(
                "Expected Binary subscript node, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Hash subscript on package-qualified scalar variable
    // Perl: $Pkg::Var{key}
    // ---------------------------------------------------------------
    #[test]
    fn qualified_scalar_hash_subscript() {
        let code = "$Pkg::Var{key};";
        assert_no_errors(code);

        let expr = first_expr(code);
        // Should be Binary { op: "{}", left: Variable($, "Pkg::Var"), right: ... }
        match &expr.kind {
            NodeKind::Binary { op, left, .. } => {
                assert_eq!(op, "{}", "Expected {{}} subscript operator, got: {op}");
                match &left.kind {
                    NodeKind::Variable { sigil, name } => {
                        assert_eq!(sigil, "$", "Expected $ sigil");
                        assert_eq!(
                            name, "Pkg::Var",
                            "Expected qualified name Pkg::Var, got: {name}"
                        );
                    }
                    _ => panic!(
                        "Expected Variable node as subscript target, got: {}",
                        left.kind.kind_name()
                    ),
                }
            }
            _ => panic!(
                "Expected Binary subscript node, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Array slice on package-qualified array variable
    // Perl: @Pkg::Arr[0..5]
    // ---------------------------------------------------------------
    #[test]
    fn qualified_array_slice() {
        let code = "@Pkg::Arr[0..5];";
        assert_no_errors(code);

        let expr = first_expr(code);
        // Should be Binary { op: "[]", left: Variable(@, "Pkg::Arr"), right: range }
        match &expr.kind {
            NodeKind::Binary { op, left, .. } => {
                assert_eq!(op, "[]", "Expected [] subscript operator, got: {op}");
                match &left.kind {
                    NodeKind::Variable { sigil, name } => {
                        assert_eq!(sigil, "@", "Expected @ sigil");
                        assert_eq!(
                            name, "Pkg::Arr",
                            "Expected qualified name Pkg::Arr, got: {name}"
                        );
                    }
                    _ => panic!(
                        "Expected Variable node as subscript target, got: {}",
                        left.kind.kind_name()
                    ),
                }
            }
            _ => panic!(
                "Expected Binary subscript node, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Hash slice on package-qualified hash variable
    // Perl: %Pkg::Hash{qw(a b)}
    // ---------------------------------------------------------------
    #[test]
    fn qualified_hash_slice() {
        // Note: %hash{} is a hash slice, which returns a list of values
        let code = "%Pkg::Hash{qw(a b)};";
        assert_no_errors(code);

        let expr = first_expr(code);
        // Should be Binary { op: "{}", left: Variable(%, "Pkg::Hash"), right: ... }
        match &expr.kind {
            NodeKind::Binary { op, left, .. } => {
                assert_eq!(op, "{}", "Expected {{}} subscript operator, got: {op}");
                match &left.kind {
                    NodeKind::Variable { sigil, name } => {
                        assert_eq!(sigil, "%", "Expected % sigil");
                        assert_eq!(
                            name, "Pkg::Hash",
                            "Expected qualified name Pkg::Hash, got: {name}"
                        );
                    }
                    _ => panic!(
                        "Expected Variable node as subscript target, got: {}",
                        left.kind.kind_name()
                    ),
                }
            }
            _ => panic!(
                "Expected Binary subscript node, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Multi-level qualified variable with hex subscript
    // Perl: $Text::Unidecode::Char[0xff]
    // ---------------------------------------------------------------
    #[test]
    fn deeply_qualified_variable_subscript() {
        let code = "$Text::Unidecode::Char[0xff];";
        assert_no_errors(code);

        let expr = first_expr(code);
        match &expr.kind {
            NodeKind::Binary { op, left, .. } => {
                assert_eq!(op, "[]", "Expected [] subscript operator, got: {op}");
                match &left.kind {
                    NodeKind::Variable { sigil, name } => {
                        assert_eq!(sigil, "$");
                        assert_eq!(name, "Text::Unidecode::Char");
                    }
                    _ => panic!("Expected Variable node, got: {}", left.kind.kind_name()),
                }
            }
            _ => panic!(
                "Expected Binary subscript, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Chained subscripts on qualified variable
    // Perl: $Pkg::Var{key}[0]
    // ---------------------------------------------------------------
    #[test]
    fn qualified_variable_chained_subscripts() {
        let code = "$Pkg::Var{key}[0];";
        assert_no_errors(code);

        let expr = first_expr(code);
        // Should be Binary { op: "[]", left: Binary { op: "{}", ... }, right: Number(0) }
        match &expr.kind {
            NodeKind::Binary { op, left, .. } => {
                assert_eq!(op, "[]", "Expected outer [] subscript");
                match &left.kind {
                    NodeKind::Binary { op: inner_op, left: inner_left, .. } => {
                        assert_eq!(inner_op, "{}", "Expected inner {{}} subscript");
                        match &inner_left.kind {
                            NodeKind::Variable { sigil, name } => {
                                assert_eq!(sigil, "$");
                                assert_eq!(name, "Pkg::Var");
                            }
                            _ => panic!(
                                "Expected Variable node, got: {}",
                                inner_left.kind.kind_name()
                            ),
                        }
                    }
                    _ => panic!("Expected inner Binary subscript, got: {}", left.kind.kind_name()),
                }
            }
            _ => panic!(
                "Expected Binary subscript, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Qualified variable subscript in assignment context
    // Perl: $Config::Default{path} = "/usr/bin";
    // ---------------------------------------------------------------
    #[test]
    fn qualified_subscript_in_assignment() {
        let code = "$Config::Default{path} = \"/usr/bin\";";
        assert_no_errors(code);
    }

    // ---------------------------------------------------------------
    // Qualified variable subscript as function argument
    // Perl: print $Pkg::Data[0];
    // ---------------------------------------------------------------
    #[test]
    fn qualified_subscript_as_arg() {
        let code = "print $Pkg::Data[0];";
        assert_no_errors(code);
    }

    // ---------------------------------------------------------------
    // Negative index on qualified array
    // Perl: $Pkg::List[-1]
    // ---------------------------------------------------------------
    #[test]
    fn qualified_negative_index() {
        let code = "$Pkg::List[-1];";
        assert_no_errors(code);

        let expr = first_expr(code);
        match &expr.kind {
            NodeKind::Binary { op, left, .. } => {
                assert_eq!(op, "[]");
                match &left.kind {
                    NodeKind::Variable { sigil, name } => {
                        assert_eq!(sigil, "$");
                        assert_eq!(name, "Pkg::List");
                    }
                    _ => panic!("Expected Variable node, got: {}", left.kind.kind_name()),
                }
            }
            _ => panic!(
                "Expected Binary subscript, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }
}
