mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn x_bang_bang_basic() {
    let ast = parse(r#"my @x = ("a") x !!$cond;"#);
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "Got error in: {}", sexp);
    assert!(sexp.contains("binary_x"), "Expected binary_x in: {}", sexp);
}

#[test]
fn x_bang_bang_no_space() {
    assert_clean_parse(r#"my @x = ("a") x!! $cond;"#);
}

#[test]
fn x_bang_bang_in_list() {
    assert_clean_parse(r#"my @x = ((status => $s) x!! $cond, bar => 2);"#);
}

#[test]
fn x_bang_bang_dancer2_pattern() {
    assert_clean_parse(r#"my @x = (status => $status) x!! $status;"#);
}

#[test]
fn x_bang_bang_multiline() {
    assert_clean_parse(
        r#"
my @args = (
    (verbosity => $opt{verbose}) x!! exists $opt{verbose},
    (jobs => $opt{jobs}) x!! exists $opt{jobs},
);
"#,
    );
}

#[test]
fn x_with_negation() {
    assert_clean_parse(r#"my @x = ("a") x -$n;"#);
}

#[test]
fn x_with_plus() {
    assert_clean_parse(r#"my @x = ("a") x +1;"#);
}

#[test]
fn x_with_backslash_ref() {
    assert_clean_parse(r#"my @x = ("a") x \$n;"#);
}

#[test]
fn x_with_bitwise_not() {
    assert_clean_parse(r#"my @x = ("a") x ~$mask;"#);
}
