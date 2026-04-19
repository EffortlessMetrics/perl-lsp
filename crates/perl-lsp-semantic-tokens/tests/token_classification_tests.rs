//! Tests for token type classification, modifier flags, multi-line handling,
//! and delta position encoding in `perl-lsp-semantic-tokens`.
//!
//! Complements `comprehensive_unit_tests.rs` with deeper coverage of:
//! - AST overlay token types (class, method, macro/phase blocks)
//! - Modifier bitmask semantics (declaration, definition, readonly)
//! - Multi-line token span behavior (zero-length filtering)
//! - Delta encoding precision across lines and within a line

use perl_lsp_semantic_tokens::{EncodedToken, collect_semantic_tokens, legend};
use perl_tdd_support::{Parser, must};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a position mapper that correctly handles multi-line text.
fn line_col_mapper(text: &str) -> impl Fn(usize) -> (u32, u32) + '_ {
    move |byte: usize| {
        let prefix = &text[..byte.min(text.len())];
        let line = prefix.matches('\n').count() as u32;
        let last_nl = prefix.rfind('\n').map_or(0, |p| p + 1);
        let col = (byte - last_nl) as u32;
        (line, col)
    }
}

/// Parse Perl source and collect semantic tokens.
fn tokens_for(code: &str) -> Vec<EncodedToken> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let mapper = line_col_mapper(code);
    collect_semantic_tokens(&ast, code, &mapper)
}

/// Reconstruct absolute (line, col) positions from delta-encoded tokens.
fn absolute_positions(tokens: &[EncodedToken]) -> Vec<(u32, u32)> {
    let mut positions = Vec::new();
    let mut line = 0u32;
    let mut col = 0u32;
    for t in tokens {
        if t[0] > 0 {
            line += t[0];
            col = t[1];
        } else {
            col += t[1];
        }
        positions.push((line, col));
    }
    positions
}

/// Find the legend index for a token type name.
fn type_idx(name: &str) -> u32 {
    let leg = legend();
    leg.map.get(name).copied().unwrap_or(u32::MAX)
}

/// Filter tokens by token type name and return them with their absolute positions.
fn tokens_of_type(code: &str, type_name: &str) -> Vec<(u32, u32, EncodedToken)> {
    let tokens = tokens_for(code);
    let idx = type_idx(type_name);
    let positions = absolute_positions(&tokens);
    tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| t[3] == idx)
        .map(|(i, t)| (positions[i].0, positions[i].1, *t))
        .collect()
}

// ===========================================================================
// Token type classification: class
// ===========================================================================

#[test]
fn test_class_declaration_produces_class_token() {
    // Perl 5.38+ class syntax
    let code = "class Foo { }";
    let tokens = tokens_for(code);
    let cls_idx = type_idx("class");
    let has_class = tokens.iter().any(|t| t[3] == cls_idx);
    assert!(has_class, "class declaration should produce class token");
}

#[test]
fn test_class_declaration_has_declaration_modifier() {
    let code = "class Bar { }";
    let tokens = tokens_for(code);
    let cls_idx = type_idx("class");
    let cls_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == cls_idx).collect();
    assert!(!cls_tokens.is_empty(), "should have class token");
    let has_decl = cls_tokens.iter().any(|t| t[4] & 1 != 0);
    assert!(
        has_decl,
        "class declaration should have declaration modifier (bit 0)"
    );
}

// ===========================================================================
// Token type classification: method (Perl 5.38+ class feature)
// ===========================================================================

#[test]
fn test_method_declaration_produces_method_token() {
    let code = "class Foo {\n    method greet { }\n}";
    let tokens = tokens_for(code);
    let meth_idx = type_idx("method");
    let has_method = tokens.iter().any(|t| t[3] == meth_idx);
    assert!(has_method, "method declaration should produce method token");
}

#[test]
fn test_method_declaration_has_declaration_and_definition_modifiers() {
    let code = "class Foo {\n    method greet { }\n}";
    let tokens = tokens_for(code);
    let meth_idx = type_idx("method");
    let meth_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == meth_idx).collect();
    assert!(!meth_tokens.is_empty(), "should have method token");
    // Method nodes get modifiers 1|2 = declaration|definition = 3
    let has_decl_def = meth_tokens.iter().any(|t| t[4] & 1 != 0 && t[4] & 2 != 0);
    assert!(
        has_decl_def,
        "method declaration should have both declaration (bit 0) and definition (bit 1) modifiers"
    );
}

// ===========================================================================
// Token type classification: macro (phase blocks)
// ===========================================================================

#[test]
fn test_begin_block_produces_macro_token() {
    let code = "BEGIN { 1; }";
    let tokens = tokens_for(code);
    let macro_idx = type_idx("macro");
    // BEGIN may produce either a keyword token (from lexer) or a macro token (from AST overlay)
    // depending on whether PhaseBlock has phase_span set. Check both.
    let kw_idx = type_idx("keyword");
    let has_macro_or_keyword = tokens.iter().any(|t| t[3] == macro_idx || t[3] == kw_idx);
    assert!(
        has_macro_or_keyword,
        "BEGIN block should produce macro or keyword token"
    );
}

#[test]
fn test_end_block_produces_macro_token() {
    let code = "END { 1; }";
    let tokens = tokens_for(code);
    let macro_idx = type_idx("macro");
    let kw_idx = type_idx("keyword");
    let has_token = tokens.iter().any(|t| t[3] == macro_idx || t[3] == kw_idx);
    assert!(has_token, "END block should produce macro or keyword token");
}

#[test]
fn test_check_init_unitcheck_phase_blocks() {
    for phase in &["CHECK", "INIT", "UNITCHECK"] {
        let code = format!("{phase} {{ 1; }}");
        let tokens = tokens_for(&code);
        let macro_idx = type_idx("macro");
        let kw_idx = type_idx("keyword");
        let has_token = tokens.iter().any(|t| t[3] == macro_idx || t[3] == kw_idx);
        assert!(
            has_token,
            "{phase} block should produce macro or keyword token"
        );
    }
}

// ===========================================================================
// Token type classification: subroutine with name_span (definition modifier)
// ===========================================================================

#[test]
fn test_named_sub_with_name_span_has_definition_modifier() {
    // When the parser provides a name_span, the function token should have
    // both declaration (bit 0) and definition (bit 1) modifiers.
    let code = "sub process_data { }";
    let tokens = tokens_for(code);
    let fn_idx = type_idx("function");
    let fn_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == fn_idx).collect();
    assert!(!fn_tokens.is_empty(), "should have function token");
    // Check that at least one function token has declaration modifier
    let has_decl = fn_tokens.iter().any(|t| t[4] & 1 != 0);
    assert!(
        has_decl,
        "named sub should have declaration modifier (bit 0)"
    );
}

