//! Comprehensive integration tests for the `perl-keywords` crate.
//!
//! These tests exercise every public constant and lookup function exported
//! by the crate, covering sorting invariants, subset relationships, boundary
//! conditions, edge cases, and negative lookups.

use perl_lexer::{
    DAP_COMPLETION_KEYWORDS, KEYWORDS, LEXER_KEYWORDS, LSP_COMPLETION_KEYWORDS,
    LSP_RUNTIME_COMPLETION_KEYWORDS, PARSER_LSP_KEYWORDS, RENAME_KEYWORDS,
    is_dap_completion_keyword, is_keyword, is_lexer_keyword, is_lsp_completion_keyword,
    is_lsp_runtime_completion_keyword, is_parser_lsp_keyword, is_rename_keyword,
};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Verify that a slice is strictly sorted (ascending) with no duplicates.
fn is_strictly_sorted(items: &[&str]) -> bool {
    items.windows(2).all(|w| w[0] < w[1])
}

// ---------------------------------------------------------------------------
// Non-emptiness
// ---------------------------------------------------------------------------

#[test]
fn all_keyword_lists_are_non_empty() {
    assert!(!KEYWORDS.is_empty());
    assert!(!LSP_COMPLETION_KEYWORDS.is_empty());
    assert!(!DAP_COMPLETION_KEYWORDS.is_empty());
    assert!(!LSP_RUNTIME_COMPLETION_KEYWORDS.is_empty());
    assert!(!RENAME_KEYWORDS.is_empty());
    assert!(!PARSER_LSP_KEYWORDS.is_empty());
    assert!(!LEXER_KEYWORDS.is_empty());
}

// ---------------------------------------------------------------------------
// Sorting invariants (required for binary_search correctness)
// ---------------------------------------------------------------------------

#[test]
fn keywords_is_sorted() {
    assert!(is_strictly_sorted(KEYWORDS), "KEYWORDS must be strictly sorted");
}

#[test]
fn lsp_completion_keywords_is_sorted() {
    assert!(
        is_strictly_sorted(LSP_COMPLETION_KEYWORDS),
        "LSP_COMPLETION_KEYWORDS must be strictly sorted"
    );
}

#[test]
fn dap_completion_keywords_is_sorted() {
    assert!(
        is_strictly_sorted(DAP_COMPLETION_KEYWORDS),
        "DAP_COMPLETION_KEYWORDS must be strictly sorted"
    );
}

#[test]
fn lsp_runtime_completion_keywords_is_sorted() {
    assert!(
        is_strictly_sorted(LSP_RUNTIME_COMPLETION_KEYWORDS),
        "LSP_RUNTIME_COMPLETION_KEYWORDS must be strictly sorted"
    );
}

#[test]
fn rename_keywords_is_sorted() {
    assert!(is_strictly_sorted(RENAME_KEYWORDS), "RENAME_KEYWORDS must be strictly sorted");
}

#[test]
fn parser_lsp_keywords_is_sorted() {
    assert!(is_strictly_sorted(PARSER_LSP_KEYWORDS), "PARSER_LSP_KEYWORDS must be strictly sorted");
}

#[test]
fn lexer_keywords_is_sorted() {
    assert!(is_strictly_sorted(LEXER_KEYWORDS), "LEXER_KEYWORDS must be strictly sorted");
}

// ---------------------------------------------------------------------------
// Subset relationships — every specialized list ⊆ KEYWORDS
// ---------------------------------------------------------------------------

#[test]
fn lsp_completion_keywords_subset_of_keywords() {
    for &kw in LSP_COMPLETION_KEYWORDS {
        assert!(is_keyword(kw), "LSP_COMPLETION_KEYWORDS entry {kw:?} missing from KEYWORDS");
    }
}

#[test]
fn dap_completion_keywords_subset_of_keywords() {
    for &kw in DAP_COMPLETION_KEYWORDS {
        assert!(is_keyword(kw), "DAP_COMPLETION_KEYWORDS entry {kw:?} missing from KEYWORDS");
    }
}

