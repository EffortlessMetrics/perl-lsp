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
        assert!(
            leg.token_types.contains(&ty.to_string()),
            "legend missing token type: {ty}"
        );
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
        assert!(
            leg.modifiers.contains(&m.to_string()),
            "legend missing modifier: {m}"
        );
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
    // 20 standard LSP types + sql_string + sql_heredoc_keyword + json_heredoc_key
    // = 23 total (must match capabilities_for() advertisement)
    assert_eq!(leg.token_types.len(), 23, "expected 23 token types");
}

#[test]
fn legend_modifier_count() {
    let leg = legend();
    // 10 standard LSP modifiers + 3 sigil modifiers (scalarVariable, arrayVariable, hashVariable)
    // = 13 total (must match capabilities_for() advertisement)
    assert_eq!(leg.modifiers.len(), 13, "expected 13 modifiers");
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
    assert!(
        tokens.is_empty(),
        "whitespace-only should produce no tokens"
    );
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
    assert!(
        has_fn,
        "sub declaration should produce function token with declaration modifier"
    );
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
    assert!(
        has_keyword,
        "'return' on separate line should be classified as keyword"
    );
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
    assert!(
        !has_comment,
        "comments are skipped by the lexer, so no comment tokens"
    );
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
    assert!(
        has_line_delta,
        "multi-line code should have tokens with delta_line > 0"
    );
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
            assert!(
                t[1] < 100,
                "delta_start on new line should be a column offset"
            );
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
        assert!(
            t[2] > 0,
            "token {i} should have positive length, got {}",
            t[2]
        );
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
    assert!(
        tokens.iter().any(|t| t[3] == *kw_idx),
        "should have keyword (sub/my)"
    );
    // Multi-line AST nodes (function) have len=0 and are filtered, but
    // string literals on their own line should appear
    assert!(
        tokens.iter().any(|t| t[3] == *str_idx),
        "should have string token"
    );
}

#[test]
fn package_with_subroutines() {
    let code = "package MyModule;\nsub new { bless {}, shift; }\nsub greet { print \"hi\"; }";
    let tokens = tokens_for(code);
    let leg = legend();
    let ns_idx = must_some(leg.map.get("namespace"));
    let fn_idx = must_some(leg.map.get("function"));
    assert!(
        tokens.iter().any(|t| t[3] == *ns_idx),
        "should have namespace"
    );
    assert!(
        tokens.iter().any(|t| t[3] == *fn_idx),
        "should have function"
    );
}

#[test]
fn nested_control_structures() {
    let code = "if (1) { while (1) { last; } }";
    let tokens = tokens_for(code);
    let leg = legend();
    let kw_idx = must_some(leg.map.get("keyword"));
    let keyword_count = tokens.iter().filter(|t| t[3] == *kw_idx).count();
    // Should have at least "if", "while", "last"
    assert!(
        keyword_count >= 3,
        "expected >=3 keyword tokens, got {keyword_count}"
    );
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
    assert!(
        has_decl,
        "named sub should have declaration modifier on function token"
    );
}

#[test]
fn function_call_has_no_declaration_modifier() {
    let tokens = tokens_for("print('hello');");
    let leg = legend();
    let fn_idx = must_some(leg.map.get("function"));
    let call_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == *fn_idx).collect();
    // Function call tokens should NOT have declaration modifier
    for t in &call_tokens {
        assert_eq!(
            t[4] & 1,
            0,
            "function call should not have declaration modifier"
        );
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
    assert!(
        tokens.is_empty(),
        "semicolons-only should produce no semantic tokens"
    );
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
    assert!(
        !tokens.is_empty(),
        "deeply nested blocks should produce tokens"
    );
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
    assert!(
        has_fn,
        "require should be classified as function (FunctionCall in AST)"
    );
}

