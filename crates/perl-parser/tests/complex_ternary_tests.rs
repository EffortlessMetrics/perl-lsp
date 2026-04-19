//! Tests for complex ternary expression parsing.
//!
//! Covers method calls in branches, nested ternaries, references in branches,
//! ternary inside hash subscripts, and builtins in conditions.

use perl_parser::Parser;
use perl_parser::ast::NodeKind;

use perl_parser::ast::Node;

/// Helper: parse a Perl snippet and return Ok(()) on success, Err with the
/// parse error message on failure.
fn try_parse(code: &str) -> Result<(), String> {
    let mut parser = Parser::new(code);
    parser.parse().map(|_| ()).map_err(|e| format!("{e}"))
}

/// Recursively walk the AST and assert that no Error node is found.
fn assert_no_error_nodes(node: &Node) {
    assert!(
        !matches!(&node.kind, NodeKind::Error { .. }),
        "Found Error node in AST: {}",
        if let NodeKind::Error { message, .. } = &node.kind {
            message.as_str()
        } else {
            ""
        }
    );
    visit_children(node, assert_no_error_nodes);
}

/// Visit immediate children of a node.
fn visit_children<F: FnMut(&Node)>(node: &Node, mut f: F) {
    match &node.kind {
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            for s in statements {
                f(s);
            }
        }
        NodeKind::VariableDeclaration {
            variable,
            initializer,
            ..
        } => {
            f(variable);
            if let Some(init) = initializer {
                f(init);
            }
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            f(lhs);
            f(rhs);
        }
        NodeKind::Binary { left, right, .. } => {
            f(left);
            f(right);
        }
        NodeKind::Unary { operand, .. } => {
            f(operand);
        }
        NodeKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            f(condition);
            f(then_expr);
            f(else_expr);
        }
        NodeKind::ExpressionStatement { expression } => {
            f(expression);
        }
        NodeKind::FunctionCall { args, .. } => {
            for a in args {
                f(a);
            }
        }
        NodeKind::MethodCall { object, args, .. } => {
            f(object);
            for a in args {
                f(a);
            }
        }
        NodeKind::If {
            condition,
            then_branch,
            elsif_branches,
            else_branch,
        } => {
            f(condition);
            f(then_branch);
            for (cond, body) in elsif_branches {
                f(cond);
                f(body);
            }
            if let Some(eb) = else_branch {
                f(eb);
            }
        }
        NodeKind::ArrayLiteral { elements } => {
            for e in elements {
                f(e);
            }
        }
        NodeKind::HashLiteral { pairs } => {
            for (k, v) in pairs {
                f(k);
                f(v);
            }
        }
        NodeKind::Return { value: Some(v) } => {
            f(v);
        }
        // Leaf nodes or nodes we don't need to recurse into for this test
        _ => {}
    }
}

// ── Method calls in ternary branches ────────────────────────────────────

#[test]
fn ternary_method_calls_in_branches() -> Result<(), String> {
    try_parse("my $x = $a ? $b->method : $c->other;")
}

#[test]
fn ternary_chained_method_in_branches() -> Result<(), String> {
    try_parse("my $x = $a ? $b->foo->bar : $c->baz;")
}

#[test]
fn ternary_method_with_args_in_branches() -> Result<(), String> {
    try_parse("my $x = $a ? $b->method($arg) : $c->other($arg2);")
}

// ── References in ternary branches ──────────────────────────────────────

#[test]
fn ternary_hashref_and_arrayref_branches() -> Result<(), String> {
    try_parse("my $x = $cond ? {key => $v} : [];")
}

#[test]
fn ternary_arrayref_both_branches() -> Result<(), String> {
    try_parse("my $x = $cond ? [1, 2] : [3, 4];")
}

// ── Nested ternary ──────────────────────────────────────────────────────

#[test]
fn ternary_nested_in_then_branch() -> Result<(), String> {
    try_parse("my $x = $a ? $b ? $c : $d : $e;")
}

#[test]
fn ternary_nested_in_else_branch() -> Result<(), String> {
    try_parse("my $x = $a ? $b : $c ? $d : $e;")
}

#[test]
fn ternary_double_nested() -> Result<(), String> {
    try_parse("my $x = $a ? $b ? $c : $d : $e ? $f : $g;")
}

// ── Ternary inside hash subscript ───────────────────────────────────────

#[test]
fn ternary_inside_hash_subscript() -> Result<(), String> {
    try_parse("$hash{$cond ? 'a' : 'b'};")
}

