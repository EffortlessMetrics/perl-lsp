//! Integration tests for bareword/regex disambiguation via `LocalSymbolTable`.
//!
//! Covers issue #1353: the lexer now consults a pre-scanned symbol table to
//! correctly identify `identifier /regex/` vs `identifier / expr`.

use perl_lexer::{LexerConfig, LocalSymbolTable, PerlLexer, TokenType};

// --- helpers ---

fn tokenize_with_table(src: &str) -> Vec<TokenType> {
    let st = LocalSymbolTable::scan_subs(src);
    let config = LexerConfig { symbol_table: Some(st), ..LexerConfig::default() };
    let mut lexer = PerlLexer::with_config(src, config);
    lexer.collect_tokens().into_iter().map(|t| t.token_type).collect()
}

fn tokenize_no_table(src: &str) -> Vec<TokenType> {
    let mut lexer = PerlLexer::new(src);
    lexer.collect_tokens().into_iter().map(|t| t.token_type).collect()
}

fn has_regex(toks: &[TokenType]) -> bool {
    toks.iter().any(|t| matches!(t, TokenType::RegexMatch))
}

fn has_division(toks: &[TokenType]) -> bool {
    toks.iter().any(|t| matches!(t, TokenType::Division))
}

// --- core fix: known sub → regex ---

#[test]
fn known_sub_forward_decl_slash_is_regex() {
    // `sub builder` declared before call site
    let src = "sub builder;\nbuilder /foo|bar/;";
    let toks = tokenize_with_table(src);
    assert!(has_regex(&toks), "slash after known sub should be lexed as regex");
    assert!(!has_division(&toks));
}

#[test]
fn known_sub_full_body_slash_is_regex() {
    let src = "sub transform { }\ntransform /pattern/;";
    let toks = tokenize_with_table(src);
    assert!(has_regex(&toks), "slash after known sub should be lexed as regex");
}

#[test]
fn known_sub_forward_reference_slash_is_regex() {
    // Call site appears BEFORE the declaration; pre-pass still sees the sub.
    let src = "builder /foo|bar/;\nsub builder { }";
    let toks = tokenize_with_table(src);
    assert!(has_regex(&toks), "pre-pass enables forward-reference disambiguation");
}

// --- safe default: unknown identifier → division ---

#[test]
fn unknown_identifier_slash_is_division() {
    // No sub declaration → default behaviour: slash is division.
    let src = "unknown_func /foo|bar/;";
    let toks = tokenize_with_table(src);
    assert!(has_division(&toks), "unknown bareword should produce division");
    assert!(!has_regex(&toks));
}

// --- built-in functions still use ExpectTerm (priority check) ---

#[test]
fn builtin_print_slash_is_regex_without_symbol_table() {
    let src = "print /pattern/;";
    let toks = tokenize_no_table(src);
    assert!(has_regex(&toks), "built-in print should still introduce regex without symbol table");
}

#[test]
fn builtin_takes_precedence_over_symbol_table() {
    // Even if 'print' were somehow in the symbol table, it's a builtin first.
    let src = "sub print { } print /p/;";
    let toks = tokenize_with_table(src);
    assert!(has_regex(&toks), "built-in check fires before symbol table lookup");
}

// --- no regression on division after terms ---

#[test]
fn division_after_variable_is_unchanged() {
    let src = "my $x = $y / 2;";
    let toks = tokenize_with_table(src);
    assert!(has_division(&toks), "division after variable must not be misread");
    assert!(!has_regex(&toks));
}

#[test]
fn division_after_number_is_unchanged() {
    let src = "my $x = 10 / 3;";
    let toks = tokenize_with_table(src);
    assert!(has_division(&toks));
}

// --- comment / string filtering doesn't leak into symbol table ---

#[test]
fn commented_out_sub_still_produces_division() {
    // `# sub foo` is a comment; `foo` should NOT be in the table.
    let src = "# sub commented { }\ncommented /x/;";
    let toks = tokenize_with_table(src);
    assert!(has_division(&toks), "commented-out sub must not be registered");
}

#[test]
fn sub_in_single_quoted_string_still_produces_division() {
    let src = r#"my $t = 'sub in_str { }'; in_str /x/;"#;
    let toks = tokenize_with_table(src);
    assert!(has_division(&toks), "sub inside single-quoted string must not be registered");
}

#[test]
fn sub_in_double_quoted_string_still_produces_division() {
    let src = r#"my $t = "sub in_str { }"; in_str /x/;"#;
    let toks = tokenize_with_table(src);
    assert!(has_division(&toks), "sub inside double-quoted string must not be registered");
}

// --- multiple subs in the same file ---

#[test]
fn multiple_known_subs_all_trigger_regex_mode() {
    let src = "sub alpha { }\nsub beta { }\nalpha /a/; beta /b/;";
    let toks = tokenize_with_table(src);
    let regex_count = toks.iter().filter(|t| matches!(t, TokenType::RegexMatch)).count();
    assert_eq!(regex_count, 2, "both known subs should trigger regex mode");
}

// --- non-lowercase keyword: partial match guard ---

#[test]
fn identifier_starting_with_sub_as_prefix_is_not_matched() {
    // "substring" should NOT be misidentified as a `sub` declaration.
    let src = "substring /x/;";
    let toks = tokenize_with_table(src);
    assert!(has_division(&toks), "'substring' should not be registered as a sub");
}
