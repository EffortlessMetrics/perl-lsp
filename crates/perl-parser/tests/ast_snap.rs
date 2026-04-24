//! Snapshot tests for AST structure and error messages.
//!
//! These tests use `insta` to capture baseline snapshots of:
//! - Parser AST structure (s-expression format) for well-formed Perl
//! - Error recovery AST for malformed Perl input
//! - Error message formatting for each ParseError variant
//! - Semantic token legend (token types and modifiers)
//!
//! Run with `cargo test -p perl-parser --test ast_snap` to execute.
//! Update snapshots with `cargo insta review` after intentional changes.

use insta::assert_snapshot;
use perl_lsp_rs_core::providers::semantic_tokens;
use perl_parser::Parser;

// ---------------------------------------------------------------------------
// Helper: parse and return sexp from recovery output
// ---------------------------------------------------------------------------

fn parse_sexp(source: &str) -> String {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    output.ast.to_sexp()
}

fn parse_errors(source: &str) -> String {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    // Format errors as a sorted, newline-separated list for stable snapshots
    let mut lines: Vec<String> = output.diagnostics.iter().map(|e| format!("{}", e)).collect();
    lines.sort();
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// 1. Clean Perl AST snapshots (CPAN-style edge cases)
// ---------------------------------------------------------------------------

#[test]
fn ast_variable_declaration() {
    assert_snapshot!(parse_sexp("my $x = 42;"));
}

#[test]
fn ast_sub_definition() {
    assert_snapshot!(parse_sexp("sub greet { return \"Hello\"; }"));
}

#[test]
fn ast_package_declaration() {
    assert_snapshot!(parse_sexp("package My::Module;"));
}

#[test]
fn ast_if_elsif_else() {
    assert_snapshot!(parse_sexp(
        "if ($x > 0) { print \"pos\"; } elsif ($x < 0) { print \"neg\"; } else { print \"zero\"; }"
    ));
}

#[test]
fn ast_array_operations() {
    assert_snapshot!(parse_sexp("my @arr = (1, 2, 3); push @arr, 4;"));
}

#[test]
fn ast_hash_operations() {
    assert_snapshot!(parse_sexp("my %h = (a => 1, b => 2); my $v = $h{a};"));
}

#[test]
fn ast_method_call() {
    assert_snapshot!(parse_sexp("$obj->method($arg1, $arg2);"));
}

#[test]
fn ast_regex_match() {
    assert_snapshot!(parse_sexp("if ($str =~ /^hello/i) { print \"matched\"; }"));
}

#[test]
fn ast_use_strict_warnings() {
    assert_snapshot!(parse_sexp("use strict;\nuse warnings;"));
}

#[test]
fn ast_while_loop() {
    assert_snapshot!(parse_sexp("while (my $line = <STDIN>) { chomp $line; print $line; }"));
}

#[test]
fn ast_for_loop() {
    assert_snapshot!(parse_sexp("for my $i (1..10) { print \"$i\\n\"; }"));
}

#[test]
fn ast_anonymous_sub() {
    assert_snapshot!(parse_sexp("my $code = sub { my ($x) = @_; return $x * 2; };"));
}

#[test]
fn ast_string_interpolation() {
    assert_snapshot!(parse_sexp("my $name = \"world\"; my $msg = \"Hello, $name!\";"));
}

#[test]
fn ast_chained_method_calls() {
    assert_snapshot!(parse_sexp("$obj->foo->bar->baz;"));
}

#[test]
fn ast_ternary_operator() {
    assert_snapshot!(parse_sexp("my $x = $cond ? \"yes\" : \"no\";"));
}

#[test]
fn ast_postfix_conditionals() {
    // Postfix conditionals are idiomatic Perl and stress statement/operator precedence.
    assert_snapshot!(parse_sexp("print $msg if $enabled; warn $msg unless $quiet;"));
}

#[test]
fn ast_foreach_keys_iteration() {
    // Iterating hash keys is a common real-world control-flow/data-access pattern.
    assert_snapshot!(parse_sexp("for my $k (keys %h) { print $k; }"));
}

// CPAN edge cases
#[test]
fn ast_empty_input() {
    // Empty .pm files are legal Perl; the parser must not panic and must emit an
    // empty program node.
    assert_snapshot!(parse_sexp(""));
}

#[test]
fn ast_use_module_qw() {
    // `use Module qw(...)` is the most common CPAN import pattern.
    assert_snapshot!(parse_sexp("use List::Util qw(sum min max first);"));
}

#[test]
fn ast_qw_list_assignment() {
    // qw// as a list literal is ubiquitous in Perl — covers the word-list quoting
    // operator which has special tokenization rules.
    assert_snapshot!(parse_sexp("my @days = qw(Mon Tue Wed Thu Fri);"));
}

#[test]
fn ast_map_grep_and_sort_pipeline() {
    // Common CPAN list pipeline pattern that mixes block forms and implicit $_.
    assert_snapshot!(parse_sexp(
        "my @result = sort { $a cmp $b } map { lc $_ } grep { /foo/ } @items;",
    ));
}

#[test]
fn ast_eval_with_localized_error_variable() {
    // Exception handling shape is very common and is sensitive to sigils and blocks.
    assert_snapshot!(parse_sexp("eval { risky_call() }; if ($@) { warn $@; }",));
}

#[test]
fn ast_state_variable_and_default_operator() {
    // `state` and defined-or are both widely used in modern Perl modules.
    assert_snapshot!(parse_sexp("state $counter = 0; $counter //= 1;"));
}

// ---------------------------------------------------------------------------
// 2. Error recovery AST snapshots (malformed input)
// ---------------------------------------------------------------------------

#[test]
fn recovery_missing_semicolon() {
    assert_snapshot!(parse_sexp("my $x = 42"));
}

#[test]
fn recovery_unclosed_block() {
    assert_snapshot!(parse_sexp("sub foo {"));
}

#[test]
fn recovery_missing_rhs() {
    assert_snapshot!(parse_sexp("my $x = ;"));
}

#[test]
fn recovery_unclosed_paren() {
    assert_snapshot!(parse_sexp("print(\"hello\";"));
}

#[test]
fn recovery_multiple_errors() {
    assert_snapshot!(parse_sexp("my $x = ;\nmy $y = ;"));
}

#[test]
fn recovery_truncated_hash() {
    assert_snapshot!(parse_sexp("my %h = (a =>"));
}

#[test]
fn recovery_truncated_array() {
    assert_snapshot!(parse_sexp("my @arr = (1, 2,"));
}

#[test]
fn recovery_partial_if() {
    assert_snapshot!(parse_sexp("if ($x > 0) {"));
}

#[test]
fn recovery_empty_sub_body() {
    assert_snapshot!(parse_sexp("sub foo"));
}

#[test]
fn recovery_statement_after_error() {
    // Parser should recover and parse the second statement correctly
    assert_snapshot!(parse_sexp("my $x = ;\nmy $y = 10;"));
}

#[test]
fn recovery_unclosed_quote_then_valid_statement() {
    // Ensure recovery can resynchronize after unterminated string literals.
    assert_snapshot!(parse_sexp("my $x = \"oops;\nmy $y = 1;"));
}

#[test]
fn recovery_unclosed_hash_subscript_then_followup_statement() {
    // Missing closing `}` in hash subscript should still allow recovery to next statement.
    assert_snapshot!(parse_sexp("my $x = $h{foo;\nmy $y = 2;"));
}

#[test]
fn recovery_broken_regex_then_followup_statement() {
    // Broken regex delimiters should not prevent parsing later statements.
    assert_snapshot!(parse_sexp("if ($text =~ /abc) { print 1; }\nmy $ok = 1;"));
}

// ---------------------------------------------------------------------------
// 3. Error message format snapshots
// ---------------------------------------------------------------------------

#[test]
fn errors_missing_rhs() {
    assert_snapshot!(parse_errors("my $x = ;"));
}

#[test]
fn errors_unclosed_block() {
    assert_snapshot!(parse_errors("sub foo {"));
}

#[test]
fn errors_multiple_statements_errors() {
    assert_snapshot!(parse_errors("my $x = ;\nmy $y = ;"));
}

#[test]
fn errors_truncated_hash() {
    assert_snapshot!(parse_errors("my %h = (a =>"));
}

#[test]
fn errors_unterminated_string_and_followup() {
    assert_snapshot!(parse_errors("my $x = \"oops;\nmy $y = 1;"));
}

#[test]
fn errors_unclosed_hash_subscript_and_followup() {
    assert_snapshot!(parse_errors("my $x = $h{foo;\nmy $y = 2;"));
}

#[test]
fn errors_broken_regex_delimiter() {
    assert_snapshot!(parse_errors("if ($text =~ /abc) { print 1; }\nmy $ok = 1;"));
}

// ---------------------------------------------------------------------------
// 4. Semantic token legend snapshot
//    The ordering of token types and modifiers is part of the LSP protocol
//    contract — clients decode by index, so any reordering is a breaking change.
// ---------------------------------------------------------------------------

#[test]
fn semantic_token_legend_types() {
    let leg = semantic_tokens::legend();
    let types_str = leg.token_types.join("\n");
    assert_snapshot!(types_str);
}

#[test]
fn semantic_token_legend_modifiers() {
    let leg = semantic_tokens::legend();
    let mods_str = leg.modifiers.join("\n");
    assert_snapshot!(mods_str);
}

#[test]
fn semantic_token_legend_index_mapping() {
    // Snapshot the full ordered legend as "index: name" pairs
    let leg = semantic_tokens::legend();
    let mut lines = Vec::new();
    lines.push("token_types:".to_string());
    for (i, t) in leg.token_types.iter().enumerate() {
        lines.push(format!("  {}: {}", i, t));
    }
    lines.push("modifiers:".to_string());
    for (i, m) in leg.modifiers.iter().enumerate() {
        lines.push(format!("  {}: {}", i, m));
    }
    assert_snapshot!(lines.join("\n"));
}
