//! IDE integration helpers (LSP/DAP runtime support).

/// LSP call hierarchy provider for function call navigation.
pub mod call_hierarchy_provider;
/// Enhanced LSP cancellation infrastructure.
pub mod cancellation;
/// Diagnostic catalog with stable codes for consistent error reporting.
pub mod diagnostics_catalog;
/// Debug Adapter Protocol (DAP) implementation for Perl debugging.
pub mod debug_adapter;
/// ExecuteCommand support for LSP and tooling integrations.
#[cfg(not(target_arch = "wasm32"))]
pub mod execute_command;
/// LSP compatibility shims for legacy integrations.
pub use crate::lsp_compat;
/// Deprecated in-crate LSP shim (use `perl_lsp` for runtime support).
pub use crate::lsp;
