mod cpan_test_helpers;
use cpan_test_helpers::*;

// === Sub-Pattern A: No-arg filetest / builtin before && / || ===
// Filetest operators like -f, -d, -w used without explicit operand
// followed by a short-circuit operator should implicitly use $_.

#[test]
fn test_filetest_no_arg_before_and() {
    // -f without operand followed by && should treat $_ as implicit operand
    assert_clean_parse("-f && -d;");
}

#[test]
fn test_filetest_no_arg_before_or() {
    // -f without operand followed by || should treat $_ as implicit operand
    assert_clean_parse("next if -f || -d;");
}

#[test]
fn test_filetest_no_arg_before_defined_or() {
    // -f without operand followed by // (defined-or) should treat $_ as implicit
    assert_clean_parse("-f // die;");
}

#[test]
fn test_next_unless_filetest_chain() {
    // Common CPAN pattern: next unless -d && -w
    assert_clean_parse("next unless -d && -w _;");
}

#[test]
fn test_filetest_in_grep_context() {
    // grep with filetest and logical operator
    assert_clean_parse("grep -f && -d, @list;");
}

#[test]
fn test_ord_before_comparison() {
    // ord without args followed by comparison operator
    assert_clean_parse("ord >= 32;");
}

#[test]
fn test_length_before_comparison() {
    // length without args followed by comparison operator
    assert_clean_parse("length > 0;");
}

#[test]
fn test_defined_before_and() {
    // defined without args followed by &&
    assert_clean_parse("grep defined && length, @list;");
}

#[test]
fn test_defined_or_die() {
    // defined without args followed by ||
    assert_clean_parse("defined || die;");
}

// === Sub-Pattern B: Special variable $: and friends ===
// Perl punctuation variables $:, $;, $, that the parser did not handle.

