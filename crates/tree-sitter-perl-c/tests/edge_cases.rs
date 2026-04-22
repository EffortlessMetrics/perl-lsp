//! Edge case tests for tree-sitter-perl-c parser binding.
//!
//! These tests exercise boundary conditions, unusual inputs, and potential
//! error paths in the parse_perl_code() wrapper and the underlying C grammar.

use tree_sitter_perl_c::parse_perl_code;

mod common;
use common::sexp;

// ---------------------------------------------------------------------------
// Boundary: Empty and minimal inputs
// ---------------------------------------------------------------------------

/// Empty string — the grammar should produce a tree (possibly with ERROR).
#[test]
fn edge_case_empty_string() -> Result<(), Box<dyn std::error::Error>> {
    let result = sexp("");
    // Empty input is a valid boundary case; it may or may not have errors
    // depending on grammar interpretation. We just verify the parser doesn't panic.
    assert!(result.is_ok());
    Ok(())
}

/// Whitespace-only input — should parse without panicking.
#[test]
fn edge_case_whitespace_only() -> Result<(), Box<dyn std::error::Error>> {
    let result = sexp("   \n\t  \r\n");
    // Whitespace-only is valid; tree should be produced
    assert!(result.is_ok());
    Ok(())
}

/// Single semicolon — minimal statement terminator.
#[test]
fn edge_case_single_semicolon() -> Result<(), Box<dyn std::error::Error>> {
    let result = sexp(";");
    // Single semicolon is a valid empty statement
    assert!(result.is_ok());
    Ok(())
}

// ---------------------------------------------------------------------------
// Boundary: Deep nesting
// ---------------------------------------------------------------------------

/// Very deep nesting — exercises parser stack depth.
#[test]
fn edge_case_deeply_nested_blocks() -> Result<(), Box<dyn std::error::Error>> {
    // 20 levels of nesting — far beyond the existing nested_blocks test (3 levels)
    let code = "sub outer { if (1) { while (1) { for (1) { foreach (1) { if (1) { while (1) { for (1) { foreach my $x (1) { if (1) { while (1) { for (1) { if (1) { while (1) { for (1) { if (1) { while (1) { for (1) { } } } } } } } } } } } } } } } } }";
    let result = sexp(code);
    // Should not panic or overflow; may have errors if grammar limits depth
    assert!(result.is_ok());
    Ok(())
}

// ---------------------------------------------------------------------------
// Boundary: Large inputs
// ---------------------------------------------------------------------------

/// Very long single statement — exercises buffer handling.
#[test]
fn edge_case_long_statement() -> Result<(), Box<dyn std::error::Error>> {
    let long_var = "x".repeat(10_000);
    let code = format!("my ${} = 1;", long_var);
    let result = sexp(&code);
    assert!(result.is_ok());
    Ok(())
}

/// Many statements in one file — exercises repeated parsing.
#[test]
fn edge_case_many_statements() -> Result<(), Box<dyn std::error::Error>> {
    let statements: Vec<_> = (0..1000).map(|i| format!("my $x{} = {};", i, i)).collect();
    let code = statements.join("\n");
    let result = sexp(&code);
    assert!(result.is_ok());
    Ok(())
}

// ---------------------------------------------------------------------------
// Boundary: Unicode
// ---------------------------------------------------------------------------

