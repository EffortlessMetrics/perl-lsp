//! LSP runtime infrastructure (Wave G2: 5 runtime crates absorbed).
//!
//! This module contains the implementation of LSP runtime support previously
//! distributed across 5 separate crates: cancellation (request/thread lifecycle),
//! limits (resource constraints), input_validation (security/hygiene),
//! launcher (process control), and text_utils (text editing utilities).
//!
//! **Note:** text_utils is semantically providers-adjacent (used by code_actions)
//! but grouped here as runtime infrastructure for organizational coherence with
//! other protocol support utilities. Verify re-exports in rs-core::providers
//! if adding new consumers.
//!
//! **Deferred (G3):** `perl-lsp-transport` cannot be absorbed here because
//! `perl-lsp-protocol` (which transport depends on for `JsonRpcRequest`/`JsonRpcResponse`)
//! already depends on `perl-lsp-rs-core` — absorbing transport would create a
//! crate dependency cycle. Defer to Wave G3 when the `perl-lsp-protocol` dependency
//! direction is resolved.
//!
//! ## Module structure
//!
//! - **cancellation**: Request-scoped cancellation tokens with atomic operations
//! - **limits**: Memory/resource budgets and deadline constraints
//! - **input_validation**: Security validation (file paths, content, LSP requests)
//! - **launcher**: CLI parsing, logging initialization, startup coordination
//! - **text_utils**: Text editing helpers (TextEditHelpers, edit composition)

pub mod cancellation;
pub mod input_validation;
pub mod launcher;
pub mod limits;
pub mod text_utils;
pub mod tuning;

// Re-exports for ergonomic access
pub use cancellation::*;
pub use input_validation::*;
pub use launcher::*;
pub use limits::*;
pub use text_utils::*;
pub use tuning::{DiagnosticMode, RuntimeMode, RuntimeTuning};
