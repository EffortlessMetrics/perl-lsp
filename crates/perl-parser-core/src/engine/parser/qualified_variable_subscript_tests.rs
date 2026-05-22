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
        assert!(!sexp.contains("ERROR"), "Parse of `{}` produced ERROR nodes: {}", code, sexp);
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

    #[test]
    fn scalar_ref_hash_slice_preserves_base_target() {
        let code = "%$href{'a', 'b'};";
        assert_no_errors(code);

        let expr = first_expr(code);
        match &expr.kind {
            NodeKind::Binary { op, left, right } => {
                assert_eq!(op, "{}", "Expected {{}} hash-slice operator, got: {op}");
                match &left.kind {
                    NodeKind::Variable { sigil, name } => {
                        assert_eq!(sigil, "%", "Expected % sigil on hash-slice target");
                        assert_eq!(name, "$href", "Expected scalar-ref hash target, got: {name}");
                    }
                    _ => panic!(
                        "Expected Variable node as hash-slice target, got: {}",
                        left.kind.kind_name()
                    ),
                }
                match &right.kind {
                    NodeKind::ArrayLiteral { elements } => {
                        assert_eq!(elements.len(), 2, "Expected two hash-slice keys");
                    }
                    _ => panic!(
                        "Expected ArrayLiteral slice key list, got: {}",
                        right.kind.kind_name()
                    ),
                }
            }
            _ => panic!(
                "Expected Binary hash-slice node, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    #[test]
    fn scalar_ref_hash_slice_list_preserves_base_target() {
        let code = "@$href{'a', 'b'};";
        assert_no_errors(code);

        let expr = first_expr(code);
        match &expr.kind {
            NodeKind::Binary { op, left, right } => {
                assert_eq!(op, "{}", "Expected {{}} hash-slice operator, got: {op}");
                match &left.kind {
                    NodeKind::Variable { sigil, name } => {
                        assert_eq!(sigil, "@", "Expected @ sigil on hash-slice target");
                        assert_eq!(name, "$href", "Expected scalar-ref hash target, got: {name}");
                    }
                    _ => panic!(
                        "Expected Variable node as hash-slice target, got: {}",
                        left.kind.kind_name()
                    ),
                }
                match &right.kind {
                    NodeKind::ArrayLiteral { elements } => {
                        assert_eq!(elements.len(), 2, "Expected two hash-slice keys");
                    }
                    _ => panic!(
                        "Expected ArrayLiteral slice key list, got: {}",
                        right.kind.kind_name()
                    ),
                }
            }
            _ => panic!(
                "Expected Binary hash-slice node, got: {} (sexp: {})",
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

    // ---------------------------------------------------------------
    // Expression index on qualified array
    // Perl: $Pkg::Var[$i + 1]
    // ---------------------------------------------------------------
    #[test]
    fn qualified_expression_index() {
        let code = "$Pkg::Var[$i + 1];";
        assert_no_errors(code);

        let expr = first_expr(code);
        match &expr.kind {
            NodeKind::Binary { op, left, right } => {
                assert_eq!(op, "[]");
                match &left.kind {
                    NodeKind::Variable { sigil, name } => {
                        assert_eq!(sigil, "$");
                        assert_eq!(name, "Pkg::Var");
                    }
                    _ => panic!("Expected Variable node, got: {}", left.kind.kind_name()),
                }
                // The index should be a binary + expression
                match &right.kind {
                    NodeKind::Binary { op: inner_op, .. } => {
                        assert_eq!(inner_op, "+", "Expected + operator in index expression");
                    }
                    _ => panic!(
                        "Expected Binary + expression in index, got: {}",
                        right.kind.kind_name()
                    ),
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
    // Variable key in qualified hash subscript
    // Perl: $Pkg::Var{$key}
    // ---------------------------------------------------------------
    #[test]
    fn qualified_hash_variable_key() {
        let code = "$Pkg::Var{$key};";
        assert_no_errors(code);

        let expr = first_expr(code);
        match &expr.kind {
            NodeKind::Binary { op, left, right } => {
                assert_eq!(op, "{}");
                match &left.kind {
                    NodeKind::Variable { sigil, name } => {
                        assert_eq!(sigil, "$");
                        assert_eq!(name, "Pkg::Var");
                    }
                    _ => panic!("Expected Variable as target, got: {}", left.kind.kind_name()),
                }
                // The key should be a variable $key
                match &right.kind {
                    NodeKind::Variable { sigil, name } => {
                        assert_eq!(sigil, "$");
                        assert_eq!(name, "key");
                    }
                    _ => panic!("Expected Variable as key, got: {}", right.kind.kind_name()),
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
    // Subscript followed by arrow dereference
    // Perl: $Pkg::Var[0]->{key}
    // ---------------------------------------------------------------
    #[test]
    fn qualified_subscript_then_arrow_deref() {
        let code = "$Pkg::Var[0]->{key};";
        assert_no_errors(code);

        let expr = first_expr(code);
        // Outermost should be arrow hash deref: ->{}
        match &expr.kind {
            NodeKind::Binary { op, left, .. } => {
                assert_eq!(op, "->{}", "Expected ->{{}} arrow deref, got: {op}");
                // Left should be the [] subscript
                match &left.kind {
                    NodeKind::Binary { op: inner_op, left: inner_left, .. } => {
                        assert_eq!(inner_op, "[]");
                        match &inner_left.kind {
                            NodeKind::Variable { sigil, name } => {
                                assert_eq!(sigil, "$");
                                assert_eq!(name, "Pkg::Var");
                            }
                            _ => panic!("Expected Variable, got: {}", inner_left.kind.kind_name()),
                        }
                    }
                    _ => panic!("Expected Binary [], got: {}", left.kind.kind_name()),
                }
            }
            _ => panic!(
                "Expected Binary arrow deref, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Qualified subscript in arithmetic expression
    // Perl: $Pkg::Var[0] + $Pkg::Var{key}
    // ---------------------------------------------------------------
    #[test]
    fn qualified_subscript_in_arithmetic() {
        let code = "$Pkg::Var[0] + $Pkg::Var{key};";
        assert_no_errors(code);

        let expr = first_expr(code);
        match &expr.kind {
            NodeKind::Binary { op, left, right } => {
                assert_eq!(op, "+");
                // Left: $Pkg::Var[0]
                match &left.kind {
                    NodeKind::Binary { op: l_op, .. } => assert_eq!(l_op, "[]"),
                    _ => panic!("Expected Binary [] on left, got: {}", left.kind.kind_name()),
                }
                // Right: $Pkg::Var{key}
                match &right.kind {
                    NodeKind::Binary { op: r_op, .. } => assert_eq!(r_op, "{}"),
                    _ => panic!("Expected Binary {{}} on right, got: {}", right.kind.kind_name()),
                }
            }
            _ => panic!(
                "Expected Binary +, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            ),
        }
    }

    // ---------------------------------------------------------------
    // Postfix increment on qualified subscripted variable
    // Perl: $Pkg::Count{hits}++
    // ---------------------------------------------------------------
    #[test]
    fn qualified_subscript_postfix_increment() {
        let code = "$Pkg::Count{hits}++;";
        assert_no_errors(code);
    }

    // ---------------------------------------------------------------
    // Qualified subscript in conditional
    // Perl: if ($Config::opt{verbose}) { ... }
    // ---------------------------------------------------------------
    #[test]
    fn qualified_subscript_in_conditional() {
        let code = "if ($Config::opt{verbose}) { 1; }";
        assert_no_errors(code);
    }

    // ---------------------------------------------------------------
    // Deeply qualified with string key
    // Perl: $Config::Config{'osname'}
    // ---------------------------------------------------------------
    #[test]
    fn deeply_qualified_string_key() {
        let code = "$Config::Config{'osname'};";
        assert_no_errors(code);

        let expr = first_expr(code);
        match &expr.kind {
            NodeKind::Binary { op, left, .. } => {
                assert_eq!(op, "{}");
                match &left.kind {
                    NodeKind::Variable { sigil, name } => {
                        assert_eq!(sigil, "$");
                        assert_eq!(name, "Config::Config");
                    }
                    _ => panic!("Expected Variable, got: {}", left.kind.kind_name()),
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
    // Hex index on deeply qualified variable (real-world pattern)
    // Perl: $Text::Unidecode::Char[0xff] (verified structure)
    // ---------------------------------------------------------------
    #[test]
    fn deeply_qualified_hex_index_structure() {
        let code = "$Text::Unidecode::Char[0xff];";
        assert_no_errors(code);

        let expr = first_expr(code);
        match &expr.kind {
            NodeKind::Binary { op, left, right } => {
                assert_eq!(op, "[]");
                match &left.kind {
                    NodeKind::Variable { sigil, name } => {
                        assert_eq!(sigil, "$");
                        assert_eq!(name, "Text::Unidecode::Char");
                    }
                    _ => panic!("Expected Variable, got: {}", left.kind.kind_name()),
                }
                // Index should be a hex number
                match &right.kind {
                    NodeKind::Number { value } => {
                        assert_eq!(value, "0xff", "Expected hex literal 0xff, got: {value}");
                    }
                    _ => panic!("Expected Number, got: {}", right.kind.kind_name()),
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
    // Qualified subscript used as return value
    // Perl: return $Pkg::cache{$key};
    // ---------------------------------------------------------------
    #[test]
    fn qualified_subscript_in_return() {
        let code = "return $Pkg::cache{$key};";
        assert_no_errors(code);
    }

    // ---------------------------------------------------------------
    // Multiple qualified subscripts in a list
    // Perl: ($Pkg::a[0], $Pkg::b{x})
    // ---------------------------------------------------------------
    #[test]
    fn qualified_subscripts_in_list() {
        let code = "my @list = ($Pkg::a[0], $Pkg::b{x});";
        assert_no_errors(code);
    }
}
