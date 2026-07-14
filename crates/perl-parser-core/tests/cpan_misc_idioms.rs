//! CPAN Pattern Tests: Miscellaneous Idioms

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{Node, NodeKind};

fn collect_indirect_calls<'a>(node: &'a Node, calls: &mut Vec<(&'a str, &'a Node, usize)>) {
    if let NodeKind::IndirectCall { method, object, args } = &node.kind {
        calls.push((method.as_str(), object.as_ref(), args.len()));
    }

    for child in node.children() {
        collect_indirect_calls(child, calls);
    }
}

fn expect_try_indirect_call(
    calls: &[(&str, &Node, usize)],
    index: usize,
    expected_method: &str,
    expected_arg_count: usize,
) -> Result<(), String> {
    let (method, object, arg_count) = calls
        .get(index)
        .ok_or_else(|| format!("expected indirect call at index {index}, got {calls:?}"))?;
    if *method != expected_method {
        return Err(format!("expected method {expected_method}, got {method}"));
    }
    if *arg_count != expected_arg_count {
        return Err(format!(
            "expected {expected_arg_count} args for {expected_method}, got {arg_count}"
        ));
    }

    match &object.kind {
        NodeKind::Identifier { name } if name == "try" => Ok(()),
        other => Err(format!("expected `try` identifier object, got {other:?}")),
    }
}

#[test]
fn bless_hashref() {
    let code = "my $self = bless {}, $class;";
    assert_clean_parse(code);
}

#[test]
fn scalar_context_force() {
    let code = "my $count = scalar @array;";
    assert_clean_parse(code);
}

#[test]
fn ref_check() {
    let code = "my $type = ref($thing) || 'not a reference';";
    assert_clean_parse(code);
}

#[test]
fn defined_or_operator() {
    let code = "my $val = $input // 'default';";
    assert_clean_parse(code);
}

#[test]
fn chained_defined_or() {
    let code = "my $val = $first // $second // $third // 'fallback';";
    assert_clean_parse(code);
}

#[test]
fn string_repetition() {
    let code = "my $line = '-' x 80;";
    assert_clean_parse(code);
}

#[test]
fn heredoc_in_function_call() {
    let code = r#"print <<END;
Hello, $name!
Welcome to $place.
END
"#;
    assert_clean_parse(code);
}

#[test]
fn qw_list() {
    let code = "my @days = qw(Mon Tue Wed Thu Fri Sat Sun);";
    assert_clean_parse(code);
}

#[test]
fn complex_deref_chain() {
    let code = "$config->{database}{hosts}[0]{port}";
    assert_clean_parse(code);
}

#[test]
fn exists_delete() {
    let code = r#"
if (exists $cache{$key}) {
    my $val = delete $cache{$key};
}
"#;
    assert_clean_parse(code);
}

#[test]
fn delete_arrow_hash_deref() {
    assert_clean_parse("delete $self->{key};");
}

#[test]
fn delete_arrow_array_deref() {
    assert_clean_parse("delete $ref->[0];");
}

#[test]
fn delete_chained_subscripts() {
    assert_clean_parse("delete $self->{a}{b};");
}

#[test]
fn delete_chained_arrow_deref() {
    assert_clean_parse("delete $self->{a}->{b};");
}

#[test]
fn exists_arrow_hash_deref() {
    assert_clean_parse("exists $self->{key};");
}

#[test]
fn exists_arrow_array_deref() {
    assert_clean_parse("exists $ref->[0];");
}

#[test]
fn delete_with_statement_modifier() {
    assert_clean_parse("delete $self->{missing} if $self->{present};");
}

#[test]
fn exists_in_if_condition() {
    assert_clean_parse("my $val = delete $cache->{$key} if exists $cache->{$key};");
}

#[test]
fn local_input_record_separator() {
    let code = "local $/ = undef;";
    assert_clean_parse(code);
}

#[test]
fn data_section() {
    let code = r#"
while (<DATA>) {
    chomp;
    print "Line: $_\n";
}
__DATA__
line one
line two
"#;
    assert_clean_parse(code);
}

#[test]
fn multiline_string_concat() {
    let code = r#"
my $sql = "SELECT u.id, u.name, u.email "
        . "FROM users u "
        . "JOIN orders o ON o.user_id = u.id "
        . "WHERE o.total > ? "
        . "ORDER BY u.name";
"#;
    assert_clean_parse(code);
}