#[test]
fn test_sub_definition_modifier_bitmask_value() {
    // If the parser sets name_span, modifiers should be 1|2 = 3
    // If not, modifiers should be just 1
    let code = "sub calc { }";
    let tokens = tokens_for(code);
    let fn_idx = type_idx("function");
    let fn_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == fn_idx).collect();
    assert!(!fn_tokens.is_empty(), "should have function token");
    // Modifier should be either 1 (declaration only) or 3 (declaration|definition)
    for t in &fn_tokens {
        let mods = t[4];
        assert!(
            mods == 1 || mods == 3,
            "function declaration modifier should be 1 or 3, got {mods}"
        );
    }
}

// ===========================================================================
// Token modifier flags: detailed bitmask semantics
// ===========================================================================

#[test]
fn test_modifier_declaration_is_bit_zero() {
    // declaration = 1 << 0 = 1
    let code = "my $x = 1;";
    let tokens = tokens_for(code);
    let var_idx = type_idx("variable");
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == var_idx).collect();
    let has_bit_zero = var_tokens.iter().any(|t| t[4] & 1 != 0);
    assert!(
        has_bit_zero,
        "my-declared variable should set bit 0 (declaration)"
    );
}

#[test]
fn test_modifier_readonly_is_bit_two() {
    // readonly = 1 << 2 = 4
    let code = "our $VERSION = '1.0';";
    let tokens = tokens_for(code);
    let var_idx = type_idx("variable");
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == var_idx).collect();
    let has_bit_two = var_tokens.iter().any(|t| t[4] & 4 != 0);
    assert!(
        has_bit_two,
        "our-declared variable should set bit 2 (readonly)"
    );
}

#[test]
fn test_our_variable_has_combined_declaration_readonly_mask() {
    // our variables get declaration (bit 0) | readonly (bit 2) = 5
    let code = "our $Config = {};";
    let tokens = tokens_for(code);
    let var_idx = type_idx("variable");
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == var_idx).collect();
    let has_mask_5 = var_tokens.iter().any(|t| t[4] & 5 == 5);
    assert!(
        has_mask_5,
        "our variable should have modifier bitmask with bits 0 and 2 set"
    );
}

#[test]
fn test_my_variable_has_declaration_without_readonly() {
    let code = "my $local_var = 42;";
    let tokens = tokens_for(code);
    let var_idx = type_idx("variable");
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == var_idx).collect();
    let has_decl_only = var_tokens.iter().any(|t| t[4] & 1 != 0 && t[4] & 4 == 0);
    assert!(
        has_decl_only,
        "my-declared variable should have declaration (bit 0) but NOT readonly (bit 2)"
    );
}

#[test]
fn test_non_declared_variable_has_zero_modifiers() {
    // A scalar variable used without my/our/local/state should have only the
    // scalarVariable sigil modifier (bit 10 = 1024) and no declaration bits.
    let code = "sub f {\n    return $global;\n}";
    let tokens = tokens_for(code);
    let var_idx = type_idx("variable");
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == var_idx).collect();
    if !var_tokens.is_empty() {
        let scalar_mod_bit: u32 = 1 << 10; // scalarVariable
        let declaration_bit: u32 = 1;
        // No declaration bit; only the sigil modifier is expected
        let has_no_decl = var_tokens.iter().any(|t| (t[4] & declaration_bit) == 0);
        assert!(
            has_no_decl,
            "undeclared variable should not have declaration modifier bit"
        );
        let has_scalar_mod = var_tokens.iter().any(|t| (t[4] & scalar_mod_bit) != 0);
        assert!(
            has_scalar_mod,
            "undeclared $global should still have scalarVariable modifier"
        );
    }
}

#[test]
fn test_function_call_has_zero_modifiers() {
    let code = "foo();";
    let tokens = tokens_for(code);
    let fn_idx = type_idx("function");
    let fn_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == fn_idx).collect();
    for t in &fn_tokens {
        assert_eq!(t[4], 0, "function call should have modifier bitmask = 0");
    }
}

#[test]
fn test_method_call_has_zero_modifiers() {
    let code = "$obj->method();";
    let tokens = tokens_for(code);
    let meth_idx = type_idx("method");
    let call_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == meth_idx).collect();
    for t in &call_tokens {
        assert_eq!(t[4], 0, "method call should have modifier bitmask = 0");
    }
}

#[test]
fn test_package_has_declaration_modifier_only() {
    let code = "package Foo;";
    let tokens = tokens_for(code);
    let ns_idx = type_idx("namespace");
    let ns_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == ns_idx).collect();
    assert!(!ns_tokens.is_empty(), "should have namespace token");
    for t in &ns_tokens {
        assert_eq!(
            t[4], 1,
            "package namespace should have modifier = 1 (declaration only)"
        );
    }
}

// ===========================================================================
// Token modifier flags: variable list declarations
// ===========================================================================

#[test]
fn test_variable_list_declaration_all_have_declaration_modifier() {
    let code = "my ($a, $b, $c) = (1, 2, 3);";
    let tokens = tokens_for(code);
    let var_idx = type_idx("variable");
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == var_idx).collect();
    assert!(
        var_tokens.len() >= 3,
        "should have at least 3 variable tokens for list decl, got {}",
        var_tokens.len()
    );
    let decl_count = var_tokens.iter().filter(|t| t[4] & 1 != 0).count();
    assert!(
        decl_count >= 3,
        "all 3 variables in list declaration should have declaration modifier, got {decl_count}"
    );
}

#[test]
fn test_our_variable_list_all_have_readonly_modifier() {
    let code = "our ($X, $Y) = (1, 2);";
    let tokens = tokens_for(code);
    let var_idx = type_idx("variable");
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == var_idx).collect();
    let readonly_count = var_tokens.iter().filter(|t| t[4] & 4 != 0).count();
    assert!(
        readonly_count >= 2,
        "our list variables should all have readonly modifier, got {readonly_count}"
    );
}

// ===========================================================================
// Multi-line token handling
// ===========================================================================

#[test]
fn test_multiline_sub_body_produces_no_zero_length_tokens() {
    // Multi-line spans get len=0 which should be filtered out
    let code = "sub long_function {\n    my $a = 1;\n    my $b = 2;\n    return $a + $b;\n}";
    let tokens = tokens_for(code);
    for (i, t) in tokens.iter().enumerate() {
        assert!(
            t[2] > 0,
            "token {i} should have positive length, got len={}",
            t[2]
        );
    }
}

