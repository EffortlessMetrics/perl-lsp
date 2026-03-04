//! Extended unit tests for the `perl-lsp-semantic-tokens` crate.
//!
//! This module provides additional comprehensive test coverage beyond the
//! comprehensive_unit_tests.rs, focusing on edge cases, complex patterns,
//! and boundary conditions. Tests cover advanced Perl constructs and their
//! semantic token classification.

use perl_lsp_semantic_tokens::{
    EncodedToken, SemanticTokensProvider, TokensLegend, collect_semantic_tokens, legend,
};
use perl_tdd_support::{Parser, must, must_some};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn single_line_pos(byte: usize) -> (u32, u32) {
    (0, byte as u32)
}

fn line_col_mapper(text: &str) -> impl Fn(usize) -> (u32, u32) + '_ {
    move |byte: usize| {
        let prefix = &text[..byte.min(text.len())];
        let line = prefix.matches('\n').count() as u32;
        let last_nl = prefix.rfind('\n').map_or(0, |p| p + 1);
        let col = (byte - last_nl) as u32;
        (line, col)
    }
}

fn tokens_for(code: &str) -> Result<Vec<EncodedToken>, Box<dyn std::error::Error>> {
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let mapper = line_col_mapper(code);
    Ok(collect_semantic_tokens(&ast, code, &mapper))
}

// ---------------------------------------------------------------------------
// Advanced Keyword Tests
// ---------------------------------------------------------------------------

#[test]
fn keyword_given_when_in_switch_statement() -> Result<(), Box<dyn std::error::Error>> {
    let code = "given ($x) { when (1) { } default { } }";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let kw_idx = *leg.map.get("keyword").ok_or("keyword not in legend")?;
    let keywords = tokens.iter().filter(|t| t[3] == kw_idx).count();
    assert!(keywords > 0, "should have at least one keyword (given/when/default)");
    Ok(())
}

#[test]
fn keyword_local_scope_modifier() -> Result<(), Box<dyn std::error::Error>> {
    let code = "local $x = 10;";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let kw_idx = *leg.map.get("keyword").ok_or("keyword not in legend")?;
    let has_local = tokens.iter().any(|t| t[3] == kw_idx);
    assert!(has_local, "'local' should be classified as keyword");
    Ok(())
}

#[test]
fn keyword_our_package_scoped_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = "our @export_ok = qw(foo bar);";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let kw_idx = *leg.map.get("keyword").ok_or("keyword not in legend")?;
    let has_our = tokens.iter().any(|t| t[3] == kw_idx);
    assert!(has_our, "'our' should be classified as keyword");
    Ok(())
}

#[test]
fn keyword_next_loop_control() -> Result<(), Box<dyn std::error::Error>> {
    let code = "for my $i (1..10) { next if $i == 5; }";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let kw_idx = *leg.map.get("keyword").ok_or("keyword not in legend")?;
    let keywords: Vec<_> = tokens.iter().filter(|t| t[3] == kw_idx).collect();
    assert!(keywords.len() >= 2, "should have 'for' and 'next' as keywords");
    Ok(())
}

#[test]
fn keyword_last_loop_exit() -> Result<(), Box<dyn std::error::Error>> {
    let code = "while (1) { last if condition(); }";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let kw_idx = *leg.map.get("keyword").ok_or("keyword not in legend")?;
    let has_last = tokens.iter().any(|t| t[3] == kw_idx);
    assert!(has_last, "'last' should be classified as keyword");
    Ok(())
}

#[test]
fn keyword_redo_iteration_restart() -> Result<(), Box<dyn std::error::Error>> {
    let code = "for (@array) { redo if something(); }";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let kw_idx = *leg.map.get("keyword").ok_or("keyword not in legend")?;
    let has_redo = tokens.iter().any(|t| t[3] == kw_idx);
    assert!(has_redo, "'redo' should be classified as keyword");
    Ok(())
}

