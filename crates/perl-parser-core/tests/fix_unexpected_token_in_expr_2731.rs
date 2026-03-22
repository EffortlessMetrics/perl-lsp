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
