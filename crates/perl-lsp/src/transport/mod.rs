//! LSP transport layer
//!
//! Handles message framing with Content-Length headers according to
//! the LSP Base Protocol specification.

mod framing;

pub use framing::*;