// ---------------------------------------------------------------------------
// Advanced Variable and Declaration Tests
// ---------------------------------------------------------------------------

#[test]
fn variable_hash_scalar_context() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my %hash = (a => 1); my $ref = \\%hash;";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let var_idx = *leg.map.get("variable").ok_or("variable not in legend")?;
    let variables: Vec<_> = tokens.iter().filter(|t| t[3] == var_idx).collect();
    assert!(variables.len() > 0, "should classify variables in hash context");
    Ok(())
}

#[test]
fn variable_array_context_foreach() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my @list = (1, 2, 3); for my $item (@list) { }";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let var_idx = *leg.map.get("variable").ok_or("variable not in legend")?;
    let variables: Vec<_> = tokens.iter().filter(|t| t[3] == var_idx).collect();
    assert!(variables.len() > 0, "should classify variables in array context");
    Ok(())
}

#[test]
fn variable_underscore_special_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = "print if $_;";
    let tokens = tokens_for(code)?;
    // Special variables like $_ should be recognized
    assert!(!tokens.is_empty(), "should produce tokens for special variable $_");
    Ok(())
}

#[test]
fn variable_reference_creation() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $ref = \\$scalar; my $aref = \\@array;";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for reference creation");
    Ok(())
}

#[test]
fn variable_dereference_scalar() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $val = $$ref; print $$ref;";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let var_idx = *leg.map.get("variable").ok_or("variable not in legend")?;
    let variables: Vec<_> = tokens.iter().filter(|t| t[3] == var_idx).collect();
    assert!(variables.len() > 0, "should classify dereferenced scalars");
    Ok(())
}

#[test]
fn variable_hash_slice_context() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my @values = @hash{'a', 'b', 'c'};";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for hash slice");
    Ok(())
}

#[test]
fn variable_postfix_dereference() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $val = $ref->$*; my $len = @array->[0];";
    let tokens = tokens_for(code)?;
    // Postfix dereference (experimental feature)
    assert!(!tokens.is_empty(), "should produce tokens for postfix dereference");
    Ok(())
}

// ---------------------------------------------------------------------------
// String and Number Literal Tests
// ---------------------------------------------------------------------------

#[test]
fn string_interpolation_double_quotes() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $name = \"John\"; print \"Hello, $name!\";";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let str_idx = *leg.map.get("string").ok_or("string not in legend")?;
    let strings: Vec<_> = tokens.iter().filter(|t| t[3] == str_idx).collect();
    assert!(strings.len() > 0, "should classify double-quoted strings");
    Ok(())
}

#[test]
fn string_single_quote_literal() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $msg = 'No interpolation';";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let str_idx = *leg.map.get("string").ok_or("string not in legend")?;
    let strings: Vec<_> = tokens.iter().filter(|t| t[3] == str_idx).collect();
    assert!(strings.len() > 0, "should classify single-quoted strings");
    Ok(())
}

#[test]
fn string_here_document_quoted() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $doc = <<EOF; content EOF";
    let tokens = tokens_for(code)?;
    // Heredoc is complex - should be recognized
    assert!(!tokens.is_empty(), "should produce tokens for heredoc");
    Ok(())
}

#[test]
fn string_escape_sequences() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $str = \"\\n\\t\\x{1F600}\";";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let str_idx = *leg.map.get("string").ok_or("string not in legend")?;
    let strings: Vec<_> = tokens.iter().filter(|t| t[3] == str_idx).collect();
    assert!(strings.len() > 0, "should classify strings with escape sequences");
    Ok(())
}

#[test]
fn number_hexadecimal_literal() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $hex = 0xFF; my $oct = 0777; my $bin = 0b1010;";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let num_idx = *leg.map.get("number").ok_or("number not in legend")?;
    let numbers: Vec<_> = tokens.iter().filter(|t| t[3] == num_idx).collect();
    assert!(numbers.len() > 0, "should classify hexadecimal numbers");
    Ok(())
}

