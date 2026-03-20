mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_typeglob_scalar_deref_hash() {
    assert_clean_parse(r"*$self->{field} = 'auto'");
}

#[test]
fn test_typeglob_scalar_deref_chain() {
    assert_clean_parse(r"*$self->{fh}->write($buf)");
}

#[test]
fn test_typeglob_dynamic_brace() {
    assert_clean_parse(r"*{$pkg . '::' . $name} = $code");
}

#[test]
fn test_typeglob_scalar_deref_method() {
    assert_clean_parse(r"*$self->{Compress}->crc32()");
}
