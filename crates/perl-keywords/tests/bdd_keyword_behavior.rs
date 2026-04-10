//! Behavior-driven integration tests for `perl-keywords`.
//!
//! The scenarios in this file are intentionally written in
//! Given/When/Then style to document expected behavior at callsites.

use perl_keywords::{
    is_dap_completion_keyword, is_keyword, is_lexer_keyword, is_lsp_completion_keyword,
    is_lsp_runtime_completion_keyword, is_parser_lsp_keyword, is_rename_keyword,
};

#[test]
fn scenario_editor_completion_and_rename_for_declaration_keyword() {
    // Given a declaration keyword used in normal Perl source
    let token = "my";

    // When all keyword classifiers are consulted
    let in_canonical = is_keyword(token);
    let in_lsp_completion = is_lsp_completion_keyword(token);
    let in_runtime_completion = is_lsp_runtime_completion_keyword(token);
    let in_rename_guard = is_rename_keyword(token);
    let in_parser_lsp = is_parser_lsp_keyword(token);
    let in_lexer = is_lexer_keyword(token);

    // Then declaration keywords are accepted across editor-facing paths
    assert!(in_canonical);
    assert!(in_lsp_completion);
    assert!(in_runtime_completion);
    assert!(in_rename_guard);
    assert!(in_parser_lsp);
    assert!(in_lexer);

    // And debug-console completions also include declaration keywords
    assert!(is_dap_completion_keyword(token));
}

#[test]
fn scenario_modern_object_keyword_is_lexer_only() {
    // Given a modern Perl object-system keyword
    let token = "method";

    // When completion and rename classifiers are consulted
    let in_completion_paths = is_lsp_completion_keyword(token)
        || is_dap_completion_keyword(token)
        || is_lsp_runtime_completion_keyword(token);

    // Then it is recognized as a canonical/lexer keyword
    assert!(is_keyword(token));
    assert!(is_lexer_keyword(token));

    // And it is intentionally excluded from current completion and rename sets
    assert!(!in_completion_paths);
    assert!(!is_rename_keyword(token));
    assert!(!is_parser_lsp_keyword(token));
}

#[test]
fn scenario_dunder_token_support_is_deliberately_narrow() {
    // Given a Perl dunder token
    let token = "__PACKAGE__";

    // When every bucket is checked
    // Then canonical lookup and LSP keyword completion should accept it
    assert!(is_keyword(token));
    assert!(is_lsp_completion_keyword(token));

    // And context-specific buckets should reject it
    assert!(!is_dap_completion_keyword(token));
    assert!(!is_lsp_runtime_completion_keyword(token));
    assert!(!is_rename_keyword(token));
    assert!(!is_parser_lsp_keyword(token));
    assert!(!is_lexer_keyword(token));
}

#[test]
fn scenario_non_keyword_identifier_is_rejected_everywhere() {
    // Given an identifier that may appear in Perl code but is not reserved
    let token = "strict";

    // When all classifiers are queried
    // Then the token is rejected by every keyword bucket
    assert!(!is_keyword(token));
    assert!(!is_lsp_completion_keyword(token));
    assert!(!is_dap_completion_keyword(token));
    assert!(!is_lsp_runtime_completion_keyword(token));
    assert!(!is_rename_keyword(token));
    assert!(!is_parser_lsp_keyword(token));
    assert!(!is_lexer_keyword(token));
}

#[test]
fn scenario_keyword_matching_is_case_sensitive() {
    // Given an uppercase control-flow token that differs only by case
    let token = "WHILE";

    // When lookup helpers are called
    // Then lowercase keyword entries do not match uppercase variants
    assert!(!is_keyword(token));
    assert!(!is_lsp_completion_keyword(token));
    assert!(!is_dap_completion_keyword(token));
    assert!(!is_lsp_runtime_completion_keyword(token));
    assert!(!is_rename_keyword(token));
    assert!(!is_parser_lsp_keyword(token));

    // And the canonical lowercase token remains valid
    assert!(is_keyword("while"));
    assert!(is_lsp_completion_keyword("while"));
    assert!(is_dap_completion_keyword("while"));
    assert!(is_lsp_runtime_completion_keyword("while"));
    assert!(is_rename_keyword("while"));
    assert!(is_parser_lsp_keyword("while"));
    assert!(is_lexer_keyword("while"));
}

#[test]
fn scenario_operator_like_token_is_not_treated_as_keyword() {
    // Given an operator token often adjacent to keyword-like code
    let token = "=>";

    // When keyword classifiers are applied
    // Then operator syntax is never classified as a keyword
    assert!(!is_keyword(token));
    assert!(!is_lsp_completion_keyword(token));
    assert!(!is_dap_completion_keyword(token));
    assert!(!is_lsp_runtime_completion_keyword(token));
    assert!(!is_rename_keyword(token));
    assert!(!is_parser_lsp_keyword(token));
    assert!(!is_lexer_keyword(token));
}