#[test]
fn ternary_inside_array_subscript() -> Result<(), String> {
    try_parse("$arr[$cond ? 0 : 1];")
}

// ── Builtin in condition ────────────────────────────────────────────────

#[test]
fn ternary_defined_in_condition() -> Result<(), String> {
    try_parse("my $x = defined $y ? $y : $default;")
}

#[test]
fn ternary_ref_in_condition() -> Result<(), String> {
    try_parse("my $x = ref $obj ? $obj->name : 'scalar';")
}

// ── Complex conditions ──────────────────────────────────────────────────

#[test]
fn ternary_logical_and_condition() -> Result<(), String> {
    try_parse("my $x = $a && $b ? 'yes' : 'no';")
}

#[test]
fn ternary_comparison_condition() -> Result<(), String> {
    try_parse("my $x = $a > $b ? $a : $b;")
}

// ── Complex expressions in branches ─────────────────────────────────────

#[test]
fn ternary_string_concat_branches() -> Result<(), String> {
    try_parse("my $x = $a ? $b . $c : $d . $e;")
}

#[test]
fn ternary_arithmetic_branches() -> Result<(), String> {
    try_parse("my $x = $a ? $b + $c : $d * $e;")
}

#[test]
fn ternary_hash_access_branches() -> Result<(), String> {
    try_parse("my $x = $a ? $h{key} : $h{other};")
}

#[test]
fn ternary_array_index_branches() -> Result<(), String> {
    try_parse("my $x = $a ? $arr[0] : $arr[1];")
}

#[test]
fn ternary_regex_match_condition() -> Result<(), String> {
    try_parse("my $x = $str =~ /foo/ ? 'match' : 'no';")
}

// ── Ternary as function argument ────────────────────────────────────────

#[test]
fn ternary_as_function_arg() -> Result<(), String> {
    try_parse("print($cond ? 'yes' : 'no');")
}

#[test]
fn ternary_as_method_arg() -> Result<(), String> {
    try_parse("$obj->method($cond ? $a : $b);")
}

// ── Ternary with method call in condition ───────────────────────────────

#[test]
fn ternary_method_call_condition() -> Result<(), String> {
    try_parse("my $x = $obj->is_valid ? $obj->name : 'unknown';")
}

// ── Edge cases: ternary with complex sub-expressions ────────────────────

#[test]
fn ternary_anonymous_sub_in_branch() -> Result<(), String> {
    try_parse("my $x = $a ? sub { 1 } : sub { 2 };")
}

#[test]
fn ternary_hashref_constructor_in_branch() -> Result<(), String> {
    try_parse("my $x = $a ? { foo => 1, bar => 2 } : { baz => 3 };")
}

#[test]
fn ternary_deref_in_branch() -> Result<(), String> {
    try_parse("my $x = $a ? $ref->{key} : $ref->[0];")
}

#[test]
fn ternary_chained_deref_in_branch() -> Result<(), String> {
    try_parse("my $x = $a ? $ref->{key}{nested} : $ref->[0][1];")
}

#[test]
fn ternary_method_call_with_complex_args() -> Result<(), String> {
    try_parse("my $x = $a ? $obj->method($b, $c) : $obj->other($d);")
}

#[test]
fn ternary_qw_in_branch() -> Result<(), String> {
    try_parse("my @x = $a ? qw(foo bar) : qw(baz qux);")
}

#[test]
fn ternary_with_do_block() -> Result<(), String> {
    try_parse("my $x = $a ? do { 1 + 2 } : do { 3 + 4 };")
}

#[test]
fn ternary_in_list_context() -> Result<(), String> {
    try_parse("my @x = ($a ? 1 : 2, $b ? 3 : 4);")
}

#[test]
fn ternary_with_negation_in_condition() -> Result<(), String> {
    try_parse("my $x = !$a ? $b : $c;")
}

#[test]
fn ternary_with_defined_or_in_branch() -> Result<(), String> {
    try_parse("my $x = $a ? $b // $c : $d // $e;")
}

#[test]
fn ternary_with_string_eq_condition() -> Result<(), String> {
    try_parse("my $x = $a eq 'foo' ? 1 : 0;")
}

#[test]
fn ternary_in_hash_value() -> Result<(), String> {
    try_parse("my %h = (key => $a ? 1 : 2);")
}

#[test]
fn ternary_with_complex_lhs_assignment() -> Result<(), String> {
    try_parse("$hash{$key} = $cond ? $val1 : $val2;")
}