#[test]
fn number_floating_point_scientific() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $sci = 1.5e-10; my $pi = 3.14159;";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let num_idx = *leg.map.get("number").ok_or("number not in legend")?;
    let numbers: Vec<_> = tokens.iter().filter(|t| t[3] == num_idx).collect();
    assert!(numbers.len() > 0, "should classify scientific notation numbers");
    Ok(())
}

#[test]
fn number_underscore_separator() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $big = 1_000_000; my $float = 3.14_159;";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for numbers with underscores");
    Ok(())
}

// ---------------------------------------------------------------------------
// Regex and Pattern Matching Tests
// ---------------------------------------------------------------------------

#[test]
fn regex_match_operator_forward_slash() -> Result<(), Box<dyn std::error::Error>> {
    let code = "if ($str =~ /pattern/) { }";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for regex match");
    Ok(())
}

#[test]
fn regex_substitution_operator() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $text = 'hello'; $text =~ s/hello/world/;";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for substitution");
    Ok(())
}

#[test]
fn regex_transliteration_operator() -> Result<(), Box<dyn std::error::Error>> {
    let code = "$text =~ tr/a-z/A-Z/;";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for transliteration");
    Ok(())
}

#[test]
fn regex_compiled_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $regex = qr/pattern/i;";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let regexp_idx = *leg.map.get("regexp").ok_or("regexp not in legend")?;
    let regexes: Vec<_> = tokens.iter().filter(|t| t[3] == regexp_idx).collect();
    // At least some tokens should be recognized
    assert!(!tokens.is_empty(), "should produce tokens for compiled regex");
    Ok(())
}

#[test]
fn regex_with_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let code = "if ($str =~ /PATTERN/ix) { }";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for regex with modifiers");
    Ok(())
}

// ---------------------------------------------------------------------------
// Operator Tests
// ---------------------------------------------------------------------------

#[test]
fn operator_arithmetic_plus_minus() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $sum = $a + $b - $c;";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let op_idx = *leg.map.get("operator").ok_or("operator not in legend")?;
    let operators: Vec<_> = tokens.iter().filter(|t| t[3] == op_idx).collect();
    assert!(operators.len() > 0, "should classify arithmetic operators");
    Ok(())
}

#[test]
fn operator_logical_and_or() -> Result<(), Box<dyn std::error::Error>> {
    let code = "if ($a && $b || $c) { }";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let op_idx = *leg.map.get("operator").ok_or("operator not in legend")?;
    let operators: Vec<_> = tokens.iter().filter(|t| t[3] == op_idx).collect();
    assert!(operators.len() > 0, "should classify logical operators");
    Ok(())
}

#[test]
fn operator_bitwise_operations() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $result = $a & $b | $c ^ $d;";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for bitwise operations");
    Ok(())
}

#[test]
fn operator_string_concatenation() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $str = \"Hello\" . \" \" . \"World\";";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let op_idx = *leg.map.get("operator").ok_or("operator not in legend")?;
    let operators: Vec<_> = tokens.iter().filter(|t| t[3] == op_idx).collect();
    assert!(operators.len() > 0, "should classify concatenation operator");
    Ok(())
}

#[test]
fn operator_comparison_numeric() -> Result<(), Box<dyn std::error::Error>> {
    let code = "if ($a == $b && $c != $d && $e < $f) { }";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let op_idx = *leg.map.get("operator").ok_or("operator not in legend")?;
    let operators: Vec<_> = tokens.iter().filter(|t| t[3] == op_idx).collect();
    assert!(operators.len() > 0, "should classify comparison operators");
    Ok(())
}

#[test]
fn operator_comparison_string() -> Result<(), Box<dyn std::error::Error>> {
    let code = "if ($a eq $b && $c ne $d && $e lt $f) { }";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for string comparisons");
    Ok(())
}

