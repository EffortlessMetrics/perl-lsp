mod cpan_test_helpers;
use cpan_test_helpers::*;

// Regression tests for issue #2404 — unexpected_rbrace_expr
// These patterns already parse correctly; tests guard against regression.

// Empty anonymous hash ref — already handled by parse_hash_or_block_inner lines 48-58
#[test]
fn test_empty_hash_ref_bare() {
    assert_clean_parse("my $h = {};");
}

#[test]
fn test_empty_hash_ref_list() {
    assert_clean_parse("my @a = ({}, {});");
}

#[test]
fn test_empty_hash_ref_nested() {
    assert_clean_parse("my $x = { a => {} };");
}

#[test]
fn test_empty_hash_ref_bless() {
    assert_clean_parse("bless {}, 'Foo';");
}

#[test]
fn test_empty_hash_ref_ternary() {
    assert_clean_parse("my $x = $c ? {} : undef;");
}

#[test]
fn test_empty_hash_ref_or() {
    assert_clean_parse("my $x = $y // {};");
}

#[test]
fn test_empty_hash_ref_return() {
    assert_clean_parse("sub f { return {}; }");
}

#[test]
fn test_empty_hash_ref_in_sub_body() {
    assert_clean_parse("sub new { my $self = {}; bless $self, 'Foo'; }");
}

// Empty do-block — already handled by parse_block() loop
#[test]
fn test_do_empty_block_stmt() {
    assert_clean_parse("do {};");
}

#[test]
fn test_do_empty_block_assign() {
    assert_clean_parse("my $x = do {};");
}

#[test]
fn test_do_empty_block_condition() {
    assert_clean_parse("if (do {}) { }");
}

#[test]
fn test_do_empty_block_with_space() {
    assert_clean_parse("do { };");
}

#[test]
fn test_dbix_hash_splice_with_semicolon_terminated_deref_body() {
    // From DBIx::Class::Storage::DBIHacks: a hash constructor may splice a
    // hash dereference whose braced expression is terminated with `;`.
    assert_clean_parse(
        r#"sub x {
    my $return = {
        %{
            $colinfos->{$source_alias}->{$colname}
              ||
            $self->throw_exception("No such column");
        },
        -result_source => $rsrc,
    };
}"#,
    );
}

#[test]
fn test_file_find_wanted_sub_with_no_arg_filetest_if_else() {
    let source = r#"
find({
    no_chdir => 1,
    wanted => sub {
        if (-s) {
            unlink($target);
        } else {
            unlink($target);
        }
    }
}, $patch_dir);
"#;
    assert_clean_parse(source);
}
