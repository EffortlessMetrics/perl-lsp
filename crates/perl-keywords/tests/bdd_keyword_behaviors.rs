//! BDD-style behavioral scenarios for `perl-keywords`.
//!
//! These tests intentionally use a Given/When/Then structure so future
//! contributors can extend coverage from a product-behavior perspective.

use perl_keywords::{
    is_dap_completion_keyword, is_keyword, is_lexer_keyword, is_lsp_completion_keyword,
    is_lsp_runtime_completion_keyword, is_parser_lsp_keyword, is_rename_keyword,
};

struct Scenario<'a> {
    name: &'a str,
    token: &'a str,
    expected: ExpectedMembership,
}

#[derive(Clone, Copy)]
struct ExpectedMembership {
    keyword: bool,
    lexer: bool,
    lsp_completion: bool,
    dap_completion: bool,
    lsp_runtime: bool,
    rename: bool,
    parser_lsp: bool,
}

fn then_keyword_membership_matches(token: &str, expected: ExpectedMembership) {
    assert_eq!(is_keyword(token), expected.keyword, "KEYWORDS mismatch for {token}");
    assert_eq!(is_lexer_keyword(token), expected.lexer, "LEXER_KEYWORDS mismatch for {token}");
    assert_eq!(
        is_lsp_completion_keyword(token),
        expected.lsp_completion,
        "LSP_COMPLETION_KEYWORDS mismatch for {token}"
    );
    assert_eq!(
        is_dap_completion_keyword(token),
        expected.dap_completion,
        "DAP_COMPLETION_KEYWORDS mismatch for {token}"
    );
    assert_eq!(
        is_lsp_runtime_completion_keyword(token),
        expected.lsp_runtime,
        "LSP_RUNTIME_COMPLETION_KEYWORDS mismatch for {token}"
    );
    assert_eq!(is_rename_keyword(token), expected.rename, "RENAME_KEYWORDS mismatch for {token}");
    assert_eq!(
        is_parser_lsp_keyword(token),
        expected.parser_lsp,
        "PARSER_LSP_KEYWORDS mismatch for {token}"
    );
}

#[test]
fn scenario_declaration_keyword_is_reserved_everywhere_expected() {
    // Given a canonical declaration keyword used by multiple editor features.
    let token = "my";

    // When the token is classified by each helper.
    let expected = ExpectedMembership {
        keyword: true,
        lexer: true,
        lsp_completion: true,
        dap_completion: true,
        lsp_runtime: true,
        rename: true,
        parser_lsp: true,
    };

    // Then every relevant bucket returns membership.
    then_keyword_membership_matches(token, expected);
}

#[test]
fn scenario_modern_object_keyword_is_lexer_only() {
    // Given a modern Perl object keyword.
    let token = "field";

    // When the token is classified for all product surfaces.
    let expected = ExpectedMembership {
        keyword: true,
        lexer: true,
        lsp_completion: false,
        dap_completion: false,
        lsp_runtime: false,
        rename: false,
        parser_lsp: false,
    };

    // Then it is recognized only for canonical + lexer classification.
    then_keyword_membership_matches(token, expected);
}

#[test]
fn scenario_non_keyword_identifier_is_rejected_everywhere() {
    // Given a realistic Perl identifier that is not reserved.
    let token = "strict";

    // When the token is classified by all helpers.
    let expected = ExpectedMembership {
        keyword: false,
        lexer: false,
        lsp_completion: false,
        dap_completion: false,
        lsp_runtime: false,
        rename: false,
        parser_lsp: false,
    };

    // Then every lookup path rejects the token.
    then_keyword_membership_matches(token, expected);
}

#[test]
fn scenario_case_variants_do_not_match_lowercase_keyword() {
    // Given a lowercase control-flow keyword and its case variants.
    let scenarios = [
        Scenario {
            name: "lowercase token",
            token: "while",
            expected: ExpectedMembership {
                keyword: true,
                lexer: true,
                lsp_completion: true,
                dap_completion: true,
                lsp_runtime: true,
                rename: true,
                parser_lsp: true,
            },
        },
        Scenario {
            name: "capitalized variant",
            token: "While",
            expected: ExpectedMembership {
                keyword: false,
                lexer: false,
                lsp_completion: false,
                dap_completion: false,
                lsp_runtime: false,
                rename: false,
                parser_lsp: false,
            },
        },
        Scenario {
            name: "uppercase variant",
            token: "WHILE",
            expected: ExpectedMembership {
                keyword: false,
                lexer: false,
                lsp_completion: false,
                dap_completion: false,
                lsp_runtime: false,
                rename: false,
                parser_lsp: false,
            },
        },
    ];

    // When each variant is classified.
    for scenario in scenarios {
        // Then only the canonical case matches.
        then_keyword_membership_matches(scenario.token, scenario.expected);
        assert!(!scenario.name.is_empty(), "scenario should have a readable name");
    }
}