#[test]
fn lsp_runtime_completion_keywords_subset_of_keywords() {
    for &kw in LSP_RUNTIME_COMPLETION_KEYWORDS {
        assert!(
            is_keyword(kw),
            "LSP_RUNTIME_COMPLETION_KEYWORDS entry {kw:?} missing from KEYWORDS"
        );
    }
}

#[test]
fn rename_keywords_subset_of_keywords() {
    for &kw in RENAME_KEYWORDS {
        assert!(is_keyword(kw), "RENAME_KEYWORDS entry {kw:?} missing from KEYWORDS");
    }
}

#[test]
fn parser_lsp_keywords_subset_of_keywords() {
    for &kw in PARSER_LSP_KEYWORDS {
        assert!(is_keyword(kw), "PARSER_LSP_KEYWORDS entry {kw:?} missing from KEYWORDS");
    }
}

#[test]
fn lexer_keywords_subset_of_keywords() {
    for &kw in LEXER_KEYWORDS {
        assert!(is_keyword(kw), "LEXER_KEYWORDS entry {kw:?} missing from KEYWORDS");
    }
}

// ---------------------------------------------------------------------------
// No duplicate entries across individual lists
// ---------------------------------------------------------------------------

fn has_no_duplicates(items: &[&str]) -> bool {
    let mut seen = std::collections::HashSet::new();
    items.iter().all(|item| seen.insert(*item))
}

#[test]
fn no_duplicates_in_any_keyword_list() {
    assert!(has_no_duplicates(KEYWORDS), "KEYWORDS has duplicates");
    assert!(has_no_duplicates(LSP_COMPLETION_KEYWORDS), "LSP_COMPLETION_KEYWORDS has duplicates");
    assert!(has_no_duplicates(DAP_COMPLETION_KEYWORDS), "DAP_COMPLETION_KEYWORDS has duplicates");
    assert!(
        has_no_duplicates(LSP_RUNTIME_COMPLETION_KEYWORDS),
        "LSP_RUNTIME_COMPLETION_KEYWORDS has duplicates"
    );
    assert!(has_no_duplicates(RENAME_KEYWORDS), "RENAME_KEYWORDS has duplicates");
    assert!(has_no_duplicates(PARSER_LSP_KEYWORDS), "PARSER_LSP_KEYWORDS has duplicates");
    assert!(has_no_duplicates(LEXER_KEYWORDS), "LEXER_KEYWORDS has duplicates");
}

// ---------------------------------------------------------------------------
// Boundary element lookups (first and last in each list)
// ---------------------------------------------------------------------------

#[test]
fn first_and_last_keywords_are_found() -> Result<(), String> {
    let first = KEYWORDS.first().ok_or("KEYWORDS empty")?;
    let last = KEYWORDS.last().ok_or("KEYWORDS empty")?;
    assert!(is_keyword(first), "first keyword not found via is_keyword");
    assert!(is_keyword(last), "last keyword not found via is_keyword");
    Ok(())
}

#[test]
fn first_and_last_lsp_completion_keywords_are_found() -> Result<(), String> {
    let first = LSP_COMPLETION_KEYWORDS.first().ok_or("LSP_COMPLETION_KEYWORDS empty")?;
    let last = LSP_COMPLETION_KEYWORDS.last().ok_or("LSP_COMPLETION_KEYWORDS empty")?;
    assert!(is_lsp_completion_keyword(first));
    assert!(is_lsp_completion_keyword(last));
    Ok(())
}

#[test]
fn first_and_last_dap_completion_keywords_are_found() -> Result<(), String> {
    let first = DAP_COMPLETION_KEYWORDS.first().ok_or("DAP_COMPLETION_KEYWORDS empty")?;
    let last = DAP_COMPLETION_KEYWORDS.last().ok_or("DAP_COMPLETION_KEYWORDS empty")?;
    assert!(is_dap_completion_keyword(first));
    assert!(is_dap_completion_keyword(last));
    Ok(())
}

