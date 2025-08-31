//! Test support utilities for LSP integration tests

pub mod client_caps;
pub mod lsp_client;
pub mod lsp_harness;
pub mod test_helpers;

// Re-export test helpers for convenience in test files that use `support::*`
// NOTE: test_helpers module exists but may not be used in all test contexts
#[allow(unused_imports)]
pub use test_helpers::*;