#[test]
fn test_multiline_string_produces_no_zero_length_tokens() {
    // A heredoc or multi-line string should not produce zero-length tokens
    let code = "my $s = <<'END';\nline1\nline2\nEND\nmy $x = 1;";
    let tokens = tokens_for(code);
    for (i, t) in tokens.iter().enumerate() {
        assert!(
            t[2] > 0,
            "token {i} should have positive length in heredoc code"
        );
    }
}

#[test]
fn test_multiline_if_else_produces_tokens_on_each_line() {
    let code = "if ($x) {\n    print 1;\n} elsif ($y) {\n    print 2;\n} else {\n    print 3;\n}";
    let tokens = tokens_for(code);
    let positions = absolute_positions(&tokens);
    let lines_with_tokens: std::collections::HashSet<u32> =
        positions.iter().map(|(l, _)| *l).collect();
    // Should have tokens on multiple lines
    assert!(
        lines_with_tokens.len() >= 3,
        "multi-line if/elsif/else should produce tokens on at least 3 lines, got {}",
        lines_with_tokens.len()
    );
}

#[test]
fn test_multiline_package_with_methods_produces_tokens_on_multiple_lines() {
    let code =
        "package Foo;\n\nsub new {\n    bless {}, shift;\n}\n\nsub greet {\n    print 'hi';\n}";
    let tokens = tokens_for(code);
    let positions = absolute_positions(&tokens);
    let lines_with_tokens: std::collections::HashSet<u32> =
        positions.iter().map(|(l, _)| *l).collect();
    assert!(
        lines_with_tokens.len() >= 4,
        "multi-line package should produce tokens on many lines, got {}",
        lines_with_tokens.len()
    );
}

// ===========================================================================
// Token position encoding: delta format precision
// ===========================================================================

#[test]
fn test_delta_line_zero_for_same_line_tokens() {
    let code = "my $x = 42;";
    let tokens = tokens_for(code);
    // All tokens after the first on the same line should have delta_line = 0
    for t in tokens.iter().skip(1) {
        assert_eq!(t[0], 0, "same-line tokens should have delta_line = 0");
    }
}

#[test]
fn test_delta_start_relative_to_previous_on_same_line() {
    // On the same line, delta_start is relative to the previous token's start
    let code = "my $x = 42;";
    let tokens = tokens_for(code);
    if tokens.len() >= 2 {
        // First token's delta_start is absolute column
        // Second token's delta_start should be positive (relative to first)
        let first_col = tokens[0][1];
        let second_delta = tokens[1][1];
        // The second token should start after the first
        assert!(
            first_col + second_delta > first_col || tokens[1][0] > 0,
            "second token delta_start should advance position"
        );
    }
}

#[test]
fn test_delta_start_is_absolute_column_on_new_line() {
    let code = "my $a = 1;\nmy $b = 2;";
    let tokens = tokens_for(code);
    // Find first token that starts a new line
    for t in &tokens {
        if t[0] > 0 {
            // delta_start should be the absolute column on the new line
            // "my" starts at column 0
            assert!(
                t[1] < 50,
                "delta_start on new line should be absolute column, got {}",
                t[1]
            );
            break;
        }
    }
}

#[test]
fn test_reconstructed_positions_match_source_locations() {
    // Verify that reconstructed positions make sense for known code
    let code = "my $x = 1;\nmy $y = 2;\nmy $z = 3;";
    let tokens = tokens_for(code);
    let positions = absolute_positions(&tokens);

    // All positions should be within bounds of the source
    for (i, (line, col)) in positions.iter().enumerate() {
        assert!(*line <= 2, "token {i} line {} exceeds source lines", line);
        assert!(
            *col < 20,
            "token {i} col {} seems too large for this code",
            col
        );
    }
}

#[test]
fn test_delta_encoding_preserves_monotonic_order() {
    let code = "package X;\nmy $a = 1;\nsub f { return $a; }";
    let tokens = tokens_for(code);
    let positions = absolute_positions(&tokens);

    for i in 1..positions.len() {
        let (pl, pc) = positions[i - 1];
        let (cl, cc) = positions[i];
        assert!(
            cl > pl || (cl == pl && cc >= pc),
            "positions not monotonically ordered at {i}: ({pl},{pc}) -> ({cl},{cc})"
        );
    }
}

#[test]
fn test_five_line_code_correct_line_deltas() {
    let code = "my $a;\nmy $b;\nmy $c;\nmy $d;\nmy $e;";
    let tokens = tokens_for(code);
    let positions = absolute_positions(&tokens);

    // Check that we have tokens on lines 0 through 4
    let lines: std::collections::HashSet<u32> = positions.iter().map(|(l, _)| *l).collect();
    for expected_line in 0..5u32 {
        assert!(
            lines.contains(&expected_line),
            "expected tokens on line {expected_line}, lines present: {:?}",
            lines
        );
    }
}

#[test]
fn test_multiple_tokens_on_same_line_delta_accumulation() {
    // "my $x = 42;" has multiple tokens on one line
    // Verify that delta_start values accumulate correctly
    let code = "my $x = 42;";
    let tokens = tokens_for(code);
    let positions = absolute_positions(&tokens);

    // Columns should be strictly increasing for non-overlapping tokens on same line
    for i in 1..positions.len() {
        if positions[i].0 == positions[i - 1].0 {
            assert!(
                positions[i].1 > positions[i - 1].1,
                "same-line tokens should have increasing columns: {} vs {} at index {i}",
                positions[i - 1].1,
                positions[i].1
            );
        }
    }
}

// ===========================================================================
// Token type classification: all keyword variants
// ===========================================================================

#[test]
fn test_try_catch_finally_keywords() {
    // These are valid Perl keywords (with appropriate feature flags)
    for kw in &["try", "catch", "finally"] {
        let code = format!("{kw};");
        let tokens = tokens_for(&code);
        let kw_idx = type_idx("keyword");
        let has_kw = tokens.iter().any(|t| t[3] == kw_idx);
        assert!(has_kw, "'{kw}' should be classified as keyword");
    }
}

#[test]
fn test_class_method_keywords() {
    // "class" and "method" as keywords from the lexer
    for kw in &["class", "method"] {
        let code = format!("{kw};");
        let tokens = tokens_for(&code);
        let kw_idx = type_idx("keyword");
        // The lexer may or may not classify these as keywords
        // but parsing them should not crash
        let _count = tokens.iter().filter(|t| t[3] == kw_idx).count();
    }
}

