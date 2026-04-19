//! Additional edge-case and cross-list tests for `perl-keywords`.
//!
//! This file complements `comprehensive_unit_tests.rs` with focused tests on
//! cross-list exclusion, binary-search boundary behavior, unicode/sigil edge
//! cases, keyword-only membership, and count stability.

use perl_lexer::{
    DAP_COMPLETION_KEYWORDS, KEYWORDS, LEXER_KEYWORDS, LSP_COMPLETION_KEYWORDS,
    LSP_RUNTIME_COMPLETION_KEYWORDS, PARSER_LSP_KEYWORDS, RENAME_KEYWORDS,
    is_dap_completion_keyword, is_keyword, is_lexer_keyword, is_lsp_completion_keyword,
    is_lsp_runtime_completion_keyword, is_parser_lsp_keyword, is_rename_keyword,
};

// ---------------------------------------------------------------------------
// Binary-search boundary behavior
// ---------------------------------------------------------------------------

#[test]
fn token_just_before_first_keyword_is_not_found() {
    // "AUTOLOAD" is first in KEYWORDS; "AUTOLOA" sorts before it.
    assert!(!is_keyword("AUTOLOA"));
}

#[test]
fn token_just_after_last_keyword_is_not_found() {
    // "y" is last in KEYWORDS; "z" sorts after it.
    assert!(!is_keyword("z"));
    assert!(!is_keyword("yy"));
}

#[test]
fn token_between_adjacent_keywords_is_not_found() {
    // "abs" and "and" are adjacent in KEYWORDS; "abe" sorts between them.
    assert!(!is_keyword("abe"));
    // "for" and "foreach" are adjacent; "forb" sorts between them.
    assert!(!is_keyword("forb"));
}

// ---------------------------------------------------------------------------
// Keywords present ONLY in KEYWORDS (not in any specialized list)
// ---------------------------------------------------------------------------

#[test]
fn autoload_only_in_keywords() {
    assert!(is_keyword("AUTOLOAD"));
    assert!(!is_lexer_keyword("AUTOLOAD"));
    assert!(!is_dap_completion_keyword("AUTOLOAD"));
    assert!(!is_lsp_runtime_completion_keyword("AUTOLOAD"));
    assert!(!is_rename_keyword("AUTOLOAD"));
    assert!(!is_parser_lsp_keyword("AUTOLOAD"));
}

#[test]
fn destroy_only_in_keywords() {
    assert!(is_keyword("DESTROY"));
    assert!(!is_lexer_keyword("DESTROY"));
    assert!(!is_dap_completion_keyword("DESTROY"));
    assert!(!is_lsp_runtime_completion_keyword("DESTROY"));
    assert!(!is_rename_keyword("DESTROY"));
    assert!(!is_parser_lsp_keyword("DESTROY"));
}

// ---------------------------------------------------------------------------
// Unicode and non-ASCII edge cases
// ---------------------------------------------------------------------------

#[test]
fn unicode_lookalikes_are_not_keywords() {
    // Cyrillic 'а' (U+0430) looks like Latin 'a' but is distinct.
    assert!(!is_keyword("аnd")); // first char is Cyrillic а
    assert!(!is_keyword("іf")); // Cyrillic і
}

#[test]
fn emoji_is_not_a_keyword() {
    assert!(!is_keyword("🦀"));
    assert!(!is_keyword("my🦀"));
}

#[test]
fn null_byte_string_is_not_a_keyword() {
    assert!(!is_keyword("\0"));
    assert!(!is_keyword("my\0"));
}

// ---------------------------------------------------------------------------
// Perl sigils and special variables are NOT keywords
// ---------------------------------------------------------------------------

#[test]
fn sigil_prefixed_keywords_are_not_keywords() {
    assert!(!is_keyword("$my"));
    assert!(!is_keyword("@push"));
    assert!(!is_keyword("%keys"));
    assert!(!is_keyword("&sub"));
    assert!(!is_keyword("*open"));
}

#[test]
fn special_perl_variables_are_not_keywords() {
    for v in ["$_", "$!", "$@", "$&", "$0", "@_", "@ARGV", "%ENV", "$^W"] {
        assert!(!is_keyword(v), "{v:?} should not be a keyword");
    }
}

// ---------------------------------------------------------------------------
// Operator-like strings are NOT keywords
// ---------------------------------------------------------------------------

#[test]
fn perl_operators_are_not_keywords() {
    for op in [
        "=~", "!~", "->", "=>", "::", "&&", "||", "//", "**", "..", "...", "~~", "<=>", "<>",
    ] {
        assert!(!is_keyword(op), "operator {op:?} should not be a keyword");
    }
}

// ---------------------------------------------------------------------------
// Numeric and punctuation strings are NOT keywords
// ---------------------------------------------------------------------------