#[test]
fn ternary_with_arrow_deref_condition() -> Result<(), String> {
    try_parse("my $x = $ref->{flag} ? $ref->{a} : $ref->{b};")
}

#[test]
fn ternary_triple_nested() -> Result<(), String> {
    try_parse("my $x = $a ? $b ? $c ? 1 : 2 : 3 : 4;")
}

#[test]
fn ternary_with_function_call_in_condition() -> Result<(), String> {
    try_parse("my $x = length($s) > 0 ? $s : 'default';")
}

#[test]
fn ternary_with_exists_in_condition() -> Result<(), String> {
    try_parse("my $x = exists $h{key} ? $h{key} : 'missing';")
}

#[test]
fn ternary_with_wantarray() -> Result<(), String> {
    try_parse("return wantarray ? @list : $scalar;")
}

#[test]
fn ternary_complex_nested_with_methods() -> Result<(), String> {
    try_parse("my $x = $obj->check ? $obj->a->b : $obj->c ? $obj->d : $obj->e;")
}

#[test]
fn ternary_in_printf_arg() -> Result<(), String> {
    try_parse("printf('%s', $flag ? 'yes' : 'no');")
}

#[test]
fn ternary_with_unary_minus() -> Result<(), String> {
    try_parse("my $x = $a ? -$b : -$c;")
}

// ── Assignment inside ternary branches ──────────────────────────────────

#[test]
fn ternary_assignment_in_then_branch() -> Result<(), String> {
    // Perl: $a ? ($b = 1) : $c  -- assignment in then branch
    // The then-branch of a ternary should allow assignment expressions
    try_parse("$a ? $b = 1 : $c;")
}

#[test]
fn ternary_assignment_in_else_branch() -> Result<(), String> {
    // Perl: $a ? $b : ($c = 1)  -- assignment in else branch
    try_parse("$a ? $b : $c = 1;")
}

#[test]
fn ternary_assignment_in_both_branches() -> Result<(), String> {
    try_parse("$a ? $b = 1 : $c = 2;")
}

#[test]
fn ternary_concat_assign_in_branch() -> Result<(), String> {
    try_parse("$a ? $b .= 'x' : $c .= 'y';")
}

/// Verify that `$a ? $b = 1 : $c` produces no error nodes.
/// Currently the parser fails because ternary branches do not allow
/// assignment expressions (the `=` is unexpected when looking for `:`).
#[test]
fn ternary_assignment_in_then_branch_no_error() -> Result<(), String> {
    let mut parser = Parser::new("$a ? $b = 1 : $c;");
    let ast = parser.parse().map_err(|e| format!("{e}"))?;

    if let NodeKind::Program { statements } = &ast.kind {
        let stmt = statements.first().ok_or("no statements")?;
        assert_no_error_nodes(stmt);
        Ok(())
    } else {
        Err("Expected Program node".to_string())
    }
}

/// Verify that `$a ? $b : $c = 1` (assignment in else-branch) has no error
/// nodes.
#[test]
fn ternary_assignment_in_else_branch_no_error() -> Result<(), String> {
    let mut parser = Parser::new("$a ? $b : $c = 1;");
    let ast = parser.parse().map_err(|e| format!("{e}"))?;

    if let NodeKind::Program { statements } = &ast.kind {
        let stmt = statements.first().ok_or("no statements")?;
        assert_no_error_nodes(stmt);
        Ok(())
    } else {
        Err("Expected Program node".to_string())
    }
}

/// Verify that compound assignment (`$a ? $b .= 'x' : $c .= 'y'`) has no
/// error nodes.
#[test]
fn ternary_compound_assign_in_branches_no_error() -> Result<(), String> {
    let mut parser = Parser::new("$a ? $b .= 'x' : $c .= 'y';");
    let ast = parser.parse().map_err(|e| format!("{e}"))?;

    if let NodeKind::Program { statements } = &ast.kind {
        let stmt = statements.first().ok_or("no statements")?;
        assert_no_error_nodes(stmt);
        Ok(())
    } else {
        Err("Expected Program node".to_string())
    }
}

