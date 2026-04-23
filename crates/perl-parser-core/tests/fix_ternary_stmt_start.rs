mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_defined_ternary_at_stmt_start() {
    assert_clean_parse(r#"defined $x ? "yes" : "no";"#);
}

#[test]
fn test_ref_ternary_at_stmt_start() {
    assert_clean_parse(r#"ref $x ? "hash" : "scalar";"#);
}

#[test]
fn test_wantarray_ternary_at_stmt_start() {
    assert_clean_parse(r#"wantarray ? @list : $scalar;"#);
}

#[test]
fn test_caller_ternary_at_stmt_start() {
    assert_clean_parse(r#"caller ? 1 : 0;"#);
}

#[test]
fn test_defined_ternary_with_arrow_chain() {
    assert_clean_parse(r#"defined $self->{key} ? $self->{key} : "default";"#);
}

#[test]
fn test_defined_ternary_assigned() {
    assert_clean_parse(r#"my $v = defined $x ? $x : "fallback";"#);
}

#[test]
fn test_nested_ternary_at_stmt_start() {
    assert_clean_parse(r#"$a ? $b ? 1 : 2 : 3;"#);
}

#[test]
fn test_ternary_in_function_arg() {
    assert_clean_parse(r#"foo($x ? 1 : 2);"#);
}

#[test]
fn test_method_call_ternary() {
    assert_clean_parse(r#"$obj->method ? "yes" : "no";"#);
}

#[test]
fn test_ternary_in_hash_value() {
    assert_clean_parse(r#"my %h = (key => $x ? 1 : 2);"#);
}

#[test]
fn test_chained_ternary() {
    assert_clean_parse(r#"$a == 1 ? "one" : $a == 2 ? "two" : "other";"#);
}

#[test]
fn test_ternary_with_regex_condition() {
    assert_clean_parse(r#"$x =~ /foo/ ? "matched" : "no match";"#);
}

#[test]
fn test_ternary_after_comparison() {
    assert_clean_parse(r#"$count > 0 ? "some" : "none";"#);
}

#[test]
fn test_ternary_in_print_stmt() {
    assert_clean_parse(r#"print($x ? "yes" : "no");"#);
}

#[test]
fn test_ternary_as_list_element() {
    assert_clean_parse(r#"my @a = ($x ? 1 : 2, $y ? 3 : 4);"#);
}

#[test]
fn test_assignment_ternary() {
    assert_clean_parse(r#"$result = $flag ? "on" : "off";"#);
}

#[test]
fn test_print_stderr_ternary() {
    assert_clean_parse(r#"print STDERR $x ? "yes" : "no";"#);
}

#[test]
fn test_anon_sub_ternary() {
    assert_clean_parse(r#"my $f = sub { $x ? 1 : 0 };"#);
}

#[test]
fn test_arrow_chain_ternary() {
    assert_clean_parse(r#"$self->{foo} ? $self->{bar} : $self->{baz};"#);
}

#[test]
fn test_hash_slice_ternary() {
    assert_clean_parse(r#"@hash{@keys} ? 1 : 0;"#);
}

#[test]
fn test_do_block_ternary() {
    assert_clean_parse(r#"do { $x } ? 1 : 0;"#);
}

#[test]
fn test_eval_block_ternary() {
    assert_clean_parse(r#"eval { $x } ? 1 : 0;"#);
}

#[test]
fn test_can_method_ternary() {
    assert_clean_parse(r#"$obj->can("method") ? $obj->method() : die;"#);
}

#[test]
fn test_hashref_value_ternary() {
    assert_clean_parse(r#"my $h = { key => $x ? 1 : 2 };"#);
}

#[test]
fn test_string_repeat_ternary() {
    assert_clean_parse(r#""x" x ($n ? $n : 1);"#);
}

#[test]
fn test_brace_regex_ternary() {
    assert_clean_parse(r#"$x =~ m{pattern}i ? 1 : 0;"#);
}

#[test]
fn test_while_condition_ternary() {
    assert_clean_parse(r#"while ($x ? 1 : 0) { last; }"#);
}

#[test]
fn test_chained_method_ternary() {
    assert_clean_parse(r#"$obj->foo->bar ? 1 : 0;"#);
}

#[test]
fn test_list_assignment_ternary() {
    assert_clean_parse(r#"($a, $b) = $x ? (1, 2) : (3, 4);"#);
}

#[test]
fn test_ternary_with_returns() {
    assert_clean_parse(r#"$x ? return 1 : return 0;"#);
}

#[test]
fn test_for_c_style_ternary() {
    assert_clean_parse(r#"for (my $i = $x ? 0 : 1; $i < 10; $i++) { 1; }"#);
}

#[test]
fn test_custom_func_no_parens_ternary() {
    assert_clean_parse(r#"is_ready $obj ? 1 : 0;"#);
}

#[test]
fn test_custom_func_no_parens_or_fallback() {
    assert_clean_parse(r#"is_ready $obj or die "not ready";"#);
}

#[test]
fn test_ternary_after_not() {
    assert_clean_parse(r#"!$x ? 1 : 0;"#);
}

#[test]
fn test_ternary_after_bitwise() {
    assert_clean_parse(r#"($a & $b) ? 1 : 0;"#);
}

#[test]
fn test_ternary_after_string_eq() {
    assert_clean_parse(r#"$a eq $b ? 1 : 0;"#);
}

#[test]
fn test_ternary_after_string_multiply() {
    assert_clean_parse(r#"$x x 2 ? 1 : 0;"#);
}

#[test]
fn test_ternary_after_builtin_call() {
    assert_clean_parse(r#"length($x) ? 1 : 0;"#);
}

#[test]
fn test_sub_ternary_in_body() {
    assert_clean_parse(r#"sub foo { return $x ? 1 : 0; }"#);
}

#[test]
fn test_return_ternary() {
    assert_clean_parse(r#"return $x ? 1 : 0;"#);
}

#[test]
fn test_warn_ternary() {
    assert_clean_parse(r#"warn $x ? "yes" : "no";"#);
}

#[test]
fn test_die_ternary() {
    assert_clean_parse(r#"die $x ? "yes" : "no";"#);
}

#[test]
fn test_exists_ternary() {
    assert_clean_parse(r#"exists $h{key} ? 1 : 0;"#);
}

#[test]
fn test_local_ternary() {
    assert_clean_parse(r#"local $x = $flag ? 1 : 0;"#);
}

#[test]
fn test_file_test_e_ternary() {
    assert_clean_parse(r#"-e $file ? 1 : 0;"#);
}

#[test]
fn test_file_test_f_ternary() {
    assert_clean_parse(r#"-f $file ? 1 : 0;"#);
}

#[test]
fn test_file_test_d_ternary() {
    assert_clean_parse(r#"-d $dir ? 1 : 0;"#);
}

#[test]
fn test_map_block_ternary() {
    assert_clean_parse(r#"map { $_ ? 1 : 0 } @items;"#);
}

#[test]
fn test_grep_with_ternary() {
    assert_clean_parse(r#"grep { $_ ? 1 : 0 } @items;"#);
}

#[test]
fn test_printf_ternary() {
    assert_clean_parse(r#"printf "%s", $x ? "yes" : "no";"#);
}

#[test]
fn test_printf_handle_ternary() {
    assert_clean_parse(r#"printf STDERR "%s", $x ? "yes" : "no";"#);
}

#[test]
fn test_push_ternary() {
    assert_clean_parse(r#"push @items, $x ? 1 : 0;"#);
}

#[test]
fn test_sort_ternary_comparison() {
    assert_clean_parse(r#"sort { $a < $b ? -1 : 1 } @items;"#);
}

#[test]
fn test_multiple_ternary_comma_list() {
    assert_clean_parse(r#"my @vals = ($a ? 1 : 0, $b ? 1 : 0, $c ? 1 : 0);"#);
}

#[test]
fn test_ternary_array_access_condition() {
    assert_clean_parse(r#"$items[0] ? 1 : 0;"#);
}

#[test]
fn test_ternary_in_arrayref() {
    assert_clean_parse(r#"my $x = [$flag ? 1 : 0];"#);
}

#[test]
fn test_ternary_in_hashref() {
    assert_clean_parse(r#"my $x = { key => $flag ? 1 : 0 };"#);
}

#[test]
fn test_ternary_multi_key_hash() {
    assert_clean_parse(r#"my %h = ($flag ? (a => 1) : (b => 2));"#);
}

#[test]
fn test_ternary_string_concat_chain() {
    assert_clean_parse(r#""x" . ($flag ? "a" : "b") . "y";"#);
}

#[test]
fn test_ternary_chained_method_call() {
    assert_clean_parse(r#"($obj->foo ? $obj->bar() : $obj->baz())->qux;"#);
}

#[test]
fn test_ternary_defined_or_chain() {
    assert_clean_parse(r#"($x // $y) ? 1 : 0;"#);
}

#[test]
fn test_ternary_negative_number() {
    assert_clean_parse(r#"(-1) ? 1 : 0;"#);
}

#[test]
fn test_ternary_ref_check_pattern() {
    assert_clean_parse(r#"ref $x eq "HASH" ? $x->{a} : 0;"#);
}

#[test]
fn test_ternary_special_var() {
    assert_clean_parse(r#"$? ? 1 : 0;"#);
}

#[test]
fn test_array_bool_ternary() {
    assert_clean_parse(r#"@items ? 1 : 0;"#);
}

#[test]
fn test_hash_bool_ternary() {
    assert_clean_parse(r#"%h ? 1 : 0;"#);
}

#[test]
fn test_scalar_deref_ternary() {
    assert_clean_parse(r#"${$ref} ? 1 : 0;"#);
}

#[test]
fn test_array_deref_ternary() {
    assert_clean_parse(r#"@{$ref} ? 1 : 0;"#);
}

#[test]
fn test_brace_deref_ternary() {
    assert_clean_parse(r#"$ref->{key} ? 1 : 0;"#);
}

#[test]
fn test_sub_prototype_ternary() {
    assert_clean_parse(r#"sub foo ($x) { $x ? 1 : 0 }"#);
}

#[test]
fn test_oo_accessor_ternary() {
    assert_clean_parse(r#"$self->name ? $self->name : "anon";"#);
}

#[test]
fn test_negated_method_ternary() {
    assert_clean_parse(r#"!$obj->ok ? 1 : 0;"#);
}

#[test]
fn test_ternary_after_string_eq_literal() {
    assert_clean_parse(r#"$x eq "y" ? 1 : 0;"#);
}

#[test]
fn test_ternary_interpolated_result() {
    assert_clean_parse(r#"my $s = "value=" . ($x ? "yes" : "no");"#);
}

#[test]
fn test_ternary_in_sprintf() {
    assert_clean_parse(r#"sprintf "%s", $x ? "yes" : "no";"#);
}

#[test]
fn test_ternary_with_concat() {
    assert_clean_parse(r#"($x ? "a" : "b") . "c";"#);
}

#[test]
fn test_ternary_with_defined_or_condition() {
    assert_clean_parse(r#"($x // 0) ? 1 : 0;"#);
}

#[test]
fn test_ternary_with_and_or_condition() {
    assert_clean_parse(r#"($a && $b) ? 1 : 0;"#);
}

#[test]
fn test_ternary_qw_operand() {
    assert_clean_parse(r#"my @x = $flag ? qw(a b) : qw(c d);"#);
}

#[test]
fn test_wantarray_method_ternary() {
    assert_clean_parse(r#"wantarray ? $obj->list : $obj->scalar;"#);
}