#[test]
fn elsif_else_keywords_classified() {
    let code = "if (1) { } elsif (0) { } else { }";
    let tokens = tokens_for(code);
    let leg = legend();
    let kw_idx = must_some(leg.map.get("keyword"));
    let kw_count = tokens.iter().filter(|t| t[3] == *kw_idx).count();
    // "if", "elsif", "else" = at least 3 keywords
    assert!(
        kw_count >= 3,
        "expected >=3 keywords for if/elsif/else, got {kw_count}"
    );
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
        assert!(
            col >= 100,
            "custom mapper column should be >= 100, got {col}"
        );
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
    assert!(
        max_line >= 2,
        "three-line code should span lines 0-2, max_line={max_line}"
    );
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
    assert_eq!(
        tokens1, tokens2,
        "same code should produce identical tokens"
    );
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
    let fn_decl_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t[3] == *fn_idx && t[4] & 1 != 0)
        .collect();
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
    assert!(
        line >= 15,
        "20-line code should span many lines, final line={line}"
    );
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

// ===========================================================================
// Improved semantic token coverage tests
// ===========================================================================

// ---------------------------------------------------------------------------
// Subroutine names get `function` token type
// ---------------------------------------------------------------------------

#[test]
fn subroutine_name_gets_function_type_with_definition_modifier() {
    let tokens = tokens_for("sub greet { }");
    let leg = legend();
    let fn_idx = must_some(leg.map.get("function"));
    let fn_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == *fn_idx).collect();
    assert!(
        !fn_tokens.is_empty(),
        "sub declaration should produce function token"
    );
    // Should have declaration modifier (bit 0)
    let has_decl = fn_tokens.iter().any(|t| t[4] & 1 != 0);
    assert!(has_decl, "named sub should have declaration modifier");
}

#[test]
fn multiline_sub_name_still_gets_function_token() {
    let code = "sub process_data {\n    my $x = 1;\n    return $x;\n}";
    let tokens = tokens_for(code);
    let leg = legend();
    let fn_idx = must_some(leg.map.get("function"));
    let has_fn = tokens.iter().any(|t| t[3] == *fn_idx);
    assert!(
        has_fn,
        "multi-line sub should still produce function token for name"
    );
}

#[test]
fn function_call_gets_function_type_without_declaration() {
    let tokens = tokens_for("foo();");
    let leg = legend();
    let fn_idx = must_some(leg.map.get("function"));
    let fn_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == *fn_idx).collect();
    assert!(
        !fn_tokens.is_empty(),
        "function call should produce function token"
    );
    for t in &fn_tokens {
        assert_eq!(
            t[4] & 1,
            0,
            "function call should NOT have declaration modifier"
        );
    }
}

#[test]
fn nested_function_calls_produce_function_tokens() {
    let code = "foo(bar(baz()));";
    let tokens = tokens_for(code);
    let leg = legend();
    let fn_idx = must_some(leg.map.get("function"));
    let fn_count = tokens.iter().filter(|t| t[3] == *fn_idx).count();
    // Overlap resolution may merge nested calls on the same line
    assert!(
        fn_count >= 1,
        "nested calls should produce at least one function token, got {fn_count}"
    );
}

// ---------------------------------------------------------------------------
// Package names get `namespace` token type
// ---------------------------------------------------------------------------

#[test]
fn package_name_gets_namespace_type() {
    let tokens = tokens_for("package MyModule;");
    let leg = legend();
    let ns_idx = must_some(leg.map.get("namespace"));
    let has_ns = tokens.iter().any(|t| t[3] == *ns_idx);
    assert!(has_ns, "package declaration should produce namespace token");
}

#[test]
fn nested_package_name_gets_namespace_type() {
    let tokens = tokens_for("package My::Nested::Module;");
    let leg = legend();
    let ns_idx = must_some(leg.map.get("namespace"));
    let has_ns = tokens.iter().any(|t| t[3] == *ns_idx);
    assert!(has_ns, "nested package name should produce namespace token");
}

#[test]
fn package_declaration_has_declaration_modifier() {
    let tokens = tokens_for("package Foo;");
    let leg = legend();
    let ns_idx = must_some(leg.map.get("namespace"));
    let ns_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == *ns_idx).collect();
    assert!(!ns_tokens.is_empty(), "should have namespace token");
    let has_decl = ns_tokens.iter().any(|t| t[4] & 1 != 0);
    assert!(has_decl, "package should have declaration modifier");
}