/// Verify nested ternary still works when branches allow assignment:
/// `$a ? $b ? $c : $d : $e` (right-associative nesting in then-branch)
#[test]
fn ternary_nested_still_works_with_assignment_fix() -> Result<(), String> {
    let mut parser = Parser::new("$a ? $b ? $c : $d : $e;");
    let ast = parser.parse().map_err(|e| format!("{e}"))?;

    if let NodeKind::Program { statements } = &ast.kind {
        let stmt = statements.first().ok_or("no statements")?;
        assert_no_error_nodes(stmt);
        Ok(())
    } else {
        Err("Expected Program node".to_string())
    }
}

/// Verify that `$a ? $b : $c ? $d : $e` (chained else-branch ternary) still
/// works and produces no errors.
#[test]
fn ternary_chained_else_still_works_with_assignment_fix() -> Result<(), String> {
    let mut parser = Parser::new("$a ? $b : $c ? $d : $e;");
    let ast = parser.parse().map_err(|e| format!("{e}"))?;

    if let NodeKind::Program { statements } = &ast.kind {
        let stmt = statements.first().ok_or("no statements")?;
        assert_no_error_nodes(stmt);
        Ok(())
    } else {
        Err("Expected Program node".to_string())
    }
}

// ── Ternary with comma (list context) inside branches ───────────────────

#[test]
fn ternary_comma_expr_in_parens_in_branch() -> Result<(), String> {
    try_parse("my @x = $a ? (1, 2, 3) : (4, 5, 6);")
}

// ── Tricky lexer edge cases ─────────────────────────────────────────────

#[test]
fn ternary_after_close_paren() -> Result<(), String> {
    // ')' followed by '?' - must not confuse lexer into regex mode
    try_parse("my $x = ($a) ? $b : $c;")
}

#[test]
fn ternary_after_close_bracket() -> Result<(), String> {
    try_parse("my $x = $arr[0] ? 'yes' : 'no';")
}

#[test]
fn ternary_after_close_brace() -> Result<(), String> {
    try_parse("my $x = $h{k} ? 'yes' : 'no';")
}

#[test]
fn ternary_after_postfix_increment() -> Result<(), String> {
    try_parse("my $x = $a++ ? 1 : 0;")
}

#[test]
fn ternary_with_package_separator_in_branches() -> Result<(), String> {
    // Colon in Foo::Bar must not be confused with ternary colon
    try_parse("my $x = $a ? Foo::Bar->new : Baz::Qux->new;")
}

#[test]
fn ternary_with_label_nearby() -> Result<(), String> {
    // Label colon must not be confused with ternary colon
    try_parse("LABEL: my $x = $a ? 1 : 0;")
}

#[test]
fn ternary_method_returns_hashref() -> Result<(), String> {
    try_parse("my $x = $obj->get_data ? $obj->get_data->{key} : 'default';")
}

#[test]
fn ternary_with_sprintf() -> Result<(), String> {
    try_parse("my $s = $flag ? sprintf('%d', $n) : sprintf('%s', $str);")
}

#[test]
fn ternary_with_map_in_branch() -> Result<(), String> {
    try_parse("my @x = $flag ? map { $_ * 2 } @arr : @arr;")
}

#[test]
fn ternary_with_grep_in_branch() -> Result<(), String> {
    try_parse("my @x = $flag ? grep { $_ > 0 } @arr : @arr;")
}

#[test]
fn ternary_with_shift_in_branch() -> Result<(), String> {
    try_parse("my $x = @_ ? shift : 'default';")
}

#[test]
fn ternary_with_local_in_branch() -> Result<(), String> {
    try_parse("my $x = $a ? local $/ : $default;")
}

#[test]
fn ternary_complex_multiline() -> Result<(), String> {
    try_parse("my $x = $condition\n    ? $true_value\n    : $false_value;")
}

#[test]
fn ternary_in_return_statement() -> Result<(), String> {
    try_parse("return $a ? $b : $c;")
}

#[test]
fn ternary_chained_four_deep() -> Result<(), String> {
    try_parse("my $x = $a ? 1 : $b ? 2 : $c ? 3 : 4;")
}

#[test]
fn ternary_with_scalar_deref_in_condition() -> Result<(), String> {
    try_parse("my $x = $$ref ? $$ref : 'none';")
}

#[test]
fn ternary_with_array_slice_in_branch() -> Result<(), String> {
    try_parse("my @x = $a ? @arr[0..2] : @arr[3..5];")
}

#[test]
fn ternary_with_chomp_in_condition() -> Result<(), String> {
    try_parse("my $x = chomp($line) ? $line : '';")
}

#[test]
fn ternary_with_die_in_else() -> Result<(), String> {
    try_parse("my $x = $a ? $a : die 'error';")
}
