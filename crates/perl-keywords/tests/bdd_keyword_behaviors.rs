//! BDD-style acceptance tests for `perl-keywords`.
//!
//! These scenarios focus on end-user behavior: when tooling asks whether a
//! token belongs to a keyword bucket, helper functions should answer
//! consistently and predictably.

use perl_keywords::{
    DAP_COMPLETION_KEYWORDS, KEYWORDS, LEXER_KEYWORDS, LSP_COMPLETION_KEYWORDS,
    LSP_RUNTIME_COMPLETION_KEYWORDS, PARSER_LSP_KEYWORDS, RENAME_KEYWORDS,
    is_dap_completion_keyword, is_keyword, is_lexer_keyword, is_lsp_completion_keyword,
    is_lsp_runtime_completion_keyword, is_parser_lsp_keyword, is_rename_keyword,
};

#[test]
fn scenario_keyword_lookup_for_core_control_flow_tokens() {
    // Given a list of canonical control-flow tokens used in editor workflows.
    let control_flow_tokens = ["if", "elsif", "else", "while", "until", "foreach"];

    // When each token is checked against all keyword buckets.
    // Then each control-flow token is in KEYWORDS and parser/LSP-aware buckets.
    for token in control_flow_tokens {
        assert!(is_keyword(token), "{token} should be recognized as a keyword");
        assert!(is_lsp_completion_keyword(token), "{token} should be in LSP completion");
        assert!(
            is_lsp_runtime_completion_keyword(token),
            "{token} should be in runtime completion"
        );
        assert!(is_parser_lsp_keyword(token), "{token} should be in parser LSP set");
        assert!(is_lexer_keyword(token), "{token} should be lexer-recognized");
    }
}

#[test]
fn scenario_rename_validation_rejects_non_reserved_identifiers() {
    // Given user-defined names that are common in Perl code.
    let identifiers = ["handler", "result", "strict", "warnings", "new_value"];

    // When rename validation checks the reserved-word set.
    // Then these identifiers are not blocked as reserved keywords.
    for token in identifiers {
        assert!(!is_rename_keyword(token), "{token} should not be reserved for rename");
    }
}

#[test]
fn scenario_lexer_only_modern_keywords_are_not_exposed_in_completion() {
    // Given modern Perl language additions recognized by the lexer.
    let modern_tokens = ["class", "field", "method", "try", "catch", "finally"];

    // When completion buckets are checked.
    // Then they remain excluded from completion and rename-specific lists.
    for token in modern_tokens {
        assert!(is_keyword(token), "{token} should remain in canonical keywords");
        assert!(is_lexer_keyword(token), "{token} should remain in lexer keywords");
        assert!(!is_lsp_completion_keyword(token), "{token} should not be in LSP completion");
        assert!(!is_dap_completion_keyword(token), "{token} should not be in DAP completion");
        assert!(
            !is_lsp_runtime_completion_keyword(token),
            "{token} should not be in runtime completion"
        );
        assert!(!is_rename_keyword(token), "{token} should not be in rename keywords");
    }
}

#[test]
fn scenario_special_tokens_are_case_sensitive() {
    // Given dunder and uppercase lifecycle tokens.
    let exact_tokens = ["__FILE__", "__LINE__", "BEGIN", "UNITCHECK"];

    // When exact and lowercased forms are checked.
    // Then only the canonical case is accepted.
    for token in exact_tokens {
        assert!(is_keyword(token), "{token} should be recognized in canonical case");
        let lower = token.to_ascii_lowercase();
        assert!(!is_keyword(&lower), "{lower} should not be recognized in lowercase");
    }
}

#[test]
fn scenario_every_specialized_bucket_stays_subset_of_keywords() {
    // Given all specialized keyword buckets used by downstream tooling.
    let buckets: [(&str, &[&str]); 6] = [
        ("LSP_COMPLETION_KEYWORDS", LSP_COMPLETION_KEYWORDS),
        ("DAP_COMPLETION_KEYWORDS", DAP_COMPLETION_KEYWORDS),
        ("LSP_RUNTIME_COMPLETION_KEYWORDS", LSP_RUNTIME_COMPLETION_KEYWORDS),
        ("RENAME_KEYWORDS", RENAME_KEYWORDS),
        ("PARSER_LSP_KEYWORDS", PARSER_LSP_KEYWORDS),
        ("LEXER_KEYWORDS", LEXER_KEYWORDS),
    ];

    // When membership is validated against canonical KEYWORDS.
    // Then every specialized entry is still part of the canonical set.
    for (name, bucket) in buckets {
        for token in bucket {
            assert!(is_keyword(token), "{name} entry {token} should exist in KEYWORDS");
        }
    }
}

#[test]
fn scenario_binary_search_helpers_reject_near_miss_tokens() {
    // Given lookalike tokens that are close to real keywords.
    let near_misses = ["fo", "forr", "whille", "unles", "retun", "packaeg"];

    // When keyword helpers evaluate those tokens.
    // Then all helpers reject them consistently.
    for token in near_misses {
        assert!(!is_keyword(token), "{token} should not be recognized as KEYWORDS member");
        assert!(!is_lexer_keyword(token), "{token} should not be recognized as LEXER member");
        assert!(
            !is_lsp_completion_keyword(token),
            "{token} should not be recognized as LSP member"
        );
        assert!(
            !is_dap_completion_keyword(token),
            "{token} should not be recognized as DAP member"
        );
        assert!(
            !is_lsp_runtime_completion_keyword(token),
            "{token} should not be recognized as runtime completion member"
        );
        assert!(!is_rename_keyword(token), "{token} should not be recognized as rename member");
        assert!(
            !is_parser_lsp_keyword(token),
            "{token} should not be recognized as parser LSP member"
        );
    }
}

#[test]
fn scenario_public_keyword_inventory_supports_expected_floor() {
    // Given the public KEYWORDS constant for downstream consumers.
    // When we sanity-check cardinality.
    // Then the crate still ships a substantial canonical inventory.
    assert!(KEYWORDS.len() >= 120, "expected at least 120 canonical keywords");
}