#[test]
fn first_and_last_lsp_runtime_keywords_are_found() -> Result<(), String> {
    let first =
        LSP_RUNTIME_COMPLETION_KEYWORDS.first().ok_or("LSP_RUNTIME_COMPLETION_KEYWORDS empty")?;
    let last =
        LSP_RUNTIME_COMPLETION_KEYWORDS.last().ok_or("LSP_RUNTIME_COMPLETION_KEYWORDS empty")?;
    assert!(is_lsp_runtime_completion_keyword(first));
    assert!(is_lsp_runtime_completion_keyword(last));
    Ok(())
}

#[test]
fn first_and_last_rename_keywords_are_found() -> Result<(), String> {
    let first = RENAME_KEYWORDS.first().ok_or("RENAME_KEYWORDS empty")?;
    let last = RENAME_KEYWORDS.last().ok_or("RENAME_KEYWORDS empty")?;
    assert!(is_rename_keyword(first));
    assert!(is_rename_keyword(last));
    Ok(())
}

#[test]
fn first_and_last_parser_lsp_keywords_are_found() -> Result<(), String> {
    let first = PARSER_LSP_KEYWORDS.first().ok_or("PARSER_LSP_KEYWORDS empty")?;
    let last = PARSER_LSP_KEYWORDS.last().ok_or("PARSER_LSP_KEYWORDS empty")?;
    assert!(is_parser_lsp_keyword(first));
    assert!(is_parser_lsp_keyword(last));
    Ok(())
}

#[test]
fn first_and_last_lexer_keywords_are_found() -> Result<(), String> {
    let first = LEXER_KEYWORDS.first().ok_or("LEXER_KEYWORDS empty")?;
    let last = LEXER_KEYWORDS.last().ok_or("LEXER_KEYWORDS empty")?;
    assert!(is_lexer_keyword(first));
    assert!(is_lexer_keyword(last));
    Ok(())
}

// ---------------------------------------------------------------------------
// Special-token categories present in KEYWORDS
// ---------------------------------------------------------------------------

#[test]
fn phase_blocks_are_keywords() {
    for kw in ["BEGIN", "CHECK", "INIT", "END", "UNITCHECK"] {
        assert!(is_keyword(kw), "phase block {kw:?} missing from KEYWORDS");
    }
}

#[test]
fn dunder_tokens_are_keywords() {
    for kw in ["__FILE__", "__LINE__", "__PACKAGE__", "__SUB__"] {
        assert!(is_keyword(kw), "dunder token {kw:?} missing from KEYWORDS");
    }
}

#[test]
fn single_char_keywords_are_present() {
    for kw in ["m", "q", "s", "y"] {
        assert!(is_keyword(kw), "single-char keyword {kw:?} missing from KEYWORDS");
    }
}

#[test]
fn quote_like_operators_are_keywords() {
    for kw in ["q", "qq", "qr", "qw", "qx"] {
        assert!(is_keyword(kw), "quote-like operator {kw:?} missing from KEYWORDS");
    }
}

#[test]
fn comparison_operators_are_keywords() {
    for kw in ["cmp", "eq", "ge", "gt", "le", "lt", "ne"] {
        assert!(is_keyword(kw), "comparison operator {kw:?} missing from KEYWORDS");
    }
}

#[test]
fn control_flow_keywords_present() {
    for kw in [
        "if", "elsif", "else", "unless", "while", "until", "for", "foreach", "last", "next",
        "redo", "return", "goto",
    ] {
        assert!(is_keyword(kw), "control-flow keyword {kw:?} missing from KEYWORDS");
    }
}

#[test]
fn variable_declaration_keywords_present() {
    for kw in ["my", "our", "local", "state"] {
        assert!(is_keyword(kw), "variable-declaration keyword {kw:?} missing from KEYWORDS");
    }
}

