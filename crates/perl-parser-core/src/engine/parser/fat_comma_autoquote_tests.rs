//! Tests for fat comma autoquoting of keywords.
//!
//! In Perl, the fat comma `=>` autoquotes the left-hand side, treating it as
//! a bareword string even if it is a reserved keyword or builtin function name.
//! For example, `(delete => 1)` should parse `delete` as a string key, not as
//! the `delete` builtin.

use crate::Parser;

fn parses_without_errors(code: &str) -> bool {
    let mut parser = Parser::new(code);
    let output = parser.parse_with_recovery();
    let sexp = output.ast.to_sexp();
    !sexp.contains("ERROR")
}

fn sexp(code: &str) -> String {
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => ast.to_sexp(),
        Err(e) => format!("PARSE_ERROR: {}", e),
    }
}

// ── Keyword TokenKind autoquoting ──────────────────────────────────────

#[test]
fn fat_comma_if_key() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my %h = (if => 1);"));
    Ok(())
}

#[test]
fn fat_comma_for_key() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my %h = (for => 2);"));
    Ok(())
}

#[test]
fn fat_comma_while_key() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my %h = (while => 1);"));
    Ok(())
}

#[test]
fn fat_comma_unless_key() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my %h = (unless => 0);"));
    Ok(())
}

#[test]
fn fat_comma_return_key() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my %h = (return => 1);"));
    Ok(())
}

#[test]
fn fat_comma_eval_key() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my %h = (eval => 1);"));
    Ok(())
}

#[test]
fn fat_comma_do_key() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my %h = (do => 1);"));
    Ok(())
}

#[test]
fn fat_comma_sub_key() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my %h = (sub => 1);"));
    Ok(())
}

#[test]
fn fat_comma_use_key() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my %h = (use => 1);"));
    Ok(())
}

#[test]
fn fat_comma_next_last_redo_keys() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my %h = (next => 1, last => 2, redo => 3);"));
    Ok(())
}

// ── Builtin function name autoquoting ──────────────────────────────────

#[test]
fn fat_comma_delete_key() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my %h = (delete => 1);"));
    let s = sexp("my %h = (delete => 1);");
    assert!(s.contains("delete"), "Expected delete as identifier: {}", s);
    Ok(())
}

#[test]
fn fat_comma_die_key() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my %dispatch = (die => 1);"));
    Ok(())
}

#[test]
fn fat_comma_print_key() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my %dispatch = (print => 1);"));
    Ok(())
}

#[test]
fn fat_comma_push_key() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my %h = (push => 1);"));
    Ok(())
}

#[test]
fn fat_comma_keys_key() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my %h = (keys => 3);"));
    Ok(())
}

// ── Multiple keyword keys ──────────────────────────────────────────────

#[test]
fn fat_comma_multiple_keyword_keys() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my %h = (if => 1, for => 2, keys => 3);"));
    Ok(())
}

// ── Function call context ──────────────────────────────────────────────

#[test]
fn fat_comma_unless_in_function_call() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my_func(unless => 0);"));
    Ok(())
}

// ── Hash literal / anonymous hash contexts ─────────────────────────────

#[test]
fn fat_comma_keywords_in_hash_literal() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my $h = {if => 1, unless => 0, for => 2};"));
    Ok(())
}

#[test]
fn fat_comma_keywords_in_anon_hash() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("my $ref = {delete => 1, die => 2};"));
    Ok(())
}

// ── Regression: keywords still work normally without => ────────────────

#[test]
fn keyword_if_still_works_as_statement() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("if (1) { print 1; }"));
    Ok(())
}

#[test]
fn keyword_for_still_works_as_statement() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("for my $i (1..10) { print $i; }"));
    Ok(())
}

#[test]
fn keyword_delete_still_works_as_builtin() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("delete $hash{key};"));
    Ok(())
}

#[test]
fn keyword_return_still_works_in_sub() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parses_without_errors("sub foo { return 42; }"));
    Ok(())
}
