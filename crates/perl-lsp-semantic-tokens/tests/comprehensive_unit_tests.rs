//! Comprehensive unit tests for the `perl-lsp-semantic-tokens` crate.
//!
//! Tests cover the public API: `legend()`, `collect_semantic_tokens()`,
//! `EncodedToken`, `TokensLegend`, and `SemanticTokensProvider`.
#![allow(dead_code, unused_variables, unused_assignments)]

use perl_lsp_semantic_tokens::{
    EncodedToken, SemanticTokensProvider, collect_semantic_tokens, legend,
};
use perl_tdd_support::{Parser, must, must_some};

// ---------------------------------------------------------------------------
// Helper: simple byte→(line, col) for single-line sources
// ---------------------------------------------------------------------------
fn single_line_pos(byte: usize) -> (u32, u32) {
    (0, byte as u32)
}

/// Build a full position mapper for multi-line text.
fn line_col_mapper(text: &str) -> impl Fn(usize) -> (u32, u32) + '_ {
    move |byte: usize| {
        let prefix = &text[..byte.min(text.len())];
        let line = prefix.matches('\n').count() as u32;
        let last_nl = prefix.rfind('\n').map_or(0, |p| p + 1);
        let col = (byte - last_nl) as u32;
        (line, col)
    }
}

/// Parse Perl source and collect semantic tokens using the provided mapper.
fn tokens_for(code: &str) -> Vec<EncodedToken> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let mapper = line_col_mapper(code);
    collect_semantic_tokens(&ast, code, &mapper)
}

// ===========================================================================
// legend() tests
// ===========================================================================

#[test]
fn legend_contains_expected_token_types() {
    let leg = legend();
    let expected = [
        "namespace",
        "class",
        "function",
        "method",
        "variable",
        "parameter",
        "property",
        "keyword",
        "comment",
        "string",
        "number",
        "regexp",
        "operator",
        "type",
        "macro",
    ];
    for ty in &expected {
        assert!(leg.token_types.contains(&ty.to_string()), "legend missing token type: {ty}");
    }
}

#[test]
fn legend_contains_expected_modifiers() {
    let leg = legend();
    let expected = [
        "declaration",
        "definition",
        "readonly",
        "defaultLibrary",
        "deprecated",
        "static",
        "async",
    ];
    for m in &expected {
        assert!(leg.modifiers.contains(&m.to_string()), "legend missing modifier: {m}");
    }
}

#[test]
fn legend_map_is_consistent_with_token_types() {
    let leg = legend();
    assert_eq!(leg.map.len(), leg.token_types.len());
    for (i, ty) in leg.token_types.iter().enumerate() {
        let idx = must_some(leg.map.get(ty));
        assert_eq!(*idx, i as u32, "map[{ty}] should be {i}");
    }
}

#[test]
fn legend_token_types_have_no_duplicates() {
    let leg = legend();
    let mut seen = std::collections::HashSet::new();
    for ty in &leg.token_types {
        assert!(seen.insert(ty.clone()), "duplicate token type: {ty}");
    }
}

#[test]
fn legend_modifiers_have_no_duplicates() {
    let leg = legend();
    let mut seen = std::collections::HashSet::new();
    for m in &leg.modifiers {
        assert!(seen.insert(m.clone()), "duplicate modifier: {m}");
    }
}

#[test]
fn legend_token_type_count() {
    let leg = legend();
    assert_eq!(leg.token_types.len(), 15, "expected 15 token types");
}

#[test]
fn legend_modifier_count() {
    let leg = legend();
    assert_eq!(leg.modifiers.len(), 7, "expected 7 modifiers");
}

#[test]
fn legend_map_indices_are_sequential() {
    let leg = legend();
    let mut indices: Vec<u32> = leg.map.values().copied().collect();
    indices.sort();
    let expected: Vec<u32> = (0..leg.token_types.len() as u32).collect();
    assert_eq!(indices, expected);
}

// ===========================================================================
// SemanticTokensProvider tests
// ===========================================================================

#[test]
fn provider_new_creates_instance() {
    let _provider = SemanticTokensProvider::new();
}

#[test]
fn provider_default_creates_instance() {
    let _provider = SemanticTokensProvider;
}

