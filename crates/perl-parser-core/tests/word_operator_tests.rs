mod cpan_test_helpers;
use cpan_test_helpers::*;

// === Basic word operator expressions ===

#[test]
fn test_word_or_basic() {
    assert_clean_parse("$a or $b;");
}

#[test]
fn test_word_not_basic() {
    assert_clean_parse("not $x;");
}

#[test]
fn test_word_and_basic() {
    assert_clean_parse("$a and $b;");
}

#[test]
fn test_word_eq_basic() {
    assert_clean_parse("$a eq $b;");
}

#[test]
fn test_word_ne_basic() {
    assert_clean_parse("$a ne $b;");
}

#[test]
fn test_word_lt_basic() {
    assert_clean_parse("$a lt $b;");
}

#[test]
fn test_word_gt_basic() {
    assert_clean_parse("$a gt $b;");
}

#[test]
fn test_word_le_basic() {
    assert_clean_parse("$a le $b;");
}

#[test]
fn test_word_ge_basic() {
    assert_clean_parse("$a ge $b;");
}

#[test]
fn test_word_cmp_basic() {
    assert_clean_parse("$a cmp $b;");
}

#[test]
fn test_word_xor_basic() {
    assert_clean_parse("$a xor $b;");
}

// === Common CPAN patterns with word operators ===

