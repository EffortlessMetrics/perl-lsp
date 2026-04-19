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
fn shift_in_parens_hash_deref() {
    parse_ok("my $x = (shift @arr)->{'key'};");
}

#[test]
fn shift_in_parens_method_call() {
    parse_ok("my $x = (shift @arr)->method();");
}

#[test]
fn pop_in_parens_deref() {
    parse_ok("my $x = (pop @stack)->{'field'};");
}

#[test]
fn assignment_shift_in_parens_deref() {
    parse_ok("my $x = ($obj = shift @to_do)->{'_tag'};");
}

#[test]
fn nested_paren_assignment_deref() {
    parse_ok("if (($ptag = ($this = shift @to_do)->{'_tag'}) eq 'pre') { 1; }");
}

#[test]
fn simple_paren_expr() {
    parse_ok("my $x = ($a + $b);");
}

#[test]
fn paren_list() {
    parse_ok("my @x = (1, 2, 3);");
}

#[test]
fn paren_with_shift_no_deref() {
    parse_ok("my $x = shift @arr;");
}