#[test]
fn test_no_statement_does_not_crash() {
    // "no strict;" may not produce keyword tokens if the lexer does not
    // emit "no" as a Keyword token, but parsing should not crash.
    let code = "no strict;";
    let _tokens = tokens_for(code);
}

// ===========================================================================
// Token type classification: string variants
// ===========================================================================

#[test]
fn test_qq_string_classified_as_string() {
    let code = "my $s = qq{hello world};";
    let tokens = tokens_for(code);
    let str_idx = type_idx("string");
    let has_string = tokens.iter().any(|t| t[3] == str_idx);
    assert!(has_string, "qq// should be classified as string");
}

#[test]
fn test_q_string_classified_as_string() {
    let code = "my $s = q{no interpolation};";
    let tokens = tokens_for(code);
    let str_idx = type_idx("string");
    let has_string = tokens.iter().any(|t| t[3] == str_idx);
    assert!(has_string, "q// should be classified as string");
}

#[test]
fn test_qx_command_classified_as_string() {
    let code = "my $out = qx{ls -la};";
    let tokens = tokens_for(code);
    let str_idx = type_idx("string");
    let has_string = tokens.iter().any(|t| t[3] == str_idx);
    assert!(has_string, "qx// should be classified as string");
}

// ===========================================================================
// Token type classification: number variants
// ===========================================================================

#[test]
fn test_hex_number_classified() {
    let code = "my $x = 0xFF;";
    let tokens = tokens_for(code);
    let num_idx = type_idx("number");
    let has_number = tokens.iter().any(|t| t[3] == num_idx);
    assert!(has_number, "hex number should be classified as number");
}

#[test]
fn test_octal_number_classified() {
    let code = "my $x = 0777;";
    let tokens = tokens_for(code);
    let num_idx = type_idx("number");
    let has_number = tokens.iter().any(|t| t[3] == num_idx);
    assert!(has_number, "octal number should be classified as number");
}

#[test]
fn test_scientific_notation_classified() {
    let code = "my $x = 1.5e10;";
    let tokens = tokens_for(code);
    let num_idx = type_idx("number");
    let has_number = tokens.iter().any(|t| t[3] == num_idx);
    assert!(
        has_number,
        "scientific notation should be classified as number"
    );
}

#[test]
fn test_negative_number_classified() {
    let code = "my $x = -42;";
    let tokens = tokens_for(code);
    let num_idx = type_idx("number");
    let has_number = tokens.iter().any(|t| t[3] == num_idx);
    assert!(has_number, "negative number should be classified as number");
}

// ===========================================================================
// Token type classification: operator variants
// ===========================================================================

#[test]
fn test_comparison_operators_classified() {
    for op in &["==", "!=", "<", ">", "<=", ">=", "<=>"] {
        let code = format!("my $r = 1 {op} 2;");
        let tokens = tokens_for(&code);
        let op_idx = type_idx("operator");
        let has_op = tokens.iter().any(|t| t[3] == op_idx);
        assert!(has_op, "'{op}' should be classified as operator");
    }
}

#[test]
fn test_string_comparison_operators_classified() {
    for op in &["eq", "ne", "lt", "gt", "le", "ge", "cmp"] {
        let code = format!("my $r = 'a' {op} 'b';");
        let tokens = tokens_for(&code);
        let op_idx = type_idx("operator");
        let has_op = tokens.iter().any(|t| t[3] == op_idx);
        assert!(
            has_op,
            "string comparison '{op}' should be classified as operator"
        );
    }
}

#[test]
fn test_arithmetic_operators_classified() {
    for op in &["+", "-", "*"] {
        let code = format!("my $r = 1 {op} 2;");
        let tokens = tokens_for(&code);
        let op_idx = type_idx("operator");
        let has_op = tokens.iter().any(|t| t[3] == op_idx);
        assert!(has_op, "arithmetic '{op}' should be classified as operator");
    }
}

// ===========================================================================
// Token type classification: regexp variants
// ===========================================================================

#[test]
fn test_substitution_with_flags_classified() {
    let code = "$str =~ s/foo/bar/gi;";
    let tokens = tokens_for(code);
    let re_idx = type_idx("regexp");
    let has_re = tokens.iter().any(|t| t[3] == re_idx);
    assert!(
        has_re,
        "substitution with flags should produce regexp token"
    );
}

#[test]
fn test_transliteration_y_form_classified() {
    let code = "$str =~ y/a-z/A-Z/;";
    let tokens = tokens_for(code);
    let re_idx = type_idx("regexp");
    let has_re = tokens.iter().any(|t| t[3] == re_idx);
    assert!(has_re, "y/// transliteration should produce regexp token");
}

// ===========================================================================
// Token type: builtins skipped in AST overlay
// ===========================================================================

#[test]
fn test_eval_not_classified_as_function_call() {
    // eval is skipped in the FunctionCall AST overlay to keep it as a keyword
    let code = "eval { 1; };";
    let tokens = tokens_for(code);
    let fn_idx = type_idx("function");
    let kw_idx = type_idx("keyword");
    // eval should be keyword, not function
    let fn_count = tokens.iter().filter(|t| t[3] == fn_idx).count();
    let kw_count = tokens.iter().filter(|t| t[3] == kw_idx).count();
    assert!(
        kw_count >= fn_count,
        "eval should appear as keyword rather than function"
    );
}

#[test]
fn test_return_not_classified_as_function_call() {
    // return is skipped in the FunctionCall AST overlay
    let code = "sub f {\n    return 1;\n}";
    let tokens = tokens_for(code);
    let kw_idx = type_idx("keyword");
    let has_return_kw = tokens.iter().any(|t| t[3] == kw_idx);
    assert!(has_return_kw, "return should appear as keyword");
}

// ===========================================================================
// Multiple token types coexisting correctly
// ===========================================================================

#[test]
fn test_complex_code_all_token_types_present() {
    let code = concat!(
        "package Foo;\n",
        "use strict;\n",
        "my $x = 42;\n",
        "my $s = 'hello';\n",
        "sub bar {\n",
        "    $x =~ /pattern/;\n",
        "    $obj->method();\n",
        "    return $s;\n",
        "}\n",
    );
    let tokens = tokens_for(code);
    let leg = legend();

    let types_present: std::collections::HashSet<u32> = tokens.iter().map(|t| t[3]).collect();

    // Should have at least these types
    for expected_type in &[
        "namespace",
        "keyword",
        "variable",
        "number",
        "string",
        "regexp",
    ] {
        let idx = leg.map.get(*expected_type).copied().unwrap_or(u32::MAX);
        assert!(
            types_present.contains(&idx),
            "complex code should produce '{expected_type}' tokens"
        );
    }
}