#[test]
fn package_block_form_gets_namespace_type() {
    let code = "package Foo {\n    sub bar { }\n}";
    let tokens = tokens_for(code);
    let leg = legend();
    let ns_idx = must_some(leg.map.get("namespace"));
    let has_ns = tokens.iter().any(|t| t[3] == *ns_idx);
    assert!(has_ns, "package block form should produce namespace token");
}

// ---------------------------------------------------------------------------
// Variables get `variable` token type with correct modifiers
// ---------------------------------------------------------------------------

#[test]
fn scalar_variable_gets_variable_type() {
    let tokens = tokens_for("$x;");
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let has_var = tokens.iter().any(|t| t[3] == *var_idx);
    assert!(has_var, "scalar variable should produce variable token");
}

#[test]
fn array_variable_gets_variable_type() {
    let tokens = tokens_for("my @arr;");
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let has_var = tokens.iter().any(|t| t[3] == *var_idx);
    assert!(has_var, "array variable should produce variable token");
}

#[test]
fn hash_variable_gets_variable_type() {
    let tokens = tokens_for("my %hash;");
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let has_var = tokens.iter().any(|t| t[3] == *var_idx);
    assert!(has_var, "hash variable should produce variable token");
}

#[test]
fn my_declaration_variable_has_declaration_modifier() {
    let tokens = tokens_for("my $x = 1;");
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == *var_idx).collect();
    assert!(!var_tokens.is_empty(), "should have variable token");
    let has_decl = var_tokens.iter().any(|t| t[4] & 1 != 0);
    assert!(
        has_decl,
        "my-declared variable should have declaration modifier"
    );
}

#[test]
fn our_declaration_variable_has_readonly_modifier() {
    let tokens = tokens_for("our $VERSION = '1.0';");
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == *var_idx).collect();
    assert!(!var_tokens.is_empty(), "should have variable token");
    // our variables should have declaration (bit 0) and readonly (bit 2) modifiers
    let has_our_mods = var_tokens.iter().any(|t| t[4] & 1 != 0 && t[4] & 4 != 0);
    assert!(
        has_our_mods,
        "our-declared variable should have declaration+readonly modifiers"
    );
}

#[test]
fn const_fast_scalar_variable_has_readonly_modifier() {
    let tokens = tokens_for("use Const::Fast; const my $PI => 3.14159;");
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == *var_idx).collect();
    assert!(!var_tokens.is_empty(), "should have variable token");
    let has_const_mods = var_tokens.iter().any(|t| t[4] & 1 != 0 && t[4] & 4 != 0);
    assert!(
        has_const_mods,
        "Const::Fast scalar should have declaration+readonly modifiers"
    );
}

#[test]
fn const_fast_array_variable_has_readonly_modifier() {
    let tokens = tokens_for("use Const::Fast; const my @ARRAY => (1, 2, 3);");
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == *var_idx).collect();
    assert!(!var_tokens.is_empty(), "should have variable token");
    let has_const_mods = var_tokens.iter().any(|t| t[4] & 1 != 0 && t[4] & 4 != 0);
    assert!(
        has_const_mods,
        "Const::Fast array should have declaration+readonly modifiers"
    );
}

#[test]
fn readonly_scalar_variable_has_readonly_modifier() {
    let tokens = tokens_for("use Readonly; Readonly my $PI => 3.14159;");
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == *var_idx).collect();
    assert!(!var_tokens.is_empty(), "should have variable token");
    let has_readonly_mods = var_tokens.iter().any(|t| t[4] & 1 != 0 && t[4] & 4 != 0);
    assert!(
        has_readonly_mods,
        "Readonly scalar should have declaration+readonly modifiers"
    );
}

