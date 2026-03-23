mod cpan_test_helpers;
use cpan_test_helpers::*;

// Test typeglob aliases for Perl's punctuation special variables
// These appear in English.pm (core Perl module)

// Pattern A: *< = typeglob for $< (real UID)
#[test]
fn test_typeglob_less_than_real_uid() {
    assert_clean_parse("*REAL_USER_ID = *<;");
}

#[test]
fn test_typeglob_less_than_uid_alias() {
    assert_clean_parse("*UID = *<;");
}

// Pattern B: *> = typeglob for $> (effective UID)
#[test]
fn test_typeglob_greater_than_euid() {
    assert_clean_parse("*EFFECTIVE_USER_ID = *>;");
}

#[test]
fn test_typeglob_greater_than_euid_alias() {
    assert_clean_parse("*EUID = *>;");
}

// Pattern C: *( = typeglob for $( (real GID)
#[test]
fn test_typeglob_open_paren_real_gid() {
    assert_clean_parse("*REAL_GROUP_ID = *(;");
}

#[test]
fn test_typeglob_open_paren_gid_alias() {
    assert_clean_parse("*GID = *(;");
}

// Pattern D: *) = typeglob for $) (effective GID)
#[test]
fn test_typeglob_close_paren_egid() {
    assert_clean_parse("*EFFECTIVE_GROUP_ID = *);");
}

#[test]
fn test_typeglob_close_paren_egid_alias() {
    assert_clean_parse("*EGID = *);");
}

// Multiple aliases in the same block (English.pm pattern)
#[test]
fn test_english_pm_uid_gid_block() {
    assert_clean_parse(
        "*REAL_USER_ID = *<;\n\
         *UID = *<;\n\
         *EFFECTIVE_USER_ID = *>;\n\
         *EUID = *>;\n\
         *REAL_GROUP_ID = *(;\n\
         *GID = *(;\n\
         *EFFECTIVE_GROUP_ID = *);\n\
         *EGID = *);",
    );
}

// Regression: existing typeglob patterns must still work
#[test]
fn test_typeglob_named_regression() {
    assert_clean_parse("*OS_ERROR = *!;");
}

#[test]
fn test_typeglob_eval_error_regression() {
    assert_clean_parse("*EVAL_ERROR = *@;");
}

#[test]
fn test_typeglob_process_id_regression() {
    assert_clean_parse("*PROCESS_ID = *$;");
}

#[test]
fn test_typeglob_caret_regression() {
    assert_clean_parse("*LAST_SUBMATCH_RESULT = *^N;");
}

#[test]
fn test_typeglob_identifier_regression() {
    assert_clean_parse("*FOO = *BAR;");
}

#[test]
fn test_typeglob_in_local_regression() {
    assert_clean_parse("local (*TO_CHLD_R, *TO_CHLD_W);");
}

// Disambiguation: *<EXPR> must NOT be parsed as a typeglob.
// The 2-token lookahead must detect that an expression follows the `<`,
// and fall through to let the operand be parsed as a readline/glob.
#[test]
fn test_star_readline_not_typeglob() {
    // *<STDIN> = glob dereference through a readline, not a typeglob named "<"
    assert_clean_parse(r#"my $line = *<STDIN>;"#);
}

#[test]
fn test_star_diamond_not_typeglob() {
    // *<> = dereference of the diamond operator, not a typeglob named "<"
    assert_clean_parse(r#"my $x = *<>;"#);
}

// Typeglob in list context (before closing bracket or brace)
#[test]
fn test_typeglob_less_in_arrayref() {
    // *< inside an anonymous arrayref: [*<]
    assert_clean_parse(r#"my $r = [*<];"#);
}

#[test]
fn test_typeglob_greater_in_arrayref() {
    assert_clean_parse(r#"my $r = [*>];"#);
}

// *( inside a hash value position (before closing brace)
#[test]
fn test_typeglob_open_paren_in_hash() {
    assert_clean_parse(r#"my %h = (gid => *();"#);
}

// sort map is the same pattern as sort grep — must not be misread as a comparator
#[test]
fn test_sort_map_not_comparator() {
    assert_clean_parse(r#"my @x = sort map { uc($_) } @list;"#);
}