#[test]
fn numeric_strings_are_not_keywords() {
    for n in ["0", "1", "42", "3.14", "-1", "0x1F", "0b101"] {
        assert!(!is_keyword(n), "numeric {n:?} should not be a keyword");
    }
}

#[test]
fn punctuation_strings_are_not_keywords() {
    for p in [";", ",", "(", ")", "{", "}", "[", "]", ".", "\\"] {
        assert!(!is_keyword(p), "punctuation {p:?} should not be a keyword");
    }
}

// ---------------------------------------------------------------------------
// DAP-specific exclusion tests
// ---------------------------------------------------------------------------

#[test]
fn phase_blocks_not_in_dap_keywords() {
    for kw in ["BEGIN", "CHECK", "INIT", "END", "UNITCHECK"] {
        assert!(
            !is_dap_completion_keyword(kw),
            "phase block {kw:?} should not be in DAP_COMPLETION_KEYWORDS"
        );
    }
}

#[test]
fn modern_perl_not_in_dap_keywords() {
    for kw in ["try", "catch", "finally", "class", "method"] {
        assert!(
            !is_dap_completion_keyword(kw),
            "{kw:?} should not be in DAP_COMPLETION_KEYWORDS"
        );
    }
}

#[test]
fn logical_operators_not_in_dap_keywords() {
    for kw in ["and", "or", "not", "xor"] {
        assert!(
            !is_dap_completion_keyword(kw),
            "{kw:?} should not be in DAP_COMPLETION_KEYWORDS"
        );
    }
}

// ---------------------------------------------------------------------------
// Rename-specific exclusion tests
// ---------------------------------------------------------------------------

#[test]
fn io_builtins_not_in_rename_keywords() {
    for kw in ["open", "close", "read", "print", "printf", "say", "write"] {
        assert!(
            !is_rename_keyword(kw),
            "{kw:?} should not be in RENAME_KEYWORDS"
        );
    }
}

#[test]
fn comparison_operators_not_in_rename_keywords() {
    for kw in ["cmp", "ge", "gt", "le", "lt"] {
        assert!(
            !is_rename_keyword(kw),
            "{kw:?} should not be in RENAME_KEYWORDS"
        );
    }
}

// ---------------------------------------------------------------------------
// Runtime completion specific exclusion tests
// ---------------------------------------------------------------------------

#[test]
fn comparison_operators_not_in_runtime_completion() {
    for kw in ["cmp", "eq", "ge", "gt", "le", "lt", "ne"] {
        assert!(
            !is_lsp_runtime_completion_keyword(kw),
            "{kw:?} should not be in LSP_RUNTIME_COMPLETION_KEYWORDS"
        );
    }
}

#[test]
fn dunder_tokens_not_in_runtime_completion() {
    for kw in ["__FILE__", "__LINE__", "__PACKAGE__"] {
        assert!(
            !is_lsp_runtime_completion_keyword(kw),
            "{kw:?} should not be in LSP_RUNTIME_COMPLETION_KEYWORDS"
        );
    }
}

// ---------------------------------------------------------------------------
// Parser LSP specific exclusion tests
// ---------------------------------------------------------------------------

#[test]
fn phase_blocks_not_in_parser_lsp() {
    for kw in ["BEGIN", "CHECK", "INIT", "END", "UNITCHECK"] {
        assert!(
            !is_parser_lsp_keyword(kw),
            "{kw:?} should not be in PARSER_LSP_KEYWORDS"
        );
    }
}

#[test]
fn string_builtins_not_in_parser_lsp() {
    for kw in [
        "chomp", "chop", "chr", "hex", "index", "lc", "length", "oct", "ord", "substr",
    ] {
        assert!(
            !is_parser_lsp_keyword(kw),
            "{kw:?} should not be in PARSER_LSP_KEYWORDS"
        );
    }
}

// ---------------------------------------------------------------------------
// Lexer keyword specific exclusion tests
// ---------------------------------------------------------------------------

#[test]
fn comparison_operators_not_in_lexer_keywords() {
    for kw in ["eq", "ne", "ge", "gt", "le", "lt"] {
        assert!(
            !is_lexer_keyword(kw),
            "{kw:?} should not be in LEXER_KEYWORDS"
        );
    }
}

#[test]
fn io_builtins_not_in_lexer_keywords() {
    for kw in ["open", "close", "read", "write"] {
        assert!(
            !is_lexer_keyword(kw),
            "{kw:?} should not be in LEXER_KEYWORDS"
        );
    }
}

// ---------------------------------------------------------------------------
// Every specialized keyword list is a proper subset of KEYWORDS
// ---------------------------------------------------------------------------