/// Unicode identifier — Perl allows Unicode in variable names under utf8.
#[test]
fn edge_case_unicode_identifier() -> Result<(), Box<dyn std::error::Error>> {
    // 日本語 in a scalar variable name (valid under 'use utf8')
    let code = "my $日本語 = 42;";
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Unicode string literal — wide characters in string content.
#[test]
fn edge_case_unicode_string() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $greeting = "こんにちは世界";"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Emoji in string — unusual but valid UTF-8.
#[test]
fn edge_case_emoji_in_string() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $emoji = "Hello 🌍🪐";"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

// ---------------------------------------------------------------------------
// Boundary: Control characters and unusual whitespace
// ---------------------------------------------------------------------------

/// String with explicit newline escape — \n in double-quoted string.
#[test]
fn edge_case_escaped_newline_in_string() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $text = "line1\nline2";"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// String containing NUL byte — \0 in string.
#[test]
fn edge_case_nul_in_string() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $data = "hello\0world";"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Tab and vertical tab in string.
#[test]
fn edge_case_whitespace_escape_in_string() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $text = "tab:\tin\nvertical\vfeed";"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

// ---------------------------------------------------------------------------
// Boundary: Statement modifiers and postfix forms
// ---------------------------------------------------------------------------

/// Postfix if modifier.
#[test]
fn edge_case_postfix_if_modifier() -> Result<(), Box<dyn std::error::Error>> {
    let code = "print 'odd' if $x % 2;";
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Postfix unless modifier.
#[test]
fn edge_case_postfix_unless_modifier() -> Result<(), Box<dyn std::error::Error>> {
    let code = "print 'even' unless $x % 2;";
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Postfix while modifier.
#[test]
fn edge_case_postfix_while_modifier() -> Result<(), Box<dyn std::error::Error>> {
    let code = "$x = 0; $x++ while $x < 10;";
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Postfix for modifier.
#[test]
fn edge_case_postfix_for_modifier() -> Result<(), Box<dyn std::error::Error>> {
    let code = "print for @items;";
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

// ---------------------------------------------------------------------------
// Boundary: Data structure literals
// ---------------------------------------------------------------------------

/// Array reference constructor — [...] anonymous array.
#[test]
fn edge_case_array_reference() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $arr = [1, 2, 3];";
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Hash reference constructor — {...} anonymous hash.
#[test]
fn edge_case_hash_reference() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $hash = { name => "Alice", age => 30 };"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Array and hash slice — @array[...] and %hash{...}.
#[test]
fn edge_case_slice_notation() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $first = $arr[0]; my $val = $hash{key};";
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

// ---------------------------------------------------------------------------
// Boundary: Subroutine references and prototypes
// ---------------------------------------------------------------------------

/// Anonymous subroutine — sub { ... }.
#[test]
fn edge_case_anonymous_subroutine() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $code = sub { return $_[0] + 1; };";
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Subroutine with prototype.
#[test]
fn edge_case_subroutine_with_prototype() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub my_sort ($) { return $_[0] <=> $_[1]; }";
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Sub call with reference — \&subname.
#[test]
fn edge_case_sub_reference() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $coderef = \&mysub;"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

// ---------------------------------------------------------------------------
// Boundary: Blessed references and objects
// ---------------------------------------------------------------------------

/// Bless expression — create an object.
#[test]
fn edge_case_bless_expression() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $obj = bless { id => 1 }, 'MyClass';";
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Method call on blessed object.
#[test]
fn edge_case_method_call() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $result = $obj->method(@args);";
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Indirect method call.
#[test]
fn edge_case_indirect_method_call() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $obj = new MyClass id => 1;";
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

// ---------------------------------------------------------------------------
// Boundary: eval and error handling
// ---------------------------------------------------------------------------

/// Eval block — string eval.
#[test]
fn edge_case_eval_string() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"eval '$x = 1 + 1';"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Eval block with block syntax.
#[test]
fn edge_case_eval_block() -> Result<(), Box<dyn std::error::Error>> {
    let code = "eval { $x = 1 / 0; };";
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Try-catch (PERL510 scope, experimental).
#[test]
fn edge_case_try_catch() -> Result<(), Box<dyn std::error::Error>> {
    let code = "try { die 'oops' } catch ($e) { warn $e; }";
    let result = sexp(code);
    // try/catch is experimental in some Perl versions; grammar may or may not support it
    assert!(result.is_ok());
    Ok(())
}

// ---------------------------------------------------------------------------
// Boundary: Various quote-like operators
// ---------------------------------------------------------------------------

/// Q// quote — single-quoted string.
#[test]
fn edge_case_q_quote() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $text = q{hello world};"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// QQ// quote — double-quoted string.
#[test]
fn edge_case_qq_quote() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $text = qq{hello $name};"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// QR// quote — regex.
#[test]
fn edge_case_qr_quote() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $re = qr{\d+};"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// QW// quote — word list.
#[test]
fn edge_case_qw_quote() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my @words = qw{apple banana cherry};"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// QX// quote — command execution.
#[test]
fn edge_case_qx_quote() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $output = qx{echo hello};"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

// ---------------------------------------------------------------------------
// Boundary: Regex edge cases
// ---------------------------------------------------------------------------

/// Regex with greedy quantifier.
#[test]
fn edge_case_regex_greedy_quantifier() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $match = $str =~ /a+b*c+/;"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Regex with non-greedy (lazy) quantifier.
#[test]
fn edge_case_regex_lazy_quantifier() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $match = $str =~ /a+?b*?c+?/;"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Regex with character class.
#[test]
fn edge_case_regex_character_class() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $match = $str =~ /[a-zA-Z0-9_]+/;"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Regex with alternation.
#[test]
fn edge_case_regex_alternation() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $match = $str =~ /foo|bar|baz/;"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Regex with capturing groups.
#[test]
fn edge_case_regex_capturing_groups() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my ($a, $b) = $str =~ /(\d+)-(\d+)/;"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

/// Regex with non-capturing group.
#[test]
fn edge_case_regex_non_capturing_group() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $match = $str =~ /(?:pre)(.+)/;"#;
    let result = sexp(code);
    assert!(result.is_ok());
    Ok(())
}

// ---------------------------------------------------------------------------
// Error handling: Malformed inputs
// ---------------------------------------------------------------------------

/// Missing closing brace — should still produce a tree (with ERROR node).
#[test]
fn edge_case_missing_closing_brace() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub foo { return 1;";
    // Grammar may recover with ERROR node; should not panic
    let tree = parse_perl_code(code)?;
    // Verify we got a tree (grammar recovery)
    assert!(tree.root_node().kind() == "source_file");
    Ok(())
}

/// Missing closing parenthesis — unbalanced.
#[test]
fn edge_case_missing_closing_paren() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = (1 + 2;";
    let tree = parse_perl_code(code)?;
    assert!(tree.root_node().kind() == "source_file");
    Ok(())
}

/// Unclosed string literal.
#[test]
fn edge_case_unclosed_string() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $x = "hello world;"#;
    let tree = parse_perl_code(code)?;
    assert!(tree.root_node().kind() == "source_file");
    Ok(())
}

/// Bareword without semicolon or following statement.
#[test]
fn edge_case_bareword_no_terminator() -> Result<(), Box<dyn std::error::Error>> {
    let code = "Foo::Bar";
    let tree = parse_perl_code(code)?;
    assert!(tree.root_node().kind() == "source_file");
    Ok(())
}