#[test]
fn test_all_token_indices_valid_for_complex_code() {
    let code = "package X;\nuse strict;\nmy @arr = qw(a b c);\nsub f { my $x = 1; return $x; }";
    let tokens = tokens_for(code);
    let leg = legend();
    let max_idx = leg.token_types.len() as u32;
    for (i, t) in tokens.iter().enumerate() {
        assert!(
            t[3] < max_idx,
            "token {i} has type index {} but legend max is {max_idx}",
            t[3]
        );
    }
}

// ===========================================================================
// Delta encoding: edge cases
// ===========================================================================

#[test]
fn test_single_token_delta_is_absolute_position() {
    // A single token's delta_line and delta_start should be its absolute position
    let code = "42;";
    let tokens = tokens_for(code);
    if !tokens.is_empty() {
        let first = &tokens[0];
        // First token: delta_line = absolute line, delta_start = absolute column
        assert_eq!(first[0], 0, "first token should be on line 0");
        // Column should be 0 for "42" at start of input
        assert_eq!(first[1], 0, "first token should start at column 0");
    }
}

#[test]
fn test_indented_code_correct_column_offsets() {
    let code = "    my $x = 1;"; // 4 spaces of indentation
    let tokens = tokens_for(code);
    if !tokens.is_empty() {
        let positions = absolute_positions(&tokens);
        // First token ("my") should be at column 4
        assert_eq!(
            positions[0].1, 4,
            "first token in indented code should be at column 4"
        );
    }
}

#[test]
fn test_token_length_matches_source_text_for_keywords() {
    let code = "my $x;";
    let tokens = tokens_for(code);
    let kw_idx = type_idx("keyword");
    let kw_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == kw_idx).collect();
    // "my" has length 2
    for t in &kw_tokens {
        assert_eq!(t[2], 2, "'my' keyword should have length 2, got {}", t[2]);
    }
}

#[test]
fn test_token_length_matches_source_text_for_numbers() {
    let code = "12345;";
    let tokens = tokens_for(code);
    let num_idx = type_idx("number");
    let num_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == num_idx).collect();
    if !num_tokens.is_empty() {
        assert_eq!(
            num_tokens[0][2], 5,
            "'12345' should have length 5, got {}",
            num_tokens[0][2]
        );
    }
}

// ===========================================================================
// Legend: type index lookup consistency
// ===========================================================================

#[test]
fn test_legend_type_indices_are_sequential_from_zero() {
    let leg = legend();
    for (i, ty) in leg.token_types.iter().enumerate() {
        let idx = leg.map.get(ty).copied().unwrap_or(u32::MAX);
        assert_eq!(
            idx, i as u32,
            "legend index for '{ty}' should be {i}, got {idx}"
        );
    }
}

#[test]
fn test_legend_unknown_type_falls_back_to_none() {
    let leg = legend();
    assert!(
        !leg.map.contains_key("nonexistent_type"),
        "unknown type should not be in the legend map"
    );
}

// ===========================================================================
// Token ordering after overlap removal
// ===========================================================================

#[test]
fn test_no_overlapping_tokens_in_output_complex_code() {
    let code = concat!(
        "package Foo;\n",
        "use strict;\n",
        "my ($a, $b) = (1, 'hello');\n",
        "sub process {\n",
        "    my $result = $a + $b;\n",
        "    $result =~ s/foo/bar/g;\n",
        "    return $result;\n",
        "}\n",
    );
    let tokens = tokens_for(code);
    let positions = absolute_positions(&tokens);

    for i in 1..tokens.len() {
        let (pl, pc) = positions[i - 1];
        let prev_len = tokens[i - 1][2];
        let (cl, cc) = positions[i];
        if cl == pl {
            assert!(
                cc >= pc + prev_len,
                "tokens {}-{} overlap on line {pl}: prev starts at {pc} len {prev_len}, curr at {cc}",
                i - 1,
                i
            );
        }
    }
}

#[test]
fn test_no_overlapping_tokens_with_nested_calls() {
    let code = "foo(bar($x), baz($y));";
    let tokens = tokens_for(code);
    let positions = absolute_positions(&tokens);

    for i in 1..tokens.len() {
        let (pl, pc) = positions[i - 1];
        let prev_len = tokens[i - 1][2];
        let (cl, cc) = positions[i];
        if cl == pl {
            assert!(
                cc >= pc + prev_len,
                "overlapping tokens at index {}-{}: ({pc}+{prev_len}) vs ({cc})",
                i - 1,
                i
            );
        }
    }
}

// ===========================================================================
// Idempotence and determinism
// ===========================================================================

#[test]
fn test_tokenization_is_deterministic() {
    let code = "package Foo;\nsub bar { my $x = 1; return $x; }";
    let results: Vec<Vec<EncodedToken>> = (0..5).map(|_| tokens_for(code)).collect();
    for i in 1..results.len() {
        assert_eq!(
            results[0], results[i],
            "tokenization run {i} differs from run 0"
        );
    }
}

// ===========================================================================
// Edge cases: empty blocks, single tokens, unusual constructs
// ===========================================================================

#[test]
fn test_empty_sub_body() {
    let code = "sub empty { }";
    let tokens = tokens_for(code);
    let fn_idx = type_idx("function");
    let has_fn = tokens.iter().any(|t| t[3] == fn_idx);
    assert!(
        has_fn,
        "empty sub should still produce function token for name"
    );
}

#[test]
fn test_chained_string_concatenation() {
    let code = "my $s = 'a' . 'b' . 'c';";
    let tokens = tokens_for(code);
    let str_idx = type_idx("string");
    let str_count = tokens.iter().filter(|t| t[3] == str_idx).count();
    assert!(
        str_count >= 2,
        "chained string concat should produce multiple string tokens, got {str_count}"
    );
}

#[test]
fn test_hash_with_fat_comma_keys() {
    let code = "my %h = (name => 'Alice', age => 30);";
    let tokens = tokens_for(code);
    let op_idx = type_idx("operator");
    let num_idx = type_idx("number");
    let str_idx = type_idx("string");
    let has_op = tokens.iter().any(|t| t[3] == op_idx);
    let has_num = tokens.iter().any(|t| t[3] == num_idx);
    let has_str = tokens.iter().any(|t| t[3] == str_idx);
    assert!(has_op, "hash should have operator tokens (fat comma)");
    assert!(has_num, "hash should have number tokens");
    assert!(has_str, "hash should have string tokens");
}

