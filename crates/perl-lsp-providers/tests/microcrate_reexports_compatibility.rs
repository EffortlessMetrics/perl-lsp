use std::sync::Arc;

use perl_parser_core::{ParseError, Parser};

fn parse_minimal_ast(source: &str) -> Result<Arc<perl_parser_core::Node>, ParseError> {
    let mut parser = Parser::new(source);
    parser.parse().map(Arc::new)
}

#[test]
fn top_level_microcrate_reexports_are_usable() -> Result<(), ParseError> {
    let source = "my $x = 1;\n$x++;";
    let ast = parse_minimal_ast(source)?;

    let _completion_provider = perl_lsp_providers::completion::CompletionProvider::new(&ast);
    let _diagnostics_provider =
        perl_lsp_providers::diagnostics::DiagnosticsProvider::new(&ast, source.to_string());
    let _code_actions_provider =
        perl_lsp_providers::code_actions::CodeActionsProvider::new(source.to_string());
    let _inlay_hints_provider = perl_lsp_providers::inlay_hints::InlayHintsProvider::new();
    let _rename_provider =
        perl_lsp_providers::rename::RenameProvider::new(&ast, source.to_string());
    let _type_definition_provider = perl_lsp_providers::navigation::TypeDefinitionProvider::new();
    let _semantic_tokens_provider =
        perl_lsp_providers::semantic_tokens::SemanticTokensProvider::new();

    let opts = perl_lsp_providers::formatting::FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: Some(true),
        insert_final_newline: Some(true),
        trim_final_newlines: Some(true),
    };
    assert_eq!(opts.tab_size, 4);

    let subprocess_output = perl_lsp_providers::tooling::SubprocessOutput {
        stdout: b"ok".to_vec(),
        stderr: Vec::new(),
        status_code: 0,
    };
    assert!(subprocess_output.success());
    Ok(())
}

#[test]
#[allow(deprecated)]
fn legacy_lsp_compat_reexports_are_still_usable() -> Result<(), ParseError> {
    let source = "my $x = 1;\n$x++;";
    let ast = parse_minimal_ast(source)?;

    let _completion_provider =
        perl_lsp_providers::ide::lsp_compat::completion::CompletionProvider::new(&ast);
    let _diagnostics_provider =
        perl_lsp_providers::ide::lsp_compat::diagnostics::DiagnosticsProvider::new(
            &ast,
            source.to_string(),
        );
    let _code_actions_provider =
        perl_lsp_providers::ide::lsp_compat::code_actions::CodeActionsProvider::new(
            source.to_string(),
        );
    let _inlay_hints_provider =
        perl_lsp_providers::ide::lsp_compat::inlay_hints::InlayHintsProvider::new();
    let _rename_provider =
        perl_lsp_providers::ide::lsp_compat::rename::RenameProvider::new(&ast, source.to_string());

    let _range = perl_lsp_providers::ide::lsp_compat::formatting::FormatRange::new(
        perl_lsp_providers::ide::lsp_compat::formatting::FormatPosition::new(0, 0),
        perl_lsp_providers::ide::lsp_compat::formatting::FormatPosition::new(0, 5),
    );
    Ok(())
}

#[test]
#[allow(deprecated)]
fn deprecated_tooling_alias_still_resolves() {
    let err = perl_lsp_providers::tooling_export::SubprocessError::new("compat");
    assert_eq!(err.message, "compat");
}