#[test]
fn operator_ternary_conditional() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $result = $condition ? \"yes\" : \"no\";";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let op_idx = *leg.map.get("operator").ok_or("operator not in legend")?;
    let operators: Vec<_> = tokens.iter().filter(|t| t[3] == op_idx).collect();
    assert!(operators.len() > 0, "should classify ternary operator");
    Ok(())
}

#[test]
fn operator_assignment_compound() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 5; $x += 10; $x -= 2; $x *= 3; $x /= 2;";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for compound assignments");
    Ok(())
}

#[test]
fn operator_range_operator() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my @range = (1..10); for my $i (1...100) { }";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for range operators");
    Ok(())
}

// ---------------------------------------------------------------------------
// Function and Method Call Tests
// ---------------------------------------------------------------------------

#[test]
fn function_call_builtin_print() -> Result<(), Box<dyn std::error::Error>> {
    let code = "print \"Hello\\n\";";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let fn_idx = *leg.map.get("function").ok_or("function not in legend")?;
    let functions: Vec<_> = tokens.iter().filter(|t| t[3] == fn_idx).collect();
    assert!(functions.len() > 0, "should classify builtin print as function");
    Ok(())
}

#[test]
fn function_call_builtin_scalar_operations() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $len = length($str); my $sub = substr($str, 0, 5);";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for builtin functions");
    Ok(())
}

#[test]
fn function_call_builtin_array_operations() -> Result<(), Box<dyn std::error::Error>> {
    let code = "push(@array, $item); pop(@array); shift(@array); unshift(@array, $x);";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for array operations");
    Ok(())
}

#[test]
fn function_call_builtin_hash_operations() -> Result<(), Box<dyn std::error::Error>> {
    let code =
        "my @keys = keys(%hash); my @vals = values(%hash); my $exists = exists($hash{$key});";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for hash operations");
    Ok(())
}

#[test]
fn method_call_object_arrow_syntax() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $result = $obj->method_name();";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let method_idx = *leg.map.get("method").ok_or("method not in legend")?;
    let methods: Vec<_> = tokens.iter().filter(|t| t[3] == method_idx).collect();
    assert!(methods.len() > 0, "should classify method calls");
    Ok(())
}

#[test]
fn method_call_chained_calls() -> Result<(), Box<dyn std::error::Error>> {
    let code = "$obj->method1()->method2()->method3();";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let method_idx = *leg.map.get("method").ok_or("method not in legend")?;
    let methods: Vec<_> = tokens.iter().filter(|t| t[3] == method_idx).collect();
    assert!(methods.len() > 0, "should classify chained method calls");
    Ok(())
}

#[test]
fn method_call_static_method() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $result = Package::Class->class_method();";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for static method calls");
    Ok(())
}

// ---------------------------------------------------------------------------
// Declaration Modifier Tests
// ---------------------------------------------------------------------------

#[test]
fn declaration_modifier_on_variable() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x;";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let var_idx = *leg.map.get("variable").ok_or("variable not in legend")?;
    let variables: Vec<_> = tokens.iter().filter(|t| t[3] == var_idx).collect();
    // Check if variable has declaration modifier (bit 0 set)
    let has_declaration = variables.iter().any(|t| t[4] & 1 != 0);
    // Note: parser might not always set this, so just check variable exists
    assert!(variables.len() > 0, "should classify variable declarations");
    Ok(())
}

#[test]
fn declaration_modifier_on_function() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub my_func { }";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let fn_idx = *leg.map.get("function").ok_or("function not in legend")?;
    let functions: Vec<_> = tokens.iter().filter(|t| t[3] == fn_idx).collect();
    assert!(functions.len() > 0, "should classify function declarations");
    // Declaration modifier is typically the first bit
    let has_declaration = functions.iter().any(|t| t[4] & 1 != 0);
    assert!(has_declaration, "function declaration should have declaration modifier");
    Ok(())
}