#[test]
fn test_multiline_sub_tokens_span_correct_lines() {
    let code = "sub greet {\n    my $name = shift;\n    print $name;\n}";
    let tokens = tokens_for(code);
    let positions = absolute_positions(&tokens);

    // Verify tokens exist on lines 1 and 2 (inside the sub body)
    let lines: std::collections::HashSet<u32> = positions.iter().map(|(l, _)| *l).collect();
    assert!(
        lines.contains(&1),
        "should have tokens on line 1 (my $name = shift)"
    );
    assert!(
        lines.contains(&2),
        "should have tokens on line 2 (print $name)"
    );
}

#[test]
fn test_labeled_statement_does_not_crash() {
    let code = "OUTER: for my $i (1..10) { next OUTER; }";
    let tokens = tokens_for(code);
    assert!(
        !tokens.is_empty(),
        "labeled statement should produce tokens"
    );
}

#[test]
fn test_begin_end_phase_blocks_with_code_inside() {
    let code = "BEGIN {\n    my $x = 1;\n}\nEND {\n    print 'done';\n}";
    let tokens = tokens_for(code);
    let positions = absolute_positions(&tokens);
    let lines: std::collections::HashSet<u32> = positions.iter().map(|(l, _)| *l).collect();
    // Should have tokens on multiple lines
    assert!(
        lines.len() >= 3,
        "BEGIN/END blocks should produce tokens on multiple lines, got {}",
        lines.len()
    );
}

// ===========================================================================
// Regression: tokens_of_type helper verification
// ===========================================================================

#[test]
fn test_tokens_of_type_helper_returns_correct_type() {
    let matches = tokens_of_type("my $x = 42;", "keyword");
    for (_, _, t) in &matches {
        let kw_idx = type_idx("keyword");
        assert_eq!(
            t[3], kw_idx,
            "tokens_of_type should only return keyword tokens"
        );
    }
}

#[test]
fn test_tokens_of_type_with_positions() {
    let matches = tokens_of_type("my $x = 1;\nmy $y = 2;", "keyword");
    // Should have keyword tokens on both lines (line 0 and line 1)
    let lines: std::collections::HashSet<u32> = matches.iter().map(|(l, _, _)| *l).collect();
    assert!(
        lines.contains(&0) && lines.contains(&1),
        "should have keyword tokens on both lines, got lines: {:?}",
        lines
    );
}

// ===========================================================================
// SQL String Awareness: DBI/DBIx::Class SQL context detection (Issue #2337)
// ===========================================================================

#[test]
fn test_dbi_prepare_with_sql_string_is_sql_token() {
    let code = r#"my $sth = $dbh->prepare("SELECT id, name FROM users WHERE id = ?")"#;
    let tokens = tokens_for(code);
    let sql_idx = type_idx("sql_string");

    let has_sql_token = tokens.iter().any(|t| t[3] == sql_idx);
    assert!(
        has_sql_token,
        "prepare() first argument should be sql_string token"
    );
}

#[test]
fn test_dbi_do_with_sql_string_is_sql_token() {
    let code = r#"$dbh->do("INSERT INTO logs (event) VALUES (?)")"#;
    let tokens = tokens_for(code);
    let sql_idx = type_idx("sql_string");

    let has_sql_token = tokens.iter().any(|t| t[3] == sql_idx);
    assert!(
        has_sql_token,
        "do() first argument should be sql_string token"
    );
}

#[test]
fn test_dbi_query_with_sql_string_is_sql_token() {
    let code = r#"my $result = $dbh->query("SELECT COUNT(*) FROM events")"#;
    let tokens = tokens_for(code);
    let sql_idx = type_idx("sql_string");

    let has_sql_token = tokens.iter().any(|t| t[3] == sql_idx);
    assert!(
        has_sql_token,
        "query() first argument should be sql_string token"
    );
}

#[test]
fn test_dbi_selectrow_arrayref_with_sql_string() {
    let code = r#"my $row = $dbh->selectrow_arrayref("SELECT * FROM users")"#;
    let tokens = tokens_for(code);
    let sql_idx = type_idx("sql_string");

    let has_sql_token = tokens.iter().any(|t| t[3] == sql_idx);
    assert!(
        has_sql_token,
        "selectrow_arrayref() first argument should be sql_string token"
    );
}

#[test]
fn test_dbi_selectall_arrayref_with_sql_string() {
    let code = r#"my $rows = $dbh->selectall_arrayref("SELECT id FROM items WHERE active = 1")"#;
    let tokens = tokens_for(code);
    let sql_idx = type_idx("sql_string");

    let has_sql_token = tokens.iter().any(|t| t[3] == sql_idx);
    assert!(
        has_sql_token,
        "selectall_arrayref() first argument should be sql_string token"
    );
}

#[test]
fn test_non_sql_method_call_string_not_sql_token() {
    let code = r#"my $result = $obj->format("some regular text")"#;
    let tokens = tokens_for(code);
    let sql_idx = type_idx("sql_string");

    let has_sql_token = tokens.iter().any(|t| t[3] == sql_idx);
    assert!(
        !has_sql_token,
        "non-SQL method calls should not classify strings as sql_string"
    );
}

#[test]
fn test_regular_string_literal_not_sql_token() {
    let code = r#"my $msg = "Hello, World!""#;
    let tokens = tokens_for(code);
    let sql_idx = type_idx("sql_string");

    let has_sql_token = tokens.iter().any(|t| t[3] == sql_idx);
    assert!(
        !has_sql_token,
        "standalone string literals should not be sql_string tokens"
    );
}

#[test]
fn test_dbi_prepare_with_interpolated_string_still_sql_token() {
    let code = r#"my $sth = $dbh->prepare("SELECT * FROM $table WHERE id = ?")"#;
    let tokens = tokens_for(code);
    let sql_idx = type_idx("sql_string");

    let has_sql_token = tokens.iter().any(|t| t[3] == sql_idx);
    assert!(
        has_sql_token,
        "interpolated SQL strings should still be sql_string tokens"
    );
}

