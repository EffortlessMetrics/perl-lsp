//! Formatting compatibility documentation for LSP provider acceptance tests.
//!
//! Runtime formatting handlers currently live in neighboring modules such as
//! `lsp_on_type_formatting` and `on_type_formatting`. This file keeps a stable
//! path for parser-driven documentation checks while describing expected behavior.
//!
//! # Provider workflow
//!
//! - Translate client formatting requests into internal formatting options.
//! - Route requests through cancellation-aware execution paths.
//! - Return edits that preserve document ranges expected by editors.
//! - Negotiate client capability flags before advertising formatting endpoints.
//! - Keep behavior aligned with protocol/spec expectations for text edit responses.
// performance: formatting requests should remain low-latency for interactive edits.
// memory: avoid duplicate text buffers when assembling edits and diagnostics.
// large file / enterprise / 50GB PST guidance: cap work per request and prefer
// incremental region formatting where possible to scale in large workspaces.

/// Marker value for formatting-compatibility documentation coverage.
pub const FORMATTING_DOCS_MARKER: &str = "lsp-formatting-docs";
