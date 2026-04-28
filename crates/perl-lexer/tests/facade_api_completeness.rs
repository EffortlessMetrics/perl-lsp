//! Guards the public API surface contributed by Wave C absorption. If an item
//! listed here becomes inaccessible at the documented path, this test fails --
//! catching accidental API breakage during future refactoring.
//!
//! Note: `TokenStream` is NOT re-exported from `perl_lexer` because it lives
//! in `perl_parser_core::tokens::token_stream` (depends on `perl-error`, which
//! would create a dependency cycle with `perl-lexer`). The smoke test lives
//! alongside the trivia modules there.

#[allow(deprecated)]
use perl_lexer::{
    // builtins
    BUILTIN_FULL_SIGS,
    BUILTIN_SIGS,
    BuiltinSignature,
    // keywords
    DAP_COMPLETION_KEYWORDS,
    KEYWORDS,
    LEXER_KEYWORDS,
    LSP_COMPLETION_KEYWORDS,
    LSP_RUNTIME_COMPLETION_KEYWORDS,
    PARSER_LSP_KEYWORDS,
    // tokenizer (AST-agnostic slice)
    PositionTracker,
    RENAME_KEYWORDS,
    TokenWithPosition,
    builtin_count,
    code_slice,
    create_builtin_signatures,
    find_data_marker_byte,
    find_data_marker_byte_lexed,
    get_param_names,
    is_builtin,
    is_dap_completion_keyword,
    is_keyword,
    is_lexer_keyword,
    is_lsp_completion_keyword,
    is_lsp_runtime_completion_keyword,
    is_parser_lsp_keyword,
    is_rename_keyword,
};

#[test]
fn keywords_accessible_at_crate_root() {
    assert!(!KEYWORDS.is_empty());
    assert!(is_keyword("my"));
    assert!(is_lexer_keyword("sub"));
    let _ = is_lsp_completion_keyword("use");
    let _ = is_dap_completion_keyword("eval");
    let _ = is_lsp_runtime_completion_keyword("print");
    let _ = is_rename_keyword("my");
    let _ = is_parser_lsp_keyword("package");
    let _ = LSP_COMPLETION_KEYWORDS;
    let _ = DAP_COMPLETION_KEYWORDS;
    let _ = LSP_RUNTIME_COMPLETION_KEYWORDS;
    let _ = RENAME_KEYWORDS;
    let _ = PARSER_LSP_KEYWORDS;
    let _ = LEXER_KEYWORDS;
}

#[test]
fn builtins_accessible_at_crate_root() {
    assert!(is_builtin("print"));
    assert!(builtin_count() > 0);
    let _: &[&str] = get_param_names("substr");
    let _ = &BUILTIN_SIGS;
    let _ = &BUILTIN_FULL_SIGS;
    let sigs = create_builtin_signatures();
    let _: Option<&BuiltinSignature> = sigs.get("print");
}

#[test]
fn tokenizer_accessible_at_crate_root() {
    let _ = code_slice("print 1;\n__DATA__\nstuff");
    let _ = find_data_marker_byte_lexed("print 1;\n__DATA__\nstuff");
    #[allow(deprecated)]
    let _: Option<usize> = find_data_marker_byte("print 1;\n");
    // Type-only smoke-checks
    let _: Option<TokenWithPosition> = None;
    let _: Option<PositionTracker<'_>> = None;
}
