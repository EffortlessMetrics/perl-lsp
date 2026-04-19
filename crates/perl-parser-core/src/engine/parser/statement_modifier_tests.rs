use super::*;
use perl_tdd_support::must;

/// Helper: parse code, return the sexp representation, and assert no ERROR nodes.
fn parse_ok(code: &str) -> String {
    let mut parser = Parser::new(code);
    let result = parser.parse();
    let ast = must(result);
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "Parse produced ERROR node(s) for: {}\nSexp: {}",
        code,
        sexp
    );
    sexp
}

/// Helper: parse code and assert the sexp contains a statement modifier node
/// with the given keyword. The sexp format is `statement_modifier_<keyword>`.
fn assert_has_modifier(code: &str, modifier_keyword: &str) {
    let sexp = parse_ok(code);
    let expected_fragment = format!("statement_modifier_{}", modifier_keyword);
    assert!(
        sexp.contains(&expected_fragment),
        "Expected statement modifier '{}' in parsed output of: {}\nSexp: {}",
        modifier_keyword,
        code,
        sexp
    );
}

// ---- Tests for statement modifiers after complex expressions ----

#[test]
fn test_modifier_after_hash_assignment() -> Result<(), Box<dyn std::error::Error>> {
    // $hash{$k} ||= '' if $cond;
    assert_has_modifier(r#"$hash{$k} ||= '' if $cond;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_push_with_deref() -> Result<(), Box<dyn std::error::Error>> {
    // push @{$ref}, $v unless $skip;
    assert_has_modifier(r#"push @{$ref}, $v unless $skip;"#, "unless");
    Ok(())
}

#[test]
fn test_modifier_with_method_call() -> Result<(), Box<dyn std::error::Error>> {
    // $obj->method for @list;
    assert_has_modifier(r#"$obj->method for @list;"#, "for");
    Ok(())
}

#[test]
fn test_modifier_after_delete() -> Result<(), Box<dyn std::error::Error>> {
    // delete $hash{$key} if exists $hash{$key};
    assert_has_modifier(r#"delete $hash{$key} if exists $hash{$key};"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_print_with_filehandle() -> Result<(), Box<dyn std::error::Error>> {
    // print $fh "hello\n" unless $quiet;
    assert_has_modifier(r#"print $fh "hello\n" unless $quiet;"#, "unless");
    Ok(())
}

// Additional edge cases

#[test]
fn test_modifier_after_push_nested_deref() -> Result<(), Box<dyn std::error::Error>> {
    // push @{$hash{$k}}, $v if $cond;
    assert_has_modifier(r#"push @{$hash{$k}}, $v if $cond;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_while_after_expression() -> Result<(), Box<dyn std::error::Error>> {
    // $count++ while $count < 10;
    assert_has_modifier(r#"$count++ while $count < 10;"#, "while");
    Ok(())
}

#[test]
fn test_modifier_until_after_expression() -> Result<(), Box<dyn std::error::Error>> {
    // $x *= 2 until $x > 100;
    assert_has_modifier(r#"$x *= 2 until $x > 100;"#, "until");
    Ok(())
}

#[test]
fn test_modifier_foreach_after_expression() -> Result<(), Box<dyn std::error::Error>> {
    // print $_ foreach @items;
    assert_has_modifier(r#"print $_ foreach @items;"#, "foreach");
    Ok(())
}

#[test]
fn test_simple_modifier_if() -> Result<(), Box<dyn std::error::Error>> {
    // Baseline: simple expression with modifier should work
    assert_has_modifier(r#"$x = 1 if $cond;"#, "if");
    Ok(())
}

// ---- More complex edge cases to verify robustness ----

#[test]
fn test_modifier_after_nested_hash_deref() -> Result<(), Box<dyn std::error::Error>> {
    // push @{$hash{$k}{$j}}, $v if $cond;
    assert_has_modifier(r#"push @{$hash{$k}{$j}}, $v if $cond;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_chained_deref() -> Result<(), Box<dyn std::error::Error>> {
    // push @{$self->{data}}, $item for @items;
    assert_has_modifier(r#"push @{$self->{data}}, $item for @items;"#, "for");
    Ok(())
}

#[test]
fn test_modifier_after_defined_or_assign() -> Result<(), Box<dyn std::error::Error>> {
    // $hash{key} //= [] if $init;
    assert_has_modifier(r#"$hash{key} //= [] if $init;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_array_deref() -> Result<(), Box<dyn std::error::Error>> {
    // @{$ref} = () unless @{$ref};
    assert_has_modifier(r#"@{$ref} = () unless @{$ref};"#, "unless");
    Ok(())
}

#[test]
fn test_modifier_after_mixed_deref() -> Result<(), Box<dyn std::error::Error>> {
    // $ref->{$k}[$i] = $v unless $skip;
    assert_has_modifier(r#"$ref->{$k}[$i] = $v unless $skip;"#, "unless");
    Ok(())
}

// ---- Subtle edge cases: keywords as hash keys + modifiers ----

#[test]
fn test_modifier_if_after_hash_key_if() -> Result<(), Box<dyn std::error::Error>> {
    // $x = $hash{if} if $y;  -- 'if' used as hash key, then as modifier
    assert_has_modifier(r#"$x = $hash{if} if $y;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_if_after_hash_key_for() -> Result<(), Box<dyn std::error::Error>> {
    // $x->{for} = 1 if $y;  -- 'for' used as hash key, then 'if' as modifier
    assert_has_modifier(r#"$x->{for} = 1 if $y;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_grep_block() -> Result<(), Box<dyn std::error::Error>> {
    // @a = grep { $_ > 0 } @b if @b;
    assert_has_modifier(r#"@a = grep { $_ > 0 } @b if @b;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_join_call() -> Result<(), Box<dyn std::error::Error>> {
    // $r = join(',', @a) if @a;
    assert_has_modifier(r#"$r = join(',', @a) if @a;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_unshift_deref() -> Result<(), Box<dyn std::error::Error>> {
    // unshift @{$data->{items}}, $new if $new;
    assert_has_modifier(r#"unshift @{$data->{items}}, $new if $new;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_splice_deref() -> Result<(), Box<dyn std::error::Error>> {
    // splice @{$arr}, $i, 1 if $remove;
    assert_has_modifier(r#"splice @{$arr}, $i, 1 if $remove;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_my_declaration() -> Result<(), Box<dyn std::error::Error>> {
    // my $x = 1 if $cond;  -- conditional declaration (Perl allows this)
    assert_has_modifier(r#"my $x = 1 if $cond;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_chomp_with_parens() -> Result<(), Box<dyn std::error::Error>> {
    // chomp(my $line = <STDIN>) if defined $line;
    assert_has_modifier(r#"chomp(my $line = <STDIN>) if $flag;"#, "if");
    Ok(())
}

// ---- Tests for expressions that are NOT builtins (general expression path) ----

#[test]
fn test_modifier_after_user_function_with_hash_arg() -> Result<(), Box<dyn std::error::Error>> {
    // foo($hash{$k}) if $cond;
    assert_has_modifier(r#"foo($hash{$k}) if $cond;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_complex_assignment_chain() -> Result<(), Box<dyn std::error::Error>> {
    // $a = $b = $c if $d;
    assert_has_modifier(r#"$a = $b = $c if $d;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_ternary() -> Result<(), Box<dyn std::error::Error>> {
    // $x = $a ? $b : $c if $d;
    assert_has_modifier(r#"$x = $a ? $b : $c if $d;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_regex_match() -> Result<(), Box<dyn std::error::Error>> {
    // $text =~ s/foo/bar/g if $text;
    assert_has_modifier(r#"$text =~ s/foo/bar/g if $text;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_string_concat() -> Result<(), Box<dyn std::error::Error>> {
    // $str .= "suffix" unless $done;
    assert_has_modifier(r#"$str .= "suffix" unless $done;"#, "unless");
    Ok(())
}

#[test]
fn test_modifier_after_complex_deref_chain() -> Result<(), Box<dyn std::error::Error>> {
    // $self->{cache}{$key} = $val if defined $val;
    assert_has_modifier(r#"$self->{cache}{$key} = $val if defined $val;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_array_slice() -> Result<(), Box<dyn std::error::Error>> {
    // @result = @array[0..2] if @array;
    assert_has_modifier(r#"@result = @array[0..2] if @array;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_die() -> Result<(), Box<dyn std::error::Error>> {
    // die "error" if $bad;
    assert_has_modifier(r#"die "error" if $bad;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_warn() -> Result<(), Box<dyn std::error::Error>> {
    // warn "oops" unless $quiet;
    assert_has_modifier(r#"warn "oops" unless $quiet;"#, "unless");
    Ok(())
}

#[test]
fn test_modifier_after_return() -> Result<(), Box<dyn std::error::Error>> {
    // return $val if defined $val;
    assert_has_modifier(r#"return $val if defined $val;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_next_in_loop() -> Result<(), Box<dyn std::error::Error>> {
    // next if $skip;
    assert_has_modifier(r#"next if $skip;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_last_in_loop() -> Result<(), Box<dyn std::error::Error>> {
    // last unless $continue;
    assert_has_modifier(r#"last unless $continue;"#, "unless");
    Ok(())
}

// ---- Tests for comma expressions and multi-arrow derefs with modifiers ----

#[test]
fn test_modifier_after_chained_method_calls() -> Result<(), Box<dyn std::error::Error>> {
    // $obj->foo->bar if $cond;
    assert_has_modifier(r#"$obj->foo->bar if $cond;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_deep_arrow_deref() -> Result<(), Box<dyn std::error::Error>> {
    // $self->{data}->[$i] = $v for @items;
    assert_has_modifier(r#"$self->{data}->[$i] = $v for @items;"#, "for");
    Ok(())
}

#[test]
fn test_modifier_after_scalar_deref() -> Result<(), Box<dyn std::error::Error>> {
    // ${$ref} = 1 if $flag;
    assert_has_modifier(r#"${$ref} = 1 if $flag;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_nested_hash_access() -> Result<(), Box<dyn std::error::Error>> {
    // $hash{a}{b}{c} = 1 unless $done;
    assert_has_modifier(r#"$hash{a}{b}{c} = 1 unless $done;"#, "unless");
    Ok(())
}

#[test]
fn test_modifier_after_local_sig_handler() -> Result<(), Box<dyn std::error::Error>> {
    // local $SIG{__WARN__} = sub {} if $quiet;
    assert_has_modifier(r#"local $SIG{__WARN__} = sub {} if $quiet;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_postfix_deref_push() -> Result<(), Box<dyn std::error::Error>> {
    // push @{$data->{list}}, map { $_ * 2 } @vals if @vals;
    assert_has_modifier(
        r#"push @{$data->{list}}, map { $_ * 2 } @vals if @vals;"#,
        "if",
    );
    Ok(())
}

#[test]
fn test_modifier_after_complex_lhs_or_assign() -> Result<(), Box<dyn std::error::Error>> {
    // $cache->{$key} ||= compute($key) if $use_cache;
    assert_has_modifier(r#"$cache->{$key} ||= compute($key) if $use_cache;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_substr_assign() -> Result<(), Box<dyn std::error::Error>> {
    // substr($str, 0, 3) = "abc" if length($str) >= 3;
    assert_has_modifier(r#"substr($str, 0, 3) = "abc" if length($str) >= 3;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_comma_expression() -> Result<(), Box<dyn std::error::Error>> {
    // ($a, $b) = (1, 2) if $cond;
    assert_has_modifier(r#"($a, $b) = (1, 2) if $cond;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_open_with_three_args() -> Result<(), Box<dyn std::error::Error>> {
    // open my $fh, '<', $file or die if -e $file;
    // Note: this is `open(my $fh, '<', $file) or die` with `if -e $file` modifier
    assert_has_modifier(r#"open my $fh, '<', $file or die if -e $file;"#, "if");
    Ok(())
}

#[test]
fn test_modifier_after_push_with_map() -> Result<(), Box<dyn std::error::Error>> {
    // push @result, map { $_->{name} } @items unless $skip;
    assert_has_modifier(
        r#"push @result, map { $_->{name} } @items unless $skip;"#,
        "unless",
    );
    Ok(())
}
