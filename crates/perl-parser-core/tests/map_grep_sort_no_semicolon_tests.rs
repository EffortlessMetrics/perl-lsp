use perl_parser_core::Parser;
use perl_tdd_support::must;

fn parse_ok(src: &str) {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "parse should succeed without errors for: {src}\ngot: {sexp}");
}

#[test]
fn map_block_no_semicolon_in_if() {
    parse_ok("if (1) { map { $_ * 2 } @arr }");
}

#[test]
fn sort_block_no_semicolon_in_if() {
    parse_ok("if (1) { sort { $a cmp $b } @arr }");
}

#[test]
fn grep_block_no_semicolon_in_if() {
    parse_ok(r#"if (1) { grep { /\d/ } @arr }"#);
}

#[test]
fn map_block_no_semicolon_with_else() {
    parse_ok(
        r#"
if (@_) {
    map { $args{$_} = 1 } @_
}
else {
    %args = ();
}
"#,
    );
}

#[test]
fn map_block_with_semicolon_regression() {
    parse_ok("if (1) { map { $_ * 2 } @arr; }");
}

#[test]
fn map_block_no_semicolon_in_sub() {
    parse_ok("sub foo { map { uc $_ } @list }");
}

#[test]
fn sort_block_no_semicolon_in_begin() {
    parse_ok("BEGIN { sort { $a <=> $b } @nums }");
}
