//! IDE integration helpers (LSP/DAP runtime support).
//!
//! Note: Debug Adapter Protocol (DAP) implementation has been moved to the `perl-dap` crate.

/// Deprecated LSP call hierarchy provider shim (moved to `perl-lsp`).
pub mod call_hierarchy_provider;
/// Deprecated LSP cancellation mod shim (moved to `perl-lsp`).
pub mod cancellation;
/// Deprecated LSP diagnostics catalog shim (moved to `perl-lsp`).
pub mod diagnostics_catalog;
/// Deprecated executeCommand shim for LSP/tooling integrations (moved to `perl-lsp`).
#[cfg(not(target_arch = "wasm32"))]
pub mod execute_command;
/// Deprecated in-crate LSP shim (use `perl_lsp` for runtime support).
pub mod lsp;
/// LSP compatibility shims for legacy integrations.
pub mod lsp_compat;