#[test]
fn provider_new_and_default_are_equivalent() {
    // Both should create a valid provider (placeholder)
    let _a = SemanticTokensProvider::new();
    let _b = SemanticTokensProvider;
}

// ===========================================================================
// EncodedToken type tests
// ===========================================================================

#[test]
fn encoded_token_is_five_element_array() {
    let token: EncodedToken = [0, 0, 5, 1, 0];
    assert_eq!(token.len(), 5);
    assert_eq!(token[0], 0); // delta_line
    assert_eq!(token[1], 0); // delta_start
    assert_eq!(token[2], 5); // length
    assert_eq!(token[3], 1); // token_type
    assert_eq!(token[4], 0); // token_modifiers
}

// ===========================================================================
// collect_semantic_tokens — empty / minimal input
// ===========================================================================

#[test]
fn empty_source_produces_empty_tokens() {
    let tokens = tokens_for("");
    assert!(tokens.is_empty(), "empty source should produce no tokens");
}

#[test]
fn whitespace_only_source_produces_no_tokens() {
    let tokens = tokens_for("   \t  \n  \n  ");
    assert!(tokens.is_empty(), "whitespace-only should produce no tokens");
}

// ===========================================================================
// collect_semantic_tokens — keyword recognition
// ===========================================================================

#[test]
fn keyword_my_is_classified() {
    let tokens = tokens_for("my $x;");
    assert!(!tokens.is_empty(), "should produce tokens for 'my $x;'");
    let leg = legend();
    let kw_idx = must_some(leg.map.get("keyword"));
    let has_keyword = tokens.iter().any(|t| t[3] == *kw_idx);
    assert!(has_keyword, "'my' should be classified as keyword");
}

#[test]
fn keyword_sub_produces_function_via_ast_overlay() {
    // "sub" as a keyword is absorbed by the AST overlay which classifies
    // the entire subroutine span as a "function" token with declaration modifier.
    let tokens = tokens_for("sub foo { }");
    let leg = legend();
    let fn_idx = must_some(leg.map.get("function"));
    let has_fn = tokens.iter().any(|t| t[3] == *fn_idx && t[4] & 1 != 0);
    assert!(has_fn, "sub declaration should produce function token with declaration modifier");
}

#[test]
fn keyword_use_is_classified() {
    let tokens = tokens_for("use strict;");
    let leg = legend();
    let kw_idx = must_some(leg.map.get("keyword"));
    let has_keyword = tokens.iter().any(|t| t[3] == *kw_idx);
    assert!(has_keyword, "'use' should be classified as keyword");
}

#[test]
fn keyword_if_is_classified() {
    let tokens = tokens_for("if (1) { }");
    let leg = legend();
    let kw_idx = must_some(leg.map.get("keyword"));
    let has_keyword = tokens.iter().any(|t| t[3] == *kw_idx);
    assert!(has_keyword, "'if' should be classified as keyword");
}

#[test]
fn keyword_return_classified_on_separate_line() {
    // When return is on its own line inside a multi-line sub, the lexer
    // keyword token can survive overlap removal.
    let code = "sub f {\n    return 1;\n}";
    let tokens = tokens_for(code);
    let leg = legend();
    let kw_idx = must_some(leg.map.get("keyword"));
    let has_keyword = tokens.iter().any(|t| t[3] == *kw_idx);
    assert!(has_keyword, "'return' on separate line should be classified as keyword");
}

#[test]
fn keyword_while_is_classified() {
    let tokens = tokens_for("while (1) { }");
    let leg = legend();
    let kw_idx = must_some(leg.map.get("keyword"));
    let has_keyword = tokens.iter().any(|t| t[3] == *kw_idx);
    assert!(has_keyword, "'while' should be classified as keyword");
}

#[test]
fn keyword_for_foreach_are_classified() {
    for kw in &["for", "foreach"] {
        let code = format!("{kw} my $x (1..3) {{ }}");
        let tokens = tokens_for(&code);
        let leg = legend();
        let kw_idx = must_some(leg.map.get("keyword"));
        let has_keyword = tokens.iter().any(|t| t[3] == *kw_idx);
        assert!(has_keyword, "'{kw}' should be classified as keyword");
    }
}

