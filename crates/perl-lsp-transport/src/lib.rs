//! LSP transport layer for perl-lsp.
//!
//! This crate provides the transport layer implementation for the Perl Language Server,
//! handling message framing according to the LSP Base Protocol specification.
//!
//! # Overview
//!
//! The LSP Base Protocol uses Content-Length based message framing over stdio (or other
//! transports). This crate provides:
//!
//! - [`ContentLengthMessageReader`] - Stateful framed reader for streaming request loops
//! - [`read_message`] - Read and parse an LSP message with Content-Length framing
//! - [`write_message`] - Write an LSP response with proper framing
//! - [`write_notification`] - Write an LSP notification with proper framing
//! - [`log_response`] - Debug logging for outgoing responses
//!
//! Incoming request bodies are decoded lossily when they contain malformed
//! UTF-8. Invalid byte sequences are replaced with U+FFFD, logged, and the
//! resulting JSON text is still parsed so transient transport corruption does
//! not necessarily drop the whole message.
//!
//! # Example
//!
//! ```no_run
//! use std::io::{BufReader, stdin, stdout};
//! use perl_lsp_transport::{read_message, write_message};
//! use perl_lsp_protocol::JsonRpcResponse;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut reader = BufReader::new(stdin());
//! let mut writer = stdout();
//!
//! // Read an incoming message
//! if let Ok(Some(request)) = read_message(&mut reader) {
//!     // Process request and create response
//!     let response = JsonRpcResponse::null(request.id);
//!
//!     // Write the response
//!     write_message(&mut writer, &response)?;
//! }
//! # Ok(())
//! # }
//! ```

#![deny(unsafe_code)]
#![deny(clippy::print_stderr, clippy::print_stdout)]
#![cfg_attr(test, allow(clippy::print_stderr, clippy::print_stdout))]
#![warn(missing_docs)]

mod framing;

pub use framing::{
    ContentLengthMessageReader, frame, log_response, read_message, write_message,
    write_notification,
};