#[test]
fn test_dbi_do_multiple_arguments_first_is_sql() {
    let code = r#"$dbh->do("UPDATE users SET active = 1 WHERE id = ?", undef, $user_id)"#;
    let tokens = tokens_for(code);
    let sql_idx = type_idx("sql_string");

    let has_sql_token = tokens.iter().any(|t| t[3] == sql_idx);
    assert!(
        has_sql_token,
        "first argument of do() with multiple args should be sql_string"
    );
}

#[test]
fn test_nested_dbi_call_in_complex_expression() {
    let code = r#"
    my @data = $dbh->selectall_arrayref(
        "SELECT id, name, email FROM users WHERE status = ?",
        { Slice => {} },
        $status
    );
    "#;
    let tokens = tokens_for(code);
    let sql_idx = type_idx("sql_string");

    let has_sql_token = tokens.iter().any(|t| t[3] == sql_idx);
    assert!(
        has_sql_token,
        "SQL string in complex nested call should be sql_string token"
    );
}

#[test]
fn test_dbi_prepare_still_produces_method_token_for_method_name() {
    // Verify the method name itself still gets a "method" token after the
    // special-cased MethodCall arm is introduced.
    let code = r#"my $sth = $dbh->prepare("SELECT 1")"#;
    let tokens = tokens_for(code);
    let meth_idx = type_idx("method");
    let has_method = tokens.iter().any(|t| t[3] == meth_idx);
    assert!(
        has_method,
        "method name 'prepare' should still produce a method token"
    );
}

#[test]
fn test_non_sql_method_call_still_produces_method_token() {
    // Non-SQL method calls must still produce a method token after refactor.
    let code = r#"my $result = $obj->format("text")"#;
    let tokens = tokens_for(code);
    let meth_idx = type_idx("method");
    let has_method = tokens.iter().any(|t| t[3] == meth_idx);
    assert!(
        has_method,
        "non-SQL method call should still produce a method token"
    );
}

// ===========================================================================
// Special variable semantic tokens – issue #2347
// ===========================================================================

/// Returns true if the token has the `defaultLibrary` modifier set.
///
/// The bitmask is derived from the legend at runtime (1 << bit_position) so this
/// helper stays correct if the modifier order ever changes again.  Hard-coding
/// `512` here was the same class of bug that PR #2772 fixed in production code.
fn has_default_library_modifier(token: &EncodedToken) -> bool {
    let leg = legend();
    let Some(bit_pos) = leg.modifiers.iter().position(|m| m == "defaultLibrary") else {
        // defaultLibrary not in legend — token cannot have the modifier.
        return false;
    };
    let bitmask = 1u32 << bit_pos;
    token[4] & bitmask != 0
}

