//! BDD-style coverage for the `qw` keyword classification edge.
//!
//! The existing suite already covers the broader inventory and most case/
//! subset relationships. This test keeps the one scenario that is easy to
//! miss in higher-level summaries: `qw` is a canonical keyword that also
//! participates in lexer and DAP completion buckets, while staying excluded
//! from the other specialized lookup paths.

use perl_lexer::{
    is_dap_completion_keyword, is_keyword, is_lexer_keyword, is_lsp_completion_keyword,
    is_lsp_runtime_completion_keyword, is_parser_lsp_keyword, is_rename_keyword,
};

#[test]
fn bdd_given_qw_when_classified_then_membership_matches_editor_contract() {
    let token = "qw";

    assert!(is_keyword(token), "qw should be a canonical keyword");
    assert!(
        is_lexer_keyword(token),
        "qw should be recognized by the lexer"
    );
    assert!(
        is_dap_completion_keyword(token),
        "qw should appear in DAP completion keywords"
    );

    assert!(
        !is_lsp_completion_keyword(token),
        "qw should not be in LSP completion keywords"
    );
    assert!(
        !is_lsp_runtime_completion_keyword(token),
        "qw should not be in runtime completion keywords"
    );
    assert!(
        !is_rename_keyword(token),
        "qw should not be in rename keywords"
    );
    assert!(
        !is_parser_lsp_keyword(token),
        "qw should not be in parser LSP keywords"
    );
}