#[test]
fn open_three_arg() {
    let code = r#"open my $fh, '<:encoding(UTF-8)', $filename or die "Cannot open $filename: $!";"#;
    assert_clean_parse(code);
}

#[test]
fn printf_format() {
    let code = r#"printf "%04d-%02d-%02d %02d:%02d:%02d", $y, $m, $d, $h, $min, $sec;"#;
    assert_clean_parse(code);
}

#[test]
fn complex_sprintf() {
    let code = r#"my $msg = sprintf("Found %d items in %.2f seconds", $count, $elapsed);"#;
    assert_clean_parse(code);
}

#[test]
fn array_slice() {
    let code = "my @first_three = @array[0..2];";
    assert_clean_parse(code);
}

#[test]
fn negative_array_index() {
    let code = "my $last = $array[-1];";
    assert_clean_parse(code);
}

#[test]
fn push_pop_shift_unshift() {
    let code = r#"
push @stack, $item;
my $top = pop @stack;
my $first = shift @queue;
unshift @queue, $new_item;
"#;
    assert_clean_parse(code);
}

#[test]
fn splice_usage() {
    let code = "my @removed = splice(@array, 2, 3, @replacement);";
    assert_clean_parse(code);
}

// ---------------------------------------------------------------------------
// print/say/printf with block-form filehandle: print { $fh } ...
// ---------------------------------------------------------------------------

#[test]
fn print_block_scalar_fh() {
    let code = r#"print { $fh } "data\n";"#;
    assert_clean_parse(code);
}

#[test]
fn print_block_scalar_fh_is_indirect_call() {
    let code = r#"print { $fh } "data\n";"#;
    let ast = parse(code);
    let sexp = ast.to_sexp();
    assert!(sexp.contains("indirect_call"), "Expected indirect_call, got: {sexp}");
}

#[test]
fn say_block_scalar_fh() {
    let code = r#"say { $fh } "data";"#;
    assert_clean_parse(code);
}

#[test]
fn printf_block_scalar_fh() {
    let code = r#"printf { $fh } "%s\n", $line;"#;
    assert_clean_parse(code);
}

#[test]
fn print_block_typeglob_stderr() {
    let code = r#"print { *STDERR } "error\n";"#;
    assert_clean_parse(code);
}

#[test]
fn print_block_typeglob_stdout() {
    let code = r#"print { *STDOUT } "ok\n";"#;
    assert_clean_parse(code);
}

#[test]
fn print_block_hash_accessor() {
    let code = r#"print { $self->{fh} } "msg\n";"#;
    assert_clean_parse(code);
}

#[test]
fn print_block_method_call() {
    let code = r#"print { $self->fh() } "msg\n";"#;
    assert_clean_parse(code);
}

#[test]
fn print_without_block_still_works() {
    // Regression: plain print without block filehandle must still work
    let code = r#"
print "hello\n";
print STDOUT "message\n";
print STDERR "error\n";
print $fh "data\n";
"#;
    assert_clean_parse(code);
}

#[test]
fn is_indirect_call_pattern_call_presence_observer_for_try_filehandles() -> Result<(), String> {
    let code = r#"
print try 'print "ok\n";';
print try "\n";
close try or die "Could not close: $!";
"#;
    assert_clean_parse(code);
    let ast = parse(code);
    let mut indirect_calls = Vec::new();
    collect_indirect_calls(&ast, &mut indirect_calls);

    if indirect_calls.len() != 3 {
        return Err(format!("expected 3 try-filehandle indirect calls, got {indirect_calls:?}"));
    }
    expect_try_indirect_call(&indirect_calls, 0, "print", 1)?;
    expect_try_indirect_call(&indirect_calls, 1, "print", 1)?;
    expect_try_indirect_call(&indirect_calls, 2, "close", 0)?;
    Ok(())
}

#[test]
fn parse_indirect_call_call_presence_observer() -> Result<(), String> {
    let code = "print try \"ok\\n\";";
    assert_clean_parse(code);
    let ast = parse(code);
    let mut indirect_calls = Vec::new();
    collect_indirect_calls(&ast, &mut indirect_calls);
    expect_try_indirect_call(&indirect_calls, 0, "print", 1)?;

    let (_, object, _) =
        indirect_calls.first().ok_or_else(|| "expected one indirect call".to_string())?;
    let try_start = code.find("try").ok_or_else(|| "expected try token in source".to_string())?;
    let try_end = try_start + "try".len();
    if object.location.start != try_start || object.location.end != try_end {
        return Err(format!(
            "expected consumed object range {try_start}..{try_end}, got {}..{}",
            object.location.start, object.location.end
        ));
    }
    Ok(())
}