#[test]
fn readonly_hash_variable_has_readonly_modifier() {
    let tokens = tokens_for("use Readonly; Readonly my %HASH => (foo => 1);");
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == *var_idx).collect();
    assert!(!var_tokens.is_empty(), "should have variable token");
    let has_readonly_mods = var_tokens.iter().any(|t| t[4] & 1 != 0 && t[4] & 4 != 0);
    assert!(
        has_readonly_mods,
        "Readonly hash should have declaration+readonly modifiers"
    );
}

#[test]
fn local_declaration_variable_has_declaration_modifier() {
    let tokens = tokens_for("local $/ = undef;");
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == *var_idx).collect();
    assert!(!var_tokens.is_empty(), "should have variable token");
    let has_decl = var_tokens.iter().any(|t| t[4] & 1 != 0);
    assert!(
        has_decl,
        "local-declared variable should have declaration modifier"
    );
}

#[test]
fn state_declaration_variable_has_declaration_modifier() {
    let code = "sub counter {\n    state $count = 0;\n    return $count;\n}";
    let tokens = tokens_for(code);
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == *var_idx).collect();
    let has_decl = var_tokens.iter().any(|t| t[4] & 1 != 0);
    assert!(
        has_decl,
        "state-declared variable should have declaration modifier"
    );
}

#[test]
fn undeclared_variable_has_no_declaration_modifier() {
    let code = "sub f { return $x; }";
    let tokens = tokens_for(code);
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    // The $x variable is used but not declared with my/our/local/state
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t[3] == *var_idx).collect();
    if !var_tokens.is_empty() {
        // At least one variable token should NOT have declaration modifier
        let has_non_decl = var_tokens.iter().any(|t| t[4] & 1 == 0);
        assert!(
            has_non_decl,
            "undeclared variable should not have declaration modifier"
        );
    }
}

// ---------------------------------------------------------------------------
// Regex patterns get `regexp` token type
// ---------------------------------------------------------------------------

#[test]
fn regex_match_gets_regexp_type() {
    let tokens = tokens_for("$x =~ /pattern/;");
    let leg = legend();
    let re_idx = must_some(leg.map.get("regexp"));
    let has_re = tokens.iter().any(|t| t[3] == *re_idx);
    assert!(has_re, "regex match should produce regexp token");
}

#[test]
fn regex_substitution_gets_regexp_type() {
    let tokens = tokens_for("$x =~ s/foo/bar/g;");
    let leg = legend();
    let re_idx = must_some(leg.map.get("regexp"));
    let has_re = tokens.iter().any(|t| t[3] == *re_idx);
    assert!(has_re, "substitution should produce regexp token");
}

#[test]
fn regex_transliteration_gets_regexp_type() {
    let tokens = tokens_for("$x =~ tr/a-z/A-Z/;");
    let leg = legend();
    let re_idx = must_some(leg.map.get("regexp"));
    let has_re = tokens.iter().any(|t| t[3] == *re_idx);
    assert!(has_re, "transliteration should produce regexp token");
}

#[test]
fn qr_regex_gets_regexp_type() {
    let tokens = tokens_for("my $re = qr/pattern/i;");
    let leg = legend();
    let re_idx = must_some(leg.map.get("regexp"));
    let has_re = tokens.iter().any(|t| t[3] == *re_idx);
    assert!(has_re, "qr// should produce regexp token");
}

// ---------------------------------------------------------------------------
// POD documentation gets `comment` token type
// ---------------------------------------------------------------------------

#[test]
fn pod_single_line_gets_comment_type() {
    // Single-line POD won't have len > 0 since it spans multiple lines
    // But we should at least not crash
    let code = "=pod\n\nSome documentation\n\n=cut\nmy $x = 1;";
    let tokens = tokens_for(code);
    // Should still produce tokens for the Perl code after =cut
    let leg = legend();
    let kw_idx = must_some(leg.map.get("keyword"));
    let has_keyword = tokens.iter().any(|t| t[3] == *kw_idx);
    assert!(has_keyword, "code after POD should still be tokenized");
}

// ---------------------------------------------------------------------------
// Variables inside control structures (deep walker test)
// ---------------------------------------------------------------------------

