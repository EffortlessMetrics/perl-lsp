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
fn test_op_private_deref_hash_slice_assignment() {
    // From B::Op_private: generated bitfield tables assign to hash slices
    // through a braced dereference whose inner key can be a builtin name.
    assert_clean_parse(
        r#"@{$bits{tie}}{3,2,1,0} = (
    'OPpASSIGN_COMMON_SCALAR',
    'OPpASSIGN_COMMON_RC1',
    'OPpASSIGN_COMMON_AGG',
    'OPpASSIGN_TRUEBOOL',
);"#,
    );
}

#[test]
fn test_op_private_untie_hash_key_assignment() {
    // From B::Op_private: `untie` is a builtin at expression start but a bare
    // hash key inside `$bits{untie}`.
    assert_clean_parse(r#"$bits{untie}{0} = $bf[0];"#);
}

#[test]
fn test_dpkg_grep_block_values_hash_deref() {
    // From Dpkg::Shlibs::Objdump::Object: grep block-list calls may take a
    // values expression over a braced hash dereference as their list operand.
    assert_clean_parse(
        r#"sub get_exported_dynamic_symbols {
    my $self = shift;
    return grep {
        $_->{defined} && $_->{dynamic} && !$_->{local}
    } values %{$self->{dynsyms}};
}"#,
    );
}