#[test]
fn print_block_with_multiple_args() {
    // print { $fh } with comma-separated arguments
    let code = r#"print { $fh } "key=", $value, "\n";"#;
    assert_clean_parse(code);
}

mod scalar_builtin_arrow_method {
    use super::*;

    /// scalar $dh->read — builtin followed by arrow-method call chain
    #[test]
    fn scalar_arrow_method() {
        let code = "my $n = scalar $dh->read;";
        assert_clean_parse(code);
    }

    /// ref $obj->method — ref followed by arrow call
    #[test]
    fn ref_arrow_method() {
        let code = r#"if (ref $obj->type eq "ARRAY") { 1; }"#;
        assert_clean_parse(code);
    }

    /// defined $self->{key} — defined with hash deref
    #[test]
    fn defined_hash_deref() {
        let code = "return unless defined $self->{value};";
        assert_clean_parse(code);
    }

    /// defined $ref->[0] — defined with array deref
    #[test]
    fn defined_array_deref() {
        let code = "my $ok = defined $ref->[0];";
        assert_clean_parse(code);
    }

    /// scalar @{$ref} context — indirect scalar on array deref
    #[test]
    fn scalar_deref_array() {
        let code = "my $count = scalar @{$aref};";
        assert_clean_parse(code);
    }

    /// Chained arrow on defined — common OO check
    #[test]
    fn defined_arrow_chain() {
        let code = "if (defined $self->{config}->{key}) { do_thing(); }";
        assert_clean_parse(code);
    }

    /// scalar in boolean context with arrow method
    #[test]
    fn scalar_arrow_method_bool() {
        let code = "if (scalar $dh->read) { process(); }";
        assert_clean_parse(code);
    }
}

mod dollar_dollar_scalar_deref {
    use super::*;

    fn sexp(source: &str) -> String {
        parse(source).to_sexp()
    }

    #[test]
    fn scalar_deref_keeps_referenced_variable_name() {
        // $$sv is an unbraced scalar dereference — equivalent to ${$sv}.
        // After the unbraced-deref fix it produces (unary_${} (variable $ sv)),
        // NOT the old buggy form (variable $ $sv).
        let sexp = sexp("my $x = $$sv;");
        assert!(
            sexp.contains("(unary_${} (variable $ sv))"),
            "expected $$sv to parse as unary_${{}} deref, got: {sexp}"
        );
        assert!(
            !sexp.contains("(variable $ $sv)"),
            "$$sv must NOT produce a variable with name $sv (old buggy form), got: {sexp}"
        );
        assert!(
            !sexp.contains("(my_declaration (variable $ x)(variable $ $))"),
            "expected $$sv not to collapse to the bare $$ PID variable, got: {sexp}"
        );
    }

    #[test]
    fn scalar_deref_keyword_named_variable_keeps_name() {
        // $$default is an unbraced scalar dereference — equivalent to ${$default}.
        let source = "my $x = $$default;";
        assert_clean_parse(source);
        let sexp = sexp(source);
        assert!(
            sexp.contains("(unary_${} (variable $ default))"),
            "expected $$default to parse as unary_${{}} deref, got: {sexp}"
        );
        assert!(
            !sexp.contains("(variable $ $default)"),
            "$$default must NOT produce a variable with name $default (old buggy form), got: {sexp}"
        );
    }

    #[test]
    fn bare_pid_special_variable_still_parses_as_pid() {
        let sexp = sexp("my $pid = $$;");
        assert!(
            sexp.contains("(variable $ $)"),
            "expected bare $$ to stay the PID special variable, got: {sexp}"
        );
    }

    #[test]
    fn b_terse_pattern_keeps_both_scalar_deref_uses() {
        // $$sv appears twice; both must parse as unary_${} deref nodes.
        let sexp = sexp(
            r#"
my $s = sprintf("%s #%d %s", class($sv), $$sv, $specialsv_name[$$sv]);
"#,
        );
        assert!(
            sexp.matches("(unary_${} (variable $ sv))").count() >= 2,
            "expected both $$sv uses to parse as unary_${{}} deref nodes, got: {sexp}"
        );
        assert!(
            !sexp.contains("(variable $ $sv)"),
            "neither $$sv use should produce the old buggy Variable form, got: {sexp}"
        );
    }
}