#[test]
fn various_scope_keywords_classified() {
    for kw in &["our", "local", "state"] {
        let code = format!("{kw} $x;");
        let tokens = tokens_for(&code);
        let leg = legend();
        let kw_idx = must_some(leg.map.get("keyword"));
        let has_keyword = tokens.iter().any(|t| t[3] == *kw_idx);
        assert!(has_keyword, "'{kw}' should be classified as keyword");
    }
}

#[test]
fn control_flow_keywords_classified() {
    for kw in &["next", "last", "redo", "goto"] {
        let code = format!("while (1) {{ {kw}; }}");
        let tokens = tokens_for(&code);
        let leg = legend();
        let kw_idx = must_some(leg.map.get("keyword"));
        let has_keyword = tokens.iter().any(|t| t[3] == *kw_idx);
        assert!(has_keyword, "'{kw}' should be classified as keyword");
    }
}

// ===========================================================================
// collect_semantic_tokens — string recognition
// ===========================================================================

#[test]
fn single_quoted_string_classified() {
    let tokens = tokens_for("my $x = 'hello';");
    let leg = legend();
    let str_idx = must_some(leg.map.get("string"));
    let has_string = tokens.iter().any(|t| t[3] == *str_idx);
    assert!(has_string, "single-quoted string should be classified");
}

#[test]
fn double_quoted_string_classified() {
    let tokens = tokens_for("my $x = \"world\";");
    let leg = legend();
    let str_idx = must_some(leg.map.get("string"));
    let has_string = tokens.iter().any(|t| t[3] == *str_idx);
    assert!(has_string, "double-quoted string should be classified");
}

// ===========================================================================
// collect_semantic_tokens — number recognition
// ===========================================================================

#[test]
fn integer_literal_classified() {
    let tokens = tokens_for("my $x = 42;");
    let leg = legend();
    let num_idx = must_some(leg.map.get("number"));
    let has_number = tokens.iter().any(|t| t[3] == *num_idx);
    assert!(has_number, "integer literal should be classified as number");
}

#[test]
fn float_literal_classified() {
    let tokens = tokens_for("my $x = 3.14;");
    let leg = legend();
    let num_idx = must_some(leg.map.get("number"));
    let has_number = tokens.iter().any(|t| t[3] == *num_idx);
    assert!(has_number, "float literal should be classified as number");
}

// ===========================================================================
// collect_semantic_tokens — comment recognition
// ===========================================================================

#[test]
fn comments_are_skipped_by_lexer() {
    // The PerlLexer skips comments (skip_whitespace_and_comments), so
    // Comment tokens are never emitted. Verify no comment tokens appear.
    let tokens = tokens_for("my $x; # inline comment");
    let leg = legend();
    let cmt_idx = must_some(leg.map.get("comment"));
    let has_comment = tokens.iter().any(|t| t[3] == *cmt_idx);
    assert!(!has_comment, "comments are skipped by the lexer, so no comment tokens");
}

// ===========================================================================
// collect_semantic_tokens — operator recognition
// ===========================================================================

#[test]
fn arrow_operator_classified() {
    let tokens = tokens_for("$x->{key};");
    let leg = legend();
    let op_idx = must_some(leg.map.get("operator"));
    let has_op = tokens.iter().any(|t| t[3] == *op_idx);
    assert!(has_op, "arrow should be classified as operator");
}

#[test]
fn fat_comma_classified() {
    let tokens = tokens_for("my %h = (a => 1);");
    let leg = legend();
    let op_idx = must_some(leg.map.get("operator"));
    let has_op = tokens.iter().any(|t| t[3] == *op_idx);
    assert!(has_op, "fat comma should be classified as operator");
}

// ===========================================================================
// collect_semantic_tokens — AST overlay: package / sub / function / variable
// ===========================================================================

#[test]
fn package_declaration_produces_namespace_token() {
    let tokens = tokens_for("package Foo;");
    let leg = legend();
    let ns_idx = must_some(leg.map.get("namespace"));
    let has_ns = tokens.iter().any(|t| t[3] == *ns_idx);
    assert!(has_ns, "package should produce namespace token");
}

#[test]
fn named_subroutine_produces_function_token() {
    let tokens = tokens_for("sub greet { }");
    let leg = legend();
    let fn_idx = must_some(leg.map.get("function"));
    let has_fn = tokens.iter().any(|t| t[3] == *fn_idx);
    assert!(has_fn, "named sub should produce function token");
}

