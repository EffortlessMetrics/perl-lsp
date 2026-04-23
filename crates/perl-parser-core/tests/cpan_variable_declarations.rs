//! CPAN Pattern Tests: Variable Declarations

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn my_scalar() {
    let code = "my $x = 42;";
    assert_clean_parse(code);
}

#[test]
fn my_array() {
    let code = "my @items = (1, 2, 3);";
    assert_clean_parse(code);
}

#[test]
fn my_hash() {
    let code = "my %opts = (verbose => 1, debug => 0, output => 'file.txt');";
    assert_clean_parse(code);
}

#[test]
fn our_variable() {
    let code = "our $VERSION = '1.23';";
    assert_clean_parse(code);
}

#[test]
fn local_variable() {
    let code = "local $/ = undef;";
    assert_clean_parse(code);
}

#[test]
fn state_variable() {
    let code = "state $count = 0;";
    assert_clean_parse(code);
}

#[test]
fn list_assignment() {
    let code = "my ($first, @rest) = @ARGV;";
    assert_clean_parse(code);
}

#[test]
fn multiple_my_in_list() {
    let code = "my ($x, $y, $z) = (1, 2, 3);";
    assert_clean_parse(code);
}

#[test]
fn anonymous_sub_assignment() {
    let code = "my $cb = sub { return $_[0] + 1 };";
    assert_clean_parse(code);
}

#[test]
fn ternary_initializer() {
    let code = "my $val = defined($x) ? $x : 'default';";
    assert_clean_parse(code);
}

#[test]
fn wantarray_pattern() {
    let code = "my @result = wantarray() ? @list : ($list[0]);";
    assert_clean_parse(code);
}

#[test]
fn chomp_with_readline() {
    let code = "chomp(my $line = <STDIN>);";
    assert_clean_parse(code);
}

// ---------------------------------------------------------------------------
// undef as placeholder in my list destructuring
// ---------------------------------------------------------------------------

#[test]
fn undef_middle_of_my_list() {
    let code = "my ($a, undef, $b) = @_;";
    assert_clean_parse(code);
}

#[test]
fn undef_first_in_my_list() {
    let code = "my (undef, $x, $y) = @_;";
    assert_clean_parse(code);
}

#[test]
fn undef_last_in_my_list() {
    let code = "my ($a, $b, undef) = @_;";
    assert_clean_parse(code);
}

#[test]
fn multiple_undef_in_my_list() {
    let code = "my ($a, undef, undef, $b) = @_;";
    assert_clean_parse(code);
}

#[test]
fn undef_in_method_signature() {
    let code = r#"
sub cat_decode {
    my ( $obj, undef, $src, $pos ) = @_;
    return $src;
}
"#;
    assert_clean_parse(code);
}

/// Full OO pattern from Encode::CN::HZ -- the original failing file.
#[test]
fn encode_module_pattern() {
    let code = r#"
sub cat_decode {
    my ( $obj, undef, $src, $pos, $trm, $chk ) = @_;
    my ( $rdst, $rsrc, $rpos ) = \@_[ 1 .. 3 ];
    return $rdst;
}
"#;
    assert_clean_parse(code);
}