#[test]
fn oop_keywords_present() {
    for kw in ["bless", "blessed", "ref", "tie", "untie"] {
        assert!(is_keyword(kw), "OOP keyword {kw:?} missing from KEYWORDS");
    }
}

#[test]
fn modern_perl_keywords_present() {
    for kw in ["try", "catch", "finally", "class", "method", "ADJUST", "isa"] {
        assert!(is_keyword(kw), "modern Perl keyword {kw:?} missing from KEYWORDS");
    }
}

#[test]
fn regex_and_transliteration_keywords_present() {
    for kw in ["m", "s", "tr", "y", "qr"] {
        assert!(is_keyword(kw), "regex/transliteration keyword {kw:?} missing from KEYWORDS");
    }
}

// ---------------------------------------------------------------------------
// Case sensitivity — lookups are exact-match
// ---------------------------------------------------------------------------

#[test]
fn lookups_are_case_sensitive() {
    assert!(is_keyword("my"));
    assert!(!is_keyword("My"));
    assert!(!is_keyword("MY"));

    assert!(is_keyword("BEGIN"));
    assert!(!is_keyword("begin"));
    assert!(!is_keyword("Begin"));

    assert!(is_keyword("__PACKAGE__"));
    assert!(!is_keyword("__package__"));
    assert!(!is_keyword("__Package__"));
}

#[test]
fn lexer_lookup_is_case_sensitive() {
    assert!(is_lexer_keyword("BEGIN"));
    assert!(!is_lexer_keyword("begin"));

    assert!(is_lexer_keyword("my"));
    assert!(!is_lexer_keyword("My"));
}

#[test]
fn lsp_completion_lookup_is_case_sensitive() {
    assert!(is_lsp_completion_keyword("my"));
    assert!(!is_lsp_completion_keyword("My"));

    assert!(is_lsp_completion_keyword("BEGIN"));
    assert!(!is_lsp_completion_keyword("begin"));
}

#[test]
fn dap_completion_lookup_is_case_sensitive() {
    assert!(is_dap_completion_keyword("my"));
    assert!(!is_dap_completion_keyword("My"));
}

// ---------------------------------------------------------------------------
// Empty and whitespace inputs (negative)
// ---------------------------------------------------------------------------

#[test]
fn empty_string_is_not_a_keyword() {
    assert!(!is_keyword(""));
    assert!(!is_lexer_keyword(""));
    assert!(!is_lsp_completion_keyword(""));
    assert!(!is_dap_completion_keyword(""));
    assert!(!is_lsp_runtime_completion_keyword(""));
    assert!(!is_rename_keyword(""));
    assert!(!is_parser_lsp_keyword(""));
}

#[test]
fn whitespace_strings_are_not_keywords() {
    for s in [" ", "\t", "\n", "  my  ", " if"] {
        assert!(!is_keyword(s), "{s:?} should not be a keyword");
    }
}

// ---------------------------------------------------------------------------
// Nonsense and near-miss inputs (negative)
// ---------------------------------------------------------------------------

#[test]
fn nonsense_tokens_are_not_keywords() {
    for s in [
        "foo", "bar", "Perl", "PERL", "perl", "123", "my_var", "$_", "@ARGV", "%ENV", "->", "::",
        "//", "=>",
    ] {
        assert!(!is_keyword(s), "{s:?} should not be a keyword");
    }
}

#[test]
fn partial_keywords_are_not_keywords() {
    for s in ["fo", "forr", "fore", "foreac", "subs", "prin", "iff"] {
        assert!(!is_keyword(s), "partial {s:?} should not be a keyword");
    }
}

#[test]
fn keywords_with_extra_chars_are_not_keywords() {
    assert!(!is_keyword("my "));
    assert!(!is_keyword("sub;"));
    assert!(!is_keyword("(if)"));
    assert!(!is_keyword("use\n"));
}

// ---------------------------------------------------------------------------
// Lookup function / constant consistency
// ---------------------------------------------------------------------------