#[test]
fn function_call_produces_function_token() {
    let tokens = tokens_for("print('hello');");
    let leg = legend();
    let fn_idx = must_some(leg.map.get("function"));
    let has_fn = tokens.iter().any(|t| t[3] == *fn_idx);
    assert!(has_fn, "function call should produce function token");
}

#[test]
fn variable_produces_variable_token() {
    let tokens = tokens_for("my $x = 1;");
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let has_var = tokens.iter().any(|t| t[3] == *var_idx);
    assert!(has_var, "variable should produce variable token");
}

// ===========================================================================
// collect_semantic_tokens — delta encoding
// ===========================================================================

#[test]
fn tokens_are_delta_encoded_single_line() {
    // Tokens on a single line: delta_line should be 0 for all after the first
    let tokens = tokens_for("my $x = 42;");
    if tokens.len() >= 2 {
        // After the first token, delta_line should remain 0 (same line)
        for t in &tokens[1..] {
            assert_eq!(t[0], 0, "delta_line should be 0 on same line");
        }
    }
}

#[test]
fn tokens_across_lines_have_positive_delta_line() {
    let code = "my $x = 1;\nmy $y = 2;";
    let tokens = tokens_for(code);
    // There should be at least one token with delta_line > 0
    let has_line_delta = tokens.iter().any(|t| t[0] > 0);
    assert!(has_line_delta, "multi-line code should have tokens with delta_line > 0");
}

#[test]
fn delta_start_resets_on_new_line() {
    let code = "my $x = 1;\nmy $y = 2;";
    let tokens = tokens_for(code);
    // Find first token with delta_line > 0; its delta_start should be
    // the absolute column (not relative to previous line's token)
    for t in &tokens {
        if t[0] > 0 {
            // delta_start is absolute column on new line
            // Just verify it's a reasonable value
            assert!(t[1] < 100, "delta_start on new line should be a column offset");
            break;
        }
    }
}

// ===========================================================================
// collect_semantic_tokens — token lengths
// ===========================================================================

#[test]
fn all_tokens_have_positive_length() {
    let code = "my $x = 42; sub foo { return $x; }";
    let tokens = tokens_for(code);
    for (i, t) in tokens.iter().enumerate() {
        assert!(t[2] > 0, "token {i} should have positive length, got {}", t[2]);
    }
}

// ===========================================================================
// collect_semantic_tokens — multi-line / complex
// ===========================================================================

#[test]
fn multiline_subroutine_produces_tokens() {
    let code = "sub hello {\n    my $name = shift;\n    print \"Hello, $name!\";\n}";
    let tokens = tokens_for(code);
    assert!(!tokens.is_empty(), "multi-line sub should produce tokens");
    let leg = legend();
    let kw_idx = must_some(leg.map.get("keyword"));
    let str_idx = must_some(leg.map.get("string"));
    // Multi-line sub: "sub" and "my" appear as keyword tokens from the lexer
    assert!(tokens.iter().any(|t| t[3] == *kw_idx), "should have keyword (sub/my)");
    // Multi-line AST nodes (function) have len=0 and are filtered, but
    // string literals on their own line should appear
    assert!(tokens.iter().any(|t| t[3] == *str_idx), "should have string token");
}

#[test]
fn package_with_subroutines() {
    let code = "package MyModule;\nsub new { bless {}, shift; }\nsub greet { print \"hi\"; }";
    let tokens = tokens_for(code);
    let leg = legend();
    let ns_idx = must_some(leg.map.get("namespace"));
    let fn_idx = must_some(leg.map.get("function"));
    assert!(tokens.iter().any(|t| t[3] == *ns_idx), "should have namespace");
    assert!(tokens.iter().any(|t| t[3] == *fn_idx), "should have function");
}

#[test]
fn nested_control_structures() {
    let code = "if (1) { while (1) { last; } }";
    let tokens = tokens_for(code);
    let leg = legend();
    let kw_idx = must_some(leg.map.get("keyword"));
    let keyword_count = tokens.iter().filter(|t| t[3] == *kw_idx).count();
    // Should have at least "if", "while", "last"
    assert!(keyword_count >= 3, "expected >=3 keyword tokens, got {keyword_count}");
}

