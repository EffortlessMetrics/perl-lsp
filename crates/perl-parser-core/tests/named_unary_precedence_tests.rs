use perl_parser_core::Parser;
use perl_tdd_support::must;

fn parse_ok(src: &str) -> String {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "parse should succeed without errors for: {src}\ngot: {sexp}"
    );
    sexp
}

#[test]
fn defined_arrow_hash_deref_stmt_start() {
    parse_ok("defined $obj->{field};");
}

#[test]
fn defined_arrow_hash_deref_in_if() {
    parse_ok("if (defined $obj->{field}) { 1; }");
}

#[test]
fn ref_arrow_hash_deref_eq_comparison() {
    let sexp = parse_ok("ref $obj->{list} eq 'ARRAY';");
    assert!(
        sexp.contains("(binary_eq"),
        "expected binary_eq root, got: {sexp}"
    );
    assert!(
        sexp.contains("(call ref (("),
        "expected ref call on the left-hand side, got: {sexp}"
    );
    assert!(
        !sexp.contains("(call ref ((binary_eq"),
        "ref ate binary_eq: {sexp}"
    );
    assert!(
        !sexp.contains("(function_call_expression (function)) (identifier eq)"),
        "ref split into multiple statements: {sexp}"
    );
}

#[test]
fn ref_variable_eq_comparison() {
    let sexp = parse_ok("ref $ref eq 'HASH';");
    assert!(
        sexp.contains("(binary_eq"),
        "expected binary_eq root, got: {sexp}"
    );
    assert!(
        sexp.contains("(call ref (("),
        "expected ref call on the left-hand side, got: {sexp}"
    );
    assert!(
        !sexp.contains("(call ref ((binary_eq"),
        "ref ate binary_eq: {sexp}"
    );
    assert!(
        !sexp.contains("(function_call_expression (function)) (identifier eq)"),
        "ref split into multiple statements: {sexp}"
    );
}

#[test]
fn defined_self_arrow_hash_deref() {
    parse_ok("defined $self->{cache};");
}

#[test]
fn defined_with_parens() {
    parse_ok("defined($obj->{field});");
}

#[test]
fn ref_with_parens() {
    parse_ok("ref($ref) eq 'HASH';");
}

#[test]
fn ref_type_check_in_condition() {
    parse_ok(
        r#"
if (ref $self->{handler} eq 'CODE') {
    $self->{handler}->();
}
"#,
    );
}

#[test]
fn defined_or_with_arrow_chain() {
    parse_ok("my $v = defined $obj->{key} ? $obj->{key} : 'default';");
}