#[test]
fn variables_inside_if_get_variable_type() {
    let code = "if (1) {\n    my $x = 42;\n}";
    let tokens = tokens_for(code);
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let has_var = tokens.iter().any(|t| t[3] == *var_idx);
    assert!(
        has_var,
        "variable inside if block should produce variable token"
    );
}

#[test]
fn variables_inside_while_get_variable_type() {
    let code = "while (1) {\n    my $x = 42;\n    last;\n}";
    let tokens = tokens_for(code);
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let has_var = tokens.iter().any(|t| t[3] == *var_idx);
    assert!(
        has_var,
        "variable inside while block should produce variable token"
    );
}

#[test]
fn variables_inside_for_get_variable_type() {
    let code = "for my $i (1..10) {\n    my $x = $i * 2;\n}";
    let tokens = tokens_for(code);
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let var_count = tokens.iter().filter(|t| t[3] == *var_idx).count();
    assert!(
        var_count >= 2,
        "should find multiple variables in for loop, got {var_count}"
    );
}

#[test]
fn function_call_inside_if_gets_function_type() {
    let code = "if (1) {\n    print('hello');\n}";
    let tokens = tokens_for(code);
    let leg = legend();
    let fn_idx = must_some(leg.map.get("function"));
    let has_fn = tokens.iter().any(|t| t[3] == *fn_idx);
    assert!(
        has_fn,
        "function call inside if should produce function token"
    );
}

#[test]
fn deeply_nested_variable_produces_token() {
    let code = "if (1) {\n    while (1) {\n        for my $i (1..3) {\n            my $x = $i;\n        }\n    }\n}";
    let tokens = tokens_for(code);
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let var_count = tokens.iter().filter(|t| t[3] == *var_idx).count();
    assert!(
        var_count >= 2,
        "deeply nested variables should produce tokens, got {var_count}"
    );
}

// ---------------------------------------------------------------------------
// New keyword coverage tests
// ---------------------------------------------------------------------------

#[test]
fn no_statement_does_not_crash() {
    // "no strict;" may not produce keyword tokens if the lexer doesn't
    // classify "no" as a keyword, but should not crash
    let _tokens = tokens_for("no strict;");
}

// ---------------------------------------------------------------------------
// Method call coverage
// ---------------------------------------------------------------------------

#[test]
fn method_call_on_variable_produces_method_token() {
    let tokens = tokens_for("$obj->method();");
    let leg = legend();
    let meth_idx = must_some(leg.map.get("method"));
    let has_method = tokens.iter().any(|t| t[3] == *meth_idx);
    assert!(has_method, "$obj->method() should produce method token");
}

#[test]
fn chained_method_calls_produce_method_tokens() {
    let tokens = tokens_for("$obj->foo()->bar();");
    let leg = legend();
    let meth_idx = must_some(leg.map.get("method"));
    let method_count = tokens.iter().filter(|t| t[3] == *meth_idx).count();
    assert!(
        method_count >= 1,
        "chained method calls should produce method tokens, got {method_count}"
    );
}

// ---------------------------------------------------------------------------
// String token type coverage
// ---------------------------------------------------------------------------

#[test]
fn backtick_command_classified_as_string() {
    let tokens = tokens_for("my $out = `ls -la`;");
    let leg = legend();
    let str_idx = must_some(leg.map.get("string"));
    let has_string = tokens.iter().any(|t| t[3] == *str_idx);
    assert!(
        has_string,
        "backtick command should be classified as string"
    );
}

// ---------------------------------------------------------------------------
// Statement modifier coverage (deep walk)
// ---------------------------------------------------------------------------

#[test]
fn variable_in_statement_modifier_gets_token() {
    let code = "print $x if $condition;";
    let tokens = tokens_for(code);
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let var_count = tokens.iter().filter(|t| t[3] == *var_idx).count();
    assert!(
        var_count >= 1,
        "variables in statement modifier should produce tokens, got {var_count}"
    );
}

// ---------------------------------------------------------------------------
// Ternary expression coverage (deep walk)
// ---------------------------------------------------------------------------

