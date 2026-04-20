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
        let ast_sexp = ast.to_sexp();
        let NodeKind::Program { mut statements } = ast.kind else {
            panic!("Expected Program with statements, got: {}", ast_sexp)
        };
        if statements.is_empty() {
            panic!("Expected Program with statements, got: {}", ast_sexp)
        };
        let stmt = statements.swap_remove(0);
        let NodeKind::ExpressionStatement { expression } = stmt.kind else {
            panic!("Expected ExpressionStatement, got: {}", stmt.kind.kind_name())
        };
        *expression
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
        let NodeKind::Binary { op, left, .. } = &expr.kind else {
            panic!(
                "Expected Binary subscript node, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(op, "[]", "Expected [] subscript operator, got: {op}");
        let NodeKind::Variable { sigil, name } = &left.kind else {
            panic!("Expected Variable node as subscript target, got: {}", left.kind.kind_name())
        };
        assert_eq!(sigil, "$", "Expected $ sigil");
        assert_eq!(name, "Pkg::Var", "Expected qualified name Pkg::Var, got: {name}");
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
        let NodeKind::Binary { op, left, .. } = &expr.kind else {
            panic!(
                "Expected Binary subscript node, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(op, "{}", "Expected {{}} subscript operator, got: {op}");
        let NodeKind::Variable { sigil, name } = &left.kind else {
            panic!("Expected Variable node as subscript target, got: {}", left.kind.kind_name())
        };
        assert_eq!(sigil, "$", "Expected $ sigil");
        assert_eq!(name, "Pkg::Var", "Expected qualified name Pkg::Var, got: {name}");
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
        let NodeKind::Binary { op, left, .. } = &expr.kind else {
            panic!(
                "Expected Binary subscript node, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(op, "[]", "Expected [] subscript operator, got: {op}");
        let NodeKind::Variable { sigil, name } = &left.kind else {
            panic!("Expected Variable node as subscript target, got: {}", left.kind.kind_name())
        };
        assert_eq!(sigil, "@", "Expected @ sigil");
        assert_eq!(name, "Pkg::Arr", "Expected qualified name Pkg::Arr, got: {name}");
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
        let NodeKind::Binary { op, left, .. } = &expr.kind else {
            panic!(
                "Expected Binary subscript node, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(op, "{}", "Expected {{}} subscript operator, got: {op}");
        let NodeKind::Variable { sigil, name } = &left.kind else {
            panic!("Expected Variable node as subscript target, got: {}", left.kind.kind_name())
        };
        assert_eq!(sigil, "%", "Expected % sigil");
        assert_eq!(name, "Pkg::Hash", "Expected qualified name Pkg::Hash, got: {name}");
    }

    #[test]
    fn scalar_ref_hash_slice_preserves_base_target() {
        let code = "%$href{'a', 'b'};";
        assert_no_errors(code);

        let expr = first_expr(code);
        let NodeKind::Binary { op, left, right } = &expr.kind else {
            panic!(
                "Expected Binary hash-slice node, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(op, "{}", "Expected {{}} hash-slice operator, got: {op}");
        let NodeKind::Variable { sigil, name } = &left.kind else {
            panic!("Expected Variable node as hash-slice target, got: {}", left.kind.kind_name())
        };
        assert_eq!(sigil, "%", "Expected % sigil on hash-slice target");
        assert_eq!(name, "$href", "Expected scalar-ref hash target, got: {name}");
        let NodeKind::ArrayLiteral { elements } = &right.kind else {
            panic!("Expected ArrayLiteral slice key list, got: {}", right.kind.kind_name())
        };
        assert_eq!(elements.len(), 2, "Expected two hash-slice keys");
    }

    #[test]
    fn scalar_ref_hash_slice_list_preserves_base_target() {
        let code = "@$href{'a', 'b'};";
        assert_no_errors(code);

        let expr = first_expr(code);
        let NodeKind::Binary { op, left, right } = &expr.kind else {
            panic!(
                "Expected Binary hash-slice node, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(op, "{}", "Expected {{}} hash-slice operator, got: {op}");
        let NodeKind::Variable { sigil, name } = &left.kind else {
            panic!("Expected Variable node as hash-slice target, got: {}", left.kind.kind_name())
        };
        assert_eq!(sigil, "@", "Expected @ sigil on hash-slice target");
        assert_eq!(name, "$href", "Expected scalar-ref hash target, got: {name}");
        let NodeKind::ArrayLiteral { elements } = &right.kind else {
            panic!("Expected ArrayLiteral slice key list, got: {}", right.kind.kind_name())
        };
        assert_eq!(elements.len(), 2, "Expected two hash-slice keys");
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
        let NodeKind::Binary { op, left, .. } = &expr.kind else {
            panic!(
                "Expected Binary subscript, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(op, "[]", "Expected [] subscript operator, got: {op}");
        let NodeKind::Variable { sigil, name } = &left.kind else {
            panic!("Expected Variable node, got: {}", left.kind.kind_name())
        };
        assert_eq!(sigil, "$");
        assert_eq!(name, "Text::Unidecode::Char");
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
        let NodeKind::Binary { op, left, .. } = &expr.kind else {
            panic!(
                "Expected Binary subscript, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(op, "[]", "Expected outer [] subscript");
        let NodeKind::Binary { op: inner_op, left: inner_left, .. } = &left.kind else {
            panic!("Expected inner Binary subscript, got: {}", left.kind.kind_name())
        };
        assert_eq!(inner_op, "{}", "Expected inner {{}} subscript");
        let NodeKind::Variable { sigil, name } = &inner_left.kind else {
            panic!("Expected Variable node, got: {}", inner_left.kind.kind_name())
        };
        assert_eq!(sigil, "$");
        assert_eq!(name, "Pkg::Var");
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
        let NodeKind::Binary { op, left, .. } = &expr.kind else {
            panic!(
                "Expected Binary subscript, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(op, "[]");
        let NodeKind::Variable { sigil, name } = &left.kind else {
            panic!("Expected Variable node, got: {}", left.kind.kind_name())
        };
        assert_eq!(sigil, "$");
        assert_eq!(name, "Pkg::List");
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
        let NodeKind::Binary { op, left, right } = &expr.kind else {
            panic!(
                "Expected Binary subscript, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(op, "[]");
        let NodeKind::Variable { sigil, name } = &left.kind else {
            panic!("Expected Variable node, got: {}", left.kind.kind_name())
        };
        assert_eq!(sigil, "$");
        assert_eq!(name, "Pkg::Var");
        // The index should be a binary + expression
        let NodeKind::Binary { op: inner_op, .. } = &right.kind else {
            panic!("Expected Binary + expression in index, got: {}", right.kind.kind_name())
        };
        assert_eq!(inner_op, "+", "Expected + operator in index expression");
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
        let NodeKind::Binary { op, left, right } = &expr.kind else {
            panic!(
                "Expected Binary subscript, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(op, "{}");
        let NodeKind::Variable { sigil, name } = &left.kind else {
            panic!("Expected Variable as target, got: {}", left.kind.kind_name())
        };
        assert_eq!(sigil, "$");
        assert_eq!(name, "Pkg::Var");
        // The key should be a variable $key
        let NodeKind::Variable { sigil: key_sigil, name: key_name } = &right.kind else {
            panic!("Expected Variable as key, got: {}", right.kind.kind_name())
        };
        assert_eq!(key_sigil, "$");
        assert_eq!(key_name, "key");
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
        let NodeKind::Binary { op, left, .. } = &expr.kind else {
            panic!(
                "Expected Binary arrow deref, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(op, "->{}", "Expected ->{{}} arrow deref, got: {op}");
        // Left should be the [] subscript
        let NodeKind::Binary { op: inner_op, left: inner_left, .. } = &left.kind else {
            panic!("Expected Binary [], got: {}", left.kind.kind_name())
        };
        assert_eq!(inner_op, "[]");
        let NodeKind::Variable { sigil, name } = &inner_left.kind else {
            panic!("Expected Variable, got: {}", inner_left.kind.kind_name())
        };
        assert_eq!(sigil, "$");
        assert_eq!(name, "Pkg::Var");
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
        let NodeKind::Binary { op, left, right } = &expr.kind else {
            panic!("Expected Binary +, got: {} (sexp: {})", expr.kind.kind_name(), expr.to_sexp())
        };
        assert_eq!(op, "+");
        // Left: $Pkg::Var[0]
        let NodeKind::Binary { op: l_op, .. } = &left.kind else {
            panic!("Expected Binary [] on left, got: {}", left.kind.kind_name())
        };
        assert_eq!(l_op, "[]");
        // Right: $Pkg::Var{key}
        let NodeKind::Binary { op: r_op, .. } = &right.kind else {
            panic!("Expected Binary {{}} on right, got: {}", right.kind.kind_name())
        };
        assert_eq!(r_op, "{}");
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
        let NodeKind::Binary { op, left, .. } = &expr.kind else {
            panic!(
                "Expected Binary subscript, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(op, "{}");
        let NodeKind::Variable { sigil, name } = &left.kind else {
            panic!("Expected Variable, got: {}", left.kind.kind_name())
        };
        assert_eq!(sigil, "$");
        assert_eq!(name, "Config::Config");
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
        let NodeKind::Binary { op, left, right } = &expr.kind else {
            panic!(
                "Expected Binary subscript, got: {} (sexp: {})",
                expr.kind.kind_name(),
                expr.to_sexp()
            )
        };
        assert_eq!(op, "[]");
        let NodeKind::Variable { sigil, name } = &left.kind else {
            panic!("Expected Variable, got: {}", left.kind.kind_name())
        };
        assert_eq!(sigil, "$");
        assert_eq!(name, "Text::Unidecode::Char");
        // Index should be a hex number
        let NodeKind::Number { value } = &right.kind else {
            panic!("Expected Number, got: {}", right.kind.kind_name())
        };
        assert_eq!(value, "0xff", "Expected hex literal 0xff, got: {value}");
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
