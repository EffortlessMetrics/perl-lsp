//! LSP request parameter extraction helpers.
//!
//! This crate centralizes common JSON parameter extraction patterns used by
//! request handlers, keeping transport/protocol concerns separate from
//! request-shape validation.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use perl_lsp_protocol::{JsonRpcError, invalid_params};
use serde_json::Value;

/// Extract the required textDocument.uri from LSP request params.
///
/// Returns INVALID_PARAMS error if the URI is missing or not a string.
pub fn req_uri(params: &Value) -> Result<&str, JsonRpcError> {
    params
        .pointer("/textDocument/uri")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params("Missing required parameter: textDocument.uri"))
}

/// Extract the required position (line, character) from LSP request params.
///
/// Returns INVALID_PARAMS error if line or character are missing or overflow u32.
pub fn req_position(params: &Value) -> Result<(u32, u32), JsonRpcError> {
    let line_u64 = params
        .pointer("/position/line")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_params("Missing required parameter: position.line"))?;
    let line =
        u32::try_from(line_u64).map_err(|_| invalid_params("position.line exceeds u32::MAX"))?;

    let character_u64 = params
        .pointer("/position/character")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_params("Missing required parameter: position.character"))?;
    let character = u32::try_from(character_u64)
        .map_err(|_| invalid_params("position.character exceeds u32::MAX"))?;

    Ok((line, character))
}

/// Extract the required range from LSP request params.
///
/// Returns INVALID_PARAMS error if any range components are missing or overflow u32.
/// Returns ((start_line, start_char), (end_line, end_char)).
pub fn req_range(params: &Value) -> Result<((u32, u32), (u32, u32)), JsonRpcError> {
    let start_line_u64 = params
        .pointer("/range/start/line")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_params("Missing required parameter: range.start.line"))?;
    let start_line = u32::try_from(start_line_u64)
        .map_err(|_| invalid_params("range.start.line exceeds u32::MAX"))?;

    let start_char_u64 = params
        .pointer("/range/start/character")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_params("Missing required parameter: range.start.character"))?;
    let start_char = u32::try_from(start_char_u64)
        .map_err(|_| invalid_params("range.start.character exceeds u32::MAX"))?;

    let end_line_u64 = params
        .pointer("/range/end/line")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_params("Missing required parameter: range.end.line"))?;
    let end_line = u32::try_from(end_line_u64)
        .map_err(|_| invalid_params("range.end.line exceeds u32::MAX"))?;

    let end_char_u64 = params
        .pointer("/range/end/character")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_params("Missing required parameter: range.end.character"))?;
    let end_char = u32::try_from(end_char_u64)
        .map_err(|_| invalid_params("range.end.character exceeds u32::MAX"))?;

    Ok(((start_line, start_char), (end_line, end_char)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_uri_position_and_range() -> Result<(), JsonRpcError> {
        let params = json!({
            "textDocument": { "uri": "file:///workspace/lib.pm" },
            "position": { "line": 12, "character": 8 },
            "range": {
                "start": { "line": 1, "character": 2 },
                "end": { "line": 3, "character": 4 }
            }
        });

        let uri = req_uri(&params)?;
        let position = req_position(&params)?;
        let range = req_range(&params)?;

        assert_eq!(uri, "file:///workspace/lib.pm");
        assert_eq!(position, (12, 8));
        assert_eq!(range, ((1, 2), (3, 4)));
        Ok(())
    }

    #[test]
    fn returns_invalid_params_for_missing_values() {
        let params = json!({ "textDocument": {} });

        let error = req_uri(&params).expect_err("missing uri should error");
        assert_eq!(error.code, perl_lsp_protocol::INVALID_PARAMS);
    }
}