#[test]
fn test_open_or_die() {
    assert_clean_parse(r#"open(FH, $file) or die "Cannot open: $!";"#);
}

#[test]
fn test_close_or_die() {
    assert_clean_parse(r#"close(FH) or die "Cannot close: $!";"#);
}

#[test]
fn test_open_three_arg_or_die() {
    assert_clean_parse(r#"open(my $fh, '<', $file) or die "Cannot open: $!";"#);
}

#[test]
fn test_chdir_or_die() {
    assert_clean_parse(r#"chdir($dir) or die "Cannot chdir: $!";"#);
}

#[test]
fn test_mkdir_or_die() {
    assert_clean_parse(r#"mkdir($dir) or die "Cannot mkdir: $!";"#);
}

#[test]
fn test_system_and_die() {
    assert_clean_parse(r#"system($cmd) and die "Command failed";"#);
}

#[test]
fn test_eval_or_die() {
    assert_clean_parse(r#"eval { require Foo } or die "Cannot load Foo";"#);
}

#[test]
fn test_do_or_die() {
    assert_clean_parse(r#"do "config.pl" or die "Cannot load config";"#);
}

// === Word operators in control flow ===

#[test]
fn test_not_in_if_condition() {
    assert_clean_parse("if (not $done) { print 1; }");
}

#[test]
fn test_and_in_condition() {
    assert_clean_parse("if ($a and $b) { print 1; }");
}

#[test]
fn test_or_in_condition() {
    assert_clean_parse("if ($a or $b) { print 1; }");
}

// === Word operators with assignments ===

#[test]
fn test_or_with_assignment_both_sides() {
    assert_clean_parse("$a = 1 or $b = 2;");
}

#[test]
fn test_and_with_assignment_both_sides() {
    assert_clean_parse("$a = 1 and $b = 2;");
}

#[test]
fn test_not_with_assignment() {
    assert_clean_parse("$a = not 0;");
}

// === Chained word operators ===

#[test]
fn test_or_chain() {
    assert_clean_parse("$a or $b or $c;");
}

#[test]
fn test_and_chain() {
    assert_clean_parse("$a and $b and $c;");
}

#[test]
fn test_mixed_word_ops() {
    assert_clean_parse("$a and $b or $c;");
}

// === Word operators with function calls ===

#[test]
fn test_defined_or() {
    assert_clean_parse("defined $x or die;");
}

#[test]
fn test_ref_eq() {
    assert_clean_parse(r#"ref $obj eq 'HASH';"#);
}

#[test]
fn test_exists_or() {
    assert_clean_parse(r#"exists $hash{$key} or die "Key not found";"#);
}

// === Word operators as statement modifiers (low precedence) ===

#[test]
fn test_print_or_die() {
    assert_clean_parse(r#"print $fh "data" or die "write failed";"#);
}

#[test]
fn test_require_or_die() {
    assert_clean_parse(r#"require Carp or die "no Carp";"#);
}

// === Complex CPAN-style patterns ===

#[test]
fn test_open_filehandle_or_die() {
    assert_clean_parse(
        r#"open(my $fh, '<:encoding(UTF-8)', $filename) or die "Cannot open $filename: $!";"#,
    );
}

#[test]
fn test_socket_or_die() {
    assert_clean_parse(r#"socket(SOCK, PF_INET, SOCK_STREAM, $proto) or die "socket: $!";"#);
}

#[test]
fn test_bind_or_die() {
    assert_clean_parse(r#"bind(SOCK, $paddr) or die "bind: $!";"#);
}

#[test]
fn test_connect_or_die() {
    assert_clean_parse(r#"connect(SOCK, $paddr) or die "connect: $!";"#);
}

#[test]
fn test_not_defined() {
    assert_clean_parse("not defined $x;");
}

#[test]
fn test_not_not_expr() {
    assert_clean_parse("not not $x;");
}

#[test]
fn test_word_ops_with_parens() {
    assert_clean_parse("($a or $b) and ($c or $d);");
}

// === Patterns that match CPAN error buckets ===
// These test patterns where word operators appear after constructs that
// use is_at_statement_end() loops, causing "expected expression, found 'or'"

#[test]
fn test_map_block_or_die() {
    // block-list builtin followed by word operator
    assert_clean_parse(r#"my @result = map { $_->name } @items or die "No items";"#);
}

#[test]
fn test_grep_block_or_die() {
    assert_clean_parse(r#"my @found = grep { defined $_ } @list or die "Empty";"#);
}

#[test]
fn test_sort_block_or_die() {
    assert_clean_parse(r#"my @sorted = sort { $a cmp $b } @list or die "Sort failed";"#);
}

#[test]
fn test_map_expr_or_die() {
    assert_clean_parse(r#"my @result = map { $_ * 2 } @list or die "Failed";"#);
}

#[test]
fn test_push_or_die() {
    // push without parens, followed by or
    assert_clean_parse(r#"push @arr, $val or die "push failed";"#);
}

#[test]
fn test_unshift_or_die() {
    assert_clean_parse(r#"unshift @arr, $val or die "unshift failed";"#);
}

#[test]
fn test_splice_or_die() {
    assert_clean_parse(r#"splice @arr, 0, 1 or die "splice failed";"#);
}

#[test]
fn test_print_filehandle_or_die() {
    // print with filehandle
    assert_clean_parse(r#"print STDERR "error\n" or die "write failed";"#);
}

#[test]
fn test_printf_or_die() {
    assert_clean_parse(r#"printf "%s\n", $msg or die "printf failed";"#);
}

#[test]
fn test_say_or_die() {
    assert_clean_parse(r#"say $fh "data" or die "say failed";"#);
}

#[test]
fn test_chmod_or_die() {
    assert_clean_parse(r#"chmod 0644, $file or die "chmod failed: $!";"#);
}

#[test]
fn test_chown_or_die() {
    assert_clean_parse(r#"chown $uid, $gid, $file or die "chown failed: $!";"#);
}

#[test]
fn test_rename_or_die() {
    assert_clean_parse(r#"rename $old, $new or die "rename failed: $!";"#);
}

#[test]
fn test_unlink_or_die() {
    assert_clean_parse(r#"unlink $file or die "unlink failed: $!";"#);
}

#[test]
fn test_symlink_or_die() {
    assert_clean_parse(r#"symlink $old, $new or die "symlink failed: $!";"#);
}

#[test]
fn test_read_or_die() {
    assert_clean_parse(r#"read $fh, $buf, 1024 or die "read failed: $!";"#);
}

#[test]
fn test_seek_or_die() {
    assert_clean_parse(r#"seek $fh, 0, 0 or die "seek failed: $!";"#);
}

#[test]
fn test_truncate_or_die() {
    assert_clean_parse(r#"truncate $fh, 0 or die "truncate failed: $!";"#);
}

#[test]
fn test_syswrite_or_die() {
    assert_clean_parse(r#"syswrite $fh, $data or die "syswrite failed: $!";"#);
}

#[test]
fn test_sysread_or_die() {
    assert_clean_parse(r#"sysread $fh, $buf, 1024 or die "sysread failed: $!";"#);
}

#[test]
fn test_not_as_function_arg() {
    // not used inside an expression that is a function argument
    assert_clean_parse(r#"warn "error" if not $ok;"#);
}

#[test]
fn test_die_and_warn_pattern() {
    // Common CPAN: die ... and warn ...
    assert_clean_parse(r#"$ok and warn "success";"#);
}

#[test]
fn test_return_or_pattern() {
    assert_clean_parse(r#"return $val or die "no value";"#);
}

#[test]
fn test_word_or_after_method_call() {
    assert_clean_parse(r#"$obj->method() or die "method failed";"#);
}

#[test]
fn test_word_and_after_method_call() {
    assert_clean_parse(r#"$obj->can("method") and $obj->method();"#);
}

#[test]
fn test_word_not_in_list_context() {
    assert_clean_parse(r#"my @result = grep { not $seen{$_}++ } @list;"#);
}

#[test]
fn test_word_or_after_chained_calls() {
    assert_clean_parse(r#"$dbh->prepare($sql) or die $dbh->errstr;"#);
}

#[test]
fn test_word_and_or_combined() {
    assert_clean_parse(r#"open(my $fh, '<', $file) and print "opened" or die "failed";"#);
}

#[test]
fn test_write_or_die() {
    assert_clean_parse(r#"syswrite($sock, $data) or die "write: $!";"#);
}

#[test]
fn test_accept_or_die() {
    assert_clean_parse(r#"accept(CLIENT, SERVER) or die "accept: $!";"#);
}

#[test]
fn test_listen_or_die() {
    assert_clean_parse(r#"listen(SOCK, 5) or die "listen: $!";"#);
}

#[test]
fn test_flock_or_die() {
    assert_clean_parse(r#"flock($fh, 2) or die "flock: $!";"#);
}

// === Edge cases that might trigger "expected expression, found 'or'" ===

#[test]
fn test_sprintf_or_die() {
    assert_clean_parse(r#"sprintf "%s %s", $a, $b or die "sprintf failed";"#);
}

#[test]
fn test_wantarray_or_die() {
    assert_clean_parse("wantarray or die;");
}

#[test]
fn test_hash_deref_or_die() {
    assert_clean_parse(r#"$self->{attr} or die "no attr";"#);
}

#[test]
fn test_array_deref_or_die() {
    assert_clean_parse(r#"$self->[0] or die "empty";"#);
}

#[test]
fn test_hash_element_or_die() {
    assert_clean_parse(r#"$hash{key} or die "no key";"#);
}

#[test]
fn test_regex_match_or_die() {
    assert_clean_parse(r#"$str =~ /pattern/ or die "no match";"#);
}

#[test]
fn test_local_or_die() {
    assert_clean_parse(r#"local $_ = $val or die "no val";"#);
}

#[test]
fn test_my_or_die() {
    assert_clean_parse(r#"my $x = open(FH, $file) or die "fail";"#);
}

#[test]
fn test_ternary_or_die() {
    assert_clean_parse(r#"$a ? $b : $c or die;"#);
}

#[test]
fn test_scalar_deref_or() {
    assert_clean_parse(r#"$$ref or die;"#);
}

#[test]
fn test_complex_deref_or() {
    assert_clean_parse(r#"$hash->{key}[0] or die;"#);
}

#[test]
fn test_multiple_word_ops_chain() {
    assert_clean_parse(r#"$a or $b and $c or die;"#);
}

#[test]
fn test_word_or_after_heredoc() {
    assert_clean_parse(
        r#"print <<END or die;
Hello
END
"#,
    );
}

#[test]
fn test_word_or_in_for_loop() {
    assert_clean_parse(r#"for my $item (@list) { $item or next; }"#);
}

#[test]
fn test_word_and_in_while() {
    assert_clean_parse(r#"while ($line = <STDIN> and $line ne "quit\n") { print $line; }"#);
}

#[test]
fn test_chomp_or() {
    assert_clean_parse(r#"chomp(my $line = <STDIN>) or die;"#);
}

#[test]
fn test_chop_or() {
    assert_clean_parse(r#"chop $str or die;"#);
}

#[test]
fn test_eval_block_or() {
    assert_clean_parse(r#"eval { die "test" } or warn "caught: $@";"#);
}

#[test]
fn test_do_file_or() {
    assert_clean_parse(r#"do "config.pl" or die "can't load config: $@";"#);
}

#[test]
fn test_require_module_or() {
    assert_clean_parse(r#"require Foo::Bar or die "can't load Foo::Bar";"#);
}

#[test]
fn test_die_unless_and() {
    // die is itself a function, and `unless` is a modifier
    assert_clean_parse(r#"die "error" unless $ok and $ready;"#);
}

#[test]
fn test_not_ref_eq() {
    assert_clean_parse(r#"not ref($obj) eq 'HASH';"#);
}

#[test]
fn test_isa_and_can() {
    assert_clean_parse(r#"$obj->isa("Foo") and $obj->can("bar");"#);
}

#[test]
fn test_complex_or_die_with_interpolation() {
    assert_clean_parse(r#"open(my $fh, '<', $file) or die "Cannot open '$file': $!";"#);
}

#[test]
fn test_word_or_after_qw() {
    assert_clean_parse(r#"my @x = qw(a b c) or die;"#);
}

#[test]
fn test_word_not_in_ternary() {
    assert_clean_parse(r#"not $a ? 1 : 0;"#);
}

#[test]
fn test_pipe_open_or_die() {
    assert_clean_parse(r#"open(my $fh, '-|', 'ls') or die "pipe: $!";"#);
}

#[test]
fn test_binmode_or_die() {
    assert_clean_parse(r#"binmode($fh, ':utf8') or die "binmode: $!";"#);
}

#[test]
fn test_setsockopt_or_die() {
    assert_clean_parse(r#"setsockopt(SOCK, SOL_SOCKET, SO_REUSEADDR, 1) or die "setsockopt: $!";"#);
}

#[test]
fn test_fcntl_or_die() {
    assert_clean_parse(r#"fcntl($fh, F_SETFL, $flags) or die "fcntl: $!";"#);
}

#[test]
fn test_word_or_after_array_assignment() {
    assert_clean_parse(r#"my @list = split /,/, $str or die "split failed";"#);
}

#[test]
fn test_word_or_after_hash_access_chain() {
    assert_clean_parse(r#"$config->{database}{host} or die "no db host";"#);
}

#[test]
fn test_word_not_with_method() {
    assert_clean_parse(r#"not $obj->is_valid;"#);
}

#[test]
fn test_word_xor_basic_expression() {
    assert_clean_parse(r#"$a xor $b;"#);
}

#[test]
fn test_chained_or_with_die() {
    assert_clean_parse(r#"$a or $b or die "both failed";"#);
}

// === Block-list function patterns that might trigger word op errors ===

#[test]
fn test_sort_bare_list_or_die() {
    // sort without block, bare list, followed by or
    // This must NOT be misclassified as an indirect call.
    let ast = parse(r#"sort @list or die;"#);
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("(binary_or"),
        "expected binary_or at top level, got: {sexp}"
    );
    assert_clean_parse(r#"sort @list or die;"#);
}

#[test]
fn test_map_expr_list_or_die() {
    // map with expr (not block), followed by or
    assert_clean_parse(r#"map { $_ + 1 } @list or die;"#);
}

#[test]
fn test_grep_defined_list_or_die() {
    assert_clean_parse(r#"grep { defined } @list or die;"#);
}

#[test]
fn test_first_block_or_die() {
    assert_clean_parse(r#"first { $_ > 0 } @list or die;"#);
}

#[test]
fn test_any_block_or_die() {
    assert_clean_parse(r#"any { $_ > 0 } @list or die;"#);
}

#[test]
fn test_all_block_or_die() {
    assert_clean_parse(r#"all { $_ > 0 } @list or die;"#);
}

#[test]
fn test_none_block_or_die() {
    assert_clean_parse(r#"none { $_ > 0 } @list or die;"#);
}

#[test]
fn test_reduce_block_or_die() {
    assert_clean_parse(r#"reduce { $a + $b } @list or die;"#);
}

#[test]
fn test_sort_cmp_list_or_die() {
    assert_clean_parse(r#"sort { $a cmp $b } @list or die;"#);
}

#[test]
fn test_grep_regex_or_die() {
    assert_clean_parse(r#"grep /pattern/, @list or die;"#);
}

// === Patterns that might trigger cascading word op errors ===

#[test]
fn test_word_or_after_empty_return() {
    assert_clean_parse("return or die;");
}

#[test]
fn test_word_and_after_last() {
    assert_clean_parse("last and die;");
}

#[test]
fn test_word_or_in_hash_value() {
    assert_clean_parse(r#"my %h = (key => $val or "default");"#);
}

#[test]
fn test_word_or_as_default_value() {
    assert_clean_parse(r#"my $x = $ENV{HOME} or $ENV{LOGDIR} or "/tmp";"#);
}

#[test]
fn test_word_not_before_defined() {
    assert_clean_parse("if (not defined $x) { die; }");
}

#[test]
fn test_word_or_after_wantarray() {
    assert_clean_parse("return wantarray ? @result : $result[0] or die;");
}

#[test]
fn test_word_or_after_neg() {
    assert_clean_parse("!$x or die;");
}

#[test]
fn test_word_or_after_paren_group() {
    assert_clean_parse(r#"($a, $b) = ($c, $d) or die;"#);
}

#[test]
fn test_word_and_with_not() {
    assert_clean_parse("$a and not $b;");
}

#[test]
fn test_word_not_in_assignment() {
    assert_clean_parse("my $flag = not $condition;");
}

#[test]
fn test_word_or_in_list_assignment() {
    assert_clean_parse("my ($a, $b) = @_ or die;");
}

#[test]
fn test_word_or_after_sub_call_no_parens() {
    assert_clean_parse(r#"foo $bar or die;"#);
}

#[test]
fn test_word_or_in_complex_control() {
    assert_clean_parse(r#"open my $fh, '<', $file or do { warn "fail"; return; };"#);
}

#[test]
fn test_word_and_after_chained_method() {
    assert_clean_parse(r#"$obj->foo->bar and print "yes";"#);
}

#[test]
fn test_word_or_in_unless() {
    assert_clean_parse(r#"die "bad" unless $x or $y;"#);
}

#[test]
fn test_nested_not() {
    assert_clean_parse("if (not not not $x) { die; }");
}

#[test]
fn test_word_op_with_string_eq_chain() {
    assert_clean_parse(r#"$type eq 'foo' or $type eq 'bar' or die "unknown type";"#);
}

#[test]
fn test_word_op_after_array_ref() {
    assert_clean_parse(r#"my $ref = [1, 2, 3] or die;"#);
}

#[test]
fn test_word_op_after_hash_ref() {
    assert_clean_parse(r#"my $ref = {a => 1} or die;"#);
}

#[test]
fn test_word_op_after_anon_sub() {
    assert_clean_parse(r#"my $cb = sub { 1 } or die;"#);
}

#[test]
fn test_word_or_after_local_array() {
    assert_clean_parse(r#"local @_ = @args or die;"#);
}

#[test]
fn test_word_or_in_eval_string() {
    assert_clean_parse(r#"eval "1 + 1" or die "eval failed: $@";"#);
}

// === Repetition operator ===

#[test]
fn test_word_op_x_string_repetition() {
    assert_clean_parse("'x' x 5;");
}

#[test]
fn test_word_op_x_array_repetition() {
    assert_clean_parse("@list x 3;");
}

// === Complex expressions combining word operators ===

#[test]
fn test_word_op_return_with_eq_and() {
    assert_clean_parse("return 1 if $a eq 'test' and $b ne 'other';");
}

#[test]
fn test_word_op_ternary_with_gt() {
    assert_clean_parse("my $result = $x gt $y ? 'greater' : 'less';");
}

#[test]
fn test_word_op_print_with_not() {
    assert_clean_parse("print 'yes' if not $disabled;");
}