#[test]
fn is_keyword_consistent_with_keywords_constant() {
    for &kw in KEYWORDS {
        assert!(is_keyword(kw), "is_keyword({kw:?}) should be true for every KEYWORDS entry");
    }
}

#[test]
fn is_lexer_keyword_consistent_with_constant() {
    for &kw in LEXER_KEYWORDS {
        assert!(is_lexer_keyword(kw), "is_lexer_keyword({kw:?}) should be true");
    }
}

#[test]
fn is_lsp_completion_keyword_consistent_with_constant() {
    for &kw in LSP_COMPLETION_KEYWORDS {
        assert!(is_lsp_completion_keyword(kw), "is_lsp_completion_keyword({kw:?}) should be true");
    }
}

#[test]
fn is_dap_completion_keyword_consistent_with_constant() {
    for &kw in DAP_COMPLETION_KEYWORDS {
        assert!(is_dap_completion_keyword(kw), "is_dap_completion_keyword({kw:?}) should be true");
    }
}

#[test]
fn is_lsp_runtime_completion_keyword_consistent_with_constant() {
    for &kw in LSP_RUNTIME_COMPLETION_KEYWORDS {
        assert!(
            is_lsp_runtime_completion_keyword(kw),
            "is_lsp_runtime_completion_keyword({kw:?}) should be true"
        );
    }
}

#[test]
fn is_rename_keyword_consistent_with_constant() {
    for &kw in RENAME_KEYWORDS {
        assert!(is_rename_keyword(kw), "is_rename_keyword({kw:?}) should be true");
    }
}

#[test]
fn is_parser_lsp_keyword_consistent_with_constant() {
    for &kw in PARSER_LSP_KEYWORDS {
        assert!(is_parser_lsp_keyword(kw), "is_parser_lsp_keyword({kw:?}) should be true");
    }
}

// ---------------------------------------------------------------------------
// Cross-list negative lookups — entries in one list but not another
// ---------------------------------------------------------------------------

#[test]
fn autoload_and_destroy_not_in_dap_keywords() {
    assert!(!is_dap_completion_keyword("AUTOLOAD"));
    assert!(!is_dap_completion_keyword("DESTROY"));
}

#[test]
fn modern_perl_not_in_rename_keywords() {
    for kw in ["try", "catch", "finally", "class", "method", "ADJUST", "isa"] {
        assert!(!is_rename_keyword(kw), "{kw:?} should not be a rename keyword");
    }
}

#[test]
fn print_not_in_lsp_completion() {
    assert!(!is_lsp_completion_keyword("print"));
    assert!(!is_lsp_completion_keyword("printf"));
}

#[test]
fn dunder_tokens_not_in_lexer_keywords() {
    assert!(!is_lexer_keyword("__FILE__"));
    assert!(!is_lexer_keyword("__LINE__"));
    assert!(!is_lexer_keyword("__PACKAGE__"));
    assert!(!is_lexer_keyword("__SUB__"));
}

#[test]
fn autoload_not_in_runtime_completion() {
    assert!(!is_lsp_runtime_completion_keyword("AUTOLOAD"));
    assert!(!is_lsp_runtime_completion_keyword("DESTROY"));
    assert!(!is_lsp_runtime_completion_keyword("BEGIN"));
}

#[test]
fn quote_operators_not_in_rename_keywords() {
    for kw in ["q", "qq", "qr", "qw", "qx", "m", "s", "tr", "y"] {
        assert!(!is_rename_keyword(kw), "{kw:?} should not be a rename keyword");
    }
}

// ---------------------------------------------------------------------------
// Cardinality sanity checks
// ---------------------------------------------------------------------------

#[test]
fn keywords_has_reasonable_count() {
    // The canonical list should have at least 100 entries.
    assert!(KEYWORDS.len() >= 100, "KEYWORDS too small: {}", KEYWORDS.len());
}

