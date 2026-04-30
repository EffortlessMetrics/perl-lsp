use crate::runtime::input_validation::constants::{
    ALLOWED_COMMANDS, ALLOWED_TEXT_DOCUMENT_URI_SCHEMES, MAX_METHOD_LENGTH, MAX_PARAMS_SIZE,
    MAX_URI_LENGTH,
};
use crate::runtime::input_validation::file_validation::validate_file_content;
use anyhow::{Result, anyhow};
use std::path::Path;

/// Validates LSP request parameters to ensure they're safe.
pub fn validate_lsp_request(method: &str, params: &serde_json::Value) -> Result<()> {
    if method.len() > MAX_METHOD_LENGTH
        || !method
            .chars()
            .all(|character| character.is_alphanumeric() || character == '/' || character == '$')
    {
        return Err(anyhow!("Invalid LSP method: {}", method));
    }

    let params_str = serde_json::to_string(params)?;
    if params_str.len() > MAX_PARAMS_SIZE {
        return Err(anyhow!("LSP parameters too large for method: {}", method));
    }

    match method {
        "textDocument/didOpen" | "textDocument/didChange" | "textDocument/didSave" => {
            validate_text_document_params(params)?;
        }
        "workspace/executeCommand" => {
            validate_execute_command_params(params)?;
        }
        _ => {
            if params_str.contains("javascript:") || params_str.contains("<script") {
                return Err(anyhow!("Suspicious content in parameters for method: {}", method));
            }
        }
    }

    Ok(())
}

fn validate_text_document_params(params: &serde_json::Value) -> Result<()> {
    if let Some(uri) = params
        .get("textDocument")
        .and_then(|text_document| text_document.get("uri"))
        .and_then(serde_json::Value::as_str)
    {
        if !ALLOWED_TEXT_DOCUMENT_URI_SCHEMES.iter().any(|scheme| uri.starts_with(scheme)) {
            return Err(anyhow!("Invalid URI scheme: {}", uri));
        }

        if uri.len() > MAX_URI_LENGTH {
            return Err(anyhow!("URI too long: {}", uri));
        }
    }

    if let Some(text) = params
        .get("textDocument")
        .and_then(|text_document| text_document.get("text"))
        .and_then(serde_json::Value::as_str)
    {
        validate_file_content(text, Path::new("<lsp_input>"))?;
    }

    Ok(())
}

fn validate_execute_command_params(params: &serde_json::Value) -> Result<()> {
    if let Some(command) = params.get("command").and_then(serde_json::Value::as_str)
        && !ALLOWED_COMMANDS.contains(&command)
    {
        return Err(anyhow!("Command not allowed: {}", command));
    }

    Ok(())
}