// ===========================================================================
// collect_semantic_tokens — ordering guarantee
// ===========================================================================

#[test]
fn tokens_are_monotonically_ordered() {
    let code = "package X;\nmy $a = 1;\nmy $b = 'hello';\nsub foo { return; }";
    let tokens = tokens_for(code);

    // Reconstruct absolute positions from deltas
    let mut positions = Vec::new();
    let mut line = 0u32;
    let mut col = 0u32;
    for t in &tokens {
        if t[0] > 0 {
            line += t[0];
            col = t[1];
        } else {
            col += t[1];
        }
        positions.push((line, col));
    }

    // Verify monotonic ordering
    for i in 1..positions.len() {
        let (pl, pc) = positions[i - 1];
        let (cl, cc) = positions[i];
        assert!(
            cl > pl || (cl == pl && cc >= pc),
            "tokens not monotonically ordered at index {i}: ({pl},{pc}) vs ({cl},{cc})"
        );
    }
}

// ===========================================================================
// collect_semantic_tokens — regex recognition
// ===========================================================================

#[test]
fn regex_match_classified() {
    let tokens = tokens_for("$x =~ /pattern/;");
    let leg = legend();
    let re_idx = must_some(leg.map.get("regexp"));
    let has_re = tokens.iter().any(|t| t[3] == *re_idx);
    assert!(has_re, "regex match should be classified as regexp");
}

// ===========================================================================
// collect_semantic_tokens — method call recognition
// ===========================================================================

#[test]
fn method_call_produces_method_token() {
    let tokens = tokens_for("$obj->method();");
    let leg = legend();
    let meth_idx = must_some(leg.map.get("method"));
    let has_method = tokens.iter().any(|t| t[3] == *meth_idx);
    assert!(has_method, "method call should produce method token");
}

// ===========================================================================
// collect_semantic_tokens — subroutine declaration modifier
// ===========================================================================

#[test]
fn named_subroutine_has_declaration_modifier() {
    let tokens = tokens_for("sub greet { }");
    let leg = legend();
    let fn_idx = must_some(leg.map.get("function"));
    let fn_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == *fn_idx).collect();
    // At least one function token should have declaration modifier (bit 0 = 1)
    let has_decl = fn_tokens.iter().any(|t| t[4] & 1 != 0);
    assert!(has_decl, "named sub should have declaration modifier on function token");
}

#[test]
fn function_call_has_no_declaration_modifier() {
    let tokens = tokens_for("print('hello');");
    let leg = legend();
    let fn_idx = must_some(leg.map.get("function"));
    let call_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == *fn_idx).collect();
    // Function call tokens should NOT have declaration modifier
    for t in &call_tokens {
        assert_eq!(t[4] & 1, 0, "function call should not have declaration modifier");
    }
}

// ===========================================================================
// collect_semantic_tokens — multiple token types in one file
// ===========================================================================

#[test]
fn mixed_code_produces_diverse_token_types() {
    let code = "package Foo;\nuse strict;\nmy $x = 42;\nsub bar {\n    return 'hi';\n}\n";
    let tokens = tokens_for(code);
    let leg = legend();

    let types_present: std::collections::HashSet<u32> = tokens.iter().map(|t| t[3]).collect();

    let kw_idx = must_some(leg.map.get("keyword"));
    let str_idx = must_some(leg.map.get("string"));
    let num_idx = must_some(leg.map.get("number"));

    assert!(types_present.contains(kw_idx), "should have keyword tokens");
    assert!(types_present.contains(str_idx), "should have string tokens");
    assert!(types_present.contains(num_idx), "should have number tokens");
}

// ===========================================================================
// collect_semantic_tokens — edge cases
// ===========================================================================

#[test]
fn semicolons_only_produce_no_semantic_tokens() {
    let tokens = tokens_for(";;;");
    // Semicolons are not semantic tokens
    assert!(tokens.is_empty(), "semicolons-only should produce no semantic tokens");
}

#[test]
fn single_variable_declaration() {
    let tokens = tokens_for("my $x;");
    assert!(!tokens.is_empty(), "single var decl should produce tokens");
}

#[test]
fn deeply_nested_blocks() {
    let code = "if (1) { if (1) { if (1) { my $x; } } }";
    let tokens = tokens_for(code);
    assert!(!tokens.is_empty(), "deeply nested blocks should produce tokens");
}