#[test]
fn specialized_lists_are_smaller_than_keywords() {
    assert!(LSP_COMPLETION_KEYWORDS.len() < KEYWORDS.len());
    assert!(DAP_COMPLETION_KEYWORDS.len() < KEYWORDS.len());
    assert!(LSP_RUNTIME_COMPLETION_KEYWORDS.len() < KEYWORDS.len());
    assert!(RENAME_KEYWORDS.len() < KEYWORDS.len());
    assert!(PARSER_LSP_KEYWORDS.len() < KEYWORDS.len());
    assert!(LEXER_KEYWORDS.len() < KEYWORDS.len());
}

#[test]
fn rename_keywords_is_smallest_specialized_list() {
    // Rename validation uses the tightest set of reserved words.
    assert!(RENAME_KEYWORDS.len() <= LSP_COMPLETION_KEYWORDS.len());
    assert!(RENAME_KEYWORDS.len() <= DAP_COMPLETION_KEYWORDS.len());
    assert!(RENAME_KEYWORDS.len() <= LEXER_KEYWORDS.len());
}

// ---------------------------------------------------------------------------
// Keyword entries contain no leading/trailing whitespace
// ---------------------------------------------------------------------------

#[test]
fn no_keyword_has_leading_or_trailing_whitespace() {
    let all_lists: &[(&str, &[&str])] = &[
        ("KEYWORDS", KEYWORDS),
        ("LSP_COMPLETION_KEYWORDS", LSP_COMPLETION_KEYWORDS),
        ("DAP_COMPLETION_KEYWORDS", DAP_COMPLETION_KEYWORDS),
        ("LSP_RUNTIME_COMPLETION_KEYWORDS", LSP_RUNTIME_COMPLETION_KEYWORDS),
        ("RENAME_KEYWORDS", RENAME_KEYWORDS),
        ("PARSER_LSP_KEYWORDS", PARSER_LSP_KEYWORDS),
        ("LEXER_KEYWORDS", LEXER_KEYWORDS),
    ];
    for &(name, list) in all_lists {
        for &kw in list {
            assert_eq!(kw, kw.trim(), "{name} entry {kw:?} has leading/trailing whitespace");
        }
    }
}

// ---------------------------------------------------------------------------
// All keywords are non-empty strings
// ---------------------------------------------------------------------------

#[test]
fn no_keyword_is_empty_string() {
    let all_lists: &[(&str, &[&str])] = &[
        ("KEYWORDS", KEYWORDS),
        ("LSP_COMPLETION_KEYWORDS", LSP_COMPLETION_KEYWORDS),
        ("DAP_COMPLETION_KEYWORDS", DAP_COMPLETION_KEYWORDS),
        ("LSP_RUNTIME_COMPLETION_KEYWORDS", LSP_RUNTIME_COMPLETION_KEYWORDS),
        ("RENAME_KEYWORDS", RENAME_KEYWORDS),
        ("PARSER_LSP_KEYWORDS", PARSER_LSP_KEYWORDS),
        ("LEXER_KEYWORDS", LEXER_KEYWORDS),
    ];
    for &(name, list) in all_lists {
        for &kw in list {
            assert!(!kw.is_empty(), "{name} contains an empty string entry");
        }
    }
}

// ---------------------------------------------------------------------------
// All keywords are ASCII (Perl keywords are pure ASCII)
// ---------------------------------------------------------------------------

#[test]
fn all_keywords_are_ascii() {
    for &kw in KEYWORDS {
        assert!(kw.is_ascii(), "keyword {kw:?} contains non-ASCII characters");
    }
}

// ---------------------------------------------------------------------------
// Specific well-known keywords appear in expected lists
// ---------------------------------------------------------------------------

#[test]
fn sub_is_in_all_relevant_lists() {
    assert!(is_keyword("sub"));
    assert!(is_lsp_completion_keyword("sub"));
    assert!(is_dap_completion_keyword("sub"));
    assert!(is_rename_keyword("sub"));
    assert!(is_parser_lsp_keyword("sub"));
    assert!(is_lexer_keyword("sub"));
}