#[test]
fn test_special_var_colon() {
    // $: is Perl's format line-break character variable
    assert_clean_parse(r#"my $prev = $:;"#);
}

#[test]
fn test_special_var_colon_assign() {
    // Assigning to $:
    assert_clean_parse(r#"$: = " -";"#);
}

#[test]
fn test_special_var_colon_local() {
    // local $: — common IO::Handle pattern
    assert_clean_parse(r#"local $: = " ";"#);
}

#[test]
fn test_special_var_semicolon() {
    // $; is Perl's subscript separator variable
    assert_clean_parse(r#"my $sep = $;;"#);
}

// === Sub-Pattern C: Typeglob with caret-prefixed name *^N ===
// English.pm uses *^N to alias control variables like $^N.

#[test]
fn test_typeglob_caret_name() {
    // *^N is a typeglob for the $^N control variable
    assert_clean_parse("*LAST = *^N;");
}

#[test]
fn test_typeglob_caret_name_w() {
    // *^W is the typeglob for $^W (warnings flag)
    assert_clean_parse("*LAST_INPUT_LINE_NUMBER = *^W;");
}

#[test]
fn test_typeglob_caret_name_f() {
    // *^F is the typeglob for $^F (system file descriptor)
    assert_clean_parse("*FORMAT_NAME = *^F;");
}

#[test]
fn test_typeglob_dash_subscript() {
    // *-{ARRAY} is a glob for the @- (LAST_MATCH_START) array via subscript
    assert_clean_parse("*MATCH_START = *-{ARRAY};");
}

#[test]
fn test_typeglob_plus_subscript() {
    // *+{ARRAY} is a glob for the @+ (LAST_MATCH_END) array via subscript
    assert_clean_parse("*MATCH_END = *+{ARRAY};");
}

// === Sub-Pattern D: __END__ / __DATA__ after no-semicolon statement ===
// A statement like __PACKAGE__ without trailing semicolon followed by
// __END__ should parse cleanly — the __END__ terminates the program.

#[test]
fn test_end_marker_after_package_no_semicolon() {
    // __PACKAGE__ as module return value (no semicolon) + __END__
    assert_clean_parse("__PACKAGE__\n__END__\n");
}

#[test]
fn test_data_marker_after_expression_no_semicolon() {
    // Integer literal without semicolon before __DATA__
    assert_clean_parse("1\n__DATA__\nsome data here\n");
}

#[test]
fn test_end_marker_after_one_no_semicolon() {
    // Classic Perl module ending: `1` without semicolon before __END__
    assert_clean_parse("1\n__END__\n");
}

#[test]
fn test_end_marker_with_pod() {
    // __END__ followed by POD documentation
    assert_clean_parse("__PACKAGE__\n__END__\n\n=pod\n\nSome docs.\n\n=cut\n");
}

// === Regression guards: explicit-arg forms must still parse correctly ===
// The optional-arg builtin guard only fires when followed by a binary operator.
// When an explicit argument is present, parsing should proceed as before.

#[test]
fn test_length_with_explicit_arg() {
    // length $str — explicit bare arg at statement start must still be consumed
    assert_clean_parse("length $str;");
}

#[test]
fn test_length_with_parens() {
    // length($str) — parens bypass the optional-arg path entirely
    assert_clean_parse("length($str);");
}

#[test]
fn test_defined_with_explicit_arg() {
    // defined $x — explicit arg must still work at statement start
    assert_clean_parse("defined $x;");
}

#[test]
fn test_defined_with_parens() {
    // defined($x) — parens bypass the optional-arg path
    assert_clean_parse("defined($x);");
}

#[test]
fn test_log_with_explicit_arg() {
    // log $x — log newly routed through parse_named_unary_statement_call
    assert_clean_parse("log $x;");
}

#[test]
fn test_abs_int_explicit_arg() {
    // abs and int with explicit args must still consume their argument
    assert_clean_parse("abs $n;");
    assert_clean_parse("int $n;");
}

#[test]
fn test_hex_oct_explicit_arg() {
    // hex and oct with explicit args
    assert_clean_parse("hex $s;");
    assert_clean_parse("oct $s;");
}

#[test]
fn test_special_var_colon_comparison() {
    // $: used in a comparison expression, not just assignment
    assert_clean_parse(r#"$: eq " -";"#);
}

#[test]
fn test_typeglob_caret_in_expression() {
    // *^N used as an rvalue in a more complex expression
    assert_clean_parse("my $g = *^N;");
}

#[test]
fn test_end_marker_with_semicolon() {
    // __END__ after a properly semicolon-terminated statement — pre-existing path
    assert_clean_parse("1;\n__END__\n");
}

// === Edge cases: Pattern A — nested / chained constructs ===

#[test]
fn test_filetest_chained_three_way() {
    // Three-way filetest chain: -f && -d && -w all using implicit $_
    assert_clean_parse("-f && -d && -w;");
}

#[test]
fn test_filetest_inside_if_condition() {
    // Filetest no-arg inside an if condition
    assert_clean_parse("if (-f && -d) { do_something(); }");
}

#[test]
fn test_defined_in_ternary() {
    // defined without args in ternary condition: defined ? $x : $y
    assert_clean_parse("my $r = defined ? $x : $y;");
}

#[test]
fn test_length_in_string_comparison() {
    // length without args in a string comparison chain
    assert_clean_parse("length == 0 || length > 100;");
}

#[test]
fn test_optional_arg_builtin_in_grep_block() {
    // Multiple optional-arg builtins inside a grep block
    assert_clean_parse("grep { defined && length } @list;");
}

#[test]
fn test_optional_arg_builtin_map_chain() {
    // map with optional-arg builtins
    assert_clean_parse("my @r = map { length > 0 ? uc : lc } @list;");
}

#[test]
fn test_ord_in_range_expr() {
    // ord without args used in a range expression
    assert_clean_parse("my @r = (ord .. 127);");
}

// === Edge cases: Pattern A — explicit arg followed by binary op ===

#[test]
fn test_length_explicit_arg_then_op() {
    // length WITH explicit arg followed by comparison — arg must be consumed first
    assert_clean_parse("length($str) > 0;");
}

#[test]
fn test_defined_explicit_arg_then_and() {
    // defined WITH explicit arg followed by && — parens delimit, && is binary
    assert_clean_parse("defined($x) && do_it();");
}

// === Edge cases: Pattern B — $: in complex contexts ===

#[test]
fn test_special_var_colon_in_hash() {
    // $: as a hash value
    assert_clean_parse(r#"my %fmt = (sep => $:);"#);
}

#[test]
fn test_special_var_colon_interpolated() {
    // $: inside a double-quoted string
    assert_clean_parse(r#"my $s = "sep: $:";"#);
}

#[test]
fn test_special_var_colon_as_arg() {
    // $: passed as a function argument
    assert_clean_parse(r#"print $:;"#);
}

// === Edge cases: Pattern C — *^N variants and nesting ===

#[test]
fn test_typeglob_caret_multiline() {
    // Multiple *^X assignments — pattern from English.pm
    assert_clean_parse("*PREMATCH = *^PREMATCH;\n*MATCH = *^N;\n*POSTMATCH = *^POSTMATCH;\n");
}

#[test]
fn test_typeglob_caret_in_array() {
    // *^N in a list context
    assert_clean_parse("my @globs = (*^N, *^W, *^F);");
}

// === Edge cases: Pattern D — __END__ / __DATA__ in various positions ===

#[test]
fn test_end_marker_after_sub_definition() {
    // __END__ immediately after a sub definition without trailing semicolon
    assert_clean_parse("sub foo { 1 }\n__END__\n");
}

#[test]
fn test_data_marker_with_content_lines() {
    // __DATA__ with multi-line content
    assert_clean_parse("1;\n__DATA__\nline1\nline2\nline3\n");
}

#[test]
fn test_end_marker_empty_body() {
    // __END__ at the very end of file with nothing after
    assert_clean_parse("1;\n__END__");
}

// === Edge cases: $: must not shadow $:: (package stash variable) ===

#[test]
fn test_dollar_doublecolon_not_shadowed() {
    // $:: is the main package stash — DoubleColon arm must still fire before Colon arm
    assert_clean_parse("my $x = $::foo;");
}

#[test]
fn test_dollar_colon_and_doublecolon_coexist() {
    // $: and $::foo in the same statement must both parse correctly
    assert_clean_parse(r#"my $a = $:; my $b = $::foo;"#);
}

// === Edge cases: Pattern A — builtin followed by dot (string concat) ===

#[test]
fn test_length_before_dot_concat() {
    // length without args before . — dot is in is_binary_operator, so length() . "x"
    assert_clean_parse(r#"length . "suffix";"#);
}

#[test]
fn test_defined_before_range() {
    // defined without args before .. range operator
    assert_clean_parse("my @r = (defined .. 10);");
}

// === Edge cases: Interaction between Pattern A and explicit-arg in expression ===

#[test]
fn test_length_no_arg_inside_paren_expr() {
    // length without args inside a parenthesized condition
    assert_clean_parse("my $ok = (length > 0 && defined);");
}

#[test]
fn test_chr_no_arg_in_ternary() {
    // chr without args (uses $_ as character code) in ternary
    assert_clean_parse("my $c = defined ? chr : undef;");
}

// === Edge cases: Pattern D — __END__ / __DATA__ interaction with expressions ===

#[test]
fn test_end_after_complex_expression() {
    // Complex expression without semicolon before __END__
    assert_clean_parse("my $x = 1 + 2\n__END__\n");
}

#[test]
fn test_end_after_hash_return() {
    // Common module pattern: return hash ref without semicolon before __END__
    assert_clean_parse("{ foo => 1, bar => 2 }\n__END__\n");
}