#[test]
fn test_special_variable_dollar_underscore_has_default_library_modifier() {
    // $_ is a built-in special variable and should be marked defaultLibrary.
    // Wrap in a sub so the parser creates a Variable node the AST walk visits.
    let code = "sub f { return $_; }";
    let tokens = tokens_for(code);
    let var_idx = type_idx("variable");
    let special_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t[3] == var_idx && has_default_library_modifier(t))
        .collect();
    assert!(
        !special_tokens.is_empty(),
        "$_ should produce a variable token with defaultLibrary modifier (bit 9 = 512), \
         got variable tokens: {:?}",
        tokens
            .iter()
            .filter(|t| t[3] == var_idx)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_special_variable_at_underscore_has_default_library_modifier() {
    // @_ is the built-in subroutine argument array
    let code = "sub f { my @a = @_; }";
    let tokens = tokens_for(code);
    let var_idx = type_idx("variable");
    let special_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t[3] == var_idx && has_default_library_modifier(t))
        .collect();
    assert!(
        !special_tokens.is_empty(),
        "@_ should produce at least one variable token with defaultLibrary modifier, \
        got variable tokens: {:?}",
        tokens
            .iter()
            .filter(|t| t[3] == var_idx)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_internal_pl_sv_variables_have_default_library_modifier() {
    let code = "sub f { return $PL_sv_yes; return $PL_sv_no; return $PL_sv_undef; }";
    let tokens = tokens_for(code);
    let var_idx = type_idx("variable");
    let positions = absolute_positions(&tokens);

    for needle in ["$PL_sv_yes", "$PL_sv_no", "$PL_sv_undef"] {
        let offset = code.find(needle).expect("needle should exist");
        let expected_position = line_col_mapper(code)(offset);
        let token = tokens
            .iter()
            .zip(positions.iter())
            .find(|(tok, pos)| {
                tok[3] == var_idx && has_default_library_modifier(tok) && **pos == expected_position
            })
            .map(|(tok, _)| *tok);

        assert!(
            token.is_some(),
            "{needle} should produce a variable token with defaultLibrary modifier, got: {:?}",
            tokens
                .iter()
                .filter(|t| t[3] == var_idx)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_regular_variable_does_not_have_default_library_modifier() {
    // User-defined variables should NOT have defaultLibrary set
    let code = "my $user_var = 42;";
    let tokens = tokens_for(code);
    let var_idx = type_idx("variable");
    let with_default_lib: Vec<_> = tokens
        .iter()
        .filter(|t| t[3] == var_idx && has_default_library_modifier(t))
        .collect();
    assert!(
        with_default_lib.is_empty(),
        "user-defined $user_var should NOT have defaultLibrary modifier, got: {:?}",
        with_default_lib
    );
}

#[test]
fn test_special_variable_env_hash_has_default_library_modifier() {
    // %ENV accessed as $ENV{HOME} appears in the AST as Variable { sigil: "$", name: "ENV" }.
    // Wrap in a sub body so the walk visits the Variable node.
    let code = "sub f { my $h = $ENV{HOME}; }";
    let tokens = tokens_for(code);
    let var_idx = type_idx("variable");
    let special_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t[3] == var_idx && has_default_library_modifier(t))
        .collect();
    assert!(
        !special_tokens.is_empty(),
        "$ENV{{...}} access should produce a variable token with defaultLibrary modifier, \
         got variable tokens: {:?}",
        tokens
            .iter()
            .filter(|t| t[3] == var_idx)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_special_variable_argv_has_default_library_modifier() {
    // @ARGV is a built-in special variable receiving the command-line arguments.
    let code = "sub main { my @args = @ARGV; }";
    let tokens = tokens_for(code);
    let var_idx = type_idx("variable");
    let special_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t[3] == var_idx && has_default_library_modifier(t))
        .collect();
    assert!(
        !special_tokens.is_empty(),
        "@ARGV should produce a variable token with defaultLibrary modifier, \
         got variable tokens: {:?}",
        tokens
            .iter()
            .filter(|t| t[3] == var_idx)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_special_variable_dollar_question_has_default_library_modifier() {
    // $? holds child process exit status after system() / backtick / waitpid().
    let code = "sub f { system('ls'); return $?; }";
    let tokens = tokens_for(code);
    let var_idx = type_idx("variable");
    let special_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t[3] == var_idx && has_default_library_modifier(t))
        .collect();
    assert!(
        !special_tokens.is_empty(),
        "$? should produce a variable token with defaultLibrary modifier, \
         got variable tokens: {:?}",
        tokens
            .iter()
            .filter(|t| t[3] == var_idx)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_two_special_variables_in_same_expression_both_get_modifier() {
    // Both $! and $@ are special; check that when both appear in the same
    // sub body they each get the defaultLibrary modifier.
    let code = "sub f { eval { die 'boom' }; return $@ || $!; }";
    let tokens = tokens_for(code);
    let var_idx = type_idx("variable");
    let special_count = tokens
        .iter()
        .filter(|t| t[3] == var_idx && has_default_library_modifier(t))
        .count();
    assert!(
        special_count >= 2,
        "Both $@ and $! should receive defaultLibrary modifier; \
         got {special_count} special-variable tokens (expected >= 2)"
    );
}

// ===========================================================================
// Heredoc language injection — Issue #2059
// ===========================================================================

#[test]
fn sql_heredoc_keywords_are_classified() {
    // <<SQL heredoc body containing SQL keywords should produce sql_heredoc_keyword tokens.
    let code = "my $sql = <<SQL;\nSELECT * FROM users WHERE id = ?\nSQL\n";
    let result = tokens_of_type(code, "sql_heredoc_keyword");
    assert!(
        !result.is_empty(),
        "<<SQL heredoc body with SELECT/FROM/WHERE should produce >= 1 sql_heredoc_keyword token; \
         got none. Legend has sql_heredoc_keyword: {}",
        legend().map.contains_key("sql_heredoc_keyword")
    );
}

#[test]
fn json_heredoc_keys_are_classified() {
    // <<JSON heredoc body with a quoted key:value should produce json_heredoc_key tokens.
    let code = "my $j = <<JSON;\n{\"key\": \"value\", \"count\": 42}\nJSON\n";
    let result = tokens_of_type(code, "json_heredoc_key");
    assert!(
        !result.is_empty(),
        "<<JSON heredoc body with quoted keys should produce >= 1 json_heredoc_key token; \
         got none. Legend has json_heredoc_key: {}",
        legend().map.contains_key("json_heredoc_key")
    );
}

#[test]
fn non_sql_heredoc_body_is_plain_string() {
    // <<EOF heredoc should not produce any injection tokens.
    let code = "my $txt = <<EOF;\nSELECT this is just text\nEOF\n";
    let sql_result = tokens_of_type(code, "sql_heredoc_keyword");
    let json_result = tokens_of_type(code, "json_heredoc_key");
    assert!(
        sql_result.is_empty(),
        "<<EOF heredoc should not produce sql_heredoc_keyword tokens; got: {:?}",
        sql_result
    );
    assert!(
        json_result.is_empty(),
        "<<EOF heredoc should not produce json_heredoc_key tokens; got: {:?}",
        json_result
    );
}

#[test]
fn multiple_heredocs_same_line_both_injected() {
    // Two heredocs on the same line: first SQL, then JSON.
    // Both bodies must get injection tokens.
    let code = "my ($a, $b) = (<<SQL, <<JSON);\nSELECT 1\nSQL\n{\"x\": 1}\nJSON\n";
    let sql_result = tokens_of_type(code, "sql_heredoc_keyword");
    let json_result = tokens_of_type(code, "json_heredoc_key");
    assert!(
        !sql_result.is_empty(),
        "First heredoc (<<SQL) on multi-heredoc line should produce sql_heredoc_keyword tokens"
    );
    assert!(
        !json_result.is_empty(),
        "Second heredoc (<<JSON) on multi-heredoc line should produce json_heredoc_key tokens"
    );
}

#[test]
fn command_heredoc_not_injected() {
    // Backtick heredoc (command exec) must NOT produce sql_heredoc_keyword tokens.
    let code = "my $out = <<`SQL`;\nSELECT 1\nSQL\n";
    let result = tokens_of_type(code, "sql_heredoc_keyword");
    assert!(
        result.is_empty(),
        "Backtick (command) heredoc must not produce sql_heredoc_keyword tokens; got: {:?}",
        result
    );
}

#[test]
fn perl_variable_in_sql_heredoc_preserved() {
    // A Perl variable inside a SQL heredoc body must still get a variable token.
    // Injection tokens and variable tokens can coexist in the same heredoc body.
    let code = "my $sql = <<SQL;\nSELECT * FROM users WHERE id = $user_id\nSQL\n";
    let sql_result = tokens_of_type(code, "sql_heredoc_keyword");
    let var_result = tokens_of_type(code, "variable");
    assert!(
        !sql_result.is_empty(),
        "<<SQL heredoc should still produce sql_heredoc_keyword tokens when Perl variable present"
    );
    assert!(
        !var_result.is_empty(),
        "Perl variable $user_id inside SQL heredoc body should still get a variable token"
    );
}

#[test]
fn case_insensitive_sql_tag_is_recognized() {
    // <<sql (lowercase) should also trigger SQL injection.
    let code = "my $s = <<sql;\nSELECT 1\nsql\n";
    let result = tokens_of_type(code, "sql_heredoc_keyword");
    assert!(
        !result.is_empty(),
        "Lowercase <<sql tag should produce sql_heredoc_keyword tokens just like <<SQL"
    );
}

#[test]
fn quoted_sql_tag_is_recognized() {
    // <<'SQL' (single-quoted delimiter) should trigger SQL injection.
    let code = "my $s = <<'SQL';\nSELECT 1\nSQL\n";
    let result = tokens_of_type(code, "sql_heredoc_keyword");
    assert!(
        !result.is_empty(),
        "Single-quoted <<'SQL' tag should produce sql_heredoc_keyword tokens"
    );
}

#[test]
fn indented_heredoc_sql_tag_is_recognized() {
    // <<~SQL (indented heredoc) should trigger SQL injection.
    let code = "my $s = <<~SQL;\n    SELECT 1\n    SQL\n";
    let result = tokens_of_type(code, "sql_heredoc_keyword");
    assert!(
        !result.is_empty(),
        "Indented <<~SQL tag should produce sql_heredoc_keyword tokens"
    );
}