#[test]
fn my_is_in_all_relevant_lists() {
    assert!(is_keyword("my"));
    assert!(is_lsp_completion_keyword("my"));
    assert!(is_dap_completion_keyword("my"));
    assert!(is_lsp_runtime_completion_keyword("my"));
    assert!(is_rename_keyword("my"));
    assert!(is_parser_lsp_keyword("my"));
    assert!(is_lexer_keyword("my"));
}

#[test]
fn if_is_in_all_relevant_lists() {
    assert!(is_keyword("if"));
    assert!(is_lsp_completion_keyword("if"));
    assert!(is_dap_completion_keyword("if"));
    assert!(is_lsp_runtime_completion_keyword("if"));
    assert!(is_rename_keyword("if"));
    assert!(is_parser_lsp_keyword("if"));
    assert!(is_lexer_keyword("if"));
}

#[test]
fn use_is_in_all_relevant_lists() {
    assert!(is_keyword("use"));
    assert!(is_lsp_completion_keyword("use"));
    assert!(is_dap_completion_keyword("use"));
    assert!(is_lsp_runtime_completion_keyword("use"));
    assert!(is_rename_keyword("use"));
    assert!(is_parser_lsp_keyword("use"));
    assert!(is_lexer_keyword("use"));
}

#[test]
fn return_is_in_all_relevant_lists() {
    assert!(is_keyword("return"));
    assert!(is_lsp_completion_keyword("return"));
    assert!(is_dap_completion_keyword("return"));
    assert!(is_lsp_runtime_completion_keyword("return"));
    assert!(is_rename_keyword("return"));
    assert!(is_parser_lsp_keyword("return"));
    assert!(is_lexer_keyword("return"));
}

// ---------------------------------------------------------------------------
// I/O and builtin function keywords
// ---------------------------------------------------------------------------

#[test]
fn io_builtins_are_keywords() {
    for kw in ["open", "close", "read", "print", "printf", "say", "write"] {
        assert!(is_keyword(kw), "I/O builtin {kw:?} missing from KEYWORDS");
    }
}

#[test]
fn string_builtins_are_keywords() {
    for kw in [
        "chomp", "chop", "chr", "hex", "index", "lc", "lcfirst", "length", "oct", "ord", "rindex",
        "substr", "uc", "ucfirst",
    ] {
        assert!(is_keyword(kw), "string builtin {kw:?} missing from KEYWORDS");
    }
}

#[test]
fn list_builtins_are_keywords() {
    for kw in [
        "grep", "join", "map", "pop", "push", "reverse", "shift", "sort", "splice", "split",
        "unshift", "values", "keys", "each",
    ] {
        assert!(is_keyword(kw), "list builtin {kw:?} missing from KEYWORDS");
    }
}

// ---------------------------------------------------------------------------
// Logical operator keywords
// ---------------------------------------------------------------------------

#[test]
fn logical_operators_are_keywords() {
    for kw in ["and", "or", "not", "xor"] {
        assert!(is_keyword(kw), "logical operator {kw:?} missing from KEYWORDS");
    }
}

// ---------------------------------------------------------------------------
// Switch-like keywords (given/when/default)
// ---------------------------------------------------------------------------

#[test]
fn switch_keywords_present() {
    for kw in ["given", "when", "default"] {
        assert!(is_keyword(kw), "switch keyword {kw:?} missing from KEYWORDS");
    }
}

// ---------------------------------------------------------------------------
// Misc keywords
// ---------------------------------------------------------------------------

#[test]
fn misc_keywords_present() {
    for kw in [
        "abs",
        "break",
        "continue",
        "defined",
        "delete",
        "die",
        "do",
        "eval",
        "exists",
        "exit",
        "format",
        "int",
        "pack",
        "scalar",
        "sprintf",
        "sqrt",
        "undef",
        "unpack",
        "wantarray",
        "warn",
    ] {
        assert!(is_keyword(kw), "misc keyword {kw:?} missing from KEYWORDS");
    }
}