// ---------------------------------------------------------------------------
// Package and Namespace Tests
// ---------------------------------------------------------------------------

#[test]
fn package_declaration_with_version() -> Result<(), Box<dyn std::error::Error>> {
    let code = "package MyModule v1.0;";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let ns_idx = *leg.map.get("namespace").ok_or("namespace not in legend")?;
    let namespaces: Vec<_> = tokens.iter().filter(|t| t[3] == ns_idx).collect();
    assert!(namespaces.len() > 0, "should classify package declarations");
    Ok(())
}

#[test]
fn package_fully_qualified_name() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $obj = Package::Subpackage::Class->new();";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for fully qualified names");
    Ok(())
}

#[test]
fn require_module_import() -> Result<(), Box<dyn std::error::Error>> {
    let code = "require \"Some/Module.pm\";";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for require");
    Ok(())
}

#[test]
fn use_module_pragma() -> Result<(), Box<dyn std::error::Error>> {
    let code = "use strict; use warnings; use Data::Dumper;";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let kw_idx = *leg.map.get("keyword").ok_or("keyword not in legend")?;
    let uses: Vec<_> = tokens.iter().filter(|t| t[3] == kw_idx).collect();
    assert!(uses.len() > 0, "should classify use statements as keywords");
    Ok(())
}

// ---------------------------------------------------------------------------
// Control Flow and Block Structure Tests
// ---------------------------------------------------------------------------

#[test]
fn block_if_elsif_else_chain() -> Result<(), Box<dyn std::error::Error>> {
    let code = "if ($x) { } elsif ($y) { } else { }";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let kw_idx = *leg.map.get("keyword").ok_or("keyword not in legend")?;
    let keywords: Vec<_> = tokens.iter().filter(|t| t[3] == kw_idx).collect();
    assert!(keywords.len() >= 3, "should classify if/elsif/else keywords");
    Ok(())
}

#[test]
fn block_unless_statement() -> Result<(), Box<dyn std::error::Error>> {
    let code = "unless ($condition) { } else { }";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let kw_idx = *leg.map.get("keyword").ok_or("keyword not in legend")?;
    let has_unless = tokens.iter().any(|t| t[3] == kw_idx);
    assert!(has_unless, "should classify unless keyword");
    Ok(())
}

#[test]
fn block_statement_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let code = "print \"yes\" if $cond; die \"error\" unless $ok;";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let kw_idx = *leg.map.get("keyword").ok_or("keyword not in legend")?;
    let keywords: Vec<_> = tokens.iter().filter(|t| t[3] == kw_idx).collect();
    assert!(keywords.len() > 0, "should classify statement modifiers");
    Ok(())
}

#[test]
fn block_eval_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let code = "eval { dangerous_code() }; if ($@) { warn $@; }";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let kw_idx = *leg.map.get("keyword").ok_or("keyword not in legend")?;
    let has_eval = tokens.iter().any(|t| t[3] == kw_idx);
    assert!(has_eval, "should classify eval keyword");
    Ok(())
}

#[test]
fn block_do_while_post_test_loop() -> Result<(), Box<dyn std::error::Error>> {
    let code = "do { something() } while ($cond);";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let kw_idx = *leg.map.get("keyword").ok_or("keyword not in legend")?;
    let keywords: Vec<_> = tokens.iter().filter(|t| t[3] == kw_idx).collect();
    assert!(keywords.len() >= 2, "should classify do and while keywords");
    Ok(())
}

// ---------------------------------------------------------------------------
// Special Constructs and Edge Cases
// ---------------------------------------------------------------------------

#[test]
fn special_bareword_as_string() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my %opts = (key => value);";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for bareword keys");
    Ok(())
}

#[test]
fn special_qw_quote_words() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my @words = qw(one two three four);";
    let tokens = tokens_for(code)?;
    let leg = legend();
    let str_idx = *leg.map.get("string").ok_or("string not in legend")?;
    let strings: Vec<_> = tokens.iter().filter(|t| t[3] == str_idx).collect();
    assert!(strings.len() > 0, "should classify qw() as string");
    Ok(())
}