#[test]
fn specialized_lists_are_proper_subsets() {
    // Proper subset means strictly smaller AND fully contained.
    let lists: &[(&str, &[&str])] = &[
        ("LSP_COMPLETION_KEYWORDS", LSP_COMPLETION_KEYWORDS),
        ("DAP_COMPLETION_KEYWORDS", DAP_COMPLETION_KEYWORDS),
        (
            "LSP_RUNTIME_COMPLETION_KEYWORDS",
            LSP_RUNTIME_COMPLETION_KEYWORDS,
        ),
        ("RENAME_KEYWORDS", RENAME_KEYWORDS),
        ("PARSER_LSP_KEYWORDS", PARSER_LSP_KEYWORDS),
        ("LEXER_KEYWORDS", LEXER_KEYWORDS),
    ];
    for &(name, list) in lists {
        assert!(
            list.len() < KEYWORDS.len(),
            "{name} is not a proper subset (same size as KEYWORDS)"
        );
        for &kw in list {
            assert!(
                KEYWORDS.binary_search(&kw).is_ok(),
                "{name} entry {kw:?} not found in KEYWORDS"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Keyword count stability (regression guards)
// ---------------------------------------------------------------------------

#[test]
fn keywords_count_is_at_least_120() {
    assert!(
        KEYWORDS.len() >= 120,
        "KEYWORDS should have at least 120 entries, found {}",
        KEYWORDS.len()
    );
}

#[test]
fn lsp_completion_keywords_count_at_least_40() {
    assert!(
        LSP_COMPLETION_KEYWORDS.len() >= 40,
        "LSP_COMPLETION_KEYWORDS should have at least 40 entries, found {}",
        LSP_COMPLETION_KEYWORDS.len()
    );
}

#[test]
fn dap_completion_keywords_count_at_least_60() {
    assert!(
        DAP_COMPLETION_KEYWORDS.len() >= 60,
        "DAP_COMPLETION_KEYWORDS should have at least 60 entries, found {}",
        DAP_COMPLETION_KEYWORDS.len()
    );
}

#[test]
fn lexer_keywords_count_at_least_50() {
    assert!(
        LEXER_KEYWORDS.len() >= 50,
        "LEXER_KEYWORDS should have at least 50 entries, found {}",
        LEXER_KEYWORDS.len()
    );
}

#[test]
fn rename_keywords_count_at_least_20() {
    assert!(
        RENAME_KEYWORDS.len() >= 20,
        "RENAME_KEYWORDS should have at least 20 entries, found {}",
        RENAME_KEYWORDS.len()
    );
}

// ---------------------------------------------------------------------------
// All specialized lists are pairwise distinct (not identical)
// ---------------------------------------------------------------------------

#[test]
fn specialized_lists_differ_from_each_other() {
    let lists: &[(&str, &[&str])] = &[
        ("LSP_COMPLETION", LSP_COMPLETION_KEYWORDS),
        ("DAP_COMPLETION", DAP_COMPLETION_KEYWORDS),
        ("LSP_RUNTIME", LSP_RUNTIME_COMPLETION_KEYWORDS),
        ("RENAME", RENAME_KEYWORDS),
        ("PARSER_LSP", PARSER_LSP_KEYWORDS),
        ("LEXER", LEXER_KEYWORDS),
    ];
    for i in 0..lists.len() {
        for j in (i + 1)..lists.len() {
            let (name_a, list_a) = lists[i];
            let (name_b, list_b) = lists[j];
            assert!(
                list_a != list_b,
                "{name_a} and {name_b} should not be identical"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// All keyword entries are valid Perl identifiers or special tokens
// ---------------------------------------------------------------------------

#[test]
fn all_keywords_are_valid_perl_identifiers_or_special_tokens() {
    for &kw in KEYWORDS {
        // Special tokens start with __
        if kw.starts_with("__") {
            assert!(
                kw.ends_with("__"),
                "dunder keyword {kw:?} should end with __"
            );
            continue;
        }
        // Single-char operators: m, s, q, y
        if kw.len() == 1 {
            assert!(
                kw.chars().all(|c| c.is_ascii_lowercase()),
                "single-char keyword {kw:?} should be lowercase ASCII"
            );
            continue;
        }
        // Regular keywords: only ASCII alphanumeric + underscore
        assert!(
            kw.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "keyword {kw:?} contains unexpected characters"
        );
    }
}

// ---------------------------------------------------------------------------
// Keyword casing convention: phase blocks are UPPER, rest mostly lower
// ---------------------------------------------------------------------------

#[test]
fn phase_blocks_are_all_uppercase() {
    for kw in [
        "BEGIN",
        "CHECK",
        "END",
        "INIT",
        "UNITCHECK",
        "AUTOLOAD",
        "DESTROY",
    ] {
        if is_keyword(kw) {
            assert!(
                kw.chars().all(|c| c.is_ascii_uppercase()),
                "phase block {kw:?} should be all uppercase"
            );
        }
    }
}

#[test]
fn non_special_keywords_are_lowercase() {
    for &kw in KEYWORDS {
        if kw.starts_with("__") || kw.chars().all(|c| c.is_ascii_uppercase()) {
            continue; // skip dunder and uppercase phase blocks
        }
        assert!(
            kw.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "regular keyword {kw:?} should be all lowercase"
        );
    }
}

// ---------------------------------------------------------------------------
// Cross-function consistency: is_* returns false for tokens NOT in its list
// ---------------------------------------------------------------------------

#[test]
fn is_keyword_returns_false_for_non_keyword() {
    let non_keywords = ["foo", "bar", "baz", "quux", "Perl", "PERL", "perl6", "raku"];
    for s in non_keywords {
        assert!(!is_keyword(s));
        assert!(!is_lexer_keyword(s));
        assert!(!is_lsp_completion_keyword(s));
        assert!(!is_dap_completion_keyword(s));
        assert!(!is_lsp_runtime_completion_keyword(s));
        assert!(!is_rename_keyword(s));
        assert!(!is_parser_lsp_keyword(s));
    }
}

// ---------------------------------------------------------------------------
// Specific cross-list membership tests for common keywords
// ---------------------------------------------------------------------------

#[test]
fn while_is_in_all_specialized_lists() {
    assert!(is_keyword("while"));
    assert!(is_lsp_completion_keyword("while"));
    assert!(is_dap_completion_keyword("while"));
    assert!(is_lsp_runtime_completion_keyword("while"));
    assert!(is_rename_keyword("while"));
    assert!(is_parser_lsp_keyword("while"));
    assert!(is_lexer_keyword("while"));
}

#[test]
fn foreach_is_in_all_specialized_lists() {
    assert!(is_keyword("foreach"));
    assert!(is_lsp_completion_keyword("foreach"));
    assert!(is_dap_completion_keyword("foreach"));
    assert!(is_lsp_runtime_completion_keyword("foreach"));
    assert!(is_rename_keyword("foreach"));
    assert!(is_parser_lsp_keyword("foreach"));
    assert!(is_lexer_keyword("foreach"));
}

#[test]
fn package_is_in_all_specialized_lists() {
    assert!(is_keyword("package"));
    assert!(is_lsp_completion_keyword("package"));
    assert!(is_dap_completion_keyword("package"));
    assert!(is_lsp_runtime_completion_keyword("package"));
    assert!(is_rename_keyword("package"));
    assert!(is_parser_lsp_keyword("package"));
    assert!(is_lexer_keyword("package"));
}

#[test]
fn eval_in_keywords_and_selected_lists() {
    assert!(is_keyword("eval"));
    assert!(is_lsp_completion_keyword("eval"));
    assert!(is_dap_completion_keyword("eval"));
    assert!(is_lexer_keyword("eval"));
    assert!(is_parser_lsp_keyword("eval"));
}

#[test]
fn die_in_keywords_and_selected_lists() {
    assert!(is_keyword("die"));
    assert!(is_lsp_completion_keyword("die"));
    assert!(is_dap_completion_keyword("die"));
    assert!(is_lsp_runtime_completion_keyword("die"));
    assert!(is_parser_lsp_keyword("die"));
    assert!(is_lexer_keyword("die"));
}

#[test]
fn warn_in_keywords_and_selected_lists() {
    assert!(is_keyword("warn"));
    assert!(is_lsp_completion_keyword("warn"));
    assert!(is_dap_completion_keyword("warn"));
    assert!(is_lsp_runtime_completion_keyword("warn"));
    assert!(is_parser_lsp_keyword("warn"));
    assert!(is_lexer_keyword("warn"));
}

// ---------------------------------------------------------------------------
// Very long strings are not keywords
// ---------------------------------------------------------------------------

#[test]
fn very_long_string_is_not_a_keyword() {
    let long = "a".repeat(10_000);
    assert!(!is_keyword(&long));
}

#[test]
fn keyword_prefix_repeated_is_not_a_keyword() {
    assert!(!is_keyword("mymy"));
    assert!(!is_keyword("subsub"));
    assert!(!is_keyword("ifif"));
}

// ---------------------------------------------------------------------------
// All ASCII lowercase letters as single-char inputs
// ---------------------------------------------------------------------------

#[test]
fn single_lowercase_letters_keyword_status() {
    let expected_single_char_keywords = ['m', 'q', 's', 'y'];
    for c in b'a'..=b'z' {
        let buf = [c];
        let s = match std::str::from_utf8(&buf) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if expected_single_char_keywords.contains(&(c as char)) {
            assert!(is_keyword(s), "'{s}' should be a keyword");
        } else {
            assert!(!is_keyword(s), "'{s}' should NOT be a keyword");
        }
    }
}
