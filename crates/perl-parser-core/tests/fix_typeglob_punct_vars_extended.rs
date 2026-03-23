mod cpan_test_helpers;
use cpan_test_helpers::*;

// Extended typeglob punctuation variable tests.
// Covers the remaining punct typeglob forms that appeared in CPAN corpus files
// (English.pm, ExtUtils/MM_Unix.pm, File/Copy.pm, IPC/Cmd.pm, MIME/Lite.pm, etc.)
// after the initial `*<`, `*>`, `*(`, `*)` fix in fix_typeglob_punct_vars.rs.
//
// These 7 forms were still failing:
//   */ *. *, *= *| *? *:
//
// Token mapping:
//   */ → TokenKind::Slash   (lookahead: terminated by ; , ) } ] EOF)
//   *. → TokenKind::Dot     (lookahead: same)
//   *, → TokenKind::Comma   (no lookahead: comma cannot start an expression)
//   *= → TokenKind::Assign  (no lookahead: *=operator is StarAssign, not Star+Assign)
//   *| → TokenKind::BitwiseOr (lookahead: same as Slash)
//   *? → TokenKind::Question  (no lookahead: ? cannot start an expression after *)
//   *: → TokenKind::Colon   (lookahead: same as Slash)

// === */ — input record separator ($/) ===

#[test]
fn test_typeglob_slash_rs() {
    // *RS = */; — English.pm pattern
    assert_clean_parse("*RS = */;");
}

#[test]
fn test_typeglob_slash_alias() {
    // *INPUT_RECORD_SEPARATOR = */;
    assert_clean_parse("*INPUT_RECORD_SEPARATOR = */;");
}

#[test]
fn test_typeglob_slash_in_block() {
    // */ inside a conditional block
    assert_clean_parse("{ *RS = */; }");
}

// === *. — input line number ($.) ===

#[test]
fn test_typeglob_dot_nr() {
    // *NR = *.; — English.pm pattern
    assert_clean_parse("*NR = *.;");
}

#[test]
fn test_typeglob_dot_input_line() {
    // *INPUT_LINE_NUMBER = *.;
    assert_clean_parse("*INPUT_LINE_NUMBER = *.;");
}

// === *, — output field separator ($,) ===

#[test]
fn test_typeglob_comma_ofs() {
    // *OFS = *,; — English.pm pattern
    assert_clean_parse("*OFS = *,;");
}

#[test]
fn test_typeglob_comma_output_field_sep() {
    // *OUTPUT_FIELD_SEPARATOR = *,;
    assert_clean_parse("*OUTPUT_FIELD_SEPARATOR = *,;");
}

#[test]
fn test_typeglob_comma_in_list() {
    // *, used as the last element in a list (before closing paren)
    assert_clean_parse("my @g = (*,, *|);");
}

// === *= — format lines per page ($=) ===

#[test]
fn test_typeglob_assign_format_lines() {
    // *FORMAT_LINES_PER_PAGE = *=;
    assert_clean_parse("*FORMAT_LINES_PER_PAGE = *=;");
}

#[test]
fn test_typeglob_assign_in_block() {
    // *= inside a hash
    assert_clean_parse("my %g = (lines => *=);");
}

// === *| — output autoflush ($|) ===

#[test]
fn test_typeglob_pipe_autoflush() {
    // *OUTPUT_AUTOFLUSH = *|; — English.pm pattern
    assert_clean_parse("*OUTPUT_AUTOFLUSH = *|;");
}

#[test]
fn test_typeglob_pipe_in_paren() {
    // *| inside a parenthesised list
    assert_clean_parse("my @g = (*|);");
}

// === *? — child process status ($?) ===

#[test]
fn test_typeglob_question_child_error() {
    // *CHILD_ERROR = *?; — English.pm pattern
    assert_clean_parse("*CHILD_ERROR = *?;");
}

#[test]
fn test_typeglob_question_in_list() {
    // *? in a list
    assert_clean_parse("my @g = (*?, *!);");
}

// === *: — format line-break characters ($:) ===

#[test]
fn test_typeglob_colon_format_linebreak() {
    // *FORMAT_LINE_BREAK_CHARACTERS = *:; — English.pm pattern
    assert_clean_parse("*FORMAT_LINE_BREAK_CHARACTERS = *:;");
}

#[test]
fn test_typeglob_colon_in_paren() {
    // *: inside a parenthesised list
    assert_clean_parse("my @g = (*:);");
}

// === Comprehensive English.pm-style block ===

#[test]
fn test_english_pm_output_vars() {
    // Output variable aliases from English.pm
    assert_clean_parse(
        "*OUTPUT_AUTOFLUSH = *|;\n\
         *OUTPUT_FIELD_SEPARATOR = *,;\n\
         *OUTPUT_RECORD_SEPARATOR = *\\;",
    );
}

#[test]
fn test_english_pm_input_vars() {
    // Input variable aliases from English.pm
    assert_clean_parse(
        "*INPUT_LINE_NUMBER = *.;\n\
         *NR = *.;\n\
         *INPUT_RECORD_SEPARATOR = */;\n\
         *RS = */;",
    );
}

#[test]
fn test_english_pm_format_vars() {
    // Format-related variable aliases from English.pm
    assert_clean_parse(
        "*FORMAT_LINES_PER_PAGE = *=;\n\
         *FORMAT_LINE_BREAK_CHARACTERS = *:;\n\
         *FORMAT_PAGE_NUMBER = *%;",
    );
}

