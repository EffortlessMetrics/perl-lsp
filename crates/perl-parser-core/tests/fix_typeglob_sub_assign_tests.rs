mod cpan_test_helpers;
use cpan_test_helpers::*;

// *GLOB = sub { ... } — typeglob assignment with anonymous sub as RHS

#[test]
fn test_typeglob_assign_sub_no_proto() {
    assert_clean_parse(r#"*CLONE = sub { my $self = shift };"#);
}

#[test]
fn test_typeglob_assign_sub_with_proto() {
    assert_clean_parse(r#"*NEEDS_REGISTRY = sub () { $needs_registry };"#);
}

#[test]
fn test_typeglob_assign_ref_to_sub() {
    assert_clean_parse(r#"*foo = \&bar;"#);
}

#[test]
fn test_typeglob_assign_word_operator_name() {
    assert_clean_parse(r#"*or = \&any;"#);
}

#[test]
fn test_typeglob_dynamic_assign_sub() {
    assert_clean_parse(r#"*{$name} = sub { return 1 };"#);
}

// Hash::MultiValue.pm exact pattern — typeglob assignment inside BEGIN block
#[test]
fn test_typeglob_sub_inside_begin_block() {
    assert_clean_parse(
        r#"
BEGIN {
    my $needs_registry = 1;
    if ($needs_registry) {
        *CLONE = sub {
            foreach my $oldaddr (keys %registry) {
                my $this = 1;
            }
        };
    }
    *NEEDS_REGISTRY = sub () { $needs_registry };
}
"#,
    );
}
