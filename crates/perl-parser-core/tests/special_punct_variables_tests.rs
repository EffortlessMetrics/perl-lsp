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
fn format_name_var_assign() {
    parse_ok("$~ = 'REPORT';");
}

#[test]
fn format_name_var_read() {
    let sexp = parse_ok("print $~;");
    assert!(!sexp.contains("binary_^"), "binary_^ in sexp: {sexp}");
}

#[test]
fn format_top_var_assign() {
    parse_ok("$^ = 'REPORT_TOP';");
}

#[test]
fn page_length_var_assign() {
    parse_ok("$= = 60;");
}

#[test]
fn page_number_var_assign() {
    parse_ok("$% = 0;");
}

#[test]
fn output_field_sep_assign() {
    parse_ok(r#"$, = ", ";"#);
}

#[test]
fn list_sep_assign() {
    parse_ok(r#"$" = ":";"#);
}

#[test]
fn subscript_sep_assign() {
    parse_ok(r#"$; = "\034";"#);
}

#[test]
fn caret_w_assign() {
    let sexp = parse_ok("$^W = 1;");
    assert!(!sexp.contains("binary_^"), "binary_^ in sexp: {sexp}");
}

#[test]
fn caret_o_read() {
    let sexp = parse_ok("my $os = $^O;");
    assert!(!sexp.contains("binary_^"), "binary_^ in sexp: {sexp}");
}

#[test]
fn caret_x_read() {
    let sexp = parse_ok("my $perl = $^X;");
    assert!(!sexp.contains("binary_^"), "binary_^ in sexp: {sexp}");
}

#[test]
fn mixed_format_vars() {
    parse_ok(
        r#"
$~ = 'REPORT';
$^ = 'REPORT_TOP';
$= = 60;
write;
"#,
    );
}

#[test]
fn prototype_semicolon_not_confused_with_special_var() {
    parse_ok("sub foo($;@) {}");
}

// ===== $>, $<, $) — process UID/GID special variables =====

#[test]
fn dollar_gt_effective_uid() {
    parse_ok("my $uid = $>;");
}

#[test]
fn dollar_lt_real_uid() {
    parse_ok("my $uid = $<;");
}

#[test]
fn dollar_rparen_effective_gid() {
    parse_ok(r#"my ($egid, @supp) = split " ", $);"#);
}

#[test]
fn dollar_rparen_in_constant() {
    parse_ok(r#"use constant GID => (split ' ', $) )[0];"#);
}

#[test]
fn dollar_gt_in_getpwuid() {
    parse_ok(r#"my $user = (getpwuid($>))[0];"#);
}

#[test]
fn dollar_rparen_in_grep() {
    parse_ok(r#"$ok = grep { $_ == $x } split /\s+/, $);"#);
}

// ===== while () { } — empty condition (infinite loop idiom) =====

#[test]
fn while_empty_condition() {
    parse_ok("while () { last; }");
}

#[test]
fn while_empty_condition_with_body() {
    parse_ok("while () { my $line = <STDIN>; last unless defined $line; }");
}
