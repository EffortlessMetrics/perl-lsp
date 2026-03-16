use perl_parser_core::Parser;
use perl_tdd_support::must;

fn parse_ok(src: &str) -> String {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "parse should succeed without errors for: {src}\ngot: {sexp}");
    sexp
}

#[test]
fn first_block_statement_level() {
    parse_ok("my $x = first { $_ > 0 } @arr;");
}

#[test]
fn any_block_statement_level() {
    parse_ok("my $ok = any { $_ > 0 } 1, 2, 3;");
}

#[test]
fn first_block_in_if_condition() {
    parse_ok("if (first { $_ > 0 } @arr) { 1; }");
}

#[test]
fn any_block_in_if_condition() {
    parse_ok("if (any { $_ > 0 } @arr) { 1; }");
}

#[test]
fn all_block_in_if_condition() {
    parse_ok("if (all { defined $_ } @arr) { 1; }");
}

#[test]
fn none_block_in_if_condition() {
    parse_ok("if (none { $_ < 0 } @arr) { 1; }");
}

#[test]
fn reduce_block_in_if_condition() {
    parse_ok("my $s = reduce { $a + $b } @arr;");
}

#[test]
fn first_block_in_unless_condition() {
    parse_ok("unless (first { $_ eq $name } @list) { 1; }");
}

#[test]
fn first_block_complex_args() {
    // Biber-style: deep deref in list argument
    parse_ok("unless (first { $_ eq $name } $ref->{names}->{key}->@*) { 1; }");
}

#[test]
fn grep_block_in_if_condition() {
    parse_ok("if (grep { $_ > 0 } @arr) { 1; }");
}

#[test]
fn map_block_in_condition() {
    parse_ok("my @r = map { $_ * 2 } @arr;");
}