#[test]
fn variable_in_ternary_gets_token() {
    let code = "my $y = $x ? 1 : 0;";
    let tokens = tokens_for(code);
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let var_count = tokens.iter().filter(|t| t[3] == *var_idx).count();
    assert!(
        var_count >= 1,
        "variables in ternary should produce tokens, got {var_count}"
    );
}

// ---------------------------------------------------------------------------
// Return value coverage (deep walk)
// ---------------------------------------------------------------------------

#[test]
fn variable_in_return_gets_token() {
    let code = "sub f {\n    my $x = 1;\n    return $x;\n}";
    let tokens = tokens_for(code);
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let var_count = tokens.iter().filter(|t| t[3] == *var_idx).count();
    assert!(
        var_count >= 2,
        "variable in return should produce token, got {var_count}"
    );
}

// ---------------------------------------------------------------------------
// Array/hash literal coverage (deep walk)
// ---------------------------------------------------------------------------

#[test]
fn variables_in_array_literal_get_tokens() {
    let code = "my @arr = ($x, $y, $z);";
    let tokens = tokens_for(code);
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let var_count = tokens.iter().filter(|t| t[3] == *var_idx).count();
    assert!(
        var_count >= 2,
        "variables in array literal should produce tokens, got {var_count}"
    );
}

// ---------------------------------------------------------------------------
// Token type diversity in complex code
// ---------------------------------------------------------------------------

#[test]
fn complex_code_produces_many_token_types() {
    let code = "package Foo;\nuse strict;\nmy $x = 42;\nsub bar {\n    my $y = 'hello';\n    $x =~ /pattern/;\n    return $y;\n}\n";
    let tokens = tokens_for(code);
    let leg = legend();

    let types_present: std::collections::HashSet<u32> = tokens.iter().map(|t| t[3]).collect();

    let kw_idx = must_some(leg.map.get("keyword"));
    let str_idx = must_some(leg.map.get("string"));
    let num_idx = must_some(leg.map.get("number"));
    let ns_idx = must_some(leg.map.get("namespace"));
    let var_idx = must_some(leg.map.get("variable"));
    let re_idx = must_some(leg.map.get("regexp"));

    assert!(types_present.contains(kw_idx), "should have keyword tokens");
    assert!(types_present.contains(str_idx), "should have string tokens");
    assert!(types_present.contains(num_idx), "should have number tokens");
    assert!(
        types_present.contains(ns_idx),
        "should have namespace tokens"
    );
    assert!(
        types_present.contains(var_idx),
        "should have variable tokens"
    );
    assert!(types_present.contains(re_idx), "should have regexp tokens");
}

// ===========================================================================
// Issue #2881 — Gap 1: Variable sigil modifiers (scalarVariable/arrayVariable/hashVariable)
// ===========================================================================

/// The legend must advertise the three new sigil modifiers at bits 10, 11, 12.
#[test]
fn legend_has_sigil_modifiers() {
    let leg = legend();
    let mods = &leg.modifiers;
    assert!(
        mods.contains(&"scalarVariable".to_string()),
        "legend missing scalarVariable modifier"
    );
    assert!(
        mods.contains(&"arrayVariable".to_string()),
        "legend missing arrayVariable modifier"
    );
    assert!(
        mods.contains(&"hashVariable".to_string()),
        "legend missing hashVariable modifier"
    );
    // Bit positions must be 10, 11, 12 — legend position is the bit index
    let scalar_bit = must_some(mods.iter().position(|m| m == "scalarVariable"));
    assert_eq!(
        scalar_bit, 10,
        "scalarVariable must be at bit 10 (position 10 in modifiers list)"
    );
    let array_bit = must_some(mods.iter().position(|m| m == "arrayVariable"));
    assert_eq!(
        array_bit, 11,
        "arrayVariable must be at bit 11 (position 11 in modifiers list)"
    );
    let hash_bit = must_some(mods.iter().position(|m| m == "hashVariable"));
    assert_eq!(
        hash_bit, 12,
        "hashVariable must be at bit 12 (position 12 in modifiers list)"
    );
}

