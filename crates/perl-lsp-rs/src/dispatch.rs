//! Request dispatch placeholder
//!
//! **NOTE**: The actual dispatch implementation is in `server_impl/dispatch.rs`.
//!
//! This module was initially planned for extracted dispatch logic, but the
//! implementation was placed directly in `server_impl/dispatch.rs` which is
//! the canonical location. This placeholder is retained for backwards
//! compatibility and documentation purposes.
//!
//! See `crate::lsp::server_impl::dispatch` for:
//! - Method routing via `handle_request()`
//! - Lifecycle state management (initialized, shutdown)
//! - Cancellation integration with `PerlLspCancellationToken`

// Intentionally empty - real dispatch logic lives in server_impl/dispatch.rs