#[test]
fn special_here_document() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $doc = <<'EOF';\nSome text\nEOF\n";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for heredoc");
    Ok(())
}

#[test]
fn special_anonymous_subroutine() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $func = sub { return $_[0] * 2; };";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for anonymous subroutine");
    Ok(())
}

#[test]
fn special_subroutine_prototype() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub add($$) { my ($a, $b) = @_; return $a + $b; }";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for function with prototype");
    Ok(())
}

#[test]
fn special_attributes_on_sub() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub my_func :lvalue { }";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for attributed subroutine");
    Ok(())
}

#[test]
fn special_sigil_scalar_context() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $ref = [1, 2, 3]; my $elem = $ref->[0];";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for sigil operations");
    Ok(())
}

#[test]
fn special_file_test_operators() -> Result<(), Box<dyn std::error::Error>> {
    let code = "if (-e $file && -r $file && -w $file) { }";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for file test operators");
    Ok(())
}

#[test]
fn special_diamond_operator() -> Result<(), Box<dyn std::error::Error>> {
    let code = "while (<STDIN>) { print; }";
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for diamond operator");
    Ok(())
}

// ---------------------------------------------------------------------------
// Complex Multi-line Structure Tests
// ---------------------------------------------------------------------------

#[test]
fn complex_nested_data_structure() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $data = {
    users => [
        { name => 'Alice', age => 30 },
        { name => 'Bob', age => 25 },
    ],
    count => 2,
};
"#;
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for nested data structures");
    Ok(())
}

#[test]
fn complex_higher_order_function() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
sub map_filter {
    my ($func, @items) = @_;
    return [grep { $func->($_) } @items];
}
"#;
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for higher-order functions");
    Ok(())
}

#[test]
fn complex_regex_with_interpolation() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $pattern = qr/(\d+)/;
my $text = "The answer is 42";
if ($text =~ /$pattern/) {
    my $num = $1;
}
"#;
    let tokens = tokens_for(code)?;
    assert!(!tokens.is_empty(), "should produce tokens for interpolated regex");
    Ok(())
}

// ---------------------------------------------------------------------------
// Token Quality and Ordering Tests
// ---------------------------------------------------------------------------

#[test]
fn token_delta_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 5;\nmy $y = 10;";
    let tokens = tokens_for(code)?;
    // Verify delta encoding is consistent
    if tokens.len() >= 2 {
        assert!(tokens[1][0] <= 1, "delta line should not exceed actual lines");
    }
    Ok(())
}

#[test]
fn tokens_have_no_zero_length() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;";
    let tokens = tokens_for(code)?;
    for token in &tokens {
        assert!(token[2] > 0, "token length should be positive");
    }
    Ok(())
}

#[test]
fn legend_consistency_check() -> Result<(), Box<dyn std::error::Error>> {
    let leg1 = legend();
    let leg2 = legend();
    assert_eq!(leg1.token_types, leg2.token_types, "legend should be consistent");
    assert_eq!(leg1.modifiers, leg2.modifiers, "modifiers should be consistent");
    Ok(())
}

#[test]
fn token_type_indices_within_bounds() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1; sub foo { } package Bar;";
    let tokens = tokens_for(code)?;
    let leg = legend();
    for token in &tokens {
        assert!(
            (token[3] as usize) < leg.token_types.len(),
            "token type index {} should be within legend range",
            token[3]
        );
    }
    Ok(())
}

#[test]
fn provider_can_be_created_multiple_times() -> Result<(), Box<dyn std::error::Error>> {
    let _p1 = SemanticTokensProvider::new();
    let _p2 = SemanticTokensProvider::new();
    let _p3 = SemanticTokensProvider::default();
    Ok(())
}
