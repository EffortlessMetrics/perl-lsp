/// Tests for `croak __PACKAGE__, "message"` — using __PACKAGE__ as a list
/// element (not the sole argument) in error-reporting calls.
/// Encode/Encoder.pm: `croak __PACKAGE__, ": unknown encoding: $encname"`
mod cpan_test_helpers;
use cpan_test_helpers::*;

/// The exact Encoder.pm pattern: croak with __PACKAGE__ and a string.
#[test]
fn test_croak_package_comma_string() {
    assert_clean_parse(
        r#"my $obj = find_encoding($encname) or croak __PACKAGE__, ": unknown encoding: $encname";"#,
    );
}

/// Simpler form: croak __PACKAGE__, "msg"
#[test]
fn test_croak_package_as_list_element() {
    assert_clean_parse(r#"croak __PACKAGE__, ": error";"#);
}

/// die with __PACKAGE__ and concat.
#[test]
fn test_die_package_concat() {
    assert_clean_parse(r#"die __PACKAGE__ . ": fatal error\n";"#);
}

/// __PACKAGE__ used as first arg in function call (parens form).
#[test]
fn test_croak_package_in_parens() {
    assert_clean_parse(r#"croak(__PACKAGE__ . ": error");"#);
}

/// __PACKAGE__ in a list context assignment.
#[test]
fn test_package_in_list() {
    assert_clean_parse(r#"my @info = (__PACKAGE__, __FILE__, __LINE__);"#);
}

/// warn with __PACKAGE__ and multiple args.
#[test]
fn test_warn_package_multi_args() {
    assert_clean_parse(r#"warn __PACKAGE__, ": something went wrong at ", __FILE__, "\n";"#);
}

/// __SUB__ as a bare-call argument (coderef to current sub).
#[test]
fn test_sub_as_argument() {
    assert_clean_parse(r#"Scalar::Util::weaken(my $weak = __SUB__);"#);
}

/// __PACKAGE__ used in string interpolation context must still work.
#[test]
fn test_package_in_sprintf() {
    assert_clean_parse(
        r#"my $msg = sprintf "%s: error at %s line %d", __PACKAGE__, __FILE__, __LINE__;"#,
    );
}
