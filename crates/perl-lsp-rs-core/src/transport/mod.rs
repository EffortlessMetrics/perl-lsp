//! LSP transport layer for perl-lsp.
//!
//! This module provides the transport layer implementation for the Perl Language Server,
//! handling message framing according to the LSP Base Protocol specification.
//!
//! Previously the standalone `perl-lsp-transport` crate; absorbed into
//! `perl-lsp-rs-core::transport` in Wave G3 (#4535).

pub mod framing;

pub use framing::{
    ContentLengthFramer, ContentLengthMessageReader, FramingError, frame, log_response,
    read_message, write_message, write_notification,
};