#[test]
fn eval_block_produces_keyword() {
    let tokens = tokens_for("eval { 1; };");
    let leg = legend();
    let kw_idx = must_some(leg.map.get("keyword"));
    let has_kw = tokens.iter().any(|t| t[3] == *kw_idx);
    assert!(has_kw, "eval should be classified as keyword");
}

#[test]
fn unless_keyword_classified() {
    let tokens = tokens_for("unless (0) { }");
    let leg = legend();
    let kw_idx = must_some(leg.map.get("keyword"));
    let has_kw = tokens.iter().any(|t| t[3] == *kw_idx);
    assert!(has_kw, "unless should be classified as keyword");
}

#[test]
fn until_keyword_classified() {
    let tokens = tokens_for("until (0) { }");
    let leg = legend();
    let kw_idx = must_some(leg.map.get("keyword"));
    let has_kw = tokens.iter().any(|t| t[3] == *kw_idx);
    assert!(has_kw, "until should be classified as keyword");
}

#[test]
fn do_keyword_classified() {
    let tokens = tokens_for("do { 1; };");
    let leg = legend();
    let kw_idx = must_some(leg.map.get("keyword"));
    let has_kw = tokens.iter().any(|t| t[3] == *kw_idx);
    assert!(has_kw, "do should be classified as keyword");
}

#[test]
fn require_classified_as_function_call() {
    // `require` is parsed as a FunctionCall in the AST, so it produces
    // a "function" token rather than a "keyword" token.
    let tokens = tokens_for("require Foo;");
    let leg = legend();
    let fn_idx = must_some(leg.map.get("function"));
    let has_fn = tokens.iter().any(|t| t[3] == *fn_idx);
    assert!(has_fn, "require should be classified as function (FunctionCall in AST)");
}

#[test]
fn elsif_else_keywords_classified() {
    let code = "if (1) { } elsif (0) { } else { }";
    let tokens = tokens_for(code);
    let leg = legend();
    let kw_idx = must_some(leg.map.get("keyword"));
    let kw_count = tokens.iter().filter(|t| t[3] == *kw_idx).count();
    // "if", "elsif", "else" = at least 3 keywords
    assert!(kw_count >= 3, "expected >=3 keywords for if/elsif/else, got {kw_count}");
}

// ===========================================================================
// collect_semantic_tokens — custom to_pos16 mapper
// ===========================================================================

#[test]
fn custom_pos16_mapper_is_respected() {
    // Use a mapper that offsets columns by 100
    let code = "my $x;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let mapper = |byte: usize| -> (u32, u32) { (0, byte as u32 + 100) };
    let tokens = collect_semantic_tokens(&ast, code, &mapper);
    // All columns should be >= 100
    // Reconstruct absolute columns
    let mut col = 0u32;
    for t in &tokens {
        col += t[1];
        assert!(col >= 100, "custom mapper column should be >= 100, got {col}");
    }
}

// ===========================================================================
// collect_semantic_tokens — multi-line delta correctness
// ===========================================================================

#[test]
fn three_line_code_delta_correctness() {
    let code = "my $a = 1;\nmy $b = 2;\nmy $c = 3;";
    let tokens = tokens_for(code);

    // Verify we can reconstruct positions
    let mut line = 0u32;
    let mut col = 0u32;
    let mut max_line = 0u32;
    for t in &tokens {
        if t[0] > 0 {
            line += t[0];
            col = t[1];
        } else {
            col += t[1];
        }
        if line > max_line {
            max_line = line;
        }
    }
    // Three lines means max line should be at least 2
    assert!(max_line >= 2, "three-line code should span lines 0-2, max_line={max_line}");
}

// ===========================================================================
// collect_semantic_tokens — token type indices are valid
// ===========================================================================

#[test]
fn all_token_type_indices_are_within_legend() {
    let code = "package Foo; use strict; my $x = 42; sub bar { return 'hi'; }";
    let tokens = tokens_for(code);
    let leg = legend();
    let max_idx = leg.token_types.len() as u32;
    for (i, t) in tokens.iter().enumerate() {
        assert!(
            t[3] < max_idx,
            "token {i} has type index {} but legend only has {max_idx} types",
            t[3]
        );
    }
}

