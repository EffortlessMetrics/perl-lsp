//! IDE integration helpers (LSP/DAP runtime support).

/// Deprecated LSP call hierarchy provider shim (moved to `perl-lsp`).
pub mod call_hierarchy_provider;
/// Deprecated LSP cancellation infrastructure shim (moved to `perl-lsp`).
pub mod cancellation;
/// Debug Adapter Protocol (DAP) implementation for Perl debugging.
pub mod debug_adapter;
/// Deprecated LSP diagnostics catalog shim (moved to `perl-lsp`).
pub mod diagnostics_catalog;
/// Deprecated executeCommand shim for LSP/tooling integrations (moved to `perl-lsp`).
#[cfg(not(target_arch = "wasm32"))]
pub mod execute_command;
/// Deprecated in-crate LSP shim (use `perl_lsp` for runtime support).
pub mod lsp;
/// LSP compatibility shims for legacy integrations.
pub mod lsp_compat;