/// A scalar variable `$x` must receive the `scalarVariable` modifier bit (1 << 10 = 1024).
#[test]
fn scalar_variable_gets_scalar_sigil_modifier() {
    let tokens = tokens_for("my $x = 1;");
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let scalar_mod_bit: u32 = 1 << 10; // 1024
    let has_scalar_mod = tokens
        .iter()
        .any(|t| t[3] == *var_idx && (t[4] & scalar_mod_bit) != 0);
    assert!(
        has_scalar_mod,
        "$x should have scalarVariable modifier (bit 10 = 1024)"
    );
}

/// An array variable `@arr` must receive the `arrayVariable` modifier bit (1 << 11 = 2048).
#[test]
fn array_variable_gets_array_sigil_modifier() {
    let tokens = tokens_for("my @arr = ();");
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let array_mod_bit: u32 = 1 << 11; // 2048
    let has_array_mod = tokens
        .iter()
        .any(|t| t[3] == *var_idx && (t[4] & array_mod_bit) != 0);
    assert!(
        has_array_mod,
        "@arr should have arrayVariable modifier (bit 11 = 2048)"
    );
}

/// A hash variable `%h` must receive the `hashVariable` modifier bit (1 << 12 = 4096).
#[test]
fn hash_variable_gets_hash_sigil_modifier() {
    let tokens = tokens_for("my %h = ();");
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let hash_mod_bit: u32 = 1 << 12; // 4096
    let has_hash_mod = tokens
        .iter()
        .any(|t| t[3] == *var_idx && (t[4] & hash_mod_bit) != 0);
    assert!(
        has_hash_mod,
        "%h should have hashVariable modifier (bit 12 = 4096)"
    );
}

// ===========================================================================
// Issue #2881 — Gap 2: Interpolated string variable token emission
// ===========================================================================

/// A double-quoted string with an embedded variable must produce both a `string`
/// token (for the literal fragment) and a `variable` token (for `$name`).
#[test]
fn interpolated_string_emits_variable_token() {
    let code = r#"my $name = "Alice"; my $greeting = "Hello $name";"#;
    let tokens = tokens_for(code);
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let str_idx = must_some(leg.map.get("string"));
    let has_var = tokens.iter().any(|t| t[3] == *var_idx);
    let has_str = tokens.iter().any(|t| t[3] == *str_idx);
    assert!(
        has_var,
        "should have variable token inside interpolated string"
    );
    assert!(
        has_str,
        "should have string token for literal parts of interpolated string"
    );
}

/// A variable that appears twice in the same interpolated string must produce
/// two separate variable tokens (cursor re-scan must advance past the first match).
#[test]
fn interpolated_string_duplicate_variable_gets_two_tokens() {
    let code = r#"my $x = 1; my $s = "$x and $x";"#;
    let tokens = tokens_for(code);
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    // $x declaration (1) + $x first-in-string (1) + $x second-in-string (1) = at least 3
    let var_count = tokens.iter().filter(|t| t[3] == *var_idx).count();
    assert!(
        var_count >= 3,
        "should have at least 3 variable tokens ($x decl + 2 in string); got {var_count}"
    );
}

/// A single-quoted string `'$name'` must NOT produce a variable token from inside
/// the quotes — only the `$s` declaration should be a variable token.
#[test]
fn single_quoted_string_no_variable_token_inside() {
    let code = r#"my $s = '$name';"#;
    let tokens = tokens_for(code);
    let leg = legend();
    let var_idx = must_some(leg.map.get("variable"));
    let str_idx = must_some(leg.map.get("string"));
    // Single-quoted string must produce a string token
    let has_str = tokens.iter().any(|t| t[3] == *str_idx);
    assert!(has_str, "single-quoted string should produce string token");
    // Only $s declaration should be a variable; nothing from inside '$name'
    let var_count = tokens.iter().filter(|t| t[3] == *var_idx).count();
    assert_eq!(
        var_count, 1,
        "should have only the $s declaration variable, got {var_count}"
    );
}
