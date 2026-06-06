//! AST explorer handler for the `perl/showAst` custom request.
//!
//! The VSCode extension's `perl-lsp.showParserAst` command sends this request
//! with `{ uri: "file:///path/to/file.pl" }` and expects either:
//!
//! - A JSON string containing the S-expression representation of the AST, or
//! - A JSON `null` value when the document has not been parsed yet (e.g. on
//!   `didOpen` before the async parse completes), or
//! - A JSON-RPC error when the document is not open in the server.
//!
//! # Response format
//!
//! The S-expression is produced by `perl_ast::Node::to_sexp()` and looks like:
//!
//! ```text
//! (source_file (use_statement ...) (subroutine name: foo ...))
//! ```
//!
//! The client displays this verbatim in a VSCode `OutputChannel`.

use super::super::*;

fn invalid_show_ast_uri_params() -> JsonRpcError {
    JsonRpcError {
        code: INVALID_PARAMS,
        message: "Missing required parameter: uri\n\n\
                  perl/showAst expects params.uri to identify an open Perl document.\n\n\
                  Example: {\"uri\":\"file:///workspace/lib/My/Module.pm\"}"
            .to_string(),
        data: None,
    }
}

fn show_ast_document_not_found(uri: &str) -> JsonRpcError {
    JsonRpcError {
        code: INVALID_PARAMS,
        message: format!(
            "Document is not open: {uri}\n\n\
             perl/showAst can only inspect documents opened with textDocument/didOpen. \
             Send textDocument/didOpen first or use the URI of an already-open document."
        ),
        data: None,
    }
}

impl LspServer {
    /// Handle the `perl/showAst` custom request.
    ///
    /// # Parameters (JSON object)
    /// - `uri` (**required**): The document URI string, e.g. `"file:///foo.pl"`.
    ///
    /// # Returns
    /// - `Ok(Some(Value::String(...)))` — the AST as an S-expression string.
    /// - `Ok(Some(Value::Null))` — document is open but has no AST yet.
    /// - `Err(INVALID_PARAMS)` — `uri` param is missing or document is not open.
    pub(super) fn handle_show_ast_dispatch(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        // Extract the `uri` string from params
        let uri = params
            .as_ref()
            .and_then(|p| p.get("uri"))
            .and_then(|u| u.as_str())
            .ok_or_else(invalid_show_ast_uri_params)?;

        let docs = self.documents_guard();

        match docs.get(uri) {
            Some(state) => {
                match &state.ast {
                    Some(ast) => {
                        // Serialize the AST to S-expression format
                        let sexp = ast.to_sexp();
                        Ok(Some(json!(sexp)))
                    }
                    None => {
                        // Document is open but has no AST yet (parse in progress,
                        // or parse failed entirely). Return null so the client can
                        // show "No AST available" gracefully.
                        Ok(Some(Value::Null))
                    }
                }
            }
            None => Err(show_ast_document_not_found(uri)),
        }
    }
}
