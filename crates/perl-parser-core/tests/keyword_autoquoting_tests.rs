use perl_parser_core::Parser;
use perl_tdd_support::must;

#[allow(clippy::unwrap_used, clippy::expect_used)]
/// Helper: parse source, assert no errors, return sexp string.
fn parse_ok(src: &str) -> String {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "parse should succeed without errors for: {src}\ngot: {sexp}");
    sexp
}

// ── Statement-level keyword autoquoting ──────────────────────

#[test]
fn if_before_fat_arrow_in_hash_assignment() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("my %h = (if => 1, for => 2, while => 3);");
    // All three keywords should be treated as string keys
    assert!(sexp.contains("(string \"if\")"), "if should be autoquoted: {sexp}");
    assert!(sexp.contains("(string \"for\")"), "for should be autoquoted: {sexp}");
    assert!(sexp.contains("(string \"while\")"), "while should be autoquoted: {sexp}");
    Ok(())
}

#[test]
fn my_and_use_before_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("my %h = (my => \"value\", use => \"something\");");
    assert!(sexp.contains("(string \"my\")"), "my should be autoquoted: {sexp}");
    assert!(sexp.contains("(string \"use\")"), "use should be autoquoted: {sexp}");
    Ok(())
}

#[test]
fn return_before_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("my %h = (return => 1);");
    assert!(sexp.contains("(string \"return\")"), "return should be autoquoted: {sexp}");
    Ok(())
}

#[test]
fn unless_before_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("my %h = (unless => 1);");
    assert!(sexp.contains("(string \"unless\")"), "unless should be autoquoted: {sexp}");
    Ok(())
}

#[test]
fn next_last_redo_before_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("my %h = (next => 1, last => 2, redo => 3);");
    assert!(sexp.contains("(string \"next\")"), "next should be autoquoted: {sexp}");
    assert!(sexp.contains("(string \"last\")"), "last should be autoquoted: {sexp}");
    assert!(sexp.contains("(string \"redo\")"), "redo should be autoquoted: {sexp}");
    Ok(())
}

#[test]
fn sub_before_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("my %h = (sub => \\&handler);");
    assert!(sexp.contains("(string \"sub\")"), "sub should be autoquoted: {sexp}");
    Ok(())
}

// ── Function call arguments ─────────────────────────────────

#[test]
fn keyword_autoquoted_in_function_call() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("func(return => 1);");
    assert!(
        sexp.contains("(string \"return\")"),
        "return should be autoquoted in func call: {sexp}"
    );
    Ok(())
}

#[test]
fn keyword_autoquoted_in_function_call_if() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("func(if => 1);");
    assert!(sexp.contains("(string \"if\")"), "if should be autoquoted in func call: {sexp}");
    Ok(())
}

// ── Hash constructor with braces ────────────────────────────

#[test]
fn keyword_in_brace_hash_constructor() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("my $h = { if => 1, for => 2 };");
    assert!(!sexp.contains("(if "), "if should NOT be parsed as control flow: {sexp}");
    Ok(())
}

// ── Statement-level bare keyword => value ───────────────────

#[test]
fn bare_if_fat_arrow_at_statement_level() -> Result<(), Box<dyn std::error::Error>> {
    // `if => 1;` at statement level should be an expression, not an if-statement
    let sexp = parse_ok("if => 1;");
    assert!(sexp.contains("(string \"if\")"), "if should be autoquoted at statement level: {sexp}");
    Ok(())
}

#[test]
fn bare_return_fat_arrow_at_statement_level() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("return => 1;");
    assert!(
        sexp.contains("(string \"return\")"),
        "return should be autoquoted at statement level: {sexp}"
    );
    Ok(())
}

#[test]
fn bare_for_fat_arrow_at_statement_level() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("for => 1;");
    assert!(
        sexp.contains("(string \"for\")"),
        "for should be autoquoted at statement level: {sexp}"
    );
    Ok(())
}

#[test]
fn multiple_keyword_pairs_at_statement_level() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("if => 1, for => 2, while => 3;");
    assert!(sexp.contains("(string \"if\")"), "if should be autoquoted: {sexp}");
    assert!(sexp.contains("(string \"for\")"), "for should be autoquoted: {sexp}");
    assert!(sexp.contains("(string \"while\")"), "while should be autoquoted: {sexp}");
    Ok(())
}

// ── Normal keyword usage should still work ──────────────────

#[test]
fn if_statement_still_works() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("if (1) { 2; }");
    assert!(sexp.contains("(if "), "if statement should still parse: {sexp}");
    Ok(())
}

#[test]
fn while_loop_still_works() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("while (1) { 2; }");
    assert!(sexp.contains("(while "), "while loop should still parse: {sexp}");
    Ok(())
}

#[test]
fn for_loop_still_works() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("for my $i (1..10) { 2; }");
    assert!(
        sexp.contains("for") || sexp.contains("(foreach "),
        "for loop should still parse: {sexp}"
    );
    Ok(())
}

#[test]
fn return_statement_still_works() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("return 42;");
    assert!(sexp.contains("(return "), "return statement should still parse: {sexp}");
    Ok(())
}

#[test]
fn my_declaration_still_works() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("my $x = 1;");
    assert!(sexp.contains("my_declaration"), "my declaration should still parse: {sexp}");
    Ok(())
}

#[test]
fn use_statement_still_works() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("use strict;");
    assert!(sexp.contains("(use "), "use statement should still parse: {sexp}");
    Ok(())
}

// ── Hash subscript should NOT autoquote ─────────────────────

#[test]
fn hash_subscript_unless_is_identifier() -> Result<(), Box<dyn std::error::Error>> {
    // $hash{unless} is a hash subscript, not autoquoting
    let sexp = parse_ok("$hash{unless};");
    // This should parse as a hash subscript, not trigger autoquoting logic
    assert!(!sexp.contains("ERROR"), "hash subscript with keyword should parse: {sexp}");
    Ok(())
}

// ── Additional keywords ─────────────────────────────────────

#[test]
fn eval_do_try_before_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("my %h = (eval => 1, do => 2);");
    assert!(!sexp.contains("ERROR"), "eval/do before => should parse: {sexp}");
    Ok(())
}

#[test]
fn package_class_method_before_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("my %h = (package => 1, class => 2, method => 3);");
    assert!(!sexp.contains("ERROR"), "package/class/method before => should parse: {sexp}");
    Ok(())
}

#[test]
fn begin_end_before_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("my %h = (BEGIN => 1, END => 2);");
    assert!(!sexp.contains("ERROR"), "BEGIN/END before => should parse: {sexp}");
    Ok(())
}

// ── Regular identifier autoquoting still works ──────────────

#[test]
fn regular_bareword_autoquoted() -> Result<(), Box<dyn std::error::Error>> {
    let sexp = parse_ok("my %h = (foo => 1, bar => 2);");
    assert!(sexp.contains("(string \"foo\")"), "foo should be autoquoted: {sexp}");
    assert!(sexp.contains("(string \"bar\")"), "bar should be autoquoted: {sexp}");
    Ok(())
}
