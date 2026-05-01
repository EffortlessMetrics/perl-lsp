//! Optional feature-gated exports.

#[cfg(feature = "incremental")]
/// Advanced AST node reuse strategies for incremental parsing.
pub use crate::incremental::incremental_advanced_reuse;
#[cfg(feature = "incremental")]
/// Checkpoint-based incremental parsing with rollback support.
pub use crate::incremental::incremental_checkpoint;
#[cfg(feature = "incremental")]
/// Document-level incremental parsing state management.
pub use crate::incremental::incremental_document;
#[cfg(feature = "incremental")]
/// Edit representation and application for incremental updates.
pub use crate::incremental::incremental_edit;
#[cfg(feature = "incremental")]
#[deprecated(note = "LSP server moved to perl-lsp; perl-parser no longer handles didChange")]
/// Legacy incremental handler (deprecated, use `perl-lsp` crate instead).
pub use crate::incremental::incremental_handler_v2;
#[cfg(feature = "incremental")]
/// Integration layer connecting incremental parsing with the full parser.
pub use crate::incremental::incremental_integration;
#[cfg(feature = "incremental")]
/// Simplified incremental parsing interface for common use cases.
pub use crate::incremental::incremental_simple;
#[cfg(feature = "incremental")]
/// Second-generation incremental parsing with improved node reuse.
pub use crate::incremental::incremental_v2;