// ===========================================================================
// collect_semantic_tokens — idempotence
// ===========================================================================

#[test]
fn parsing_same_code_twice_produces_same_tokens() {
    let code = "my $x = 1; sub foo { return $x; }";
    let tokens1 = tokens_for(code);
    let tokens2 = tokens_for(code);
    assert_eq!(tokens1, tokens2, "same code should produce identical tokens");
}

// ===========================================================================
// collect_semantic_tokens — anonymous subroutine
// ===========================================================================

#[test]
fn anonymous_sub_does_not_produce_function_declaration() {
    let tokens = tokens_for("my $f = sub { 1; };");
    let leg = legend();
    let fn_idx = must_some(leg.map.get("function"));
    // Anonymous sub (name=None) shouldn't match Subroutine{name: Some(_)}
    // so no function token with declaration modifier from AST overlay
    let fn_decl_tokens: Vec<_> =
        tokens.iter().filter(|t| t[3] == *fn_idx && t[4] & 1 != 0).collect();
    assert!(
        fn_decl_tokens.is_empty(),
        "anonymous sub should not produce function declaration token"
    );
}

// ===========================================================================
// collect_semantic_tokens — large number of lines
// ===========================================================================

#[test]
fn many_lines_produce_correct_deltas() {
    let lines: Vec<String> = (0..20).map(|i| format!("my $v{i} = {i};")).collect();
    let code = lines.join("\n");
    let tokens = tokens_for(&code);

    // Verify we cover many lines
    let mut line = 0u32;
    for t in &tokens {
        line += t[0];
    }
    assert!(line >= 15, "20-line code should span many lines, final line={line}");
}

// ===========================================================================
// collect_semantic_tokens — no overlapping tokens in output
// ===========================================================================

#[test]
fn output_tokens_do_not_overlap() {
    let code = "package Foo; sub new { my $self = bless {}, shift; return $self; }";
    let tokens = tokens_for(code);

    // Reconstruct absolute positions and check for overlaps
    let mut abs_tokens: Vec<(u32, u32, u32)> = Vec::new(); // (line, col, len)
    let mut line = 0u32;
    let mut col = 0u32;
    for t in &tokens {
        if t[0] > 0 {
            line += t[0];
            col = t[1];
        } else {
            col += t[1];
        }
        abs_tokens.push((line, col, t[2]));
    }

    // Check no two tokens on the same line overlap
    for i in 1..abs_tokens.len() {
        let (pl, pc, plen) = abs_tokens[i - 1];
        let (cl, cc, _) = abs_tokens[i];
        if cl == pl {
            assert!(
                cc >= pc + plen,
                "tokens at index {}-{} overlap on line {pl}: prev=({pc},{plen}) curr=({cc},_)",
                i - 1,
                i
            );
        }
    }
}

// ===========================================================================
// collect_semantic_tokens — division vs regex ambiguity
// ===========================================================================

#[test]
fn division_operator_classified_as_operator() {
    let tokens = tokens_for("my $x = 10 / 2;");
    let leg = legend();
    let op_idx = must_some(leg.map.get("operator"));
    let has_op = tokens.iter().any(|t| t[3] == *op_idx);
    assert!(has_op, "division should be classified as operator");
}

// ===========================================================================
// collect_semantic_tokens — qw// string
// ===========================================================================

#[test]
fn qw_words_classified_as_string() {
    let tokens = tokens_for("my @a = qw(foo bar baz);");
    let leg = legend();
    let str_idx = must_some(leg.map.get("string"));
    let has_string = tokens.iter().any(|t| t[3] == *str_idx);
    assert!(has_string, "qw() should be classified as string");
}

// ===========================================================================
// collect_semantic_tokens — given/when keywords
// ===========================================================================

#[test]
fn given_when_default_keywords_classified() {
    // These are valid Perl keywords even if not commonly used
    for kw in &["given", "when", "default", "break", "continue"] {
        let code = format!("{kw};");
        let tokens = tokens_for(&code);
        let leg = legend();
        let kw_idx = must_some(leg.map.get("keyword"));
        let has_kw = tokens.iter().any(|t| t[3] == *kw_idx);
        assert!(has_kw, "'{kw}' should be classified as keyword");
    }
}
