//! Debug launch command construction.

use serde_json::json;

use crate::protocol::JsonRpcError;

pub(super) fn unresolved_debug_perl_error(resolved: &std::path::Path) -> JsonRpcError {
    JsonRpcError {
        code: -32603,
        message: format!(
            "Cannot start Perl debugger for '{}': Perl binary could not be resolved from \
             `perl.path` or PATH. Configure `perl.path` to an explicit Perl executable; \
             refusing ambient fallback.",
            resolved.display()
        ),
        data: Some(json!({"file": resolved.display().to_string()})),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn debug_command_from_oracle(
    oracle: Option<perl_lsp_rs_core::config::PerlOracleEnv>,
    resolved: &std::path::Path,
) -> Result<std::process::Command, JsonRpcError> {
    oracle
        .as_ref()
        .map(perl_lsp_rs_core::config::PerlOracleEnv::into_command)
        .ok_or_else(|| unresolved_debug_perl_error(resolved))
}