#[test]
fn test_english_pm_error_vars() {
    // Error variable aliases
    assert_clean_parse("*CHILD_ERROR = *?;\n*OS_ERROR = *!;\n*EVAL_ERROR = *@;");
}

// === Regression guards: operators using same tokens must still work ===

#[test]
fn test_multiply_then_regex_not_typeglob() {
    // $x * /pattern/ should NOT be confused with */ typeglob
    // After fix: */ is only typeglob when followed by ; , ) } ] EOF
    // (this tests that */ NOT at a terminator falls through to binary operator)
    assert_clean_parse("my $n = 2 * /foo/ ? 1 : 0;");
}

#[test]
fn test_multiply_then_string_not_typeglob() {
    // $x * "str" should not be confused with *. typeglob
    assert_clean_parse(r#"my $n = 2 * "3";"#);
}

#[test]
fn test_comma_as_list_separator_not_typeglob() {
    // List separator comma must not be confused with *, typeglob in complex context
    assert_clean_parse("my @a = (1, 2, 3);");
}

#[test]
fn test_assign_compound_star_still_works() {
    // *= as StarAssign compound assignment must still be parsed correctly
    assert_clean_parse("my $x = 1; $x *= 2;");
}

#[test]
fn test_bitwise_or_not_typeglob() {
    // Bitwise OR used as binary operator: must not be confused with *| typeglob
    assert_clean_parse("my $n = $a | $b;");
}

#[test]
fn test_question_in_ternary_not_typeglob() {
    // Ternary ? must still work
    assert_clean_parse("my $x = 1 ? 2 : 3;");
}

#[test]
fn test_colon_in_ternary_not_typeglob() {
    // Ternary colon must still work
    assert_clean_parse("my $y = $ok ? 'yes' : 'no';");
}

#[test]
fn test_slash_in_regex_not_typeglob() {
    // Regex /pattern/ must not be confused with */ typeglob
    assert_clean_parse("my @m = grep { /foo/ } @list;");
}

// === Edge cases ===

#[test]
fn test_typeglob_slash_in_array_ref() {
    // */ inside an anonymous arrayref
    assert_clean_parse("my $r = [*/];");
}

#[test]
fn test_typeglob_dot_in_array_ref() {
    // *. inside an anonymous arrayref
    assert_clean_parse("my $r = [*.];");
}

#[test]
fn test_typeglob_pipe_in_array_ref() {
    // *| inside an anonymous arrayref
    assert_clean_parse("my $r = [*|];");
}

#[test]
fn test_typeglob_colon_in_array_ref() {
    // *: inside an anonymous arrayref
    assert_clean_parse("my $r = [*:];");
}

#[test]
fn test_typeglob_comma_before_closing_brace() {
    // *, as a hash value (before closing brace)
    assert_clean_parse("my %h = (sep => *,);");
}

#[test]
fn test_typeglob_question_before_closing_bracket() {
    // *? before ]
    assert_clean_parse("my $r = [*?];");
}

#[test]
fn test_typeglob_assign_before_closing_paren() {
    // *= before )
    assert_clean_parse("foo(*=);");
}

// local(*/) pattern
#[test]
fn test_local_typeglob_slash() {
    assert_clean_parse("local(*RS) = local(*/);");
}

// Multiple punct typeglob assignments in a single statement
#[test]
fn test_multiple_punct_typeglobs_same_statement() {
    assert_clean_parse("(*RS, *OFS) = (*/, *,);");
}

// === Additional edge cases added by deep review ===

// *= as lvalue: typeglob *= can appear on the left side of an assignment.
// *other = *= aliases the FORMAT_LINES_PER_PAGE slot of *other.
#[test]
fn test_typeglob_assign_as_lvalue() {
    assert_clean_parse("*other = *=;");
}

// */ at end of file with no trailing semicolon.
// Exercises the None / Eof branch of is_typeglob_punct_terminator.
#[test]
fn test_typeglob_slash_at_eof() {
    assert_clean_parse("*RS = */");
}

// *. as a list element followed by a comma separator.
// Comma is listed as a terminator for the lookahead forms; this confirms it.
#[test]
fn test_typeglob_dot_followed_by_comma_in_list() {
    assert_clean_parse("my @g = (*., *,);");
}

// *: inside a hash value position (RightParen as terminator for the lookahead).
#[test]
fn test_typeglob_colon_in_hash_value() {
    assert_clean_parse("my %h = (linebreak => *:);");
}

// Whitespace and newline between */ and ; must not defeat the lookahead.
// Token stream skips trivia, so peek_second sees the semicolon directly.
#[test]
fn test_typeglob_slash_with_newline_before_semi() {
    assert_clean_parse("*RS = */\n;");
}

// *| followed by } — RightBrace as terminator for BitwiseOr lookahead.
#[test]
fn test_typeglob_pipe_before_closing_brace() {
    assert_clean_parse("my %h = (flush => *|);");
}

// Regression: $x * $y / $z stays as multiply-then-divide, not */ typeglob.
// The * here is infix and never reaches the unary Star arm.
#[test]
fn test_multiply_divide_chain_not_typeglob() {
    assert_clean_parse("my $n = $x * $y / $z;");
}
